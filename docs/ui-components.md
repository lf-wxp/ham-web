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
   `Button` / `ButtonLink` 内部与 §8 清单里的几处例外使用），更不新增本地 `const INPUT` / `CHIP_ON`。外观规则见 §0.5。
2. **标签关联、无障碍名、多语言文案**是组件与调用点**共同**的责任，缺一样都算没写完
   （e2e 也靠它们定位，见 §4）。
3. 改完**必须**过 `cargo make check`；动到页面组件再跑相关 e2e（见 §7）。

## 0.5 像素规范（Pixel Art 视觉基线）

整站是「16 位掌机 + 复古电台」风格。**外观只在 `crates/app/style/pixel/` 定义一次**，组件与页面只用
语义 token 与 `pxl-*` 控件类，不要在页面里再写圆角、模糊、渐变、软阴影。

| 文件 | 管什么 |
|---|---|
| `tokens.css` | 亮 / 暗两套调色板（映射到 `--background` / `--primary` / `--border` 等**既有语义名**）、零圆角、硬偏移阴影、字号阶梯、`steps()` 缓动 |
| `schemes.css` | 其余配色方案（森林 / 海洋 / 晚霞 / 石墨）的亮 / 暗变量块；由 `cargo make pixel-schemes` 从 `crates/core/src/color_scheme.rs` 生成，**勿手改** |
| `palette.css` | Tailwind 默认色阶（`green-600` 等）整体换成像素调色板；由固定规则生成，**不要手调单个色值** |
| `fonts.css` / `fonts-pixel.css` | 字体声明；后者由 `cargo make fonts-pixel`（`crates/tools/src/pixel/fonts.rs`）生成，**勿手改** |
| `surfaces.css` | 全局基线、背景底纹、存量卡片兜底、滚动条、图标 / 精灵尺寸 |
| `controls.css` | `pxl-btn` / `pxl-field` / `pxl-window` / `pxl-popover` / `pxl-check` / `pxl-switch` / `pxl-chip` / `pxl-bar` 等控件原语 |
| `motion.css` | 全部逐帧动画与「静态降级」 |

**硬规则**

1. **零圆角、零模糊、零渐变**：不要写 `rounded-*`（全局已归零，写了也无效）、`backdrop-blur`、
   `bg-gradient-*`、带模糊半径的 `shadow-[…]`。需要强调就换色、加粗描边、用抖动底纹。
   真要一个圆（极少）给元素加 `data-keep-round`。
2. **尺寸取 4 的倍数**：间距、控件高度走 Tailwind 刻度（`h-8` / `h-10` / `h-12`）；描边 2px，
   位移 2px 的整数倍。**不要写 `h-[37px]` 这类奇数**，点阵边缘会错位。
3. **图标按 24px 的整数倍显示**（`size-6` / `size-12`）：像素图标是 24×24 网格，
   16 / 20px 会有半个像素的抗锯齿。`size-4` / `h-4 w-4` 在 `surfaces.css` 里被统一抬到 24px，
   新代码直接写 `size-6`。
4. **字体**：正文走点阵中文体（缝合像素）；标题 / HUD / 数字读数用 `pxl-title`（Press Start 2P）
   或 `pxl-label`（Silkscreen）。**点阵字体没有粗体**：不要用 `font-bold` / `font-semibold`
   做强调（会被合成成糊边），改用颜色、`pxl-title` 或加框。
5. **状态只信 ARIA**：选中 / 按下态由 `aria-checked` / `aria-pressed` / `data-state` 驱动样式，
   不要在 Rust 里再拼一份 `ON` / `OFF` 类名（`Chip` 就是这么做的）。
6. **颜色分「填充」与「文字」两套**：`--pxl-hp` / `xp` / `gold` / `win` 是画在深色槽里的**填充色**
   （血条、金币），直接拿来写字对比度不够（亮色下金色文字落在奶油底只有 1.4:1）。**写字一律用
   `text-hp-text` / `text-xp-text` / `text-gold-text` / `text-win-text`**；不要写
   `text-[color:var(--pxl-gold)]` 这类任意值。Tailwind 色阶（`text-green-600`、`dark:text-red-400`）
   可以照常用 —— 它们的文字档由 `cargo make pixel-palette` 保证在真实表面上达标。
   改了调色板就跑它（`cargo make pixel-check` 只审计，不达标时退出码为 1，已并入 `check` / `ci`）。
