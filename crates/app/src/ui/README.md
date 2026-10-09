# 基础 UI 组件（`crates/app/src/ui`）

> **写代码前先读 [`docs/ui-components.md`](../../../../docs/ui-components.md)**：那里是「用哪个组件、
> 怎么写、有哪些坑、怎么验收」的规范；本文件是组件清单、props 与逐个使用示例。

本目录是全站**表单控件的唯一来源**。页面里不再直接写 `<input>` / `<select>` / `<textarea>` /
`<input type="range">`，统一用这里的组件 —— 颜色、圆角、间距、字体与 hover / focus /
disabled / error 状态反馈因此只有一处定义。

视觉基线沿用项目现有的 shadcn/ui（new-york）Token：`--primary` / `--muted` / `--ring` /
`--destructive`、`--radius`、`Geist` 字体，暗色模式由 `.dark` 覆盖，无需组件侧额外适配。

---

## 一、组件清单

| 组件 | 替代的原生写法 | 关键 props |
|---|---|---|
| `Button` | `<button class=button_class(…)>` | `variant` `size` `kind` `disabled` `loading` `class` `aria_label` `title` `on_click` `node_ref` |
| `ButtonLink` | `<a href class=button_class(…)>`（跳转用的「按钮」） | `href`（`TextValue`）`variant` `size` `class` `aria_label` `title` `target` `rel` `on_click`（收 `MouseEvent`） |
| `Input` | `<input>`（text / search / password / email / tel / url；日期与时间见 `DatePicker` / `TimePicker`） | `value` `on_change` `kind` `size` `prefix` `suffix` `clearable` `on_enter` `on_keydown` `invalid` `maxlength` `title` `autocomplete` `autocapitalize` `spellcheck` `aria_describedby` `node_ref` |
| `NumberField` | `<input type="number">` | `value` `on_change` `min` `max` `step` `controls`（加减步进）`kind`（`NumberKind::Decimal` = 文本框 + 数字键盘，负数与半截输入可用） |
| `Textarea` | `<textarea>` | `value` `on_change` `rows` `auto_resize` |
| `Select` + `SelectItem` | `<select>`（选项需要富文本展示时） | `value` `on_change` `trigger` `placeholder` `size` `invalid` |
| `NativeSelect` | `<select>`（选项固定的筛选器） | `value` `on_change` `options`（`SelectOption::grouped` 可加分组标题）`placeholder` `size` |
| `DatePicker` | `<input type="date">` | `value`（`YYYY-MM-DD`）`on_change` `placeholder` `size` |
| `TimePicker` | `<input type="time">` | `value`（`HH:MM`）`on_change` `placeholder` `size` |
| `Checkbox` | `<input type="checkbox">`（表单项） | `checked` `on_change` `disabled` |
| `Switch` | `<input type="checkbox">`（切换即生效的设置） | `checked` `on_change` `disabled` |
| `RadioGroup` + `RadioGroupItem` | `<input type="radio">`（表单里的单选，选项带文字） | `value` `on_change` `disabled` |
| `ChipGroup` + `Chip` | 手写「分段选择」胶囊按钮（工具条上的短选项） | `value` `on_change` `disabled` |
| `ChipToggle` | 手写「chip 开关」（筛选条上的独立开关） | `active` `on_change` `title` `disabled` |
| `Slider` | `<input type="range">` | `value` `on_change` `min` `max` `step` `aria_valuetext` |
| `FileInput` | 隐藏 `<input type="file">` + 触发按钮 | `on_files` `accept` `multiple` `label` |
| `Field` | 手写的 `label + 控件 + 提示` 排版 | `label` `r#for` `hint` `error` `required` |
| `Label` | `<label>`（单独使用，如 Checkbox 旁） | `r#for` |
| `Dialog` / `Sheet` 及 `DialogHeader` 等 | — | 见源码 |
| `Progress` / `Separator` / `Stat` | — | 见源码 |

辅助导出：`Variant` / `Size`（按钮）、`ControlSize`（输入类控件）、`TextValue`（文案）、
`button_class` / `badge_class` / `card_class` / `label_class`（类名工厂）。

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

四个**弹层**控件（`Select` / `NativeSelect` / `DatePicker` / `TimePicker`）的类名与开合行为
集中在 `ui/popover.rs`：弹层容器、选项行、遮罩、退场延迟卸载、`Esc` 关闭都只有一份实现，
所以「弹出的选择框和语言切换一样」是结构上的保证，而不是靠人工对齐。

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

