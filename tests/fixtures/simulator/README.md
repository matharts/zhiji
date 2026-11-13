# 模拟器参考样例

来源为 `ziwei-simulator` 的固定提交 `5a5ace5898f43911ce91de031d79f9c4ccffaaef`。本目录是原模拟器的浏览器截图；当前 Rune 应用的视觉基准位于 `tests/fixtures/visual/`。

## 内容与口径

- 本目录 12 张图片在 macOS、Headless Chrome 156、1300 × 800 视口、1× 下等待字体加载后采集，包含初始盘、男女盘、飞化、大限流年、手动命宫、显示开关、颜色面板与导出。
- [计算样例](../simulator-oracle.json)保存原实现的五组输入与输出；[浏览器观察](../simulator-browser-observations.json)记录控件与结果。
- 全界面截图中的 React Scan 浮层来自原应用，不属于本项目界面。
- 重采集时原项目缺少配置引用的 `@rsbuild/plugin-babel` 直接依赖；旧采集使用临时副本补装 1.0.6。重现时记录实际工具和依赖版本，不将该副本视为原锁文件可直接运行的证明。

这些样例用于行为与视觉对照，不证明所有排盘流派规则正确，也不替代本应用的测试或平台验收。

## 计算样例重现

在本仓库根目录运行，提供上述提交的干净检出。生成器会拒绝其他提交或已修改的跟踪文件。

```sh
SIMULATOR_CHECKOUT=/path/to/ziwei-simulator
bun tests/fixtures/generate-simulator-oracle.ts "$SIMULATOR_CHECKOUT" > /tmp/simulator-oracle.json
cmp tests/fixtures/simulator-oracle.json /tmp/simulator-oracle.json
node tests/fixtures/verify-simulator-fixtures.cjs
```

扩大组合覆盖时生成临时样例，再交给核心测试：

```sh
bun tests/fixtures/generate-simulator-oracle.ts "$SIMULATOR_CHECKOUT" --matrix > /tmp/simulator-matrix.json
ZHIJI_SIMULATOR_ORACLE=/tmp/simulator-matrix.json mise exec -- cargo test --locked -p zhiji_core --test simulator_reference reference_cases_match_every_palace_and_star
```

`verify-simulator-fixtures.cjs` 检查历史样例内部一致性，不运行当前桌面应用。

## 浏览器重采集

在参考项目启动开发服务器，例如 `bun run dev -- --port 4593`；然后从本仓库根目录运行：

```sh
mkdir -p output/playwright
playwright-cli -s=simulator open http://localhost:4593
playwright-cli -s=simulator resize 1300 800
playwright-cli -s=simulator --json run-code --filename=tests/fixtures/capture-simulator.js > /tmp/simulator-browser-result.json
node tests/fixtures/verify-simulator-fixtures.cjs /tmp/simulator-browser-result.json
playwright-cli -s=simulator close
```

新截图写入 `output/playwright/`。核对环境并审阅差异后才更新固定样例，不能用当前实现输出替换参考实现的结果。
