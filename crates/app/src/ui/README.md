# 基础 UI 组件（`crates/app/src/ui`）

> **写代码前先读 [`docs/ui/pixel.md`](../../../../docs/ui/pixel.md)（像素风）与 [`docs/ui/modern.md`](../../../../docs/ui/modern.md)（现代风）**：那里是「用哪个组件、
> 怎么写、有哪些坑、怎么验收」的规范；本文件是组件清单、props 与逐个使用示例。

本目录是全站**表单控件的唯一来源**。页面里不再直接写 `<input>` / `<select>` / `<textarea>` /
`<input type="range">`，统一用这里的组件 —— 颜色、边框、间距、字体与 hover / focus /
disabled / error 状态反馈因此只有一处定义。

视觉基线是**像素风（Pixel Art）**：token 与控件原语都在 `crates/app/style/pixel/`
（`tokens.css` 调色板 / 零圆角 / 硬阴影，`controls.css` 的 `pxl-btn` / `pxl-field` / `pxl-window` …）。
语义 token 名沿用 shadcn（`--primary` / `--muted` / `--ring` / `--destructive`），只换了值，
所以页面里的 `bg-card` / `text-primary` 照常可用；暗色由 `.dark` 覆盖。组件只拼 `pxl-*` 类名，
**两套系统的规范见 [`docs/ui/pixel.md`](../../../../docs/ui/pixel.md) 与 [`docs/ui/modern.md`](../../../../docs/ui/modern.md)**。

---

## 一、组件清单

