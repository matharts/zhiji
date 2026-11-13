Status: ready-for-agent
Progress: resolved
Blocked by: none

# 接入 macOS 文件系统集成

## Result

文档类型、图标、启动事件、外部请求排队、重复去重、未保存保护、错误保留与原生窗口前置代码已实现。规格见 `../spec.md`。

`mise exec -- node tools/verify.mjs rust --offline`：452 项通过。编译、严格 Clippy、资源检查通过；Release 构建、macOS 打包及 `codesign --verify --deep --strict` 通过。最终原生脚本已重新编译通过。代码规格轴与规范轴复核无剩余阻断问题。保留工作区改动，未创建提交。

已注销并清理本轮注册的三个隔离测试应用副本，重新注册正式输出路径的知几包；没有修改用户已选定的默认打开应用。合成验收文件保留在 `/tmp/zhiji-finder.7lVZBu/文件集成验收.ziwei`，便于前台检查。

完整原生验收未通过，不能用本任务的实现完成代替 `02-native-acceptance.md`。