- 输入类（`Input` / `NumberField` / `Textarea` / `Select` / `NativeSelect` / `DatePicker` /
  `TimePicker`）用 `ControlSize`：
  `Sm` = `h-8`（工具栏筛选）、`Default` = `h-9`（表单主体）、`Lg` = `h-11`（触屏主表单）。
- 按钮用 `Size`：`Sm` / `Default` / `Icon`。

`Default` 在移动端保留 16px 字号，避免 iOS 聚焦时页面被自动放大。

### 3. 状态反馈

| 状态 | 表现 |
|---|---|
| hover | 颜色加深 + 阴影加大（`hover:shadow-*`，Tailwind v4 下仅 hover 设备生效） |
| active | 按下 `scale(0.97)` + 去阴影 + 涟漪泛光（`btn-ripple`，见 `style/input.css`） |
| focus-visible | `border-ring` + `ring-ring/50` + `ring-[3px]` |
| disabled | `opacity-50` + 去阴影 + 降饱和 + `cursor-not-allowed` + `pointer-events-none` |
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

带前缀 / 后缀 / 清除按钮的 `Input`（以及 `NativeSelect`）会多包一层 `relative` 容器用于
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
`Input` / `NumberField` / `Textarea` / `NativeSelect` / `Select` / `Slider` / `Checkbox` /
`Switch` / `RadioGroup` / `FileInput`。只剩 6 处隐藏的 `<input type="file">`（上传按钮背后的
`class="hidden"` 元素，不参与展示）。

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

`Select` 是自绘的弹层列表（`role="combobox"` + `role="option"`），用于**不希望出现原生下拉面板**的场合，
例如导航栏的语言切换：

```rust
let locale = i18n::locale();
let current = locale.clone();
let trigger = move || view! { {current.get().label()} }.into_any();

view! {
  <Select
    value=Signal::derive(move || Some(locale.get().code().to_owned()))
    on_change=Callback::new(move |v: String| i18n::set_locale(Locale::from_code(&v)))
    placeholder=Signal::derive(move || t("切换语言"))
    trigger=trigger
    size=ControlSize::Sm
    class="w-auto max-w-[8rem]"
  >
    {move || {
      Locale::ALL
        .iter()
        .map(|&l| view! { <SelectItem value=l.code()>{l.label()}</SelectItem> })
        .collect_view()
    }}
  </Select>
}
```

`trigger` 会在响应式上下文里重跑，因此直接读信号即可拿到最新文案。
注意它不是 `<select>`，Playwright 要用 `getByRole("combobox")` + `getByRole("option")`，
不能用 `selectOption()`（`NativeSelect` / `DatePicker` / `TimePicker` 同理，
见 `e2e/tests/fixtures.ts` 的 `pickOption` / `pickDate` / `pickTime`）。

### 下拉筛选（弹层式，与语言切换同款）

筛选器选项固定，用 `Vec<SelectOption>` 描述数据即可。弹层外观与 `Select` **共用
`ui/popover.rs` 的定义**，因此和导航栏语言切换长得一模一样：

