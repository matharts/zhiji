//! 将离屏产品截图与导出 PNG 对照已检入的独立参考图。

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[path = "support/visual.rs"]
mod visual;

use visual::{Image, compare, fixture};

#[cfg(target_os = "windows")]
#[test]
#[ignore = "requires a Windows GPU adapter for caption rendering"]
/// 验证系统与应用主题不同时，窗口按钮可见且低 DPI 大限文字居中。
fn windows_caption_buttons_follow_application_appearance() {
    let binary = Path::new(env!("CARGO_BIN_EXE_zhiji"));
    let output = tempfile::tempdir().unwrap();
    let script = output.path().join("night.json");
    std::fs::write(
        &script,
        r#"{"events":[{"t":0.1,"type":"tap","key":"night-mode"},{"t":0.2,"type":"move","x":20,"y":100}]}"#,
    )
    .unwrap();
    for (system, night) in [("light", true), ("dark", false)] {
        for scale in [1, 2] {
            let actual = output.path().join(format!("{system}-{scale}.png"));
            let scale_arg = scale.to_string();
            let mut arguments = vec![
                "--scene",
                "reference",
                "--snap",
                actual.to_str().unwrap(),
                "--at",
                "1",
                "--size",
                "1300x800",
                "--scale",
                &scale_arg,
                "--theme",
                system,
            ];
            if night {
                arguments.extend(["--script", script.to_str().unwrap()]);
            }
            run(binary, &arguments);
            let image = Image::read(&actual);
            for button in 0..3 {
                let center_x = 1300 - 46 * (3 - button) + 23;
                let mut visible = 0;
                for y in 16 * scale..28 * scale {
                    for x in (center_x - 6) * scale..(center_x + 6) * scale {
                        let offset = (y * image.width as usize + x) * 4;
                        let rgb = &image.rgba[offset..offset + 3];
                        visible += usize::from(if night {
                            rgb.iter().all(|channel| *channel > 150)
                        } else {
                            rgb.iter().all(|channel| *channel < 120)
                        });
                    }
                }
                assert!(
                    visible >= 4 * scale,
                    "{system} {scale}x button={button}: {visible}"
                );
            }
            // 固定参考盘左上“大田”和右侧“大疾”的横栏；排除边线后检查实际墨迹。
            for (left, top) in [(89, 198), (539, 348)] {
                let mut first = usize::MAX;
                let mut last = 0;
                for y in (top + 2) * scale..(top + 24) * scale {
                    for x in (left + 3) * scale..(left + 97) * scale {
                        let offset = (y * image.width as usize + x) * 4;
                        let rgb = &image.rgba[offset..offset + 3];
                        let ink = if night {
                            rgb.iter().all(|channel| *channel > 150)
                        } else {
                            rgb.iter().all(|channel| *channel < 120)
                        };
                        if ink {
                            first = first.min(y);
                            last = last.max(y);
                        }
                    }
                }
                assert_ne!(first, usize::MAX, "missing decade label at {left},{top}");
                let center = (first + last + 1) as f32 / (2 * scale) as f32;
                assert!(
                    (center - top as f32 - 12.5).abs() <= 1.0,
                    "{system} {scale}x decade at {left},{top}: ink center={center}"
                );
            }
        }
    }
}

