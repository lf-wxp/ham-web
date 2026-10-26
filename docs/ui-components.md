# 通用 UI 组件使用规范

> 写给**要改界面的人与 AI agent**。这个仓库的界面只有一套控件来源 ——
> `crates/app/src/ui/`。本文件是**写新代码时的规则与清单**；
> 组件清单、每个 prop 的含义、逐个使用示例 → [`crates/app/src/ui/README.md`](../crates/app/src/ui/README.md)。
> 两份文档的分工：**本文件管「怎么写」，ui/README.md 管「有什么」。**
>
> 从哪读起：新写 / 改页面看 §1 → §3（控件在 §3.1、内容块在 §3.2）→ §4；改外观看 §2；
> 遇到「这里为什么有原生 `<button>`」查 §10（例外清单）；收工前过 §12。
> 本文件只写**规范**：怎么写、为什么这么定、怎么验。评审过程与量化数字是历史，看 `git log`。

## 目录

1. 三条底线
2. 像素规范（视觉基线）
3. 决策表：要什么 → 用哪个
4. 页面骨架与内容宽度
5. 组件 API 约定
6. 无障碍（e2e 也依赖它）
7. 多语言
8. 常见坑
9. 新增 / 扩展组件
10. 例外清单：刻意保留的原生元素
11. 自检命令
12. 验收清单

## 1. 三条底线

1. **页面里不写原生表单元素**，也不写类名工厂（`input_class` 已删除；`button_class` 只允许
   `Button` / `ButtonLink` 内部与 §10 清单里的几处例外使用），更不新增本地 `const INPUT` / `CHIP_ON`。
   外观规则见 §2。
2. **标签关联、无障碍名、多语言文案**是组件与调用点**共同**的责任，缺一样都算没写完
   （e2e 也靠它们定位，见 §6）。
3. 改完**必须**过 `cargo make check`；动到页面组件再跑相关 e2e（见 §12）。

## 2. 像素规范（视觉基线）

整站是「16 位掌机 + 复古电台」风格。**外观只在 `crates/app/style/pixel/` 定义一次**，组件与页面只用
语义 token 与 `pxl-*` 控件类，不要在页面里再写圆角、模糊、渐变、软阴影。

| 文件 | 管什么 |
|---|---|
| `tokens.css` | 亮 / 暗两套调色板（映射到 `--background` / `--primary` / `--border` 等**既有语义名**）、零圆角、硬偏移阴影、字号阶梯、`steps()` 缓动 |
| `schemes.css` | 其余配色方案（森林 / 海洋 / 晚霞 / 石墨与 PICO-8 / Game Boy / 红白机 / C64 / CGA / ZX Spectrum）的亮 / 暗变量块；由 `cargo make pixel-schemes` 从 `crates/core/src/color_scheme.rs` 生成，**勿手改** |
| `palette.css` | Tailwind 默认色阶（`green-600` 等）整体换成像素调色板；由固定规则生成，**不要手调单个色值** |
| `fonts.css` / `fonts-pixel.css` | 字体声明；后者由 `cargo make fonts-pixel`（`crates/tools/src/pixel/fonts.rs`）生成，**勿手改** |
| `surfaces.css` | 全局基线、背景底纹、存量卡片兜底、滚动条、图标 / 精灵尺寸 |
| `controls.css` | `pxl-btn` / `pxl-field` / `pxl-window` / `pxl-titlebar` / `pxl-popover` / `pxl-overlay` / `pxl-check` / `pxl-radio` / `pxl-switch` / `pxl-chip` / `pxl-badge` / `pxl-item` / `pxl-bar` 等控件原语 |
| `motion.css` | 全部逐帧动画与「静态降级」 |

**硬规则**

1. **零圆角、零模糊、零渐变**：不要写 `rounded-*`（2026-10-13 已从视图里清干净 —— 它们
   在像素规范下全部失效，且一度被 `surfaces.css` 当卡片皮肤的钩子用；层外规则仍兜住第三方 /
   动态 HTML，自检 ⑤ 会拦住回归）、`backdrop-blur`、`bg-gradient-*`、带模糊半径的 `shadow-[…]`。
   需要强调就换色、加粗描边、用抖动底纹。真要一个圆（极少）给元素加 `data-keep-round`。
   （**例外**：无模糊半径的像素硬阴影是合规的，如 `shadow-[0_2px_0_0_var(--pxl-shadow)]`。）
2. **尺寸取 4 的倍数**：间距、控件高度走 Tailwind 刻度（`h-6` / `h-8` / `h-10` / `h-12`）；描边 2px，
   位移 2px 的整数倍。**不要写 `h-[37px]` 这类奇数**，点阵边缘会错位。
   **触屏密度**：`<sm`（≤639px）下 `pxl-chip` / `pxl-tab` 由 `controls.css` 统一抬到
   `min-height: 2.25rem`（36px）—— 24–26px 的 chip 在手机上一排六个必然点错。写版式时不用自己加
   `min-h-*`，类名已覆盖；要更小只允许在 `size` 档位里选（`Size::Xs` 等）。
3. **图标按 24px 的整数倍显示**（`size-6` / `size-12`）：像素图标是 24×24 网格，
   16 / 20px 会有半个像素的抗锯齿。`size-4` / `h-4 w-4` 在 `surfaces.css` 里被统一抬到 24px，
   新代码直接写 `size-6`（`check-ui` ㉓ 已拦 `h-3/3.5/4/5`）——**源码要等于渲染**，
   靠 CSS 隐式放大只会让后来者以为那里是 16px。
4. **字体**：正文走点阵中文体（缝合像素）；标题 / HUD / 数字读数用 `pxl-title`（Press Start 2P）
   或 `pxl-label`（Silkscreen）。**点阵字体没有粗体**：不要用 `font-bold` / `font-semibold` / `font-medium`
   做强调（会被合成成糊边；**易读字体模式下它们反而会生效，两种模式层级就不一致了**），
   改用颜色、`pxl-title` 或加框。`cargo make check-ui` 已拦（⑪）。
   **区段标题（卡片标题栏 / 页内分组标题）用 `pxl-section-title` / `pxl-section-head`**（见下）：
   它们自带 18px 字号与 Silkscreen，不必再写 `text-*`；标题的层级靠字号 / 字体 / 色带表达，
   因为 `font-semibold` 在这里不产生任何视觉差。
   **卡片内的标题用 `card_title_class(...)`**（`pxl-label` + 18px），不要写 `<h2 class="… text-sm">`
   ——12px 的标题比它管的 18px 正文还小，卡片就「一片平字」（`check-ui` ㉑ 已拦）。
   **页面标题只有一个来源**：`PageHeader`（`pxl-title` + 吸顶 + 主色左轨）。页面里不要手写
   `<h1 class="text-lg">`（字体族会退化成正文点阵，字号也不跟断点走）；纯无障碍标题写
   `sr-only`，确需页内 `<h1>` 时带 `pxl-title`（`check-ui` ⑳ 已拦，只放过 404 与打印页）。
5. **状态只信 ARIA**：选中 / 按下态由 `aria-checked` / `aria-pressed` / `data-state` 驱动样式，
   不要在 Rust 里再拼一份 `ON` / `OFF` 类名（`Chip` 就是这么做的）。
6. **颜色分「填充」与「文字」两套**：`--pxl-hp` / `xp` / `gold` / `win` 是画在深色槽里的**填充色**
   （血条、金币），直接拿来写字对比度不够（亮色下金色文字落在奶油底只有 1.4:1）。**写字一律用
   `text-hp-text` / `text-xp-text` / `text-gold-text` / `text-win-text`**；不要写
   `text-[color:var(--pxl-gold)]` 这类任意值。**不要再写 Tailwind 原始色阶**（`text-green-600`、`bg-emerald-500`、`dark:text-red-400`）：
   它们是 Sweetie-16 的固定色相，不随 11 套配色方案变，切到 Game Boy / CGA 就跳出方案；
   `cargo make check-ui` 已拦（⑯，只放过打印 / 导出画布）。状态一律用语义 token：
   对 / 通关 `win`、错 / 危险 `destructive`（文字用 `text-hp-text`）、警示 `gold`、信息 `xp`
   （文字 `text-win-text` / `text-hp-text` / `text-gold-text` / `text-xp-text`，底色 `bg-win/10`，
   描边 `border-win`），它们自己随明暗换色，**不用再写 `dark:` 对**。
   **红色文字不要写 `text-destructive`**：那是按钮填充色，暗色下落在较亮的弹层面上不足 4.5:1
   （axe 在 `rpg_flow` 的「交卷空卷（dark）」里抓到过），写字一律 `text-hp-text`
   （`check-ui` ⑲ 已拦）。
   **「身份色」（分类标识、图表序列）也不许从数据层带 `#rrggbb`**：知识库分类色曾经是
   `crates/core` 里的 10 个 hex，切到 Game Boy / CGA 时圆点还是玫瑰红、徽章还是「任意底色 + 白字」。
   现在数据层只给档位（`CategoryTone`），颜色在 `crates/app/src/tone.rs` 映射到
   `--cat-1..8`（第 9 / 10 档用「空心」第二通道区分）；`check-structure` 已拦
   `crates/core` 里的色值字面量。用法约束：分类色**只做图形锚点**（圆点 / 2px 描边 / 浅底），
   不铺整块文字底。
   **色相不够用时补方案种子，不要回退色阶**：11 套方案 + 16 色盘下，波段用途、雷达序列、
  分类标识这类「需要互相区分」的场景一定会撞色。现状是「语义 token 循环 + 第二通道（描边 /
  空心 / 图案）」；真要更多色相，就改 `crates/core/src/color_scheme.rs` 的种子（见下），
  而不是在页面里写 Tailwind 原始色阶。
  `pixel-palette` 仍保证共享色阶的文字档在真实表面上达标（给第三方 / 动态 HTML 兜底），
   改了调色板就跑它（`cargo make pixel-check` 只审计，不达标时退出码为 1，已并入 `check` / `ci`）。
