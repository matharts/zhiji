//! 基于参数的参考模拟器排盘计算。
//!
//! 复现观察到的模拟器行为，包括输入不完整的情况；不接收公历日期，
//! 也不代表其中的术数规则经过独立验证。

use std::str::FromStr;

const STEM_NAMES: [&str; 10] = ["甲", "乙", "丙", "丁", "戊", "己", "庚", "辛", "壬", "癸"];
const BRANCH_NAMES: [&str; 12] = [
    "子", "丑", "寅", "卯", "辰", "巳", "午", "未", "申", "酉", "戌", "亥",
];
const PALACE_NAMES: [&str; 12] = [
    "命宮", "兄弟", "夫妻", "子女", "財帛", "疾厄", "遷移", "交友", "官祿", "田宅", "福德", "父母",
];
const DECADE_NAMES: [&str; 12] = [
    "大命", "大兄", "大夫", "大子", "大財", "大疾", "大遷", "大友", "大官", "大田", "大福", "大父",
];
const YEARLY_NAMES: [&str; 12] = [
    "流命", "流兄", "流夫", "流子", "流財", "流疾", "流遷", "流友", "流官", "流田", "流福", "流父",
];
const FIVE_ELEMENTS: [[u8; 6]; 5] = [
    [2, 6, 3, 5, 4, 6],
    [6, 5, 4, 3, 2, 5],
    [5, 3, 2, 4, 6, 3],
    [3, 4, 6, 2, 5, 4],
    [4, 2, 5, 6, 3, 2],
];
const LAIYIN: [u8; 10] = [10, 9, 8, 7, 6, 5, 4, 3, 2, 11];
const TRANSFORMED_STARS: [[&str; 4]; 10] = [
    ["廉貞", "破軍", "武曲", "太陽"],
    ["天機", "天梁", "紫微", "太陰"],
    ["天同", "天機", "文昌", "廉貞"],
    ["太陰", "天同", "天機", "巨門"],
    ["貪狼", "太陰", "右弼", "天機"],
    ["武曲", "貪狼", "天梁", "文曲"],
    ["太陽", "武曲", "太陰", "天同"],
    ["巨門", "太陽", "文曲", "文昌"],
    ["天梁", "紫微", "左辅", "武曲"],
    ["破軍", "巨門", "太陰", "貪狼"],
];

/// 从甲开始编号的天干。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stem(u8);

impl Stem {
    /// 从 0 至 9 构造天干；越界时返回空值。
    pub const fn from_index(index: u8) -> Option<Self> {
        if index < 10 { Some(Self(index)) } else { None }
    }
    /// 返回从甲开始的零基编号。
    pub const fn index(self) -> u8 {
        self.0
    }
    /// 返回参考模拟器使用的天干名称。
    pub fn name(self) -> &'static str {
        STEM_NAMES[self.0 as usize]
    }
    /// 按禄、权、科、忌的顺序返回该天干对应的四化星名。
    pub fn transformed_stars(self) -> [&'static str; 4] {
        TRANSFORMED_STARS[self.0 as usize]
    }
}

impl FromStr for Stem {
    type Err = &'static str;
    /// 精确匹配天干名称；未知名称返回错误，不做空白或别名归一化。
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        STEM_NAMES
            .iter()
            .position(|name| *name == value)
            .map(|i| Self(i as u8))
            .ok_or("unknown stem")
    }
}

/// 从子开始编号的地支。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Branch(u8);

impl Branch {
    /// 从 0 至 11 构造地支；越界时返回空值。
    pub const fn from_index(index: u8) -> Option<Self> {
        if index < 12 { Some(Self(index)) } else { None }
    }
    /// 返回从子开始的零基编号。
    pub const fn index(self) -> u8 {
        self.0
    }
    /// 返回参考模拟器使用的地支名称。
    pub fn name(self) -> &'static str {
        BRANCH_NAMES[self.0 as usize]
    }
    /// 转换为从寅开始的命盘数组下标。
    fn palace_index(self) -> usize {
        // 地支从子开始编号，命盘数组则从寅开始。
        (self.0 as usize + 10) % 12
    }
}

impl FromStr for Branch {
    type Err = &'static str;
    /// 精确匹配地支名称；未知名称返回错误。
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        BRANCH_NAMES
            .iter()
            .position(|name| *name == value)
            .map(|i| Self(i as u8))
            .ok_or("unknown branch")
    }
}

