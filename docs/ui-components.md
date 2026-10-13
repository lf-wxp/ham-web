# 通用 UI 组件使用规范

> 写给**要改界面的人与 AI agent**。这个仓库的界面只有一套控件来源 ——
> `crates/app/src/ui/`。本文件是**写新代码时的规则与清单**；
> 组件清单、每个 prop 的含义、逐个使用示例 → [`crates/app/src/ui/README.md`](../crates/app/src/ui/README.md)。
> 两份文档的分工：**本文件管「怎么写」，ui/README.md 管「有什么」。**
>
> 从哪读起：新写 / 改页面看 §1 → §3（控件在 §3.1、内容块在 §3.2）→ §4；改外观看 §2；
> 遇到「这里为什么有原生 `<button>`」查 §10（例外清单）；收工前过 §12。

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
- 附录 A：控件收口记录

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
| `schemes.css` | 其余配色方案（森林 / 海洋 / 晚霞 / 石墨）的亮 / 暗变量块；由 `cargo make pixel-schemes` 从 `crates/core/src/color_scheme.rs` 生成，**勿手改** |
| `palette.css` | Tailwind 默认色阶（`green-600` 等）整体换成像素调色板；由固定规则生成，**不要手调单个色值** |
| `fonts.css` / `fonts-pixel.css` | 字体声明；后者由 `cargo make fonts-pixel`（`crates/tools/src/pixel/fonts.rs`）生成，**勿手改** |
| `surfaces.css` | 全局基线、背景底纹、存量卡片兜底、滚动条、图标 / 精灵尺寸 |
| `controls.css` | `pxl-btn` / `pxl-field` / `pxl-window` / `pxl-titlebar` / `pxl-popover` / `pxl-overlay` / `pxl-check` / `pxl-radio` / `pxl-switch` / `pxl-chip` / `pxl-badge` / `pxl-item` / `pxl-bar` 等控件原语 |
| `motion.css` | 全部逐帧动画与「静态降级」 |

**硬规则**

1. **零圆角、零模糊、零渐变**：不要写 `rounded-*`（全局已归零，写了也无效）、`backdrop-blur`、
   `bg-gradient-*`、带模糊半径的 `shadow-[…]`。需要强调就换色、加粗描边、用抖动底纹。
   真要一个圆（极少）给元素加 `data-keep-round`。
   （**例外**：无模糊半径的像素硬阴影是合规的，如 `shadow-[0_2px_0_0_var(--pxl-shadow)]`。）
2. **尺寸取 4 的倍数**：间距、控件高度走 Tailwind 刻度（`h-6` / `h-8` / `h-10` / `h-12`）；描边 2px，
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
| 答题选项（点击即判分） | `QuizOption` | 手写 `<button class="…">` |
| 选择文件（按钮） | `FileInput` | `<input type="file" class="hidden">` + 标签按钮 |
| 选择文件（**整块拖放区**） | `FileDropZone` | 手写 `<label for>` + 隐藏 input |
| **执行动作**（提交、打开弹层、切换状态） | `Button` | `<button class=…>` |
| **跳转**（能中键新开、右键复制地址） | `ButtonLink` | `<a class=button_class(…)>` |
| 标签 + 控件 + 提示 / 错误 | `Field`（`hint` / `error` / `required`） | 手写 `<label>` + `<span>` |
| 进度、分隔线、统计块、标签文字 | `Progress` / `Separator` / `Stat` / `Label` | 手写 |

四个易混对子的区别：

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

### 3.1 按钮的尺寸与变体速查

