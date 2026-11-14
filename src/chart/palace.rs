//! 单宫背景、文字、星曜及四化标记的绘制。

use zhiji_core::{Branch, Palace, Transformation};
use zhiji_ui::ColorToken;

use super::motion::LabelMotion;
use super::{
    BIRTH_WEIGHT, BLACK, DETAIL_INK, Drawing, FONT, PAPER, SELF_WEIGHT, TRANSFORM_INK, WHITE,
    geometry, grid,
};
use crate::color::Palette;
use crate::session::{Display, Flying};

pub(super) struct Painter<'a> {
    out: &'a mut Drawing,
    display: &'a Display,
    palette: &'a Palette,
    flying: Option<Flying>,
    baseline_offset: f32,
}

impl<'a> Painter<'a> {
    /// 绑定本次绘制的显示选项、配色、飞宫和基线修正。
    pub(super) fn new(
        out: &'a mut Drawing,
        display: &'a Display,
        palette: &'a Palette,
        flying: Option<Flying>,
        baseline_offset: f32,
    ) -> Self {
        Self {
            out,
            display,
            palette,
            flying,
            baseline_offset,
        }
    }

    /// 在指定宫位绘制背景及框内信息。
    pub(super) fn frame(&mut self, index: usize, palace: &Palace, decade: Option<Branch>) {
        let bounds = geometry::palace_rect(index);
        self.frame_and_labels(palace, decade, bounds);
    }

    /// 绘制全盘共享主格线，避免相邻宫重复描边。
    pub(super) fn grid(&mut self) {
        grid::main(|rule| {
            self.out
                .rule(rule, [0.0, 0.0], geometry::GRID_WIDTH, super::GRID_INK)
        });
    }

    /// 组合单宫星曜、大限流年和向心信息。
    pub(super) fn contents(&mut self, index: usize, palace: &Palace, opposite_inward: bool) {
        let bounds = geometry::palace_rect(index);
        self.stars(index, palace, [bounds[0], bounds[1]]);
        self.period_and_inward(index, palace, [bounds[0], bounds[1]], opposite_inward);
    }

    /// 按显示开关绘制宫干支、宫名、来因印及必要的内部细分线。
    fn frame_and_labels(&mut self, palace: &Palace, decade: Option<Branch>, bounds: [f32; 4]) {
        let [x, y, width, height] = bounds;
        let Self {
            out,
            display,
            flying,
            baseline_offset,
            ..
        } = self;
        let flying = *flying;
        let baseline_offset = *baseline_offset;
        let fill = if flying.is_some_and(|flying| flying.branch == palace.branch) {
            ColorToken::ChartFlyingSurface
        } else if decade == Some(palace.branch) {
            ColorToken::ChartDecadeSurface
        } else {
            PAPER
        };
        out.rect(x, y, width, height, Some(fill), 0.0);
        if display.laiyin && palace.is_laiyin {
            out.seals.push([x, y]);
        }
        if let Some(stem) = palace.stem.filter(|_| display.stem) {
            let offset = if display.branch { 108.0 } else { 126.0 };
            out.label(
                crate::i18n::chart_t(stem.name()),
                [x + 4.25, y + offset + baseline_offset],
                FONT,
                600.0,
                BLACK,
                false,
            );
        }
        if display.branch {
            out.label(
                crate::i18n::chart_t(palace.branch.name()),
                [x + 4.25, y + 126.0 + baseline_offset],
                FONT,
                600.0,
                BLACK,
                false,
            );
        }
        if let Some(name) = palace.name.filter(|_| display.palace) {
            let localized = crate::i18n::chart_t(name);
            for (i, character) in localized.chars().enumerate() {
                out.label(
                    character.to_string(),
                    [x + 129.25, y + 108.0 + baseline_offset + i as f32 * 18.0],
                    FONT,
                    600.0,
                    BLACK,
                    false,
                )
                .identify(
                    format!("palace-name-{}-{i}", palace.branch.index()),
                    LabelMotion::Palace(palace.branch, "natal", name),
                );
            }
        }
        if let Some(name) = palace.decade_name.filter(|_| display.palace) {
            // Windows 低 DPI 窗口补齐与宫职、星曜相同的基线修正；
            // macOS/Linux 保留现有参考位置，2× 复制图片也不受影响。
            let offset = if cfg!(target_os = "windows") {
                baseline_offset
            } else {
                0.0
            };
            out.label(
                crate::i18n::chart_t(name),
                [x + 75.0, y + 102.90625 + offset],
                FONT,
                600.0,
                BLACK,
                true,
            )
            .identify(
                format!("decade-name-{}", palace.branch.index()),
                LabelMotion::Palace(palace.branch, "decade", name),
            );
        }
        if let Some((start, end)) = palace.decade_range.filter(|_| display.palace)
            && palace.name.is_some()
        {
            out.label(
                format!("{start}~{end}"),
                [x + 75.0, y + 130.84375],
                12.0,
                600.0,
                BLACK,
                true,
            )
            .identify(format!("decade-range-{start}-{end}"), LabelMotion::Travel);
        }
        let sections = grid::Sections {
            left: display.stem
                && display.branch
                && display.palace
                && palace.stem.is_some()
                && palace.name.is_some(),
            upper: display.palace && palace.decade_name.is_some(),
            lower: display.palace && palace.decade_range.is_some() && palace.name.is_some(),
            right: display.palace && palace.name.is_some(),
        };
        grid::internal(sections, |rule| {
            out.rule(rule, [x, y], geometry::DETAIL_WIDTH, DETAIL_INK)
        });
    }