/// 执行离屏测试命令，失败时保留进程诊断。
fn run(binary: &Path, arguments: &[&str]) {
    use std::io::{Read, Seek, SeekFrom};
    use std::sync::OnceLock;

    // 其他 Cargo 命令可能在本套件运行期间替换 target/debug 中的程序。
    // 隔离可执行文件，确保所有截图使用同一次编译产物。
    static BINARY: OnceLock<(tempfile::TempDir, std::path::PathBuf)> = OnceLock::new();
    let (_, binary) = BINARY.get_or_init(|| {
        let directory =
            tempfile::tempdir().expect("create isolated screenshot executable directory");
        let copy = directory
            .path()
            .join(binary.file_name().expect("screenshot executable filename"));
        std::fs::copy(binary, &copy).expect("copy screenshot executable");
        (directory, copy)
    });

    let config = tempfile::tempdir().expect("create isolated screenshot configuration");
    let mut log = tempfile::tempfile().expect("create screenshot log");
    let mut child = Command::new(binary)
        .args(arguments)
        .env("ZHIJI_CONFIG_DIR", config.path())
        .stdout(Stdio::from(log.try_clone().unwrap()))
        .stderr(Stdio::from(log.try_clone().unwrap()))
        .spawn()
        .expect("run desktop headlessly");
    let deadline = Instant::now() + Duration::from_secs(90);
    let status = loop {
        if let Some(status) = child.try_wait().expect("check desktop process") {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("desktop screenshot timed out: {arguments:?}");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let mut output = String::new();
    log.seek(SeekFrom::Start(0)).unwrap();
    log.read_to_string(&mut output).unwrap();
    assert!(status.success(), "desktop failed: {status}\n{output}");
}

#[test]
/// 以可见变化作负向对照，验证比较器确实能拒绝回归。
fn comparison_rejects_a_visible_change() {
    let expected = Image::read(&fixture("default-1x.png"));
    let mut altered = Image {
        width: expected.width,
        height: expected.height,
        rgba: expected.rgba.clone(),
    };
    for y in 100..112usize {
        for x in 100..112usize {
            let offset = (y * altered.width as usize + x) * 4;
            altered.rgba[offset] ^= 255;
            altered.rgba[offset + 1] ^= 255;
            altered.rgba[offset + 2] ^= 255;
        }
    }
    assert!(compare(&expected, &expected).is_ok());
    assert!(compare(&expected, &altered).is_err());
}

#[test]
/// 允许后端对整帧合成产生不可辨的舍入，但拒绝继续增大的通道偏移。
fn comparison_limits_backend_rounding() {
    let expected = Image::read(&fixture("help-guide-1x.png"));
    let mut rounded = Image {
        width: expected.width,
        height: expected.height,
        rgba: expected.rgba.clone(),
    };
    for channel in &mut rounded.rgba {
        *channel = channel.saturating_add(4);
    }
    assert!(compare(&expected, &rounded).is_ok());
    for channel in &mut rounded.rgba {
        *channel = channel.saturating_add(1);
    }
    assert!(compare(&expected, &rounded).is_err());
}

#[test]
#[ignore = "requires a GPU adapter for headless screenshots"]
/// 验证单帧绘制匹配固定参考图。
fn single_frame_matches_reference_fixture() {
    let binary = Path::new(env!("CARGO_BIN_EXE_zhiji"));
    let output = tempfile::tempdir().unwrap();
    let actual = output.path().join("reference.png");
    run(
        binary,
        &[
            "--scene",
            "reference",
            "--snap",
            actual.to_str().unwrap(),
            "--at",
            "0",
            "--size",
            "1300x800",
            "--scale",
            "1",
        ],
    );
    compare(
        &Image::read(&fixture("reference-1x.png")),
        &Image::read(&actual),
    )
    .unwrap_or_else(|error| panic!("single frame: {error}"));
}

#[test]
#[ignore = "requires a GPU adapter for headless screenshots"]
/// 验证独立命盘不受窗口布局影响且匹配原始参考。
fn chart_matches_original_reference_independently_of_window_layout() {
    let binary = Path::new(env!("CARGO_BIN_EXE_zhiji"));
    let output = tempfile::tempdir().unwrap();
    let actual = output.path().join("chart.png");
    run(
        binary,
        &[
            "--scene",
            "reference",
            "--crop",
            "chart",
            "--snap",
            actual.to_str().unwrap(),
            "--at",
            "0",
            "--size",
            "1300x800",
            "--scale",
            "1",
        ],
    );
    compare(
        &Image::read(&fixture("chart-original-1x.png")),
        &Image::read(&actual),
    )
    .unwrap_or_else(|error| panic!("original chart: {error}"));
}

#[test]
#[ignore = "requires a GPU adapter for headless screenshots"]
/// 验证顶部离心代码位于外侧留白中心。
fn top_outward_code_is_centered_in_the_outer_white_margin() {
    let binary = Path::new(env!("CARGO_BIN_EXE_zhiji"));
    let output = tempfile::tempdir().unwrap();
    for scale in [1usize, 2] {
        let path = output.path().join(format!("top-code-{scale}.png"));
        run(
            binary,
            &[
                "--scene",
                "reference",
                "--snap",
                path.to_str().unwrap(),
                "--at",
                "0",
                "--size",
                "1300x800",
                "--scale",
                &scale.to_string(),
            ],
        );
        let image = Image::read(&path);
        let mut rows = Vec::new();
        for y in 68 * scale..85 * scale {
            for x in 185 * scale..210 * scale {
                let offset = (y * image.width as usize + x) * 4;
                let [r, g, b] = image.rgba[offset..offset + 3] else {
                    unreachable!()
                };
                if r > 89
                    && u16::from(r) * 10 > u16::from(g) * 14
                    && u16::from(r) * 10 > u16::from(b) * 12
                    && g < 204
                {
                    rows.push(y);
                }
            }
        }
        let top = *rows.iter().min().expect("top outward code") as f32;
        let bottom = *rows.iter().max().unwrap() as f32;
        let margin_center = (68 * scale + 85 * scale - 1) as f32 / 2.0;
        assert!(
            ((top + bottom) / 2.0 - margin_center).abs() <= 0.6,
            "{scale}x: code rows {top}..{bottom}, margin center {margin_center}"
        );
    }
}

#[test]
#[ignore = "requires a GPU adapter for headless rendering"]
/// 验证侧栏过渡保持居中且可反向。
fn sidebar_transition_centers_chart_and_reverses() {
    use serde_json::Value;

    let binary = Path::new(env!("CARGO_BIN_EXE_zhiji"));
    let output = tempfile::tempdir().unwrap();
    let tree = |script: &str, time: &str| -> Value {
        let path = output.path().join(format!("{script}-{time}.json"));
        let script = fixture(script);
        run(
            binary,
            &[
                "--scene",
                "reference",
                "--script",
                script.to_str().unwrap(),
                "--tree",
                path.to_str().unwrap(),
                "--at",
                time,
                "--size",
                "1300x800",
                "--scale",
                "1",
            ],
        );
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    };
    let element = |tree: &Value, key: &str| -> Value {
        tree["elements"]
            .as_array()
            .unwrap()
            .iter()
            .find(|element| element["key"] == key && element.get("exiting").is_none())
            .unwrap_or_else(|| panic!("missing {key}"))
            .clone()
    };
    let initial = tree("collapsed.json", "0.1");
    let middle = tree("collapsed.json", "0.25");
    let collapsed = tree("collapsed.json", "0.8");
    let chart = element(&initial, "chart")["rect"].clone();
    assert_eq!(chart, serde_json::json!([34.0, 68.0, 660.0, 660.0]));
    for frame in [&middle, &collapsed] {
        assert_eq!(frame["viewport"], serde_json::json!([1300.0, 800.0]));
        let toggle = element(frame, "toggle-sidebar");
        let chart_rect = element(frame, "chart")["rect"].clone();
        let toolbar_rect = element(frame, "annotation-toolbar")["rect"].clone();
        let chart_center = chart_rect[0].as_f64().unwrap() + chart_rect[2].as_f64().unwrap() / 2.0;
        let toolbar_center =
            toolbar_rect[0].as_f64().unwrap() + toolbar_rect[2].as_f64().unwrap() / 2.0;
        assert!((chart_center - toolbar_center).abs() < 0.1);
        assert!(toolbar_rect[1].as_f64().unwrap() + toolbar_rect[3].as_f64().unwrap() <= 800.0);
        let right = toggle["rect"][0].as_f64().unwrap() + toggle["rect"][2].as_f64().unwrap();
        assert!(right <= frame["viewport"][0].as_f64().unwrap());
    }
    let initial_export_x = element(&initial, "export-image")["rect"][0]
        .as_f64()
        .unwrap();
    let middle_export_x = element(&middle, "export-image")["rect"][0]
        .as_f64()
        .unwrap();
    let collapsed_export_x = element(&collapsed, "export-image")["rect"][0]
        .as_f64()
        .unwrap();
    assert!(initial_export_x < middle_export_x && middle_export_x < collapsed_export_x);
    let middle_chart = element(&middle, "chart");
    let x = middle_chart["rect"][0].as_f64().unwrap();
    let side = middle_chart["rect"][2].as_f64().unwrap();
    assert!(x > 34.0 && x < 300.0 && side > 660.0 && side < 700.0);
    assert_eq!(
        middle_chart["rect"],
        element(&middle, "chart-lines")["rect"]
    );
    let exiting_panel = middle["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["key"] == "settings")
        .unwrap();
    let opacity = exiting_panel["opacity"].as_f64().unwrap();
    assert!(opacity > 0.0 && opacity < 1.0);
    let panel_x = exiting_panel["rect"][0].as_f64().unwrap();
    assert!(panel_x > 728.0 && panel_x < 1300.0);
    assert!(x + side < panel_x, "panel overlaps the moving chart");
    assert!(((panel_x - 728.0) / 572.0 - (x - 34.0) / 266.0).abs() < 0.02);
    assert_eq!(
        element(&collapsed, "chart")["rect"],
        serde_json::json!([300.0, 42.0, 700.0, 700.0])
    );
    assert!(
        collapsed["elements"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["key"] != "settings")
    );

    let continuing_to_collapse = tree("collapsed.json", "0.35");
    let reversing = tree("sidebar-reverse.json", "0.35");
    let restored = tree("sidebar-reverse.json", "1.0");
    assert!(
        element(&reversing, "chart")["rect"][0].as_f64().unwrap()
            < element(&continuing_to_collapse, "chart")["rect"][0]
                .as_f64()
                .unwrap()
    );
    assert_eq!(restored["viewport"], serde_json::json!([1300.0, 800.0]));
    assert_eq!(element(&restored, "chart")["rect"], chart);
    for frame in [&reversing, &restored] {
        assert!(element(frame, "settings")["opacity"].as_f64().unwrap() > 0.0);
    }

    let opening = tree("sidebar-expand.json", "1.05");
    let opening_chart = element(&opening, "chart");
    let opening_panel = element(&opening, "settings");
    let chart_x = opening_chart["rect"][0].as_f64().unwrap();
    let chart_right = chart_x + opening_chart["rect"][2].as_f64().unwrap();
    let panel_x = opening_panel["rect"][0].as_f64().unwrap();
    assert!(panel_x > 728.0 && panel_x < 1300.0 && chart_right < panel_x);
    assert!(((panel_x - 728.0) / 572.0 - (chart_x - 34.0) / 266.0).abs() < 0.02);

    let reverse_opening = tree("sidebar-reverse-opening.json", "2.0");
    assert_eq!(
        element(&reverse_opening, "chart")["rect"],
        serde_json::json!([300.0, 42.0, 700.0, 700.0])
    );

    // 面板进出时必须在浅色窗口背景上淡入淡出。
    let actual = output.path().join("sidebar-midpoint.png");
    let script = fixture("collapsed.json");
    run(
        binary,
        &[
            "--scene",
            "reference",
            "--script",
            script.to_str().unwrap(),
            "--snap",
            actual.to_str().unwrap(),
            "--at",
            "0.25",
            "--size",
            "1300x800",
            "--scale",
            "1",
        ],
    );
    let image = Image::read(&actual);
    let offset = (780 * image.width as usize + 1100) * 4;
    assert!(
        image.rgba[offset..offset + 3]
            .iter()
            .all(|channel| *channel >= 240)
    );
}

#[test]
#[ignore = "requires a GPU adapter for headless screenshots"]
/// 验证缩放后宫格线宽一致。
fn resized_grid_line_has_consistent_weight() {
    let binary = Path::new(env!("CARGO_BIN_EXE_zhiji"));
    let output = tempfile::tempdir().unwrap();
    let actual = output.path().join("small-chart.png");
    run(
        binary,
        &[
            "--snap",
            actual.to_str().unwrap(),
            "--crop",
            "chart",
            "--at",
            "0.8",
            "--size",
            "900x600",
            "--scale",
            "1",
        ],
    );
    let image = Image::read(&actual);
    assert_eq!((image.width, image.height), (504, 504));
    let pixel = |x: usize, y: usize| {
        let offset = (y * image.width as usize + x) * 4;
        &image.rgba[offset..offset + 3]
    };
    for y in [22, 23] {
        let expected = pixel(100, y);
        for x in 30..475 {
            // 这些位置是竖向边线与顶部边线的交点。
            if [137_isize, 252, 367]
                .iter()
                .any(|junction| (x as isize - junction).abs() <= 2)
            {
                continue;
            }
            let actual = pixel(x, y);
            assert!(
                actual
                    .iter()
                    .zip(expected)
                    .all(|(a, b)| a.abs_diff(*b) <= 3),
                "uneven top rule at ({x}, {y}): {actual:?} versus {expected:?}"
            );
        }
    }
}

#[test]
#[ignore = "requires a GPU adapter for headless screenshots"]
/// 验证命盘边框线宽和箭尾对齐。
fn reference_chart_preserves_border_widths_and_aligned_tails() {
    let binary = Path::new(env!("CARGO_BIN_EXE_zhiji"));
    let output = tempfile::tempdir().unwrap();
    let actual = output.path().join("reference-chart.png");
    run(
        binary,
        &[
            "--scene",
            "reference",
            "--snap",
            actual.to_str().unwrap(),
            "--crop",
            "chart",
            "--at",
            "0.8",
            "--size",
            "1300x800",
            "--scale",
            "2",
        ],
    );
    let image = Image::read(&actual);
    assert_eq!((image.width, image.height), (1320, 1320));
    let pixel = |x: usize, y: usize| {
        let offset = (y * image.width as usize + x) * 4;
        &image.rgba[offset..offset + 3]
    };
    let paper = [255_u8, 255, 255];
    let frame = [0_u8, 0, 0];
    let detail = [0_u8, 0, 0];
    let arrow = [165_u8, 29, 37];
    let matches = |actual: &[u8], expected: &[u8]| {
        actual
            .iter()
            .zip(expected)
            .all(|(a, b)| a.abs_diff(*b) <= 8)
    };
    for (x, from, to, ink) in [
        (200, 34, 46, frame),
        (200, 58, 62, frame),
        (140, 258, 262, detail),
        (150, 786, 790, arrow),
    ] {
        assert!(matches(pixel(x, from - 1), &paper));
        for y in from..to {
            assert!(matches(pixel(x, y), &ink), "rule at ({x}, {y})");
        }
        assert!(matches(pixel(x, to), &paper));
    }
    // 天相的两个字应与其他定位星使用相同灰色。
    let location_ink = [128_u8, 128, 128];
    for (name, top, bottom) in [("天", 370, 400), ("相", 402, 432)] {
        let gray_pixels = (top..bottom)
            .flat_map(|y| (276..308).map(move |x| (x, y)))
            .filter(|&(x, y)| matches(pixel(x, y), &location_ink))
            .count();
        assert!(
            gray_pixels >= 40,
            "{name} is not gray: {gray_pixels} pixels"
        );
    }
    // 生年四化与自化标记共用清晰可读的深色。
    let transform_ink = [165_u8, 29, 37];
    for (name, left, top, right, bottom, minimum) in [
        ("birth A", 1210, 138, 1255, 170, 60),
        ("birth B", 1210, 738, 1255, 772, 60),
        ("birth C", 909, 1038, 956, 1070, 60),
        ("birth D", 610, 1038, 655, 1070, 60),
        ("inward B", 400, 370, 435, 400, 40),
        ("outward C", 315, 0, 345, 30, 40),
        ("outward A", 570, 1290, 614, 1320, 40),
    ] {
        let count = (top..bottom)
            .flat_map(|y| (left..right).map(move |x| (x, y)))
            .filter(|&(x, y)| matches(pixel(x, y), &transform_ink))
            .count();
        assert!(count >= minimum, "{name} is too faint: {count} pixels");
    }
    let is_red = |color: &[u8]| {
        color[0] > 150
            && color[0] > color[1].saturating_add(70)
            && color[0] > color[2].saturating_add(70)
    };
    // 覆盖四条边线的外缘及角点连接处。
    for (edge, outside) in [
        ((510, 358), (510, 357)),
        ((810, 358), (810, 357)),
        ((358, 509), (357, 509)),
        ((961, 510), (962, 510)),
        ((359, 961), (359, 962)),
        ((960, 961), (960, 962)),
    ] {
        assert!(
            is_red(pixel(edge.0, edge.1)),
            "inward tail does not cover the border at {edge:?}"
        );
        assert!(
            !is_red(pixel(outside.0, outside.1)),
            "inward tail crosses the border at {outside:?}"
        );
    }
    // 内向箭尾可覆盖边线，但不能进入相邻宫位。
    for (inside, overlap, outside) in [
        ((511, 362), (510, 360), (510, 357)),
        ((808, 362), (809, 360), (809, 357)),
        ((957, 510), (959, 510), (962, 510)),
    ] {
        for (x, y) in [inside, overlap] {
            assert!(is_red(pixel(x, y)), "inward tail does not reach ({x}, {y})");
        }
        assert!(matches(pixel(outside.0, outside.1), &paper));
    }
    // 排除字母标签后，检查外向箭尖没有进入边距。
    for (left, top, right, bottom) in [
        (29, 776, 34, 826),
        (1286, 776, 1291, 802),
        (320, 29, 341, 34),
        (582, 1286, 603, 1292),
    ] {
        for y in top..bottom {
            for x in left..right {
                let color = pixel(x, y);
                assert!(
                    !(color[0] > color[1].saturating_add(8)
                        && color[0] > color[2].saturating_add(8)),
                    "outward arrow crossed the frame at ({x}, {y}): {color:?}"
                );
            }
        }
    }

    let actual_1x = output.path().join("reference-chart-1x.png");
    run(
        binary,
        &[
            "--scene",
            "reference",
            "--snap",
            actual_1x.to_str().unwrap(),
            "--crop",
            "chart",
            "--at",
            "0.8",
            "--size",
            "1300x800",
            "--scale",
            "1",
        ],
    );
    let image_1x = Image::read(&actual_1x);
    assert_eq!((image_1x.width, image_1x.height), (660, 660));
    let pixel_1x = |x: usize, y: usize| {
        let offset = (y * image_1x.width as usize + x) * 4;
        &image_1x.rgba[offset..offset + 3]
    };
    for (edge, outside) in [
        ((255, 179), (255, 178)),
        ((179, 254), (178, 254)),
        ((480, 255), (481, 255)),
        ((179, 480), (179, 481)),
    ] {
        assert!(is_red(pixel_1x(edge.0, edge.1)), "1x border at {edge:?}");
        assert!(
            !is_red(pixel_1x(outside.0, outside.1)),
            "1x tail crosses the border at {outside:?}"
        );
    }
}

#[test]
#[ignore = "requires a GPU adapter for headless screenshots"]
/// 验证隐藏字段仍保留布局和可见分隔线连接。
fn hidden_stem_or_palace_preserves_layout_and_joins_visible_rules() {
    let binary = Path::new(env!("CARGO_BIN_EXE_zhiji"));
    let output = tempfile::tempdir().unwrap();
    let script = fixture("border_visibility.json");
    for scale in [1_usize, 2] {
        for (state, at, stem_hidden, palace_hidden) in [
            ("stem-hidden", "0.2", true, false),
            ("palace-hidden", "0.6", false, true),
            ("both-hidden", "0.8", true, true),
        ] {
            let actual = output.path().join(format!("{state}-{scale}.png"));
            run(
                binary,
                &[
                    "--scene",
                    "reference",
                    "--script",
                    script.to_str().unwrap(),
                    "--snap",
                    actual.to_str().unwrap(),
                    "--crop",
                    "chart",
                    "--at",
                    at,
                    "--size",
                    "1300x800",
                    "--scale",
                    &scale.to_string(),
                ],
            );
            visual::assert_hidden_grid(
                &Image::read(&actual),
                scale,
                stem_hidden,
                palace_hidden,
                state,
            );
        }
    }
}

#[test]
#[ignore = "requires a GPU adapter for headless screenshots"]
/// 对窗口固定场景逐一执行视觉基准比较。
fn window_matches_visual_fixtures() {
    let binary = Path::new(env!("CARGO_BIN_EXE_zhiji"));
    let output = tempfile::tempdir().unwrap();
    let script = fixture("interaction.json");
    let script = script.to_str().unwrap();
    let flying_script = fixture("flying.json");
    let flying_script = flying_script.to_str().unwrap();
    let settings_script = fixture("settings.json");
    let settings_script = settings_script.to_str().unwrap();
    let settings_large_script = fixture("settings-large.json");
    let settings_large_script = settings_large_script.to_str().unwrap();
    let traditional_script = fixture("traditional.json");
    let traditional_script = traditional_script.to_str().unwrap();
    let settings_chart_script = fixture("settings-chart.json");
    let settings_chart_script = settings_chart_script.to_str().unwrap();
    let settings_about_script = fixture("settings-about.json");
    let settings_about_script = settings_about_script.to_str().unwrap();
    let help_guide_script = fixture("help-guide.json");
    let help_guide_script = help_guide_script.to_str().unwrap();
    let collapsed_script = fixture("collapsed.json");
    let collapsed_script = collapsed_script.to_str().unwrap();
    let visibility_script = fixture("visibility.json");
    let visibility_script = visibility_script.to_str().unwrap();
    let marking_script = fixture("marking.json");
    let marking_script = marking_script.to_str().unwrap();
    let marking_edit_script = fixture("marking_edit.json");
    let marking_edit_script = marking_edit_script.to_str().unwrap();
    let marking_more_script = fixture("marking_more.json");
    let marking_more_script = marking_more_script.to_str().unwrap();
    let marking_highlighter_script = fixture("marking_highlighter.json");
    let marking_highlighter_script = marking_highlighter_script.to_str().unwrap();
    let marking_custom_script = fixture("marking_custom.json");
    let marking_custom_script = marking_custom_script.to_str().unwrap();
    type Case<'a> = (
        &'static str,
        &'a [&'a str],
        &'static str,
        &'static str,
        &'static str,
    );
    let cases: &[Case<'_>] = &[
        ("default-1x", &[], "1300x800", "1", "0.8"),
        (
            "reference-1x",
            &["--scene", "reference"],
            "1300x800",
            "1",
            "0.8",
        ),
        (
            "reference-2x",
            &["--scene", "reference"],
            "1300x800",
            "2",
            "0.8",
        ),
        ("female-1x", &["--scene", "female"], "1300x800", "1", "0.8"),
        ("small-1x", &["--scene", "reference"], "900x600", "1", "0.8"),
        (
            "collapsed-1x",
            &[
                "--scene",
                "reference",
                "--script",
                collapsed_script,
                "--crop",
                "window-root",
            ],
            "1300x800",
            "1",
            "1.2",
        ),
        (
            "collapsed-2x",
            &[
                "--scene",
                "reference",
                "--script",
                collapsed_script,
                "--crop",
                "window-root",
            ],
            "1300x800",
            "2",
            "1.2",
        ),
        (
            "collapsed-square-1x",
            &[
                "--scene",
                "reference",
                "--script",
                collapsed_script,
                "--crop",
                "window-root",
            ],
            "1000x1000",
            "1",
            "1.2",
        ),
        (
            "settings-1x",
            &["--scene", "reference", "--script", settings_script],
            "1300x800",
            "1",
            "0.8",
        ),
        (
            "settings-small-1x",
            &["--scene", "reference", "--script", settings_script],
            "900x600",
            "1",
            "0.8",
        ),
        (
            "settings-large-small-1x",
            &["--scene", "reference", "--script", settings_large_script],
            "900x600",
            "1",
            "0.9",
        ),
        (
            "traditional-1x",
            &["--scene", "reference", "--script", traditional_script],
            "1300x800",
            "1",
            "1.2",
        ),
        (
            "settings-chart-1x",
            &["--scene", "reference", "--script", settings_chart_script],
            "1300x800",
            "1",
            "0.8",
        ),
        (
            "settings-about-1x",
            &["--scene", "reference", "--script", settings_about_script],
            "1300x800",
            "1",
            "0.8",
        ),
        (
            "help-guide-1x",
            &["--scene", "reference", "--script", help_guide_script],
            "1300x800",
            "1",
            "1.1",
        ),
        (
            "update-available-1x",
            &["--scene", "update-available"],
            "1300x800",
            "1",
            "0.8",
        ),
        (
            "flying-1x",
            &["--scene", "reference", "--script", flying_script],
            "1300x800",
            "1",
            "0.8",
        ),
        (
            "picker-1x",
            &["--scene", "reference", "--script", script],
            "1300x800",
            "1",
            "0.85",
        ),
        (
            // 年龄区间会跨宫移动，因此比较动画结束后的几何位置。
            "interaction-1x",
            &["--scene", "reference", "--script", script],
            "1300x800",
            "1",
            "2.0",
        ),
        (
            "interaction-2x",
            &["--scene", "reference", "--script", script],
            "1300x800",
            "2",
            "2.0",
        ),
        // 控件稳定且指针移开后再截图，避免悬停反馈影响基准比较。
        (
            "visibility-hidden-1x",
            &["--scene", "reference", "--script", visibility_script],
            "1300x800",
            "1",
            "0.8",
        ),
        (
            "visibility-mixed-1x",
            &["--scene", "reference", "--script", visibility_script],
            "1300x800",
            "1",
            "1.3",
        ),
        (
            "marking-palette-1x",
            &["--scene", "reference", "--script", marking_script],
            "1300x800",
            "1",
            "0.32",
        ),
        (
            "marking-custom-1x",
            &["--scene", "reference", "--script", marking_custom_script],
            "1300x800",
            "1",
            "0.8",
        ),
        (
            "marking-1x",
            &["--scene", "reference", "--script", marking_script],
            "1300x800",
            "1",
            "3.8",
        ),
        (
            "marking-2x",
            &["--scene", "reference", "--script", marking_script],
            "1300x800",
            "2",
            "3.8",
        ),
        (
            "marking-selected-ellipse-1x",
            &["--scene", "reference", "--script", marking_script],
            "1300x800",
            "1",
            "1.5",
        ),
        (
            "marking-selected-arrow-1x",
            &["--scene", "reference", "--script", marking_script],
            "1300x800",
            "1",
            "2.3",
        ),
        (
            "marking-text-edit-1x",
            &["--scene", "reference", "--script", marking_edit_script],
            "1300x800",
            "1",
            "2.1",
        ),
        (
            "marking-more-1x",
            &["--scene", "reference", "--script", marking_more_script],
            "1300x800",
            "1",
            "1.2",
        ),
        (
            "marking-highlighter-1x",
            &[
                "--scene",
                "reference",
                "--script",
                marking_highlighter_script,
            ],
            "1300x800",
            "1",
            "1.2",
        ),
    ];
    for (name, flags, size, scale, time) in cases {
        let actual = output.path().join(format!("{name}.png"));
        let mut args = flags.to_vec();
        args.extend([
            "--snap",
            actual.to_str().unwrap(),
            "--at",
            time,
            "--size",
            size,
            "--scale",
            scale,
        ]);
        run(binary, &args);
        println!("{name}");
        let expected = Image::read(&fixture(&format!("{name}.png")));
        let actual = Image::read(&actual);
        let result = compare(&expected, &actual);
        result.unwrap_or_else(|error| panic!("{name}: {error}"));
    }
}

#[test]
#[ignore = "requires a GPU adapter for headless screenshots"]
/// 验证设置颜色输入文字垂直居中。
fn settings_color_input_text_is_vertically_centered() {
    let binary = Path::new(env!("CARGO_BIN_EXE_zhiji"));
    let output = tempfile::tempdir().unwrap();
    let script = fixture("interaction.json");
    for scale in [1usize, 2] {
        let scale_arg = scale.to_string();
        for (name, key, left, right) in [
            ("颜色值", "color-hex", 21, 5),
            ("不透明度", "color-opacity-text", 4, 18),
        ] {
            let actual = output.path().join(format!("{key}-{scale}x.png"));
            run(
                binary,
                &[
                    "--scene",
                    "reference",
                    "--script",
                    script.to_str().unwrap(),
                    "--crop",
                    key,
                    "--snap",
                    actual.to_str().unwrap(),
                    "--at",
                    "0.85",
                    "--size",
                    "1300x800",
                    "--scale",
                    &scale_arg,
                ],
            );
            let image = Image::read(&actual);
            let (x1, x2, y1, y2) = (
                left * scale,
                image.width as usize - right * scale,
                0,
                image.height as usize,
            );
            let ink_rows: Vec<_> = (y1 + scale..y2 - scale)
                .filter(|&y| {
                    (x1..x2).any(|x| {
                        let i = (y * image.width as usize + x) * 4;
                        image.rgba[i..i + 3].iter().all(|channel| *channel < 150)
                    })
                })
                .collect();
            let first = *ink_rows
                .first()
                .unwrap_or_else(|| panic!("{name}文字未显示"));
            let last = *ink_rows.last().unwrap();
            let offset = (first as f32 + last as f32 + 1.0 - (y1 + y2) as f32) / 2.0;
            assert!(
                offset.abs() <= 2.5 * scale as f32,
                "{name}文字垂直偏移 {offset} 像素（{scale} 倍屏）"
            );
        }
    }
}

#[test]
#[ignore = "requires a GPU adapter for headless screenshots"]
/// 验证颜色输入光标可见。
fn color_inputs_show_a_visible_caret() {
    use serde_json::json;

    let binary = Path::new(env!("CARGO_BIN_EXE_zhiji"));
    let output = tempfile::tempdir().unwrap();
    let tap = |t, key| json!({"t": t, "type": "tap", "key": key});
    let cases = [
        (
            "color-hex",
            vec![
                tap(0.1, "open-settings"),
                tap(0.2, "settings-category-appearance"),
                tap(0.3, "color-0"),
            ],
        ),
        (
            "color-opacity-text",
            vec![
                tap(0.1, "open-settings"),
                tap(0.2, "settings-category-appearance"),
                tap(0.3, "color-0"),
            ],
        ),
        (
            "color-channel-0",
            vec![
                tap(0.1, "open-settings"),
                tap(0.2, "settings-category-appearance"),
                tap(0.3, "color-0"),
                tap(0.5, "color-format"),
                tap(0.6, "color-format-rgb"),
            ],
        ),
        (
            "color-channel-1",
            vec![
                tap(0.1, "open-settings"),
                tap(0.2, "settings-category-appearance"),
                tap(0.3, "color-0"),
                tap(0.5, "color-format"),
                tap(0.6, "color-format-hsb"),
            ],
        ),
    ];
    for (key, mut events) in cases {
        events.push(tap(0.9, key));
        let script = output.path().join(format!("{key}.json"));
        std::fs::write(
            &script,
            serde_json::to_vec(&json!({"events": events})).unwrap(),
        )
        .unwrap();
        let mut images = Vec::new();
        for time in ["0.8", "1.0"] {
            let path = output.path().join(format!("{key}-{time}.png"));
            run(
                binary,
                &[
                    "--scene",
                    "reference",
                    "--script",
                    script.to_str().unwrap(),
                    "--crop",
                    key,
                    "--snap",
                    path.to_str().unwrap(),
                    "--at",
                    time,
                    "--size",
                    "1300x800",
                    "--scale",
                    "1",
                ],
            );
            images.push(Image::read(&path));
        }
        assert_eq!(
            (images[0].width, images[0].height),
            (images[1].width, images[1].height)
        );
        let darker = images[0]
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .zip(images[1].rgba.as_chunks::<4>().0)
            .filter(|(before, after)| (0..3).all(|c| before[c] > 230 && after[c] < 160))
            .count();
        assert!(darker >= 5, "{key} 聚焦后没有可见光标：{darker} pixels");
    }
}

#[test]
#[ignore = "requires a GPU adapter for headless screenshots"]
/// 验证标注编辑光标可见。
fn annotation_editor_shows_a_visible_caret() {
    let binary = Path::new(env!("CARGO_BIN_EXE_zhiji"));
    let output = tempfile::tempdir().unwrap();
    let script = output.path().join("annotation.json");
    std::fs::write(&script, r#"{"events":[{"t":0.1,"type":"tap","key":"mark-tool-text"},{"t":0.3,"type":"tap","x":300,"y":280}]}"#).unwrap();
    for scale in [1usize, 2] {
        let mut counts = Vec::new();
        for time in ["0.4", "0.8", "1.0", "1.2", "1.4"] {
            let path = output.path().join(format!("annotation-{scale}-{time}.png"));
            run(
                binary,
                &[
                    "--scene",
                    "reference",
                    "--script",
                    script.to_str().unwrap(),
                    "--crop",
                    "annotation-text",
                    "--snap",
                    path.to_str().unwrap(),
                    "--at",
                    time,
                    "--size",
                    "1300x800",
                    "--scale",
                    &scale.to_string(),
                ],
            );
            let image = Image::read(&path);
            let mut ink = 0;
            for y in 2 * scale..image.height as usize - 2 * scale {
                for x in 2 * scale..image.width as usize - 2 * scale {
                    let i = (y * image.width as usize + x) * 4;
                    if image.rgba[i..i + 3].iter().all(|channel| *channel < 150) {
                        ink += 1;
                    }
                }
            }
            counts.push(ink);
        }
        assert!(
            counts[0] >= 5 * scale * scale,
            "空标注输入框缺少光标（{scale} 倍屏）: {counts:?}"
        );
        assert!(
            counts[1..].iter().any(|count| *count < counts[0] / 2),
            "光标未闪烁（{scale} 倍屏）: {counts:?}"
        );
    }
}

#[test]
#[ignore = "requires a GPU adapter for headless screenshots"]
/// 验证文字提交后位置保持。
fn annotation_text_keeps_its_position_when_committed() {
    let binary = Path::new(env!("CARGO_BIN_EXE_zhiji"));
    let output = tempfile::tempdir().unwrap();
    let script = output.path().join("annotation-position.json");
    std::fs::write(&script, r#"{"events":[{"t":0.1,"type":"tap","key":"mark-tool-text"},{"t":0.2,"type":"tap","key":"mark-color-0"},{"t":0.4,"type":"tap","x":300,"y":280},{"t":0.6,"type":"type","text":"北京市 Wręby 𥑤","every":0.01},{"t":1.5,"type":"key","name":"Enter"}]}"#).unwrap();
    let mut images = Vec::new();
    for time in ["1.2", "2.0"] {
        let path = output.path().join(format!("annotation-{time}.png"));
        run(
            binary,
            &[
                "--scene",
                "reference",
                "--script",
                script.to_str().unwrap(),
                "--snap",
                path.to_str().unwrap(),
                "--at",
                time,
                "--size",
                "1300x800",
                "--scale",
                "2",
            ],
        );
        images.push(Image::read(&path));
    }
    let mut edited = 0;
    let mut retained = 0;
    for y in 2 * 280..2 * 310 {
        for x in 2 * 300..2 * 500 {
            let i = (y * images[0].width as usize + x) * 4;
            if images[0].rgba[i..i + 3] == [23, 105, 210] {
                edited += 1;
                if images[1].rgba[i..i + 3] == [23, 105, 210] {
                    retained += 1;
                }
            }
        }
    }
    assert!(edited > 200, "未检测到编辑中的标注文字");
    assert!(
        retained * 100 >= edited * 97,
        "标注完成前后文字位置不一致：{retained}/{edited}"
    );
}

#[test]
#[ignore = "requires a GPU adapter for headless screenshots"]
/// 验证颜色通道字段垂直居中。
fn settings_color_channels_are_vertically_centered() {
    let binary = Path::new(env!("CARGO_BIN_EXE_zhiji"));
    let output = tempfile::tempdir().unwrap();
    for mode in ["rgb", "hsb"] {
        let script = output.path().join(format!("color-{mode}.json"));
        std::fs::write(&script, format!(r#"{{"events":[{{"t":0.1,"type":"tap","key":"open-settings"}},{{"t":0.2,"type":"tap","key":"settings-category-appearance"}},{{"t":0.3,"type":"tap","key":"color-0"}},{{"t":0.5,"type":"tap","key":"color-format"}},{{"t":0.6,"type":"tap","key":"color-format-{mode}"}}]}}"#)).unwrap();
        for scale in [1usize, 2] {
            for index in 0..3 {
                let path = output.path().join(format!("{mode}-{scale}-{index}.png"));
                run(
                    binary,
                    &[
                        "--scene",
                        "reference",
                        "--script",
                        script.to_str().unwrap(),
                        "--crop",
                        &format!("color-channel-{index}"),
                        "--snap",
                        path.to_str().unwrap(),
                        "--at",
                        "0.8",
                        "--size",
                        "1300x800",
                        "--scale",
                        &scale.to_string(),
                    ],
                );
                let image = Image::read(&path);
                let rows: Vec<_> = (2 * scale..22 * scale)
                    .filter(|&y| {
                        (3 * scale..25 * scale).any(|x| {
                            let i = (y * image.width as usize + x) * 4;
                            image.rgba[i..i + 3].iter().all(|channel| *channel < 150)
                        })
                    })
                    .collect();
                let first = *rows
                    .first()
                    .unwrap_or_else(|| panic!("{mode} 通道 {index} 文字未显示"));
                let last = *rows.last().unwrap();
                let offset = (first as f32 + last as f32 + 1.0 - 24.0 * scale as f32) / 2.0;
                assert!(
                    offset.abs() <= 2.5 * scale as f32,
                    "{mode} 通道 {index} 垂直偏移 {offset} 像素（{scale} 倍屏）"
                );
            }
        }
    }
}
