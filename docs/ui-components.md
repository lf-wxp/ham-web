# 通用 UI 组件使用规范

> 写给**要改界面的人与 AI agent**。这个仓库的界面只有一套控件来源 ——
> `crates/app/src/ui/`。本文件是**写新代码时的规则与清单**；
> 组件清单、每个 prop 的含义、逐个使用示例 → [`crates/app/src/ui/README.md`](../crates/app/src/ui/README.md)。
>
> 两份文档的分工：**本文件管「怎么写」，ui/README.md 管「有什么」。**
>
> 控件迁移已经收口（原生表单元素清零、`input_class` 工厂删除）。收口过程中**刻意留下的
> 例外**都写进了代码注释，用 `grep -rn "刻意的例外" crates/app/src` 一次列全；
> 清单与原因见本文件 §5 第 4 条与 §8。

## 0. 三条底线

1. **页面里不写原生表单元素**，也不写类名工厂（`input_class` 已删除；`button_class` 只允许
   `Button` / `ButtonLink` 内部与 §8 清单里的几处例外使用），更不新增本地 `const INPUT` / `CHIP_ON`。
2. **标签关联、无障碍名、多语言文案**是组件与调用点**共同**的责任，缺一样都算没写完
   （e2e 也靠它们定位，见 §4）。
3. 改完**必须**过 `cargo make check`；动到页面组件再跑相关 e2e（见 §7）。

## 1. 决策表：要什么 → 用哪个

| 场景 | 用 | 不要用 |
|---|---|---|
| 单行文本 / 搜索 / 密码 / 邮箱 / tel / url | `Input`（`kind=InputType::Search` 等） | `<input>` |
| 数字（有上下限、步进） | `NumberField`（`controls=false` 关掉加减按钮） | `<input type="number">` |
| 多行文本 | `Textarea` | `<textarea>` |
| 少量固定选项 | `NativeSelect` + `SelectOption` | `<select>` |
| 选项要富文本 / 长列表 | `Select` + `SelectItem` | — |
| 日期 / 时间 | `DatePicker` / `TimePicker` | `<input type="date">` |
| 是 / 否（**改完立即生效**的设置项） | `Switch` | `<input type="checkbox">` |
| 是 / 否（**随表单一起提交**的字段） | `Checkbox` | `<input type="checkbox">` |
| 数值范围（滑杆） | `Slider` | `<input type="range">` |
| 互斥选择（表单里、选项带文字、和标签配套） | `RadioGroup` + `RadioGroupItem` | 手写 chip |
| 互斥选择（工具条上、选项很短、横排一行） | `ChipGroup` + `Chip` | 手写 chip |
| 筛选开关（可以一个都不选） | `ChipToggle` | — |
| 选择文件 | `FileInput` | `<input type="file" class="hidden">` + 标签按钮 |
| **执行动作**（提交、打开弹层、切换状态） | `Button` | `<button class=…>` |
| **跳转**（能中键新开、右键复制地址） | `ButtonLink` | `<a class=button_class(…)>` |
| 标签 + 控件 + 提示 / 错误 | `Field`（`hint` / `error` / `required`） | 手写 `<label>` + `<span>` |
| 进度、分隔线、统计块、标签文字 | `Progress` / `Separator` / `Stat` / `Label` | 手写 |

`RadioGroup` 与 `ChipGroup` 的区别是**密度不是语义**（都输出 `role="radiogroup"` /
`role="radio"` + `aria-checked`）：选项带文字、和 `Field` 标签配套用前者；选项本身很短
（`20m`、`三单元八木`）、横排一行用后者。
`Button` 与 `ButtonLink` 的区别是**元素语义**：动作与跳转不要混用（理由见 ui/README.md）。

## 2. 组件 API 约定

- **受控值**：`value` / `checked` / `active` 都是 `Signal<T>`（标了 `#[prop(into)]`），
  配套 `on_change: Callback<T>`（回调收的是**新值**，不是事件）。
  传值三种写法：`RwSignal` 直接传、`Signal::derive(move || …)`、需要复用时 `Memo`。
  属性里**不要**写裸闭包 —— 除非该 prop 声明的就是闭包（`label=` 等 `TextValue`
  收的是 `Signal<String>`，`trigger=` 收的是 `ViewFn`）。
- **文案**：`TextValue` 收 `&str` / `String` / `Signal<String>`；要跟随语言切换就必须
  `Signal::derive(move || t("key"))`（`TextValue` **不接受闭包**）。
- **尺寸与宽度**：高度交给 `size`（`ControlSize::{Sm,Default,Lg}`），宽度/字体等修饰写在
  `class`。`class` 是 `String`，要**静态字面量**（Tailwind 只扫描完整类名）；
  宽度随状态变时，在闭包里先算好 `String` 再赋给 `class`（见 `pages/morse/morse_trainer.rs`）。
- **表单行的 id**：`Field` 负责 `label` 与 `r#for`，控件要有同一个 `id`，值用
  `util::unique_id("…")` 生成（§6 有陷阱）。