7. **配色一律走语义 token，别在页面里写死色值**：`bg-card` / `text-muted-foreground` /
   `border-ink` 会随用户选的**配色方案**（11 套：经典 / 森林 / 海洋 / 晚霞 / 石墨 +
   PICO-8 / Game Boy / 红白机 / C64 / CGA / ZX Spectrum，与明暗正交）换色；
   写死 `#fff8e1` 的地方切方案后就成了孤岛（DOM 里没有这类写死的色值；`share_score.rs` 这类导出成绩
   图的画布色是有意固定的，不在此列）。状态色（`hp` / `xp` / `gold` / `win` / `destructive`）
   是语义色，不随方案变 —— 这正是状态要用它们、而不是原始色阶的原因（规则 6）。
   **实色填充上不要放白字**（`bg-win text-white` 只有 2.6:1）：改成「`bg-card` 底 + 同色描边 +
   `text-win-text`」（答题卡的 ✓ / ✗ 小牌就是这么做的）。
   **表面不要用 alpha 调浓淡**：`bg-muted/50` 这类写法叠在卡片上会随背后内容变色（等于绕过限色）。
   「比卡片低一层」的内凹面用 `bg-inset`（不透明 token）；热力 / 进度这类「同色不同浓淡」用
   `color-mix(in srgb, var(--cat-N) X%, var(--card))` —— 混的是两个调色板颜色、结果不透明。
   **新增 / 调整一个方案**：只改 `crates/core/src/color_scheme.rs` 里的 5 个种子色（底 / 墨 / 主色 /
   强调面 / 焦点环，亮暗各一套），然后 `cargo make pixel-schemes`。其余变量按规则派生，字色按它
   实际会落到的每个表面校正；单测会拦下「表面太亮 / 太暗」这类会让共享色阶不够对比度的种子色，
   `pixel-check` 再按真实表面审计整套 Tailwind 色阶。**别手改 `schemes.css`**。
8. **表面只分三级，一页里只让「当前任务块」用最重的一级**：L0 = 页底（`--background`），
   L1 = 平卡（`pxl-card border bg-card`，只有 2px 墨框，无影无高光；手写卡片与
   `flat_card_class()` 都属这一级），L2 = 窗口（`pxl-window` / `card_class()`，带内高光与
   4px 硬投影）。**一屏里所有块同重量 = 没有落点**：首页只有 Hero 与「快速开始」是 L2，
   今日任务、模块入口、装饰频谱都是 L1。目录 / 关键指标这类「一眼要看」的块才升到 L2。
9. **字号按「读」还是「扫」分，不按「长不长」分**：
   先记住字阶实际只有一串整数倍：**12 / 18 / 24 / 30 / 36…px**。`text-xs` 与 `text-sm` 同为 12px，
   `text-lg` 与 `text-xl` 同为 24px（`tokens.css`）—— 层级靠颜色与字体族，不靠 2px 的字号差；
   不要写 `text-[10px]` / `text-[11px]` 这类非 6 倍数的任意字号（点阵字体非整数倍缩放会发糊，
   `check-ui` ⑮ 已拦；SVG `<text>` 的字号是 viewBox 用户单位，不在此列）。
   - **主阅读对象**（题干、页面副标题这类必须逐字读的）用 body 默认的 18px（`text-base`）；
   - **扫读型内容**（解析正文、知识库要点 / 术语 / 步骤、表格、以及所有元信息）用 12px
     （`text-xs` / `text-sm` 同值）。判据是「一屏要能看几条」：390px 宽下 18px 会把一屏
     从 6 条压到 3 条，翻页成本高于单条舒适度。**2026-10-23 把解析正文与知识库 section
     放大到 18px，移动端实测后全部回退** —— 不要再按「长文就该更大」去统一放大；
   - **数字读数**（倒计时、比分、统计值）走 `pxl-label`（Silkscreen）或 `.pxl-stat`
     + `tabular-nums`，不要混在 12px 灰字里。
10. **动画只用 `motion.css` 里的**：位移 / 显隐 / 抖动 / 闪烁，缓动一律 `steps()`；
   **不要做缩放动画**（`scale-*` / `zoom-*` / 悬停旋转；中间帧会让点阵文字变糊，`check-ui` ⑫ 已拦）。
   **「上抬 / 下沉」只有一种实现：独立的 `translate` 属性**（`.pxl-btn` 与 `.motion-lift` 都是），
   `transform` 留给入场动画（`pxl-enter` / `reveal`）——两者各占一个属性才不会互相覆盖，
   测试读 `getComputedStyle(el).translate` 而不是矩阵。
   **焦点环是 2px 虚线**（全局 `:focus-visible` + `pxl-*` 控件自带），不要写 `ring-*` /
   `ring-offset-*` 顶替它（⑭）。悬停反馈是上抬 + 投影拉长，不用 `brightness()` 滤镜
   （会算出调色板之外的颜色）。新增动画必须同时写静态降级。
   `animate-in` / `animate-out`（tw-animate）的默认缓动是平滑 `ease`，`motion.css` 已在层外
   把它们收回 `steps(4, end)`；但**时长仍取 `--motion-*` 四档**（`motion-instant/fast/normal/slow`），
   写成 `duration-(--motion-normal)`；不要写 `duration-300` 这类毫秒数，也不要 `transition-all`
   （会动画布局属性）——写 `transition`（⑬）。

11. **答题选项的选中态用「整行」，不要动形状**：单选是切角八边形、多选是方形，形状是用来区分
   题型的；不要为了「更明显」去换组件或加圆角。选中态给整行浅底 + 左 2px 主色条
   （`components/question_card.rs` 的 `option_row_class`），并用等量负外边距抵消内边距，
   保证文字一格都不位移。判分后的对错配色是另一套（`pxl-chip` / 自绘选项），两套不要混。
   **判分时机由 `AnswerGate` 决定**（`ham_web_core::question`）：`show_answer` 只表示「允许显示答案」，
   不是「提前给答案」——没作答（也没主动展开）就不画答案行；多选题要显式「检查答案」
   （`QuestionCard.confirm_multiple`），勾第一个框不能把整套答案亮出来（练习页已开启）。
   判分结论给三个通道：行边色 + 行尾 ✓/✗（`common::OptionMark`）+ 常驻 `role="status"` 的文字结论。
   **「先选 → 看到答案 → 改成正确项」算不算偷看是统计语义**（影响错题本 / 连对 / 本周作答），
   改它要同时改 `question_card.rs` 与 `study.spec.ts`（「自己答对的才计」是既有契约），
   不要在界面层单独改。

**可用的像素元素**

- 窗口面板：`pxl-window`（卡片原语，自带 2px 墨色描边 + 硬投影）、`pxl-popover`（弹层）、
  `pxl-titlebar`（标题栏）。
- 区段标题：`pxl-section-title`（卡片内的标题栏：微底纹 + 2px 墨色下沿 + 左侧主色方块）、
  `pxl-section-head`（页内分组标题，同样式但不带底纹与下沿，用在卡片外的 `h2`）。
  **不要再手抄 `border-b px-4 py-3 text-sm font-semibold`** —— 2026-10-20 之前这样的手抄有
  216 处，`font-semibold` 在点阵字体下不生效，标题既没有字重、又比 18px 的正文更小。
- 手写卡片：`pxl-card` —— **只是标记，本身不带任何样式**。卡片的 2px 描边、以及「卡片内贴边
  表格去掉最外圈双线」由 `surfaces.css` 按这个标记给（2026-10-13 之前这些规则拿
  `:is(.rounded-xl, …)` 当钩子）。手写卡片写成 `pxl-card border bg-card`；新写卡片优先用
  `SectionCard` / `card_class()`（= `pxl-window`，自带描边与投影，但那组表格规则只认
  `pxl-card`）。
- 进度条：`Progress` + `class="pxl-bar-hp"`（血条，低于 25% 自动闪烁）/ `pxl-bar-xp`（经验）/
  `pxl-bar-win` / `pxl-bar-gold`。
- 精灵：`PixelSprite name="hero" scale=4`（`scale` 是**每格的屏幕像素数**，从接口上杜绝非整数倍）；
  成就徽章用 `badge_sprite(id)` 取精灵名。
- 动效类：`pxl-shake`（受击）、`pxl-flash`（闪白）、`pxl-bob`（待机）、`pxl-blink`、`pxl-float-up`（飘字）、
  `pxl-typewriter`（打字机，`--steps` 传字符数）；`pxl-enter` / `pxl-enter-fade` 是弹层与遮罩的入场
  （`Dialog` / `Popover` 内部用，页面不用手写）。

**新增像素资源**（图标 / 精灵 / 字体都是「源数据 → 生成物」，产物已入库，日常 / CI 不重跑，**勿手改**）：

- **图标 / 精灵**：源数据在 `crates/tools/src/pixel/sprite_art.rs`（`ICON_MAP` 把 `IconKind` 映射到
  pixelarticons 文件名，`None` 的手绘稿在 `HAND_ICONS`，角色 / Boss / 怪物 / 徽章在 `SPRITES`）；
  改完跑 `cargo make pixel-sprites`（需 `PIXELARTICONS_DIR`）重新生成
  `crates/app/src/icons/pixel/` 下的 `icon_paths.rs` / `sprite_data.rs`。新增 `IconKind` 漏了映射会
  直接编译失败（`name()` 与生成的 `icon_path()` 都是穷尽 `match`）。
- **字体**：跑 `cargo make fonts-pixel`（`FONT_SRC` 指向原始 TTF）重新生成
  `public/fonts/pixel-*.woff2` 与 `crates/app/style/pixel/fonts-pixel.css`。**只在新增大量中文文案、
  或换字体版本时才需要重跑**；语料外的生僻字由字体栈末尾的系统中文字体兜底，不影响使用。
  原始字体的下载地址与许可证见 `public/fonts/LICENSES`。

**两个显示偏好**（设置里，持久化在 `ui:pixelMotion` / `ui:readableFont`，落在 `<html>` 的 `data-*`）：
`data-pixel-motion="off"` 或系统「减少动态效果」时所有动画降为静态帧；
`data-readable-font="on"` 时正文切回抗锯齿的 Geist（标题 / HUD 仍是像素字体）。
**新增样式要在这两种状态下都看一眼**。