7. **配色一律走语义 token，别在页面里写死色值**：`bg-card` / `text-muted-foreground` /
   `border-ink` 会随用户选的**配色方案**（经典 / 森林 / 海洋 / 晚霞 / 石墨，与明暗正交）换色；
   写死 `#fff8e1` 的地方切方案后就成了孤岛（DOM 里没有这类写死的色值；`share_score.rs` 这类导出成绩
   图的画布色是有意固定的，不在此列）。状态色（`hp` / `xp` / `gold` / `win` / `destructive`）
   是语义色，不随方案变。
   **新增 / 调整一个方案**：只改 `crates/core/src/color_scheme.rs` 里的 5 个种子色（底 / 墨 / 主色 /
   强调面 / 焦点环，亮暗各一套），然后 `cargo make pixel-schemes`。其余变量按规则派生，字色按它
   实际会落到的每个表面校正；单测会拦下「表面太亮 / 太暗」这类会让共享色阶不够对比度的种子色，
   `pixel-check` 再按真实表面审计整套 Tailwind 色阶。**别手改 `schemes.css`**。
8. **动画只用 `motion.css` 里的**：位移 / 显隐 / 抖动 / 闪烁，缓动一律 `steps()`；
   **不要做缩放动画**（中间帧会让点阵文字变糊）。新增动画必须同时写静态降级。

**可用的像素元素**

- 窗口面板：`pxl-window`（卡片）、`pxl-popover`（弹层）、`pxl-titlebar`（标题栏）。
- 进度条：`Progress` + `class="pxl-bar-hp"`（血条，低于 25% 自动闪烁）/ `pxl-bar-xp`（经验）/
  `pxl-bar-win` / `pxl-bar-gold`。
- 精灵：`PixelSprite name="hero" scale=4`（`scale` 是**每格的屏幕像素数**，从接口上杜绝非整数倍）；
  成就徽章用 `badge_sprite(id)` 取精灵名。
- 动效类：`pxl-shake`（受击）、`pxl-flash`（闪白）、`pxl-bob`（待机）、`pxl-blink`、`pxl-float-up`（飘字）、
  `pxl-typewriter`（打字机，`--steps` 传字符数）。

**两个显示偏好**（设置里，持久化在 `ui:pixelMotion` / `ui:readableFont`，落在 `<html>` 的 `data-*`）：
`data-pixel-motion="off"` 或系统「减少动态效果」时所有动画降为静态帧；
`data-readable-font="on"` 时正文切回抗锯齿的 Geist（标题 / HUD 仍是像素字体）。
**新增样式要在这两种状态下都看一眼**。

## 1. 决策表：要什么 → 用哪个

| 场景 | 用 | 不要用 |
|---|---|---|
| 单行文本 / 搜索 / 密码 / 邮箱 / tel / url | `Input`（`kind=InputType::Search` 等） | `<input>` |
| 数字（有上下限、步进） | `NumberField`（`controls=false` 关掉加减按钮） | `<input type="number">` |
| 数字（**负数**，或要保留 `-` / `1.` 这类半截输入） | `NumberField` + `kind=NumberKind::Decimal` | 默认的 `type="number"`：半截输入时 `value` 会返回空串 |
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

## 1.5 页面骨架与内容宽度

内容页**只有一种宽度**：`max-w-5xl`（1024px），而且只在两处定义 —— `PageContainer`
（正文）与 `PageHeader`（页头标题栏），都在 `crates/app/src/components/common/`。
页面里**不写** `max-w-*`、**不写** `container`、**不写** `mx-auto` 的宽度容器。

一个内容页的骨架就这两行：

```rust
<PageHeader title=move || t("域.标题") subtitle=move || t("域.副标题") />
<PageContainer>
  …内容块（`SectionCard` 等）…
</PageContainer>
```