| 组件 | 替代的原生写法 | 关键 props |
|---|---|---|
| `Button` | `<button class=button_class(…)>` | `variant` `size` `kind` `disabled` `loading`（像素风是琥珀灯闪，现代风转圈）`led` `led_on`（像素风按键里的指示灯）`class` `aria_label` `title` `aria_describedby`（禁用时的原因说明）`aria_expanded` `aria_controls`（折叠 / 下拉开关）`aria_current`（一组里「当前这个」，如答题卡的当前题）`on_click` `node_ref` |
| `ButtonLink` | `<a href class=button_class(…)>`（跳转用的「按钮」） | `href`（`TextValue`）`variant` `size` `led` `led_on` `class` `aria_label` `title` `target` `rel` `download`（保存 `data:` / `blob:` 资源时的文件名）`on_click`（收 `MouseEvent`） |
| `Input` | `<input>`（text / search / password / email / tel / url；日期与时间见 `DatePicker` / `TimePicker`） | `value` `on_change` `kind` `size` `prefix` `suffix` `clearable` `on_enter` `on_keydown` `invalid` `maxlength` `title` `autocomplete` `autocapitalize` `spellcheck` `aria_describedby` `node_ref`。`kind=InputType::Search` 的搜索框在输入法组字期间不回调 `on_change`、选字确认补一次；其余类型逐字回调 |
| `NumberField` | `<input type="number">` | `value` `on_change` `min` `max` `step` `controls`（加减步进）`kind`（`NumberKind::Decimal` = 文本框 + 数字键盘，负数与半截输入可用） |
| `Textarea` | `<textarea>` | `value` `on_change` `rows` `auto_resize` `autocomplete` `autocapitalize` `spellcheck`（填令牌 / 密钥时设 `off` / `false`） |
| `Select` + `SelectItem`（+ `SelectGroup`） | `<select>`（选项需要图标 / 徽章等富文本时，目前只有题库版本选择器）。**全站唯一的下拉引擎**：键盘、焦点、ARIA、箭头动画只在这里写一遍 | `value` `on_change` `trigger` `placeholder` `size` `invalid` `aria_label` `title` |
| `SimpleSelect` | `<select>`（选项是纯文字：筛选器、语言切换、日志「模式」）。`Select` 的数据式封装，**自己没有任何交互逻辑** | `value` `on_change` `options`（`Vec` 或 `Signal<Vec>`；`SelectOption::grouped` 可加分组标题）`placeholder` `clearable` `size` `aria_label` `title` |
| `DatePicker` | `<input type="date">` | `value`（`YYYY-MM-DD`）`on_change` `placeholder` `size` |
| `TimePicker` | `<input type="time">` | `value`（`HH:MM`）`on_change` `placeholder` `size` |
| `Checkbox` | `<input type="checkbox">`（表单项） | `checked` `on_change` `id` `aria_label` `disabled` `invalid` `class` |
| `Switch` | `<input type="checkbox">`（切换即生效的设置） | `checked` `on_change` `id` `aria_label` `disabled` `invalid` `class` |
| `RadioGroup` + `RadioGroupItem` | `<input type="radio">`（表单里的单选，选项带文字） | `value` `on_change` `disabled` `invalid`（挂在 `radiogroup` 上，样式往下传给每个单选框） |
| `ChipGroup` + `Chip` | 手写「分段选择」胶囊按钮（工具条上的短选项） | `value` `on_change` `disabled` |
| `ChipToggle` | 手写「chip 开关」（筛选条上的独立开关） | `active` `on_change` `title` `disabled` |
| `Tabs` + `TabsList` / `TabsTrigger` / `TabsPanel` | 手写 `role="tablist"` + `if` 藏显面板 / 用 `ChipGroup` 冒充视图切换 | `value` `on_change`；`TabsList` 的 `aria_label`（多组时必填）；`TabsTrigger` 的 `value`（ascii slug，同时拼 id）`disabled`；`TabsPanel` 的 `value`。面板内容第一次被切到才挂载，之后常驻、只切显隐（保留状态）。**筛选用 `ChipGroup`，不要用 Tabs** |
| `Slider` | `<input type="range">` | `value` `on_change` `min` `max` `step` `aria_valuetext` |
| `Knob` | `<input type="range">`（面板上的调谐量：音调 / 音量 / 增益） | `value` `on_change` `min` `max` `step` `label`（可见标签）`aria_label` `readout` + `unit`（读数屏）`aria_valuetext`（读屏 + 现代风的可见数值）`class`（只给现代风的 `Slider`）；像素风是 80px 旋钮 + 读数屏（拖动 / 有焦点时滚轮 / 方向键，`role="slider"`），现代风是「标签 + `Slider` + 数值」一行 |
| `Led` | — | 指示灯：`tone`（`LedTone::Rx` / `Tx` / `Warn` / `Info`）`state`（`LedState::Off` / `On` / `Blink`）`small`；`aria-hidden`，旁边必须有文字 |
| `Readout` | — | 七段读数：`text` `label`（读屏名，不给则 `aria-hidden`）`scale`（1–4）`tone`（`ReadoutTone`）；只放在屏幕（`pxl-display`）里 |
| `FileInput` | 隐藏 `<input type="file">` + 触发按钮 | `on_files` `accept` `multiple` `label` |
| `FileDropZone` | 手写 `<label for>` + 隐藏 input 的整块拖放区 | `on_file` `accept` `title`（同时是无障碍名）`icon` `hint` `choose_hint`（覆盖默认的「拖拽音频文件」提示行）`disabled` `id`（e2e 按固定 id 定位时传） |
| `Field` | 手写的 `label + 控件 + 提示` 排版 | `label` `r#for` `hint` `error` `invalid` `required` `validate_on`（默认 `Immediate`；表单类字段传 `Blur` 则失焦后才显示错误）。**表单语义的唯一来源**：`hint` / `error` 的 id 与必填状态经 context 下发给 `Input` / `Textarea` / `NumberField` / `Select` / `SimpleSelect` / `DatePicker` / `TimePicker`，控件自动补 `aria-describedby` / `aria-required` |
| `Label` | `<label>`（单独使用，如 Checkbox 旁） | `r#for` |
| `Dialog` / `Sheet` 及 `DialogHeader` 等 | — | 见源码 |
| `MobileNavDrawer` | 移动端手写「悬浮按钮 + 抽屉」/ 横滑分类条（小屏下替代被 `hidden` 藏掉的侧栏） | `open`（由调用方持有，选中项后主动 `set(false)`）`title` `button_label`（悬浮按钮无障碍名）`icon` `class`（断点类，与侧栏断点配套）+ `children`（抽屉内容，与桌面侧栏共用同一份列表）；悬浮按钮可拖动改位置，收拢视口内并持久化在 `localStorage["fab-position"]`（全站共享）；用法与坑见 `docs/ui/pixel.md` §3.3 |
| `Progress` / `Separator` / `Stat` | — | `Progress` 是分格条；加 `pxl-bar-hp` / `pxl-bar-xp` / `pxl-bar-win` / `pxl-bar-gold` 变血条 / 经验条 / 通关绿 / 金币 |
| `Table` + `TableHead` / `TableBody` / `TableRow` / `TableHeader` / `TableCell` / `TableEmpty` | 手写的 `<div class="overflow-x-auto"><table>…` | 根组件自带滚动容器；`class` 写 `min-w-[…]` 保列宽；限高写 `wrapper_class="max-h-[…]"`；`borderless` 去网格线；无可见标题传 `aria_label`；**容器溢出时自动加 `tabindex="0"` + `role="region"`**（键盘可横滚）；`TableHeader` 支持 `colspan` / `sort`（`SortOrder` → `aria-sort`），`TableCell` 支持 `colspan` / `numeric`（右对齐 + 等宽数字），空表用 `TableEmpty`（`cols` 与列数一致） |
| `ScrollRegion` | 手写的 `<div class="md-table-scroll">` 包着裸 `<table>`（现代风知识页 / 速查页，版式由 `.md-panel table:not(.md-table)` / `.md-table` 承担，不走 `Table`） | `class`（滚动样式由调用方给，如 `md-table-scroll`）`aria_label`；**溢出时自动加 `tabindex="0"` + `role="region"` 并挂上名字**，与 `Table` 同一套溢出检测（`use_scroll_overflow`）；**第一个子元素必须就是那张 `<table>`**，外面再包一层会让列宽变化漏报 |
| `Pagination` | 手写分页 / `.slice()` 截断 | `page` `total_pages`（1 起）`on_change`；页数 ≤1 不渲染；窗口算法在 `ham_web_core::pagination` |
| `ToastRoot` + `push` / `push_success` / `push_error` / `push_info` | `util::alert`（原生 alert，已清零）/ 内联「成功」文字 | 全局挂载在 `App`，可见反馈传送到 body、位于模态遮罩之上；悬停 / 聚焦时暂停计时，离开后再留 2 秒（WCAG 2.2.1）；`push_info` 给既非成功也非失败的中性提示（计时预警这类）；Dialog / Sheet 内同步公告；只在队首变化时重置计时；同类别同文案合并为 `×N`（退场中的队首不接受合并），入队规则在 `ham_web_core::toast_queue`（带单测）；页面只调 `push_*`，需确认的失败仍用 `MessageDialog` |
| `ConfirmRoot` + `confirm(title, description, confirmText, tone, onConfirm)` | `window.confirm`（已全站清零）/ 各页面自己搭确认弹窗 | 全局确认服务：`App` 根部挂一次，任何地方调 `confirm(…)` 弹像素风确认框（取消 / 执行）；`ConfirmTone::Danger` 给删除类，`Normal` 给「合并 / 批量标记」这类不可逆但无破坏的动作；执行回调只在点了执行按钮后运行（用 `try_run`，发起确认的页面已卸载则静默放弃；路由一变确认框自动收起；退场动画期间重复点击不会二次执行） |
| `Tooltip` | 原生 `title=` 气泡 | 悬停 / 聚焦显示，Escape 隐藏；组件为实际触发器关联 `aria-describedby`；气泡传送到 body 并翻转／钳制视口边界；子元素不可聚焦时补 `tabindex`，数值信息另写 `aria-label` |
| `PixelSprite`（`crate::icons`） | — | 像素精灵：`name` `scale`（每格屏幕像素数）`label`；成就徽章用 `badge_sprite(id)` |

