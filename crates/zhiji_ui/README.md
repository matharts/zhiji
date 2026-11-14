# zhiji_ui

桌面应用的内部 Rune 控件。组件接收当前状态，构建 `El`；状态更新由调用方完成。组件不依赖 `AppState`、排盘计算或应用功能模块。

单选项和开关采用一致的构建方式。以开关为例：

```rust,ignore
Switch::new("show-transforms", "显示四化", state.show_transforms)
    .on_toggle_tx(|state: &mut AppState, tx| {
        state.toggle_transforms();
        tx.instant();
    })
    .build()
```

- `key` 应在同一窗口内稳定且唯一，不能依赖易变化的显示文字。
- `SegmentedOption::new` 和 `Switch::new` 接收由调用方持有的当前状态；回调发起操作，下一次绘制仍以调用方状态为准。
- `Segmented` 组织单选项并提供组名称、焦点组和容器样式；调用方负责每个选项的状态和动作，以及外部标题和操作的布局。可设置容器与选项圆角以匹配现有界面。
- `SegmentedOption::selection_width` 可单独限制选中底色宽度，保留外层点击区域与文字中心位置。
- 固定宽度选项可使用 `selection_indicator` 或 `optional_selection_indicator` 绘制选中背景，选项使用 `group_indicator`；调用方提供稳定的指示器 key 和选中索引。选中状态立即更新，背景以 Ant Design 的时长和曲线滑动；减少动态效果时立即到位。
- 共用指示器的未选中选项在悬停时以 200 毫秒过渡背景和文字颜色，选中项保留选中背景；禁用选项不响应悬停。
- 等宽、非默认尺寸的单行选项可用 `sized_selection_indicator` 指定背景尺寸、选项间距和边框。多行且宽度随布局变化的选项使用 `selection_width` 的短暂淡入淡出，保持选中区域和命中区域不变。
- 单选项的 `.label(...)` 设置可访问名称，默认使用可见文字。开关使用提供的文字作为名称；不要以空文字代替必要的名称。
- 默认 `Switch` 的动作绑定在开关控件上，标签不扩大点击区域，保持应用现有交互。
- `Switch::compact` 提供适合密集选项的微型开关，整项可点击；状态和动画仍由调用方控制。
- `FONT_FAMILY` 是应用注册的字体族名称，字体文件由应用加载。`font` 随界面倍率缩放，`fixed_font` 使用调用方指定的固定字号和行高，供命盘和标注使用。组件现有尺寸和颜色服务于当前界面；改变它们须按[测试约定](../../docs/testing.md)审阅视觉和交互影响。

控件的构建器、受控状态和无障碍测试参考项目固定版本的 Rune / rune-kit；不直接依赖 rune-kit，也不套用其默认视觉主题。业务专属控件和状态继续放在应用中。

```sh
mise exec -- cargo test --locked -p zhiji_ui
```

颜色控件 `ColorPlane`、`HueSlider` 和 `OpacitySlider` 是受控构建器；`color_preview` 绘制透明预览。拖动回调提供归一化坐标、手势阶段和取消状态，坐标保留范围外的值，由调用方决定拖动边界和事务策略。滑条通过 `on_step_tx` 绑定键盘及无障碍步进。`ColorPickerPixels` 统一控件尺寸、像素坐标与色相面板缓存；调用方提供 sRGB 颜色计算并持有颜色数据、编辑事务和文本输入。

`IconButton` 统一图标按钮的名称、焦点和禁用动作；`menu_panel` 与 `menu_item` 提供菜单外观，菜单项的动作由调用方绑定。`ConfirmationDialog` 和 `dialog_button` 提供确认面板和按钮；`ConfirmationDialog::modal` 要求调用方提供取消操作和焦点返回目标，并将确认按钮设为默认按钮。

