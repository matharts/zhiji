//! 内存中的排盘会话及显示选项，不持有窗口或文件资源。

use zhiji_core::{Branch, Chart, Gender, Input, Month, Stem, Transformation, calculate};

use crate::color::Palette;
use crate::transformation;

mod display;
mod snapshot;

pub(crate) use display::{Display, Flying, SelfDirection, Transformations, Visibility};
pub(crate) use snapshot::{ChartSnapshot, ChartView};

#[derive(Clone, Copy)]
pub(crate) enum Action {
    Gender(Gender),
    Stem(Stem),
    Branch(Branch),
    Ziwei(Branch),
    Month(Month),
    Hour(Branch),
    Main(Branch),
    Decade(Branch),
    Yearly(Branch),
    ClearYearly,
    Toggle(Visibility),
    ToggleAll(Transformations),
    ToggleTransformation(Transformations, Transformation),
    SetSelfDirection(SelfDirection),
}

/// 日期取数及紫占实际回填的六项参数，全部由取数入口确定。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DateChartInput {
    pub(crate) gender: Gender,
    pub(crate) stem: Stem,
    pub(crate) branch: Branch,
    pub(crate) ziwei: Branch,
    pub(crate) month: Month,
    pub(crate) hour: Branch,
}

pub(crate) struct Session {
    input: Input,
    chart: Chart,
    display: Display,
    flying: Option<Flying>,
    palette: Palette,
}

/// 重算输入；命宫变化时将大限同步到新命宫，再生成最终命盘。
fn calculate_after_input_change(input: &mut Input, previous_main: Option<Branch>) -> Chart {
    let mut chart = calculate(input);
    if chart.main != previous_main {
        input.decade = chart.main;
        chart = calculate(input);
    }
    chart
}

/// 判断年干支阴阳是否匹配；未选天干时不允许选择地支。
fn branch_matches_stem(stem: Option<Stem>, branch: Branch) -> bool {
    stem.is_some_and(|stem| stem.index() % 2 == branch.index() % 2)
}

/// 校验已选生年地支与天干的配对，未选地支是合法的部分输入。
pub(crate) fn valid_birth_branch(input: &Input) -> bool {
    input
        .branch
        .is_none_or(|branch| branch_matches_stem(input.stem, branch))
}

impl Default for Session {
    /// 用默认的部分输入创建会话。
    fn default() -> Self {
        Self::with_input(Input::default())
    }
}

impl Session {
    /// 一次性应用已校验的日期取数结果，保留显示偏好和标注所属的外层状态。
    pub(crate) fn apply_date_input(&mut self, value: DateChartInput) {
        let previous_main = self.chart.main;
        self.input.gender = value.gender;
        self.input.stem = Some(value.stem);
        self.input.branch = Some(value.branch);
        self.input.ziwei = Some(value.ziwei);
        self.input.month = Some(value.month);
        self.input.hour = Some(value.hour);
        self.input.main = None;
        self.chart = calculate_after_input_change(&mut self.input, previous_main);
        self.refresh_flying();
    }
    /// 计算初始命盘；未指定大限时以命宫作为默认大限。
    pub(crate) fn with_input(mut input: Input) -> Self {
        let chart = calculate(&input);
        if input.decade.is_none() {
            input.decade = chart.main;
        }
        Self {
            chart,
            input,
            display: Display::default(),
            flying: None,
            palette: Palette::default(),
        }
    }

    /// 从草稿各部分重建会话，保留草稿已有的显示、颜色和飞宫数据。
    pub(crate) fn restore(
        input: Input,
        display: Display,
        palette: Palette,
        flying: Option<Flying>,
    ) -> Self {
        let mut session = Self::with_input(input);
        session.display = display;
        session.palette = palette;
        session.flying = flying;
        session
    }

    /// 借用当前生效的排盘输入。
    pub(crate) fn input(&self) -> &Input {
        &self.input
    }

    /// 借用与当前输入对应的计算结果。
    pub(crate) fn chart(&self) -> &Chart {
        &self.chart
    }

    /// 借用纯显示选项。
    pub(crate) fn display(&self) -> &Display {
        &self.display
    }

    /// 替换显示选项，不重算排盘参数。
    pub(crate) fn set_display(&mut self, display: Display) {
        self.display = display;
    }

    /// 借用当前盘面配色。
    pub(crate) fn palette(&self) -> &Palette {
        &self.palette
    }

    /// 替换盘面配色，不改变计算结果。
    pub(crate) fn set_palette(&mut self, palette: Palette) {
        self.palette = palette;
    }