辅助导出：`Variant` / `Size`（按钮）、`ControlSize`（输入类控件）、`TextValue`（文案）、
`button_class` / `badge_class` / `card_class`（L2 窗口）/ `flat_card_class`（L1 平卡）/
`label_class`（类名工厂）。

卡片的两个层级：`card_class` = `pxl-window`（内高光 + 4px 硬投影，给「当前任务块」用），
`flat_card_class` = `pxl-card border bg-card`（只有 2px 墨框，给次要块用）。一页里不要把
所有块都写成 L2 —— 全站同重量等于没有层级（见 `docs/ui/pixel.md` §2 硬规则 8）。

`Button` 与 `ButtonLink` 的分工是**元素语义**：要执行动作（提交、打开弹层）用 `Button`，
要跳转（能中键新开、能右键复制地址、读屏念「链接」）用 `ButtonLink` —— 两者共用同一份
`button_class`，所以视觉一致，但**不要**为了省事把链接写成 `Button`。`ButtonLink.href` 收
`TextValue`（地址常依赖页面状态），`on_click` 收 `MouseEvent`（便于在守卫里
`prevent_default()` 拦住跳转）。

`RadioGroup` 与 `ChipGroup` 的分工是**密度**而不是语义：两者都输出
`role="radiogroup"` / `role="radio"` + `aria-checked`，读屏与键盘行为一致。
表单里的单选（选项带文字、和 `Field` 标签配套）用 `RadioGroup`；工具条上的分段选择
（选项本身很短，`20m`、「三单元八木」这类，横排一行、选中即高亮）用 `ChipGroup` ——
塞进 `Field` 反而挤。独立开关（筛选条上的「只看需要的」）用 `ChipToggle`，它是
`aria-pressed` 语义，不是单选。
`input_class` 已随表单控件迁移完成删除（见下「表单控件已经收敛」）。