`ColorToken` 是界面与绘制颜色的唯一语义来源，浅色和夜间值成对定义在 `tokens.rs`。`color(token)` 在当前线程的视图构建作用域内解析为 Rune 颜色；组件接口只接受 token，调用方不得在使用处直接写十六进制颜色。`theme_scope` 切换当前渲染作用域并在离开时恢复之前的模式，延迟执行的视图构建回调需要捕获模式并自行进入作用域。用户选择的颜色、草稿配色和标注自定义颜色属于运行时数据，通过 `Rgb`/`Color` 转换，不进入 token 枚举。

命盘、标注内置色和印章读取各自的语义 token。命盘绘制按快照中的夜间状态显式调用 `ColorToken::resolve(night)`，不依赖界面线程的 `theme_scope`；这样屏幕、导出和复制不会因构建线程不同而改变颜色。

`popover_transition` 为调用方提供朝向触发器的短暂缩放与位移，进入时保持可交互；弹层位置与触发方式仍由调用方负责。

图标按钮、未选中的单选项和可用菜单项的悬停反馈使用短暂的颜色过渡；图标按钮按住时轻微缩小，系统开启减少动态效果时不缩放。控件状态和键盘操作仍即时响应，调用方的即时事务不会被组件覆盖。

日期与地区选择器的展示部件同样由调用方控制，不依赖日期计算、地区数据或应用状态：

- `ChoiceOption` 复用 `SegmentedOption`，可通过 `size`、`fill_width`、`subtitle` 与 `content` 组织网格选项；新网格使用 `borderless`，列表使用 `list_item`。`Segmented::columns` 提供等宽多行布局，容器尺寸由 `size` 指定。默认选中配色为浅紫底与深紫字。
- `LegendOption` 接收调用方绘制的符号和选中状态；`selection_fill`、`hover_fill` 与 `ink` 支持白底编码图例等外观，组件自身不解释符号。可用选项提供悬停过渡和轻微按压缩放；减少动态效果时取消缩放，禁用时无指针反馈。选中颜色的过渡由调用方的动画事务控制。
- `CalendarGrid` 只排列调用方提供的单元，支持自定列数与间距；空白日期由普通 `El` 表示。日期选项使用 `ChoiceOption::calendar_cell`，方向键移动焦点，Return/Space 选择日期。`SelectList` 提供滚动和焦点组；上下键、Home/End、键入查找移动焦点，Return/Space 触发选项动作。`CascadeColumns` 排列三级列，各列的标题、选项和关联由调用方提供。
- `DropdownTrigger` 由 `expanded` 控制，点击、Space 和上下键请求打开或关闭；`trailing_space` 可为外部叠放的独立控件预留文字与箭头之间的空间。列表的位置与 Escape 关闭路径由调用方组合。
- `SearchField` 接收 `TextEdit`，使用 `on_edit` 绑定调用方编辑器，并通过 `on_clear_tx` 发出清除请求；它不持有搜索词或结果。搜索图标、输入与清除按钮分别使用 16、36、28 点尺寸，禁用时禁止编辑和动作。
- `Button` 默认 72×32 点，`compact` 为 56×28 点及 13 点文字；`primary` 提供主操作外观。`ModalDialog` 默认 592×750 点，`viewport` 将面板夹紧到视口内 24 点边距，正文滚动、标题和底部操作保持稳定。`modal` 必须提供关闭回调与焦点返回目标，可通过 `close_button` 放入调用方绑定的关闭按钮。`ModalDialog::custom` 接收调用方组合的面板内容，仍共用尺寸限制、遮罩、淡入淡出、模态焦点与焦点返回。业务状态及 Escape 优先级由调用方处理。

可用的开关和复选框提供悬停及按压颜色反馈；禁用控件不显示指针反馈。

开关滑块和复选标记在动画事务中跟随受控状态过渡；确认面板和菜单也提供淡入、淡出过渡。调用方可通过 `tx.instant()` 保持某次更新即时，系统减少动态效果时滑块不会移动。

`SearchField::on_input_key` 将键盘处理绑定到内部编辑器，供调用方实现搜索建议导航；返回 `false` 保留输入法和文本编辑的默认行为。