| 要什么 | 用 | 说明 |
|---|---|---|
| 粘性页头（标题 + 副标题） | `PageHeader` | `actions=ViewFn::from(move || view! { … })` 放右侧操作区；页头上方还有别的粘性栏时传 `class="top-16"` |
| 多行页头（标题行 + 工具条 / 确认条） | `PageHeader` 的 `children` | 子节点渲染在标题行下方；自己写一行 `mx-auto flex max-w-5xl …` 与标题行左边缘对齐 |
| 页面正文 | `PageContainer` | 默认 `space-y-6 px-4 py-5`；间距 / 内边距不同就用 `class` 覆盖；页面内锚点用 `id="…"` |
| 知识库正文页 | `KnowledgePage` | = `PageHeader` + `PageContainer`，并负责按需拉取知识库译文 |
| 打印类内容（`/print` 的纸、QSL 标签、QSL 设计器画布） | 物理尺寸（`max-w-[210mm]`、`@page size`、`mm` 值） | 量的是**纸**，不是内容宽度 —— **唯一**不受 `max-w-5xl` 约束的地方，理由见下 |

### 为什么打印类页面不用 `max-w-5xl`

它们量的是纸，和内容宽度不是一套坐标系：

- `max-w-5xl` = 1024px，按 96dpi 折算 ≈ **271mm**，比 A4 的 210mm 宽 29%。套上去屏幕上的
  「纸」就不是 A4 比例了，预览会骗人。
- 打印时的版心由 `@page` 决定（`crates/app/style/pixel/surfaces.css`：`size: A4; margin: 14mm 12mm`），
  页面上用 `print:max-w-none print:p-0` 把屏幕约束整个放开。所以 `max-w-[210mm]` +
  `px-[14mm] py-[12mm]` **只在屏幕上复刻那份 `@page` 版心**（所见即所得），根本不进打印流程。
- 同族的还有 `pages/qsl_labels/`（`@page { size: … }` 按标签纸动态给尺寸、
  单元格用 `px-[2.5mm] py-[1.8mm]`）与 `pages/qsl_designer.rs`（`width: 210mm; height: 296mm`）。

给它们套 `PageContainer` 的结果是「打印出来一模一样，但屏幕预览变成 271mm 的假纸」，
属于「统一了但统一错了」—— 所以这一类**允许**写 `mm` 尺寸，别再来「统一」一遍。

注意范围：**只有「纸」本身**用物理尺寸。这类页面的**外壳照旧**走 `PageHeader` /
`PageContainer`（`qsl_labels`、`qsl_designer` 的工具栏就是），别把整套页面都豁免掉。
唯一的另一处是 `/print` 的工具栏：它刻意用 `max-w-[210mm]` 与预览的纸左右对齐。

**硬规则**

1. **页面里不写宽度类**。`mx-auto` + `max-w-*`（含 `container`）都交给 `PageHeader` /
   `PageContainer`；要改全站宽度只改组件，不搜页面。
2. **间距只在 `PageContainer` 的 `class` 上覆盖**：`cn` 会做 tailwind-merge，`class="space-y-4"`
   覆盖默认的 `space-y-6`。原页面没有纵向间距时写 `class="space-y-0"`，不要为此另起一个
   `<div>`。负边距 / `pb-*` 之类的例外同样写在 `class` 里。
3. **只有一种内容宽度**。嫌宽 / 嫌窄都先用满 `max-w-5xl`，把内容本身做窄（限宽交给内容块，
   不要缩容器）。真要另一种宽度，先在评审里说明，不要悄悄写 `max-w-3xl`。
   **例外只有打印类页面**：它们写 `mm`（纸张尺寸），不受这条约束，理由见上。
4. **页脚与底栏与正文同宽**（`max-w-5xl`），左边缘与正文对齐。
   **顶栏是唯一的外壳例外**：`nav_bar.rs` 继续用 `container`（宽度随断点增长）—— 整行桌面
   导航 + HUD 在 xl 断点需要 >1024px，收窄到 `max-w-5xl` 会在 1280px 视口把顶栏撑出屏幕
   （`e2e/tests/rpg_flow.spec.ts` 的「顶栏：各宽度 × 各语言都不撑出屏幕」会拦住这种改动）。