四个**弹层**控件（`Select` / `SimpleSelect` / `DatePicker` / `TimePicker`）的外观集中在 `ui/popover.rs`
（弹层容器、选项行、遮罩类名）；开合行为各有一个来源：退场延迟卸载与 `data-state` / `inert` / `aria-hidden`
在 `crate::presence`，`Esc` 关闭在 `crate::focus::use_escape_close`。
所以「弹出的选择框和语言切换一样」是结构上的保证，而不是靠人工对齐。

下拉再进一步：`SimpleSelect` 不是第二个下拉，它把 `Vec<SelectOption>` 拼成 `Select` + `SelectItem`
+ `SelectGroup` 交给 `Select`。键盘导航（`ham_web_core::listbox`，有单测）、`aria-activedescendant`、
Tab 离开即收起、箭头旋转过渡都只在 `Select` 里写一遍 —— 语言切换、日志「模式」与页面里的 66 处筛选器
（全是 `SimpleSelect`）和题库版本选择器（组合式 `Select`）行为天然一致。**不要**再单独写一份「简化版下拉」，要新形态就在 `Select` 上加。

---

## 二、通用约定

### 1. 受控值

所有输入组件都是**受控**的：`value` 只读、`on_change` 只写。

```rust
let kw = RwSignal::new(String::new());

view! {
  <Input
    value=kw
    on_change=Callback::new(move |v: String| kw.set(v))
  />
}
```

`value` 声明为 `#[prop(into)] Signal<…>`，可直接传 `RwSignal`、`Memo` 或 `Signal::derive`；
回调里可以先校验 / 格式化再决定是否回写。

### 2. 尺寸

- 输入类（`Input` / `NumberField` / `Textarea` / `Select` / `SimpleSelect` / `DatePicker` /
  `TimePicker`）用 `ControlSize`：
  `Sm` = `h-8`（工具栏筛选）、`Default` = `h-10`（表单主体）、`Lg` = `h-12`（触屏主表单）。
- 按钮用 `Size`：`Xs`（`h-6`，表格行内 / 工具条的**纯文本**小按钮）/ `Sm`（`h-8`）/ `Default`
  （`h-10`）/ `Lg`（`h-12`）/ `Icon`（`size-10`）/ `IconSm`（`size-6` 紧凑图标按钮 —— 像素图标
  是 24px 网格，1:1 正好撑满）。`Lg` 配更大的字号，只给落地页的主操作（首页 Hero 的三个
  入口）用，表单与工具栏里不要用。
- `Variant::Text` 是文字按钮（透明底、无边框与投影，悬停只换文字色）：句内的「编辑 /
  删除 / 关闭」用它配 `Size::Xs`；要悬停变红就在调用点补 `class="hover:text-hp-text"`。

`Default` 在移动端保留 16px 字号，避免 iOS 聚焦时页面被自动放大。

### 3. 状态反馈

