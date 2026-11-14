//! 官方国内区划与独立来源的海外地点目录、查询和暂存选择。

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct County {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) longitude: f64,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct City {
    #[serde(default)]
    pub(crate) longitude: f64,
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) counties: Vec<County>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Province {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) cities: Vec<City>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Country {
    pub(crate) code: String,
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) continent: String,
}

pub(crate) const CONTINENTS: [(&str, &str); 7] = [
    ("AS", "亚洲"),
    ("EU", "欧洲"),
    ("AF", "非洲"),
    ("NA", "北美洲"),
    ("SA", "南美洲"),
    ("OC", "大洋洲"),
    ("AN", "南极洲"),
];

/// 紧凑七字段地点记录：标识、名称、英文名、一级地区、国家代码、经度及人口。
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct OverseasPlace(String, String, String, String, String, f64, u64);

impl OverseasPlace {
    /// 返回稳定地点标识。
    pub(crate) fn id(&self) -> &str {
        &self.0
    }
    /// 返回主要展示名称。
    pub(crate) fn name(&self) -> &str {
        &self.1
    }
    /// 返回英文名称，供双语展示和搜索。
    pub(crate) fn english_name(&self) -> &str {
        &self.2
    }
    /// 返回一级地区名称，用于同名地点消歧。
    pub(crate) fn region(&self) -> &str {
        &self.3
    }
    /// 返回二字母国家或地区代码。
    pub(crate) fn country(&self) -> &str {
        &self.4
    }
    /// 返回东正西负的地点经度。
    pub(crate) fn longitude(&self) -> f64 {
        self.5
    }
    /// 组合主要名称和英文名，相同时只显示一次。
    pub(crate) fn display_name(&self) -> String {
        if self.1 == self.2 {
            self.1.clone()
        } else {
            format!("{} · {}", self.1, self.2)
        }
    }
}

#[derive(Debug, Deserialize)]
struct ChinaData {
    provinces: Vec<Province>,
}
#[derive(Debug, Deserialize)]
struct WorldData {
    places: Vec<OverseasPlace>,
}
#[derive(Debug, Deserialize)]
struct CountryData {
    countries: Vec<Country>,
}
#[derive(Debug, Deserialize)]
struct MajorCityData {
    ids: BTreeSet<String>,
}

#[derive(Debug)]
pub(crate) struct PlaceCatalog {
    provinces: Vec<Province>,
    places: Vec<OverseasPlace>,
    all_countries: Vec<Country>,
    countries: Vec<Country>,
    major_city_ids: Option<BTreeSet<String>>,
    timezones: BTreeMap<String, Vec<String>>,
}

static EMBEDDED: OnceLock<Result<PlaceCatalog, String>> = OnceLock::new();

/// 只加载一次内嵌目录，成功或失败结果均在编辑器和测试间共享。
pub(crate) fn ensure_catalog() -> &'static Result<PlaceCatalog, String> {
    EMBEDDED.get_or_init(PlaceCatalog::load_embedded)
}

/// 查询已经加载的目录，不触发新的加载。
pub(crate) fn loaded_catalog() -> Option<&'static Result<PlaceCatalog, String>> {
    EMBEDDED.get()
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum PlaceMode {
    #[default]
    Domestic,
    Overseas,
}

/// 判断是否使用国内区划入口的中国及港澳台代码。
pub(crate) fn is_domestic_code(code: &str) -> bool {
    matches!(code, "CN" | "HK" | "MO" | "TW")
}

