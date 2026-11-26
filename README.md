# 知几 · Zhiji

知几是一款用 Rust 和 Rune 构建的紫微斗数排盘与研习桌面应用。名称取意《周易·系辞下》「知几其神乎！」

- 手动设置排盘参数，或通过公历、农历和干支反推获取排盘参数。
- 查看十二宫、星曜、飞宫与自化，切换大限和流年。
- 为命盘添加标注和分组批注，保存为可继续编辑的 `.ziwei` 草稿，也可导出或复制命盘图片。

[知几官网](https://zhiji.0x219.com/) · [使用帮助](docs/user-guide.md) · [文档目录](docs/README.md)

## 运行桌面应用

三平台首发计划见 [发布里程碑](https://github.com/matharts/zhiji/issues/9)。开发期间可以从源码运行。安装 [mise](https://mise.jdx.dev/) 后，在仓库根目录执行以下命令。

```sh
mise install --include-task-tools
mise run desktop:run
```

各平台的构建依赖和提交规范见[开发约定](CONTRIBUTING.md)。要打开固定参考命盘，使用[视觉测试构建](docs/testing.md#基准与样例)。

## 项目进度

需求、进度和验收记录见[当前项目梳理](https://github.com/matharts/zhiji/issues/14)与 [GitHub Issues](https://github.com/matharts/zhiji/issues)。

项目源码使用 [MIT 许可证](LICENSE)。第三方资源的来源与许可见[资源索引](docs/README.md#专项参考)。