## 3. 决策表：要什么 → 用哪个

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
| 互斥选择（**整行宽度的列表**：侧栏、图表配套） | `SidebarItem` | 手写列表项 |
| 筛选开关（可以一个都不选） | `ChipToggle` | — |
| **视图切换**（选中项决定下面显示哪一块面板） | `Tabs` + `TabsList` / `TabsTrigger` / `TabsPanel` | 用 `ChipGroup` 冒充（语义是 radio，读屏不会按 tabpanel 播报）/ 手写 `role="tablist"` |
| 答题选项（点击即判分） | `QuizOption` | 手写 `<button class="…">` |
| 选择文件（按钮） | `FileInput` | `<input type="file" class="hidden">` + 标签按钮 |
| 选择文件（**整块拖放区**） | `FileDropZone` | 手写 `<label for>` + 隐藏 input |
| **执行动作**（提交、打开弹层、切换状态） | `Button` | `<button class=…>` |
| **跳转**（能中键新开、右键复制地址） | `ButtonLink` | `<a class=button_class(…)>` |
| 标签 + 控件 + 提示 / 错误 | `Field`（`hint` / `error` / `required`） | 手写 `<label>` + `<span>` |
| 进度、分隔线、统计块、标签文字 | `Progress` / `Separator` / `Stat` / `Label` | 手写 |
| 数据表格 | `Table` + `TableHead` / `TableBody` / `TableRow` / `TableHeader` / `TableCell` | 手写 `<table>`（2026-10-15 起收口） |
| 列表分页 | `Pagination` | 手写页码 / `.slice()` 截断 |
| 轻量操作反馈（成功 / 已复制 / 导入完成） | `ui::push_success` / `push_error`（全局 `ToastRoot`） | 原生 `alert`（已清零）/ 内联「成功」文字 |
| 需要用户确认的失败 | `MessageDialog` | Toast（Toast 自动消失，确认类信息会被错过） |
| **破坏性 / 不可逆操作**（删除、清空、合并、批量标记） | `ui::confirm(title, description, confirm_text, tone, on_confirm)`（全局 `ConfirmRoot` 挂在 `App`） | 原生 `window.confirm`（2026-10-23 已清零：不能本地化按钮、与像素主题割裂）/ 单击即生效 |
| 失败态（请求失败 / 解析失败） | `EmptyState` + 就地重试按钮 | 与「没有数据」共用一个空态（用户分不清是网断了还是本来没有） |
| 解码 / 长任务的「已用时」读数 | `DecodeElapsed`（配 `DecodeSession.elapsed_ms`） | 手写 `setInterval` / 只给一个转圈 |
| 悬停 / 聚焦才显示的提示 | `Tooltip` | 原生 `title=`；子元素不可聚焦时补 `tabindex="0"`，数值信息另写 `aria-label` |
| 移动端侧栏替代（分类 / 导航：右下角悬浮按钮 + 右侧抽屉） | `MobileNavDrawer` | 手写横滑分类条 / 手搭悬浮按钮 + 抽屉 |

五个易混对子的区别：

- `RadioGroup` 与 `ChipGroup` —— **密度不是语义**（都输出 `role="radiogroup"` / `role="radio"` +
  `aria-checked`）：选项带文字、和 `Field` 标签配套用前者；选项本身很短（`20m`、`三单元八木`）、
  横排一行用后者。侧栏那种「整行宽 + 左色点 + 右计数」的列表用 `SidebarItem`。
- `Button` 与 `ButtonLink` —— **元素语义**：动作与跳转不要混用（理由见 ui/README.md）。
- `Button` 与 `Chip` —— 单选语义是 `chip`（`aria-checked`）、独立开关是 `ChipToggle`
  （`aria-pressed`）；纯动作才是 `Button`。
- `FileInput` 与 `FileDropZone` —— **交互形态**：只要一个「选择文件」按钮用前者；上传解码类
  页面那种「整块区域可点击、可把文件拖进来」的用后者（`title` / `icon` / `hint` / `disabled`
  传不同参数即可，通用提示行「点击选择或拖拽」由组件内置；非音频上传位用 `choose_hint`
  覆盖该行的音频版文案）。
- `ChipGroup` 与 `Tabs` —— **筛选还是视图**：选中后**下面那块内容被过滤**（`20m`、`A 类`、
  `未寄出 / 全部`）用 `ChipGroup`（`role="radiogroup"`）；选中后**换了一整块内容**
  （Koch 训练 / 呼号抄收 / 发报练习这类互斥面板）用 `Tabs`（`role="tablist"`）。
  判错的代价是语义与键盘行为都错：把筛选器标成 tab，读屏会按 tabpanel 播报；
  把互斥视图留在 chip 里，则拿不到方向键换页与 `aria-controls` 的关联。
  一页里**并列且需要同时看**的分区两个都不要用：走 §4 的 `StickyPanel` 目录 + 纵向堆叠。

### 3.1 按钮的尺寸与变体速查

| 场景 | 用 |
|---|---|
| 表格行内 / 工具条的**纯文本**小按钮 | `Size::Xs`（`h-6`）+ `Variant::Outline` |
| 紧凑图标按钮（朗读、关闭、缩放、删除） | `Size::IconSm`（`size-6`）+ `Variant::Ghost` / `Outline`；**必填** `aria_label` |
| 句内的「编辑 / 删除 / 关闭」（透明底、悬停换色） | `Variant::Text` + `Size::Xs`；悬停变红补 `class="hover:text-hp-text"` |
| 工具栏筛选、卡片操作 | `Size::Sm`（`h-8`） |
| 表单主体、弹层主操作 | `Size::Default`（`h-10`，默认档） |
| 落地页主操作（首页 Hero） | `Size::Lg`（`h-12`）—— 表单与工具栏里不要用 |
| 折叠 / 下拉开关 | 上面任一形态 + `aria_expanded=Signal::derive(…)`（面板有 id 时配 `aria_controls`） |

尺寸档对应的高度（`h-6` / `h-8` / `h-10` / `h-12`）都是 4 的倍数（§2 硬规则 2）；
`Size::IconSm` 是 `size-6`，正好装下 24px 的像素图标（硬规则 3）。

### 3.2 页面内容块（`components/common/`）

上面是控件，这些是**内容块** —— 不在 `ui/` 里，但同样是「只有一个来源」：同一个版式在页面里
手抄第二份，就是下一次视觉走样的起点（知识库各页的要点 / 术语 / 表格 / 步骤已经收口到这四个）。

| 要什么 | 用 | 不要用 |
|---|---|---|
| 知识库正文的要点 / 术语 / 三列对照表 / 步骤 | `BulletSection` / `ConceptsSection` / `TableSection` / `StepsSection` | 手写 `<ul>` / 两列网格 / `<table>` |
| 页脚的依据溯源行（「依据 XX，YYYY-MM 核对」） | `ProvenanceNote` | 手写一行小字 |
| 空态（图标 + 标题 + 说明 + 操作） | 页面级：`EmptyState`；卡片内：`EmptyState plain=true` | 手写居中的提示块；卡片内再套带框版本（双线框） |
| 内容卡片（标题栏 + 正文） | `SectionCard`（`meta` 放标题栏右侧元信息，`href` 做整卡可点，见下） | 再抄一份 `pxl-card border bg-card`；整张手写 `<a class="block pxl-card …">`；`flat_card_class` + `CARD_HEADER` + `card_title_class` 那三件套（新代码） |
| 卡片里的区段标题 / 卡片外的分组标题 | `pxl-section-title` / `pxl-section-head` | 手抄 `border-b px-4 py-3 text-sm font-semibold` |
| 单按钮结果提示（成功 / 失败 / 已复制） | `MessageDialog` | 手搭 `Dialog` + 一个按钮 |
| 加载中（短等待）/ 首屏骨架 / 整页加载 | `Loading` / `Skeleton` / `PageLoading` | 手写 spinner / 一片留白 / 一行 `p-6` 文字（会脱出 `PageContainer`） |
| 解码 / 长任务的「已用时」读数 | `DecodeElapsed`（配 `DecodeSession.elapsed_ms`） | 手写 `setInterval` 或干脆不给反馈 |
| 图例条（色块 + 文字） | `Legend` | 手写色块 + 文字 |
| 题图（缩略图 + 点击放大） | `PreviewableImage` | 手搭 `Dialog` + `<img>` |
| 题目笔记编辑区 | `NoteEditor` | 手写 `Textarea` + 存取逻辑 |
| 答题进度头 / 底部固定操作栏 | `QuestionProgressHeader` / `BottomBar` | 手写进度行 / 固定底栏 |
| 限时考试的剩余时间读数 | `ExamClock`（档位由 `ham_web_core::exam_clock` 判定） | 手抄 `format_ms` + 自己定「最后 60 秒变红」 |
| 答题卡（抽屉 + 题号格子） | `AnswerCardSheet` / `AnswerCardCell` | 手写格子网格 |
| 答案解析（折叠 + 法规引用 + 朗读） | `ExplanationCard` | 手写折叠区 |

**内容卡片的外壳只有一份（`SectionCard`）**。首页下半部分曾同时存在三种外壳（窗口级
`card_class`、平卡 `flat_card_class`、整张手写的 `<a>`）：标题体、内边距、卡片间距、悬停反馈
各不相同，用户凭外观判断不出哪张能点。统一后：

| 场景 | 写法 | 外观 |
|---|---|---|
| 普通内容卡 | `<SectionCard title=… meta=…>正文（自带 p-4）</SectionCard>` | L1 平卡：2px 墨框、零投影；标题栏是 `pxl-section-title`（Silkscreen + 主色方块 + 墨色下沿）；标题是真正的 `h2` |
| 标题栏右侧的元信息（待复习数、连续天数、「查看详情 →」） | `meta=ViewFn::from(move \|\| …)` | 12px 灰字，放在 `h2` **外面**，不进标题的无障碍名 |
| **整卡可点** | `href="/x"`（标题成为铺满整卡的链接） | 额外带 `motion-lift`：悬停上抬 2px + 硬投影 + 淡强调底；键盘焦点是画在**整张卡**上的虚线框 |
| 一页里的「当前任务块」 | 不用 `SectionCard`，用 `card_class` / `pxl-window`（L2） | 内高光 + 4px 硬投影（§2 硬规则 8） |

