//! 界面语义颜色及视图构建作用域内的昼夜模式。

use rune::Color;
use std::cell::Cell;

thread_local! { static NIGHT: Cell<bool> = const { Cell::new(false) }; }

/// 主题模式作用域；提前返回时也会恢复此前模式。
pub struct ThemeScope(bool);

impl Drop for ThemeScope {
    /// 恢复进入作用域前的线程局部主题。
    fn drop(&mut self) {
        NIGHT.with(|night| night.set(self.0));
    }
}

/// 为当前视图构建选择主题，返回负责恢复的作用域守卫。
pub fn theme_scope(night: bool) -> ThemeScope {
    ThemeScope(NIGHT.with(|current| current.replace(night)))
}

/// 查询当前线程的视图构建是否采用夜间主题。
pub fn is_night() -> bool {
    NIGHT.with(Cell::get)
}

macro_rules! tokens {
    ($($variant:ident => ($light:literal, $night:literal)),+ $(,)?) => {
        /// 控件和绘制表面共享的语义颜色。
        ///
        /// 使用处不引入原始颜色字符串；用户配色及图片数据使用 Rune 运行时颜色转换。
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum ColorToken {
            $($variant),+
        }

        impl ColorToken {
            pub const ALL: &[Self] = &[$(Self::$variant),+];

            /// 返回显式昼夜模式下的静态十六进制色值。
            pub const fn hex(self, night: bool) -> &'static str {
                match self {
                    $(Self::$variant => if night { $night } else { $light }),+
                }
            }
        }
    };
}

tokens! {
    Canvas => ("#ffffff", "#211f29"),
    Window => ("#f5f5f5", "#292630"),
    SurfaceCool => ("#f2f2f5", "#292630"),
    SurfaceTint => ("#f2f2f2", "#292630"),
    SurfaceHoverSoft => ("#f0f0f0", "#393541"),
    SurfaceHover => ("#eeeeee", "#393541"),
    SurfaceActive => ("#e9e9e9", "#393541"),
    BorderSoft => ("#e8e8e8", "#393541"),
    Divider => ("#e6e6e6", "#504a59"),
    SurfaceHoverStrong => ("#dedede", "#504a59"),
    SurfacePressed => ("#dddddd", "#504a59"),
    Border => ("#d9d9d9", "#504a59"),

    InkPrimary => ("#000000", "#ede8f1"),
    InkTitle => ("#1f1f1f", "#e2dce7"),
    InkHeading => ("#292929", "#e2dce7"),
    InkIcon => ("#303030", "#e2dce7"),
    InkBody => ("#333333", "#e2dce7"),
    InkSwitchOn => ("#343137", "#e2dce7"),
    InkSecondary => ("#555555", "#bdb5c4"),
    InkLabelMuted => ("#59595e", "#bdb5c4"),
    InkNavigation => ("#63636b", "#bdb5c4"),
    InkMuted => ("#666666", "#bdb5c4"),
    InkHint => ("#777777", "#b9afbd"),
    InkDisabled => ("#888888", "#b9afbd"),
    InkDisabledAlt => ("#89838a", "#b9afbd"),
    InkPlaceholder => ("#999999", "#b9afbd"),
    InkDisabledFaint => ("#bfbfbf", "#a69cac"),
    InkDisabledMedium => ("#b8b8b8", "#a69cac"),
    InkDisabledSoft => ("#aaaaaa", "#a69cac"),
    InkOnPrimary => ("#ffffff", "#211f29"),

    Accent => ("#593d75", "#d2b1e8"),
    AccentSurface => ("#eee8f5", "#51445e"),
    AccentSurfaceHover => ("#e4d9ef", "#60506c"),
    SwitchSurface => ("#f2f0f3", "#454047"),
    SwitchSurfaceHover => ("#ddd7df", "#56505a"),
    LegendNeutral => ("#f1eff0", "#37323d"),
    DirectionSurface => ("#f8f6f5", "#37323d"),
    DirectionSurfaceHover => ("#f0edeb", "#37323d"),
    AnnotationOn => ("#8f2937", "#e88c95"),
    AnnotationNeutral => ("#b8afb1", "#948a97"),
    AnnotationNeutralMuted => ("#a9a3a6", "#948a97"),
    Error => ("#b11f2d", "#e88c95"),
    StatusDanger => ("#cf1322", "#f38a91"),
    StatusSuccess => ("#389e0d", "#8dcf77"),
    ToggleOn => ("#706575", "#c5b8cc"),
    ToggleOff => ("#89838a", "#b9afbd"),
    SwitchTrackOff => ("#bfbfbf", "#a69cac"),

    TextInputBorder => ("#bdbdbd", "#bdbdbd"),
    CaretFocus => ("#356cb8", "#356cb8"),
    Checkerboard => ("#dedede", "#dedede"),
    AnnotationHalo => ("#c5b8c7", "#c5b8c7"),
    AnnotationSelection => ("#356cb8", "#356cb8"),
    IconDisabled => ("#c3c3c3", "#c3c3c3"),
    WeightSelected => ("#07b860", "#07b860"),
    CustomColorGold => ("#c59547", "#c59547"),
    CustomColorGreen => ("#55a884", "#55a884"),
    CustomColorBlue => ("#5686bc", "#5686bc"),
    CustomColorViolet => ("#a176ab", "#a176ab"),

    WaterElementSurface => ("#f2f2f2", "#292630"),
    WoodElementInk => ("#00e09e", "#00e09e"),
    WoodElementSurface => ("#e5fff7", "#23483d"),
    MetalElementInk => ("#e29e28", "#e29e28"),
    MetalElementSurface => ("#fcf5e9", "#483b28"),
    EarthElementInk => ("#d6a01d", "#d6a01d"),
    EarthElementSurface => ("#fcf6e9", "#483b28"),
    FireElementInk => ("#c3272b", "#cd8585"),
    FireElementSurface => ("#fbeaea", "#482b31"),

    AnnotationRed => ("#b11f2d", "#b11f2d"),
    AnnotationOrange => ("#c17b20", "#c17b20"),
    AnnotationYellow => ("#c9a21d", "#c9a21d"),
    AnnotationGreen => ("#318441", "#318441"),
    AnnotationBlue => ("#2673b9", "#2673b9"),
    AnnotationBlack => ("#333333", "#333333"),
    AnnotationIndigo => ("#1769d2", "#1769d2"),
    AnnotationTeal => ("#008387", "#008387"),
    AnnotationJade => ("#508000", "#508000"),
    AnnotationViolet => ("#8938b2", "#8938b2"),
    AnnotationGold => ("#a96600", "#a96600"),
    AnnotationGraphite => ("#343b48", "#343b48"),

    ChartTransform => ("#a51d25", "#cd8585"),
    ChartInk => ("#000000", "#e8e3ed"),
    ChartGrid => ("#000000", "#aaa1ad"),
    ChartPaper => ("#ffffff", "#1b1921"),
    ChartFlyingSurface => ("#f0f0f4", "#3c3744"),
    ChartDecadeSurface => ("#fffbf0", "#302d34"),
    ChartZiwei => ("#7e1671", "#df9bd7"),
    ChartNeutralStar => ("#808080", "#b6b0bd"),
    ChartLiteraryStar => ("#806d9e", "#c2b2dc"),
    ChartOnStrong => ("#ffffff", "#ffffff"),
    SealInk => ("#c3272b", "#d98b8a"),
}

