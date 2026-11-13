//! 桌面实例间的文档请求转发。

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use fs2::FileExt;
use serde::{Deserialize, Serialize};

const LIMIT: usize = 128 * 1024;
const WAIT: Duration = Duration::from_secs(4);

#[derive(Clone, Copy, PartialEq, Eq)]
enum DeliveryState {
    Pending,
    Claimed,
    Cancelled,
}

/// UI 领取请求才确认成功；超时撤销尚未领取的闭包，避免退出丢失和迟到重复执行。
#[derive(Clone)]
pub(crate) struct Delivery(Arc<(Mutex<DeliveryState>, Condvar)>);

impl Delivery {
    /// 创建一次打开请求的领取凭证。
    pub(crate) fn new() -> Self {
        Self(Arc::new((
            Mutex::new(DeliveryState::Pending),
            Condvar::new(),
        )))
    }

    /// 主线程开始处理时原子领取，超时后的闭包必须跳过。
    pub(crate) fn claim(&self) -> bool {
        let (state, changed) = &*self.0;
        let Ok(mut state) = state.lock() else {
            return false;
        };
        if *state != DeliveryState::Pending {
            return false;
        }
        *state = DeliveryState::Claimed;
        changed.notify_one();
        true
    }

    /// 等待 UI 领取；期限到达后取消，监听线程不会确认无人消费的队列。
    pub(crate) fn wait(&self) -> bool {
        self.wait_for(Duration::from_secs(2))
    }

    /// 有界等待与取消共用状态锁，避免超时和领取同时发生时出现二义性。
    fn wait_for(&self, timeout: Duration) -> bool {
        let (state, changed) = &*self.0;
        let Ok(state) = state.lock() else {
            return false;
        };
        let Ok((mut state, _)) =
            changed.wait_timeout_while(state, timeout, |state| *state == DeliveryState::Pending)
        else {
            return false;
        };
        if *state == DeliveryState::Pending {
            *state = DeliveryState::Cancelled;
        }
        *state == DeliveryState::Claimed
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Request {
    Activate,
    Open(Vec<PathBuf>),
}

#[derive(Serialize, Deserialize)]
struct Endpoint {
    port: u16,
    token: [u8; 32],
}

#[derive(Serialize, Deserialize)]
struct Packet {
    token: [u8; 32],
    paths: Option<Vec<Vec<u8>>>,
}

/// 持有进程锁和监听线程，释放后允许后续启动接管。
pub(crate) struct Instance {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    endpoint: PathBuf,
    _lock: File,
}

impl Instance {
    /// 抢占唯一实例；若已有实例则可靠转发请求并等待其处理确认。
    pub(crate) fn open(
        directory: &Path,
        request: Request,
        handler: impl Fn(Request) -> bool + Send + 'static,
    ) -> io::Result<Option<Self>> {
        fs::create_dir_all(directory)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(directory.join("instance.lock"))?;
        let endpoint = directory.join("instance.json");
        let deadline = Instant::now() + WAIT;
        loop {
            match lock.try_lock_exclusive() {
                Ok(()) => break,
                Err(error)
                    if error.kind() == io::ErrorKind::WouldBlock
                        || error
                            .raw_os_error()
                            .zip(fs2::lock_contended_error().raw_os_error())
                            .is_some_and(|(actual, expected)| actual == expected) => {}
                Err(error) => return Err(error),
            }
            if forward(&endpoint, &request).is_ok() {
                return Ok(None);
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "无法将打开请求交给正在运行的知几",
                ));
            }
            thread::sleep(Duration::from_millis(50));
        }

        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))?;
        listener.set_nonblocking(true)?;
        let mut token = [0; 32];
        getrandom::fill(&mut token)
            .map_err(|error| io::Error::other(format!("无法创建单实例凭据：{error}")))?;
        let address = Endpoint {
            port: listener.local_addr()?.port(),
            token,
        };
        let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
        serde_json::to_writer(&mut temporary, &address)?;
        temporary.flush()?;
        temporary.persist(&endpoint).map_err(|error| error.error)?;

        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let worker = thread::Builder::new()
            .name("zhiji-instance".into())
            .spawn(move || {
                while !thread_stop.load(Ordering::Acquire) {
                    match listener.accept() {
                        Ok((mut stream, peer)) if peer.ip().is_loopback() => {
                            let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
                            let result =
                                receive(&mut stream, &token, &thread_stop).is_ok_and(&handler);
                            let _ = stream.write_all(&[u8::from(result)]);
                        }
                        Ok(_) => {}
                        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(30))
                        }
                        Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                        Err(error) => {
                            eprintln!("单实例监听失败：{error}");
                            break;
                        }
                    }
                }
            })?;
        Ok(Some(Self {
            stop,
            worker: Some(worker),
            endpoint,
            _lock: lock,
        }))
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        // 锁仍由本实例持有；删除旧地址后才释放，避免清除新实例的地址。
        let _ = fs::remove_file(&self.endpoint);
    }
}

