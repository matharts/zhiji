//! 成功打开或保存的本地文件历史，只记录路径，不存储命盘内容。
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::mpsc;

const LIMIT: usize = 10;
const MAX_BYTES: u64 = 64 * 1024;

#[derive(Default)]
pub(super) struct Recent {
    storage: Option<PathBuf>,
    paths: Vec<PathBuf>,
    pending: Option<mpsc::Receiver<Result<(), String>>>,
    queued: bool,
    wake: Option<rune::Waker>,
}

impl Recent {
    /// 有界读取最近文件，仅保留去重后的绝对路径，最多十项。
    pub(super) fn load(storage: Option<PathBuf>) -> Self {
        let mut paths = Vec::new();
        if let Some(file) = storage
            .as_ref()
            .and_then(|path| std::fs::File::open(path).ok())
        {
            let mut bytes = Vec::new();
            if file.take(MAX_BYTES + 1).read_to_end(&mut bytes).is_ok()
                && bytes.len() <= MAX_BYTES as usize
                && let Ok(saved) = serde_json::from_slice::<Vec<PathBuf>>(&bytes)
            {
                for path in saved {
                    if path.is_absolute() && !paths.contains(&path) {
                        paths.push(path);
                    }
                    if paths.len() == LIMIT {
                        break;
                    }
                }
            }
        }
        Self {
            storage,
            paths,
            ..Self::default()
        }
    }

    /// 按最近使用顺序借用路径列表。
    pub(super) fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    /// 将成功访问的路径移到首位，并排队保存最新列表。
    pub(super) fn record(&mut self, path: PathBuf) {
        self.paths.retain(|existing| *existing != path);
        self.paths.insert(0, path);
        self.paths.truncate(LIMIT);
        self.persist();
    }

    /// 清空路径历史并保存，不删除对应用户文件。
    pub(super) fn clear(&mut self) {
        self.paths.clear();
        self.persist();
    }

    /// 设置后台保存结束时使用的窗口唤醒器。
    pub(super) fn set_waker(&mut self, wake: rune::Waker) {
        self.wake = Some(wake);
    }

    /// 判断是否有正在持久化的列表。
    pub(super) fn is_working(&self) -> bool {
        self.pending.is_some()
    }

    /// 接收保存结果，若期间列表变化则继续保存合并后的最新状态。
    pub(super) fn poll(&mut self) -> bool {
        let Some(receiver) = &self.pending else {
            return false;
        };
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return false,
            Err(mpsc::TryRecvError::Disconnected) => Err("最近文件保存任务已中断".into()),
        };
        self.pending = None;
        if let Err(error) = result {
            eprintln!("保存最近文件列表失败：{error}");
        }
        if std::mem::take(&mut self.queued) {
            self.persist();
        }
        true
    }

    /// 一次只启动一个保存任务，忙碌期间的多次变化合并为一次后续写入。
    fn persist(&mut self) {
        let Some(path) = self.storage.clone() else {
            return;
        };
        if self.pending.is_some() {
            self.queued = true;
            return;
        }
        let paths = self.paths.clone();
        let wake = self.wake.clone();
        let (sender, receiver) = mpsc::channel();
        match std::thread::Builder::new()
            .name("recent-files".into())
            .spawn(move || {
                let result = persist_paths(path, paths);
                let _ = sender.send(result);
                if let Some(wake) = wake {
                    wake();
                }
            }) {
            Ok(_) => self.pending = Some(receiver),
            Err(error) => eprintln!("无法启动最近文件保存任务：{error}"),
        }
    }
}

/// 在同目录写入临时 JSON 后替换最近文件列表。
fn persist_paths(path: PathBuf, paths: Vec<PathBuf>) -> Result<(), String> {
    let operation = || -> Result<(), Box<dyn std::error::Error>> {
        let Some(parent) = path.parent() else {
            return Ok(());
        };
        std::fs::create_dir_all(parent)?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        serde_json::to_writer(&mut file, &paths)?;
        file.flush()?;
        // 先同步临时文件，再替换列表，避免崩溃后留下未落盘的路径记录。
        file.as_file().sync_all()?;
        file.persist(path)?;
        Ok(())
    };
    operation().map_err(|error| error.to_string())
}