约定：**悬停有反馈 = 整卡可点**，不可点的卡片不要加 `motion-lift`（悬停位移会干扰阅读）。
整卡可点的卡片里如果还有按钮 / 链接，给它们 `relative z-10` 浮到铺满的标题链接之上（否则点不到），
也不要把整张卡写成一个 `<a>`（链接里嵌按钮是无效嵌套）。卡片之间的间距由外层容器的 `space-y-6`
给，卡片自己**不带** `mb-*`（`card_class` / `flat_card_class` 带 `mb-[24px]`，混用会让间距时 24 时 48）。
手写悬停位移 / 带色软阴影（`hover:-translate-y-1`、`hover:shadow-lg`、`shadow-primary/10`）
`check-ui` ⑰ 已拦。

`EmptyState` 的两个档按**上下文**选，不按文案长短：页面级用默认档（自带 `pxl-card` 与入场），
卡片内的空列表用 `plain=true`（无边框、上下留白更小，否则卡片里套卡片）。三类**不要**迁：
打印区（`/print`、QSL 标签：边框与入场会印到纸上）、`<tbody>` 内（`div` 会破坏表格语义）、
「条幅式提示」（`spot_coverage`：左对齐一条，不是居中空态）。

考试 / 练习 / 导航三族组件（`components/exam/`、`components/practice/`、`components/navigation/`）
只在对应流程里用，不在此表：改它们直接读文件头注释（弹层的开合、`Portal`、焦点与滚动锁都写在里面）。

### 3.3 移动端侧栏替代（`MobileNavDrawer`）

应用外壳自己的移动端导航也只有一个来源：`nav_bar.rs` 的三段折叠（`MobileNavSection`，
原生 `<details>`，当前页所在段默认展开）与底部 `MobileTabs`（激活态 = 顶部 4px 主色条 + 浅底，
不只靠字色）。页面不要另写导航 / 底部条。

那个下拉面板**挂在 `<nav>` 上，不在顶栏那层 `container` 里**：`container` 在 400–639px 之间是
定宽居中（`max-w` 400）的，面板若以它为定位祖先，左右就会各空出一条、露出下面的页面；挪到
`<nav>` 下（`sticky` 即定位祖先）后铺满整宽，内容仍由面板内部的 `container` 与顶栏对齐。

排版在 `nav_bar.rs` 的网格上，顺序是**先导航、后设置**：

- `<sm`（手机竖屏）一列：三段折叠在上，虚线之下是「语言 / 外观与动效 / 数据备份」；
- `sm`–`xl`（横屏手机 / 平板 / 窄桌面）三列：三段各占一列，虚线之下是同一排工具。

单列铺到 640–1279px 会拉成一条通栏长列表（900px 下高约 370px，三列后约 146px）。面板里**不放
「首页」**：底部 `MobileTabs` 与顶栏 brand 都直达首页，再列一次只会把导航挤到第二屏；工具也没
必要放在最上面 —— 点开菜单第一眼看到的应该是导航。
**分隔虚线也画在那里**：一列时是三条（三段之间 + 工具区前一条）、三列时只有一条横跨整行，第一段上面不画；不要塞回
`MobileNavSection`（三列并排时每段自带的 `border-t` 会断成三段短虚线）。
工具区那一排三格必须「一条基线」：左缘与段标题图标齐平（同款 `px-3`），竖直方向互相居中
（`sm:self-center`）—— 少任何一条，三列并排时就出现「语言那格两行、右边一格一行」的高低错落。

- 两个按钮是 `h-auto … whitespace-normal`：`.pxl-btn` 默认 `nowrap` + 固定 `h-10`，西语
  「Apariencia y movimiento」在窄格里会溢出边框；它们各自包在 `px-3` 里（外层 `sm:contents`
  让包装在一列时是两格、多列时各占一格），按钮左缘必须和段标题图标齐平，否则偏出 12px。
- 「语言」与三个语言按钮同处一行（`px-2` 的按钮才放得下），标签 `hidden md:inline`：
  `sm` 那档每列只有 192px，塞不进一行；读屏名字由 `role=group` 的 `aria-label` 提供，
  可见标签 `aria-hidden`，不重复念。

侧栏在移动端被 `hidden md:block` / `hidden lg:block` 藏掉之后，分类 / 导航入口要用
`ui::MobileNavDrawer`（右下角悬浮按钮 + 右侧抽屉，复用 `Sheet`）补上。浏览 / 术语表 /
工具目录三个页面已收口到这里 —— 以后「小屏藏侧栏」的场景照这个来，**不要再写横滑分类条**，
也不要手搭「悬浮按钮 + 抽屉」。

```rust
let cat_open = RwSignal::new(false);
// 分类列表：桌面侧栏与移动端抽屉共用同一份
let category_tree = move || {
  view! {
    <SidebarItem active=… on_click=Callback::new(move |_| {
      select(ALL);
      cat_open.set(false);   // 选中后收起抽屉（桌面端点这一下无副作用）
    })>…</SidebarItem>
    {…其余列表项，点击回调同样带 `cat_open.set(false)`…}
  }
};
view! {
  <aside class="hidden md:block"><div class="sticky …">{category_tree()}</div></aside>
  <MobileNavDrawer
    open=cat_open
    title=Signal::derive(move || t("域.抽屉标题"))
    button_label=Signal::derive(move || t("域.按钮名"))
    icon=IconKind::SlidersHorizontal
    class="md:hidden"        // 与侧栏断点配套：md:block → md:hidden；lg:block → lg:hidden
  >
    {category_tree()}
  </MobileNavDrawer>
}
```

使用要点：

- **`open` 由调用方持有**：抽屉内容的点击回调里要主动 `open.set(false)` 收起；
- **`class` 传断点类**（`md:hidden` / `lg:hidden`），永远与侧栏的显示断点配套；
- `title` / `button_label` 用 `Signal::derive(move || t("…"))`；`button_label` 是悬浮按钮的
  无障碍名（图标按钮必填，§6）；
- **轻点打开、拖动移动**：位移 ≤ 6px 视为轻点（打开抽屉），超过则拖动改位置；位置收拢在
  视口内、松手写入 `localStorage["fab-position"]`（**全站共享**同一个位置）；旋转 / 换设备后
  由 resize 收拢回屏内。拖动判定与收拢在 `ham_web_core::fab`（纯计算，带单测）；
- **键盘可达**：Enter / Space 走 `Button` 的 click 通道，照常打开抽屉；
- 抽屉内容较长时 `Sheet` 自己会滚（`children` 外层已带 `overflow-y-auto`）。

已知取舍：抽屉永远从**右侧**滑出（复用 `Sheet`），按钮拖到左边也不会换侧；悬浮按钮是
像素风方形（§2 零圆角），不是系统风格的圆形 FAB；位置存的是绝对像素，不记忆「贴左 / 贴右」。

**入口文案用白话，隐喻只留在图标里**：移动端底部 Tab（`components/navigation/mobile_tabs.rs`）
与抽屉的分类名按「用户找得到」命名（首页 / 练习 / 考试 / 错题 / 更多），不要用「基地 / 地图 /
图鉴」这类关卡隐喻 —— 隐喻是氛围，不是导航。

## 4. 页面骨架与内容宽度

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
| 粘性页头（标题 + 副标题） | `PageHeader` | `actions=ViewFn::from(move || view! { … })` 放右侧操作区；吸顶偏移默认 `top-[66px]`（导航栏 64px 行高 + 2px 底描边，与导航下沿精确对齐，避免吸顶瞬间错位）；吸顶骨架类名集中在 `PAGE_HEADER_BAR`，刻意的例外（打印页）也复用同一常量 |
| 页内第二根吸顶栏（目录导航 / 侧栏） | `StickyPanel` | 吸顶偏移自动量「页头底边 + 面板与页头之间的原始间距」，页头折行 / 切语言 / 断点切换都跟随，零错位；面板内需要限高的滚动子元素用 `var(--sticky-top)` 计算剩余高度（如 `max-h-[calc(100vh-var(--sticky-top)-16px)]`） |
| 多行页头（标题行 + 工具条 / 确认条） | `PageHeader` 的 `children` | 子节点渲染在标题行下方；自己写一行 `mx-auto flex max-w-5xl …` 与标题行左边缘对齐 |
| 页面正文 | `PageContainer` | 默认 `space-y-6 px-4 py-5`；间距 / 内边距不同就用 `class` 覆盖；页面内锚点用 `id="…"` |
| 内容块滚动浮现 | `PageContainer` 的 `reveal=true` | 直接子元素进入视口时逐个浮现（同批 60ms 错峰）；`KnowledgePage` 已默认开启，普通内容页不必开 |
| 知识库正文页 | `KnowledgePage` | = `PageHeader` + `PageContainer`，并负责按需拉取知识库译文 |
| 打印类内容（`/print` 的纸、QSL 标签、QSL 设计器画布） | 物理尺寸（`max-w-[210mm]`、`@page size`、`mm` 值） | 量的是**纸**，不是内容宽度 —— **唯一**不受 `max-w-5xl` 约束的地方，理由见下 |

### 断点只有四档

Tailwind 的 `sm` / `md` / `lg` 之外，只多一档 `xs`（400px，定义在 `tokens.css` 的
`--breakpoint-xs`）。**不要在页面里写 `min-[560px]:` 这类任意值断点**（`check-ui` ㉒ 已拦）；
确实需要新的中间态，就加一条 `--breakpoint-*` 再全局用。

注意区分两种「方括号里的像素值」：

- `min-[…px]:` = 断点（上面这条管着）；
- `min-w-[560px]` = **表格最小宽度**，靠它触发横向滚动（全仓 87 处，各表按列数取，
  不受断点规则约束）。改窄屏布局时不要顺手把它们删掉 —— 删了表格会挤成一坨。

### 滚动模型

纵向滚动**只有一层**：窗口（`html` / `body`）。页面里不要造「整页高度 + `overflow-auto`」的
滚动容器，也不要用 `h-screen` 包住内容 —— 会出现双滚动条、吸顶错位、移动端地址栏收回时跳动。

**窗口滚动条是自绘的**（`ui::ScrollBar`，挂在 `App` 根部）：

