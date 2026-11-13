# 出生地数据

## 国内行政区划

名称、代码和隶属关系来自[民政部国家地名信息库](https://dmfw.mca.gov.cn/XzqhVersionPublish.html)的现行行政区划树，采集日期为 **2026-11-10**。查询地址为 `https://dmfw.mca.gov.cn/xzqh/getList?code=0&trimCode=true&maxLevel=3`。发布页的年度表为 `Xzqh20251231`，本项目采用现行树，包含 2026 年草湖市等更新；采集日期不代表官方为整棵树标注了统一生效日期。

- 保留 34 个省级名称、334 个地级行政区及省直接管辖的下级地区。选择器支持省、市、区县三级，区县选填，选到市即可确认：普通省份显示市、州、盟、地区及省直辖地区，直辖市显示区县。第二级共 453 个可选择地区，含港澳两个整体代表点；其下有 2,729 个可选区县。无下级区县的地区不虚构中间层级。
- 国内 ID 使用 `cn:` 前缀，与海外 GeoNames ID 及 `wd:` Wikidata ID 隔离。台湾省在该官方树中标记“资料暂缺”，因此保留省级入口和明确空态，不把旧目录冒充官方下级名录；其内部选择器 ID 为 `cn:710000`。
- 内地 451 个可选地区的代表点来自国家地名库 `stname/listPub` 查询，按省名和标准名称核对；坐标来源记录 ID 保存在原始资料目录。草湖市暂无同名市级点，采用同库草湖镇居民点，政府驻地依据[新疆自治区政府公告](https://www.xinjiang.gov.cn/xinjiang/tzgg/202604/3691350b13064945947d8f78933ccb3e.shtml)。代表点不是具体出生地址。
- 第三级区县代表点于 2026-11-11 从同一国家地名库采集。2,726 个使用县级行政区记录；濉溪县、点军区、岑岭县暂无同名县级点，分别采用库内所属濉溪镇、点军街道、新华镇的代表点，并在来源表明确记录，不冒充县级点或精确出生地址。
- 港澳下级目录未在此官方树提供，保留同名整体选项，代表点沿用 GeoNames 香港、澳门地区记录。海外地点按下方所述分阶段迁移，国家／地区显示名称使用 Unicode CLDR。
- 时区与行政区划分别维护：内地采用 Asia/Shanghai；新疆提供北京时间和新疆时间两种显式选择；香港、澳门各自使用 IANA 时区规则。行政区划目录不作为历史时区的来源。

原始行政树与坐标来源表位于 [tools/data-sources](../../tools/data-sources/)。从仓库根目录执行 `node tools/china-divisions.mjs` 可离线重建国内目录、时区映射和资源校验值。更新官方快照时应同时审阅增删、改名、隶属关系与代表点，不仅修改日期。资源 SHA-256 统一维护在 [manifest.json](../manifest.json)。

## 海外数据与许可

海外目录以 2026-11-10 GeoNames 快照为基础，含国内与海外共 34,208 条记录；国内选择器只使用独立的 `cn:` 目录。2026-11-11 对美国、加拿大、澳大利亚、新西兰、英国、马来西亚、新加坡、日本、韩国第一批 3,397 条可交叉匹配记录迁移为 `wd:` 标识，其名称和代表点来自 Wikidata，地区名参考 Natural Earth，时区按代表点使用现有 tzf 边界映射；第一批 128 个可确认的地区名称也改为中文。第二批覆盖 96 个国家及地区，迁移 3,229 条地点、中文化 572 组地区名称；第三批从 Geoapify 提供的 OpenStreetMap 离线居民地提取资料迁移 1,176 条地点、中文化 154 组地区名称。第四批迁移 6,172 条暂无中文名的 OSM 地点，并中文化 205 组地区名称；这些地点继续以原文显示。第五批补充迁移 614 条地点，其中 74 条增加中文名。五批合计 14,588 条地点，原先没有汉字名称的地点中有 3,264 条得到中文名称。另从 Natural Earth 的居民地资料补充 1,014 条中文名，再从 Wikidata 和 OSM 补充 3,101 条中文名，从國家教育研究院开放数据补充 1,686 条中文名，并补充 2 条 OSM 中文名；后续依据地点名称、国家、坐标和居民地类型补充 4,618 条中文名；均按地点关联、名称及坐标交叉匹配。名称含简繁体，未经逐条人工核验。剩余海外记录仍来自 GeoNames，全球来源替换尚未完成，目录也不是完整城市目录。

匹配依据、排除记录、快照和重建命令见 [数据来源说明](../../tools/data-sources/README.md#海外重点地区分阶段迁移)。跨源代表点相差超过 5 公里的记录未迁移；地图边界与城市代表点均有误差，历史时区仍应以实际记录核对。原 GeoNames 国内时区映射目前仍保留，但国内选择器不使用这些旧 ID。

## 许可与归属

GeoNames 来源的地理数据归属 **GeoNames**（[geonames.org](https://www.geonames.org/)），采用 [Creative Commons Attribution 4.0 International](https://creativecommons.org/licenses/by/4.0/)（CC BY 4.0）。2026-11-10 核对 [GeoNames 官方导出说明](https://download.geonames.org/export/dump/readme.txt) 及[官方介绍](https://www.geonames.org/about.html)：数据按原样提供，不保证准确、及时或完整，其坐标使用 WGS84；此坐标基准说明不适用于民政部来源的代表点。上方列出了本项目沿用的整理与显示修改；GeoNames 未为本应用背书。

第三批居民地数据由 [Geoapify 离线提取](https://www.geoapify.com/data-share/localities/readme.html)提供，来源为 © [OpenStreetMap contributors](https://www.openstreetmap.org/copyright)，按 [ODbL 1.0](LICENSE-ODbL.txt) 提供；本项目根据名称、行政区和坐标筛选并整理，未得到 Geoapify 或 OpenStreetMap 背书。

Wikidata 结构化数据按 [CC0](https://www.wikidata.org/wiki/Wikidata:Licensing) 提供；Natural Earth 行政区数据为[公有领域](https://www.naturalearthdata.com/about/terms-of-use/)。本目录保留来源字段和转换工具。

海外按洲、国家或地区、城市选择；洲别采用 Unicode CLDR 地区分组，北美洲包含加勒比和中美洲，匹配信息见[数据来源说明](../../tools/data-sources/README.md#海外洲别)。国家／地区显示名所用 Unicode CLDR 数据遵循 [Unicode License V3](https://www.unicode.org/license.txt)，版权归 Unicode, Inc.。许可全文随附于 [LICENSE-Unicode.txt](LICENSE-Unicode.txt)。

补全的时区边界来源于 [Timezone Boundary Builder](https://github.com/evansiroky/timezone-boundary-builder) 和 © [OpenStreetMap contributors](https://www.openstreetmap.org/copyright)，由 [tzf-dist](https://github.com/ringsaturn/tzf-dist) 分发。相应边界衍生数据按 [ODbL 1.0](LICENSE-ODbL.txt) 提供；本仓库公开映射结果与生成工具，保留来源归属。`tzf-rs` 及 `tzf-dist` 的软件代码许可为 MIT。

打包分发时应同时包含本说明、Unicode 与 ODbL 许可；保留 GeoNames 和 OpenStreetMap 归属及许可链接。


國家教育研究院（2015），《國家教育研究院-外國地名譯名》，[政府資料開放平臺資料集 15211](https://data.gov.tw/dataset/15211)。本项目使用经名称及坐标筛选的译名；开放资料依[政府資料開放授權條款第 1 版](https://data.gov.tw/license)公开，使用者遵守该条款即可利用。所用镜像的源资料下载于 2026-06-06，筛选方式见[数据来源说明](../../tools/data-sources/README.md#國家教育研究院中文地名补充)。

Unicode CLDR 48.2 的简体中文时区代表城市名另用于补充 3 条地点中文名；来源及条件见[数据来源说明](../../tools/data-sources/README.md#按地点名称与类型补充中文名)。

另有 8 条中文显示名来自 GeoNames 的中文别名，仅补充名称，所用地点原有标识、经度和时区不变；匹配记录与许可见[数据来源说明](../../tools/data-sources/README.md#geonames-现有中文别名补充)。

另从 Geofabrik 的 OpenStreetMap 地点节点补充 22 条马来西亚、新加坡和老挝中文名；节点与来源信息见[数据来源说明](../../tools/data-sources/README.md#openstreetmap-地点节点补充)。

Geofabrik 泰国 2026-11-09 OSM 提取资料另补充 1 条中文地名，节点、数据哈希与许可见[数据来源说明](../../tools/data-sources/README.md#openstreetmap-地点节点补充)。

Geofabrik 越南 2026-11-09 OSM 提取资料另补充 5 条中文地名；匹配记录及许可见[数据来源说明](../../tools/data-sources/README.md#openstreetmap-地点节点补充)。

海外出生地选择和搜索使用 `birthplace-major-cities.json` 中的主要城市清单，包含有中文名称的首都、重要城市、人口较大的城市以及部分海外华人常用城市；较小国家／地区保留已有中文名的人口最多地点。既有完整目录仍供历史选择解析，详见[数据来源说明](../../tools/data-sources/README.md#海外主要城市选择)。
