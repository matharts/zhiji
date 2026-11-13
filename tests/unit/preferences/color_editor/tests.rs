use super::*;
use crate::color::DEFAULT_COLORS;
use crate::color::Rgb;

#[test]
/// 验证面板关闭后的过期输入不改变配色。
fn stale_input_after_picker_closes_does_not_change_palette() {
    let mut editor = ColorEditor::default();
    let mut palette = Palette::default();
    editor
        .with_palette(&mut palette)
        .open_color(Transformation::A);
    editor.with_palette(&mut palette).close_color();
    editor
        .with_palette(&mut palette)
        .edit(ColorField::Hex)
        .replace_text("123456");
    editor
        .with_palette(&mut palette)
        .input_changed(ColorField::Hex, EditPhase::Change);
    assert_eq!(&palette, &Palette::default());
}
use zhiji_core::Transformation;

#[test]
/// 验证各四化颜色编辑相互隔离。
fn each_transformation_edits_its_own_color() {
    let mut editor = ColorEditor::default();
    let mut palette = Palette::default();
    for (transformation, value) in [
        (Transformation::A, "123456"),
        (Transformation::B, "234567"),
        (Transformation::C, "345678"),
        (Transformation::D, "456789"),
    ] {
        editor.with_palette(&mut palette).open_color(transformation);
        editor
            .with_palette(&mut palette)
            .edit(ColorField::Hex)
            .replace_text(value);
        editor
            .with_palette(&mut palette)
            .input_changed(ColorField::Hex, EditPhase::Change);
        editor.with_palette(&mut palette).close_color();
    }
    assert_eq!(palette.color(Transformation::A), Rgb(0x12, 0x34, 0x56));
    assert_eq!(palette.color(Transformation::B), Rgb(0x23, 0x45, 0x67));
    assert_eq!(palette.color(Transformation::C), Rgb(0x34, 0x56, 0x78));
    assert_eq!(palette.color(Transformation::D), Rgb(0x45, 0x67, 0x89));
}

#[test]
/// 验证旧拖动取消不会回滚另一个四化颜色。
fn cancelling_a_stale_drag_cannot_restore_a_different_color() {
    let mut editor = ColorEditor::default();
    let mut palette = Palette::default();
    editor
        .with_palette(&mut palette)
        .open_color(Transformation::A);
    editor
        .with_palette(&mut palette)
        .drag(Phase::Down, false, ColorDrag::Hue(0.5));
    let a_after_drag = palette.color(Transformation::A);
    let b_before = palette.color(Transformation::B);
    let b_opacity_before = palette.opacity(Transformation::B);

    editor
        .with_palette(&mut palette)
        .open_color(Transformation::B);
    editor
        .with_palette(&mut palette)
        .drag(Phase::Up, true, ColorDrag::Hue(0.7));

    assert_eq!(palette.color(Transformation::A), a_after_drag);
    assert_eq!(palette.color(Transformation::B), b_before);
    assert_eq!(palette.opacity(Transformation::B), b_opacity_before);
    assert_eq!(editor.active_color(), Some(Transformation::B));
}

#[test]
/// 验证字段切换会提交待定输入。
fn switching_fields_commits_pending_values() {
    let mut editor = ColorEditor::default();
    let mut palette = Palette::default();
    editor
        .with_palette(&mut palette)
        .open_color(Transformation::A);
    editor
        .with_palette(&mut palette)
        .select_color_format(Format::Rgb);
    editor
        .with_palette(&mut palette)
        .edit(ColorField::Channel(Channel::First))
        .replace_text("300");
    editor
        .with_palette(&mut palette)
        .input_changed(ColorField::Channel(Channel::First), EditPhase::Change);
    assert_eq!(palette.color(Transformation::A), DEFAULT_COLORS[0]);

    editor
        .with_palette(&mut palette)
        .edit(ColorField::Opacity)
        .replace_text("-20");
    editor
        .with_palette(&mut palette)
        .input_changed(ColorField::Opacity, EditPhase::Change);
    assert_eq!(palette.color(Transformation::A), Rgb(255, 139, 34));
    assert_eq!(palette.opacity(Transformation::A), 255);

    editor
        .with_palette(&mut palette)
        .edit(ColorField::Channel(Channel::Second))
        .replace_text("20");
    editor
        .with_palette(&mut palette)
        .input_changed(ColorField::Channel(Channel::Second), EditPhase::Change);
    assert_eq!(palette.color(Transformation::A), Rgb(255, 20, 34));
    assert_eq!(palette.opacity(Transformation::A), 0);
}