5. 需要事件的正文容器（如答题页的 `on:touchstart`）：外层包一个只带事件属性的 `<div>`，
   `PageContainer` 放里面。

自检（输出是一份**很短的封闭清单**，逐条对得上即可；出现新行就说明又手抄了一份容器）：

```bash
cd crates/app/src
grep -rn --include='*.rs' --exclude-dir=ui -E 'class="[^"]*\bmax-w-(2xl|3xl|4xl|5xl|6xl)\b|class="container' pages components app \
  | grep -vE 'min-w-|<Dialog|sm:max-w'
```

| 位置 | 为什么允许 |
|---|---|
| `components/common/page_header.rs` | 页头宽度就在这里定义（`page_container.rs` 用 `cn(&[…])` 拼类名，不出现在这条 grep 里） |
| `app/footer.rs`、`components/common/bottom_bar.rs` | 外壳与正文左边缘对齐 |
| `pages/mistakes/mistakes_page.rs`、`pages/bookmarks/bookmarks_page.rs`、`pages/qsl_labels/qsl_labels_page.rs` | `PageHeader` 的 children 行，与标题行对齐 |
| `components/navigation/nav_bar.rs`（2 处 `container`） | 顶栏例外，理由见上 |
| `app/storage_warning.rs` | 全站横幅，复刻页头行的宽度 |

不在范围内的：弹窗宽度（`Dialog` 的 `sm:max-w-*`）是另一套尺度；
`pages/morse/morse_trainer.rs` 报文行的读宽上限写在 `class = if … { "…" }` 的字符串里，
同样与页面宽度无关。

**写新页面的顺序**：`PageHeader` → `PageContainer` → 内容块用现成组件（`SectionCard` 等）
→ 跑 `cargo make check`。

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
# ④ 手写 chip 的样式常量：应为 0
grep -rln --include='*.rs' --exclude-dir=ui 'CHIP_ON' .
# ⑦ 把填充色 token 当文字色用（对比度不够）：应为 0
grep -rnE --include='*.rs' 'text-\[color:var\(--pxl-(hp|xp|gold|win)\)\]' crates/app/src
# ⑥ 页面里残留的圆角 / 模糊 / 渐变：全局 CSS 已兜底（零圆角、零模糊），但新代码不该再写
grep -rnE --include='*.rs' --exclude-dir=ui 'backdrop-blur|bg-gradient|bg-linear|shadow-\[0' .
# ⑤ 自造的胶囊样式：逐条确认都是徽章 / 链接，而不是「按钮自己写样式、自己切选中态」
#    —— 后者扫描器认不出（常量名与组件都没有），只能靠这份清单人工过（见下）
grep -rn --include='*.rs' --exclude-dir=ui 'rounded-full border' .
# ⑧ 页面级宽度类：输出应等于 §1.5 那份封闭清单（组件自身 / 页头对齐行 / 顶栏 container / 横幅 / 报文读宽）
grep -rn --include='*.rs' --exclude-dir=ui -E 'class="[^"]*\bmax-w-(2xl|3xl|4xl|5xl|6xl)\b|class="container' pages components app \
  | grep -vE 'min-w-|<Dialog|sm:max-w'
```

截至最后一次收口：①0 ②6 ③0 ④0 ⑤73 处（同一行带 `<button` 的为 0，全部是徽章 / 链接）。`button_class` 在页面侧只剩 8 条刻意的例外注释，
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

⑤ 那份清单里是**徽章与链接**（答题趋势的判定标签、错题诊断的提示、首页的更新提示、
`/gear` 对比表里的「依据不足」标签等）：它们渲染一次就定形，没有「点一下就换选中态」的
交互，不属于 chip。判据是「可点的互斥 / 开关按钮自己写样式」——`/gear` 的类别、机型、
预设三组曾这么写过（内联 `rounded-full border …` + `class=move || …`），已迁到
`ui::ChipGroup` + `ui::Chip`（互斥）/ `ui::ChipToggle`（可开关），顺带补上了
`role="radio"` + `aria-checked` / `aria-pressed`。新写选择器时先看上面第 ⑤ 条清单里
有没有 `<button`，有就说明又长出一只手写 chip。
