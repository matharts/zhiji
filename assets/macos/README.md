# macOS 文档声明

`Info.plist` 由 cargo-packager 合并到应用包，声明 `.ziwei` 为“知几命盘草稿”，类型标识为 `com.matharts.zhiji.draft`，符合 `public.json`。应用角色为 Editor，文档图标复用包内的 `app.icns`；图标来源见[图标说明](../icons/README.md)。

声明只让系统发现处理能力，不调用 API 修改用户已选定的默认打开应用。修改本文件夹中的 plist 时同步更新 `assets/manifest.json` 的校验值，并验证生成包的声明及实际 LaunchServices 打开行为。

自动更新与另两端共用 Rust 实现，不在此 plist 保存另一套更新偏好。签名公钥和更新资产协议见[更新包说明](../../packaging/updates/README.md)。