/// 模拟器选择的月份，范围为正月（1）至腊月（12）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Month(u8);

impl Month {
    /// 构造一基月份；仅接受 1 至 12，不表达闰月。
    pub const fn new(value: u8) -> Option<Self> {
        if value >= 1 && value <= 12 {
            Some(Self(value))
        } else {
            None
        }
    }
    /// 返回 1 至 12 的月份数值。
    pub const fn value(self) -> u8 {
        self.0
    }
}

/// 参与大限顺逆计算的性别选项。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Gender {
    #[default]
    Male,
    Female,
}

/// 排盘输入；可选字段对应模拟器中允许留空的控件。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Input {
    pub gender: Gender,
    pub stem: Option<Stem>,
    pub branch: Option<Branch>,
    pub ziwei: Option<Branch>,
    pub month: Option<Month>,
    pub hour: Option<Branch>,
    pub main: Option<Branch>,
    pub decade: Option<Branch>,
    pub yearly: Option<Branch>,
    pub show_location_stars: bool,
}

impl Default for Input {
    /// 保持排盘选项未选择，默认男性并显示定位星。
    fn default() -> Self {
        Self {
            gender: Gender::Male,
            stem: None,
            branch: None,
            ziwei: None,
            month: None,
            hour: None,
            main: None,
            decade: None,
            yearly: None,
            show_location_stars: true,
        }
    }
}

/// 四化标记；A、B、C、D 依次表示禄、权、科、忌。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transformation {
    A,
    B,
    C,
    D,
}

impl Transformation {
    /// 与四化星表列顺序一致的完整标记序列。
    pub const ALL: [Self; 4] = [Self::A, Self::B, Self::C, Self::D];
    /// 返回界面和参考数据使用的字母代码。
    pub const fn code(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
        }
    }
}

/// 大限沿十二宫排列推进的方向。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Reverse,
}

/// 五行局的局数及其显示名称。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FiveElement {
    pub number: u8,
    pub name: &'static str,
}

/// 一颗星及其生年、本宫向外和对宫向内四化标记。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Star {
    pub name: &'static str,
    pub birth: Option<Transformation>,
    pub outward: Option<Transformation>,
    pub inward: Option<Transformation>,
}

/// 单宫计算结果；依赖未选输入的内容保持为空。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Palace {
    pub branch: Branch,
    pub stem: Option<Stem>,
    pub name: Option<&'static str>,
    pub decade_name: Option<&'static str>,
    pub decade_range: Option<(u8, u8)>,
    pub yearly_age: Option<u8>,
    pub yearly_name: Option<&'static str>,
    pub is_laiyin: bool,
    pub stars: Vec<Star>,
}

/// 排盘快照，包含当前定位、大限方向及十二宫数据。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chart {
    pub main: Option<Branch>,
    pub decade: Option<Branch>,
    pub yearly: Option<Branch>,
    pub five_element: Option<FiveElement>,
    pub direction: Option<Direction>,
    /// 按参考顺序排列的十二宫，从寅到丑。
    pub palaces: [Palace; 12],
}

/// 将正负位移归一到十二宫下标，保证逆向移动不会得到负余数。
fn wrap(index: i32) -> usize {
    index.rem_euclid(12) as usize
}

/// 查找指定天干赋予该星的四化；天干缺失或星名不在表中时为空。
fn transformation(stem: Option<Stem>, star: &str) -> Option<Transformation> {
    stem.and_then(|stem| {
        stem.transformed_stars()
            .iter()
            .position(|name| *name == star)
    })
    .map(|index| Transformation::ALL[index])
}

