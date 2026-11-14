#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod annotation;
mod app;
mod assets;
mod chart;
mod color;
mod diagnostics;
mod draft;
mod export;
mod fonts;
mod i18n;
#[cfg(feature = "perf-probe")]
mod perf_probe;
mod platform;
mod preferences;
#[cfg(any(test, feature = "visual-test"))]
#[path = "../tests/support/scenes.rs"]
mod scenes;
mod session;
mod task;
mod transformation;
mod updates;
mod workspace;

use app::AppState;
use rune::{App, AppSpec, FontSmoothing};

const WINDOW_SIZE: (f32, f32) = preferences::settings::DEFAULT_WINDOW_SIZE;

/// 仅在开启布局恢复且坐标、尺寸有效时返回原生窗口恢复矩形。
fn restored_window_frame() -> Option<[f32; 4]> {
    let preferences = preferences::settings::read(&preferences::settings::path()?).ok()?;
    if !preferences.values.restore_layout {
        return None;
    }
    let [x, y] = preferences.window.position?;
    let [width, height] = preferences.window.size?;
    (x.is_finite()
        && y.is_finite()
        && width.is_finite()
        && height.is_finite()
        && width >= 600.0
        && height >= 400.0)
        .then_some([x, y, width, height])
}

/// 将原有主屏逻辑坐标转换为创建参数，避免持续定位约束在重开时覆盖用户移动。
fn restored_window_options(
    frame: Option<[f32; 4]>,
    displays: &[rune::Display],
) -> rune::WindowOptions {
    let mut options = rune::WindowOptions::default();
    if let Some([x, y, width, height]) = frame
        && let Some(display) = displays.iter().find(|display| display.primary)
    {
        let scale = display.scale;
        options.position = Some((
            display.position.0 + (f64::from(x) * scale).round() as i32,
            display.position.1 + (f64::from(y) * scale).round() as i32,
        ));
        options.physical_size = Some((
            (f64::from(width) * scale).round() as u32,
            (f64::from(height) * scale).round() as u32,
        ));
    }
    options
}

/// 注册字体、窗口回调和测试场景，普通构建不包含视觉场景入口。
fn window_spec() -> AppSpec<AppState> {
    AppSpec {
        fonts: fonts::FONTS.files,
        families: fonts::FONTS.families,
        min_size: Some(WINDOW_SIZE),
        setup: Some(app::setup),
        on_key: Some(app::keyboard::on_key),
        poll: Some(AppState::poll),
        #[cfg(feature = "visual-test")]
        scenes: &[
            ("reference", |state| scenes::select_sample(state, false)),
            ("launch-play", |s| {
                s.launch = app::launch::Launch::playback_fixture()
            }),
            ("female", |state| scenes::select_sample(state, true)),
            ("update-available", |state| {
                scenes::update_preview(state, false)
            }),
            ("update-error", |state| scenes::update_preview(state, true)),
            ("launch-stars", |s| {
                s.launch = app::launch::Launch::fixture(1.25)
            }),
            ("launch-lights", |s| {
                s.launch = app::launch::Launch::fixture(2.7)
            }),
            ("launch-logo", |s| {
                s.launch = app::launch::Launch::fixture(4.8)
            }),
            ("launch-grid", |s| {
                s.launch = app::launch::Launch::fixture(6.35)
            }),
            ("launch-landing", |s| {
                s.launch = app::launch::Launch::fixture(7.8)
            }),
        ],
        ..AppSpec::new("知几", WINDOW_SIZE, AppState::default, app::view::render)
    }
}

/// 文件管理器传递的是位置参数；Rune 自身的选项继续原样交给它解析。
#[cfg(any(target_os = "windows", target_os = "linux", test))]
fn desktop_arguments(
    args: Vec<std::ffi::OsString>,
) -> std::io::Result<(Vec<String>, platform::instance::Request)> {
    use platform::instance::Request;
    let document_launch = args
        .first()
        .is_some_and(|arg| !arg.to_string_lossy().starts_with('-'));
    if document_launch {
        Ok((
            Vec::new(),
            Request::Open(
                args.into_iter()
                    .map(std::path::absolute)
                    .collect::<std::io::Result<_>>()?,
            ),
        ))
    } else {
        Ok((
            args.into_iter()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect(),
            Request::Activate,
        ))
    }
}