- **提交**：`Button` 默认输出 `type="button"`，**要提交表单必须写 `kind=ButtonKind::Submit`**
  （这是刻意的：原先手写 `<button>` 不带 `type`，在 `<form>` 里会意外提交）。
  异步提交用 `loading=Signal`：自动禁用 + 置 `aria-busy`，文案保留。
- **错误态**：`Input` / `NumberField` / `Select` / `Textarea` / `Switch` 都有
  `invalid=Signal<bool>`，会加 `aria-invalid` 并把描边转成 `destructive`；不要再手写红色描边。

## 3. 无障碍（e2e 也依赖它）

- **有可见标签**：用 `Field`（`r#for` + 控件 `id` 配对），或把控件包在 `<label>` 里。
- **没有可见标签**：必须给 `aria_label=Signal::derive(move || t("…"))`（图标按钮 / 图标链接
  **必填**，例如 `Button variant=Variant::Ghost size=Size::Icon aria_label=…`）。
- **互斥 / 多选**：用组件的 `RadioGroup` / `ChipGroup` / `Checkbox` / `Switch`，
  不要自己写 `role=`、`aria-checked`、`aria-pressed`。
- **动态提示**：实时结果区用 `aria-live`，并用 `Input` 的 `aria_describedby` 关联 id。
- Playwright 用例大量使用 `getByLabel(...)` / `getByRole(...)`：**标签关联与 role 是测试契约**。
  改了 `role` / `aria-*` 必须同步 spec（§7）。

## 4. 多语言

- 界面文案一律 `t("域.词条")`，**字面量写在调用点**（`check-i18n` 只采集直接实参；
  经变量/函数转手会被判成死条目或漏统计）。
- 新增词条：`data/i18n/{zh,en,es}/<域>.json` **三份都加**（key 前缀与文件名域一致，
  三种语言的 key 集合必须相同）→ `cargo make i18n-pack` 重新生成运行时语言包 →
  `cargo make i18n-check`。
- **中文值不能重复**（两个 key 用同一句中文会被判「重复」）；确实要复用时改成引用同一个 key。
- 三个语言的词条数不同属于正常（翻译进度不同），但**key 集合必须一致**。
- 代码里**不要硬编码**界面中文（如 `title="切换到深色模式"`）——`check-i18n` 抓不到它，
  但 en / es 界面上会直接露出中文。发现这类残留请顺手补词条。

## 5. 常见坑

1. **`Field` 的 id 与闭包**：把 `unique_id()` 生成的 id 声明在组件体里、再在
   `{move || …}` 里用（哪怕 `.clone()`），闭包会退化成 `FnOnce` → 编译失败。
   id 必须与使用点**同作用域**：顶层 `view!` 用 → 声明在组件体；在 `move ||` 里用 →
   声明在那个闭包里；也可以用 `view!` 的 `{{ let id = …; view!{ … } }}` 块。
2. **Tailwind 只认完整字面量类名**，不要 `format!("{} {}", "px", n)` 拼片段。
3. **依赖 context 的子组件**（`RadioGroupItem` / `SelectItem` / `Chip`）必须在父组件的
   `children` 里就地构建，不能提前 `collect_view()` 到外面再传进去。
4. **动态 variant 与 `aria-expanded` 不支持**：`Button` / `ButtonLink` 的 `variant` 是静态
   prop，两个组件也都没有 `aria-expanded` 通道。因此「选中态 / 展开态要换 variant」或
   「下拉开关要报展开状态」的地方**保留原生元素 + `button_class`** —— 只有 §8 清单里的
   那几处，每处都写了 `刻意的例外` 注释（`grep -rn "刻意的例外" crates/app/src` 可一次列全）。
   **不要**为了「统一」把这些清掉（会丢状态反馈 / 无障碍状态）；真要收编，先给
   `Button` / `ButtonLink` 补通道（见 §6），再改调用点。
5. **`Input.on_change` 是「每次输入」**：需要「失焦 / 回车才提交」语义时用 `on_enter`，
   或把它放进 `<form on:submit>` 用提交按钮。
6. **`Slider` 收 `f64`**：`f32` 信号要显式转换（`Signal::derive(move || f64::from(x.get()))`
   + 回调里 `as f32`）。
7. **`NumberField` / `Input` 的 `value` 是 `String`**（保留用户输入的中间态如 `-`、`1.`），
   不要在组件外把它 parse 成数字再喂回去。
8. **筛选器的「一个都不选」**：不要用 `ChipGroup`（它是单选，必有选中项），用 `ChipToggle`。

## 6. 新增 / 扩展组件

- **先扩展现有组件**：缺通道（例如 `title`、`maxlength`、`aria_expanded`）时，优先给现有组件
  补 prop，而不是在页面里退回原生元素。补完在 `ui/README.md` 组件表里补一行。
- **什么时候才新增组件**：同一个交互在 ≥3 处重复，且现有组件 + prop 表达不了。
- **放哪**：**一个组件一个文件**（这条对业务组件同样适用，见 `AGENTS.md`）——
  `crates/app/src/ui/<name>.rs`，在 `ui/mod.rs` 里 `mod` + `pub use`，并在 `ui/README.md`
  组件表登记（组件名 / 替代的原生元素 / 主要 props）。