- 原生窗口滚动条在 `surfaces.css` 里隐藏，**根元素要两条规则一起写**：
  `html { scrollbar-width: none }`（Chromium 121+ / Firefox 64+ 的标准属性，新版 Chromium 的
  根滚动条实际靠它关 —— 只留伪元素那条时 macOS 上仍会画出原生浮层条）+ 
  `html::-webkit-scrollbar { display: none }`（老 Chromium / Safari）。Firefox 另外在
  `@supports not selector(::-webkit-scrollbar)` 里补 `scrollbar-color` 与
  `* { scrollbar-width: auto }`。改画一条 12px 的像素滚动条，**从吸顶导航下沿开始**到视口底部
  （导航是「机身」、滚动条属于「屏幕」，不并排到头部去）。`top` 由组件实测 `[data-nav]` 的高度写入。
  `scrollbar-width` 规范上是**非继承**属性（MDN 明确，只有 `scrollbar-color` 才继承）：`html`
  为 `none` 时嵌套滚动区算出来仍是 `auto`，12px 像素滚动条照旧，所以嵌套滚动区不用在
  Chromium / Firefox 侧再兜。`* { scrollbar-width: auto }` 只是防御性兜底（显式钉回 `auto`，
  不依赖「非继承」这一实现细节），`*` 的特异性 (0,0,0) 低于 `html` 的 (0,0,1)，碰不到根元素
  —— 根一旦拿到 `auto`，原生窗口滚动条会回来和自绘那条叠成两条。
- **浮在内容之上，不占布局**：没有轨道底色、只有 thumb，`main` / `footer` **不要**留 12px
  内边距 —— 留了会在没有滚动条时也空出一条，而且滚动条出现 / 消失时内容会横向抖。
  代价是拖动区覆盖最右 12px（与系统覆盖式滚动条一致），所以**不要**在右边缘 12px 内放
  必须点的控件。**改这三处要一起看**：`ui/scrollbar.rs`、`surfaces.css` 的 `html` 段与
  `.pxl-scrollbar*` 段、`dialog/shared.rs::lock_scroll`。
- **不要再写 `scrollbar-gutter`**：原生窗口滚动条已不存在，没有槽位可言；弹层锁滚动时也**不需要**
  补 `padding-right`（隐藏前后布局宽度不变，页面不会横向跳）。
- 触屏（`hover: none`）整条隐藏 —— 那里没有常驻滚动条，手指滚动时多一条浮层只是噪声。
- **嵌套滚动区仍用原生自绘样式**（`*::-webkit-scrollbar`）：宽表格、弹层内容、吸顶侧栏。

**唯一允许的嵌套纵向滚动**是这几类局部区域：

| 区域 | 限高写法 | 必须带 |
|---|---|---|
| 弹层 / 抽屉的内容区（`Dialog` / `Sheet` / 答题卡） | `max-h-[calc(100svh-2rem)]` 这类，由组件自己给 | `overscroll-contain` |
| 吸顶侧栏里的目录 / 列表（`StickyPanel` 内） | `max-h-[calc(100svh-var(--sticky-top)-16px)]` | `overscroll-contain` |
| 下拉面板 / 搜索弹层的结果区 | `max-h-[60svh]` 之类 | `overscroll-contain` |

- **限高一律 `svh`，不要 `vh`**：`vh` 按「地址栏收起后」的大视口算，移动端上浮层会伸出屏幕，
  底部按钮点不到。`svh` 取小视口、且不随地址栏伸缩重排。
- **页面根不要写 `min-h-screen`**：`<body>` 已经是 `min-h-dvh flex flex-col`、`<main>` 是 `flex-1`。
  再写一层 `min-h-screen`（= `100vh`）在手机上必然多出一截可滚动的空白（现在统一写 `min-h-svh`）。
- **不做平滑滚动**：`html` 是 `scroll-behavior: auto`（这套语言的位移是逐帧的，平滑滚动会把
  `steps()` 的手感冲掉）。跳到锚点用 `util::scroll_to_element`（按吸顶页头的**实际底边**算偏移），
  不要手写 `scroll-mt-24` —— 页头高度随译文折行变化。
- **路由**：进入新页面回到顶部；**浏览器后退 / 前进保留原位置**（`app/main_content.rs` 用
  `popstate` 标记跳过复位）。改这段时注意「只改 hash」的导航不会触发复位，标记要能自己失效。
- **横向滚动**只出现在宽表格：`Table` 自带 `overflow-x-auto` + 调用点给 `min-w-[…px]`；
  不要在页面里再包一层 `overflow-x-auto`，也不要为了「不横滚」把列删掉（`mobile_overflow`
  守的是**整页**不横滚，表格内部横滚是预期行为）。

### 页内第二根吸顶栏（`StickyPanel`）

页头下方还要再吸一根「页内导航」（目录 / 分类侧栏）时用 `StickyPanel`
（`crates/app/src/components/common/`）。**不要手写 `sticky top-[…px]`**：页头高度随宽度
折行与译文长度变化，固定偏移要么在吸顶瞬间相对页头错位滑动，要么直接插进页头底下被盖住。
组件自动量「页头底边 + 面板与页头之间的原始间距」作为吸顶偏移，页头怎么变都跟随，吸顶前后
零错位。browse / glossary / tools 三个页面已收口，以后同类场景照抄：

```rust
// 桌面侧栏；移动端配套 MobileNavDrawer（见 §3.3）
<aside class="hidden md:block">
  <StickyPanel class="max-h-[calc(100vh-var(--sticky-top)-16px)] overflow-y-auto pxl-card border bg-card p-2">
    {category_tree()}
  </StickyPanel>
</aside>
```

使用要点：

- **`class` 只写面板自身的视觉与限高**（`pxl-card border bg-card p-*`、`max-h-*`、
  `overflow-y-auto`），吸顶位置不用写 —— 组件挂载后自动量好并写到内联 `top` 上；
- **面板内需要限高的滚动子元素**用 CSS 变量 `--sticky-top` 算剩余高度
  （`max-h-[calc(100vh-var(--sticky-top)-16px)]`，16px 是底部留白；面板自身限高时也一样）；
  组件已把该变量写在面板元素上；
- **断点配套**：`hidden md:block` / `hidden lg:block` 之类的显示断点与移动端
  `MobileNavDrawer` 配套。面板隐藏（`display:none`）期间组件不测量；窗口缩放跨断点后
  面板可见时会自动补量，不需要调用方处理；
- **切语言 / 页头折行**都会改变页头高度，组件用 `ResizeObserver` 盯住页头自动重算偏移；
- 间距是按面板的**实际自然位置**量的：面板上方 `PageContainer` 的内边距等会自动算进去，
  不用配置；
- 页面上**必须有 `PageHeader`**（组件按 `main header` 找它量高度）；找不到时面板不设置
  偏移、保持自然位置（不吸顶）。

原理与取舍：测量间距时把**页头和面板同时**临时置为 `position: static`，在同一自然坐标系
读取两者的边界并立即恢复样式；否则小屏隐藏侧栏、滚动后跨断点首次显示，会把滚动量缓存进间距。
之后按恢复吸顶后的页头底边加原始间距定位。`ResizeObserver`、窗口缩放和入场动画结束都会重算；
400ms 定时器只是无动画环境的兜底，不是保证首次测量发生在动画之后的屏障。

`Tooltip` 的描述通过唯一 `id` 与实际触发元素的 `aria-describedby` 关联；全站共用一个
`body` 气泡及根级委托监听，做上下翻转、视口边界钳制和内部滚动追踪，不受表格／热图裁剪。
格子不再各自分配 Portal、开合信号与事件监听，降低渲染成本和主 WASM 体积；数值仍保留可访问名称。
`ToastRoot` 的可见反馈同样传送到 `body`，层级高于模态遮罩；`Dialog` / `Sheet` 内有配套
live region。队尾入队不会重置当前消息计时，只有队首身份变化才开始下一条播放。

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

自检见 §11 的 ⑧（输出是一份**很短的封闭清单**，当前 **11 行 / 9 个文件**，逐条对得上即可；
出现新行就说明又手抄了一份容器）。

这份清单现在是**机械守着**的（`cargo make check-ui`）：清单外出现 `max-w-*` / `container` 会红，
清单内的**数量漂移**（期望 11 处 / 9 文件）也会红 —— 合法新增要同时改上面这张表与
`crates/tools/src/ui_check.rs` 里的 `WIDTH_ALLOWED`：

| 位置 | 为什么允许 |
|---|---|
| `components/common/page_header.rs` | 页头宽度就在这里定义（`page_container.rs` 用 `cn(&[…])` 拼类名，不出现在这条 grep 里） |
| `app/footer.rs`、`components/common/bottom_bar.rs` | 外壳与正文左边缘对齐 |
| `pages/mistakes/mistakes_page.rs`、`pages/bookmarks/bookmarks_page.rs`、`pages/qsl_labels/qsl_labels_page.rs`（2 处：筛选工具条 + 未设置呼号提示行）、`pages/daily_challenge.rs`（吸顶计时条） | `PageHeader` 的 children 行，与标题行对齐 |
| `components/navigation/nav_bar.rs`（2 处 `container`） | 顶栏例外，理由见上 |
| `app/storage_warning.rs` | 全站横幅，复刻页头行的宽度 |

不在范围内的：弹窗宽度（`Dialog` 的 `sm:max-w-*`）是另一套尺度；表单控件自身的宽度上限
（`Field` / `NativeSelect` 的 `class="sm:max-w-xs"` 一类，避免下拉在宽屏上铺满整行）量的是
**控件**，不是内容宽度，同样不受本节约束，§11 的 ⑧ 也特意放过 `sm:max-w`；
`pages/morse/morse_trainer.rs` 报文行的读宽上限写在 `class = if … { "…" }` 的字符串里，
同样与页面宽度无关。内容件自身的量宽（`max-w-sm` / `max-w-md`，如结果列表、预览图、图表容器）
也是内容宽度：⑧ 的正则只扫 `2xl` 及以上，别把它们的 `mx-auto` 当成页面容器去「统一」。

**写新页面的顺序**：`PageHeader` → `PageContainer` → 内容块用现成组件（`SectionCard` 等）
→ 跑 `cargo make check`。

## 5. 组件 API 约定

- **受控值**：`value` / `checked` / `active` 都是 `Signal<T>`（标了 `#[prop(into)]`），
  配套 `on_change: Callback<T>`（回调收的是**新值**，不是事件）。
  传值三种写法：`RwSignal` 直接传、`Signal::derive(move || …)`、需要复用时 `Memo`。
  属性里**不要**写裸闭包 —— 除非该 prop 声明的就是闭包（`label=` 等 `TextValue`
  收的是 `Signal<String>`，`trigger=` 收的是 `ViewFn`）。
