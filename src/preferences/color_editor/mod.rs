//! 飞宫配色的临时状态、字段提交和可取消拖动事务。

pub(crate) mod view;

use crate::color::Palette;
use crate::transformation;
use rune::{Phase, Pixels, TextEdit};
use zhiji_core::Transformation;
mod picker;

pub(crate) use picker::Format;
use picker::{DragSnapshot, PendingEdit, Picker};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColorField {
    Hex,
    Channel(Channel),
    Opacity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Channel {
    First,
    Second,
    Third,
}

impl Channel {
    pub(crate) const ALL: [Self; 3] = [Self::First, Self::Second, Self::Third];

    /// 返回三个颜色通道的固定数组下标。
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::First => 0,
            Self::Second => 1,
            Self::Third => 2,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditPhase {
    Change,
    Commit,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum FormatMenuAction {
    Toggle,
    Previous,
    Next,
    Select,
    Dismiss,
}

pub(crate) enum ColorDrag {
    Hue(f32),
    Plane(f32, f32),
    Opacity(f32),
}

#[derive(Clone, Copy)]
pub(crate) enum ColorSlider {
    Hue,
    Opacity,
}

pub(crate) struct ColorEditor {
    picker: Option<ActivePicker>,
    formats: [Format; 4],
    hovered_field: Option<ColorField>,
    inactive_edit: TextEdit,
}

struct ActivePicker {
    transformation: Transformation,
    state: Picker,
    drag: Option<DragSnapshot>,
}

impl Default for ColorEditor {
    /// 创建关闭的编辑器，各四化默认记忆 HEX 格式。
    fn default() -> Self {
        Self {
            picker: None,
            formats: [Format::Hex; 4],
            hovered_field: None,
            inactive_edit: TextEdit::new(""),
        }
    }
}

impl ColorEditor {
    /// 借用活动调色器所需的只读视图，不泄漏内部可变状态。
    pub(crate) fn picker(&self) -> Option<PickerView<'_>> {
        self.picker.as_ref().map(|picker| PickerView {
            transformation: picker.transformation,
            format: picker.state.format,
            format_menu_open: picker.state.format_menu_open,
            format_highlight: picker.state.format_highlight,
            hue: picker.state.hue,
            saturation: picker.state.saturation,
            value: picker.state.value,
            edit: &picker.state.edit,
            channel_edits: &picker.state.channel_edits,
            opacity: picker.state.opacity,
            opacity_edit: &picker.state.opacity_edit,
            plane: picker.state.pixels.plane(),
            hue_strip: picker.state.pixels.hue_strip(),
        })
    }

    /// 返回正在编辑的四化颜色，面板关闭时为空。
    pub(crate) fn active_color(&self) -> Option<Transformation> {
        self.picker.as_ref().map(|picker| picker.transformation)
    }

    /// 读取当前悬停字段，用于显示数值步进器。
    pub(crate) fn hovered_field(&self) -> Option<ColorField> {
        self.hovered_field
    }

    /// 更新悬停字段，旧字段离开不能清除已经移入的新字段。
    pub(crate) fn set_field_hover(&mut self, field: ColorField, inside: bool) {
        if inside {
            self.hovered_field = Some(field);
        } else if self.hovered_field == Some(field) {
            self.hovered_field = None;
        }
    }

    /// 将临时编辑器与会话有效配色绑定为同一生命周期的编辑事务。
    pub(crate) fn with_palette<'a>(&'a mut self, palette: &'a mut Palette) -> ColorEdit<'a> {
        ColorEdit {
            editor: self,
            palette,
        }
    }
}

/// 渲染活动调色器所需的不可变借用数据。
pub(crate) struct PickerView<'a> {
    pub(crate) transformation: Transformation,
    pub(crate) format: Format,
    pub(crate) format_menu_open: bool,
    pub(crate) format_highlight: Format,
    pub(crate) hue: f32,
    pub(crate) saturation: f32,
    pub(crate) value: f32,
    pub(crate) edit: &'a TextEdit,
    pub(crate) channel_edits: &'a [TextEdit; 3],
    pub(crate) opacity: u8,
    pub(crate) opacity_edit: &'a TextEdit,
    pub(crate) plane: &'a Pixels,
    pub(crate) hue_strip: &'a Pixels,
}

/// 界面编辑状态与会话有效配色的临时绑定。
pub(crate) struct ColorEdit<'a> {
    editor: &'a mut ColorEditor,
    palette: &'a mut Palette,
}

