# 日期取数：Rust 历法库候选核查

查询日期：2026-11-10。本文是选型研究，未将候选库接入产品，未修改产品 Cargo 依赖，也不表示计算方案已采纳。

后续已开展固定版本的[逐日历法验证](tyme4rs-calendar-validation.md)，包含官方历表比较与宽范围逐日失败位置；适用性判断应同时阅读该报告。用户已确定真太阳时与干支反推自行实现，统一放入独立历法包 `zhiji_calendar`。

## 结论

建议优先继续验证 **tyme4rs 1.5.1 作为历法底座**。它具备本项目需要的公农历转换、农历月事实、秒级节气和干支原语，维护也较活跃；但其 Rust 四柱反查在本地非子时抽样中返回空结果，不能把“有反查 API”视为可以直接使用。若保留四柱反查需求，当前三个候选均不能宣布满足全部要求。

候选顺序为：

1. **tyme4rs 1.5.1**：最值得优先验证的历法底座；反查和晚子时接口须单独处理。
2. **lunar-lite 1.3.0**：只取转换结果、闰月和月天数时的窄替代；错误类型与边界说明较清楚，但公开四柱接口不适合精确出生时刻，也没有反查接口。
3. **lunar_rust 1.0.1**：四柱选项和反查入口较全，但发布较旧、数值边界缺失，内部混用儒略历与 Chrono 日期，且反查实现不适合直接承担完整范围的桌面查询。