- **文案**：`TextValue` 收 `&str` / `String` / `Signal<String>`；要跟随语言切换就必须
  `Signal::derive(move || t("key"))`（`TextValue` **不接受闭包**）。
- **尺寸与宽度**：高度交给 `size`（`ControlSize::{Sm,Default,Lg}`，按钮见 §3.1），宽度 / 字体等
  修饰写在 `class`。`class` 是 `String`，要**静态字面量**（Tailwind 只扫描完整类名）；
  宽度随状态变时，在闭包里先算好 `String` 再赋给 `class`（见 `pages/morse/morse_trainer.rs`）。
  需要「随状态重算 class」的控件用不了 `Button`，那是 §10.2 的例外。
- **外层容器**：`Input`（带 `prefix` / `suffix` / `clearable` 时）与 `NativeSelect` 外面会多包一层
  `relative` 容器用于绝对定位；`flex-1` / `min-w-*` 这类要作用在**外层 flex 子项**上的类写
  `wrapper_class`，写进 `class` 只作用于控件本身。
- **表单行的 id**：`Field` 负责 `label` 与 `r#for`，控件要有同一个 `id`，值用
  `util::unique_id("…")` 生成（§8 有陷阱）。
- **提交**：`Button` 默认输出 `type="button"`，**要提交表单必须写 `kind=ButtonKind::Submit`**
  （这是刻意的：原先手写 `<button>` 不带 `type`，在 `<form>` 里会意外提交）。
  异步提交用 `loading=Signal`：自动禁用 + 置 `aria-busy`，文案保留。
- **错误态**：`Input` / `NumberField` / `Textarea` / `NativeSelect` / `Select` / `DatePicker` /
  `TimePicker` / `Switch` 都有 `invalid=Signal<bool>`，会加 `aria-invalid` 并把描边转成
  `destructive`；不要再手写红色描边。

## 6. 无障碍（e2e 也依赖它）

- **有可见标签**：用 `Field`（`r#for` + 控件 `id` 配对），或把控件包在 `<label>` 里。
- **没有可见标签**：必须给 `aria_label=Signal::derive(move || t("…"))`（图标按钮 / 图标链接
  **必填**，例如 `Button variant=Variant::Ghost size=Size::IconSm aria_label=…`）。
- **互斥 / 多选**：用组件的 `RadioGroup` / `ChipGroup` / `Checkbox` / `Switch`，
  不要自己写 `role=`、`aria-checked`、`aria-pressed`。`ChipGroup` 的 `Chip`（`role="radio"`）
  支持**方向键移动选择**（选中随焦点，WAI-ARIA 的 radiogroup 交互）；刻意**没做** roving
  tabindex —— 每个 chip 仍在 Tab 序里，读屏按 Tab 逐项经过也能用。
- **视图切换**：用 `Tabs`，不要自己写 `role="tablist"` / `aria-selected`。`TabsTrigger` 按
  WAI-ARIA tabs 交互：`←` / `→`（`↑` / `↓`）环形移动**并顺带激活**，`Home` / `End` 跳首尾；
  组内是 roving tabindex（只有激活项在 Tab 序里）—— 与 `ChipGroup` 刻意不做 roving 的选择
  不同：tabs 下面挂着一整块面板，方向键是「换页」，逐项 Tab 过去反而会逐个经过看不见的内容。
  面板常驻挂载、非激活时 `hidden`：`aria-controls` / `aria-labelledby` 靠组件生成的 id 配对，
  **页面里不要手写这对 id**。
- **折叠 / 下拉**：`Button` 的 `aria_expanded` / `aria_controls`（面板有 id 时一并给）。
- **一组按钮里的「当前」**：`Button` 的 `aria_current`（答题卡当前题、分页当前页），
  不要用自定义 class 表达；`aria-current` 只在一处为真。
- **动态提示**：实时结果区用 `aria-live`，并用 `Input` 的 `aria_describedby` 关联 id。
  判分 / 倒计时这类「用户刚做完一件事」的结论放**常驻**的 `role="status"` 容器（节点先于内容存在，
  部分读屏器不播报「插入时就带内容」的区域）；每秒变化的读数（倒计时）**不要**加 `aria-live`，
  否则是持续噪声 —— 跨档提醒交给 Toast。
- **模态与快捷键**：任何 window 级键盘监听都要先 `shortcuts::modal_is_open()`
  （判 `[aria-modal="true"]`），否则弹层开着时数字键 / 方向键会改到背后的内容。
  `Dialog` / `Sheet` 关掉时有 200–240ms 退场，这段时间快捷键仍被挡住（可接受）。
- **限时提示**：Toast 悬停 / 聚焦暂停计时（WCAG 2.2.1），离开后按剩余时间续（不短于 2 秒）；
  同类同文合并为 `×N`。页面只调 `push_*`，需要用户确认的失败仍用 `MessageDialog`。
- Playwright 用例大量使用 `getByLabel(...)` / `getByRole(...)`：**标签关联与 role 是测试契约**。
  改了 `role` / `aria-*` 必须同步 spec（§12）。

## 7. 多语言

- 界面文案一律 `t("域.词条")`，**字面量写在调用点**（`check-i18n` 只采集直接实参；
  经变量/函数转手会被判成死条目或漏统计）。
- 新增词条：`data/i18n/{zh,en,es}/<域>.json` **三份都加**（key 前缀与文件名域一致，
  三种语言的 key 集合必须相同）→ `cargo make i18n-pack` 重新生成运行时语言包 →
  `cargo make i18n-check`。
- **中文值不能重复**（两个 key 用同一句中文会被判「重复」）；确实要复用时改成引用同一个 key。
- 三个语言文件的**行数**不同属于正常（en / es 的复数变体会多出 `one` / `other` 行），但按 key 归并后
  **key 集合必须一致**（`check-i18n` 强制）。
- 代码里**不要硬编码**界面中文（如 `title="切换到深色模式"`）——`check-i18n` 抓不到它，
  但 en / es 界面上会直接露出中文。发现这类残留请顺手补词条。

## 8. 常见坑

1. **`Field` 的 id 与闭包**：把 `unique_id()` 生成的 id 声明在组件体里、再在
   `{move || …}` 里用（哪怕 `.clone()`），闭包会退化成 `FnOnce` → 编译失败。
   id 必须与使用点**同作用域**：顶层 `view!` 用 → 声明在组件体；在 `move ||` 里用 →
   声明在那个闭包里；也可以用 `view!` 的 `{{ let id = …; view!{ … } }}` 块。
   同理，`Callback` 里按值消耗的 `String`（如每行一份的 key）要在**每次重渲染的闭包开头**
   `.clone()`，否则外层 `move ||` 也会退化成 `FnOnce`。
2. **Tailwind 只认完整字面量类名**，不要 `format!("{} {}", "px", n)` 拼片段。
3. **依赖 context 的子组件**（`RadioGroupItem` / `SelectItem` / `Chip`）必须在父组件的
   `children` 里就地构建，不能提前 `collect_view()` 到外面再传进去。
4. **动态 `variant` / 动态 `class` 不支持**：`Button` / `ButtonLink` 的 `variant` 是静态 prop，
   `class` 是静态 `String`（不能随状态重算）。因此「选中态要换 `variant`」或「底色 / 强调色
   要随状态变」的地方**保留原生元素 + `button_class`** —— 见 §10.2 清单，每处都写了
   `刻意的例外` 注释（`grep -rn "刻意的例外" crates/app/src` 一次列全）。
   **不要**为了「统一」把这些清掉（会丢状态反馈）；真要收编，先给 `Button` 补通道（见 §9），
   再改调用点。（`aria-expanded` / `aria-controls` / `aria-label` 已经有通道，不再是保留理由。）
5. **`Input.on_change` 是「每次输入」**：需要「失焦 / 回车才提交」语义时用 `on_enter`，
   或把它放进 `<form on:submit>` 用提交按钮。
6. **`Slider` 收 `f64`**：`f32` 信号要显式转换（`Signal::derive(move || f64::from(x.get()))`
   + 回调里 `as f32`）。
7. **`NumberField` / `Input` 的 `value` 是 `String`**（保留用户输入的中间态如 `-`、`1.`），
   不要在组件外把它 parse 成数字再喂回去。
8. **筛选器的「一个都不选」**：不要用 `ChipGroup`（它是单选，必有选中项），用 `ChipToggle`。
9. **`Chip` / `ChipToggle` 的 children 要包一层 `view!`**：`ChipGroup` 的 children 里，
   每个 `Chip` 的 children 写成 `{view! { {move || …} }.into_any()}` —— 直接把
   `{move || …}` 放在外层会让响应式文本失去响应式上下文。
10. **别让动作「看起来能点、点了没反应」**：不可用的动作把 `disabled` 传下去（`Button`
    会如实置灰，读屏也读得到），不要在回调里 `if !allowed { return; }` 静默吞掉
    （机考仿真的「上一题」、答题卡的跳转都踩过）。
11. **`ui_check` 只看 `.rs` 里的类名字面量**：写在 `.css` 或内联 `style="…"` 里的同类问题
    在射程外（曾经把一条 `border-right` 声明并成非法值、箭头直接消失，门禁全绿）。
    改 `.css` / `style=` 时人眼过一遍 `+` 侧。

## 9. 新增 / 扩展组件

- **先扩展现有组件**：缺通道（例如 `title`、`maxlength`、`aria_expanded`）时，优先给现有组件
  补 prop，而不是在页面里退回原生元素。补完在 `ui/README.md` 组件表里补一行。
  **「只是颜色淡一点 / 小一号 / 更紧凑」也一样**：先补 `Size` 档（`Xs` / `IconSm`）或
  `Variant`（`Text` / `Ghost`），不要在页面里复制一份 `class`——复制出去的那份不会跟着
  `pxl-btn` 的悬停 / 按下 / 禁用态一起变。
- **什么时候才新增组件**：同一个交互在 ≥3 处重复，且现有组件 + prop 表达不了。
- **放哪**：基础控件放 `crates/app/src/ui/<name>.rs`（在 `ui/mod.rs` 里 `mod` + `pub use`，
  并在 `ui/README.md` 组件表登记）；业务组件放 `crates/app/src/components/<name>.rs`
  （在 `components/mod.rs` 里 `pub mod`）。两者都遵守**一个组件一个文件**
  （对业务组件同样适用，见 `AGENTS.md`）。