| 状态 | 表现 |
|---|---|
| hover | 按钮上浮 2px，外投影变长（不用 `brightness()` 滤镜：它会算出调色板之外的颜色）；输入框描边换成 `--ring`。都只在 `@media (hover: hover)` 生效，触屏不会「粘住」悬停态 |
| active | 按钮沉 2px、外投影归零、内高光与内阴影对调（像真被按进去） |
| focus-visible | 按钮与输入类控件统一是「主色 2px 断开虚线框」（`outline: 2px dashed var(--ring)`），不是柔光环 |
| disabled | 按钮：抖动网点遮罩（`::after`）+ 去投影 + `saturate(.4)`；输入框：网点底 + `opacity: .7`；两者都 `cursor-not-allowed` + `pointer-events-none` |
| loading | （仅 `Button`）旋转图标 + `aria-busy="true"` + 自动禁用；文案保留，读屏名字不变 |
| error | 传 `invalid=…`，输出 `aria-invalid="true"`，描边切到 `destructive` |

`aria-invalid` **只在错误态出现在 DOM 上**：Tailwind 的 `aria-invalid:` 变体按属性存在与否
命中，常态写成 `"false"` 会让整站输入框都套上 destructive 描边。

聚焦环画在元素外框之外，因此**会被祖先的 `overflow` 容器裁掉**。把输入框放进滚动容器时
要给容器留 4px 余量，并用负外边距抵掉，避免视觉位置偏移：

```rust
<div class="-m-1 min-h-0 space-y-3 overflow-auto p-1">
  <Input … />
</div>
```

### 4. 文案与多语言（`TextValue`）

界面文案由 `i18n::t` 按全局语言信号翻译，切语言时**只有响应式上下文里的调用会重算**。
因此 `placeholder` / `aria_label` / `label` 一类属性统一用 `TextValue`，两种写法都支持：

```rust
placeholder = t("搜索题目")                              // 静态：够用于不会切语言的文案
placeholder = Signal::derive(move || t("搜索题目"))       // 响应式：切语言后自动更新
```

`SelectOption::label` 同样接受这两种写法。

### 5. 类名覆盖

`class` 会经 `cn()` 与默认类名合并，**后写胜出**，可安全覆盖 `w-*` / `h-*` / `text-*` 等：

```rust
<Input class="w-36 font-mono uppercase" size=ControlSize::Sm />
```

注意尺寸类带 `md:` 断点（如 `text-base md:text-sm`），要覆盖字号需同时写
`text-lg md:text-lg`。

带前缀 / 后缀 / 清除按钮的 `Input`（以及 `SimpleSelect`）会多包一层 `relative` 容器用于
绝对定位。此时 `flex-1`、`min-w-*` 这类需要作用在**外层 flex 子项**上的类要写进
`wrapper_class`，写进 `class` 只会作用于 `<input>` 本身：

```rust
<Input
  value=query
  on_change=cb
  prefix=move || view! { <Icon kind=IconKind::Search /> }
  clearable=true
  wrapper_class="min-w-48 flex-1"
/>
```

### 6. 表单控件已经收敛

全站的原生 `<input>` / `<select>` / `<textarea>` 已清干净，`input_class()` 这个「让未迁移的
输入框暂时长得一样」的类名工厂也随之删除 —— 表单控件现在只有一个来源：`ui/` 的
`Input` / `NumberField` / `Textarea` / `SimpleSelect` / `Select` / `Slider` / `Checkbox` /
`Switch` / `RadioGroup` / `FileInput` / `FileDropZone`。**页面里已经没有手写的
`<input type="file">`**：按钮式的上传走 `FileInput`，整块拖放区走 `FileDropZone` —— 后者内部是
`sr-only` 的 input，留在 Tab 序里、焦点环由 label 的 `has-[:focus-visible]` 显示
（`hidden` = `display:none` 会让键盘用户选不了文件）；非音频上传位记得覆盖 `choose_hint`。

`type="search"` 由组件额外抹掉浏览器原生外观（Safari 的圆角、Chrome 自带的「×」），
避免原生清除按钮与组件自带的清除按钮叠在一起。

### 7. 无障碍

- 无可见标签时必须传 `aria_label`；
- `Field` 的 `r#for` 与控件的 `id` 要配对（点击标签聚焦，e2e 与读屏都按此关联定位）；
- `Switch` / `Checkbox` 内部是 `<button>`，属于可关联元素，直接放进 `<label>` 即可。