    /// 按参考顺序安放星名、生年四化、飞宫底色及离心箭头。
    fn stars(&mut self, index: usize, palace: &Palace, origin: [f32; 2]) {
        let [x, y] = origin;
        let Self {
            out,
            display,
            palette,
            flying,
            baseline_offset,
        } = self;
        let flying = *flying;
        let baseline_offset = *baseline_offset;
        use geometry::{ArrowKind, OutwardSpacing, outward};
        let outward_total = palace
            .stars
            .iter()
            .filter(|star| star.outward.is_some())
            .count();
        let mut outward_ordinal = 0;
        for (star_index, star) in palace.stars.iter().enumerate() {
            let local_x = 127.0 - 19.0 * star_index as f32;
            let star_origin = [x + local_x, y + 4.0];
            let fly_transformation =
                flying.and_then(|selection| selection.transformation_for(star.name));
            if let Some(value) = fly_transformation {
                let fill = palette.color(value);
                let fill = rune::Color::from(fill).alpha(f32::from(palette.opacity(value)) / 255.0);
                out.color_rect(
                    star.name,
                    [star_origin[0] - 1.6, star_origin[1] - 0.4, 19.2, 35.2],
                    fill,
                );
            }
            let ink = if fly_transformation.is_some() {
                WHITE
            } else {
                match star.name {
                    "紫微" => ColorToken::ChartZiwei,
                    "天府" | "天相" | "七殺" => ColorToken::ChartNeutralStar,
                    "左辅" | "右弼" | "文昌" | "文曲" => ColorToken::ChartLiteraryStar,
                    _ => BLACK,
                }
            };
            let localized = crate::i18n::chart_t(star.name);
            for (i, character) in localized.chars().enumerate() {
                out.label(
                    character.to_string(),
                    [
                        star_origin[0],
                        y + 3.40625 + baseline_offset + 16.0 * i as f32,
                    ],
                    FONT,
                    600.0,
                    ink,
                    false,
                )
                .identify(
                    format!("star-{}-{i}", star.name),
                    LabelMotion::Star(star.name),
                );
            }
            if let Some(birth) = star
                .birth
                .filter(|birth| Display::includes(&display.birth, *birth))
            {
                out.label(
                    birth.code(),
                    [star_origin[0] + 8.0, y + 37.40625],
                    FONT,
                    BIRTH_WEIGHT,
                    TRANSFORM_INK,
                    true,
                )
                .identify(
                    format!("birth-{}-{}", star.name, birth.code()),
                    LabelMotion::Mark,
                );
            }
            if let Some(outward_transform) = star.outward {
                if display.self_direction.outward()
                    && Display::includes(&display.self_transform, outward_transform)
                {
                    let spacing = OutwardSpacing {
                        ordinal: outward_ordinal,
                        total: outward_total,
                        has_birth: star.birth.is_some(),
                    };
                    let (points, label) = outward(index, star_origin, local_x, spacing);
                    out.arrow(
                        format!("outward-{}-{}", star.name, outward_transform.code()),
                        &points,
                        ArrowKind::Outward,
                        None,
                    );
                    out.label(
                        outward_transform.code(),
                        [label[0], label[1] - 13.0],
                        14.0,
                        SELF_WEIGHT,
                        TRANSFORM_INK,
                        true,
                    )
                    .identify(
                        format!("outward-code-{}-{}", star.name, outward_transform.code()),
                        LabelMotion::Mark,
                    );
                }
                outward_ordinal += 1;
            }
        }
    }

    /// 绘制流年信息及向心四化，并协调对宫是否同时显示向心箭头。
    fn period_and_inward(
        &mut self,
        index: usize,
        palace: &Palace,
        origin: [f32; 2],
        opposite_inward: bool,
    ) {
        let [x, y] = origin;
        let Self { out, display, .. } = self;
        use geometry::{ArrowKind, add, inward};
        let yearly = palace.yearly_name.map(crate::i18n::chart_t).or_else(|| {
            palace
                .yearly_age
                .map(|age| crate::i18n::chart_t(format!("{age} 歲")))
        });
        if let Some(yearly) = yearly.filter(|_| display.yearly && display.palace) {
            let label = out.label(yearly, [x + 75.0, y + 86.0], 10.0, 600.0, BLACK, true);
            if let Some(name) = palace.yearly_name {
                label.identify(
                    format!("yearly-name-{}", palace.branch.index()),
                    LabelMotion::Palace(palace.branch, "yearly", name),
                );
            } else if let Some(age) = palace.yearly_age {
                label.identify(format!("yearly-age-{age}"), LabelMotion::Travel);
            }
        }
        let inward_codes: String = palace
            .stars
            .iter()
            .filter_map(|star| {
                star.inward.filter(|t| {
                    display.self_direction.inward()
                        && Display::includes(&display.self_transform, *t)
                })
            })
            .map(Transformation::code)
            .collect();
        if !inward_codes.is_empty() {
            let (points, label) = inward(index);
            out.arrow(
                format!("inward-{index}-{inward_codes}"),
                &points.map(|point| add([x, y], point)),
                ArrowKind::Inward,
                opposite_inward.then_some(index < 6),
            );
            let position = add([x, y], label);
            out.label(
                &inward_codes,
                [position[0], position[1] - 13.0],
                14.0,
                SELF_WEIGHT,
                TRANSFORM_INK,
                true,
            )
            .identify(
                format!("inward-code-{index}-{inward_codes}"),
                LabelMotion::Mark,
            );
        }
    }
}
