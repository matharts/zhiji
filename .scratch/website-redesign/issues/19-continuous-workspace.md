# 连续工作区与产品内容整合

Status: ready-for-agent
Progress: resolved
Blocked by: none
Type: task

## Scope

用户在整站设计 skill 评审后要求优化。合并重复功能展示，用完整工作区贯穿同一命盘；功能页补强机制与参考界面。保留黑白紫、星空、3D、当前首屏和导航。

## Result

- 新增 WorkspaceStory、FlightControls、DesktopTools。现有 FlightDemo 用插槽复用完整命盘和既有四化逻辑，不复制一份排盘计算。
- 确认参数、观察飞化、标注、留存四步共用同一参考盘。切换步骤时停止四化播放并保留选择，圈注和连线从四化的实际落宫出发。
- 草稿与 PNG 明确区分；倍率只是输出设置示意，不写入文件。示例参数来自独立 fixture，纳入参考一致性断言。
- 首页移出重复分类卡片、键盘墙、六能力选择器与独立标注/输出大区；保留自化、取数、桌面习惯、资源、FAQ和下载，原组件未删除。
- 功能页保留八个主题和深链接，增加机制图与参考宫位。取数图分别表达公历、农历和干支反推口径。
- FAQ 增加计算范围与 MIT 许可证入口，不编造价格、客户或性能数据。
- 基线：`.scratch/website-redesign/workflow-baseline/`；首屏 hero 文本与三个 3D 源文件逐字断言不变。

## Validation

- `mise exec -- node tools/verify.ts website`：14 tests pass，55 个 Astro 文件无错误/警告，7 页构建成功；最终锚点调整后 check/build 再次通过。
- Impeccable detector：新增/修改的主要组件、页面与样式返回 `[]`。`git diff --check` 通过。
- 浏览器：首页 320/390/700/701/1280px，功能页 320/390/701px 无横向溢出；1440×1000 截图覆盖标注、输出和功能页机制图。
- 实测卯宫化科保留到标注步、标注开关、导出倍率、切步骤停止四化播放、重复同一锚点、Enter 键切步骤。在线体验定位窗口上沿 108px。控制台无 warning/error。
- `.impeccable/review/workflow/` 保存 desktop/output/mobile/mobile-detail/features-mobile/features-desktop/user-1280 截图。
- 独立评审指出圈注落在起宫的问题。修复后，桌面实测寅宫化禄落亥宫、卯宫化禄落丑宫，圈注在标注与纸面输出两个步骤均正确。390px 直接访问 `#annotation` 也能显示默认亥宫圈注。对应截图为 `annotation-yin-to-hai.jpg`、`output-yin-to-hai.jpg`、`annotation-mao-to-chou.jpg`、`output-mao-to-chou.jpg` 和 `mobile-destination-fixed.jpg`。
- 落宫修复后再次运行 `mise exec -- node tools/verify.ts website`，14 项测试、55 个 Astro 文件检查与 7 页构建通过。未测量 GPU 性能或实际读屏播报。