---

## 三、使用示例

### 输入框（搜索框：前置图标 + 一键清除 + 回车提交）

```rust
use crate::icons::{Icon, IconKind};
use crate::ui::{ControlSize, Input, InputType};

view! {
  <Input
    value=query
    on_change=Callback::new(move |v: String| query.set(v))
    kind=InputType::Search
    size=ControlSize::Sm
    placeholder=Signal::derive(move || t("搜索题目"))
    aria_label=Signal::derive(move || t("搜索关键词"))
    prefix=move || view! { <Icon kind=IconKind::Search /> }
    clearable=true
    on_enter=Callback::new(move |_| do_search())
    class="w-36"
  />
}
```

### 数字输入（带上下限与加减步进）

```rust
use crate::ui::{ControlSize, NumberField};

view! {
  <NumberField
    value=Signal::derive(move || singles.get().to_string())
    on_change=Callback::new(move |v: String| singles.set(v.parse().unwrap_or(0)))
    min=0.0
    max=300.0
    step=1.0
    size=ControlSize::Default
  />
}
```

`controls=false` 时退回浏览器原生 spinner；`step` 的小数位决定格式化精度（不会写出
`0.30000000000000004`）。

### 下拉菜单（弹层式，选项可带富文本）

`Select` 是自绘的弹层列表（`role="combobox"` + `role="option"`），用于**不希望出现原生下拉面板**的场合。
**选项只是文字就不要直接用它**，用下面的 `SimpleSelect`（导航栏的语言切换、日志表单的「模式」都是）。
只有选项里要放图标、徽章、多行文字时才用组合式 —— 目前全站只有题库版本选择器：

```rust
let trigger = move || view! { <span>{selected_version_name()}</span> }.into_any();

view! {
  <Select
    value=selected_version                       // Signal<Option<String>>：None = 没有选中项，显示 placeholder
    on_change=Callback::new(move |v| selected_version.set(Some(v)))
    placeholder=Signal::derive(move || t("exam.select-a-bank-version"))
    trigger=trigger
  >
    {move || versions.get().into_iter().map(|v| view! {
      <SelectItem value=v.id.clone()>
        <div class="flex items-center gap-2">{status_icon(&v)} <span>{v.name.clone()}</span> {latest_badge(&v)}</div>
      </SelectItem>
    }).collect_view()}
  </Select>
}
```

`trigger` 会在响应式上下文里重跑，因此直接读信号即可拿到最新文案。
注意它不是 `<select>`，Playwright 要用 `getByRole("combobox")` + `getByRole("option")`，
不能用 `selectOption()`（`SimpleSelect` / `DatePicker` / `TimePicker` 同理，
见 `e2e/tests/fixtures.ts` 的 `pickOption` / `pickDate` / `pickTime`）。

键盘（焦点始终留在触发器上，用 `aria-activedescendant` 指当前行）：`↓` / `↑` 打开并移动，
`Home` / `End` 跳首尾，`Enter` / `Space` 选中，`Esc` 收起，`Tab` / `Shift+Tab` 离开即收起。
行为契约在 `e2e/tests/select.spec.ts`（组合式 × 数据式 × 两套皮肤各跑一遍）。

两条写 `Select` 周边代码时容易踩的坑（都在这个组件里踩过）：

- **同一页并排的多个 `Select` 不能靠 `provide_context` 共享上下文。** Leptos 的组件不会各自开一个
  作用域，后创建的会覆盖先创建的，先创建那个的选项（打开面板时才构建）读到的是兄弟的值，
  `aria-selected` / 键盘高亮全部判错。根组件用 `leptos::context::Provider` 给每个下拉自己的作用域；
  新写类似的「根 + 子项」组件同理。
- **`role="listbox"` 要挂在可滚动的那一层。** 放在外层、里面再套一层带 `tabindex` 的滚动容器，
  axe 的 `aria-required-children` 会判列表框里没有合格子节点。外层只管外观与进出场动画。

### 下拉筛选（数据式，`Select` 的封装）

筛选器选项固定，用 `Vec<SelectOption>` 描述数据即可。它内部就是 `Select`，
因此外观、键盘、动画和导航栏语言切换完全一样：