impl PlaceCatalog {
    /// 加载内嵌地点、主要城市和时区映射，并检查所有可选地点的时区有效性。
    pub(crate) fn load_embedded() -> Result<Self, String> {
        let mut catalog = Self::from_json(
            include_str!("../../../assets/data/birthplace-china.json"),
            include_str!("../../../assets/data/birthplace-regions.json"),
            include_str!("../../../assets/data/birthplace-countries.json"),
        )?;
        let major: MajorCityData = serde_json::from_str(include_str!(
            "../../../assets/data/birthplace-major-cities.json"
        ))
        .map_err(|error| format!("海外城市目录无效：{error}"))?;
        let available: BTreeSet<_> = catalog
            .places
            .iter()
            .filter(|place| !is_domestic_code(place.country()))
            .map(|place| place.id())
            .collect();
        if major.ids.is_empty() || major.ids.iter().any(|id| !available.contains(id.as_str())) {
            return Err("海外城市目录包含未知地点。".into());
        }
        let selectable_codes: BTreeSet<_> = catalog
            .places
            .iter()
            .filter(|place| major.ids.contains(place.id()))
            .map(|place| place.country())
            .collect();
        catalog
            .countries
            .retain(|country| selectable_codes.contains(country.code.as_str()));
        catalog.major_city_ids = Some(major.ids);
        catalog.timezones = serde_json::from_str(include_str!(
            "../../../assets/data/birthplace-timezones.json"
        ))
        .map_err(|error| format!("地区时区数据无效：{error}"))?;
        let ids = catalog
            .provinces
            .iter()
            .flat_map(|p| &p.cities)
            .flat_map(|c| {
                std::iter::once(c.id.as_str()).chain(c.counties.iter().map(|v| v.id.as_str()))
            })
            .chain(catalog.places.iter().map(|p| p.id()));
        if ids.into_iter().any(|id| {
            catalog.timezones.get(id).is_none_or(|zones| {
                zones.is_empty() || zones.iter().any(|z| !zhiji_calendar::valid_time_zone(z))
            })
        }) {
            return Err("地区时区数据缺失或无法识别。".into());
        }
        Ok(catalog)
    }

    /// 解析基础目录并验证标识、名称、国家代码和经度，不读取外部文件。
    fn from_json(china: &str, world: &str, countries: &str) -> Result<Self, String> {
        let china: ChinaData =
            serde_json::from_str(china).map_err(|error| format!("国内地区数据无效：{error}"))?;
        let world: WorldData =
            serde_json::from_str(world).map_err(|error| format!("海外地区数据无效：{error}"))?;
        let countries: CountryData = serde_json::from_str(countries)
            .map_err(|error| format!("国家名称数据无效：{error}"))?;
        let valid_longitude =
            |longitude: f64| longitude.is_finite() && (-180.0..=180.0).contains(&longitude);
        if china.provinces.is_empty()
            || world.places.is_empty()
            || countries.countries.iter().any(|country| {
                country.code.len() != 2
                    || country.name.is_empty()
                    || !CONTINENTS
                        .iter()
                        .any(|(code, _)| *code == country.continent)
            })
            || china.provinces.iter().any(|province| {
                province.id.is_empty()
                    || province.name.is_empty()
                    || province.cities.iter().any(|city| {
                        city.id.is_empty()
                            || city.name.is_empty()
                            || !valid_longitude(city.longitude)
                            || city.counties.iter().any(|county| {
                                county.id.is_empty()
                                    || county.name.is_empty()
                                    || !valid_longitude(county.longitude)
                            })
                    })
            })
            || world.places.iter().any(|place| {
                place.id().is_empty()
                    || place.name().is_empty()
                    || place.country().len() != 2
                    || !valid_longitude(place.longitude())
            })
        {
            return Err("地区数据包含无效地点或经度。".into());
        }
        let country_codes: BTreeSet<_> = world
            .places
            .iter()
            .map(|place| place.country().to_owned())
            .collect();
        let mut catalog = Self {
            provinces: china.provinces,
            places: world.places,
            all_countries: countries.countries,
            countries: Vec::new(),
            major_city_ids: None,
            timezones: BTreeMap::new(),
        };
        catalog.all_countries.retain(|country| {
            !is_domestic_code(&country.code) && country_codes.contains(&country.code)
        });
        catalog.countries.clone_from(&catalog.all_countries);
        Ok(catalog)
    }