- **一族多组件用文件夹模块**（`Dialog` + 它的 5 个部件、`RadioGroup` + `RadioGroupItem`、
  `Select` + `SelectItem`、`ChipGroup` + `Chip` + `ChipToggle` 都是这么放的）：
  `xxx/mod.rs` **只放**模块文档 + `mod` 声明 + `pub use` 重导出，每个组件一个文件，
  共享的样式 token / context 放 `xxx/shared.rs`。文件名不能与父目录同名
  （`chip/chip.rs` 会被 Rust 拒绝），照 `dialog_root.rs` / `select_root.rs` / `chip_item.rs` 起名。
- **约定**：`#[prop(optional, into)]` 标可选；`data-slot="<组件名>"`；
  复用 `control_class` / `button_class` / `badge_class` 等底座，**不要另起一套样式**；
  文案类 prop 用 `TextValue`；回调用 `Callback<T>`；颜色只用 Token
  （`bg-card` / `text-muted-foreground` / `ring-ring` …），不写死 hex。

## 7. 验收清单

```bash
# 1) 静态检查（格式 + clippy + 单元测试 + 文案词典 + 知识库译文 + 解析表）
cargo make check
# 2) 提交前（上面 + release 前端构建）
cargo make ci
# 3) 改了文档注释里的 intra-doc link 时（当前基线：0 warning）
cargo doc -p ham-web-app --no-deps
```

动到**页面组件**后再跑相关 e2e（先起 `trunk serve`，或先 `cargo make build-web`）：

```bash
cd e2e && E2E_BASE_URL=http://127.0.0.1:3030 npx playwright test tests/<spec>.spec.ts
```

两条经验（都踩过）：

- **改了 `role` / `aria-*`，要按「断言里出现过这些属性」全仓搜 spec，不要按页面名找** ——
  smith 圆图的用例住在 `waveform_lab.spec.ts` 里，按页面名找就会漏。
- **`trunk serve` 的产物可能缺 `public/` 资产**（语言包、题库、worker），表现为页面空白 /
  整页挂载失败 / 英文界面回退中文 / `page.goto: Page crashed`。自检：
  `curl -s http://127.0.0.1:3030/data/i18n/en.json | head -c 1`（正常是 `{`）。
- **dev 构建没有 `wasm-opt`**，CPU 密集用例（如 Yagi 优化）并行时容易超时，
  `--workers=1` 复跑通过即非回归。整批回归请用 `cargo make e2e`（release 口径，与 CI 一致）。

## 8. 现状快照（自检命令）

```bash
cd crates/app/src
# ① 本地控件样式常量：应为 0
grep -rn --include='*.rs' --exclude-dir=ui -E '^const (INPUT|TEXTAREA): ' .
# ② 原生表单元素：应只剩 6 处隐藏 <input type="file">（不参与展示，按约定不迁移）
grep -rn --include='*.rs' --exclude-dir=ui -E '<input|<select|<textarea' .
# ③ 类名工厂：input_class 应为 0（工厂已删）
grep -rn --include='*.rs' --exclude-dir=ui -c 'input_class(' . | grep -v ':0$'
# ④ 手写 chip：应为 0
grep -rln --include='*.rs' --exclude-dir=ui 'CHIP_ON' .
```

截至最后一次收口：①0 ②6 ③0 ④0。`button_class` 在页面侧只剩 8 条刻意的例外注释，
覆盖 10 个原生元素（8 个 `<button>` + 2 个 `<a>`；`group_menu.rs` 那条是两个菜单开关共用，
`grep -rn "刻意的例外" crates/app/src` 可列全）：

| 位置 | 为什么保留原生元素 |
| --- | --- |
| `components/navigation/nav_bar.rs` | 导航项与下拉 / 菜单开关（2 个 `<button>` + 1 个 `<a>`）：高亮要切 `variant`，下拉开关还要 `aria-expanded` |
| `components/navigation/group_menu.rs` | 「知识库」「工具」两个下拉开关（共用一条注释，渲染出 2 个 `<button>`）：理由同 nav_bar；面板是分组侧栏 + 条目区，见文件头 |
| `pages/callsign_copy.rs` | 两个模式按钮：选中态切 `variant` |
| `components/exam/answer_card_sheet.rs` | 答题卡筛选按钮：当前筛选切 `variant` |
| `pages/home/cards_card.rs` | 「今日待复习」入口：有待复习时切 `variant` |
| `components/bank_selector.rs` | 刷新按钮：底色随刷新状态（成功 / 失败 / 加载）变化，`class` 也要按状态合并 |

它们连同 `Button` / `ButtonLink` 内部都继续用 `button_class` —— **`ui::button_class` 是这套
组件的底座，不要删**。另有 6 处隐藏的 `<input type="file">`（上传按钮背后、不参与展示），
按约定不迁移。
