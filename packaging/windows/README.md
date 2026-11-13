# Windows 安装模板

`installer.nsi` 源自 cargo-packager 0.11.8 的默认 NSIS 模板，按同目录 `LICENSE-cargo-packager-MIT.txt` 使用。与上游的唯一业务差异是增删 `.ziwei` 的 `OpenWithProgids` 和知几专属 ProgID：不写扩展名默认值，卸载时仅在处理程序仍指向本次安装位置时清理。安装器的文件图标沿用可执行文件内嵌图标。

升级 cargo-packager 时，先对照对应版本的上游模板，审阅并合并安装、升级和卸载变化，再验证安装、用户已有默认应用、覆盖安装及卸载；不能只替换模板或只验证模板编译。

打开动词设为 `MultiSelectModel=Single`，与应用的单工作区一致；资源管理器多选时不提供该动作，避免默认 Document 模型启动多个单文件进程并依次替换文档。实现依据 Microsoft 的 “How to Employ the Verb Selection Model”；测试时需同时检查“打开方式”与默认应用路径，不能用手工构造多参数替代 Shell 验收。