impl ColorToken {
    /// 按显式昼夜模式解析颜色，不依赖线程局部主题。
    pub fn resolve(self, night: bool) -> Color {
        rune::color::hex(self.hex(night)).expect("color tokens contain valid hexadecimal colors")
    }
}

/// 按当前视图构建作用域解析语义颜色。
pub fn color(token: ColorToken) -> Color {
    token.resolve(is_night())
}

#[cfg(test)]
mod tests {
    use super::ColorToken;

    #[test]
    /// 验证所有语义颜色在昼夜模式下都可解析为有效颜色。
    fn every_token_has_valid_light_and_night_colors() {
        for token in ColorToken::ALL {
            for night in [false, true] {
                assert!(
                    rune::color::hex(token.hex(night)).is_some(),
                    "{token:?} has an invalid {} color",
                    if night { "night" } else { "light" }
                );
            }
        }
    }

    #[test]
    /// 验证既有昼夜配色对应关系，避免主题整理改变视觉基准。
    fn established_light_night_pairs_are_preserved() {
        assert_eq!(ColorToken::Canvas.hex(true), "#211f29");
        assert_eq!(ColorToken::SurfaceCool.hex(true), "#292630");
        assert_eq!(ColorToken::StatusSuccess.hex(true), "#8dcf77");
        assert_eq!(ColorToken::ChartInk.hex(true), "#e8e3ed");
        assert_eq!(ColorToken::ChartGrid.hex(true), "#aaa1ad");
        assert_eq!(ColorToken::SealInk.hex(true), "#d98b8a");
    }
}