这是基于源码接口与风险的研究排序，不是准确率排名。三个库并非三个独立算法证据：Tyme 引用寿星天文历，lunar_rust 是 lunar-python 的移植，lunar-lite 的天文内核又改编自 Tyme。互相一致只能证明部分兼容性，不能证明三者共同正确。[Tyme 来源说明](https://github.com/6tail/tyme4rs/blob/84fec158fb7b054b67c5862c58166a4d5f9050ff/README.md)、[lunar_rust 来源说明](https://github.com/flameoftheforest/lunar_rust/blob/f075e601a75170d2dbd8f45b5e6386e97b4f3cb9/README.md)、[lunar-lite 来源与准确性限制](https://github.com/JinyangWang27/lunar-lite-rs/blob/1b255918fa9a054fb469933b5383867b248d98fa/src/lib.rs#L20-L46)。

## 本项目的判断边界

本次用户确认的用途是从日期回填现有手动排盘参数；月份、时辰和生年四化仍是当前手动模型。计算方案尚在讨论，不把历法库的八字结果整体当作紫微排盘结果。

- 生年换年可选正月初一或立春，默认正月初一。
- 闰月初一至十五按本月，十六以后按下月，日数保留原值，不减十五。
- 公历晚子时可选当日或次日，默认当日。
- 公历有经度时用真太阳时，无经度时用钟表时间。
- 农历输入只有十二时辰，不拆早子、晚子。
- 年份范围随最终历法底座及验证结果确定；本研究不预先替产品承诺某个完整区间。

这些是应用规则。候选库的默认四柱流派、闰月月柱规则和时辰索引均不能覆盖或改写它们。尤其要区分“真实农历日期”与“回填手动排盘的有效月份”：后者的闰月折算应在获得前者之后执行。

## 发布版本和来源固定

以下“当前版本”是查询当日 crates.io API 返回的最新稳定、未撤回版本；源码使用相应发布包，不使用浮动的 `master` 或其他语言端口推断 Rust 行为。读取的 16 个关键源文件已逐一与 `.crate` 归档内容核对一致，归档 SHA-256 与 crates.io 元数据一致。

| 库 | 当前发布版与时间（UTC） | 发布包记录的 Git 提交 | 许可证 |
| --- | --- | --- | --- |
| tyme4rs | 1.5.1，2026-11-05 | [`84fec158fb7b054b67c5862c58166a4d5f9050ff`](https://github.com/6tail/tyme4rs/tree/84fec158fb7b054b67c5862c58166a4d5f9050ff) | [MIT](https://github.com/6tail/tyme4rs/blob/84fec158fb7b054b67c5862c58166a4d5f9050ff/LICENSE) |
| lunar-lite | 1.3.0，2026-07-03 | [`1b255918fa9a054fb469933b5383867b248d98fa`](https://github.com/JinyangWang27/lunar-lite-rs/tree/1b255918fa9a054fb469933b5383867b248d98fa) | [MIT](https://github.com/JinyangWang27/lunar-lite-rs/blob/1b255918fa9a054fb469933b5383867b248d98fa/LICENSE)，另保留 [Tyme 等第三方声明](https://github.com/JinyangWang27/lunar-lite-rs/blob/1b255918fa9a054fb469933b5383867b248d98fa/THIRD_PARTY_LICENSES.md) |
| lunar_rust | 1.0.1，2023-12-07 | [`f075e601a75170d2dbd8f45b5e6386e97b4f3cb9`](https://github.com/flameoftheforest/lunar_rust/tree/f075e601a75170d2dbd8f45b5e6386e97b4f3cb9) | [MIT](https://github.com/flameoftheforest/lunar_rust/blob/f075e601a75170d2dbd8f45b5e6386e97b4f3cb9/LICENSE) |

发布信息来源：[tyme4rs API](https://crates.io/api/v1/crates/tyme4rs)、[lunar-lite API](https://crates.io/api/v1/crates/lunar-lite)、[lunar_rust API](https://crates.io/api/v1/crates/lunar_rust)。具体发布包也可从版本页复查：[tyme4rs 1.5.1](https://crates.io/crates/tyme4rs/1.5.1)、[lunar-lite 1.3.0](https://crates.io/crates/lunar-lite/1.3.0)、[lunar_rust 1.0.1](https://crates.io/crates/lunar_rust/1.0.1)。

归档校验值：

```text
tyme4rs 1.5.1   0a32c38e8ea6464a418f6c89dad69f2fdb505173c2f015e3d7d44a2122ea2191
lunar-lite 1.3.0 2c9ab4e5fcc2bf83c2e49b948242c2307920daaadf1c64071500727d5ca19d6c
lunar_rust 1.0.1 ef0dbbe4fe9efb022a7dc7960397e22cf061c784a89e09b7a26690f5d5bbafda
```

## 数值范围与历制

必须分开三类范围：构造器接受的数值区间、双向转换实际能走通的日期集合、独立资料能验证的准确区间。它们不能互相替代。

| 核查项 | tyme4rs 1.5.1 | lunar-lite 1.3.0 | lunar_rust 1.0.1 |
| --- | --- | --- | --- |
| 公历数值年界 | `SolarYear::validate` 为 **1..=9999** | 转换 API 明确为 **1..=9999** | 输入类型为 `i64`，但未声明或校验可承诺的完整年界；**不能写支持 i64 全域或擅定 ±9999** |
| 农历数值年界 | `LunarYear::validate` 为 **-1..=9999**，不等于每个农历日都能转为合法公历 | 月份事实 API 为 **-1..=9999**；农历转公历还要求结果落在公历 1..=9999 | `LunarYear::from_lunar_year(i64)` 没有显式数值年界；实际转换受内部运算及 Chrono 等约束，支持区间待测 |
| 农历端点 | 农历年与公历年不对齐；两端必须按日期验证，不能仅看年份构造成功 | 文档明确农历 -1 年全部早于公历 1 年，完整转换会报 `SolarYearOutOfRange`；农历 9999 年末也须受公历结果上界约束 | 无可直接作为产品契约的农历端点定义 |
| 公历改历 | 1582-10-04 后接 10-15；拒绝 10-05 至 10-14；之前按儒略历 | 明确与 Tyme 相同的混合历制及十天空档 | 儒略日、闰年、日期步进采用混合历制，但部分计算转成 Chrono 日期，存在历制冲突风险 |
| 全区间准确性证据 | 官方网页称“约 1 到 9999 年”，本次未找到覆盖整个数值区间的独立验证报告 | 作者明确：全区间表示确定的 Tyme 兼容模型结果，不表示每年都具历史或天文学权威性 | 本次未找到完整数值范围及其独立准确性报告 |

范围依据：[Tyme 公历校验与闰年](https://github.com/6tail/tyme4rs/blob/84fec158fb7b054b67c5862c58166a4d5f9050ff/src/tyme/solar.rs#L55-L107)、[Tyme 公历日校验](https://github.com/6tail/tyme4rs/blob/84fec158fb7b054b67c5862c58166a4d5f9050ff/src/tyme/solar.rs#L910-L935)、[Tyme 农历年校验](https://github.com/6tail/tyme4rs/blob/84fec158fb7b054b67c5862c58166a4d5f9050ff/src/tyme/lunar.rs#L102-L115)、[Tyme 作者文档](https://6tail.cn/tyme.html)、[lunar-lite 范围和模型说明](https://github.com/JinyangWang27/lunar-lite-rs/blob/1b255918fa9a054fb469933b5383867b248d98fa/src/lib.rs#L20-L46)、[lunar_rust 公历构造](https://github.com/flameoftheforest/lunar_rust/blob/f075e601a75170d2dbd8f45b5e6386e97b4f3cb9/src/lib/solar.rs#L256-L297)、[lunar_rust 农历年构造](https://github.com/flameoftheforest/lunar_rust/blob/f075e601a75170d2dbd8f45b5e6386e97b4f3cb9/src/lib/lunar_year.rs#L411-L439)、[lunar_rust 闰年与改历](https://github.com/flameoftheforest/lunar_rust/blob/f075e601a75170d2dbd8f45b5e6386e97b4f3cb9/src/lib/util/solar_util.rs#L78-L129)。

因此，界面若写“公历”，需明确是否接受上述混合历制。当前项目 `src/workspace/input/model.rs` 的 `CivilDate` 和 `month_length` 使用前推格里高利历规则：1500 年不是闰年，1582 年十月也不排除十天空档。这与候选库不一致，是接入前必须决定的输入契约；不能把日期字段直接传过去。古代日期还涉及当时实际施行的中国历法，算法的现代规则外推不能自动代表历史颁历结果。

## 能力对照

| 能力 | tyme4rs 1.5.1 | lunar-lite 1.3.0 | lunar_rust 1.0.1 |
| --- | --- | --- | --- |
| 公历 → 农历 | `SolarDay::get_lunar_day()` | `solar_to_lunar(SolarDate) -> Result` | `solar::from_ymd(...).get_lunar()` |
| 农历 → 公历 | `LunarDay::get_solar_day()` | `lunar_to_solar(LunarDate) -> Result` | `lunar::from_ymd(...).get_solar()` |
| 闰月、月天数 | 支持；闰月以负月份表示 | `leap_month`、`lunar_month_days`、`validate_lunar_date` | 支持；闰月以负月份表示 |
| 节气 | 公开 `SolarTerm`；日期口径和精确时刻口径分开 | 公开秒级 `li_chun_datetime`；其他节气计算主要是内部模块，不能等同通用公开节气 API | `get_jie_qi_table` 等接口提供节气日期时刻 |
| 干支与换年 | 农历年干支与节气干支对象分开；有精确时刻的四柱 | 有农历年干支、四柱；`YearDivide::Exact` **仅按立春日期换年** | 有正月初一、立春日、立春交接时刻等独立年干支口径 |
| 晚子时自定义 | 默认次日；公开 `LunarSect2EightCharProvider` 可直接算当天日柱；Rust 无公开全局 provider setter | 晚子索引 12 固定将日柱加一天，公开选项不含日界切换 | `EightChar::set_sect(2)` 当天，`1` 次日；默认 2 |
| 四柱反查 | `EightChar::get_solar_times(start_year,end_year)`；本地已复现非子时漏解，详见下节 | **未提供公开反查 API** | `solar::from_bazi(...,sect,base_year)`；没有显式结束年参数，受当前系统日期影响 |
| 真太阳时 | 本次核查未见接收经度和均时差修正的公开入口 | 作者明确不做经度、时区或真太阳时修正 | 本次核查未见接收经度和均时差修正的公开入口 |

转换与公开 API 依据：[Tyme 公转农](https://github.com/6tail/tyme4rs/blob/84fec158fb7b054b67c5862c58166a4d5f9050ff/src/tyme/solar.rs#L1125-L1133)、[Tyme 农历类型](https://github.com/6tail/tyme4rs/blob/84fec158fb7b054b67c5862c58166a4d5f9050ff/src/tyme/lunar.rs)、[Tyme 节气时刻与日历日期](https://github.com/6tail/tyme4rs/blob/84fec158fb7b054b67c5862c58166a4d5f9050ff/src/tyme/solar.rs#L1806-L1855)、[lunar-lite 导出接口](https://github.com/JinyangWang27/lunar-lite-rs/blob/1b255918fa9a054fb469933b5383867b248d98fa/src/lib.rs#L95-L107)、[lunar-lite 四柱规则](https://github.com/JinyangWang27/lunar-lite-rs/blob/1b255918fa9a054fb469933b5383867b248d98fa/src/four_pillars.rs#L1-L25)、[lunar_rust 换年实现](https://github.com/flameoftheforest/lunar_rust/blob/f075e601a75170d2dbd8f45b5e6386e97b4f3cb9/src/lib/lunar.rs#L272-L329)、[lunar_rust API 文档](https://docs.rs/lunar_rust/1.0.1/lunar_rust/)。

时间语义也要单独定义。Tyme 系内核的节气使用中国标准时口径；lunar-lite 文档明确为 UTC+8 钟表值，不自动感知本地时区、经度或真太阳时。不能将系统本地 `DateTime` 与节气钟表值不经换算地比较。[lunar-lite 节气时刻契约](https://github.com/JinyangWang27/lunar-lite-rs/blob/1b255918fa9a054fb469933b5383867b248d98fa/src/solar_term_datetime.rs#L13-L58)。

## 会改变选型判断的实现限制

### tyme4rs：反查与流派接口不能照搬其他语言文档

`get_solar_times` 虽接收起止年，但发布版源码在算出目标时辰小时 `h` 后执行 `hours.push(0)`，仅当 `h == 0` 时追加 `23`；之后用这些小时构造候选并进行正向八字校验。按此实现，非子时并未枚举目标小时。本地四个非子时抽样均未返回原输入的四柱对应时刻，行为复现见下节；这项能力当前不能直接使用。[固定源码](https://github.com/6tail/tyme4rs/blob/84fec158fb7b054b67c5862c58166a4d5f9050ff/src/tyme/eightchar/mod.rs#L111-L183)。

Rust 的 `EIGHT_CHAR_PROVIDER` 是私有静态对象，当前源码没有公开 setter。可以直接调用公开的 `LunarSect2EightCharProvider::get_eight_char`，取得晚子当天日柱；但内置反查校验调用 `LunarHour::get_eight_char()`，仍走私有默认 provider。因此“正向用自选 provider，反向用内置反查”并不自然保证同一流派。[provider 实现](https://github.com/6tail/tyme4rs/blob/84fec158fb7b054b67c5862c58166a4d5f9050ff/src/tyme/eightchar/provider.rs#L12-L64)、[私有默认对象](https://github.com/6tail/tyme4rs/blob/84fec158fb7b054b67c5862c58166a4d5f9050ff/src/tyme/lunar.rs#L982-L985)、[四柱调用入口](https://github.com/6tail/tyme4rs/blob/84fec158fb7b054b67c5862c58166a4d5f9050ff/src/tyme/lunar.rs#L1138-L1143)。

`new` 常返回 `Result`，`from_*` 便捷构造通常直接 `unwrap()`；转换结果也不是统一的 `Result`。仅对输入年份做校验，不能保证跨农历年端点或后续日期推移不会触发 panic。[公历构造示例](https://github.com/6tail/tyme4rs/blob/84fec158fb7b054b67c5862c58166a4d5f9050ff/src/tyme/solar.rs#L55-L68)、[农历日构造示例](https://github.com/6tail/tyme4rs/blob/84fec158fb7b054b67c5862c58166a4d5f9050ff/src/tyme/lunar.rs#L734-L755)。

### lunar-lite：转换契约清楚，但四柱是兼容接口

它的四柱接口只接收 `time_index` 0..=12；早子、晚子分别为 0、12，并以 `hour=max(index*2-1,0), minute=30` 合成时刻。`YearDivide::Exact` 使用立春日期，`MonthDivide::Exact` 则以合成时刻比较节令到秒。因此它不能直接回答“真实钟表时刻在立春前还是后”，也不能直接承接默认晚子当天的要求。单独的 `li_chun_datetime` 确有秒级立春原语，但这不改变四柱接口的日期级换年契约。[四柱输入与换年](https://github.com/JinyangWang27/lunar-lite-rs/blob/1b255918fa9a054fb469933b5383867b248d98fa/src/four_pillars.rs#L1-L25)、[晚子固定进日](https://github.com/JinyangWang27/lunar-lite-rs/blob/1b255918fa9a054fb469933b5383867b248d98fa/src/four_pillars.rs#L140-L160)、[独立立春接口说明](https://github.com/JinyangWang27/lunar-lite-rs/blob/1b255918fa9a054fb469933b5383867b248d98fa/src/solar_term_datetime.rs#L45-L58)。

其 `lunar_to_solar` 会先规范化农历输入：当年该月份没有闰月时，传入的闰月标志会被清除，而非一律报错。若界面要拒绝“不存在的闰月”，须先使用严格的 `validate_lunar_date`；不能把 `Result` 等同于“所有错误输入均原样拒绝”。[转换入口](https://github.com/JinyangWang27/lunar-lite-rs/blob/1b255918fa9a054fb469933b5383867b248d98fa/src/convert.rs#L35-L53)、[规范化实现](https://github.com/JinyangWang27/lunar-lite-rs/blob/1b255918fa9a054fb469933b5383867b248d98fa/src/normalize.rs#L4-L28)。

### lunar_rust：反查范围、边界漏解与运行成本

`from_bazi` 默认 `sect=2`、`base_year=1900`；通过 `Local::now()` 推算要搜索的六十年周期，而非接收明确的结束年。它对目标时辰只取一个代表整点，子时额外取 23 点，再逐日正向核验；若节气在该时辰内部交接，代表整点可能漏掉交接后的合法时间段。返回结果也不是完整的出生时间区间。[反查实现](https://github.com/flameoftheforest/lunar_rust/blob/f075e601a75170d2dbd8f45b5e6386e97b4f3cb9/src/lib/solar.rs#L107-L214)。

实现还把候选按每十个一组创建操作系统线程。扩大年份范围会扩大候选集合、线程数和全局缓存，不能未经测量就放到桌面主线程，或宣称支持整个历法范围的交互式反查。[线程实现](https://github.com/flameoftheforest/lunar_rust/blob/f075e601a75170d2dbd8f45b5e6386e97b4f3cb9/src/lib/solar.rs#L216-L240)。

此外，历法运算对 1600 年前按儒略闰年处理，但农历年干支计算又调用 `to_naivedate()`，其内部使用 Chrono `NaiveDate::from_ymd_opt(...).unwrap()`。例如儒略历存在而前推格里高利历不存在的 1500-02-29，是必须验证的冲突输入。[闰年规则](https://github.com/flameoftheforest/lunar_rust/blob/f075e601a75170d2dbd8f45b5e6386e97b4f3cb9/src/lib/util/solar_util.rs#L78-L82)、[转 Chrono 日期](https://github.com/flameoftheforest/lunar_rust/blob/f075e601a75170d2dbd8f45b5e6386e97b4f3cb9/src/lib/solar/helper_traits.rs#L322-L333)、[农历内部调用](https://github.com/flameoftheforest/lunar_rust/blob/f075e601a75170d2dbd8f45b5e6386e97b4f3cb9/src/lib/lunar.rs#L298-L308)。

## 本地抽样结果

主任务在项目外的隔离 Cargo 工程中，对确切发布版 Tyme 1.5.1 运行探针；本研究复核了探针源文件及 `results.tsv`。此处只有 Tyme 的行为抽样，未把另外两个候选的源码核查写成运行验证。探针使用 `catch_unwind` 记录失败；这不是建议产品用捕获 panic 代替错误接口。

| 检查 | 实际结果 | 证明边界 |
| --- | --- | --- |
| 公历端点抽样 | `1-01-01`、`1-12-31`、`9999-01-01`、`9999-12-31` 公→农→公相等 | 只证明这四天走通，不证明每个年份、每一天都正确 |
| 改历抽样 | `1500-02-29` 接受并往返；`1582-10-04`、`10-15` 接受；`10-05`、`10-14` 返回无效日期 | 与混合历制一致；不符合当前 `CivilDate` 的前推公历规则 |
| 超界错误 | `SolarDay::new(10000,1,1)` 触发 panic，未返回 `Err` | 不能只看函数返回 `Result` 就假定它对所有输入均无 panic |
| 农历低端 | -1 年首日、末日，0 年首日转公历均 panic；0 年末日可转为 `1-02-11`；1 年正月初一为 `1-02-12` | 农历 -1..=9999 不是完整转换范围，也不能把农历 0 年整体判为无效或整体判为可用 |
| 农历高端 | 9999 年正月初一可转 `9999-02-08`；年末日转公历 panic | 农历 9999 年不能整年开放；`9999-12-31` 对应该年腊月初二 |
| 四柱反查 | 先正算 `2000-07-01` 的 00、02、08、12、22、23 时四柱，再反查 `[2000,2000]`：00 时有 1 条且含原日期/小时；02/08/12/22 时均 0 条；23 时有 1 条但不含原日期/小时 | 已复现非子时漏解和晚子原输入未找回；不是十二时辰或所有流派的穷尽测试 |
| 2023、2033 两整年 | 共 730 天公→农→公断言全部通过 | **库内往返一致性**，不是 730 天独立历表准确性核对 |
| 独立历表抽样 | 主任务在线核对香港天文台三个日期与 Tyme 一致：`2023-01-22` 正月初一，`2023-03-22` 闰二月初一，`2033-12-22` 闰十一月初一 | 仅 3 个独立样例；不外推为整个现代区间已验证 |

探针及原始独立工程曾保存在本地 `target/calendar-library-evaluation/` 和 `/tmp/atlas-calendar-range-probe-20261110/`，均未纳入版本控制，当前仓库没有可复跑的源码、manifest 或结果副本。因此上表只能作为当时的验证记录，不能从当前仓库独立复现。

独立样例来源：[香港天文台 2023 年公农历对照表](https://www.hko.gov.hk/tc/gts/time/calendar/text/files/T2023c.txt)、[2033 年公农历对照表](https://www.hko.gov.hk/tc/gts/time/calendar/text/files/T2033c.txt)。两个整年数据文件因网络代理失败及直连超时未完成本地下载，因此未完成 730 天逐项独立比对。香港天文台[公农历对照表](https://www.hko.gov.hk/tc/gts/time/conversion.htm)覆盖 1901–2100，其说明另提示 2057、2089、2097 年若朔的计算时刻接近午夜，可能出现一日差异；历表的覆盖范围与这些说明均不能直接当作候选库全范围准确性证明。

## 定范围前必须解决的验证问题

1. **公农历边界**：最早及最晚可转换的具体日期、整个农历年是否完整可用、农历年末跨公历上界、无效日期和不存在闰月的行为。单个年份构造成功不足以承诺整年。
2. **独立历表**：先用独立官方历表覆盖现代日期、闰月、月长及跨年；库内测试、跨端口一致和往返一致分别记录，均不代替独立正确性证据。
3. **节气瞬间**：立春及各节令前后各一秒，确认精确时刻与“立春当日”接口没有混用；“返回秒”是表示分辨率，不等于已证明误差小于一秒。
4. **四柱反查**：十二地支、早晚子、两种日界、节令落在时辰内部、六十年重现与搜索端点；每条候选正向复核，并明确返回代表时刻还是完整可选时间段。
5. **项目规则**：闰月十五/十六日、闰十二月进一月的生年口径、晚子跨公农历年、真太阳时修正跨日及跨节气。应用层参数规则须独立验收，不能因库结果稳定就视为已通过。
6. **历制与时间**：古代“公历”是否采用混合历制，以及经度、均时差、时区、历史夏令时和节气瞬间分别以什么时间尺度比较。这些尚未因挑选历法库而自动确定。

仅就本研究的第一方来源核查，尚不能给出三个库任何一个“独立证实准确的完整年份区间”。最终产品的可选范围应由库的有效输入域、各方向转换可用域和实际验证证据共同约束；也可以保留模型计算范围，但必须与准确性承诺分别表述。