```rust
use crate::ui::{ControlSize, NativeSelect, SelectOption};

let options: Vec<SelectOption> = bands
  .iter()
  .map(|b| SelectOption::new(b.as_str(), b.as_str()))
  .collect();

view! {
  <NativeSelect
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
选项**动态**变化时（如录入表单的「模式」会带上 CAT 读到的非标准模式）改用
`Select` + `SelectItem`，它的子节点是响应式的。

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

轨道 / 滑块的跨浏览器自绘在 `style/input.css` 的 `.ui-slider`；组件只把进度以
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

## 四、迁移对照表

| 原来写法 | 改为 |
|---|---|
| `<input type="text">` / `type="search"` / `type="password"` … | `<Input kind=InputType::…>` |
| `<input type="date">` | `<DatePicker>` |
| `<input type="time">` | `<TimePicker>` |
| `<input type="datetime-local">` | `<DatePicker>` + `<TimePicker>`（各自只改自己那一半） |
| `<input type="number">` | `<NumberField>` |
| `<textarea>` | `<Textarea>` |
| `<select>` + `<option>`（选项固定） | `<NativeSelect options=…>`（弹层式，外观同语言切换） |
| `<select>`（选项需富文本 / 动态） | `<Select>` + `<SelectItem>` |
| `<input type="checkbox">`（表单项） | `<Checkbox>` |
| `<input type="checkbox">`（设置项） | `<Switch>` |
| `<input type="radio">` | `<RadioGroup>` + `<RadioGroupItem>` |
| `<input type="range">` | `<Slider>` |
| 隐藏 `<input type="file">` + 按钮 | `<FileInput>` |
| 「搜索图标 + 输入框 + 清除按钮」手写组合 | `<Input kind=InputType::Search prefix=… clearable=true>` |
| `<button class=button_class(…)>` | `<Button>` |
| 手写 `<div class="space-y-1.5"><label>…</label>…` | `<Field>` |

迁移步骤：

1. 把原生元素的 `prop:value` / `on:input` 换成 `value` / `on_change`（回调收的是新值字符串）；
2. 修饰类（宽度、字体等）搬到 `class="…"`，高度交给 `size`；
3. 需要多语言跟随的 `placeholder` / `aria-label` 用 `Signal::derive(move || t("…"))`；
4. 有标签的一并换成 `Field`，并让 `r#for` 与控件 `id` 配对；
5. 跑 `cargo make check`（格式 + clippy + 测试 + 文案校验）。

---

## 五、开源方案评估

结论：**不引入第三方组件库，继续在本目录自维护**。理由：

1. **没有与本项目管线对齐的成熟实现**。shadcn/ui 官方只覆盖 React / Vue / Svelte，
   其「复制粘贴式组件」的精髓就是类名写在自己仓库里；Leptos 侧的移植项目
   （如 `leptonic`、各类 `leptos-shadcn` 尝试）要么自带一套与本项目 oklch Token 不兼容的
   主题体系，要么跟进 Leptos 版本滞后（`leptos` 0.8 的响应式 API 在 0.7 之后有破坏性变更）。
2. **可定制性**。本项目的 `cn()`（`src/cn.rs`）已经实现了 tailwind-merge 的冲突组覆盖，
   组件类名可直接被 `class` 覆盖；引入外部库反而要跨过它的样式封装层去改。
3. **体积与构建**。前端是 `wasm32` + `wasm-opt -z` 的体积优先构建，任何额外的
   组件依赖都会直接体现在产物大小上；而当前每个组件只是几十行 `view!`，编译期即展开。
4. **兼容性**。项目是纯 CSR（`leptos::mount::mount_to_body`），没有 SSR / hydration 约束，
   自维护组件的边界处理（滚动锁、焦点、Portal）已在 `ui/dialog/shared.rs` 里沉淀。

需要新增组件时，按 `ui/` 现有约定补齐即可：优先复用 `control_class` / `CONTROL_BASE`
保证外观一致，交互与无障碍处理参考 `dialog/shared.rs`。

---

## 六、本轮迁移进度

基线（改动前全量扫描 `src/`）：原生 `<select>` 26 处、`<textarea>` 2 处、`<input>` 约 207 处
（其中 number 106、text 37、checkbox 16、range 10、file 10、date 4、time 3、search 2）。

已替换为组件：

| 位置 | 替换内容 |
|---|---|
| `pages/log/grid_map/toolbar.rs` | 2 × `Input`、5 × `NativeSelect`、2 × `Switch`、`Button` |
| `pages/log/entry_list.rs` | `Input`（search + 清除）、3 × `NativeSelect` |
| `pages/tools/ohms_law.rs` | 3 × `NumberField` + `Field` |
| `components/exam/custom_paper_dialog.rs` | 3 × `NumberField` + `Field`、2 × `Button` |
| `pages/morse/morse_page.rs` | 2 × `Slider` |
| `pages/morse/koch_trainer.rs` | `Input`、`Button`（`ButtonKind::Submit`） |
| `components/common/note_editor.rs` | `Textarea`、2 × `Button` |
| `pages/tools/backup_tool.rs` | 2 × `FileInput`、`Button` |
| `components/search_dialog.rs` | `Input`（`InputType::Search` + 前置图标 + 清除） |
| `components/practice/practice_search_dialog.rs` | `Input`（回车跳转 + 清除，取代手写输入框与清除按钮） |
| `pages/browse/browse_page.rs` | `Input`（搜索） |
| `pages/glossary/glossary_view.rs` | `Input`（搜索，内置清除取代手写清除按钮） |
| `pages/qcode/qcode_view.rs` | `Input`（搜索） |
| `pages/most_wanted/wanted_tracker.rs` | `Input`（搜索） |
| `components/navigation/locale_toggle.rs` | `Select` + `SelectItem`（界面语言切换，弹层式菜单） |