    /// 借用国内省级区划列表。
    pub(crate) fn provinces(&self) -> &[Province] {
        &self.provinces
    }
    /// 借用当前有可选海外城市的国家列表。
    pub(crate) fn countries(&self) -> &[Country] {
        &self.countries
    }
    /// 从完整海外国家目录按代码查找记录。
    pub(crate) fn country(&self, code: &str) -> Option<&Country> {
        self.all_countries
            .iter()
            .find(|country| country.code == code)
    }
    /// 仅枚举含可选国家的洲，保留固定展示顺序。
    pub(crate) fn continents(&self) -> impl Iterator<Item = (&'static str, &'static str)> + '_ {
        CONTINENTS.into_iter().filter(|(code, _)| {
            self.countries
                .iter()
                .any(|country| country.continent == *code)
        })
    }
    /// 按稳定标识查找国内省级记录。
    pub(crate) fn province(&self, id: &str) -> Option<&Province> {
        self.provinces.iter().find(|province| province.id == id)
    }
    /// 在指定省份内查找城市，拒绝跨父级匹配。
    pub(crate) fn city(&self, province: &str, city: &str) -> Option<&City> {
        self.province(province)?
            .cities
            .iter()
            .find(|item| item.id == city)
    }
    /// 沿省、市路径查找县级记录。
    pub(crate) fn county(&self, province: &str, city: &str, county: &str) -> Option<&County> {
        self.city(province, city)?
            .counties
            .iter()
            .find(|item| item.id == county)
    }
    /// 返回国家名称，未知代码保留原文以避免空标签。
    pub(crate) fn country_name<'a>(&'a self, code: &'a str) -> &'a str {
        self.all_countries
            .iter()
            .find(|country| country.code == code)
            .map_or(code, |country| country.name.as_str())
    }
    /// 返回国家所属洲的中文名，缺失时为空。
    pub(crate) fn continent_name(&self, country: &str) -> &str {
        self.country(country)
            .and_then(|country| {
                CONTINENTS
                    .iter()
                    .find(|(code, _)| *code == country.continent)
            })
            .map_or("", |(_, name)| *name)
    }
    /// 按名称、地区、人口和标识稳定排序指定国家的可选城市。
    pub(crate) fn overseas_country_cities(&self, country: &str) -> Vec<&OverseasPlace> {
        if is_domestic_code(country) {
            return Vec::new();
        }
        let mut cities: Vec<_> = self
            .places
            .iter()
            .filter(|place| place.country() == country && self.is_selectable(place))
            .collect();
        cities.sort_by(|left, right| {
            left.name()
                .cmp(right.name())
                .then(left.region().cmp(right.region()))
                .then(right.6.cmp(&left.6))
                .then(left.id().cmp(right.id()))
        });
        cities
    }
    /// 已加载主要城市目录时按其筛选，基础测试目录未配置时允许全部地点。
    fn is_selectable(&self, place: &OverseasPlace) -> bool {
        self.major_city_ids
            .as_ref()
            .is_none_or(|ids| ids.contains(place.id()))
    }
    /// 按国家、一级地区和稳定标识精确查找海外地点。
    pub(crate) fn overseas_city(
        &self,
        country: &str,
        region: &str,
        city: &str,
    ) -> Option<&OverseasPlace> {
        if is_domestic_code(country) {
            return None;
        }
        self.places.iter().find(|place| {
            place.country() == country && place.region() == region && place.id() == city
        })
    }

    /// 将完整选择解析为可提交地点，组合消歧标签、经度及候选时区。
    pub(crate) fn resolve(&self, selection: &PlaceSelection) -> Option<CommittedPlace> {
        let (parts, longitude) = match selection {
            PlaceSelection::Domestic {
                province,
                city,
                county,
            } => {
                let province_value = self.province(province)?;
                let city_value = self.city(province, city)?;
                if county.is_empty() {
                    (
                        vec![province_value.name.clone(), city_value.name.clone()],
                        city_value.longitude,
                    )
                } else {
                    let county_value = self.county(province, city, county)?;
                    (
                        vec![
                            province_value.name.clone(),
                            city_value.name.clone(),
                            county_value.name.clone(),
                        ],
                        county_value.longitude,
                    )
                }
            }
            PlaceSelection::Overseas {
                country,
                region,
                city,
            } => {
                let place = self.overseas_city(country, region, city)?;
                let duplicate_name = self.places.iter().any(|other| {
                    other.country() == country && other.name() == place.name() && other.id() != city
                });
                let mut parts = vec![
                    self.continent_name(country).to_owned(),
                    self.country_name(country).to_owned(),
                ];
                if duplicate_name {
                    parts.push(region.clone());
                }
                parts.push(place.name().to_owned());
                (parts, place.longitude())
            }
        };
        let mut display_parts: Vec<String> = Vec::new();
        for part in parts {
            if !part.is_empty() && display_parts.last() != Some(&part) {
                display_parts.push(part);
            }
        }
        let id = match selection {
            PlaceSelection::Domestic { city, county, .. } => {
                if county.is_empty() {
                    city
                } else {
                    county
                }
            }
            PlaceSelection::Overseas { city, .. } => city,
        };
        Some(CommittedPlace {
            selection: selection.clone(),
            label: display_parts.join(" / "),
            longitude,
            timezones: self.timezones.get(id).cloned().unwrap_or_default(),
        })
    }

    /// 搜索国内市县名称或海外双语地名及上下文，结果携带完整路径以区分同名。
    /// 名称命中优先，稳定排序后最多返回 50 项。
    pub(crate) fn search(&self, mode: PlaceMode, query: &str) -> Vec<PlaceSearchResult> {
        let query = query.trim();
        if query.is_empty() {
            return Vec::new();
        }
        // 目录来源同时含简体和繁体名称；查询也必须同时覆盖两种用字。
        let queries = search_queries(query);
        let mut results = Vec::new();
        match mode {
            PlaceMode::Domestic => {
                for province in &self.provinces {
                    for city in &province.cities {
                        if matches_search(&format!("{} {}", city.name, province.name), &queries) {
                            results.push(PlaceSearchResult {
                                name: city.name.clone(),
                                context: province.name.clone(),
                                target: PlaceSearchTarget::City {
                                    province: province.id.clone(),
                                    city: city.id.clone(),
                                },
                            });
                        }
                        for county in &city.counties {
                            let context = format!("{} / {}", province.name, city.name);
                            if matches_search(&format!("{} {}", county.name, context), &queries) {
                                results.push(PlaceSearchResult {
                                    name: county.name.clone(),
                                    context,
                                    target: PlaceSearchTarget::Selection(
                                        PlaceSelection::Domestic {
                                            province: province.id.clone(),
                                            city: city.id.clone(),
                                            county: county.id.clone(),
                                        },
                                    ),
                                });
                            }
                        }
                    }
                }
            }
            PlaceMode::Overseas => {
                for place in &self.places {
                    if is_domestic_code(place.country()) || !self.is_selectable(place) {
                        continue;
                    }
                    let context = format!(
                        "{} / {} / {}",
                        self.continent_name(place.country()),
                        self.country_name(place.country()),
                        place.region()
                    );
                    if matches_search(
                        &format!("{} {} {}", place.name(), place.english_name(), context),
                        &queries,
                    ) {
                        results.push(PlaceSearchResult {
                            name: place.name().to_owned(),
                            context,
                            target: PlaceSearchTarget::Selection(PlaceSelection::Overseas {
                                country: place.country().to_owned(),
                                region: place.region().to_owned(),
                                city: place.id().to_owned(),
                            }),
                        });
                    }
                }
            }
        }
        results.sort_by(|left, right| {
            matches_search(&right.name, &queries)
                .cmp(&matches_search(&left.name, &queries))
                .then(left.name.cmp(&right.name))
                .then(left.context.cmp(&right.context))
        });
        results.truncate(50);
        results
    }
}

