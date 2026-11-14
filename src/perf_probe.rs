//! 按功能开关启用的原生首帧及输入测量，不包含系统合成延迟。

use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

use rune::App;

use crate::app::AppState;

/// 按显式环境变量安装测量回调，首帧报告写出后退出测试实例。
pub(crate) fn attach(app: App<AppState>, started: Instant) -> App<AppState> {
    let mut app = app;
    if let Some(path) = std::env::var_os("ZHIJI_INPUT_REPORT").map(PathBuf::from) {
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            Ok(mut file) => {
                let mut recorded = None;
                app = app.on_frame_encoded(move |frame| {
                    // 测量回调会替换应用安装的帧回调，仍需在每帧结束时释放语言锁。
                    crate::fonts::finish_frame();
                    let Some(input) = rune::last_input() else {
                        return;
                    };
                    if frame.outcome() != rune::FrameOutcome::Drawn
                        || recorded == Some(input)
                        || frame.at() < input
                    {
                        return;
                    }
                    recorded = Some(input);
                    let delay = frame.at().duration_since(input).as_secs_f64() * 1_000.0;
                    let _ = writeln!(
                        file,
                        "input_to_submit_ms={delay:.3} update_ms={:.3} encode_ms={:.3}",
                        frame.update_ms().unwrap_or_default(),
                        frame.encode_ms().unwrap_or_default(),
                    );
                });
            }
            Err(error) => eprintln!("zhiji: cannot write input measurement: {error}"),
        }
    }
    if let Some(path) = std::env::var_os("ZHIJI_FIRST_FRAME_REPORT").map(PathBuf::from) {
        let handle = app.handle();
        let mut path = Some(path);
        app = app.on_frame_drawn(move |_| {
            let Some(path) = path.take() else {
                return;
            };
            let elapsed = started.elapsed().as_secs_f64() * 1_000.0;
            let result = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .and_then(|mut file| writeln!(file, "main_to_first_draw_ms={elapsed:.3}"));
            if let Err(error) = result {
                eprintln!("zhiji: cannot write first-frame measurement: {error}");
            }
            handle.quit();
        });
    }
    app
}
