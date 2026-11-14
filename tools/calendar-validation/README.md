# tyme4rs 历法验证

独立开发工具，固定 `tyme4rs =1.5.1`，使用自己的 Cargo 工作区和锁文件。应用工作区不依赖本工具；真太阳时与干支反推另按[架构边界](../../docs/architecture.md#日期计算包)自行实现。

从仓库根目录运行：

```sh
# 校验并整理固定参考数据
mise exec -- node tools/calendar-validation/references.ts
# 执行完整历法评估
mise exec -- node tools/calendar-validation/evaluate.ts
# 检查独立工程与参考解析测试
mise exec -- node tools/calendar-validation/verify.ts
```

命令默认离线。首次缺少依赖缓存时先执行：

```sh
mise exec -- cargo fetch --locked --manifest-path tools/calendar-validation/Cargo.toml
```

结果写入 `target/calendar-validation/results/`。评估程序会完成所有比较，并保留日期差异、往返问题及边界 panic；退出零表示评估流程完整执行，准确性结论以[验证报告](../../docs/research/tyme4rs-calendar-validation.md)为准。

## 参考数据

`fixtures/reference-extracts.json.gz` 保存此前从官方 PDF 提取的带行号文本快照，`source-manifest.json` 固定各组原始提取文件的字节数和 SHA-256。它不是原始 PDF 的字节归档，也不是历法库生成的期望结果。

- [香港天文台公农历对照表](https://www.hko.gov.hk/tc/gts/time/conversion.htm)：1901–2100 年。完整性检查要求每份 PDF 提取的所有行号齐全，逐日连续、月长合法、闰月与跨年顺序正确。2040 年使用同一官方机构 `www.weather.gov.hk` 的补提取版本。1901 年最初十九天没有显式月标记，保留未知月份，仍核对年和日数。
- [台湾中央气象署日历资料](https://www.cwa.gov.tw/Data/astronomy/2024cal.pdf)：2018–2027 年节气分钟表。部分缓存省略了无关 PDF 页；验证要求所用节气表及 UTC+8 说明的行号连续、两侧共二十四项完整，保留原始分钟精度。
- 本轮重新打开并核对了 HKO 2057、1916、1979 年表及 CWA 2024 年表。来源 URL 和逐份提取内容哈希另写入结果目录的 `../references/sources.json`。

参考资料的完整性与抽样复核不能排除所有出版或文本提取问题。原表与模型的差异完整保留，不通过改写参考值来消除差异。

## 检查范围

- HKO 逐日农历年、带闰月符号的月份与日期；同一输入的库内公农历往返。
- HKO 的节气日历日期，分别对照 `SolarTerm::get_solar_day()` 和精确时刻投影的日期。
- CWA 公开到分钟的节气时刻，对照精确时刻数值，记录差值，不将分钟资料当作秒级准确性证明。
- 公历数值范围内逐日及月首尾、农历数值范围内可取得月长的逐日及首尾；无法取得元数据、无效输入与两端跨界另外记录。
- 参考解析与比较的负向对照、严格 Clippy、格式和独立工程测试。

宽范围逐日扫描检查库内部的双向一致性；独立准确性比较仅覆盖上面列出的官方参考范围。`catch_unwind` 仅用于隔离探测，不能作为 App 接入的错误处理方案。
