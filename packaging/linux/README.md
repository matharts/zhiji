# Linux 文件集成

AppImage 是支持应用内自动更新的发行格式，网站下载优先选择它。更新器仅替换 `APPIMAGE` 指向的可写镜像，不改写 DEB 的系统安装目录。AppImage 显式携带动态加载的窗口和键盘库及 `xdg-open`；图形驱动、Vulkan 加载器和桌面门户服务仍由宿主系统提供。与 DEB 一样，Ubuntu 24.04 构建的 glibc 兼容下限需要单独验收。

DEB 将 `zhiji.xml` 安装到 `/usr/share/mime/packages/`，桌面入口注册 `application/vnd.matharts.zhiji+json` 并用 `%F` 传入路径。图标名与安装的 `zhiji` 图标一致；不调用 `xdg-mime default` 或修改用户的 `mimeapps.list`。

包依赖 `shared-mime-info`、`desktop-file-utils` 和用于启动失败提示的 `zenity`。安装与卸载验收须检查 MIME、桌面数据库的更新结果及原默认应用保持；不能仅凭 XML 与 desktop 文件存在认定完成。具体结果见 `.scratch/file-integration/issues/03-windows-linux.md`。