| 场景 | 用 |
|---|---|
| 表格行内 / 工具条的**纯文本**小按钮 | `Size::Xs`（`h-6`）+ `Variant::Outline` |
| 紧凑图标按钮（朗读、关闭、缩放、删除） | `Size::IconSm`（`size-6`）+ `Variant::Ghost` / `Outline`；**必填** `aria_label` |
| 句内的「编辑 / 删除 / 关闭」（透明底、悬停换色） | `Variant::Text` + `Size::Xs`；悬停变红补 `class="hover:text-destructive"` |
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
| 空态（图标 + 标题 + 说明 + 操作） | `EmptyState` | 手写居中的提示块 |
| 内容卡片（标题栏 + 正文） | `SectionCard` | 再抄一份 `rounded-xl border bg-card` |
| 单按钮结果提示（成功 / 失败 / 已复制） | `MessageDialog` | 手搭 `Dialog` + 一个按钮 |
| 加载中（短等待）/ 首屏骨架 | `Loading` / `Skeleton` | 手写 spinner / 一片留白 |
| 图例条（色块 + 文字） | `Legend` | 手写色块 + 文字 |
| 题图（缩略图 + 点击放大） | `PreviewableImage` | 手搭 `Dialog` + `<img>` |
| 题目笔记编辑区 | `NoteEditor` | 手写 `Textarea` + 存取逻辑 |
| 答题进度头 / 底部固定操作栏 | `QuestionProgressHeader` / `BottomBar` | 手写进度行 / 固定底栏 |
| 答案解析（折叠 + 法规引用 + 朗读） | `ExplanationCard` | 手写折叠区 |

考试 / 练习 / 导航三族组件（`components/exam/`、`components/practice/`、`components/navigation/`）
只在对应流程里用，不在此表：改它们直接读文件头注释（弹层的开合、`Portal`、焦点与滚动锁都写在里面）。

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
| 粘性页头（标题 + 副标题） | `PageHeader` | `actions=ViewFn::from(move || view! { … })` 放右侧操作区；页头上方还有别的粘性栏时传 `class="top-16"` |
| 多行页头（标题行 + 工具条 / 确认条） | `PageHeader` 的 `children` | 子节点渲染在标题行下方；自己写一行 `mx-auto flex max-w-5xl …` 与标题行左边缘对齐 |
| 页面正文 | `PageContainer` | 默认 `space-y-6 px-4 py-5`；间距 / 内边距不同就用 `class` 覆盖；页面内锚点用 `id="…"` |
| 内容块滚动浮现 | `PageContainer` 的 `reveal=true` | 直接子元素进入视口时逐个浮现（同批 60ms 错峰）；`KnowledgePage` 已默认开启，普通内容页不必开 |
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

自检见 §11 的 ⑧（输出是一份**很短的封闭清单**，当前 **10 行 / 9 个文件**，逐条对得上即可；
出现新行就说明又手抄了一份容器）：

| 位置 | 为什么允许 |
|---|---|
| `components/common/page_header.rs` | 页头宽度就在这里定义（`page_container.rs` 用 `cn(&[…])` 拼类名，不出现在这条 grep 里） |
| `app/footer.rs`、`components/common/bottom_bar.rs` | 外壳与正文左边缘对齐 |
| `pages/mistakes/mistakes_page.rs`、`pages/bookmarks/bookmarks_page.rs`、`pages/qsl_labels/qsl_labels_page.rs`（2 处：筛选工具条 + 未设置呼号提示行） | `PageHeader` 的 children 行，与标题行对齐 |
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
- **折叠 / 下拉**：`Button` 的 `aria_expanded` / `aria_controls`（面板有 id 时一并给）。
- **动态提示**：实时结果区用 `aria-live`，并用 `Input` 的 `aria_describedby` 关联 id。
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

## 9. 新增 / 扩展组件

