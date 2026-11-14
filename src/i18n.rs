//! 简体与繁体界面用字；转换只作用于界面和命盘文案，不改用户输入。

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use ferrous_opencc::{OpenCC, config::BuiltinConfig};
use rune::El;
use serde::{Deserialize, Serialize};
use zhiji_ui::ColorToken;

/// 界面和命盘使用的中文用字标准。
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ChineseVariant {
    #[default]
    Simplified,
    Traditional,
}

impl ChineseVariant {
    /// 返回 Rune 排版使用的语言标签，同时决定字体的区域字形。
    pub(crate) fn tag(self) -> &'static str {
        match self {
            Self::Simplified => "zh-Hans",
            Self::Traditional => "zh-Hant",
        }
    }

    /// 返回用字选项在分段控件中的顺序。
    pub(crate) fn index(self) -> usize {
        match self {
            Self::Simplified => 0,
            Self::Traditional => 1,
        }
    }

    /// 将源文案统一转换为当前用字标准。
    fn convert(self, text: &str) -> String {
        CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            let map = match self {
                Self::Simplified => &mut cache.simplified,
                Self::Traditional => &mut cache.traditional,
            };
            if let Some(converted) = map.get(text) {
                return converted.clone();
            }
            let converted = self.convert_uncached(text);
            map.insert(text.to_owned(), converted.clone());
            converted
        })
    }

    /// 转换不进入长期缓存的动态输入。
    fn convert_uncached(self, text: &str) -> String {
        CONVERTERS.with(|converters| {
            let mut converters = converters.borrow_mut();
            let converters = converters.get_or_insert_with(Converters::new);
            match self {
                Self::Simplified => converters.t2s.convert(text),
                Self::Traditional => taiwan_terms(&unmask_domain(
                    &converters.s2twp.convert(&mask_domain(text)),
                )),
            }
        })
    }
}

struct Converters {
    s2twp: OpenCC,
    t2s: OpenCC,
}

impl Converters {
    /// 内嵌词典来自 OpenCC，构建资源损坏时属于不可继续的程序错误。
    fn new() -> Self {
        Self {
            s2twp: OpenCC::from_config(BuiltinConfig::S2twp).expect("内置台湾繁简词典"),
            t2s: OpenCC::from_config(BuiltinConfig::T2s).expect("内置繁简词典"),
        }
    }
}

/// 天干地支在简繁两套文本中使用相同字形，转换时以私用区占位，避免“丑”误转为“醜”。
const DOMAIN_CHARS: [char; 22] = [
    '甲', '乙', '丙', '丁', '戊', '己', '庚', '辛', '壬', '癸', '子', '丑', '寅', '卯', '辰', '巳',
    '午', '未', '申', '酉', '戌', '亥',
];

const DOMAIN_MASK_BASE: u32 = 0xE000;

/// 地名中的“乾”是专名用字（如乾县、乾安、乾州），不能被 T2s 当作“干”处理。
const PLACE_NOUN_CHARS: [char; 1] = ['乾'];
const PLACE_NOUN_MASK_BASE: u32 = 0xE200;

/// 将天干地支替换为私用区占位符；私用区字符不参与 OpenCC 词形转换。
fn mask_domain(text: &str) -> String {
    text.chars()
        .map(
            |character| match DOMAIN_CHARS.iter().position(|value| *value == character) {
                Some(index) => char::from_u32(DOMAIN_MASK_BASE + index as u32)
                    .expect("domain mask range is valid"),
                None => character,
            },
        )
        .collect()
}

/// 保护地名专名中不能按普通词语简化的字。
fn mask_place_nouns(text: &str) -> String {
    text.chars()
        .map(|character| {
            PLACE_NOUN_CHARS
                .iter()
                .position(|value| *value == character)
                .and_then(|index| char::from_u32(PLACE_NOUN_MASK_BASE + index as u32))
                .unwrap_or(character)
        })
        .collect()
}

/// 恢复地名专名占位符。
fn unmask_place_nouns(text: &str) -> String {
    text.chars()
        .map(|character| {
            (character as u32)
                .checked_sub(PLACE_NOUN_MASK_BASE)
                .and_then(|index| PLACE_NOUN_CHARS.get(index as usize).copied())
                .unwrap_or(character)
        })
        .collect()
}