- **一族多组件用文件夹模块**（`Dialog` + 它的 5 个部件、`RadioGroup` + `RadioGroupItem`、
  `Select` + `SelectItem`、`ChipGroup` + `Chip` + `ChipToggle` 都是这么放的）：
  `xxx/mod.rs` **只放**模块文档 + `mod` 声明 + `pub use` 重导出，每个组件一个文件，
  共享的样式 token / context 放 `xxx/shared.rs`。文件名不能与父目录同名
  （`chip/chip.rs` 会被 Rust 拒绝），照 `dialog_root.rs` / `select_root.rs` / `chip_item.rs` 起名。
- **约定**：`#[prop(optional, into)]` 标可选；`data-slot="<组件名>"`；
  复用 `control_class` / `button_class` / `badge_class` 等底座，**不要另起一套样式**；
  文案类 prop 用 `TextValue`；回调用 `Callback<T>`；颜色只用 Token
  （`bg-card` / `text-muted-foreground` / `ring-ring` …），不写死 hex。

## 10. 例外清单：刻意保留的原生元素

规范之外**有意留下**的地方都在这里。每一处都在代码里写了 `刻意的例外` 注释，
`grep -rn "刻意的例外" crates/app/src` 可列全（当前 **11 条注释 / 10 个文件**，
其中 `nav_bar.rs` 占 2 条、`group_menu.rs` 占 1 条；`pagination.rs` 那条只是解释
「为什么不收录」，不是例外本身）。大多数走 `button_class` + 原生元素，另有几处用
`cn` / 原生类名（`nav_bar.rs` 的移动端语言切换、`dx_spots.rs` 的表格行按钮）、共用吸顶骨架常量
（`page_header.rs`、`print_page.rs`）或保留整表原生 `<table>`（`bands_page.rs`）—— 各有理由见下表；
**新写界面不要模仿它们**，它们各自的理由如下。

确认类弹窗不在例外之列：破坏性操作的二次确认已全部收口到 `ui::confirm`（全局 `ConfirmRoot`），
`window.confirm` 在 2026-10-23 清零；「删了就撤销」没做 —— Toast 目前是纯数据（`PartialEq`），
要撤销得给它加携带回调的动作通道，成本高于「确认一次」。

### 10.1 表单控件：拖放上传区（已全部收编）

整块拖放目标（解码类页面 + 图片处理页）一律走 `ui::FileDropZone`：组件内部就是 `<label for>` +
`sr-only` 的 `<input type="file">`（`display:none` 不可聚焦，键盘用户就没法选文件），焦点环由
label 的 `has-[:focus-visible]:ring-2` 显示。**页面里不要再手写拖放区**；只要按钮的上传用
`FileInput`。

非音频的上传位记得覆盖那一行「点击选择或拖拽…」（`choose_hint`）：组件默认是音频版
（`tools.click-to-choose-or-2`），图片上传位传 `tools.click-to-choose-or`。

### 10.2 `button_class` + 原生元素

| 位置 | 为什么保留原生元素 |
| --- | --- |
| `components/navigation/nav_bar.rs` | ① 导航项（`<a>`）：高亮要切 `variant`（`ButtonLink.variant` 是静态 prop）；② 移动端面板的语言切换（`<button>`×N）：选中态要重算 `class`（`Button.class` 是静态串，顺带输出 `aria-pressed`）。分组下拉开关已拆到 `group_menu.rs`，见下一行 |
| `components/navigation/group_menu.rs` | 「考试中心」「知识库」「工具」三个下拉开关（三个菜单共用一个组件，同一条注释，渲染出 3 个 `<button>`）：理由同 nav_bar；面板是分组侧栏 + 条目区，见文件头 |
| `pages/callsign_copy.rs` | 两个模式按钮：选中态切 `variant` |
| `components/exam/answer_card_sheet.rs` | 答题卡筛选按钮：当前筛选切 `variant` |
| `pages/home/cards_card.rs` | 「今日待复习」入口：有待复习时切 `variant` |
| `pages/dx_spots.rs` | 热点表格行内的紧凑内容按钮（呼号 + 国家两行）：像素按钮的高度与内边距会把行高撑开、破坏表格密度 |
| `components/common/page_header.rs` | 吸顶骨架 `PAGE_HEADER_BAR` 常量：全站吸顶页头只有这一份骨架类名，打印页等「纸宽坐标系」的页头也复用它，吸顶偏移只在这一处改（见文件头注释与 §4） |
| `pages/print_sheet/print_page.rs` | 打印页量的是纸宽（`max-w-[210mm]`），不套 `PageHeader`；吸顶骨架与全站共用 `PAGE_HEADER_BAR`，偏移保持单一来源 |
| `pages/bands/bands_page.rs` | 波段表的 tbody 有大量动态 `rowspan` / `colspan`（按波段跨行合并单元格），`TableCell` 的静态 `colspan: u32` 表达不了，整表保留原生 `<table>` |
| `ui/pagination.rs` | 页码按钮的「当前页高亮」靠 `aria-current` 属性动态切换，`Button` 的 `class` 是静态串、也没有 `aria_current` 通道；分页条住在 `ui/` 里，样式与 `Button` 同源（`button_class`） |

它们连同 `Button` / `ButtonLink` 内部都继续用 `button_class` —— **`ui::button_class` 是这套
组件的底座，不要删**。

### 10.3 保留自绘的其他控件

| 类别 | 例子 | 为什么不迁 |
|---|---|---|
| 多选 / 带对错态的答题选项 | `mistakes/*` 的选项、`components/topic_quiz/quiz_question.rs` | 多选 + 判分后的对错配色，单选款的 `QuizOption` 表达不了 |
| 卡片点击区、热图格、地图区域 | `pages/cards.rs` 卡片、`morse_card` / `phonetic_card`、`muf` 热图格、`zone_map` 地图 | 点的是「内容」不是按钮，`Button` 的尺寸与视觉都表达不了 |
| morse 大圆键 | `pages/morse/morse_key.rs` 的 `MorseKey`（`koch_trainer` / `morse_trainer` 的试听键、`send_trainer` 的长按发报键共用） | 要 `pointerdown` / `pointerup` / `pointerleave` 一组事件，`Button` 没有这条通道。外观是 `pxl-btn pxl-btn-primary`，「播放中 / 按住中」由 `data-state="active"` 驱动按下态，不做缩放（原先的 `animate-ping` 扩散圈也是缩放，已换成内框逐帧闪烁） |
| 二级子分类项 | `pages/browse/browse_page.rs` 的 `ml-4` 缩进项 | 刻意更轻的选中态（`bg-accent`，不是主色填充），与一级项的重量分级 |
| 导航胶囊 / 快捷入口 | `pages/tools/tools_page.rs` 的工具条、`mistakes` 的薄弱分类 | 是「跳到某处」，没有选中态 |
| chip 旁带第二个按钮 | `pages/bookmarks/bookmarks_page.rs` 的分组筛选（每个 chip 带「×」删除） | `Chip` 内不能嵌套按钮 |
| 句内文字链接按钮 | `pages/practice/view.rs` 空态的「练习全部题目」 | 是句内下划线样式，不是按钮 |
| 药丸操作按钮 | `pages/morse/pill_button.rs`（「重播 / 跳过 / 结束本轮 / 显示答案」四处） | 扁平药丸（透明底 + 1px 边框，无 `pxl-btn` 的凸起描边与硬投影）：这几个是练习页里的次要动作，凸起按钮会抢走中央大圆键的注意力。四处逐字相同，已由该业务组件收敛 |

### 10.4 相关的历史说明

- **⑤ 类「自造的胶囊样式」**（§11 自检 ⑤ 的清单）是**徽章与链接**（答题趋势的判定标签、
  错题诊断的提示、首页的更新提示、`/gear` 对比表里的「依据不足」标签等）：它们渲染一次就定形，
  没有「点一下就换选中态」的交互，不属于 chip。判据是「**可点的互斥 / 开关按钮自己写样式**」——
  `/gear` 的类别、机型、预设三组曾这么写过，已迁到 `ui::ChipGroup` + `ui::Chip`（互斥）/
  `ui::ChipToggle`（可开关）。新写选择器时先看 ⑤ 的清单里有没有 `<button`，
  有就说明又长出一只手写 chip。
- **迁移历史**（哪一轮迁了什么、e2e 契约怎么变的）看 `git log --oneline -- crates/app/src/ui`；
  本章只留判断依据，不记流水账。

## 11. 自检命令

**机械可判定的条目已经接进 CI**：`cargo make check-ui`（含在 `cargo make check` 与 `cargo make ci`
里，源码见 `crates/tools/src/ui_check.rs`）会扫 ①②③④⑤⑦、⑥ 的模糊 / 渐变与**任意值阴影的模糊位**、
⑧ 的**页面级宽度容器封闭清单**（清单外命中、清单内计数漂移都会红），以及 2026-10-24 补上的 ⑪–㉓
（字重 / 缩放 / `transition-all`+`duration-N` / 实线焦点环 / 非 6 倍数字号 / 原始色阶 / 手写悬停位移 /
空属性 / `text-destructive` / 手写 `<h1>` / 12px 卡片标题 / 任意值断点 / 16px 图标），命中即让门禁红。
下面保留手工命令是为了看清判定细节，以及覆盖工具刻意不做的部分：内联 `style` 里的阴影、
打印页的 `mm` 尺寸、⑨ 的例外注释计数。

