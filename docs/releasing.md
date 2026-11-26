# 构建与发布

本页用于构建和验收安装包、准备签名更新，以及部署官网。三端首发的安排和进度见[发布里程碑](https://github.com/matharts/zhiji/issues/9)。

## 本地打包

工具版本来自 [mise.toml](../mise.toml)，包名、图标、资源与平台依赖来自 [desktop Cargo.toml](../Cargo.toml)。先构建，再在对应系统选择格式：

```sh
mise exec -- cargo build --locked --release -p zhiji
mise exec -- cargo packager --release --formats app   # macOS
mise exec -- cargo packager --release --formats nsis  # Windows
mise exec -- cargo packager --release --formats appimage,deb # Linux
```

macOS 输出为 `target/release/知几.app`，可验证签名：

```sh
codesign --verify --deep --strict 'target/release/知几.app'
```

当前 macOS 配置使用临时签名；正式外部分发前须配置适当的签名和公证。Linux CI 基于 Ubuntu 24.04 打包，声明 glibc 最低版本 2.39；发布目标必须与这些依赖相符。

macOS 文档类型声明来自 [Info.plist](../assets/macos/Info.plist)，打包时合并到最终配置，资源约定见[说明](../assets/macos/README.md)。Windows 使用[自定义 NSIS 模板](../packaging/windows/README.md)注册 `OpenWithProgids`、专属 ProgID、文档图标及单选打开命令；卸载先核对打开命令是否仍属于当前安装位置。Linux DEB 使用 [desktop 模板](../packaging/linux/zhiji.desktop)及 [MIME 声明](../packaging/linux/zhiji.xml)，通过 `%F` 传入文件。Windows/Linux 使用进程锁和本机通道把后续打开交给已有实例；Linux 的启动失败系统提示由 DEB 依赖 `zenity` 提供。安装包不主动覆盖用户默认应用。

分别验收三平台的安装、覆盖安装、卸载、文件管理器发现与双击。Linux 还需在安装和卸载后检查 `shared-mime-info`、`desktop-file-utils` 的数据库触发器是否更新系统缓存。已有实现和当时的验收缺口见[跨平台文件集成记录](../.scratch/file-integration/issues/03-windows-linux.md)。

## 发布检查

1. 确定版本、对应提交、变更说明和目标平台。正式产物从该提交的干净检出构建，使用已提交的锁文件与工具版本；保留提交 SHA、CI 运行和产物校验值的对应关系。未提交工作树的本地检查不作为正式发布基线。用户可见变化、兼容影响与迁移步骤写入该次发布说明；发布历史形成后再维护 `CHANGELOG.md`。
2. 执行[测试约定](testing.md)要求的检查，保留命令、环境和结果；各平台安装、运行和原生验收分别记录。
3. 核对打包资源、图标和第三方许可。字体、图标和地区数据说明随包分发，来源见[字体说明](../assets/fonts/README.md)、[图标说明](../assets/icons/README.md)和[地区数据说明](../assets/data/README.md)。
   Rune 配套补丁的来源与许可证见 [vendor 说明](../vendor/README.md)，许可证随应用打包。
4. 核对项目 [MIT 许可证](../LICENSE)及第三方资源许可已随发布产物提供。
5. 检查安装、启动、手动排盘、草稿打开与保存、PNG 导出及复制，并记录卸载或替换旧版本的方法。签名检查通过不替代实际安装验证。
6. 审阅产物与变更说明后发布；上传 CI 构建产物与对外发布属于不同阶段。

## 命名与兼容

产品中文名为“知几”，英文名为“Zhiji”；Cargo 包、可执行文件与 Debian 包名为 `zhiji`，系统标识为 `com.matharts.zhiji`，Rune 应用 ID 为 `zhiji`。配置目录使用 `zhiji`，环境变量使用 `ZHIJI_*`。内部模块统一为 `zhiji_core`、`zhiji_ui` 和 `zhiji_calendar`，字体资源使用 `Zhiji` 前缀。草稿扩展名仍为 `.ziwei`。

切换应用标识后，首次使用前应将原配置目录中的设置复制到新的配置目录，不覆盖已有新配置。系统辅助功能授权可能需要重新授予。

应用内使用帮助可离线打开，问题反馈和公开发布资产通过 `matharts/zhiji` 提供。官网下载入口跟随[公开发布状态](../website/README.md#下载与发布状态)显示。

## 签名更新

三端共用 Rust 更新实现及 cargo-packager 的资产协议。macOS 使用应用包，Windows 使用 NSIS，Linux 使用 AppImage。Linux DEB 通过包管理渠道更新。检查、下载和重启操作见[使用帮助](user-guide.md#设置与退出)。

更新元数据和更新包均要求 Minisign/Ed25519 签名，公钥位于 `packaging/updates/public-key.txt` 并编入客户端。本机私钥在仓库外的 `~/.config/zhiji-updater/updater.key`；macOS/Linux 文件权限为 0600，Windows 须核对用户目录的 ACL。CI 使用 `CARGO_PACKAGER_SIGN_PRIVATE_KEY` 及可选密码 secret；没有密钥或密钥与应用公钥不匹配时，版本 tag 构建失败，不产出伪签名更新。

```sh
# 仅首次配置；已有私钥时绝不覆盖。公钥变更须单独设计客户端信任过渡。
mise exec -- node tools/release/updates.ts keygen
# 在对应系统和架构完成构建打包后，生成签名更新资产。
mise exec -- node tools/release/updates.ts prepare
# 使用客户端校验实际签名产物；必须选中并通过一项测试。
mise exec -- node tools/verify.ts updates
```

发布 tag 使用 `v<三段正式版本>`，与 Cargo 版本及应用显示版本一致。各目标生成 `latest-<系统>-<架构>.json`、对应更新包、签名和 SHA-256。将全部目标资产上传到同一次正式 Release；源固定为本项目 `releases/latest/download/latest-<系统>-<架构>.json`。签名元数据绑定目标、正式版本、下载 URL、字节数和包签名，签名后不得编辑。生成工具独立验证签名与应用公钥一致。CI 只上传构建产物，不自动公开发布；配置 secret 和正式发布须单独完成。尚无公开源时显示“暂无公开更新”，不误报为最新版。

首次正式发布后须分别在三端用公开源完成真实升级，检查原路径替换、版本、草稿与偏好保持、取消及错误恢复。隔离源和篡改拒绝测试不能替代公开链路验收，交叉编译不等于 Windows/Linux 安装及重启验收。更新包签名不替代 macOS Developer ID/公证或 Windows 系统分发信任。Unix 安装要求目标目录可写，并保留回滚备份；非常规受保护目录的更新失败须明确报告，不能删除用户数据后重试。

## 官网部署

[Website 工作流](../.github/workflows/website.yml) 在 `main` 推送或从 `main` 手动运行时，使用通过检查的 Astro 静态产物部署腾讯云 COS。部署先上传资源，再更新 HTML，保留桶中的已有对象。发布后通过官网的 HTTPS 地址访问首页、下载页和参考盘页面，核对返回的 HTML 是否与本次构建一致。

1. 在 COS 存储桶启用静态网站，索引文档设为 `index.html`，并允许访客读取网站对象。将官网域名绑定到静态网站源站，按腾讯云控制台给出的 CNAME 配置 DNS，并为该域名绑定 HTTPS 证书。[静态网站配置](https://cloud.tencent.com/document/product/436/32670) · [自定义源站域名](https://cloud.tencent.com/document/product/436/36638)
2. 为 CI 创建专用 CAM 凭据，授权写入工作流指定的存储桶。当前官网产物只需对象上传权限 `name/cos:PutObject`。将 SecretId 和 SecretKey 分别存入 GitHub 的 `website-production` 环境 Secrets，名称为 `TENCENT_SECRET_ID` 和 `TENCENT_SECRET_KEY`。该环境仅允许 `main` 部署。[CAM 用户策略](https://cloud.tencent.com/document/product/436/68280)
3. 到 GitHub Actions 的 [Website 工作流](https://github.com/matharts/zhiji/actions/workflows/website.yml) 选择 `Run workflow`，使用 `main` 执行首次部署并查看结果。域名或证书修复后也可从这里重新运行。

存储桶策略允许访客读取已有官网页面与资源目录。参考盘的 `experience/index.html` 在上传时单独设置公开读取权限。

## 草稿兼容与诊断

当前草稿格式不支持旧版本，见[领域规则](domain.md)。不自动删除用户配置目录或已有草稿。

正常桌面启动会在配置目录的 `diagnostics/` 下异步保存默认诊断。`zhiji.log` 与 `zhiji.previous.log` 各最多 512 KiB，只保留当前及上一份；单条摘要最多 4096 字节，时间戳使用 Unix 秒。默认记录启动环境、导出与草稿读写的失败阶段和错误类别，以及设置保存失败。不保存命盘、用户路径、渲染文本或异常载荷。目录在首次写入或明确打开时创建；测试和无头工具默认不启用自动写入。

正常桌面启用诊断时另安装一次 panic hook，沿用既有文件锁与轮换，在异常发生时同步尽力记录固定标记和主线程／其他线程分类，再调用原有 hook；默认日志不包含异常载荷、动态线程名或源码路径。Rust 的 panic hook 也会观察到随后被 `catch_unwind` 捕获的异常，因此 `rust panic observed` 不表示进程一定退出。锁竞争或文件写入失败时可跳过该条记录；强制终止、断电、系统级故障也不保证留下记录。原有 hook 对标准错误的输出行为保持不变，可能包含异常详情。

“设置 → 关于”的“诊断日志”行可打开实际目录或复制路径。Windows 通过系统 Shell 打开目录，macOS 使用 `open`，Linux 使用 DEB 依赖 `xdg-utils` 提供的 `xdg-open`；操作在后台执行，创建或启动失败保留可见提示和复制路径的恢复方式。系统接收打开请求不等于目录窗口已经显示。

图片任务的详细诊断仍异步写入标准错误。可额外将 `ZHIJI_DIAGNOSTICS` 指向可写文件，保留详细导出日志；这一显式指定的文件不参与默认轮换，可能含底层错误中的用户路径。日志均为尽力记录，竞争中的写入可能跳过，强制退出不保证最后一条落盘。收集诊断时检查是否包含私人信息。

默认日志及其备份路径不接受显式详细日志追加；当 `ZHIJI_DIAGNOSTICS` 指向这两个文件或可解析的符号链接时，仅保留默认摘要，详细内容仍输出到标准错误。