/// 恢复被保护的天干地支字符。
fn unmask_domain(text: &str) -> String {
    text.chars()
        .map(|character| {
            let index = character as u32;
            index
                .checked_sub(DOMAIN_MASK_BASE)
                .and_then(|index| DOMAIN_CHARS.get(index as usize).copied())
                .unwrap_or(character)
        })
        .collect()
}

/// OpenCC 未覆盖的台湾软件常用词；按长词优先替换，避免被短词拆散。
const TAIWAN_TERMS: [(&str, &str); 45] = [
    ("應用程式包", "應用程式套件"),
    ("反饋問題", "回報問題"),
    ("反饋", "回饋"),
    ("另存為", "另存新檔"),
    ("自定義", "自訂"),
    ("十六進位制", "十六進位"),
    ("替換已有", "取代現有"),
    ("替換", "取代"),
    ("覆蓋", "覆寫"),
    ("儲存設定退出會", "儲存設定並結束會"),
    ("儲存設定退出後", "儲存設定並結束後"),
    ("退出", "結束"),
    ("鏈接", "連結"),
    ("更改", "變更"),
    ("丟失", "遺失"),
    ("工具欄", "工具列"),
    ("高亮筆", "螢光筆"),
    ("撤銷", "復原"),
    ("清空", "清除"),
    ("重置", "重設"),
    ("數據", "資料"),
    ("配置目錄", "設定目錄"),
    ("許可證", "授權條款"),
    ("當前", "目前"),
    ("雙列布局", "雙欄版面"),
    ("單列布局", "單欄版面"),
    ("視窗布局", "視窗版面"),
    ("上次布局", "上次版面"),
    ("布局", "版面"),
    ("佈局", "版面"),
    ("常規", "一般"),
    ("字號", "文字大小"),
    ("引數", "參數"),
    ("快捷鍵", "快速鍵"),
    ("日誌", "記錄"),
    ("宮幹", "宮干"),
    ("天幹", "天干"),
    ("年幹", "年干"),
    ("月幹", "月干"),
    ("日幹", "日干"),
    ("時幹", "時干"),
    ("幹支", "干支"),
    ("恢復", "回復"),
    ("幫助", "說明"),
    ("最小化", "縮到最小"),
];

/// 应用名词与动词在繁体中的写法不同，先保护已转换的名词再处理动词。
const APP_NOUN_MASK: char = '\u{E100}';

/// 将通用繁体用词调整为台湾软件习惯。
fn taiwan_terms(text: &str) -> String {
    if text == "應用" {
        return "套用".to_owned();
    }
    let text = text.replace("應用自定義顏色", "套用自訂顏色");
    let text = text.replace("應用程式", &APP_NOUN_MASK.to_string());
    let text = text.replace("應用", "應用程式");
    let text = text.replace(APP_NOUN_MASK, "應用程式");
    TAIWAN_TERMS
        .iter()
        .fold(text.to_owned(), |text, (from, to)| text.replace(from, to))
}

#[derive(Default)]
struct Cache {
    simplified: HashMap<String, String>,
    traditional: HashMap<String, String>,
}

thread_local! {
    static VARIANT: Cell<ChineseVariant> = const { Cell::new(ChineseVariant::Simplified) };
    static CONVERTERS: RefCell<Option<Converters>> = const { RefCell::new(None) };
    static CACHE: RefCell<Cache> = RefCell::new(Cache::default());
}

/// 视图构建期间的当前用字标准，离开作用域时恢复调用方原值。
pub(crate) struct Scope(ChineseVariant);

impl Drop for Scope {
    /// 恢复进入作用域前的用字标准，避免影响后续视图或测试。
    fn drop(&mut self) {
        VARIANT.with(|variant| variant.set(self.0));
    }
}

/// 在当前线程切换界面用字标准。
pub(crate) fn scope(variant: ChineseVariant) -> Scope {
    Scope(VARIANT.with(|current| current.replace(variant)))
}

/// 返回当前视图使用的中文用字标准。
pub(crate) fn variant() -> ChineseVariant {
    VARIANT.with(Cell::get)
}

/// 将界面文案转换为当前用字标准；源文案统一使用简体。
pub(crate) fn t(text: impl AsRef<str>) -> String {
    let text = text.as_ref();
    if text.is_empty() {
        return String::new();
    }
    match variant() {
        ChineseVariant::Simplified => text.to_owned(),
        ChineseVariant::Traditional => ChineseVariant::Traditional.convert(text),
    }
}