```rust
use crate::ui::{ControlSize, SimpleSelect, SelectOption};

let options: Vec<SelectOption> = bands
  .iter()
  .map(|b| SelectOption::new(b.as_str(), b.as_str()))
  .collect();

view! {
  <SimpleSelect
    value=band_filter
    on_change=Callback::new(move |v: String| band_filter.set(v))
    options=options
    placeholder=Signal::derive(move || t("全部波段"))
    size=ControlSize::Sm
    aria_label=Signal::derive(move || t("波段筛选"))
    class="w-auto"
  />
}
```

空串表示「未选择」：触发器显示 `placeholder`，同时弹层首行就是这条 `placeholder`
（`value=""`）—— 原生的 `<select>` 正是把它渲染成第一个 `<option>`，筛选器靠它回到
「全部」。`SelectOption::label` 是 `TextValue`，传 `Signal::derive` 即可跟随语言切换。
`options` 既可以直接传 `Vec<SelectOption>`（固定的筛选器，66 处都是这样），也可以传 `Signal::derive(…)`：
选项随别的状态变化时（日志表单的「模式」会把导入记录里的非预设模式补进列表）信号一变列表就跟着更新，
打开着的面板不会被重建。

**必填项**要「空值时显示占位、但列表里不能选回空」，传 `clearable=false`（默认是有占位文案就放「清空」行）。
没有可见标签的下拉（导航栏语言切换）传 `aria_label` 与 `title`；语言永远有值，所以不传 `placeholder`。

### 选择日期与时间

原生 `<input type="date">` / `type="time">` 的面板由浏览器绘制，配色、圆角、阴影都无法
跟随站点 —— 与语言切换并排时格外割裂。`DatePicker` / `TimePicker` 改为自绘弹层：日期是
月历网格（星期一开头、今天高亮），时间是时 / 分双列可滚列表；对外值与原生输入一致。

```rust
<DatePicker
  value=date            // "YYYY-MM-DD"，空串表示未选择
  on_change=Callback::new(move |v: String| date.set(v))
  // 无障碍名称同样要走 `Signal::derive`：静态 `t(…)` 在切语言后不会更新。
  aria_label=Signal::derive(move || t("log.date-utc"))
/>
<TimePicker
  value=time            // "HH:MM"
  on_change=Callback::new(move |v: String| time.set(v))
  aria_label=Signal::derive(move || t("log.time-utc"))
/>
```

星期与月份名称由浏览器 `Intl` 按当前界面语言生成（`ui/intl.rs`），既不占词典词条，
新增语言时也不会漏翻。

### 用枚举驱动单选（`RadioGroup` 只认 `String`）

`RadioGroup` 的 `value` / `on_change` 对外只有 `String`。选项来自一个枚举时，不要拿下标当值，
写一张 `(枚举值, 单选值, 文案 key)` 的常量表，双向查：

```rust
// pages/tools/filter_design.rs
const KINDS: [(FilterKind, &str, &str); 4] = [
  (FilterKind::LowPass, "low-pass", "低通"),
  (FilterKind::HighPass, "high-pass", "高通"),
  // …
];
```

单选值取**语义名**（`"low-pass"`）而不是下标：重排选项不会把用户已经选的那一项换成另一项，
也不会让存在 `localStorage` 里的旧值悄悄指向别的东西。

### 开关

```rust
<label class="flex cursor-pointer items-center gap-1.5 text-muted-foreground">
  <Switch checked=show_grayline on_change=Callback::new(move |v| show_grayline.set(v)) />
  {move || t("灰线")}
</label>
```

### 滑块

```rust
<Slider
  value=Signal::derive(move || f64::from(tone_hz.get()))
  on_change=Callback::new(move |v: f64| set_tone(v as u32))
  min=300.0
  max=1200.0
  step=10.0
  aria_label=Signal::derive(move || t("播放音调"))
  aria_valuetext=Signal::derive(move || format!("{} Hz", tone_hz.get()))
  class="w-32"
/>
```

轨道 / 滑块的跨浏览器自绘在 `style/pixel/surfaces.css` 的 `.ui-slider`；组件只把进度以
`--slider-fill` 传给 CSS，无需 JS 参与。

### 文件选择