/// 从指定起宫逆向分配宫名；起宫未定时整组名称保持为空。
fn names_for(main: Option<usize>, labels: &[&'static str; 12]) -> [Option<&'static str>; 12] {
    std::array::from_fn(|i| main.map(|main| labels[wrap(main as i32 - i as i32)]))
}

/// 根据允许不完整的输入计算当前可显示的模拟器数据。
pub fn calculate(input: &Input) -> Chart {
    // 手动选择优先于月、时推算；月或时缺失时不推算命宫。
    let main_index = input.main.map(Branch::palace_index).or_else(|| {
        input
            .month
            .zip(input.hour)
            .map(|(month, hour)| wrap(month.0 as i32 - 1 - hour.0 as i32))
    });
    let main = main_index.map(|i| Branch(((i + 2) % 12) as u8));
    let decade = input.decade.or(main);
    let decade_index = decade.map(Branch::palace_index);
    let yearly_index = input.yearly.map(Branch::palace_index);
    let stems: [Option<Stem>; 12] = std::array::from_fn(|i| {
        input.stem.map(|stem| {
            let starts = [2, 4, 6, 8, 0];
            Stem(((starts[stem.0 as usize % 5] + i) % 10) as u8)
        })
    });
    let names = names_for(main_index, &PALACE_NAMES);
    let decade_names = names_for(decade_index, &DECADE_NAMES);
    let yearly_names = names_for(yearly_index, &YEARLY_NAMES);
    let five_element = input.stem.zip(main).map(|(stem, branch)| {
        let number = FIVE_ELEMENTS[stem.0 as usize % 5][branch.0 as usize / 2];
        let name = match number {
            2 => "水二局",
            3 => "木三局",
            4 => "金四局",
            5 => "土五局",
            6 => "火六局",
            _ => unreachable!(),
        };
        FiveElement { number, name }
    });
    let direction = input.stem.map(|stem| {
        let gender = if input.gender == Gender::Male { 1 } else { 0 };
        if gender == (stem.0 + 1) % 2 {
            Direction::Forward
        } else {
            Direction::Reverse
        }
    });
    let mut ranges = [None; 12];
    if let (Some(main), Some(direction), Some(element)) = (main_index, direction, five_element) {
        let step = if direction == Direction::Forward {
            1
        } else {
            -1
        };
        for i in 0..12 {
            let start = element.number + i as u8 * 10;
            ranges[wrap(main as i32 + step * i)] = Some((start, start + 9));
        }
    }
    // 流年序列依赖所选大限的年龄范围和生年地支。
    let yearly_start = decade_index.zip(input.branch).and_then(|(decade, branch)| {
        ranges[decade]
            .map(|(start, _)| (start, wrap(branch.palace_index() as i32 + start as i32 - 1)))
    });

    let mut stars: [Vec<&'static str>; 12] = std::array::from_fn(|_| Vec::new());
    if let Some(ziwei) = input.ziwei {
        let ziwei = ziwei.palace_index() as i32;
        // 天府与紫微以命盘的寅申轴对称。
        let tianfu = wrap(12 - ziwei) as i32;
        for (name, index) in [
            ("紫微", ziwei),
            ("天機", ziwei - 1),
            ("太陽", ziwei - 3),
            ("武曲", ziwei - 4),
            ("天同", ziwei - 5),
            ("廉貞", ziwei - 8),
            ("太陰", tianfu + 1),
            ("貪狼", tianfu + 2),
            ("巨門", tianfu + 3),
            ("天梁", tianfu + 5),
            ("破軍", tianfu + 10),
        ] {
            stars[wrap(index)].push(name);
        }
        if input.show_location_stars {
            for (name, index) in [("天府", tianfu), ("天相", tianfu + 4), ("七殺", tianfu + 6)]
            {
                stars[wrap(index)].push(name);
            }
        }
    }
    if let Some(month) = input.month {
        let month = month.0 as i32 - 1;
        stars[wrap(2 + month)].push("左辅");
        stars[wrap(8 - month)].push("右弼");
    }
    if let Some(hour) = input.hour {
        stars[wrap(8 - hour.0 as i32)].push("文昌");
        stars[wrap(2 + hour.0 as i32)].push("文曲");
    }

    let palaces = std::array::from_fn(|i| {
        let branch = Branch(((i + 2) % 12) as u8);
        let yearly_age = yearly_start.and_then(|(start, start_index)| {
            let offset = wrap(i as i32 - start_index as i32);
            (offset < 10).then_some(start + offset as u8)
        });
        Palace {
            branch,
            stem: stems[i],
            name: names[i],
            decade_name: decade_names[i],
            decade_range: ranges[i],
            yearly_age,
            yearly_name: yearly_names[i],
            is_laiyin: input
                .stem
                .is_some_and(|stem| LAIYIN[stem.0 as usize] == branch.0),
            stars: stars[i]
                .iter()
                .map(|name| Star {
                    name,
                    birth: transformation(input.stem, name),
                    outward: transformation(stems[i], name),
                    // 对宫与当前宫位相隔六格。
                    inward: transformation(stems[(i + 6) % 12], name),
                })
                .collect(),
        }
    });
    Chart {
        main,
        decade,
        yearly: input.yearly,
        five_element,
        direction,
        palaces,
    }
}