/// 转换系统菜单专用文案；macOS 繁体把“复制”写作“拷貝”。
#[cfg(target_os = "macos")]
pub(crate) fn menu_t(text: impl AsRef<str>) -> String {
    let text = text.as_ref();
    if variant() == ChineseVariant::Traditional && text == "复制" {
        "拷貝".to_owned()
    } else {
        t(text)
    }
}

/// 将命盘领域名称转换为当前用字标准；源名称可能简繁混用。
pub(crate) fn chart_t(text: impl AsRef<str>) -> String {
    mixed_t(text.as_ref())
}

/// 将来源简繁混杂的展示数据转换为当前用字标准。
pub(crate) fn data_t(text: impl AsRef<str>) -> String {
    let text = text.as_ref();
    if text.is_empty() {
        return String::new();
    }
    unmask_place_nouns(&variant().convert(&mask_place_nouns(text)))
}

/// 双向转换无法假定原始用字的领域名称或数据。
fn mixed_t(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    variant().convert(text)
}

/// 转换操作反馈的前缀；冒号后的诊断详情可能含路径，保持原样。
pub(crate) fn message(text: &str) -> String {
    match text.split_once('：') {
        Some((head, detail)) => format!("{}：{detail}", t(head)),
        None => t(text),
    }
}

/// 将检索输入规范为简体，兼容繁体界面下的地点搜索。
pub(crate) fn to_simplified(text: &str) -> String {
    let text = mask_place_nouns(text);
    unmask_place_nouns(&ChineseVariant::Simplified.convert_uncached(&text))
}

/// 将检索输入规范为繁体，兼容简体界面下输入繁体地名。
pub(crate) fn to_traditional(text: &str) -> String {
    let text = mask_place_nouns(text);
    unmask_place_nouns(&ChineseVariant::Traditional.convert_uncached(&text))
}

/// 创建本地化后的半粗标签。
pub(crate) fn label(value: impl AsRef<str>, size: f32, ink: ColorToken) -> El {
    zhiji_ui::label(t(value), size, ink)
}

/// 创建本地化后的常规标签。
pub(crate) fn regular_label(value: impl AsRef<str>, size: f32, ink: ColorToken) -> El {
    zhiji_ui::regular_label(t(value), size, ink)
}

/// 为 Rune 元素设置本地化后的可访问名称、提示或描述。
pub(crate) trait LocalizedEl {
    fn local_label(self, value: impl AsRef<str>) -> Self;
    fn local_help(self, value: impl AsRef<str>) -> Self;
    fn local_description(self, value: impl AsRef<str>) -> Self;
}

impl LocalizedEl for El {
    fn local_label(self, value: impl AsRef<str>) -> Self {
        self.label(t(value))
    }

    fn local_help(self, value: impl AsRef<str>) -> Self {
        self.help(t(value))
    }