- **先扩展现有组件**：缺通道（例如 `title`、`maxlength`、`aria_expanded`）时，优先给现有组件
  补 prop，而不是在页面里退回原生元素。补完在 `ui/README.md` 组件表里补一行。
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
`grep -rn "刻意的例外" crates/app/src` 可列全（当前 **9 条注释 / 7 个文件**，全在 §10.2，
其中 `nav_bar.rs` 占 3 条；9 条里 7 条走 `button_class`，`nav_bar.rs` 的移动端语言切换与
`dx_spots.rs` 的表格行按钮用的是 `cn` / 原生类名）；
**新写界面不要模仿它们**，它们各自的理由如下。

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
| `components/navigation/nav_bar.rs` | ① 导航项（`<a>`）：高亮要切 `variant`（`ButtonLink.variant` 是静态 prop）；② 分组下拉开关（`<button>`）：高亮随路由切 `variant`（`aria-expanded` / `aria-controls` 已有通道，不再是理由）；③ 移动端面板的语言切换（`<button>`×N）：选中态要重算 `class`（`Button.class` 是静态串，顺带输出 `aria-pressed`） |
| `components/navigation/group_menu.rs` | 「知识库」「工具」两个下拉开关（共用一条注释，渲染出 2 个 `<button>`）：理由同 nav_bar；面板是分组侧栏 + 条目区，见文件头 |
| `pages/callsign_copy.rs` | 两个模式按钮：选中态切 `variant` |
| `components/exam/answer_card_sheet.rs` | 答题卡筛选按钮：当前筛选切 `variant` |
| `pages/home/cards_card.rs` | 「今日待复习」入口：有待复习时切 `variant` |
| `components/bank_selector/mod.rs` | 刷新按钮：底色随刷新状态（成功 / 失败 / 加载）变化，`class` 也要按状态合并 |
| `pages/dx_spots.rs` | 热点表格行内的紧凑内容按钮（呼号 + 国家两行）：像素按钮的高度与内边距会把行高撑开、破坏表格密度 |

它们连同 `Button` / `ButtonLink` 内部都继续用 `button_class` —— **`ui::button_class` 是这套
组件的底座，不要删**。

### 10.3 保留自绘的其他控件

| 类别 | 例子 | 为什么不迁 |
|---|---|---|
| 多选 / 带对错态的答题选项 | `mistakes/*` 的选项、`components/topic_quiz/quiz_question.rs` | 多选 + 判分后的对错配色，单选款的 `QuizOption` 表达不了 |
| 卡片点击区、热图格、地图区域 | `pages/cards.rs` 卡片、`morse_card` / `phonetic_card`、`muf` 热图格、`zone_map` 地图 | 点的是「内容」不是按钮，`Button` 的尺寸与视觉都表达不了 |
| morse 大圆键 | `koch_trainer` / `morse_trainer` 的试听键、`send_trainer` 的长按发报键 | 交互不同（点击 vs 长按，带扩散圈动画），共用的只有外壳 |
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
- **迁移历史**（哪一轮迁了什么、e2e 契约怎么变的）见附录 A。

## 11. 自检命令

```bash
cd crates/app/src
# ① 本地控件样式常量：应为 0
grep -rn --include='*.rs' --exclude-dir=ui -E '^const (INPUT|TEXTAREA): ' .
# ② 原生表单元素：应为 0（拖放区在 ui::FileDropZone 里，见 §10.1）
grep -rn --include='*.rs' --exclude-dir=ui -E '<input|<select|<textarea' .
# ③ 类名工厂：input_class 应为 0（工厂已删）
grep -rn --include='*.rs' --exclude-dir=ui -c 'input_class(' . | grep -v ':0$'
# ④ 手写 chip 的样式常量：应为 0
grep -rln --include='*.rs' --exclude-dir=ui 'CHIP_ON' .
# ⑤ 自造的胶囊样式：逐条确认都是徽章 / 链接 / 展示用胶囊，而不是「按钮自己写样式、自己切选中态」
#    —— 后者扫描器认不出（常量名与组件都没有），只能靠这份清单人工过（判据见 §10.4）
#    本仓的例外只有 `pages/morse/pill_button.rs`（业务组件底座，理由见 §10.3）
grep -rn --include='*.rs' --exclude-dir=ui 'rounded-full border' .
# ⑥ 页面里残留的圆角 / 模糊 / 渐变：应为 0（输出里 `shadow-[0_Npx_0_0_…]` 是像素硬阴影，合规）
grep -rnE --include='*.rs' --exclude-dir=ui 'backdrop-blur|bg-gradient|bg-linear|shadow-\[0' .
# ⑦ 把填充色 token 当文字色用（对比度不够）：应为 0
grep -rnE --include='*.rs' 'text-\[color:var\(--pxl-(hp|xp|gold|win)\)\]' .
# ⑧ 页面级宽度类：输出应等于 §4 那份封闭清单（10 行 / 9 文件：组件自身 / 页头对齐行 / 顶栏 container / 横幅）
grep -rn --include='*.rs' --exclude-dir=ui -E 'class="[^"]*\bmax-w-(2xl|3xl|4xl|5xl|6xl)\b|class="container' pages components app \
  | grep -vE 'min-w-|<Dialog|sm:max-w'
# ⑨ 刻意保留的原生元素：应等于 §10.2 的清单（9 条注释 / 7 个文件）
grep -rn "刻意的例外" .
```