```bash
cd crates/app/src
# ① 本地控件样式常量：应为 0（check-ui 已拦）
grep -rn --include='*.rs' --exclude-dir=ui -E '^const (INPUT|TEXTAREA): ' .
# ② 原生表单元素：应为 0（拖放区在 ui::FileDropZone 里，见 §10.1；check-ui 已拦）
grep -rn --include='*.rs' --exclude-dir=ui -E '<input|<select|<textarea' .
# ③ 类名工厂：input_class 应为 0（工厂已删；check-ui 已拦）
grep -rn --include='*.rs' --exclude-dir=ui -c 'input_class(' . | grep -v ':0$'
# ④ 手写 chip 的样式常量：应为 0（check-ui 已拦）
grep -rln --include='*.rs' --exclude-dir=ui 'CHIP_ON' .
# ⑤ 已失效的圆角类：应为 0（卡片皮肤改用 `pxl-card` 标记，见 §2；check-ui 已拦）
#    手工命令的两处放过：`cn.rs`（类名合并器，字符串里出现工具类名是它的本职工作）、
#    行首注释里的历史说明（`quiz_option.rs` / `pill_button.rs` 各一条）；
#    工具版更严：不排除 ui/、且按行剥注释后只在字符串字面量里判定
grep -rnE --include='*.rs' --exclude-dir=ui --exclude=cn.rs \
  '\brounded-(full|none|sm|md|lg|xl|2xl|3xl|t|r|b|l)(\b|-)' . | grep -vE ':[0-9]+: *//'
# ⑥ 页面里残留的模糊 / 渐变 / 软阴影：模糊（含 Tailwind v4 的 `blur-xs` 与裸 `blur`）、
#    渐变与「任意值阴影的模糊位」已由 check-ui 拦；这条手工命令用于复核
#    （输出里 `shadow-[0_Npx_0_0_…]` 是像素硬阴影，合规；`cn.rs` 里的
#    `("backdrop-blur", …)` 是 cn 的冲突组配置，不是 class，故排除）
grep -rnE --include='*.rs' --exclude-dir=ui --exclude=cn.rs 'blur|drop-shadow-|bg-gradient|bg-linear|shadow-\[0' .
# ⑦ 把填充色 token 当文字色用（对比度不够）：应为 0
grep -rnE --include='*.rs' 'text-\[color:var\(--pxl-(hp|xp|gold|win)\)\]' .
# ⑧ 页面级宽度类：输出应等于 §4 那份封闭清单（11 行 / 9 文件：组件自身 / 页头对齐行 / 顶栏 container / 横幅）
#    check-ui 已按同一份清单机械守着（清单外命中、清单内计数漂移都会红），这条用来复核
grep -rn --include='*.rs' --exclude-dir=ui -E 'class="[^"]*\bmax-w-(2xl|3xl|4xl|5xl|6xl)\b|class="container' pages components app \
  | grep -vE '<Dialog|sm:max-w'
# ⑨ 刻意保留的原生元素：应等于 §10.2 的清单（11 条注释 / 10 个文件）
grep -rn "刻意的例外" .
# ⑪ 字重类：应为 0（check-ui 已拦）
grep -rnE --include='*.rs' '\bfont-(semibold|bold|medium|extrabold)\b' . | grep -v 'cn.rs'
# ⑫ 缩放 / 悬停旋转：应为 0（check-ui 已拦）
grep -rnE --include='*.rs' '(^|[ "])([a-z-]+:)*(-?scale-|zoom-(in|out)-|hover:rotate-)' . | grep -v 'cn.rs'
# ⑬ transition-all 与写死毫秒的 duration：应为 0（check-ui 已拦；`duration-(--motion-*)` 合规）
grep -rnE --include='*.rs' 'transition-all|\bduration-[0-9]+' . | grep -v 'cn.rs'
# ⑭ 实线焦点环：应为 0（check-ui 已拦；`focus-visible:outline-ring` 是虚线的颜色，不算）
grep -rnE --include='*.rs' '(^|[ "])([a-z-]+:)*ring(-offset-[a-z0-9]+|-[0-9]+|-ring(/[0-9]+)?)?([ "]|$)' . | grep -v 'cn.rs'
# ⑮ 非 6 倍数的任意像素字号：应为 0（check-ui 已拦；放过 nec/nvis.rs 的 SVG <text> 与 print_sheet）
grep -rnE --include='*.rs' 'text-\[[0-9.]+px\]' . | grep -v 'cn.rs\|nvis.rs\|print_sheet'
# ⑯ Tailwind 原始色阶：应为 0（check-ui 已拦；放过 print_sheet / qsl_labels / qsl_designer 与 print: 变体）
grep -rnE --include='*.rs' '(bg|text|border|ring|fill|stroke|from|to|via|divide|outline)-(red|green|emerald|amber|blue|yellow|orange|sky|rose|purple|indigo|teal|violet|gray|zinc|slate)-[0-9]{2,3}' . | grep -v 'cn.rs\|print_sheet\|qsl_\|print:'
# ⑰ 手写的悬停位移 / 悬停阴影 / 带色阴影：应为 0（check-ui 已拦；居中的 `-translate-y-1/2` 与 2px 的 `-translate-y-0.5` 不算）
grep -rnE --include='*.rs' '(hover:|active:)-?translate-y-[1-9]|hover:shadow-|shadow-(primary|accent|win|hp|xp|gold|destructive)/' . | grep -v 'cn.rs'
# ⑱ 空属性：应为 0（check-ui 已拦）。批量删类名（字重 / 色阶）后把属性一起删掉
grep -rnE --include='*.rs' 'class=""|style=""' .
# ⑳ 页面手写 <h1>：只允许 PageHeader / 带 pxl-title / sr-only（check-ui 已拦；print_page 为例外）
grep -rn --include='*.rs' '<h1' crates/app/src/pages | grep -v 'sr-only\|pxl-title'
# ㉑ 卡片标题用 12px：应为 0（check-ui 已拦）。区段标题走 card_title_class(...)
grep -rn --include='*.rs' '<h2' crates/app/src | grep 'text-sm'
# ㉒ 任意值断点：应为 0（check-ui 已拦；断点只有 xs / sm / md / lg）
grep -rnE --include='*.rs' 'min-\[[0-9]+px\]:' crates/app/src
# ㉓ 图标写 16/20px：应为 0（check-ui 已拦；像素图标是 24px 网格，写 size-6）
grep -rn --include='*.rs' '<Icon' crates/app/src | grep -E 'class="[^"]*\b(h-[345]|size-[345])\b'
# ⑲ 把 destructive 填充色当文字色：应为 0（check-ui 已拦）。
#    这是填充色，暗色下落在弹层表面上不足 4.5:1；写字用 text-hp-text。
#    `text-destructive-foreground`（实色按钮上的字）是另一个 token，不在此列
grep -rnE --include='*.rs' 'text-destructive([^-]|$)' . | grep -v 'cn.rs'
# ⑩ 页面里手写的页内吸顶偏移：应为 0（第二根吸顶栏走 StickyPanel，见 §4；
#     吸顶骨架唯一来源是 common 里的 PAGE_HEADER_BAR；表格粘性表头的 `sticky top-0` 不在此列）
grep -rnE --include='*.rs' 'sticky top-\[[0-9]' pages components | grep -v 'components/common'
```

各条当前状态（2026-10-24 第二轮复核）：①0 ②**0** ③0 ④0 ⑤**0**（`rounded-*` 已从视图清零，
卡片皮肤改用 `pxl-card` 标记）⑥0（剩下的 4 行是 3 处像素硬阴影 —— `shadow-[0_2px_0_0_…]` 与它的
`-2px` / `4px` 变体 —— 与 `cn.rs` 里的冲突组配置，都合规）⑦0 ⑧11 行 / 9 文件（= §4 的封闭清单）⑨11 条 / 10 文件（2026-10-24 刷新按钮收编后）⑪–㉓ 0（2026-10-24 新增）
⑩0（2026-10-16：browse / glossary / tools 已收口到 `StickyPanel`）。
其中 ①②③④⑤、⑥ 的模糊 / 渐变与任意值阴影、⑧ 的封闭清单由 `cargo make check-ui` 持续守着，
不必再靠人工记得跑。

## 12. 验收清单

```bash
# 1) 静态检查（格式 + clippy + 单元测试 + 文案词典 + 知识库译文 + 解析表 + 像素审计）
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

端口说明：`trunk serve` 默认是 3000（`crates/app/Trunk.toml`），这里与 AGENTS.md 一致用 3030，
起服务时要带 `--port 3030`，并让 `E2E_BASE_URL` 与之对应。

三条经验（都踩过）：

- **改了 `role` / `aria-*`，要按「断言里出现过这些属性」全仓搜 spec，不要按页面名找** ——
  smith 圆图的用例住在 `waveform_lab.spec.ts` 里，按页面名找就会漏。
- **`trunk serve` 的产物可能缺 `public/` 资产**（语言包、题库、worker），表现为页面空白 /
  整页挂载失败 / 英文界面回退中文 / `page.goto: Page crashed`。自检：
  `curl -s http://127.0.0.1:3030/data/i18n/en.json | head -c 1`（正常是 `{`）。
  另外 trunk 重建失败时页面会整页盖一层「Build failure」遮罩 —— 重启 `trunk serve` 即可，
  不是代码问题。
- **dev 构建没有 `wasm-opt`**，CPU 密集用例（如 Yagi 优化）并行时容易超时，
  `--workers=1` 复跑通过即非回归。整批回归请用 `cargo make e2e`（release 口径，与 CI 一致）。
- **只跑 `check-ui` 不够**：它只查类名字面量。wasm 目标的 clippy 也不覆盖 host 的测试构建
  （`impl IntoView` 默认把入参生命周期带进返回类型，只有 `cargo test` 会报，改法是
  `impl IntoView + use<>`）。**收工前跑一次 `cargo make check`。**

**按改动类型挑 e2e**（这些用例是规范的可执行部分，改对应面必跑）：

| 改了什么 | 必须跑 |
|---|---|
| 颜色 / 表面 token / 配色方案 | `color_scheme.spec.ts`（11 方案 × 明暗 × 设置弹层，**axe 对比度**）+ `smoke.spec.ts`（全站 × 明暗，含 axe） |
| 布局 / 字号 / 间距 / 断点 | `mobile_overflow.spec.ts` + `i18n_layout.spec.ts`（**180 路由 × 三语** × 320 / 390px） |
| 题目卡 / 答题卡 / 底栏 / 首页卡片 / 移动菜单 | `ui_spec.spec.ts`（用例标题带编号，能对上 §2 的规则） |
| 判分 / 练习 / 考试流程 | `study.spec.ts`、`study_modes.spec.ts`、`exam_flow.spec.ts`、`practice_flow.spec.ts` |
| 表格密度 / 触控目标 | `mobile_overflow.spec.ts` + 目视 320px 下的一屏条数 |
| 组件改名 / 新增 `role`、`aria-*` | 按属性全仓搜 spec（不要按页面名找） |