### 下拉与日期时间统一（本轮）

- 全站剩余原生 `<select>` **20 处** → `NativeSelect`（`cat_control` 2、`qsl_labels` 2、
  `entry_form` 2、`contest_log/view` 3、`listen` 2、`cabrillo` / `print_page` / `voacap_card` /
  `nec/canvas` / `bookmark_card` / `contest_scorer` / `feedline_loss` / `pass_predictor` /
  `mistakes` 各 1）；选项动态的「模式」用 `Select` + `SelectItem`。
- 原生 `date` 4 处、`time` 3 处、`datetime-local` 1 处 → `DatePicker` / `TimePicker`。
- 新增 `ui/popover.rs`（弹层基座）、`ui/intl.rs`（`Intl` 日期格式化）；
  新增词条 `common.previous-month` / `common.next-month` / `exam.target-date`。

剩余部分集中在 `pages/log/*`（表单与筛选）、`pages/morse/*`（range 与文本输入），
可按上表逐页推进；`pages/*/…_calculator.rs`、`pages/log/qsl_sync_dialog.rs` 与
`pages/tools/mod.rs` 的本地 `const INPUT` / `const TEXTAREA` 均已删除（`grep` 自检 ① 通过）。

### 小工具页统一（本轮）

`pages/tools/*` 的 39 个工具组件此前各自手写表单，同一个页面里「欧姆定律」和它的邻居长得
不一样：

- **控件**：原生 `type="number"` 输入约 90 处、`type="text"` 7 处、弹层下拉 2 处；
- **标签**：两种排版 —— 手写 `<label>` 里的小字 `text-xs text-muted-foreground`，
  与 `Field` 的 `text-sm font-medium` 混用；
- **互斥小按钮**：4 种互不相同的写法（`rounded-md` 实心、`rounded-full` 实心、
  `rounded-full` 描边 + `bg-primary/10`、`rounded-lg` 描边 + `bg-primary/10`），
  选中态在 `bg-primary` 与 `border-primary bg-primary/10` 之间摇摆。

本轮全部改为 `ui/` 组件：

| 原来写法 | 改为 | 出现位置 |
|---|---|---|
| 原生 `<input type="number">` | `<Field>` + `<NumberField controls=false>` | 35 个文件，90 处 |
| 原生 `<input type="text">` | `<Field>` + `<Input>` | `aprs_codec` / `callsign_lookup` / `cascade_gain` / `distance_bearing` / `mode_encoder` / `resistor_parallel` |
| 手写 `<label>` + 小字标签 | `<Field label=… r#for=…>`（`id` 由 `util::unique_id` 生成） | 全部 39 个工具组件 |
| 手写互斥 chip 按钮 | `<RadioGroup>` + `<RadioGroupItem>` | `filter_design`（滤波类型）、`cw_bandwidth`（衰落 / 非衰落）、`tone_squelch`（中继波段、正负频差） |
| `<select>` 外包手写标签 | `<Field>` + `<NativeSelect>` | `contest_scorer` / `feedline_loss` |

两个取舍：

1. **`controls=false`**：工具页录入的是连续量（14.2 MHz、0.1 步进、任意小数），加减步进按钮
   既没用又挤占宽度；同页已迁移的 `ohms_law` 也是这么写的，保持一致。
2. **删除 `pages/tools/mod.rs` 的本地 `const INPUT`**：它写的是 `h-10 rounded-lg … ring-2`，
   与全站 `control_class` 的 `h-9 rounded-md … shadow-xs … ring-[3px]` 不一致 ——
   这正是「同一页两种输入框」的来源。`const RESULT` 保留：它只是只读结果区，不是表单控件。

`RadioGroup` 对外只有 `String`，因此 `filter_design` 的 `FilterKind` 走一张
`(枚举值, 单选值, 文案 key)` 常量表做双向映射（单选值取语义名而不是下标，重排选项不会
把用户的选择换掉）。