    fn local_description(self, value: impl AsRef<str>) -> Self {
        self.description(t(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 验证简繁转换覆盖界面和命盘常用术语，并随作用域恢复。
    fn variant_scope_converts_both_directions() {
        let current = scope(ChineseVariant::Traditional);
        assert_eq!(
            message("保存草稿失败：/tmp/设置.ziwei"),
            "儲存草稿失敗：/tmp/设置.ziwei"
        );
        assert_eq!(chart_t("命宮"), "命宮");

        drop(current);
        assert_eq!(variant(), ChineseVariant::Simplified);
        assert_eq!(t("设置与命盘"), "设置与命盘");
        assert_eq!(t("乾县"), "乾县");
        assert!(message("保存草稿失败：/tmp/设置.ziwei").ends_with("：/tmp/设置.ziwei"));
        assert_eq!(chart_t("命宮"), "命宫");
        assert_eq!(chart_t("財帛"), "财帛");
    }

    #[test]
    /// 验证繁体采用台湾软件用词，而非只把字形逐字换成繁体。
    fn traditional_uses_taiwan_terms() {
        let _locale = scope(ChineseVariant::Traditional);
        assert_eq!(t("设置与命盘"), "設定與命盤");
        assert_eq!(t("界面外观"), "介面外觀");
        assert_eq!(t("界面字号"), "介面文字大小");
        assert_eq!(t("恢复上次布局"), "回復上次版面");
        assert_eq!(t("常规"), "一般");
        assert_eq!(t("排盘参数"), "排盤參數");
        assert_eq!(t("快捷键"), "快速鍵");
        assert_eq!(t("诊断日志"), "診斷記錄");
        assert_eq!(t("文件"), "檔案");
        assert_eq!(t("保存草稿"), "儲存草稿");
        assert_eq!(t("导出图片"), "匯出圖片");
        assert_eq!(t("默认取数方式"), "預設取數方式");
        assert_eq!(t("打开最近"), "開啟最近");
        assert_eq!(t("窗口"), "視窗");
        assert_eq!(t("帮助"), "說明");
        assert_eq!(t("撤销标记"), "復原標記");
        assert_eq!(t("高亮笔"), "螢光筆");
        assert_eq!(t("命盘标记工具栏"), "命盤標記工具列");
        assert_eq!(t("宫干"), "宮干");
        assert_eq!(t("另存为"), "另存新檔");
        assert_eq!(t("当前文件"), "目前檔案");
        assert_eq!(t("数据"), "資料");
        assert_eq!(t("恢复默认设置"), "回復預設設定");
        assert_eq!(t("切换为双列布局"), "切換為雙欄版面");
        assert_eq!(t("乾县"), "乾縣");
        assert_eq!(
            t("目标文件已存在，是否替换？"),
            "目標檔案已存在，是否取代？"
        );
        assert_eq!(t("应用"), "套用");
        assert_eq!(t("应用自定义颜色"), "套用自訂顏色");
        assert_eq!(
            t("当前应用只支持当前草稿格式"),
            "目前應用程式只支援目前草稿格式"
        );
        assert_eq!(t("重新打开应用后重试"), "重新開啟應用程式後重試");
        assert_eq!(t("自定义颜色十六进制值"), "自訂顏色十六進位值");
        assert_eq!(t("替换已有文件"), "取代現有檔案");
        assert_eq!(
            t("原文件已被修改或删除。覆盖将替换当前磁盘内容。"),
            "原檔案已被修改或刪除。覆寫將取代目前磁碟內容。"
        );
        assert_eq!(t("退出知几"), "結束知幾");
        assert_eq!(
            t("不保存设置退出会丢失本次设置修改。"),
            "不儲存設定並結束會遺失本次設定修改。"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    /// 验证 macOS 系统菜单的“复制”使用台湾菜单用词。
    fn macos_menu_uses_taiwan_copy_term() {
        let _locale = scope(ChineseVariant::Traditional);
        assert_eq!(menu_t("复制"), "拷貝");
    }

    #[test]
    /// 验证天干地支不会被字形转换误改。
    fn domain_characters_survive_traditional_conversion() {
        let _locale = scope(ChineseVariant::Traditional);
        assert_eq!(t("地支 丑"), "地支 丑");
        assert_eq!(t("丑时"), "丑時");
        assert_eq!(t("紫微在丑"), "紫微在丑");
        assert_eq!(chart_t("丑"), "丑");
        assert_eq!(chart_t("財帛"), "財帛");
    }

    #[test]
    /// 验证来源混杂的展示数据在简体界面也会被规范化。
    fn mixed_source_data_normalizes_in_both_directions() {
        let _locale = scope(ChineseVariant::Simplified);
        assert_eq!(data_t("河內長春"), "河内长春");
        assert_eq!(data_t("乾县"), "乾县");
        assert_eq!(to_simplified("乾县"), "乾县");
        assert_eq!(to_simplified("河內"), "河内");
        assert_eq!(to_traditional("河内"), "河內");
        assert_eq!(to_traditional("长春"), "長春");
        assert_eq!(to_traditional("乾县"), "乾縣");

        drop(_locale);
        let _locale = scope(ChineseVariant::Traditional);
        assert_eq!(data_t("河内长春"), "河內長春");
    }

    #[test]
    /// 验证检索输入不会把任意动态字符串永久写入转换缓存。
    fn dynamic_search_queries_are_not_cached() {
        let before = CACHE.with(|cache| cache.borrow().simplified.len());
        for index in 0..64 {
            to_simplified(&format!("繁{index}"));
            to_traditional(&format!("繁{index}"));
        }
        let after = CACHE.with(|cache| cache.borrow().simplified.len());
        assert_eq!(before, after);
    }
}
