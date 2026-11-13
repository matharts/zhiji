# 国内出生地官方快照

采集日期：2026-11-10。

- `mca-division-tree-2026-11-10.json`：民政部国家地名信息库现行树的原始 JSON，GET `https://dmfw.mca.gov.cn/xzqh/getList?code=0&trimCode=true&maxLevel=3`。行政区划代码发布入口：<https://dmfw.mca.gov.cn/XzqhVersionPublish.html>。该树未提供统一版本日期，因此以采集日期命名。
- `mca-birthplace-points-2026-11-10.json`：每个可选地区的代表点与来源记录 ID。内地按省名、标准名称匹配 `https://dmfw.mca.gov.cn/stname/listPub`；地级行政区类别为 `21300`，县级为 `21400`，分页每次最多 100 条，必须校验 total、唯一 ID 和完整覆盖。查询参数参考 <https://dmfw.mca.gov.cn/interface.html>。服务器错误不应当作空结果写入目录。
- 草湖市使用其政府驻地草湖镇的居民点记录，依据 <https://www.xinjiang.gov.cn/xinjiang/tzgg/202604/3691350b13064945947d8f78933ccb3e.shtml>。港澳整体代表点标明 GeoNames 来源，未声称由民政部提供。

从仓库根目录执行 `node tools/china-divisions.mjs`，重建含可选第三级区县的目录、国内时区映射及资源 SHA-256。改动原始快照前审阅行政区划增删改名、每个新代码的坐标来源，以及坐标有效性。香港、澳门可选整体地区；台湾下级目录由此来源标为资料暂缺。

## 可选区县代表点（2026-11-11）

`mca-county-points-2026-11-11.json` 覆盖现行树内全部 2,729 个第三级区县。对 `stname/listPub` 使用 `placeTypeCode=21400`、空 `code` 和 `stName`、`searchType=模糊`、`size=100`，共 29 页、2,848 个唯一来源 ID；按省、县名及行政代码或所属市匹配。