    /// 将会话数据组合成无需克隆的只读绘制视图。
    pub(crate) fn view(&self) -> ChartView<'_> {
        ChartView {
            chart: &self.chart,
            display: &self.display,
            palette: &self.palette,
            flying: self.flying,
        }
    }

    /// 冻结拥有独立数据的命盘快照，供后台输出使用。
    pub(crate) fn snapshot(&self) -> ChartSnapshot {
        ChartSnapshot {
            chart: self.chart.clone(),
            display: self.display.clone(),
            palette: self.palette.clone(),
            flying: self.flying,
        }
    }

    /// 提供配色编辑入口，不暴露其余会话内部状态。
    pub(crate) fn palette_mut(&mut self) -> &mut Palette {
        &mut self.palette
    }

    /// 重置排盘输入及飞宫，保留显示偏好、定位星开关和配色。
    pub(crate) fn reset_input(&mut self) {
        self.input = Input {
            show_location_stars: self.input.show_location_stars,
            ..Input::default()
        };
        self.chart = calculate(&self.input);
        self.flying = None;
    }

    /// 按新宫干刷新当前飞宫四化；原宫失去宫干时清除飞宫。
    fn refresh_flying(&mut self) {
        self.flying = self.flying.and_then(|flying| {
            self.chart
                .palaces
                .iter()
                .find(|palace| palace.branch == flying.branch)
                .and_then(|palace| palace.stem)
                .map(|stem| Flying {
                    branch: flying.branch,
                    stars: stem.transformed_stars(),
                })
        });
    }

    /// 判断该地支是否可与当前生年天干配对。
    pub(crate) fn is_branch_enabled(&self, branch: Branch) -> bool {
        branch_matches_stem(self.input.stem, branch)
    }

    /// 仅在已选年干和紫微位置后允许手动指定命宫。
    pub(crate) fn is_main_enabled(&self) -> bool {
        self.input.stem.is_some() && self.input.ziwei.is_some()
    }

    /// 应用排盘或显示动作并维护联动；返回是否产生有效状态变化。
    pub(crate) fn apply(&mut self, action: Action) -> bool {
        let previous_input = self.input.clone();
        let previous_main = self.chart.main;
        match action {
            Action::Gender(value) => self.input.gender = value,
            Action::Stem(value) => {
                if self.input.stem != Some(value) && self.input.branch.is_some() {
                    self.input.branch = Branch::from_index(value.index() % 2);
                }
                self.input.stem = Some(value);
            }
            Action::Branch(value) if self.is_branch_enabled(value) => {
                self.input.branch = Some(value)
            }
            Action::Ziwei(value) => self.input.ziwei = Some(value),
            Action::Month(value) => {
                self.input.month = Some(value);
                self.update_main_from_month_hour();
            }
            Action::Hour(value) => {
                self.input.hour = Some(value);
                self.update_main_from_month_hour();
            }
            Action::Main(value) if self.is_main_enabled() => {
                self.input.main = Some(value);
                self.input.month = None;
                self.input.hour = None;
            }
            Action::Decade(value) if self.chart.main.is_some() => self.input.decade = Some(value),
            Action::Yearly(value) if self.chart.decade.is_some() => self.input.yearly = Some(value),
            Action::ClearYearly => self.input.yearly = None,
            Action::Toggle(Visibility::LocationStars) => {
                self.input.show_location_stars = !self.input.show_location_stars;
            }
            Action::Toggle(flag) => {
                if let Some(value) = self.display.visibility_mut(flag) {
                    *value = !*value;
                    return true;
                }
                return false;
            }
            Action::ToggleAll(group) => {
                let values = self.display.transforms_mut(group);
                let checked = !values.iter().all(|value| *value);
                let changed = values.iter().any(|value| *value != checked);
                values.fill(checked);
                return changed;
            }
            Action::ToggleTransformation(group, transformation) => {
                let values = self.display.transforms_mut(group);
                let index = transformation::index(transformation);
                values[index] = !values[index];
                return true;
            }
            Action::SetSelfDirection(value) => {
                let changed = self.display.self_direction != value;
                self.display.self_direction = value;
                return changed;
            }
            _ => return false,
        }
        if previous_input == self.input {
            return false;
        }
        self.chart = calculate_after_input_change(&mut self.input, previous_main);
        self.refresh_flying();
        true
    }

    /// 月、时均已选择时按现有规则同步命宫，否则保留当前选择。
    fn update_main_from_month_hour(&mut self) {
        if let (Some(month), Some(hour)) = (self.input.month, self.input.hour) {
            let index = (i32::from(month.value()) - 1 - i32::from(hour.index()) + 2).rem_euclid(12);
            self.input.main = Branch::from_index(index as u8);
        }
    }

    /// 切换指定宫的飞化，再次点击同宫关闭；无宫干时不生成飞化。
    pub(crate) fn toggle_flying(&mut self, branch: Branch) {
        self.flying = if self.flying.is_some_and(|flying| flying.branch == branch) {
            None
        } else {
            self.chart
                .palaces
                .iter()
                .find(|palace| palace.branch == branch)
                .and_then(|palace| palace.stem)
                .map(|stem| Flying {
                    branch,
                    stars: stem.transformed_stars(),
                })
        };
    }
}

#[cfg(test)]
#[path = "../../tests/unit/session/tests.rs"]
mod tests;