/// 将查询扩展为原文、简体和繁体三种形式，并保持顺序去重。
fn search_queries(query: &str) -> Vec<String> {
    let mut queries = Vec::with_capacity(3);
    for value in [
        query.to_owned(),
        crate::i18n::to_simplified(query),
        crate::i18n::to_traditional(query),
    ] {
        let value = value.to_lowercase();
        if !queries.contains(&value) {
            queries.push(value);
        }
    }
    queries
}

/// 检查候选文本是否命中任意简繁查询形式。
fn matches_search(text: &str, queries: &[String]) -> bool {
    let text = text.to_lowercase();
    queries.iter().any(|query| text.contains(query))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PlaceSelection {
    Domestic {
        province: String,
        city: String,
        county: String,
    },
    Overseas {
        country: String,
        region: String,
        city: String,
    },
}
impl PlaceSelection {
    /// 根据选择路径判断其属于国内还是海外入口。
    pub(crate) fn mode(&self) -> PlaceMode {
        match self {
            Self::Domestic { .. } => PlaceMode::Domestic,
            Self::Overseas { .. } => PlaceMode::Overseas,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CommittedPlace {
    pub(crate) selection: PlaceSelection,
    pub(crate) label: String,
    pub(crate) longitude: f64,
    pub(crate) timezones: Vec<String>,
}
impl CommittedPlace {
    /// 将经度格式化为东经或西经及五位小数。
    pub(crate) fn longitude_label(&self) -> String {
        format!(
            "{} {:.5}°",
            if self.longitude < 0.0 {
                "西经"
            } else {
                "东经"
            },
            self.longitude.abs()
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PlaceSearchTarget {
    City { province: String, city: String },
    Selection(PlaceSelection),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlaceSearchResult {
    pub(crate) name: String,
    pub(crate) context: String,
    pub(crate) target: PlaceSearchTarget,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct PlaceDraft {
    pub(crate) is_open: bool,
    pub(crate) mode: PlaceMode,
    pub(crate) query: String,
    continent: Option<String>,
    country: Option<String>,
    region: Option<String>,
    city: Option<String>,
    county: Option<String>,
    committed: Option<CommittedPlace>,
}

impl PlaceDraft {
    /// 清除暂存和已提交出生地，恢复无地点状态。
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }

    /// 借用最后一次明确确认的地点。
    pub(crate) fn committed(&self) -> Option<&CommittedPlace> {
        self.committed.as_ref()
    }
    /// 返回暂存国家代码。
    pub(crate) fn country(&self) -> Option<&str> {
        self.country.as_deref()
    }
    /// 返回暂存洲代码。
    pub(crate) fn continent(&self) -> Option<&str> {
        self.continent.as_deref()
    }
    /// 国内返回省级标识，海外返回一级地区名称。
    pub(crate) fn region(&self) -> Option<&str> {
        self.region.as_deref()
    }
    /// 返回暂存城市标识。
    pub(crate) fn city(&self) -> Option<&str> {
        self.city.as_deref()
    }
    /// 返回暂存县级标识，空值可表示只选择城市。
    pub(crate) fn county(&self) -> Option<&str> {
        self.county.as_deref()
    }

    /// 打开指定地点入口，仅在模式一致时从已提交路径恢复暂存选择。
    pub(crate) fn open(&mut self, mode: PlaceMode, catalog: &PlaceCatalog) {
        self.is_open = true;
        self.mode = mode;
        self.query.clear();
        self.continent = None;
        self.country = None;
        self.region = None;
        self.city = None;
        self.county = None;
        if let Some(committed) = &self.committed
            && committed.selection.mode() == mode
        {
            match &committed.selection {
                PlaceSelection::Domestic {
                    province,
                    city,
                    county,
                } => {
                    self.region = Some(province.clone());
                    self.city = Some(city.clone());
                    self.county = Some(county.clone());
                }
                PlaceSelection::Overseas {
                    country,
                    region,
                    city,
                } => {
                    self.continent = catalog.country(country).map(|item| item.continent.clone());
                    self.country = Some(country.clone());
                    self.region = Some(region.clone());
                    self.city = Some(city.clone());
                }
            }
        }
    }
    /// 关闭选择器并清空搜索，不改变最后确认的地点。
    pub(crate) fn cancel(&mut self) {
        self.is_open = false;
        self.query.clear();
    }
    /// 校验海外洲选择并清空全部下级暂存值。
    pub(crate) fn choose_continent(&mut self, catalog: &PlaceCatalog, continent: &str) -> bool {
        if self.mode != PlaceMode::Overseas
            || !catalog.continents().any(|(code, _)| code == continent)
        {
            return false;
        }
        self.continent = Some(continent.to_owned());
        self.country = None;
        self.region = None;
        self.city = None;
        self.county = None;
        true
    }
    /// 仅选择当前洲内的国家，并清空地区及城市选择。
    pub(crate) fn choose_country(&mut self, catalog: &PlaceCatalog, country: &str) -> bool {
        if self.mode != PlaceMode::Overseas
            || !catalog
                .country(country)
                .is_some_and(|item| Some(item.continent.as_str()) == self.continent())
        {
            return false;
        }
        self.country = Some(country.to_owned());
        self.region = None;
        self.city = None;
        self.county = None;
        true
    }
    /// 选择国内省级地区并清除市县；海外不使用该级联入口。
    pub(crate) fn choose_region(&mut self, catalog: &PlaceCatalog, region: &str) -> bool {
        let valid = match self.mode {
            PlaceMode::Domestic => catalog.province(region).is_some(),
            PlaceMode::Overseas => false,
        };
        if !valid {
            return false;
        }
        self.region = Some(region.to_owned());
        self.city = None;
        self.county = None;
        true
    }
    /// 校验城市父级后暂存选择，海外城市同时补齐其一级地区。
    pub(crate) fn choose_city(&mut self, catalog: &PlaceCatalog, city: &str) -> bool {
        let valid = match self.mode {
            PlaceMode::Domestic => self
                .region()
                .and_then(|region| catalog.city(region, city))
                .is_some(),
            PlaceMode::Overseas => self.country().is_some_and(|country| {
                catalog
                    .places
                    .iter()
                    .any(|place| place.country() == country && place.id() == city)
            }),
        };
        if !valid {
            return false;
        }
        if self.mode == PlaceMode::Overseas {
            self.region = self.country().and_then(|country| {
                catalog
                    .places
                    .iter()
                    .find(|place| place.country() == country && place.id() == city)
                    .map(|place| place.region().to_owned())
            });
        }
        self.city = Some(city.to_owned());
        self.county = None;
        true
    }
    /// 选择当前城市下的县级地区，空字符串表示使用城市本身。
    pub(crate) fn choose_county(&mut self, catalog: &PlaceCatalog, county: &str) -> bool {
        if county.is_empty()
            && self.mode == PlaceMode::Domestic
            && self
                .region()
                .zip(self.city())
                .and_then(|(province, city)| catalog.city(province, city))
                .is_some()
        {
            self.county = None;
            return true;
        }
        if self.mode != PlaceMode::Domestic
            || self
                .region()
                .zip(self.city())
                .and_then(|(province, city)| catalog.county(province, city, county))
                .is_none()
        {
            return false;
        }
        self.county = Some(county.to_owned());
        true
    }
    /// 从暂存级联值组合完整路径，必要父级缺失时返回空值。
    pub(crate) fn selection(&self) -> Option<PlaceSelection> {
        match self.mode {
            PlaceMode::Domestic => Some(PlaceSelection::Domestic {
                province: self.region.clone()?,
                city: self.city.clone()?,
                county: self.county.clone().unwrap_or_default(),
            }),
            PlaceMode::Overseas => Some(PlaceSelection::Overseas {
                country: self.country.clone()?,
                region: self.region.clone()?,
                city: self.city.clone()?,
            }),
        }
    }
    /// 仅在当前暂存路径可被目录完整解析时允许确认。
    pub(crate) fn can_confirm(&self, catalog: &PlaceCatalog) -> bool {
        self.selection()
            .and_then(|selection| catalog.resolve(&selection))
            .is_some()
    }
    /// 成功解析后替换已提交地点并关闭选择器，否则保留编辑。
    pub(crate) fn confirm(&mut self, catalog: &PlaceCatalog) -> bool {
        let Some(committed) = self
            .selection()
            .and_then(|selection| catalog.resolve(&selection))
        else {
            return false;
        };
        self.committed = Some(committed);
        self.cancel();
        true
    }
    /// 将合法搜索结果安装到暂存级联，仍需用户确认后才成为生效地点。
    pub(crate) fn choose_search_result(
        &mut self,
        catalog: &PlaceCatalog,
        result: &PlaceSearchResult,
    ) -> bool {
        match &result.target {
            PlaceSearchTarget::City { province, city } => {
                if self.mode != PlaceMode::Domestic || catalog.city(province, city).is_none() {
                    return false;
                }
                self.region = Some(province.clone());
                self.city = Some(city.clone());
                self.county = None;
            }
            PlaceSearchTarget::Selection(selection) => {
                if self.mode != selection.mode() || catalog.resolve(selection).is_none() {
                    return false;
                }
                match selection {
                    PlaceSelection::Domestic {
                        province,
                        city,
                        county,
                    } => {
                        self.region = Some(province.clone());
                        self.city = Some(city.clone());
                        self.county = Some(county.clone());
                    }
                    PlaceSelection::Overseas {
                        country,
                        region,
                        city,
                    } => {
                        self.continent =
                            catalog.country(country).map(|item| item.continent.clone());
                        self.country = Some(country.clone());
                        self.region = Some(region.clone());
                        self.city = Some(city.clone());
                        self.county = None;
                    }
                }
            }
        }
        self.query.clear();
        true
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/workspace/input/places.rs"]
mod tests;