- 奎文区有两个同名记录，采用 `place_code=37070599921400000000`，其代码类别与县级类别一致。
- 濉溪县、点军区暂无同名县级记录，分别使用库内明确归属该县区的濉溪镇、点军街道代表点。
- 岑岭县使用库内新华镇代表点；政府驻地依据[新疆自治区政府公告](https://www.xinjiang.gov.cn/xinjiang/tzgg/202603/285e5d25b8ad4eae8df6b0f0efe847f6.shtml)。
- 三个乡级代表点均标记 `mca-local-representative`，保留原名、来源 ID 和选择说明；不将其声称为县级坐标。

## 海外地点候选快照（未接入应用）

`overseas-candidates-2026-11-11.json` 是海外来源切换的审计输入，不是可发布目录，也不参与应用构建。地点和名称来自 [Wikidata Gazetteer 固定快照](https://huggingface.co/datasets/yuiseki/wikidata-gazetteer/tree/ef7674b5b00f2af07e792332b1d2d9f8ffccdff1)（底层 Wikidata dump 2026-08-31，CC0）；保留有坐标、带英文或中文标签，且 P31 命中 city、town、village 及其通过 Wikidata P279 查询到的子类型，同时人口至少 15,000 或直接属于 city/town 的候选；另补入 6 个经核对的美国城市、美国统计居民点、美国非建制社区、日本市、日本区和郊区类型（Q1093829、Q498162、Q17343829、Q494721、Q137773、Q188509），仍沿用上述人口与基础类型筛选。筛选存在遗漏，类型闭包也可能包含行政统计区等非目标对象。

2026-11-11 另外对保留 QID 查询 P17 声明，去掉带结束时间和 deprecated rank 的声明，保留所有其余国家 QID；ISO 对照来自同日 Wikidata P297 查询。缺少结束时间不等于声明一定有效，所以仍需核验。名称优先 `zh-hans`，再取 `zh`；英文保留作辅助。父级为快照中的首个 P131 值，**可能为历史归属**。例如伦敦父级错误地指向韦塞克斯，不能直接用于界面。

马来西亚、新加坡、文莱的 OSM 补充取自 Geofabrik 地区 PBF，记录节点 ID、QID、名称和坐标；只保留带 `place` 标签、英文名与 Wikidata 名一致、坐标相距不超过 5 公里的节点。最多在 Wikidata 无中文名且 OSM 名称唯一时作为中文候选。PBF SHA-256 为 `9ffc49333e3affba6573f619dc1907fcae07aa6e482d8c1b3b85118feec97fac`。OSM 数据 © OpenStreetMap contributors，按 [ODbL](../../assets/data/LICENSE-ODbL.txt) 提供；本文件的混合数据库按 ODbL 分发。

从项目根目录运行：

```sh
node tools/overseas-candidate-audit.mjs
node tools/overseas-candidate-audit.mjs tools/data-sources/overseas-candidates-2026-11-11.json /tmp/overseas-candidate-audit.json
```

第二条命令输出每条候选及国家、名称冲突；所有候选均标注行政层级和时区尚未核验。不要把结果替换到 `assets/data`：尚未补齐国家和地区入口、校验地名和层级、去重、匹配时区，也没有独立的自动生成验证。上述快照筛选 SQL、样本和限制见 [海外覆盖调研](../../docs/research/overseas-place-coverage.md)。

为辅助发现国家声明冲突，候选快照另记录 `geographicCountries`：使用 [Natural Earth 1:10m Admin 0 Map Units](https://www.naturalearthdata.com/downloads/10m-cultural-vectors/) 5.1.1 面与候选坐标做点面相交，原始 ZIP SHA-256 为 `45bebe2aaf8bf42b9daf4428594925bcb11eabd32fe9dc6e0acf681438053eb5`。Natural Earth 数据为公有领域，其边界为制图概括且存在不同地区观点；点落不到面内或跨境不代表实际归属错误，亦不能用作单独的国家认定。审计工具只报告地理候选及冲突，不自动修正 Wikidata 的国家声明。

`geographicRegions` 按相同点面方法取得 Natural Earth 1:10m Admin 1 States and Provinces 5.1.1 的地区名称和 ISO 代码，原始 ZIP SHA-256 为 `efc59726337323058f9446210adc96673179cd344e053666ee3d28cb58ba2b05`。制图层级并不在各国完全一致，例如新加坡候选点落入社区发展理事会；这些结果仅供审核，不可直接视为适合选择器的一级行政区。

`timezoneCandidates` 在生成该快照时通过经纬度查询 `tzf-rs 2.1.1` 与 `tzf-dist 0.0.2026-f` 的简化时区边界，保留全部重叠结果。边界基于 Timezone Boundary Builder 和 OpenStreetMap，数据库按 ODbL 共享；匹配当前边界不保证历史归属正确，亦尚未检查所有标识是否被应用当前 IANA 版本支持。

对该次快照生成的 40,232 个时区候选标识，使用项目已构建的 `chrono-tz` 依赖逐项解析，未发现无法识别的标识；这仅验证标识有效，不能验证地点归属或历史时差。

对原目录仍缺入口的 15 个国家或地区，另按 Wikidata `P297 → P36` 查询首府或行政中心，并将其标记为 `capitalFor` 候选；这些声明及同一 QID 的多个坐标可能冲突，需人工审核，不自动作为国家归属。

## 海外重点地区分阶段迁移

`overseas-priority-migration-2026-11-11.json` 是首批迁移记录，不是整个海外目录。它覆盖美国、加拿大、澳大利亚、新西兰、英国、马来西亚、新加坡、日本和韩国中 3,397 个地点：先用 Wikidata P1566 找到原地点 ID，再要求英文名称归一化后相同、现行国家声明唯一、Natural Earth 国家面一致、经度差小于 0.03°；还要求首级地区面可匹配、候选时区非空、地点 QID 一对一。Natural Earth 地区中文名与原地区多点一致时用于整组；英国的英格兰、苏格兰、威尔士、北爱尔兰按原有层级固定中文名。相邻行政区和映射不唯一的地点不按此规则迁移。

另用 2026-11-11 下载的 GeoNames cities15000 记录比对完整经纬度，排除与 Wikidata 代表点距离超过 5 公里的 21 条，清单为 `overseas-priority-excluded-2026-11-11.json`；代表点差异不一定表示任一来源错误。名称优先 Wikidata 中文，缺失时可使用经 QID、名称与坐标交叉确认的 OSM 中文候选。时区来自 `tzf-rs 2.1.1` / `tzf-dist 0.0.2026-f`，按 Wikidata 坐标查询，保留重叠结果。地点经度、名称与时区仍需独立历史核验。各条记录保留原 ID、旧名称和旧经度，便于比较；其他地点暂时保留 GeoNames 数据，全球源替换尚未完成。

从仓库根目录执行 `node tools/overseas-priority-migration.mjs` 可对现有混合目录幂等重放本次迁移并更新清单校验值；输入记录的 ID、原名称或经度不符会停止。该脚本不从原始 Wikidata dump 重新生成审核快照。变更快照应重新核对来源、匹配、坐标、时区及数据许可，不能只更新日期。

### 第二批海外地点

`overseas-global-migration-2026-11-11.json` 在第一批的基础上补充 96 个国家和地区的 3,229 条地点，包含重点地区中此前未迁移但符合条件的地点。依然要求 Wikidata ID、英文名、国家声明、国家及地区点面、经度、完整经纬度不超过 5 公里、时区及一对一匹配；地区名称无法在原有分组中一致确认的记录留在旧目录。第二批带来 1,508 个此前不含汉字的中文地点名称及 572 组中文地区名称。保留现有地区和未匹配地点，海外来源替换仍未完成。

重放时按顺序执行：

```sh
node tools/overseas-priority-migration.mjs
node tools/overseas-priority-migration.mjs tools/data-sources/overseas-global-migration-2026-11-11.json
```

两批脚本可幂等重放；变更输入前仍须重新核验来源、代表点和时区，不得跳过差异审阅。

第二批中 Erbaa 与 Karasu 的 Wikidata 人口值为小数，不能作为人口整数使用；迁移快照将人口设为 0（未知），仅影响地点排序。生成器拒绝负数、非整数或超出 JavaScript 安全整数范围的人口值，避免地区目录无法加载。

### 第三批：OpenStreetMap 居民地

`overseas-osm-migration-2026-11-11.json` 记录从 [Geoapify OSM Locality Extracts](https://www.geoapify.com/data-share/localities/readme.html) 迁移的 1,176 个地点，覆盖 70 个国家和地区；每个来源国家 ZIP 的 SHA-256 记录在快照的 `archiveSha256` 中。下载于 2026-11-11，提取包内文件日期多为 2025-09-16，不代表 OSM 的实时状态。

只使用 OpenStreetMap 的地点节点，并要求有中文名称、英文名称归一化后与旧目录相同、原地点同名匹配唯一、坐标距离不超过 2 公里、Natural Earth 国家面及一级地区与原有地区一致或可按原文行政区名称确认，且节点 ID 一对一。英国沿用原有国家内部地区层级；地区名称仅对组内匹配一致的地点统一翻译。节点中存在多名称字符串或中文指向旧行政实体的两个案例已排除，记录见 `overseas-osm-excluded-2026-11-11.json`。匹配与来源仍可能有错，坐标、地区和中文名称不等于逐条人工认证。

本批记录使用 `osm:node:` 标识、OSM 中文名称和坐标；时区来自 `tzf-rs 2.1.1` / `tzf-dist 0.0.2026-f` 的对应地点边界，保留重叠项。数据 © OpenStreetMap contributors，按 [ODbL 1.0](../../assets/data/LICENSE-ODbL.txt) 分发。缺少可靠中文名或存在国家、地区、坐标冲突的记录继续保留旧数据，整体来源替换尚未完成。

在前两批之后重放第三批：

```sh
node tools/overseas-priority-migration.mjs tools/data-sources/overseas-osm-migration-2026-11-11.json
```

### 第四批：缺少中文名的 OpenStreetMap 地点

`overseas-osm-fallback-migration-2026-11-11.json` 包含 110 个国家和地区的 6,172 条地点。这些地点在 OSM 提取数据中未找到中文名称，迁移前后均以原文显示；不将其计为新增中文名，也不使用自动音译。名称须与原显示名归一化后一致，保留原有中文地点不被原文替换。国家及地区、名称、2 公里代表点距离、节点一对一和时区校验与第三批相同；其中 205 组原行政地区名改为经过一致性检查的中文名。数据与许可同第三批，原始国家 ZIP 的 SHA-256 保存在快照中。

按第一、二、三批之后的顺序重放：

```sh
node tools/overseas-priority-migration.mjs tools/data-sources/overseas-osm-fallback-migration-2026-11-11.json
```

这一批只转换地点数据来源，**不解决这些地点的中文名称缺口**；剩余 GeoNames 记录及没有可靠中文名的地点仍需进一步处理。

### 第五批：Wikidata 补充匹配

`overseas-wikidata-followup-2026-11-11.json` 在地区名称统一后补充迁移 614 个地点，覆盖 65 个国家和地区；其中 74 条此前没有中文名，326 条仍仅以原文显示。匹配和时区规则与第二批一致，缺少中文名时仅在原文名称归一化一致、且旧名称本身不含汉字时保留 Wikidata 原文。地区归属仍无法确认的记录未迁移。

按前四批之后的顺序重放：

```sh
node tools/overseas-priority-migration.mjs tools/data-sources/overseas-wikidata-followup-2026-11-11.json
```

## 海外洲别

`cldr-territory-containment-48.2.0.json` 为 [Unicode CLDR 48.2.0 的 territory containment](https://github.com/unicode-org/cldr-json/blob/48.2.0/cldr-json/cldr-core/supplemental/territoryContainment.json)。由 `node tools/overseas-continents.mjs` 根据 UN M49 区域树生成国家／地区洲别：北美洲包含加勒比和中美洲，南美洲单列；`AQ` 明确归入南极洲。它是用于选择器的地理分组，不作为国家主权或时区判断依据。Unicode 数据的许可证见 [LICENSE-Unicode.txt](../../assets/data/LICENSE-Unicode.txt)。

## Natural Earth 中文地名补充

`overseas-natural-earth-names-2026-11-11.json` 保留来自 [Natural Earth 1:10m Populated Places](https://www.naturalearthdata.com/downloads/10m-cultural-vectors/) 的中文名称和匹配证据，原始 ZIP 的 SHA-256 记在快照内。要求国家和归一化英文名一致、代表点距离不超过 2 公里、匹配唯一；排除名称以省、郡、县或区结尾的候选，以减少行政层级混淆。匹配不是逐条人工核验，仍可能存在译名或地点歧义；未找到中文名的条目保留原文。仅更新名称，不改变 ID、经度或时区。

重建时按上文先后执行全部五批迁移，再执行 `node tools/overseas-natural-earth-names.mjs`。迁移脚本会重写迁移地点的名称，因此每次重放迁移后须重新执行补名脚本；该脚本会校验原数据并更新资源校验值。洲别使用下文命令单独生成。

## 跨来源中文名补充

`overseas-cross-source-names-2026-11-11.json` 包含 2,807 条 Wikidata 和 294 条 OpenStreetMap 中文名称，补充此前显示原文的地点。Wikidata 从上述固定 Parquet 快照中按旧 GeoNames 关联、国家声明、归一化英文名及 2 公里内坐标匹配；已有 Wikidata ID 的地点还要求 QID 一致。OSM 采用 Geoapify 地点节点快照，按旧地点匹配、国家、英文名和 2 公里内坐标确定候选。每个地点要求唯一候选，排除部分可能指向行政区的名称；同一地点两者均有名称时优先 Wikidata。以上均为自动匹配，不能视为逐条人工核准。匹配快照记录原地点和来源点的坐标、来源 ID 以及来源文件哈希，仅替换名称，不改变标识、经度或时区。

从仓库根目录运行 `node tools/overseas-rebuild.mjs`，按顺序重放五批地点迁移、全部补名和洲别生成，并核验名称与实体关联。脚本对现有混合目录重放，不从原始资料重新生成匹配快照。

运行 `node tools/overseas-name-coverage.mjs` 可统计当前海外名称的汉字覆盖，附 `--places` 列出未含汉字的所有地点。汉字覆盖不代表中文名称准确或经过人工核验。


## 國家教育研究院中文地名补充

國家教育研究院（2015），[《國家教育研究院-外國地名譯名》](https://data.gov.tw/dataset/15211)，依[政府資料開放授權條款第 1 版](https://data.gov.tw/license)公開，使用者遵守條款即可利用。本项目取得 [immich-geodata-zh-tw 固定版本整理镜像](https://github.com/RxChi1d/immich-geodata-zh-tw/blob/672435fdbdbcd5629d384579c5b96bfd623fd0a6/data/vendor/naer/naer_place_names.csv)：镜像说明原始资料下载于 2026-06-06，文件 SHA-256 为 `6b4a8d86e5c8c7182da07ec4c437c275e199fed3e04466b04210e7dbcc8a483b`。镜像清理了译名、英文匹配键、国家代码与经纬度，并标记疑似自然地物；使用的是整理后的镜像，而非独立校验过的原始 CSV。

`overseas-naer-names-2026-11-11.json` 记录 1,686 条此前无汉字的名称及匹配证据：要求国家代码、归一化英文名一致，坐标距离不超过 2 公里，唯一匹配，排除疑似自然地物和部分行政区后缀；不改变已有 ID、经度、时区。来源坐标有取整，匹配和译名尚未逐条人工核验，部分名称为繁体或地区习惯译法。

`overseas-osm-extra-names-2026-11-11.json` 补充 2 条此前未匹配的 OSM 居民地中文名：采用 Geoapify 提取资料中的地名，要求国家、英文名和 2 公里内坐标一致、原地点唯一且候选译名一致，记录来源节点与各国数据包哈希。

## 按地点名称与类型补充中文名

`overseas-further-names-2026-11-11.json` 新增 3,275 条：2,859 条 Wikidata 中文标签按国家、英文标签和 2 公里内坐标匹配；265 条 Wikidata 标签按原 GeoNames 关联、国家、英文名及 5 公里内坐标匹配；151 条國家教育研究院译名按国家、英文名和 5 公里内坐标匹配。Wikidata 项需属于居民地或适用行政区类型，并排除车站等设施；类型快照为 `wikidata-settlement-subtypes-2026-11-11.json`，补充类型列在匹配快照中。`overseas-normalized-names-2026-11-11.json` 的 100 条使用归一化英文标签、国家、2 公里内坐标及相同类型约束；`overseas-english-alias-names-2026-11-11.json` 的 315 条使用归一化英文别名、国家、2 公里内坐标及相同类型约束。地物代表点不同，尤其 5 公里规则存在误配风险；这些均非逐条人工核验。

类型审计发现旧补名批次中有 18 条 Wikidata 项为车站，已撤回中文名并保存证据在 `overseas-name-infrastructure-excluded-2026-11-11.json`；新候选中另外排除 204 条类型不符或无法确认的记录，见 `overseas-further-names-excluded-2026-11-11.json`。

另有 `overseas-english-alias-wide-names-2026-11-11.json` 的 234 条及 `overseas-additional-types-names-2026-11-11.json` 的 75 条，采用英文标签或别名、国家、5 公里内坐标和唯一地点实体匹配，且要求居民地或相应行政区类型，排除车站等设施。两份快照保留 Wikidata ID、实例类型及匹配坐标；匹配未经逐条人工核验。

跨批复核还发现 9 条不同地点关联到同一 Wikidata 实体，已暂时撤回后匹配的名称；记录见 `overseas-name-duplicate-entity-excluded-2026-11-11.json`，待核对是否为同一地点或其子区域。

`overseas-name-corrections-2026-11-11.json` 修正三条来源标签指向附近设施的名称：乌布和林梦选用对应 Wikidata 条目的城市中文标签，Nesttun 在找到可靠城市译名前恢复原文。此步骤保留地点 ID、经度及全部时区。运行 `node tools/overseas-name-audit.mjs` 核验各批名称、地点字段与跨批 Wikidata 实体重复关联。

`overseas-cldr-names-2026-11-11.json` 从 Unicode CLDR 48.2 简体中文时区城市名补充 3 条，要求地点英文名与时区代表城市名一致、对应时区存在且目录中地点名称唯一。源文件及哈希见快照，使用 [Unicode License v3](../../assets/data/LICENSE-Unicode.txt)。

`overseas-multilingual-names-2026-11-11.json` 补充 470 条 Wikidata 中文标签，要求现有地点英文名与 Wikidata 非中文语言标签归一化后相同、国家一致、点位相距不超过 2 公里，并通过居民地类型和跨批实体唯一性检查；排除疑似设施名称。原文语言标签不一定是英文，本批仍为自动匹配，未逐条人工核验。

`overseas-multilingual-wide-names-2026-11-11.json` 另有 73 条，使用非中文语言标签或别名、国家和 5 公里内坐标，以及同样的类型、设施和实体唯一性筛选；仍需逐条核验。

`overseas-multilingual-alias-names-2026-11-11.json` 补充 30 条，按非中文语言标签或别名、国家和 2 公里内坐标匹配，要求实体类型及唯一性，并排除部分不同层级或疑似误译的名称。

`overseas-chinese-alias-names-2026-11-11.json` 收录 5 条 Wikidata 中文别名，要求对应英文名、国家、2 公里内坐标、地点类型和实体唯一性均匹配，排除设施名称。

`overseas-multilingual-chinese-alias-names-2026-11-11.json` 补充 6 条 Wikidata 中文别名，通过非中文原名或别名、国家、2 公里内坐标及地点类型和实体唯一性匹配，并排除部分设施或不同层级名称。

`overseas-wikidata-current-names-2026-11-11.json` 收录 14 条 2026-11-11 从 Wikidata API 获取的中文标签或中文维基百科条目名称，基于已知地点的原文名称、国家、2 公里内坐标、实体类型与唯一性匹配；在线查询曾遇速率限制，后续降低频率继续复查；未查出的记录不代表不存在中文译名。廷利帕克、伍德布里奇和克利夫頓帕克的中文写法采用相应 Wikidata 条目在同日指向的中文维基百科条目名称，记录在对应匹配快照中。

`overseas-chinese-locales-names-2026-11-11.json` 补充 7 条 Wikidata 中文地区标签，按其他语言原名或别名、国家、2 公里内坐标、实体类型和唯一性匹配；排除部分不同层级的候选。

`overseas-naer-alias-names-2026-11-11.json` 收录 12 条國家教育研究院译名：以 Wikidata 英文别名建立实体关联，译名库原文与现有英文名包含关系一致、国家相同、坐标在 2 公里内，且实体类型和唯一性检查通过。保留来源条目名称、对应 Wikidata ID 和坐标。

`overseas-naer-alias-wide-names-2026-11-11.json` 另有 2 条以同样的英文名、国家和实体匹配方式补充，代表点距离不超过 5 公里。

## GeoNames 现有中文别名补充

`overseas-geonames-zh-names-2026-11-11.json` 记录 8 条来自 GeoNames 2026-10-01 快照、带中文语言标记的现有别名；仅补充显示名，不改变现有地点来源、标识、经度或时区。以旧 GeoNames ID 关联，核对国家、归一化英文名与 2 公里内坐标，排除历史名、简称、俗名、带有效期名称和部分设施或行政区名称。原始 Parquet 的固定版本、哈希及别名 ID 记录在快照中。数据按 [CC BY 4.0](https://www.geonames.org/export/) 提供，自动关联尚未逐条人工核验。

## OpenStreetMap 地点节点补充

`overseas-osm-locality-names-2026-11-11.json` 包含 22 条来自 Geofabrik 的马来西亚、新加坡及老挝 OSM 地点节点中文名。匹配要求居民地节点的原名或英文名归一化一致、与目录地点距离不超过 2 公里、地点及节点关联唯一；若两边均有 Wikidata ID，则需相同。数据快照记录节点、坐标和 PBF 哈希；仅修改名称，不改变原地点标识、经度或时区。来源为 © OpenStreetMap contributors，遵循 [ODbL 1.0](../../assets/data/LICENSE-ODbL.txt)，自动匹配尚未逐条人工核验。

`overseas-osm-thailand-names-2026-11-11.json` 收录 1 条来自 Geofabrik 泰国 2026-11-09 快照的中文地点名称，采用相同的节点类型、名称、坐标和唯一性条件；PBF 原始 URL 及哈希记录在快照中。

`overseas-osm-vietnam-names-2026-11-11.json` 收录 5 条来自 Geofabrik 越南 2026-11-09 快照的中文地点名称，采用相同的节点类型、名称、坐标和唯一性条件；原始 URL、哈希和节点关联记录在快照中。

## 海外主要城市选择

`overseas-major-city-selection-2026-11-11.json` 固定记录 Natural Earth 1:10m 居民地中的国家首都和比例尺等级不超过 5 的地点匹配结果、GeoNames 城市分区（PPLX）标识以及按海外华人常用地点补充的清单。匹配需国家、归一化英文名一致且坐标相距不超过 5 公里；数据来源、关联 ID 和距离保存在快照中，自动关联未逐条人工核验。`node tools/overseas-major-cities.mjs` 以此为依据，结合目录内人口至少 20 万的地点（排除已识别的城市分区）及小国家／地区的最大有中文名地点，生成可选城市清单。筛选仅使用已有含汉字名称，不生成译名；人口可能过时，主要城市不完全等同人口阈值，清单可继续人工补正。城市总表和时区用于解析旧选择，未随筛选删除。
