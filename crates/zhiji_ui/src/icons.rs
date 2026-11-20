//! 在统一 20 点网格上绘制的界面图标。
use crate::{ColorToken, color};
use rune::{El, LineCap, Shape, canvas};

#[derive(Clone, Copy)]
pub enum Icon {
    Calendar,
    Sparkles,
    Columns,
    List,
    Sidebar,
    Sun,
    Moon,
    Settings,
    Export,
    Copy,
    Close,
    Left,
    Right,
    Down,
    Up,
    Search,
    Globe,
    Check,
    Select,
    Rectangle,
    Ellipse,
    Arrow,
    Line,
    Pen,
    Highlighter,
    Text,
    Undo,
    Redo,
    Delete,
    More,
}

/// 返回图标的折线路径，坐标使用统一设计网格。
fn icon_paths(kind: Icon) -> Vec<Vec<[f32; 2]>> {
    let mut paths: Vec<Vec<[f32; 2]>> = Vec::new();
    let circle = |cx: f32, cy: f32, rx: f32, ry: f32| -> Vec<[f32; 2]> {
        (0..=32)
            .map(|i| {
                let a = i as f32 * std::f32::consts::TAU / 32.0;
                [cx + rx * a.cos(), cy + ry * a.sin()]
            })
            .collect()
    };
    let rounded_box = |left: f32, top: f32, right: f32, bottom: f32, radius: f32| {
        let mut points = Vec::new();
        for (cx, cy, start) in [
            (right - radius, top + radius, -std::f32::consts::FRAC_PI_2),
            (right - radius, bottom - radius, 0.0),
            (left + radius, bottom - radius, std::f32::consts::FRAC_PI_2),
            (left + radius, top + radius, std::f32::consts::PI),
        ] {
            for step in 0..=4 {
                let a = start + step as f32 * std::f32::consts::FRAC_PI_2 / 4.0;
                points.push([cx + radius * a.cos(), cy + radius * a.sin()]);
            }
        }
        points.push(points[0]);
        points
    };
    match kind {
        Icon::Calendar => paths.extend([
            rounded_box(3., 4.5, 17., 16.5, 1.5),
            vec![[3., 8.5], [17., 8.5]],
            vec![[6.5, 3.], [6.5, 6.]],
            vec![[13.5, 3.], [13.5, 6.]],
            vec![[6.5, 11.5], [7.5, 11.5]],
            vec![[12.5, 11.5], [13.5, 11.5]],
            vec![[6.5, 14.], [7.5, 14.]],
        ]),
        Icon::Sparkles => paths.extend([
            vec![
                [8., 4.],
                [9.6, 8.4],
                [14., 10.],
                [9.6, 11.6],
                [8., 16.],
                [6.4, 11.6],
                [2., 10.],
                [6.4, 8.4],
                [8., 4.],
            ],
            vec![[15.5, 2.5], [15.5, 7.5]],
            vec![[13., 5.], [18., 5.]],
            vec![[16., 13.5], [16., 17.5]],
            vec![[14., 15.5], [18., 15.5]],
        ]),
        Icon::Columns => paths.extend([
            rounded_box(3., 4., 17., 16., 1.5),
            vec![[10., 4.], [10., 16.]],
            vec![[5.5, 7.5], [7.5, 7.5]],
            vec![[12.5, 7.5], [14.5, 7.5]],
        ]),
        Icon::List => paths.extend([
            rounded_box(3., 4., 17., 16., 1.5),
            vec![[6., 7.], [14., 7.]],
            vec![[6., 10.], [14., 10.]],
            vec![[6., 13.], [11.5, 13.]],
        ]),
        Icon::Sidebar => paths.extend([
            rounded_box(2.75, 4., 17.25, 16., 1.5),
            vec![[11.5, 4.], [11.5, 16.]],
            vec![[13.75, 7.5], [15., 7.5]],
            vec![[13.75, 10.], [15., 10.]],
        ]),
        Icon::Sun => {
            paths.push(circle(10., 10., 3.6, 3.6));
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.;
                paths.push(vec![
                    [10. + 5.8 * a.cos(), 10. + 5.8 * a.sin()],
                    [10. + 7.3 * a.cos(), 10. + 7.3 * a.sin()],
                ]);
            }
        }
        Icon::Moon => {
            let mut moon = Vec::new();
            for i in 0..=24 {
                let a = (-65.0 - i as f32 * 235.0 / 24.0).to_radians();
                moon.push([10. + 6.5 * a.cos(), 10. + 6.5 * a.sin()]);
            }
            let start = moon[24];
            let end = moon[0];
            let control = [5.5, 11.0];
            for i in 1..=20 {
                let t = i as f32 / 20.0;
                moon.push([
                    (1.0 - t).powi(2) * start[0]
                        + 2.0 * (1.0 - t) * t * control[0]
                        + t * t * end[0],
                    (1.0 - t).powi(2) * start[1]
                        + 2.0 * (1.0 - t) * t * control[1]
                        + t * t * end[1],
                ]);
            }
            paths.push(moon);
        }
        Icon::Settings => paths.extend([
            vec![[3., 5.5], [6., 5.5]],
            circle(7.6, 5.5, 1.6, 1.6),
            vec![[9.2, 5.5], [17., 5.5]],
            vec![[3., 10.], [10.8, 10.]],
            circle(12.4, 10., 1.6, 1.6),
            vec![[14., 10.], [17., 10.]],
            vec![[3., 14.5], [6., 14.5]],
            circle(7.6, 14.5, 1.6, 1.6),
            vec![[9.2, 14.5], [17., 14.5]],
        ]),
        Icon::Export => paths.extend([
            vec![[10., 3.5], [10., 12.]],
            vec![[6.8, 8.8], [10., 12.], [13.2, 8.8]],
            vec![[4., 12.], [4., 16.5], [16., 16.5], [16., 12.]],
        ]),
        Icon::Copy => paths.extend([
            rounded_box(7., 7., 17., 17., 1.5),
            vec![[5., 13.], [3., 13.], [3., 3.], [13., 3.], [13., 5.]],
        ]),
        Icon::Close => paths.extend([vec![[5., 5.], [15., 15.]], vec![[15., 5.], [5., 15.]]]),
        Icon::Left => paths.push(vec![[12.5, 4.5], [7., 10.], [12.5, 15.5]]),
        Icon::Right => paths.push(vec![[7.5, 4.5], [13., 10.], [7.5, 15.5]]),
        Icon::Down => paths.push(vec![[5., 7.5], [10., 12.5], [15., 7.5]]),
        Icon::Up => paths.push(vec![[5., 12.5], [10., 7.5], [15., 12.5]]),
        Icon::Search => {
            paths.extend([circle(8.5, 8.5, 5.3, 5.3), vec![[12.4, 12.4], [16.5, 16.5]]])
        }
        Icon::Globe => paths.extend([
            circle(10., 10., 7., 7.),
            vec![[3., 10.], [17., 10.]],
            vec![[10., 3.], [7.5, 5.5], [6.7, 10.], [7.5, 14.5], [10., 17.]],
            vec![
                [10., 3.],
                [12.5, 5.5],
                [13.3, 10.],
                [12.5, 14.5],
                [10., 17.],
            ],
        ]),
        Icon::Check => paths.push(vec![[3.5, 10.], [8., 14.2], [16.5, 5.5]]),
        Icon::Select => paths.push(vec![
            [4.5, 3.],
            [4.5, 15.],
            [7.8, 11.8],
            [10.3, 17.],
            [12.5, 16.],
            [10., 10.8],
            [14.5, 10.8],
            [4.5, 3.],
        ]),
        Icon::Rectangle => paths.push(vec![[3., 5.], [17., 5.], [17., 15.], [3., 15.], [3., 5.]]),
        Icon::Ellipse => paths.push(circle(10., 10., 7., 5.3)),
        Icon::Arrow => paths.extend([
            vec![[4., 16.], [16., 4.]],
            vec![[10.5, 4.], [16., 4.], [16., 9.5]],
        ]),
        Icon::Line => paths.push(vec![[4., 16.], [16., 4.]]),
        Icon::Pen => paths.extend([
            vec![
                [3.5, 16.5],
                [5., 12.5],
                [13.5, 4.],
                [16., 6.5],
                [7.5, 15.],
                [3.5, 16.5],
            ],
            vec![[11.8, 5.7], [14.3, 8.2]],
        ]),
        Icon::Highlighter => paths.extend([
            vec![
                [4., 15.5],
                [11.5, 5.],
                [16., 8.5],
                [8.5, 17.],
                [4., 17.],
                [4., 15.5],
            ],
            vec![[10., 7.], [14.5, 10.5]],
            vec![[3., 18.], [9., 18.]],
        ]),
        Icon::Text => paths.extend([
            vec![[4., 5.], [16., 5.]],
            vec![[10., 5.], [10., 16.]],
            vec![[7., 16.], [13., 16.]],
        ]),
        Icon::Undo | Icon::Redo => {
            let mirror = matches!(kind, Icon::Redo);
            for mut p in [
                vec![[7., 4.5], [3.5, 8.], [7., 11.5]],
                vec![
                    [4., 8.],
                    [11., 8.],
                    [14., 9.],
                    [15.5, 11.5],
                    [15., 14.],
                    [12.5, 15.5],
                    [8.5, 15.5],
                ],
            ] {
                if mirror {
                    for point in &mut p {
                        point[0] = 20. - point[0];
                    }
                }
                paths.push(p);
            }
        }
        Icon::Delete => paths.extend([
            vec![[4., 5.5], [16., 5.5]],
            vec![[7., 5.5], [7.8, 3.5], [12.2, 3.5], [13., 5.5]],
            vec![[5.5, 5.5], [6.5, 16.5], [13.5, 16.5], [14.5, 5.5]],
            vec![[8.5, 8.5], [8.5, 13.5]],
            vec![[11.5, 8.5], [11.5, 13.5]],
        ]),
        Icon::More => {
            for x in [5., 10., 15.] {
                paths.push(circle(x, 10., 0.4, 0.4));
            }
        }
    }
    // 按视觉重心修正：斜向工具和开放轮廓的感知中心与方形画布中心并不相同。
    let (dx, dy) = match kind {
        Icon::Moon => (0.0, 0.0),
        Icon::Export => (0.0, 0.0),
        Icon::Select => (0.5, 0.0),
        Icon::Pen => (0.25, -0.25),
        Icon::Highlighter => (0.5, -1.5),
        Icon::Text => (0.0, -0.5),
        Icon::Undo => (0.5, 0.0),
        Icon::Redo => (-0.5, 0.0),
        _ => (0.0, 0.0),
    };
    for path in &mut paths {
        for point in path {
            point[0] += dx;
            point[1] += dy;
        }
    }
    paths
}

/// 绘制装饰性图标；可访问名称由包含它的控件提供。
pub fn icon(kind: Icon, size: f32, ink: ColorToken) -> El {
    let scale = size / 20.;
    let mut shape = Shape::default();
    for path in icon_paths(kind) {
        shape = shape.stroke_with(
            path.into_iter()
                .map(|[x, y]| [x * scale, y * scale])
                .collect::<Vec<_>>(),
            false,
            1.45 * scale,
            LineCap::Round,
            color(ink),
        );
    }
    canvas(shape).size(size, size).a11y_hidden()
}
