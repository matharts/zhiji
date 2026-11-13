# 海外出生地中文覆盖抽样：Wikidata 与 OpenStreetMap

调研日期：2026-11-11。结论：样本中的匹配和中文覆盖均有缺口，暂不能证明任一来源可直接替换现有目录；需扩大审计并明确缺失中文名的处理方式。

## 比较口径

共同基准为当前 `assets/data/birthplace-regions.json`（SHA-256 `0f6c1cac0017e2d9302ce2362f96b6995f7e66712a0f76c84f9adf147c1659a0`）。从马来西亚、新加坡、约旦分别按人口降序取前 5 项，再从剩余记录按等间隔取 10 项；文莱和冰岛所有记录均纳入。合计 58 项。这是方便获取 OSM 分区快照的探索样本，国家选择及人口分层均非随机，不能推断全球覆盖率。基准数据包含城镇、城区及其他地点，不能一概当作独立城市。

Wikidata：2026-11-11 以基准 GeoNames ID 查询 [P1566](https://www.wikidata.org/wiki/Property:P1566)，读取条目的 `zh`、`zh-hans` 标签和 P625 坐标。未匹配仅说明这一关联方法未找到条目，不证明地点不存在于 Wikidata。多坐标和重复结果按基准地点去重。

OSM：解析 Geofabrik 下载的 [冰岛](https://download.geofabrik.de/europe/iceland.html)、[马来西亚／新加坡／文莱](https://download.geofabrik.de/asia/malaysia-singapore-brunei.html)及[约旦](https://download.geofabrik.de/asia/jordan.html) PBF；采集日期 2026-11-11，SHA-256 分别为 `7eb14b6e6fb6eae8c671c0aac9ae3b08f53fc7c274f91ff5293233a9b5055217`、`9ffc49333e3affba6573f619dc1907fcae07aa6e482d8c1b3b85118feec97fac`、`b63692f96a59139f37f8eb5c5bcb6ae4d1f1d692be352832123011b53f1a40d0`。优先通过 Wikidata QID 或 GeoNames 标签关联；否则将 OSM 的 place 节点英文名归一化后与基准英文名精确匹配，且经度差小于 0.06°；如 Wikidata 有坐标，则距离同时小于 10 km。缺少纬度对照时仅凭名称和经度的匹配属于候选，不保证同一地点。查找 `name:zh-Hans`、`name:zh-CN` 或 `name:zh`，存在任一标签即计入；同一地点多对象合并计一次，不等于名称正确或为简体。由于匹配依赖链接和名称，这也是保守的可识别候选数，而非 OSM 全部中文覆盖。

抽查逐条标签后排除了三个明显的错误关联：Wikidata 将 Novena 对应地铁站、将 Kampar 镇对应县；OSM 的 Jurong Town 候选对应裕廊岛。Novena 与 Kampar 仍有按名称和坐标找到的 OSM 地点候选；Jurong 的 Wikidata 地区条目保留。其他潜在混淆尚未完成全面的地理实体核验。

## 结果

| 地区 | 样本 | 原目录含汉字 | Wikidata ID 匹配，剔除明显误配 | Wikidata 中文标签 | OSM 找到候选 | OSM 中文标签 | 任一来源中文候选 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 马来西亚 | 15 | 7 | 9 | 8 | 11 | 11 | 11 |
| 新加坡 | 15 | 8 | 4 | 3 | 7 | 7 | 8 |
| 约旦 | 15 | 4 | 12 | 7 | 8 | 3 | 7 |
| 文莱 | 7 | 2 | 6 | 3 | 7 | 3 | 3 |
| 冰岛 | 6 | 5 | 6 | 5 | 6 | 6 | 6 |
| 合计 | 58 | 26 | 37 | 26 | 39 | 30 | 35 |

任一来源有中文候选的 35 条中仍可能存在错误或繁体；23 条未找到不代表这些来源没有对应地点或中文名。匹配失败不应直接理解为数据源没有该地点。统计针对本次查找方式，没有用于全球替换的代表性。原目录含汉字只检查字符，并未验证译名正确。

## 具体问题与替换判断

- **准确性**：仅靠第三方 ID 或共享 QID 不足以确保同一层级的地点。应交叉验证实体类型、国家、父级、坐标和名称，处理城区、城镇、行政区、车站、岛屿的差异；低置信度候选人工审阅。
- **中文**：Wikidata `zh` 和 OSM `name:zh` 可能是繁体或其他中文变体，不能直接视为经过确认的简体译名；见 [OSM 中文名称标记](https://wiki.openstreetmap.org/wiki/Key:name:zh)。可以优先明确的简体标签，但不可将自动简繁转换等同于规范译名核验。
- **坐标与时区**：Wikidata 提供 WGS84 地理坐标；OSM 边界对象不总有现成代表点，应确认坐标含义。当前时区仍需单独关联 IANA 数据，简化边界或现今边界不能保证历史归属准确。见 [Wikidata 坐标属性](https://www.wikidata.org/wiki/Property:P625)、[Timezone Boundary Builder](https://github.com/evansiroky/timezone-boundary-builder)。
- **分发**：Wikidata 数据采用 [CC0](https://www.wikidata.org/wiki/Wikidata:Licensing)；OSM 数据采用 [ODbL](https://www.openstreetmap.org/copyright)，需保留署名并评估数据库共享义务。混合数据要分别保留来源和处理方式。
- **建议**：把 Wikidata 与 OSM 作为新目录的候选来源，用全球分层样本验证地点、中文、经度、国家地区归属与时区后，再决定替换。优先通过可靠标识匹配，并建立冲突清单、重复检查及坐标核验；不可通过增加来源就默认覆盖完整。现阶段不修改应用数据。

## 样本明细

下表为检索到的中文标签候选，空格用“—”表示未找到或已排除明显误匹配；同一地点多个变体以斜线分隔，不代表已经校正或选定正式名称。原目录首列为 GeoNames ID。

| 国家代码 | 原 ID | 原英文名 | 原目录名称 | Wikidata 中文候选 | OSM 中文候选 |
| --- | --- | --- | --- | --- | --- |
| MY | 1735161 | Kuala Lumpur | 吉隆坡 | 吉隆坡 | 吉隆坡 / 吉隆坡联邦直辖区 |
| MY | 1732752 | Johor Bahru | 新山 | 新山 | 新山 |
| MY | 1771023 | Kampung Baru Subang | Kampung Baru Subang | — | — |
| MY | 1735158 | Petaling Jaya | 八打灵再也 | 八打灵再也 | 八打灵再也 |
| MY | 1734634 | Ipoh | 怡保 | 怡保 | 怡保 |
| MY | 1732903 | Shah Alam | 沙亞南 | 莎阿南 | 莎亚南 |
| MY | 13118272 | Bandar Seri Alam | Bandar Seri Alam | — | 至达城 |
| MY | 1769014 | Ijok | Ijok | — | 依约 |
| MY | 1733697 | Semporna | 仙本那 | 仙本那 | 仙本那 |
| MY | 7566098 | Kampung Perak | Kampung Perak | — | — |
| MY | 1734399 | Selama | Selama | — | — |
| MY | 1738412 | Ulu Kelang | Ulu Kelang | — | — |
| MY | 1734881 | Batu Anam | Batu Anam | 峇都安南 | 峇都安南 |
| MY | 1735274 | Kampar | Kampar | — | 金宝 / 金宝县 |
| MY | 1736870 | Engkilili | 英吉利裏 | 英吉利里 | 英吉利里 |
| SG | 1880252 | Singapore | 新加坡 | — | 新加坡 |
| SG | 1880159 | Ulu Bedok | Ulu Bedok | — | — |
| SG | 1884382 | Bedok New Town | 勿洛 | — | — |
| SG | 7289760 | Sengkang New Town | 盛港 | 盛港 | 盛港 |
| SG | 1880216 | Tampines Estate | Tampines Estate | — | — |
| SG | 1881953 | Jurong Town | 裕廊 | 裕廊 | — |
| SG | 1880348 | Punggol | 榜鹅 | 榜鹅 / 榜鵝 | 榜鹅 / 北岸 |
| SG | 1880594 | Kembangan | Kembangan | — | 景万岸 |
| SG | 1884386 | Queenstown Estate | 女皇镇 | — | — |
| SG | 7281987 | Novena | 诺维娜 | — | 诺维娜 |
| SG | 1881957 | Kaki Bukit Estate | Kaki Bukit Estate | — | — |
| SG | 1880184 | Thomson | Thomson | — | 汤申 |
| SG | 13118140 | Teck Whye | Teck Whye | — | 德惠 |
| SG | 1880226 | Sungai Simpang | 新邦 | — | — |
| SG | 1880177 | Tiong Bahru Estate | Tiong Bahru Estate | — | — |
| JO | 250441 | Amman | 安曼 | 安曼 | 安曼 |
| JO | 250090 | Zarqa | 扎尔卡 | 扎尔卡 | — |
| JO | 248946 | Irbid | 伊爾比德 | 伊尔比德 / 伊爾比德 | 伊爾比德 |
| JO | 7838895 | Russeifa | Russeifa | 鲁赛法 | — |
| JO | 248583 | Ḩayy Khildā | Ḩayy Khildā | — | — |
| JO | 248460 | Khuraybat as Sūq | Khuraybat as Sūq | — | — |
| JO | 13286467 | Ṣuwayliḥ | Ṣuwayliḥ | — | — |
| JO | 250774 | Aqaba | 亞喀巴 | 亚喀巴 / 亞喀巴 | 亚喀巴 |
| JO | 248370 | Mādabā | Mādabā | 米底巴 | — |
| JO | 248382 | Ma'an | Ma'an | 马安 / 馬安 | — |
| JO | 250461 | Ḩayy al Quwaysimah | Ḩayy al Quwaysimah | — | — |
| JO | 248923 | ‘Izrā | ‘Izrā | — | — |
| JO | 248803 | Judita | Judita | — | — |
| JO | 246314 | Umm as Summāq | Umm as Summāq | — | — |
| JO | 250267 | Safi | Safi | — | — |
| BN | 1820906 | Bandar Seri Begawan | 斯里巴加湾市 | 斯里巴加湾市 / 斯里巴加灣市 | 斯里巴加湾市 |
| BN | 1820575 | Sengkurong | Sengkurong | — | — |
| BN | 1820635 | Mentiri | Mentiri | — | — |
| BN | 1820491 | Kuala Belait | 马来奕 | 马来奕 | 瓜拉马来奕 |
| BN | 1820071 | Tutong | Tutong | 北干都东 / 都東城 | 都东 |
| BN | 1820574 | Serasa | Serasa | — | — |
| BN | 1820557 | Liang | Liang | — | — |
| IS | 3413829 | Reykjavík | 雷克亞維克 | 雷克雅未克 | 雷克雅未克 / 雷克雅維克 |
| IS | 3415212 | Kópavogur | 科帕沃于尔 | 科帕沃于尔 | 科帕沃于尔 |
| IS | 3416706 | Hafnarfjörður | 哈夫纳夫约杜尔 | 哈布纳菲厄泽 | 哈布纳菲厄泽 |
| IS | 8644037 | Reykjanesbær | Reykjanesbær | — | 雷恰内斯拜尔 |
| IS | 2633274 | Akureyri | 阿克雷里 | 阿克雷里 | 阿克雷里 |
| IS | 3415496 | Keflavík | 凯夫拉维克 | 凯夫拉维克 | 凯夫拉维克 |

## 扩展抽样（2026-11-11）

为补充中美洲、南美洲、非洲、欧洲与东南亚，分别从哥斯达黎加、乌拉圭、加纳、葡萄牙和老挝按前述人口分层方法各抽取 15 条，共 75 条。此批采用同样的 Wikidata P1566 查询和 OSM 对象匹配规则。下表为**自动关联后的中文标签候选数**，未经全部逐项核验，不能视为真实覆盖率；不同地点层级、名称相同但坐标不一致的条目可能混入。

| 地区 | 样本 | 原目录含汉字 | Wikidata ID 关联 | Wikidata 中文候选 | OSM 对象候选 | OSM 中文候选 | 任一来源中文候选 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 哥斯达黎加 | 15 | 4 | 14 | 10 | 15 | 1 | 10 |
| 乌拉圭 | 15 | 14 | 15 | 15 | 15 | 1 | 15 |
| 加纳 | 15 | 3 | 13 | 5 | 14 | 3 | 5 |
| 葡萄牙 | 15 | 5 | 12 | 1 | 14 | 4 | 4 |
| 老挝 | 15 | 3 | 12 | 10 | 12 | 10 | 11 |
| 合计 | 75 | 29 | 66 | 41 | 70 | 19 | 45 |

上述自动数值**尚未剔除疑似冲突**：哥斯达黎加 San Rafael Abajo 的现有经度为 -84.29008，而相同 GeoNames ID 关联到的 Wikidata 地点经度为 -84.0836529；两者是否同一地点仍待核实。老挝 Muang Sing 的 Wikidata 中文候选为“勐新县”，OSM 还存在镇与县的不同对象，不能未经核验将县名写入镇记录。因此这 45 条不能直接作为可发布数据。

葡萄牙 Porto 也说明仅凭 GeoNames ID 会低估中文可用性：匹配到的 [城市条目 Q33707919](https://www.wikidata.org/wiki/Q33707919) 没有本次查询的中文标签，而另有 [市镇条目 Q36433](https://www.wikidata.org/wiki/Q36433)。两个条目有不同的实体边界，不能自动将后者当作前者的同义项。

本次 OSM 来源为 Geofabrik 的 [哥斯达黎加](https://download.geofabrik.de/central-america/costa-rica.html)、[乌拉圭](https://download.geofabrik.de/south-america/uruguay.html)、[加纳](https://download.geofabrik.de/africa/ghana.html)、[葡萄牙](https://download.geofabrik.de/europe/portugal.html)和[老挝](https://download.geofabrik.de/asia/laos.html) PBF，SHA-256 按上述顺序分别为：

- 哥斯达黎加：`532f05613570f00e0f1e560b49a3d66e4a7747d385a25bcc2e8c96ccbc8cce1e`
- 乌拉圭：`f56784ee26a06e406efb0f19f467216168d5beb999a25311b76d0a23ab740409`
- 加纳：`d4fadc1c124ee89b058bba303553f59451c6cd9ab0d687d22ba38bbf0c48a3ac`
- 葡萄牙：`926f6f524f37e27cba59fca5d8d048deef0b18b08b58ba11be4bbc5b1581a7d6`
- 老挝：`c823a843fa829417236d922a11408f4ee0a17c558993b68376839923543fb842`

### 可用于下一阶段的全球快照

[Wikidata Gazetteer](https://huggingface.co/datasets/yuiseki/wikidata-gazetteer/tree/ef7674b5b00f2af07e792332b1d2d9f8ffccdff1) 提供以 2026-08-31 Wikidata dump 构建的地点与多语言名称 Parquet；该版本标识为 `ef7674b5b00f2af07e792332b1d2d9f8ffccdff1`，数据声明 CC0。它包含 QID、坐标、行政关系、类型与名称，可作为构建离线候选目录的输入；样本中仍使用 GeoNames ID 作为对照，生产目录可以独立使用 QID。

该快照包含建筑、车站及自然地物等，不是城市清单；P17 国家字段只保留首值，P31 类别还需建立子类闭包或人工审核的白名单。来源纳入规则包括带坐标或带 GeoNames ID 的实体，因此也应记录筛选偏差。快照已下载并用 Parquet 查询验证可读取；未对全部国家、行政归属或经纬度进行验证。下一步可据此建立独立生成器和可审计候选数据，再运行更大规模验证；在可接受覆盖率及错配规则明确前，不覆盖应用资源。

### 扩展样本明细

中文名称为自动候选，未完成逐项审定；多个候选以斜线分隔。原目录 ID 仅用于抽样对照。

| 国家代码 | 原 ID | 原英文名 | 原目录名称 | Wikidata 中文候选 | OSM 中文候选 |
| --- | --- | --- | --- | --- | --- |
| CR | 3621849 | San José | 聖荷西 | 圣何塞 | 聖荷西 |
| CR | 3622247 | Limón | 利蒙 | 利蒙 | — |
| CR | 3621911 | San Francisco | San Francisco | — | — |
| CR | 3624955 | Alajuela | 阿拉胡埃拉 | — | — |
| CR | 3623076 | Liberia | 利韋里亞 | 利韦里亚 / 利韋里亞 | — |
| CR | 3622547 | Paraíso | Paraíso | 帕賴索 | — |
| CR | 3623977 | Curridabat | Curridabat | 庫里達巴特 | — |
| CR | 3622217 | Purral | Purral | 普拉爾區 | — |
| CR | 3621659 | San Rafael Abajo | San Rafael Abajo | 聖拉斐爾阿瓦霍區 | — |
| CR | 3624288 | Chacarita | Chacarita | — | — |
| CR | 3624848 | Aserrí | Aserrí | 阿塞里區 | — |
| CR | 3621335 | Tejar | Tejar | — | — |
| CR | 3624468 | Cañas | Cañas | 卡尼亞斯區 | — |
| CR | 3624174 | Colima | Colima | — | — |
| CR | 3621655 | San Rafael Arriba | San Rafael Arriba | 聖拉斐爾阿里瓦區 | — |
| UY | 3441575 | Montevideo | 蒙得维的亚 | 蒙得维的亚 | 蒙得維的亞 / 蒙得维的亚省 |
| UY | 3440714 | Salto | 萨尔托 | 萨尔托 | — |
| UY | 3441894 | Maldonado | 馬爾多納多 | 馬爾多納多 | — |
| UY | 3440781 | Rivera | 里韦拉 | 里韦拉 | — |
| UY | 3441243 | Paysandú | 派桑杜 | 派桑杜 | — |
| UY | 3442057 | Las Piedras | 拉斯彼德拉斯 | 拉斯彼德拉斯 | — |
| UY | 3441702 | Melo | 梅洛 | 梅洛 | — |
| UY | 3442727 | Durazno | 杜拉斯諾 | 杜拉斯诺 / 杜拉斯諾 | — |
| UY | 3440639 | San José de Mayo | 聖何塞德馬約 | 圣何塞德马约 / 聖何塞德馬約 | — |
| UY | 3440696 | San Carlos | 聖卡洛斯 | 聖卡洛斯 | — |
| UY | 3442568 | Fray Bentos | 弗賴本托斯 | 弗赖本托斯 / 弗賴本托斯 | — |
| UY | 3441354 | Pando | 潘多 | 潘多 | — |
| UY | 3443341 | Carmelo | 卡梅洛 | 卡梅洛 (烏拉圭) | — |
| UY | 3439525 | Young | 楊格 | 楊格 | — |
| UY | 3441292 | Paso de Carrasco | Paso de Carrasco | 帕索德卡拉斯科 | — |
| GH | 2298890 | Kumasi | 库马西 | 库马西 | 库马西 |
| GH | 2306104 | Accra | 阿克拉 | 阿克拉 | 阿克拉 |
| GH | 2294877 | Tamale | Tamale | 塔马利 | 塔馬利 |
| GH | 2294915 | Takoradi | Takoradi | — | — |
| GH | 11808941 | Sekondi | Sekondi | — | — |
| GH | 2302357 | Cape Coast | 海岸角 | 海岸角 | — |
| GH | 2298330 | Medina Estates | Medina Estates | — | — |
| GH | 2294206 | Wa | Wa | 瓦城 | — |
| GH | 2294962 | Swedru | Swedru | — | — |
| GH | 12640506 | Nkawkaw | Nkawkaw | — | — |
| GH | 11467879 | Assin Foso | Assin Foso | — | — |
| GH | 2295692 | Sakumona | Sakumona | — | — |
| GH | 2293805 | Yegyi | Yegyi | — | — |
| GH | 2301424 | Dambai | Dambai | — | — |
| GH | 2303258 | Bechem | Bechem | — | — |
| PT | 2267057 | Lisbon | 里斯本 | 里斯本 | 里斯本 |
| PT | 2735943 | Porto | 波圖 | — | 波尔图 |
| PT | 2742032 | Braga | Braga | — | 布拉加 |
| PT | 2271772 | Amadora | Amadora | — | — |
| PT | 2740637 | Coimbra | 科英布拉 | — | — |
| PT | 2267095 | Leiria | Leiria | — | — |
| PT | 2735083 | Rio Tinto | 里奧廷托 | — | — |
| PT | 2732773 | Viana do Castelo | Viana do Castelo | — | 维亚纳堡 |
| PT | 2741667 | Campanhã | 坎帕尼扬 | — | — |
| PT | 2265326 | Paço de Arcos | Paço de Arcos | — | — |
| PT | 11886964 | Parque das Nações | Parque das Nações | — | — |
| PT | 2742504 | Baião | Baião | — | — |
| PT | 2263523 | Santa Iria da Azóia | Santa Iria da Azóia | — | — |
| PT | 2734363 | São Pedro da Cova | São Pedro da Cova | — | — |
| PT | 2262582 | Torres Novas | Torres Novas | — | — |
| LA | 1651944 | Vientiane | 万象 | 万象 / 永珍 | 万象 |
| LA | 1653316 | Savannakhet | Savannakhet | 凯山丰威汉市 | 凯山丰威汉市 |
| LA | 1655199 | Thakhèk | Thakhèk | 他曲 | 他曲 |
| LA | 1654379 | Pakse | 巴色 | 巴色 | 巴色/百細 |
| LA | 11244752 | Ban Khoan | Ban Khoan | — | — |
| LA | 1652203 | Xam Nua | Xam Nua | 桑怒 | — |
| LA | 1655559 | Luang Prabang | 琅勃拉邦 | 琅勃拉邦 | 琅勃拉邦 |
| LA | 1655123 | Muang Phônsavan | Muang Phônsavan | 丰沙湾 / 豐沙灣 | 丰沙湾 |
| LA | 1655078 | Muang Xay | Muang Xay | 芒赛 | 孟赛 |
| LA | 1655087 | Vang Vieng | Vang Vieng | 万荣 / 旺陽 | 万荣 |
| LA | 1655107 | Muang Sing | Muang Sing | 勐新县 | 孟辛 / 辛县 |
| LA | 1662589 | Ban Laongam | Ban Laongam | — | — |
| LA | 9948272 | Sekong | Sekong | — | 塞公 |
| LA | 1657897 | Borikhan | Borikhan | — | — |
| LA | 1904391 | Ban Houakhoua | Ban Houakhoua | — | — |

## 全球快照的直接类型试算

已下载并读取固定版本的两张 Parquet；`places.parquet` SHA-256 为 `9664e36775afd7191cfd265f0d6c81b213521326d7abf543fcd0a8fc576ca4cc`，`names.parquet` 为 `c58e8f2b627d22aabe0855cfd3f2874bc8f0ee5c00634bb85b99310aec366346`。此步骤使用临时 DuckDB，未更改应用或项目依赖。

仅选有经纬度且 P31 **直接包含** city（Q515）、town（Q3957）或 village（Q532）之一的条目；中文候选只计 `kind=label` 且 `lang` 为 `zh`、`zh-hans` 或 `zh-cn` 的条目。下面每类单独计数，同一条目可能属于多个类型，不能相加。中文标签没有人工审核，也未检查是否为简体：

| 直接类型 | 有坐标条目 | 有中文标签 | 比例 |
| --- | ---: | ---: | ---: |
| city（Q515） | 8,285 | 5,966 | 72.0% |
| town（Q3957） | 15,941 | 6,160 | 38.6% |
| village（Q532） | 200,839 | 6,964 | 3.5% |
| 上述三类中人口字段至少 15,000 | 6,038 | 5,140 | 85.1% |

三种类型合并去重为 224,661 条，其中 18,814 条有中文标签。人口字段为空的地点不会进入人口筛选，因此最后一行不是所有人口超过 15,000 的地点。

**这些数字不是全球城市覆盖率。** 各国特有城市类型可能通过 P279 子类关系归入城市，而未直接标记为 Q515；本次没有展开该层级，也没有排除历史地名或非海外条目。人口、国家和城市类型都有缺失或可能失准。试算只表明快照可以离线处理，且小地点的中文标签缺口可能很大，不能据此发布替换目录。

复算逻辑：对 `places.parquet` 筛选 `lat IS NOT NULL AND lon IS NOT NULL`，用 `list_contains(instance_of, 'Q515')` 等分别统计；对 `names.parquet` 筛选上述语言与 `kind='label'`，按 QID 去重后左连接。人口筛选仅针对三个直接类型的合并集合，条件为 `population >= 15000`。

## 2026-11-11 离线候选进展

后续建立的 [海外候选快照](../../tools/data-sources/overseas-candidates-2026-11-11.json)包含 39,922 条有坐标的候选，其中 26,225 条找到中文标签；这些不是经过认证的城市数，也不代表可以直接上线。美国及日本城市使用补充的类型白名单；另外按国家或地区首府声明补入原筛选漏掉的小地区候选。

独立使用 Natural Earth 点面结果交叉检查后，仍有 763 条的 Wikidata 国家声明无法唯一映射到当前 ISO 名录，988 条未落入 Natural Earth 国家面，194 条唯一国家声明与点面结果不同，936 条没有一级地区面。310 条候选命中多个时区边界；全部 40,232 个时区候选标识可以由应用当前使用的 `chrono-tz` 解析，但历史时区归属未核验。不同统计可以重叠；Natural Earth 是概括地图，不能以这些数字直接判定 Wikidata 错误。

后续已将五批共 14,588 条经过跨源匹配的海外记录分阶段接入；其余候选仍未接入应用。继续迁移前应核对地区覆盖损失、重名和实体类型、国家及地区归属、中文名称和多时区选择，并提供未收录地点的输入方式。 [审计工具和来源说明](../../tools/data-sources/README.md#海外地点候选快照未接入应用)列出数据来源及运行方法。

## 2026-11-11 后续补名与缺口

本轮基于 Wikidata 的多语言标签和别名、國家教育研究院的开放译名镜像与 Unicode CLDR 时区城市名补充名称。匹配按来源分别检查国家、名称、地点类型、距离及跨批实体唯一性；其中部分代表点相距 2–5 公里，结果仍需人工核验。发现的车站误关联和重复实体关联已记录在数据来源目录，并撤回对应名称；乌布、林梦等错误设施名称也已修正。具体快照、来源、重放和校验命令见[数据来源说明](../../tools/data-sources/README.md#按地点名称与类型补充中文名)。

当前海外目录含 31,871 条，显示名含汉字 23,966 条，不含汉字 7,905 条；这只是字符统计，不能代表中文翻译准确或每条都已人工审核。`node tools/overseas-name-coverage.mjs --places` 可列出当前缺口。用户选择只使用可追溯的现成中文名，未找到译名的地点暂保留原文。维基数据在线复查曾发生 429 限流，后续降低频率继续复查；剩余条目不能推断为不存在译名。

后来对 GeoNames 带语言标记的中文别名及 Geofabrik 的马来西亚、新加坡、文莱、老挝等地点节点进行补充核对，新增 8 条 GeoNames 别名和 22 条 OSM 中文名，仍保留原地点标识、经度与时区。

另从 Geofabrik 泰国的现成 OSM 地点节点补充 1 条中文名。

越南 Geofabrik OSM 地点节点补充 5 条中文名；新西兰同类快照未找到符合匹配条件的新名称。

### 其他候选资料

[外文译名词典](https://nametochinese.com/places)称收录约 17 万条地名，其[服务条款](https://nametochinese.com/terms)不允许未经许可批量抓取、复制或转售；网站还说明译名可能来自公开资料或规则算法，因此未将其作为可批量导入的可追溯来源。[CJKI 中英地名库](https://www.cjk.org/data/chinese/proper/chinese-english-database-place-names/)是商业授权候选，尚未取得使用权。

用户后续将范围调整为各国首都、重要城市和海外华人常用城市。当前选择器采用独立主要城市清单；上文全球缺口统计属于保留的完整历史目录，并非当前可选城市的中文缺口。筛选条件、自动匹配限制及重建方式见[数据来源说明](../../tools/data-sources/README.md#海外主要城市选择)。
