use super::*;
#[test]
/// 验证剪贴板 PNG 保留尺寸及像素内容。
fn png_keeps_dimensions_and_pixel_contents() {
    let image = encode(Raster {
        width: 2,
        height: 1,
        rgba: vec![255, 0, 0, 255, 0, 0, 255, 255],
    })
    .unwrap();
    assert_eq!(image.size(), (2, 1));
    let mut reader = png::Decoder::new(std::io::Cursor::new(image.png()))
        .read_info()
        .unwrap();
    let mut output = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut output).unwrap();
    assert_eq!(
        &output[..info.buffer_size()],
        &[255, 0, 0, 255, 0, 0, 255, 255]
    );
}

#[test]
#[ignore = "requires a GPU adapter for offscreen rendering"]
/// 验证复制冻结可见命盘并排除隐藏标注。
fn copy_freezes_the_visible_chart_and_hidden_marks_are_excluded() {
    use crate::annotation::Tool;
    use crate::app::AppState;
    use rune::{Env, Phase, Runtime};
    use std::time::Duration;

    let (wake, woke) = std::sync::mpsc::channel();
    let mut state = AppState::with_input(crate::scenes::sample_input(false));
    state.set_test_waker(std::sync::Arc::new(move || {
        let _ = wake.send(());
    }));
    state
        .workspace
        .marking_mut_for_test()
        .select(Tool::Highlighter);
    state
        .workspace
        .marking_mut_for_test()
        .gesture(Phase::Down, false, [240.0, 260.0]);
    state
        .workspace
        .marking_mut_for_test()
        .gesture(Phase::Move, false, [275.0, 260.0]);
    state
        .workspace
        .marking_mut_for_test()
        .gesture(Phase::Up, false, [310.0, 260.0]);
    let marked = render::rasterize(
        crate::workspace::ImageSnapshot::from(state.workspace.session().snapshot())
            .with_annotations(state.workspace.marking_for_test().visible_marks().to_vec()),
        2,
    )
    .unwrap();
    let clean = render::rasterize(
        crate::workspace::ImageSnapshot::from(state.workspace.session().snapshot()),
        2,
    )
    .unwrap();
    assert!(
        marked
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .zip(clean.rgba.as_chunks::<4>().0)
            .any(|(after, before)| {
                before[..3] == [255, 255, 255]
                    && after[0] > 201
                    && after[0] < 255
                    && after[1] > 162
                    && after[1] < 255
                    && after[2] > 29
                    && after[2] < 255
            }),
        "the highlighter must be translucent over the chart"
    );
    let mut runtime = Runtime::new(
        state,
        crate::app::view::render,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.poll = Some(AppState::poll);
    let clipboard = rune::clipboard::memory();
    runtime.use_clipboard(&clipboard);
    runtime.frame(0.0);
    runtime.update(|state, tx| {
        state.request_copy_image(tx);
        state.workspace.marking_mut_for_test().toggle_visibility();
    });
    woke.recv_timeout(Duration::from_secs(30)).unwrap();
    runtime.frame(0.0);
    let copied = clipboard.content().unwrap().image().unwrap().clone();
    let decode = |image: &ClipboardImage| {
        let mut reader = png::Decoder::new(std::io::Cursor::new(image.png()))
            .read_info()
            .unwrap();
        let mut bytes = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut bytes).unwrap();
        bytes.truncate(info.buffer_size());
        bytes
    };
    assert_eq!(
        decode(&copied),
        marked.rgba,
        "clipboard uses the frozen visible chart"
    );
    runtime.update(|state, tx| state.request_copy_image(tx));
    woke.recv_timeout(Duration::from_secs(30)).unwrap();
    runtime.frame(0.0);
    let hidden = clipboard.content().unwrap().image().unwrap().clone();
    assert_eq!(decode(&hidden), clean.rgba, "hidden markings are excluded");
}