各条当前状态（2026-10-13 复核）：①0 ②**0** ③0 ④0
⑤53 处（52 处是徽章 / 链接 / 展示用胶囊，例外是 `pages/morse/pill_button.rs` 这个业务组件的底座，
理由见 §10.3）⑥0（剩下的 4 行是 3 处像素硬阴影 —— `shadow-[0_2px_0_0_…]` 与它的 `-2px` / `4px`
变体 —— 与 `cn.rs` 里的冲突组配置，都合规）⑦0 ⑧10 行 / 9 文件（= §4 的封闭清单）⑨9。

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

## 附录 A：控件收口记录

> 收口分多轮完成，这里是**结果索引**（逐文件明细看 git log）。写新代码不需要读它；
> 遇到「这里为什么是 chip / 那边为什么留了原生按钮」时来查。

| 轮次 | 做了什么 |
|---|---|
| 早前 | 原生表单控件清零（`Input` / `Select` / `Textarea` / `Slider` / `NumberField` / `Checkbox` / `Switch`…），`input_class` 工厂删除；内容宽度收口到 `PageHeader` / `PageContainer`；像素风基线与 `pxl-*` 控件原语落地 |
| 2026-10-12 | 手写动作按钮 → `Button` / `ButtonLink`（13 文件）；手写互斥选择 / 开关 → `ChipGroup` / `Chip` / `ChipToggle`（15 文件，含 morse 三件套、打印页、错题页） |
| 2026-10-13 | 给 `Button` 补 `Size::Xs` / `Size::IconSm` / `Variant::Text` / `aria_expanded` / `aria_controls`；收编 nec 表格工具条（11）、文字按钮（8）、图标按钮（6）、折叠开关（5）；提取 `QuizOption`（8 处）/ `SidebarItem`（4 处）/ `PillButton`（4 处）；两处同名的 `pill()` 私有函数删除 |
| 2026-10-13 | 提取 `ui::FileDropZone`：解码类页面（APT / SSTV / WSPR / RTTY / Olivia / NAVTEX / LRPT / DominoEX / PSK31 / 离线瀑布分析）与图片处理页的整块拖放上传区全部收口，删除 `AptUploader` 与 psk / 瀑布页手写的拖放区；非音频上传位用 `choose_hint` 覆盖默认的「拖拽音频文件」提示行 |
| 2026-10-13 | `pages/most_wanted/wanted_tracker.rs` 的手写勾选框 → `ui::Checkbox`（顺带补上 `role="checkbox"` / `aria-checked`；§10.3 的对应条目已删除） |

**这轮迁移带来的 e2e 契约变化**（改 role 就要同步 spec）：`Chip` 输出 `role="radio"` +
`aria-checked`，`ChipToggle` 输出 `aria-pressed`；`browse` / `planning` / `practice_flow` /
`cards` / `study_modes` 的定位与 `fixtures.option()`（选项正则加负向断言，避开「A 类」
这类同名 chip）都已同步。

**沉淀下来的原则**（为什么这么迁）：

- 外观与 `pxl-btn` 一致的 → 直接用 `Button`；只是「颜色淡一点 / 小一号」的，先补尺寸档
  （`Size::Xs` / `Size::IconSm`）或变体（`Variant::Text`），**不要**在页面里复制一份样式。
- 「可点的互斥 / 开关按钮自己写样式」→ `ChipGroup` / `ChipToggle`（顺带补上
  `role="radio"` / `aria-pressed`）。
- 同一个交互复制 **≥3 次**且现有组件表达不了 → 提取组件（`QuizOption` / `SidebarItem` /
  `PillButton` 都是这么来的）。
- 缺的只是**通道**（`aria_expanded` / `aria_controls` / 尺寸档）→ 补 `Button`，不要退回原生元素。