impl<'a> ColorEdit<'a> {
    /// 准备字段提交关系后返回文本编辑状态；无活动面板时使用隔离空编辑器。
    pub(crate) fn edit(mut self, field: ColorField) -> &'a mut TextEdit {
        self.prepare_color_field(field);
        let editor = self.editor;
        match (&mut editor.picker, field) {
            (Some(picker), ColorField::Hex) => &mut picker.state.edit,
            (Some(picker), ColorField::Channel(channel)) => {
                &mut picker.state.channel_edits[channel.index()]
            }
            (Some(picker), ColorField::Opacity) => &mut picker.state.opacity_edit,
            (None, _) => &mut editor.inactive_edit,
        }
    }

    /// 按字段和编辑阶段选择即时解析或最终提交。
    pub(crate) fn input_changed(&mut self, field: ColorField, phase: EditPhase) {
        match field {
            ColorField::Hex => self.update_color_text(),
            ColorField::Channel(channel) => self.update_color_channel(channel, phase),
            ColorField::Opacity => self.update_opacity_text(phase),
        }
    }

    /// 对数值通道和透明度执行步进，HEX 字段不支持步进。
    pub(crate) fn step(&mut self, field: ColorField, delta: i32) {
        match field {
            ColorField::Channel(channel) => self.step_color_channel(channel, delta),
            ColorField::Opacity => self.step_color_opacity(delta),
            ColorField::Hex => {}
        }
    }

    /// 将滑条无障碍步进按角度或百分比换算为归一化变化。
    pub(crate) fn step_slider(&mut self, slider: ColorSlider, delta: i32) {
        let Some(picker) = &self.editor.picker else {
            return;
        };
        // 无障碍层根据滑条显示值把绝对目标值转换为步进增量。
        match slider {
            ColorSlider::Hue => self.update_color_hue(picker.state.hue + delta as f32 / 360.0),
            ColorSlider::Opacity => self.update_color_opacity(
                f32::from(picker.state.opacity) / 255.0 + delta as f32 / 100.0,
            ),
        }
    }

    /// 优先关闭格式菜单，再提交并关闭颜色面板。
    pub(crate) fn dismiss(&mut self) -> bool {
        let Some(picker) = &mut self.editor.picker else {
            return false;
        };
        if picker.state.format_menu_open {
            picker.state.format_menu_open = false;
        } else {
            self.close_color();
        }
        true
    }

    /// 新手势开始前提交待定字段并关闭格式菜单。
    pub(crate) fn prepare_interaction(&mut self) {
        self.commit_pending_color();
        if let Some(picker) = &mut self.editor.picker {
            picker.state.format_menu_open = false;
        }
    }

    /// 处理格式菜单开关、循环导航、选择和取消，返回是否消费动作。
    pub(crate) fn format_menu_action(&mut self, action: FormatMenuAction) -> bool {
        if action != FormatMenuAction::Dismiss {
            self.commit_pending_color();
        }
        let Some(picker) = &mut self.editor.picker else {
            return false;
        };
        match action {
            FormatMenuAction::Toggle => {
                picker.state.format_menu_open = !picker.state.format_menu_open;
                picker.state.format_highlight = picker.state.format;
            }
            FormatMenuAction::Previous | FormatMenuAction::Next => {
                if picker.state.format_menu_open {
                    let delta = if action == FormatMenuAction::Previous {
                        -1
                    } else {
                        1
                    };
                    picker.state.format_highlight = picker.state.format_highlight.moved(delta);
                } else {
                    picker.state.format_menu_open = true;
                    picker.state.format_highlight = picker.state.format;
                }
            }
            FormatMenuAction::Select if picker.state.format_menu_open => {
                let format = picker.state.format_highlight;
                self.select_color_format(format);
            }
            FormatMenuAction::Select => {
                picker.state.format_menu_open = true;
                picker.state.format_highlight = picker.state.format;
            }
            FormatMenuAction::Dismiss if picker.state.format_menu_open => {
                picker.state.format_menu_open = false;
            }
            FormatMenuAction::Dismiss => return false,
        }
        true
    }

    /// 打开指定四化的颜色编辑，再次点击同一色样则提交并关闭。
    pub(crate) fn open_color(&mut self, transformation: Transformation) {
        if self
            .editor
            .picker
            .as_ref()
            .is_some_and(|picker| picker.transformation == transformation)
        {
            self.close_color();
            return;
        }
        self.commit_pending_color();
        self.editor.hovered_field = None;
        self.editor.picker = Some(ActivePicker {
            transformation,
            drag: None,
            state: Picker::new(
                self.palette.color(transformation),
                self.palette.opacity(transformation),
                self.editor.formats[transformation::index(transformation)],
            ),
        });
    }

    /// 提交待定字段后关闭调色器并清除悬停状态。
    pub(crate) fn close_color(&mut self) {
        self.commit_pending_color();
        self.editor.picker = None;
        self.editor.hovered_field = None;
    }

    /// 将暂时无法即时应用的通道或透明度文本归一化并提交。
    fn commit_pending_color(&mut self) {
        if let Some(picker) = &mut self.editor.picker {
            match picker.state.pending_edit.take() {
                Some(PendingEdit::Channel(channel)) => {
                    if let Some(color) = picker.state.commit_channel(channel.index()) {
                        self.palette.set_color(picker.transformation, color);
                        self.palette
                            .set_opacity(picker.transformation, picker.state.opacity);
                    }
                }
                Some(PendingEdit::Opacity) => {
                    self.palette
                        .set_opacity(picker.transformation, picker.state.commit_opacity_text());
                }
                None => {}
            }
        }
    }

    /// 即将编辑另一个字段时先收尾旧字段，避免状态相互覆盖。
    fn commit_other_pending_color(&mut self, edit: PendingEdit) {
        if self
            .editor
            .picker
            .as_ref()
            .and_then(|picker| picker.state.pending_edit)
            .is_some_and(|pending| pending != edit)
        {
            self.commit_pending_color();
        }
    }

    /// 失焦切换字段时提交旧输入，新字段获焦时收起格式菜单。
    fn prepare_color_field(&mut self, field: ColorField) {
        let Some(picker) = &self.editor.picker else {
            return;
        };
        let focused = match field {
            ColorField::Hex => picker.state.edit.focused,
            ColorField::Channel(channel) => picker.state.channel_edits[channel.index()].focused,
            ColorField::Opacity => picker.state.opacity_edit.focused,
        };
        let pending = match picker.state.pending_edit {
            Some(PendingEdit::Channel(channel)) => Some((
                ColorField::Channel(channel),
                picker.state.channel_edits[channel.index()].focused,
            )),
            Some(PendingEdit::Opacity) => {
                Some((ColorField::Opacity, picker.state.opacity_edit.focused))
            }
            None => None,
        };
        if pending.is_some_and(|(pending_field, is_focused)| pending_field != field && !is_focused)
        {
            self.commit_pending_color();
        }
        if focused && let Some(picker) = &mut self.editor.picker {
            picker.state.format_menu_open = false;
        }
    }

    /// 恢复默认配色并关闭编辑器，不保留未提交文本。
    pub(crate) fn reset_colors(&mut self) {
        *self.palette = Palette::default();
        self.editor.picker = None;
        self.editor.hovered_field = None;
    }

    /// 完成旧数值输入后将有效 HEX 文本应用到会话配色。
    fn update_color_text(&mut self) {
        self.commit_pending_color();
        if let Some(picker) = &mut self.editor.picker
            && let Some((color, opacity)) = picker.state.apply_text()
        {
            self.palette.set_color(picker.transformation, color);
            self.palette.set_opacity(picker.transformation, opacity);
        }
    }

    /// 提交输入并记忆当前四化的格式选择。
    pub(crate) fn select_color_format(&mut self, format: Format) {
        self.commit_pending_color();
        self.editor.hovered_field = None;
        if let Some(picker) = &mut self.editor.picker {
            self.editor.formats[transformation::index(picker.transformation)] = format;
            picker.state.set_format(format);
        }
    }

    /// 应用或暂存通道输入，仅有效结果更新会话颜色。
    fn update_color_channel(&mut self, channel: Channel, phase: EditPhase) {
        self.commit_other_pending_color(PendingEdit::Channel(channel));
        if let Some(picker) = &mut self.editor.picker {
            let color = match phase {
                EditPhase::Change => picker.state.apply_channel(channel.index()),
                EditPhase::Commit => picker.state.commit_channel(channel.index()),
            };
            picker.state.pending_edit = if phase == EditPhase::Change && color.is_none() {
                Some(PendingEdit::Channel(channel))
            } else {
                None
            };
            if let Some(color) = color {
                self.palette.set_color(picker.transformation, color);
                self.palette
                    .set_opacity(picker.transformation, picker.state.opacity);
            }
        }
    }

    /// 提交其他字段后步进指定通道，并清除该通道的待提交标记。
    fn step_color_channel(&mut self, channel: Channel, delta: i32) {
        self.commit_other_pending_color(PendingEdit::Channel(channel));
        if let Some(picker) = &mut self.editor.picker
            && let Some(color) = picker.state.step_channel(channel.index(), delta)
        {
            self.palette.set_color(picker.transformation, color);
            self.palette
                .set_opacity(picker.transformation, picker.state.opacity);
            picker.state.pending_edit = None;
        }
    }

    /// 提交待定文本后更新色相和有效 RGB。
    fn update_color_hue(&mut self, hue: f32) {
        self.commit_pending_color();
        if let Some(picker) = &mut self.editor.picker {
            self.palette
                .set_color(picker.transformation, picker.state.set_hue(hue));
        }
    }

    /// 提交待定文本后更新饱和度、明度和有效 RGB。
    fn update_color_sv(&mut self, saturation: f32, value: f32) {
        self.commit_pending_color();
        if let Some(picker) = &mut self.editor.picker {
            self.palette.set_color(
                picker.transformation,
                picker.state.set_sv(saturation, value),
            );
        }
    }

    /// 提交待定文本后更新归一化不透明度。
    fn update_color_opacity(&mut self, opacity: f32) {
        self.commit_pending_color();
        if let Some(picker) = &mut self.editor.picker {
            self.palette
                .set_opacity(picker.transformation, picker.state.set_opacity(opacity));
        }
    }

    /// 按编辑阶段应用百分比文本，无效即时输入保留为待提交状态。
    fn update_opacity_text(&mut self, phase: EditPhase) {
        self.commit_other_pending_color(PendingEdit::Opacity);
        if let Some(picker) = &mut self.editor.picker {
            let opacity = match phase {
                EditPhase::Change => picker.state.apply_opacity_text(),
                EditPhase::Commit => Some(picker.state.commit_opacity_text()),
            };
            picker.state.pending_edit =
                (phase == EditPhase::Change && opacity.is_none()).then_some(PendingEdit::Opacity);
            if let Some(opacity) = opacity {
                self.palette.set_opacity(picker.transformation, opacity);
            }
        }
    }

    /// 按百分比步进会话不透明度并结束待提交状态。
    fn step_color_opacity(&mut self, delta: i32) {
        self.commit_other_pending_color(PendingEdit::Opacity);
        if let Some(picker) = &mut self.editor.picker {
            self.palette
                .set_opacity(picker.transformation, picker.state.step_opacity(delta));
            picker.state.pending_edit = None;
        }
    }

    /// 拖动开始时捕获完整颜色状态，取消时回滚，正常结束时保留新值。
    pub(crate) fn drag(&mut self, phase: Phase, cancelled: bool, value: ColorDrag) {
        if phase == Phase::Down {
            self.prepare_interaction();
            if let Some(picker) = &mut self.editor.picker {
                picker.drag = Some(
                    picker
                        .state
                        .drag_snapshot(self.palette.color(picker.transformation)),
                );
            }
        }
        if cancelled {
            if let Some(picker) = &mut self.editor.picker
                && let Some(snapshot) = picker.drag.take()
            {
                self.palette
                    .set_color(picker.transformation, snapshot.color);
                self.palette
                    .set_opacity(picker.transformation, snapshot.opacity);
                picker.state.restore_drag(snapshot);
            }
        } else {
            match value {
                ColorDrag::Hue(hue) => self.update_color_hue(hue),
                ColorDrag::Plane(saturation, value) => self.update_color_sv(saturation, value),
                ColorDrag::Opacity(opacity) => self.update_color_opacity(opacity),
            }
            if phase == Phase::Up
                && let Some(picker) = &mut self.editor.picker
            {
                picker.drag = None;
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/preferences/color_editor/tests.rs"]
mod tests;