```rust
<FileInput
  accept=".json"
  label=t("导入备份")
  variant=Variant::Outline
  on_files=Callback::new(move |files: Vec<web_sys::File>| {
    if let Some(f) = files.into_iter().next() {
      run_import(f, false);
    }
  })
/>
```

整块可点击、可拖放的上传区（上传解码类页面统一用它，别在页面手写 `<label for>` + 隐藏 input）：

```rust
<FileDropZone
  on_file=Callback::new(move |file: web_sys::File| start_decode(file))
  accept=".wav,audio/wav,audio/x-wav,audio/*"
  title=Signal::derive(move || t("tools.choose-a-rtty-recording"))
  icon=IconKind::Waves
  hint=Signal::derive(move || t("tools.supports-martin-m1-m2"))
  disabled=processing
/>
```

通用提示行「点击选择或拖拽」由组件内置；`hint` 是可选的补充说明行，不传则不渲染。
上传的不是音频时用 `choose_hint` 覆盖内置提示行（图片上传位传
`tools.click-to-choose-or`）。e2e 需要固定 id 定位（如 `#psk-file`）时传 `id`，
否则组件内部生成。

### 表单行（标签 + 说明 + 错误）

```rust
<Field
  label=Signal::derive(move || format!("{}（{unit}）", t(label)))
  r#for=id.clone()
  error=error_message
>
  <NumberField id=id value=signal on_change=cb />
</Field>
```

### 按钮

```rust
<Button
  variant=Variant::Default
  size=Size::Sm
  disabled=Signal::derive(move || busy.get())
  on_click=Callback::new(move |_| save())
>
  {move || t("保存")}
</Button>
```

表单里的提交按钮用 `kind=ButtonKind::Submit`。

---

## 四、写法对照表（原生元素 → 该用哪个组件）

| 原来写法 | 改为 |
|---|---|
| `<input type="text">` / `type="search"` / `type="password"` … | `<Input kind=InputType::…>` |
| `<input type="date">` | `<DatePicker>` |
| `<input type="time">` | `<TimePicker>` |
| `<input type="datetime-local">` | `<DatePicker>` + `<TimePicker>`（各自只改自己那一半） |
| `<input type="number">` | `<NumberField>` |
| `<textarea>` | `<Textarea>` |
| `<select>` + `<option>`（选项固定） | `<SimpleSelect options=…>`（弹层式，外观同语言切换） |
| `<select>`（选项需富文本 / 动态） | `<Select>` + `<SelectItem>` |
| `<input type="checkbox">`（表单项） | `<Checkbox>` |
| `<input type="checkbox">`（设置项） | `<Switch>` |
| `<input type="radio">` | `<RadioGroup>` + `<RadioGroupItem>` |
| `<input type="range">` | `<Slider>` |
| 隐藏 `<input type="file">` + 按钮 | `<FileInput>` |
| 手写 `<label for>` + 隐藏 input 的整块拖放区 | `<FileDropZone>` |
| 「搜索图标 + 输入框 + 清除按钮」手写组合 | `<Input kind=InputType::Search prefix=… clearable=true>` |
| `<button class=button_class(…)>` | `<Button>` |
| 手写 `<div class="space-y-1.5"><label>…</label>…` | `<Field>` |

把已有的原生写法换成组件时：

1. 把原生元素的 `prop:value` / `on:input` 换成 `value` / `on_change`（回调收的是新值字符串）；
2. 修饰类（宽度、字体等）搬到 `class="…"`，高度交给 `size`；
3. 需要多语言跟随的 `placeholder` / `aria-label` 用 `Signal::derive(move || t("…"))`；
4. 有标签的一并换成 `Field`，并让 `r#for` 与控件 `id` 配对；
5. 跑 `cargo make check`（格式 + clippy + 测试 + 文案校验）。

---

## 五、开源方案评估

结论：**不引入第三方组件库，继续在本目录自维护**。理由（逐条）见
[`docs/ui/decisions.md`](../../../../docs/ui/decisions.md) §10。需要新增组件时，按 `ui/` 现有约定补齐即可：
优先复用 `control_class` / `CONTROL_BASE` 保证外观一致，模态的交互与无障碍处理参考 `dialog/dialog_root.rs`（焦点 / 滚动锁 / Esc 三件套 + 在场属性）。