/// 将请求发送到持锁进程，确认该进程已接收请求。
fn forward(endpoint: &Path, request: &Request) -> io::Result<()> {
    let mut bytes = Vec::new();
    File::open(endpoint)?.take(4097).read_to_end(&mut bytes)?;
    if bytes.len() > 4096 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "单实例地址过大"));
    }
    let endpoint: Endpoint = serde_json::from_slice(&bytes)?;
    let packet = Packet {
        token: endpoint.token,
        paths: match request {
            Request::Activate => None,
            Request::Open(paths) => Some(paths.iter().map(|path| path_bytes(path)).collect()),
        },
    };
    let payload = serde_json::to_vec(&packet)?;
    if payload.len() > LIMIT {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "打开请求过大"));
    }
    let mut stream = TcpStream::connect_timeout(
        &SocketAddrV4::new(Ipv4Addr::LOCALHOST, endpoint.port).into(),
        Duration::from_millis(250),
    )?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    stream.write_all(&(payload.len() as u32).to_be_bytes())?;
    stream.write_all(&payload)?;
    let mut reply = [0];
    stream.read_exact(&mut reply)?;
    if reply == [1] {
        Ok(())
    } else {
        Err(io::Error::other("已有实例未接收打开请求"))
    }
}

/// 在同一绝对期限内读取整条消息，退出时及时取消未完成的连接。
fn read_request(
    stream: &mut TcpStream,
    mut bytes: &mut [u8],
    deadline: Instant,
    stop: &AtomicBool,
) -> io::Result<()> {
    while !bytes.is_empty() {
        if stop.load(Ordering::Acquire) {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "实例正在退出"));
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "打开请求接收超时"));
        }
        stream.set_read_timeout(Some(remaining.min(Duration::from_millis(100))))?;
        match stream.read(bytes) {
            Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
            Ok(length) => bytes = &mut bytes[length..],
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::TimedOut
                        | io::ErrorKind::WouldBlock
                        | io::ErrorKind::Interrupted
                ) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

/// 校验消息大小、认证凭据及路径数量，再交给应用。
fn receive(stream: &mut TcpStream, token: &[u8; 32], stop: &AtomicBool) -> io::Result<Request> {
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut length = [0; 4];
    read_request(stream, &mut length, deadline, stop)?;
    let length = u32::from_be_bytes(length) as usize;
    if length > LIMIT {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "打开请求过大"));
    }
    let mut payload = vec![0; length];
    read_request(stream, &mut payload, deadline, stop)?;
    let packet: Packet = serde_json::from_slice(&payload)?;
    if &packet.token != token {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "单实例请求未认证",
        ));
    }
    match packet.paths {
        None => Ok(Request::Activate),
        Some(paths) if paths.len() <= 32 => Ok(Request::Open(
            paths
                .into_iter()
                .map(path_from_bytes)
                .collect::<io::Result<_>>()?,
        )),
        Some(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "打开请求包含过多文件",
        )),
    }
}

#[cfg(unix)]
/// 保留 Unix 文件名的原始字节。
fn path_bytes(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}
#[cfg(unix)]
/// 从原始字节恢复 Unix 文件名。
fn path_from_bytes(bytes: Vec<u8>) -> io::Result<PathBuf> {
    use std::os::unix::ffi::OsStringExt;
    Ok(PathBuf::from(std::ffi::OsString::from_vec(bytes)))
}
#[cfg(windows)]
/// 保留 Windows 文件名的 UTF-16 编码单元。
fn path_bytes(path: &Path) -> Vec<u8> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str()
        .encode_wide()
        .flat_map(u16::to_le_bytes)
        .collect()
}
#[cfg(windows)]
/// 校验编码长度并恢复 Windows 文件名。
fn path_from_bytes(bytes: Vec<u8>) -> io::Result<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    if !bytes.len().is_multiple_of(2) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "文件路径编码无效",
        ));
    }
    let wide: Vec<_> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    Ok(PathBuf::from(std::ffi::OsString::from_wide(&wide)))
}

#[cfg(test)]
#[path = "../../tests/unit/instance.rs"]
mod tests;