/// 创建桌面应用，安装平台文件事件、生命周期和菜单后进入事件循环。
fn main() {
    if platform::update_install::finish_restart() {
        return;
    }
    fonts::select_language(fonts::DEFAULT_LANGUAGE);
    #[cfg(feature = "perf-probe")]
    let started = std::time::Instant::now();
    // 与 Linux 打包器生成的桌面入口名称一致。
    rune::set_app_id("zhiji");
    rune::set_app_icon(&include_bytes!("../assets/icons/app.icns")[..])
        .expect("the bundled app icon must decode");
    let app = App::new(window_spec());
    let handle = app.handle();
    #[cfg(target_os = "macos")]
    let headless = rune::tooling::is_headless();
    #[cfg(target_os = "macos")]
    if !headless {
        let document_handle = handle.clone();
        if let Err(error) = platform::documents::install(move |paths| {
            document_handle.update(move |state, tx| state.open_external_files(paths, tx));
        }) {
            eprintln!("安装系统文件打开处理失败：{error}");
        }
    }
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    let (rune_args, request) = desktop_arguments(std::env::args_os().skip(1).collect())
        .unwrap_or_else(|error| platform::startup_failed(&error));
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    let headless = rune::tooling::Args::parse_from(rune_args.clone()).headless();
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    let _instance = if headless {
        None
    } else {
        use platform::instance::{Instance, Request};
        let receiver = handle.clone();
        let result = preferences::settings::config_dir()
            .ok_or_else(|| std::io::Error::other("无法确定知几配置目录"))
            .and_then(|directory| {
                Instance::open(&directory, request.clone(), move |request| {
                    let delivery = platform::instance::Delivery::new();
                    let pending = delivery.clone();
                    receiver.update(move |state, tx| {
                        if !pending.claim() {
                            return;
                        }
                        match request {
                            Request::Activate => state.activate_existing(tx),
                            Request::Open(paths) => state.open_external_files(Ok(paths), tx),
                        }
                    });
                    delivery.wait()
                })
            });
        match result {
            Ok(None) => return,
            Ok(Some(instance)) => {
                if let Request::Open(paths) = request {
                    handle.update(move |state, tx| state.open_external_files(Ok(paths), tx));
                }
                Some(instance)
            }
            Err(error) => platform::startup_failed(&error),
        }
    };
    #[cfg(target_os = "macos")]
    let rune_args: Vec<String> = std::env::args().skip(1).collect();
    let restored_frame = restored_window_frame();
    let app = app
        .init(move || AppState::with_handle(handle, headless))
        .font_smoothing(FontSmoothing::Off)
        .titlebar(rune::Titlebar::new(44.0))
        .keep_running()
        .standard_menus()
        .on_close(AppState::on_close)
        .on_reopen(AppState::on_reopen)
        // Rune 的桌面帧在视图构建后还要排版和编码，必须等整帧提交完再释放语言锁。
        .on_frame_encoded(|_| fonts::finish_frame());
    #[cfg(feature = "perf-probe")]
    let app = perf_probe::attach(app, started);
    rune::tooling::run_windows_with_args(rune::RunOptions::default(), rune_args, move |displays| {
        let options = restored_window_options(restored_frame, displays);
        let mut position = options.position;
        let app = app::menu::install(app)
            .window_options(options)
            .on_window(move |window| {
                #[cfg(target_os = "macos")]
                platform::documents::window_created(window);
                // 系统创建与窗框初始化可能调整位置，只在首次创建后补正，不持续约束窗口。
                if let Some(position) = position.take() {
                    window.set_position(position);
                }
                #[cfg(target_os = "windows")]
                platform::window_icon::apply(window);
            });
        [rune::WindowSpec::from_app("main", app)]
    });
}

#[cfg(test)]
#[path = "../tests/unit/desktop_arguments.rs"]
mod argument_tests;
