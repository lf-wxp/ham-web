# S9 CONSOLE 设计规范

> **性质**：`s9-console-design.html`（30 张设计板 + 外壳）的**现行开发规范**，供改板、扩展、落地实现与评审时直接对照。
> **事实源**：运行中的 `s9-console-design.html` 源码。本文档是规则的可读表述，不替代源码；冲突时以源码为准并立即回写（§1.3）。
> **读者**：改设计板的开发人员、把设计板落成界面代码的开发人员、评审者。
> **初始版本**：2026-11-17。修订方式见 §10.4。

---

## 0. 快速索引

| 我要做什么 | 看哪一节 |
| --- | --- |
| 判断某条规则能不能破例 | §1.2 规范等级、§1.3 冲突裁决 |
| 取颜色 / 圆角 / 字号 / 层级 / 时长 | §3 样式约定 |
| 找一个控件该用哪个工厂 | §4.2 组件分类索引、附录 A |
| 改顶栏「更多」路由目录 | §4.7 |
| 写表单 / 选择器 / 日期时间 | §8.1、§8.2、§7 场景 1–3 |
| 写弹层、退场动画、Toast | §4.4、§6.2、§8.3、§7 场景 5–7 |
| 写动画 | §3.4、§6.1、§8.5、§7 场景 8–9 |
| 加一个页面板 / 一个控件 / 一个状态 | §8.4、§8.6、§10.3 |
| 判断命名是否合规 | §5 命名与结构规则 |
| 无障碍、键盘、对比度、降级 | §6 交互与可访问性 |
| 提交前自检 | §9 验收清单 |
| 改了 token / 工厂后要同步什么 | §10.2 改动同步矩阵 |
| 在源码里定位某个约定 | 附录 A 符号索引 |

---

## 1. 适用范围、优先级与效力

### 1.1 适用范围

**【必须】本规范约束**：

- `docs/design/s9-console-design.html` 内的全部 CSS、JS 模板、页面板（`S9.B.<id>`）、外壳与浮层。
- 由该文件派生的实现：把设计板落成界面代码时，视觉取值、组件选择、状态语义、动画档位、无障碍契约以本规范为准。

**【必须】本规范不约束**：

- 产品代码 `crates/app/`、`style/modern/`、`kit_modern/`、`components/` 的现行写法 —— 那些以 [`docs/README.md`](../README.md) 索引下的 `docs/ui/*.md`、`docs/engineering/*.md` 为准（例如 `AGENTS.md` 的硬规则、`docs/ui/motion.md §五` 的弹层口径）。两套适用范围不同，不要互相套用取值。
- `s9-console-ui-prompts.md` 的生图提示词内容 —— 它是面向图像模型的**设计提案**，每条提示词自包含，见 §1.4。
- 打印输出的分页与纸张参数（`@media print` 的自有规则）。

### 1.2 规范等级

| 等级 | 含义 | 破例条件 |
| --- | --- | --- |
| **【必须】** | 违反即视为回归：源码扫描、语法检查或 Playwright 回归会失败，或产生已修复过的视觉 / 交互缺陷 | 无。确需改变规则时先改 §10 的单一来源并同步文档，**不是**在页面里开特例 |
| **【推荐】** | 默认遵守；允许局部例外，但必须在代码注释写明「为什么」，且不得引入第二套实现 | 有明确理由 + 同行注释 |
| **【可选】** | 建议做法，不影响验收 | — |

### 1.3 冲突裁决顺序

当两份材料说法不一致，**从上到下取第一个**：

1. `s9-console-design.html` **运行中的源码**（CSS token 实际值、`F.*` 实现、`S9.*` 运行时契约）。
2. 本文档 `s9-console-design-spec.md`（现行规则的完整表述）。
3. 设计板内可视化摘要：`kitall` 板「09 统一化契约」表、`components` 板「交互状态 × 过渡口径」表、`kitbase` 等板的说明文字。它们是**速查指针**，不复制规则全文。
4. `s9-console-ui-prompts.md`（仅表达视觉意图）。
5. 产品代码规范 `docs/ui/*.md`（另一适用范围，正常情况不冲突）。

**【必须】** 特别提示：`s9-console-ui-prompts.md §2` 的浅色色值（暖白 `#FFFEFB`、高饱和橙 `#FF4D1F`）、字体清单（Bricolage / Geist）、「七段屏非激活段 12% 亮度」等是**提案阶段的取值，已被现行源码取代**，不得作为开发取值依据。现行值见 §3.1、§3.2、§4.3。

### 1.4 与既有文档的关系

| 文件 | 职责 | 关系 |
| --- | --- | --- |
| `s9-console-design.html` | 可运行的设计板 + 事实源 | 本规范描述它 |
| `s9-console-design-spec.md` | 现行开发规范（本文件） | 唯一完整规则表述 |
| `s9-console-ui-prompts.md` | 生图提示词交付稿 | 只表达视觉意图；取值已过期，见 §1.3 |
| `docs/README.md` + `docs/ui/*.md` | 产品代码现行规范 | 另一适用范围，见 §1.1 |
| `AGENTS.md` | 仓库级硬规则入口 | 产品代码约束，不覆盖设计板 |

三份设计稿都在 `docs/design/`，与产品代码的现行规范分开归类，理由见 §10.5。

---

## 2. 设计原则

八条原则，每条都对应一处可 grep 的落地锚点。新增代码若与原则冲突，先改原则的实现，不要在页面里绕开。

| # | 原则 | 结论 | 落地锚点 |
| --- | --- | --- | --- |
| P1 | **双主题同构** | 两套主题共用同一组 token 名与语义角色，**只换值不换角色**；亮色不用纯白，按 `页面底 → 卡面 → 内凹层 → 仪器屏` 明度递进，统一冷灰色温 | `:root` / `html[data-theme='light']` |
| P2 | **单一来源** | 一个事实只在一处写全：时长、层级、遮罩、阴影、色阶、状态词表、控件工厂、卸载延迟各只有一份定义 | §10.1 |
| P3 | **控件只有一套来源** | 所有交互控件由 `F.*` 工厂生成；页面不写原生 `<input>/<select>/<textarea>`，不新增本地类名工厂 | `const F = S9.frag` |
| P4 | **动画只跑合成层** | 循环动画只改 `transform` / `opacity`，不写 `height` / `left` / `width`；多元素循环用负相位，一开就是完整流动 | §3.4、§6.1 |
| P5 | **视觉层级只留一层线** | 容器只有一层 `border`，层次靠底色与柔和内凹表达；不叠 inset 1px 线、不给屏幕再描一圈 | §7 场景 11 |
| P6 | **语义化图标** | 图标取 `ICON_PATHS` + `F.icon*`，统一 `--ico 20px` / 1.8px 描边 / 24 网格，`aria-hidden="true"`；不用字符型箭头冒充控件图标 | `ICON_PATHS` |
| P7 | **状态用小写 slug** | DOM `data-*` 状态值一律小写；显示文案可沿用大写仪器字，转换只在 `S9.stateToken` | §5.2 |
| P8 | **无障碍三通道** | 状态差异同时体现在视觉 + 文字 + ARIA；颜色不单独承载语义；错误态只认 `aria-invalid` | §6 |

---

## 3. 样式约定

### 3.1 双主题角色表

**【必须】** 组件只引用 token，不写死色值；新增颜色先在 `:root` 定义并在亮色块给覆盖值。

| 角色 | token | 暗色 | 亮色 | 用途 |
| --- | --- | --- | --- | --- |
| 页面底 | `--bg` | `#04050a` | `#e8ecf3` | 页面根底，亮色非纯白 |
| 卡面 | `--surface` | `#0b0d13` | `#f7f9fc` | 面板 / 卡片 / 浮层 |
| 内凹层 | `--surface-2` | `#12151e` | `#edf1f7` | 输入框底、内凹面板 |
| 深层 | `--surface-3` | `#1a1f2b` | `#dfe6f0` | 悬浮层、滑块游标 |
| 正文 | `--fg` | `#f2f4fa` | `#141922` | 主文字 |
| 次要文字 | `--muted` | `#8a92a6` | `#5b6573` | 标签、注释、刻度 |
| 行动色（底） | `--accent` | `#ff6a3d` | `#c2410c` | 唯一实心 CTA、选中边框、峰值 |
| 行动色（文字） | `--accent-ink` | `#ff8a63` | `#9a3412` | 链接、只读值、目录选中 |
| 行动色上文字 | `--on-accent` | `#100604` | `#fff` | 实心按钮 / FAB 图标，按底明度反转 |
| 焦点 / 选中蓝 | `--blue` | `#6f8cff` | `#2f5bd8` | 焦点环、选中态 |
| 分隔线 | `--line` | `#222839` | `#d5dce7` | 低对比层级线 |
| 控件边界 | `--line-strong` | `#3a4460` | `#bcc7d6` | 输入 / 按钮边框 |
| 洗底 | `--wash` / `--blue-wash` | 橙 / 蓝 8% | 橙 6% / 蓝 7% | 选中底、内凹提示 |
| 成功 / 告警 / 金 | `--green` `--red` `--gold` | `#35e39f` `#ff5d5d` `#ffcf6b` | `#0f7a5a` `#c0362c` `#a8760a` | 同色相、亮色压明度 |
| 仪器屏 | `--screen-bg` `--screen-shadow` | `#020306` + 内凹微光 | `#e7edf7` + 柔和内阴影 | 示波屏 / 七段屏，不写死暗色 |
| 数据柱 | `--bar-top` `--bar-bottom` `--bar-edge` `--bar-hit-glow` | 亮蓝渐层 | 降饱和蓝灰 `#93a9d0` 系 | 频谱柱 / 图表柱 |
| 光晕 | `--glow` `--glow-blue` | 8px 橙 / 蓝 | **归零** | 暗色描边微光；亮色不发辉光 |

**【必须】** 一屏只有一个实心行动色；次操作用透明底 + `--line-strong` 边框，不叠底色。

### 3.2 几何、字体与图标

| 项 | 取值 | 规则 |
| --- | --- | --- |
| 圆角 | `--r1:4px` `--r2:6px` `--r3:8px` `--r4:10px` `--r5:14px` | 弹层面板四角 `--r3`；卡片 / 面板 `--r4`~`--r5`；不写裸 `px` 圆角 |
| 版心 | `--page-max: 1200px` | 页面宽度只有一个来源，页面内不写 `max-w-*` / 自造居中容器 |
| 图标 | `--ico: 20px` | 1.8px 描边 / 24 网格；指示灯 6–8px 属另一类 |
| 正文 | 15px / `line-height: 1.55` | `body` 基线 |
| 字体 | `--font-display` / `--font` / `--font-mono` | 数字、频率、倒计时、呼号一律 `--font-mono` + `tabular-nums` |
| 工程网格 | 24px / `--grid` | 只在 `data-grid='1'` 时出现 |

### 3.3 层级 `--z-*`

**【必须】** 组件不写裸 `z-index` 数字；新增浮层先在 `:root` 取号。

| 分组 | token | 值 | 成员 |
| --- | --- | --- | --- |
| 版式内局部层 | `--z-flow-caption` / `--z-grid` / `--z-board` | 2 / 3 / 4 | 流程标签、网格叠层、板内容 |
| 手机预览内 | `--z-fab` / `--z-fab-mask` / `--z-fab-sheet` | 6 / 8 / 9 | FAB、遮罩、抽屉 |
| 局部吸顶 | `--z-local-dock` | 20 | 手机 dock、吸顶 frame（局部 stacking context，不与全局浮层比大小） |
| 全局浮层 | `--z-overlay` / `--z-panel` / `--z-modal` | 20 / 21 / 22 | 遮罩、调谐台 / 抽屉、模态 |
| 展示层 | `--z-demo-dialog` | 40 | 设计板内 Dialog 演示层 |
| 最上层 | `--z-popover` / `--z-toast` | 90 / 100 | Picker 面板（挂 `body`）、Toast |

### 3.4 动画时长与缓动

**【必须】** CSS 不写裸毫秒，JS 不写裸毫秒（见 `S9.motion`）。

| 用途 | token | 值 |
| --- | --- | --- |
| 按压 / 快反馈 / **退场** | `--dur-fast` → `--dur-press` | 120ms |
| 状态换态 / **进场** | `--dur` → `--dur-state` | 180ms |
| 展开 / 折叠 | `--dur-slow` → `--dur-expand` | 320ms |
| 扫描 / 涟漪 | `--dur-scan` | 560ms |
| 实时波形循环 | `--dur-live` | 2400ms |
| 塔台电波（复用 modern UI） | `--dur-ring` | 4200ms，对齐 `md-ring-out` |
| 卫星漂浮 / EME 去程 / 回程 | `--dur-sat` / `--dur-eme` / `--dur-eme-back` | 7000 / 5000 / 7000ms |
| 月盘呼吸 / 极光漂移 | `--dur-moon` / `--dur-aurora` | 9000 / 18000ms |
| 闪烁 | `--dur-blink` | 900ms |
| 骨架扫光 | `--dur-skel` | 1600ms |
| 首帧阶梯 | `--dur-enter-1..5` | 420 / 520 / 560 / 640 / 720ms |
| 首帧错峰 | `--dur-stagger-1..7` | 26 / 40 / 70 / 110 / 140 / 180 / 250ms |
| 缓动 | `--ease` | `cubic-bezier(.2,.7,.3,1)` |
| 勾选微弹 | `--ease-pop` | `cubic-bezier(.34,1.56,.64,1)` |
| 塔台扩散（唯一例外） | `--ease-out` | `cubic-bezier(.16,1,.3,1)`，对齐 `--md-ease-out`，仅用于环波 |
| 主题切换 | `--tr-theme` | 只含 `background-color / border-color / color / box-shadow / fill / stroke`，各 180ms；**不含** `transform / opacity` |

派生关系用 `calc`，不复制数值：塔台电波相位 `calc(var(--dur-ring) / -3)`、`calc(var(--dur-ring) * -2 / 3)`（等效 modern UI 的 1.4s / 2.8s 错峰，负相位让首帧铺满）。

**`S9.motion`（JS 计时唯一来源）**：

```js
S9.motion = {
  exit: 120,          // 对齐 --dur-fast，S9.presence 唯一卸载延迟
  toastDwell: 1800,   // Toast 停留
  ripple: 560,        // 对齐 --dur-scan
  scanStep: 65, scanTotal: 900,
  countFrame: 47.5, countSteps: 16,
  feedback: 520, flash: 340, bubble: 2000,
  tick: 1000,         // 功能周期（UTC 时钟 / 考试倒计时）
};
```

### 3.5 遮罩与阴影

**【必须】** 同类效果只取一套 token，亮色块覆盖为柔和冷灰。

| token | 暗色 | 亮色 |
| --- | --- | --- |
| `--scrim` | `rgba(2,3,6,.72)` | `rgba(20,25,34,.48)` |
| `--scrim-soft` | `rgba(2,3,6,.62)` | `rgba(20,25,34,.42)` |
| `--scrim-mini` | `rgba(2,3,6,.55)` | `rgba(20,25,34,.36)` |
| `--shadow-pop-sm` | `0 14px 32px rgba(0,0,0,.45)` | `0 10px 24px rgba(20,25,34,.14)` |
| `--shadow-pop-md` | `0 18px 48px rgba(0,0,0,.45)` | `0 12px 32px rgba(20,25,34,.15)` |
| `--shadow-pop` | `0 24px 80px rgba(0,0,0,.55)` | `0 18px 48px rgba(20,25,34,.16)` |
| `--fab-shadow` | `0 6px 18px rgba(255,106,61,.32)` | `0 4px 12px rgba(20,25,34,.12)` |
| `--glow` | `0 0 8px rgba(255,106,61,.32)` | `0 0 0 rgba(0,0,0,0)` |
| `--tower-glow`（塔身辉光，对齐 modern UI `--md-glow-2`） | `rgba(111,140,255,.14)` | `rgba(47,91,216,.12)` |

> 亮色不发橙色光晕（历史教训：FAB 曾把暗色 `--on-accent` 与橙光晕写死，见 §7 场景 10）。

### 3.6 CSS 书写规则

**【必须】**

- 2 空格缩进；`{` 前空格、`:` 后空格；`}` 独占一行；多选择器每个独占一行。
- 组件样式用 class 选择器；`#board` / `#overlay` 等仅用于**页面级作用域限定**。
- 不用 `!important`；无边框用 `0` 代替 `none`；小数不写前导零（`.12` 而非 `0.12`）。
- 类名小写 + 连字符，修饰符用 `--`，元素用 `-` 后缀（§5.1）。
- 不在组件里写浏览器前缀、不写死主题色值（§3.1）、不写裸时长（§3.4）、不写裸层级（§3.3）。

**【推荐】** 字符串用单引号；颜色值小写；注释写「为什么」不写「做了什么」。

### 3.7 主题切换

**【必须】** `case 'theme'` 只调 `S9.syncChrome()`，不调 `S9.render()`。整板重建会丢 DOM、打断颜色过渡与滚动位置。所有大色块（`body` / `.s9-bar` / `.s9-rail` / `.s9-panel` / `.s9-card` / `.s9-scope` / `.s9-seven`）走 `--tr-theme` 180ms；`color-scheme: dark|light` 同步原生控件；`html[data-reduce='1']` 把时长归 1ms。

---

## 4. 组件约定

### 4.1 唯一来源

**【必须】** 所有界面控件由 `F.*`（`S9.frag`，共 112 个工厂）生成。页面里不写原生 `<input>` / `<select>` / `<textarea>`，不新增本地 `const INPUT` / `CHIP_ON` 之类的类名工厂，不直接拼控件类名。

- 完整清单以运行时 `Object.keys(S9.frag)` 为准；设计板 `kitall`「08 页面 → 全局组件接入」表由源码实时扫描 `F.*` 调用生成，改任何板会跟着变。
- **【推荐】** 新增控件先查 §4.2 是否已有同形态工厂；确需新增时加在工厂区，不要落在页面板里。

### 4.2 组件分类索引

| 类别 | 代表工厂 | 什么时候用 |
| --- | --- | --- |
| 外壳 / 版式 | `F.frame` `F.frameMore` `F.frameMoreTrigger` `F.frameMorePanel` `F.frameMoreGroup` `F.frameMoreItem` `F.head` `F.panelHead` `F.panel` `F.card` `F.section` `F.sectionHead` `F.row` `F.stack` `F.cols` `F.spread` `F.mobileDock` `F.phoneDock` `F.barTools` `F.navItem` | 页面骨架、栅格、顶栏与顶栏二级菜单、导航 |
| 排版标签 | `F.label` `F.note` `F.mono` `F.tag` `F.badge` `F.rule` `F.formula` | 标签、说明、等宽值、标记、分隔 |
| 基础控件 | `F.chip` `F.chips` `F.chipClose` `F.choice` `F.choiceBox` `F.btn` `F.toolButton` `F.switch` `F.switchButton` `F.tabs` `F.pager` `F.pagerBtn` | 按钮、Chip、复选 / 单选、开关、页签、分页 |
| 表单 | `F.input` `F.select` `F.textarea` `F.inputAddon` `F.field` `F.valueField` | 所有输入，含错误态 / 提示 / addon |
| 选择器 / 日历 / 时钟 | `F.pkPop` `F.pkTrigger` `F.pkPanel` `F.pkGroup` `F.pkOpt` `F.pkOpts` `F.selectPop` `F.pkCalendar` `F.pkCalBar` `F.pkCalGrid` `F.pkYearGrid` `F.pkCell` `F.pkClock` `F.pkClockItem` `F.pkStep` | 弹层式下拉、DatePicker、TimePicker、步进输入 |
| 导航 / 目录 | `F.dirSearch` `F.dirItem` `F.dirSubItem` `F.dirSub` | 侧栏目录、分组折叠 |
| FAB / 抽屉 | `F.fabIcon` `F.fabToggle` `F.fabClose` `F.fabRow` `F.fabSubItem` `F.fabMask` `F.fabSub` | 窄屏悬浮入口 |
| 弹层 / 反馈 | `F.dialog` `F.dialogLayer` `F.dialogSurface` `F.toast` `F.errorState` `F.tableEmpty` `F.skeleton` | 模态、Toast、空态、错误块、骨架 |
| 上传 | `F.dropZone` `F.fileRow` `F.fileList` | 拖入区与文件进度 |
| 表格 / 数据 | `F.table` `F.readout` `F.tile` `F.tileButton` `F.stat` `F.progress` `F.bars` `F.waterfall` `F.spectrum` `F.scope` | 表格、读数、统计、图表容器 |
| 仪器 | `F.knob` `F.slider` `F.meter` `F.wave` `F.scope` `F.spectrum` `F.seven` `F.led` `F.ledRow` `F.bandChips` | 频谱实验室 / 仪表盘专属读数 |
| 图标 | `F.iconSvg` `F.icon` `F.iconCell` | 语义图标统一出口 |
| 页面专属 | `F.qsl` `F.answerGrid` `F.tower`（默认 modern UI 首屏主视觉，`{ simple: true }` 简笔塔）`F.workbenchChart` `F.toolChart` | 单页内容，不属通用控件 |

### 4.3 控件契约

| 契约 | 规则 |
| --- | --- |
| 状态载体 | 一律 `data-*`（`data-on` / `data-state` / `data-checked` / `data-expanded` / `data-selected` / `data-open`），值小写 slug（§5.2）；不用 class 切换状态 |
| 错误态 | **只认 `aria-invalid="true"`**，不另造 `data-invalid` / `.is-error`；Select 容器镜像同一属性，选中后清除 |
| 禁用态 | `disabled` 属性 + `aria-disabled`（弹层式控件）；样式由 `:disabled` / `[aria-disabled]` 驱动 |
| 标签关联 | `F.field` 的 `<label for>` 与控件 `id` 一一配对 |
| 无可见标签 | 图标按钮、无文字控件必填 `aria-label` |
| 开关 | `role="switch"` + `aria-checked`；ON 态同时换外框 `--accent`、轨道底 `--wash`、拨片实心 `--accent`、刻度 `--on-accent`，刻度用 3 条 `<i>` + `space-evenly` 四等分 |
| 选择项 | `role="option"` + `aria-selected`；单选组 `role="radiogroup"`，复选 `role="checkbox"` + `aria-checked`；日历格用 `button` + `aria-pressed` + `aria-label="YYYY年M月D日"`（不用 `gridcell`） |
| 图标 | 语义图标取 `ICON_PATHS`（`calendar` / `clock` / `chevronLeft` / `chevronRight` / `chevron` / …），`aria-hidden="true"`，尺寸 `--ico` |
| 七段读数 | `F.seven(value, opt)` **只输出一层** `.s9-seven-value`；不叠 ghost 暗段 |

### 4.4 三类弹层

| 类型 | 宿主 | 层级 | 进场 / 退场 | 关闭态 |
| --- | --- | --- | --- | --- |
| **Picker**（Select / Date / Time / Popover） | 挂 `body` 的 `.s9-pk-layer`，`position: fixed`，按触发器 `getBoundingClientRect()` 定位 | `--z-popover: 90` | 180ms 进 / 120ms 出，`data-state` 驱动 | `pointer-events: none` |
| **桌面浮层**（调谐台 / 抽屉 / 模态 + 遮罩） | `#overlay` / `#palette` / `#drawer` / `#modal` | `--z-overlay` / `--z-panel` / `--z-modal` | 180ms 进 / 120ms 出，`data-state="closed"` | `pointer-events: none` |
| **Toast** | `body` 直接子节点 | `--z-toast: 100` | 180ms 进 / 120ms 出 | `pointer-events: none` |

**【必须】Picker 面板几何**：与触发器留 **6px** 间隙、四角 `--r3` 圆角、空间不足上翻并 clamp 到视口内；面板脱离祖先 `overflow` 与 stacking context，不依赖 `overflow: visible` 补丁。滚动 / resize 时重定位；切板与打印前 `S9.pkLayerSweep()` 清理孤儿面板。

顶栏「更多」路由目录是 Picker 的**宽版变体**（`data-pk-kind="nav"`）：桌面宿主、层级、进出场与卸载完全同上，只由 `S9.pkPlace()` 按整条顶栏的 rect 取宽度与下沿（6px 间隙从顶栏下沿算，不按触发器）；移动端宿主换成 `.s9-mobile-shell` 的屏内 Sheet，几何全交给 CSS。完整契约见 §4.7。

**【必须】卸载延迟**：只用 `S9.presence.close(el, done)`，其 `exitMs()` 取 `S9.motion.exit`（120ms，REDUCE 归 0）。不得在组件里另写 `setTimeout(…, 120)` 或直接 `el.remove()` / `el.hidden = true`。

**【必须】互斥**：打开调谐台 / 抽屉 / 全局搜索 Dialog 前先 `S9.pkCloseAll(true)`，避免 Picker 压在模态之上。

### 4.5 允许的例外

**【必须】** 页面板内允许保留的原生 `<button>` 仅限下列**页面专属内容**（不是通用控件，不进 `F.*`）：

`s9-option`、`s9-bento-cell`、`s9-due-bar`、`s9-cat-row`、`s9-heat-cell`、`s9-wb-anchor`、`s9-card`（QSL 卡式按钮）。

新增原生按钮前先确认不属于通用控件；每个例外仍须满足 §6 的无障碍要求。工厂实现内部生成的原生标签不算例外（它们就是唯一来源）。

### 4.6 页面骨架与注册表

**【必须】**

- 一张板一个函数 `S9.B.<id> = () => \`...\``，返回 `${F.frame('…')}<div class="s9-page">…</div>`。
- `S9.render()` 统一装配 `F.head(p)` 与 `.s9-board-body`（含移动端手机壳），板函数不自己写这层。
- 每板在 `S9.pages` 注册 `{ id, group, title, route, sig, note }`；`sig` 取已有页面签名，新增路由必须挑一个。
- 首帧错峰用 `S9.attrI(n)` 生成 `--i`，不手填序号。

### 4.7 顶栏二级菜单「更多」

**【必须】** 桌面顶栏（`F.frame`）只留六个一级入口；第七项「更多」是**全站路由目录**，把注册表的每条路由逐条列出，不折成 `+N`、不按设计板合并成一条，否则用户看不到站点有哪些能力。**移动端入口只留一个**：顶栏不放「更多」，全站 184 条目录只由底部 dock 最右那枚「更多」打开；顶栏改为补一枚搜索 icon 做**全站搜索**（内容 + 页面，不只是路由）。两个入口各管一件事，不互相退化、不在窄屏叠两份「更多」。

| 契约 | 规则 |
| --- | --- |
| 位置 | `F.frameMore()` 挂在 `.s9-frame-nav` 末尾，**只在桌面出现**；`#board[data-view='mobile']` 整条 `.s9-frame-nav` 退场（六个一级入口与「更多」一起隐藏），移动端顶栏只剩品牌 + 搜索 icon + 设置 icon。dock「更多」是移动端唯一的目录入口 |
| 移动端分工 | **【必须】** `.s9-frame-search`（`data-act="palette"` + `aria-label="打开全站搜索"` + `F.icon('search')`）桌面隐藏、仅 `#board[data-view='mobile']` 显示；点击走 `S9.openPalette()` 的屏内分支打开 `#mobile-search`（术语 / 简语 / 电码 / 公式 / 题库 / 页面法规五组静态内容 + 页面）。dock「更多」打开 184 条路由目录 Sheet。顶栏搜索 icon 与 dock「更多」不共用触发者、不共用弹层 |
| 数据 | `S9.ROUTES` 是 `crates/core/src/registry.rs::MODULES` 的只读快照（`[path, title, groupKey]`），当前 184 条 / 17 个功能分组 + 5 条直链；分组名、顺序与落板兜底写在 `MORE_GROUPS`。改注册表必须重生成快照，§9 一键脚本逐条比对路径集合 |
| 落板 | `routeBoard()` 先按真实路径在 `S9.pages[].route` 里找设计板，找不到按功能分组兜底；每条路由都能点，且 `data-board` 必须是已注册板 id |
| 面板（桌面） | `F.frameMorePanel()` 走 `F.pkPanel()`，是 Picker 的宽版变体：挂 `.s9-pk-layer`、`--r3`、`--z-popover`、`S9.presence` 卸载；`S9.pkPlace()` 的 `data-pk-kind="nav"` 分支取**整条顶栏**的 rect —— 左沿与顶栏版心对齐、宽度随顶栏、6px 间隙从顶栏下沿算，高度上限 `min(620px, 视口−顶栏下沿−16px)` |
| 面板（移动端） | **【必须】** `#board[data-view='mobile']` 下 `S9.pkMount()` 把同一个面板挂到 `.s9-mobile-shell`（不是 body 层），`S9.pkPlace()` 的 `nav` 分支清空内联几何、由 CSS 用 `position: absolute; inset: 26px 10px 10px` 铺满手机屏，`z-index: var(--z-demo-dialog)` 盖过 dock，`transform-origin: center bottom`、退场 `translateY(16px)`。没有 `F.frame` 的 12 张规格 / 组件板由 `S9.render()` 在手机壳里补一份 `.s9-more-portal` 面板；`S9.moreToggle()` 直接以 dock 按钮为触发者（顶栏已无触发器），归位容器取 `.s9-frame-more` 或 `.s9-more-portal`，保证 30 张板行为一致 |
| 分区滚动 | **【必须】** 面板是 `display: flex; flex-direction: column; overflow: hidden`：`.s9-frame-more-head`（标题 / 计数 / 筛选框 / ⌘K）与 `.s9-frame-more-foot`（说明 / 按钮）`flex: none` 常驻不滚；只有 `.s9-frame-more-body`（`flex: 1; min-height: 0; overflow-y: auto; overscroll-behavior: contain`）滚动。滚动容器必须**包住** `columns: 4` 的 grid，而不是给 grid 自身限高 —— 分栏一旦被约束高度就会横向长出新列 |
| 内容 | 18 个分组块（`role="group"`）`columns: 4` 排布，块头是「段名 / 组名 + 条数」；每条路由一行 `path` + `title`（`role="option"`），**184 条一条不少**；头尾各留一个 ⌘K 入口 |
| 筛选 | `F.input` 的 `data-more-search` 按名称 / 路径 / 分组就地过滤，隐藏行不卸载；整组无命中才隐藏组块，全无命中显示 `NO SIGNAL`；换关键词后把中间区 `scrollTop` 归零，高度变化后重调 `S9.pkPlace()` |
| 当前项 | 当前设计板 `route` 里的路径对应行 `data-on="1"` + `aria-current="page"` + `aria-selected="true"` + 「当前」文字标签（视觉 / 文字 / ARIA 三通道）；当前板不在六个一级入口时触发器 `data-on="1"` + `aria-current="true"`，沿用主导航的 2px 行动色下边线 |
| 触发器 | 桌面顶栏 `aria-haspopup` + `aria-expanded` + `aria-controls` 三件套；展开时 chevron 翻 180°、下边线换行动色。dock「更多」是移动端唯一触发器：`aria-haspopup="dialog"` + `aria-expanded`，`S9.moreToggle()` 置位、`S9.pkCloseAll()` 统一落回 `false`；收起时焦点还给 dock 按钮 |
| 键盘 | 复用 Picker 游标：↑↓ / Home / End 移 `data-active` 并同步 `aria-activedescendant`，Enter / Space 激活，Tab 收起，Esc 由 `S9.closeAll()` 收起；**焦点在筛选框里时方向键 / Enter 归还输入控件**，隐藏行不进游标。`S9.pkMounted()` / `S9.pkPanelFor()` / `S9.pkLayerSweep()` 的挂载点口径必须同时包含 `.s9-pk-layer` 与 `.s9-mobile-shell > .s9-pk-panel`，否则 Esc / 换板会漏掉屏内面板 |
| 关闭 | 点条目换板、点面板外、Esc、× 关闭键（`data-act="frame-more-close"`）、dock「更多」再次点击都收敛到 `S9.pkCloseAll()`；点面板内不能被当成「点外部」提前收起。屏内 Sheet 展开期带 `role="dialog"` + `aria-modal`，收起后移除模态属性并把焦点还给触发者；关闭态 `pointer-events: none` |
| 窄屏 | `width < 900px` 四列收成两列，`width < 480px` 收成单列且真实窄屏隐藏六个一级按钮（保留桌面「更多」）；断点只取 §3 登记的五档。**手机预览不按窗口宽度**：`#board[data-view='mobile']` 用容器选择器隐藏整条导航、显示 `.s9-frame-search`、强制 `columns: 1`、头部换行（筛选框独占一行、计数与 ⌘K 收起、× 显示）、页脚只留说明 |

---

## 5. 命名与结构规则

### 5.1 CSS 类名

**【必须】**

- 前缀 `s9-`，小写 + 连字符；块 `s9-panel`，元素 `s9-panel-body`（`-` 后缀，不用 `__`），修饰符 `s9-panel--inset` / `s9-chip--sm`。
- 页面专属类放在 CSS 的「页面专属」分组内，不污染通用组件区。
- 同名排版类与对应工厂等价（`F.label` ↔ `.s9-label`）—— 样式只在 CSS 定义一次，引用多少次都不算重复实现。

### 5.2 `data-*` 状态词表

**【必须】**

| 用途 | 允许值 | 说明 |
| --- | --- | --- |
| 开关量 | `data-on="1" / "0"`、`data-open`、`data-checked`、`data-selected`、`data-expanded`、`data-hit` | 布尔用 `1/0` |
| 关闭态 | `data-state="closed"` | **唯一写法**；桌面浮层、Picker、Toast 统一小写 |
| 控件态 | `data-state="on" / "off" / "mixed" / "error" / "disabled" / "hover" / "active" / "focus" / "selected" / "uploading" / "done" / "idle" / "blink"` | 全小写 slug |
| 状态机（显示文案） | `IDLE` / `SCANNING` / `LOCKED` / `NO SIGNAL` | 仅用于**面板显示文字**；写入 DOM 前经 `S9.stateToken()` 转小写 |

- `data-act` 用「域-动作」：`pk-toggle`、`kit-check`、`dir-item`、`fab-close`、`theme`、`go`；新动作在 `S9.onAction` 的 `switch` 里加分支。
- `data-*` 键在 JS 里写 camelCase，由 `dataAttrs()` 转 kebab-case（`pkDecimal` → `data-pk-decimal`）；**不要**手写 `data-pkdecimal`。
- 禁止 `CLOSED` / `Closed` / `closed` 混用，禁止新增大写 DOM 状态值。

### 5.3 JS 命名空间

| 前缀 | 职责 |
| --- | --- |
| `S9.state` | 全局状态（主题、风格、日期时间、开关量…） |
| `S9.motion` | 纯 JS 计时常量（§3.4） |
| `S9.presence` | 弹层卸载延迟唯一实现 |
| `S9.stateToken` | 状态机显示值 → DOM slug |
| `S9.SCALE` | 11 档数据色阶（唯一登记处） |
| `S9.pages` / `S9.B.<id>` | 板注册表 / 板函数 |
| `S9.frag` / `F.<name>` | 组件工厂 |
| `S9.onAction(act, el, ev)` | 所有 `data-act` 的唯一分发入口 |
| `ICON_PATHS` | 语义图标路径表 |

### 5.4 图标与文案

**【必须】** 语义图标走 `ICON_PATHS` + `F.iconSvg(name)` / `F.icon(name, opt)`；装饰性 SVG 带 `aria-hidden="true"`。

**【推荐】** 设计板内文案用 `S9.t(key)` + `S9.dict`（zh / en / es 三语齐全）；面向用户的短标签、注释可直接写中文，但**同一概念只用一个词**。

> 产品代码的文案走 `t("域.词条")` 与 `docs/data/i18n.md`，与本设计板的 `S9.dict` 是两套范围，不要互相引用。

---

## 6. 交互与可访问性要求

### 6.1 状态矩阵（现行口径）

| 状态 | 组件 | 时长 | 缓动 | 视觉响应 |
| --- | --- | --- | --- | --- |
| HOVER | Button / Chip / Option / 目录项 / 图标按钮 | 120ms 位移 · 180ms 换色 | `--ease` | 箭头右移 2–3px、描边提亮 |
| ACTIVE | Button / Chip / 开关 / 折叠头 | 120ms | `--ease` | 整体下沉 2px |
| FOCUS | 全部可交互控件 | 180ms | `--ease` | `outline: 2px dashed var(--blue)` + `outline-offset: 2px` |
| SELECTED | Checkbox / Radio / Chip / 选项 / 表格行 / 页签 | 180ms | `--ease` | 勾 / 圆点 `--ease-pop` 微弹，橙左条滑出 |
| EXPANDED | 目录分组 / 下拉 / 抽屉子类 / 折叠块 | 320ms | `--ease` | chevron 旋转、FAB 三横线 → × |
| DISABLED | 全部控件 | 180ms | `--ease` | 图标 40% 不透明，去饱和 |
| ENTER / EXIT 弹层 | 面板 / 遮罩 / Toast / 气泡 / Sheet / Drawer / Modal | **180ms 进 · 120ms 出** | `--ease` | `data-state="closed"` 驱动，`presence` 卸载，关闭态不拦截点击 |
| PICKER 浮层 | Select / DatePicker / TimePicker / Popover | 180ms 进 · 120ms 出 | `--ease` | 挂 `body` 避裁切 |
| LIVE 波形 | 实时传播数据柱 | 2400ms 循环 | `ease-in-out` | 负相位行波，只跑 `transform`，滚出视口暂停 |
| TOWER 主视觉 | 首页发射塔场景（电波 / 卫星 / EME / 月盘 / 极光 / 山峦 / 网格） | 4200 / 5000 / 7000 / 9000 / 18000ms 循环 | `--ease` / `--ease-out` | 整套复刻 modern UI 首屏 · 环波负相位扩散 · 各循环层滚出视口暂停、REDUCE 静止 |
| ICON | 全部语义图标 | `--ico 20px` | 1.8px 描边 / 24 网格 | 指示灯、分镜微缩不在此列 |

**【必须】** 循环动画滚出视口要停（`data-offscreen` / `data-live-off` → `animation-play-state: paused`），打印时 `animation: none`。

### 6.2 弹层进出场标准流程

```text
打开：clearTimeout(closeTimer) → 清 data-state → 显示 → 写入 aria-expanded="true"
      → （Picker）S9.pkMount 挂到 .s9-pk-layer 并 S9.pkPlace 定位
关闭：设 data-state="closed" + pointer-events:none → 同步 aria-expanded="false"
      → S9.presence.close(el, done) 播完 120ms 再卸载 / 归位
重开：clearTimeout(closeTimer) 取消卸载 + 清 data-state，直接淡入，不闪
```

**【必须】** 关闭路径统一收敛到 `S9.closeAll()`（`Esc`、点遮罩、`data-act="close"`、切板都走它）；打开前先关 Picker（§4.4）。

### 6.3 键盘与焦点

| 要求 | 规则 |
| --- | --- |
| 焦点环 | 元素外 2px 虚线 `--blue`，容器留 ≥4px 余量；不用 `outline: none` 抹掉 |
| 模态 | `$('#modal').setAttribute('tabindex','-1')` 后 `.focus()`；关闭归还触发者 |
| 全局键 | `Esc` 关闭全部浮层；`⌘K` 打开调谐台；`M` 切移动端预览；`↑↓` 在结果列表移动；`1–4` 选项 |
| 焦点保持 | `S9.render()` 全量替换 `#board` 前，按 `data-*` 身份串找回同一控件（排除 `state` / `on` 等易变键），避免键盘操作被打断 |
| 关闭态 | 一律 `pointer-events: none`，不拦截点击 |

### 6.4 ARIA 契约

| 场景 | 必填 |
| --- | --- |
| 图标按钮 / 无可见标签控件 | `aria-label` |
| 表单错误 | `aria-invalid="true"`（唯一入口） |
| 弹层触发器 | `aria-haspopup` + `aria-expanded` + `aria-controls`；Select 另加 `role="combobox"` |
| 选项 | `role="option"` + `aria-selected`；`role="listbox"` 容器 |
| 日期 / 时间面板 | `role="dialog"` + `aria-label`；日历格 `aria-pressed` + 完整日期 `aria-label` |
| 开关 / 复选 / 单选 | `role="switch"` + `aria-checked`；`aria-checked="mixed"` 表半选 |
| 滑块 | `role="slider"` + `aria-valuenow/min/max` |
| 表格排序 | `aria-sort`；`data-act="…-sort"` 时同步 |
| 导航 / 当前项 | `role="navigation"` / `aria-current` |
| 模态 | `role="dialog"` + `aria-modal` |
| 装饰性 SVG / 图标 | `aria-hidden="true"` |

**【必须】** 状态变化要同步「视觉 + 文字 + ARIA」三通道；颜色不单独承载语义，灰度模式下仍可辨认。

### 6.5 对比度（WCAG AA）

**【必须】** 正文 ≥ 4.5:1，图形 / 控件边界 ≥ 3:1。现行实测：

| 组合 | 暗色 | 亮色 |
| --- | --- | --- |
| 正文 / 卡面 | 17.66:1 | 16.71:1 |
| 正文 / 页面底 | 18.52:1 | 14.87:1 |
| 次要文字 / 页面底 | 6.54:1 | 4.99:1 |
| 行动色文字 / 卡面 | 6.83:1 | 4.91:1 |
| 按钮文字 / 行动色底 | 7.03:1 | 5.18:1 |
| 焦点环 / 卡面 | 6.36:1 | 5.52:1 |

边框属装饰层级（亮色约 1.6:1），控件辨识由 ≥5.5:1 的焦点环与底色 / 文字共同保障，**不靠加重描边**。

### 6.6 降级

| 条件 | 行为 |
| --- | --- |
| `S9.isReduced()`（`data-reduce=1` 或系统 `prefers-reduced-motion: reduce`） | `--dur*` 归 1ms；`presence.exitMs() = 0`，弹层立即隐藏卸载；循环动画 `animation: none` |
| `data-anim='off'` | 首页首帧与循环动画为静态终态 |
| `@media print` | 浮层 / Toast / Picker 层 `display:none`；屏幕用淡灰底区分，不补描边；动画全关 |

---

## 7. 常见场景：正例 / 反例

以下反例均来自本设计板已经修掉的真实问题，**不要重新引入**。

### 场景 1 · 表单输入

❌ 页面里手写原生控件（含浏览器原生下拉）：

```html
<div class="s9-field"><label>执照等级</label><select class="s9-input s9-select">…</select></div>
```

✅ 全部由工厂生成，`Select` 与 `Select`/`SimpleSelect` 共用弹层引擎：

```js
${F.field({
  id: 'kb-level', label: '执照等级', headExtra: 'SELECT',
  control: F.select({ id: 'kb-level', invalid: true, value: '请选择等级',
    options: ['请选择等级', 'A 类', 'B 类', 'C 类'] }),
  error: '请选择执照等级 · 红描边由 aria-invalid 驱动',
})}
```

> 为什么：`F.field` 保证 `for`/`id` 配对；`F.select` 是 `role="combobox"` 的弹层组件，不是系统下拉；错误态只认 `aria-invalid`。

### 场景 2 · 日期与时间选择

❌ 固定写死 42 格、无年份入口、时分共用一个滚动容器、字符箭头当图标。
✅ `F.pkCalendar()`（年月导航 + 12 年一页年份视图，按 `new Date(y,m,1)` 重算溢出日，回填 `YYYY-MM-DD`）、`F.pkClock()`（时 24 / 分 12 各自 `overflow-y:auto` + `overscroll-behavior: contain`）、触发器用 `F.iconSvg('calendar')` / `F.iconSvg('clock')`。

> 打开时把两个选中项分别滚到各自列中央；点选只在该列内切 `data-on` 并回填 `HH:MM`。

### 场景 3 · 图标

❌ CSS 伪元素画 `content: '▾'`，或页面里手拼 `<svg><path d="…">`。
✅ `F.icon('chevron', { chevron: true })` / `F.iconSvg('chevronLeft')`，路径登记在 `ICON_PATHS`，装饰性图标带 `aria-hidden="true"`。

### 场景 4 · 面板层级

❌ `.s9-panel` border + `.s9-panel--inset` 的 `box-shadow: inset 0 0 0 1px` + `.s9-scope` 自己的 border（三层线）。
✅ 只保留容器一层 `border`；屏幕靠 `--screen-bg` + `--screen-shadow` 柔和内凹；`PEAK` 标签改实心行动色块而不是橙框。

### 场景 5 · 弹出面板定位

❌ 绝对定位贴在触发器下，`top: calc(100% - 1px)`、下方两角圆角、被祖先 `overflow:hidden` 裁掉。
✅ 挂 `body` 的 `.s9-pk-layer`（`--z-popover`），按 `getBoundingClientRect()` 定位、留 6px 间隙、四角 `--r3`、空间不足上翻、滚动 / resize 重定位。

### 场景 6 · 关闭动画

❌ `el.hidden = true` / `el.remove()`（直接消失，或「愣 120ms 再突然消失」）。
✅ `S9.presence.close(el, done)` + `data-state="closed"` + `pointer-events:none`；桌面四个浮层各自有 `closed` 规则（overlay 淡出、palette/modal `s9-pop-out`、drawer `s9-drawer-out` 右移淡出）；`hidePortals()` 清掉状态，重开不套用退场动画。

### 场景 7 · Toast

❌ `setTimeout(() => el.remove(), 1800)`，无退场；`s9-rise` 的 `transform` 还覆盖水平居中的 `translateX(-50%)`。
✅ `S9.toast()` 停留取 `S9.motion.toastDwell`，退场走 `presence`；`s9-toast-in` / `s9-toast-out` 用独立 `translate` / `scale`，不碰 `transform`。

### 场景 8 · 循环动画属性

❌ `animation` 改 `height` / `left`（每帧布局重排）；JS 每 3s 随机写 `style.height` 造成跳变；21 根柱子用正值 `animation-delay` 逐根排队启动。
✅ 纯 CSS `s9-live-wave` 行波只改 `transform: scaleY()`，按卡片 / 列算**负相位**；主视觉循环层只改 `transform` / `opacity` / `stroke-dashoffset`；JS 只轮换峰值卡与 UTC 时钟，不碰 `height`。

> 判据：循环动画不得触发逐帧布局；用 CDP `LayoutCount` 前后对比应接近基线。

### 场景 9 · 主题换肤

❌ 组件里写死 `#020306`（示波屏）、`rgba(111,140,255,.12)`（柱底）、`#100604`（FAB 图标）、`#fff`（卡面）；切主题调 `S9.render()` 重建 DOM。
✅ 全部取 `--screen-*` / `--bar-*` / `--on-accent` / `--surface`，亮色块覆盖；`case 'theme'` 只调 `S9.syncChrome()`，颜色走 `--tr-theme` 180ms。

### 场景 10 · 开关

❌ 刻度用 `repeating-linear-gradient(…5px)` 除不尽滑块宽度；ON 态只改外框 `border-color`。
✅ `F.switchButton` 生成 3 条 `<i>` 刻度 + `justify-content: space-evenly` 四等分；ON 态同时换外框、轨道底 `--wash`、拨片 `--accent`、刻度 `--on-accent`。

### 场景 11 · 七段读数

❌ `<span class="s9-seven-ghost">88.888</span>` + `<span class="s9-seven-value">14.200</span>` 坐标完全重叠，放大后重影毛边。
✅ `F.seven(value, opt)` 只输出一层；暗段质感由屏底与微光承担，`data-freq` / `data-exam-timer` 仍挂在这层上供 JS 就地更新。

### 场景 12 · 状态词表与裸值

❌ 桌面浮层 `data-state="CLOSED"` 与 Toast / Picker 的 `closed` 两套写法；`z-index: 21`；`animation: … 900ms`；11 档色阶数组复制两份。
✅ DOM 状态统一小写 slug（显示文案经 `S9.stateToken` 转大写）；层级取 `--z-*`；时长取 `--dur-*` / `S9.motion`；色阶只读 `S9.SCALE`。

### 场景 13 · 首页发射塔主视觉（跨体系复用）

❌ 设计板自造一套简笔塔：三角塔 + 两条橙色弧 + 频率刻度，与 modern UI 的完整场景（大圆底盘 / 电离层辉光 / 星点 / 卫星与下行波束 / EME 月面反射 / 月盘 / 远山 / 透视网格）完全不是一回事；**只把环波时长抄过来不算复用**。
✅ 整套复刻 `crates/app/src/kit_modern/home/hero.rs` 的 `viewBox="0 0 420 360"` SVG 与 `style/modern/kit/{01-base,03-atmosphere,05-menu}.css` 的动效：`md-*` 变量映射到 `s9-*` token，渐变 id 带序号避免打印栈副本重复；`F.tower()` 默认返回完整场景，`{ simple: true }` 保留简笔塔给窄屏预览板。**最外圈 / 虚线内圈 / 竖轴也画进同一张 SVG**，以场景中心 `(210,170)` 为同心点、`r=150` 完整落在画布内 —— 不用 CSS div 另画一圈（那样会与 SVG 缩放脱节、圆心错位 67px 并上下溢出到 header）。`s9-aurora` 完全落在主视觉容器内，不越界。

> 循环层：环波 4.2s 扩散（`--ease-out`、负相位错峰）、卫星 7s 漂浮、EME 5s / 7s 虚线流动、月盘 9s 呼吸、极光 18s 漂移；统一滚出视口暂停、REDUCE 静止、打印栈不参与。浅色按 modern UI 同档规则降辉光 / 提线稿对比。改 modern UI 主视觉时，本设计的 SVG、token、状态矩阵与 §3.4 / §6.1 要一起跟。

---

## 8. 可复制模板

### 8.1 一组表单

```js
${F.field({ id: 'call', label: '呼号 / CALLSIGN', headExtra: '必填',
  value: 'BA4ST', hint: '大写字母与数字，不含空格' })}
${F.field({ id: 'freq', label: '频率', headExtra: 'MHz',
  control: F.inputAddon({ id: 'freq', value: '14.200', addon: 'MHz' }),
  hint: '1.8 – 28 MHz · 等宽 tabular-nums' })}
${F.field({ id: 'note', label: '值机备注', headExtra: 'TEXTAREA',
  control: F.textarea({ id: 'note', placeholder: '记录通联情况…' }) })}
```

### 8.2 弹层式 Select（带分组）

```js
${F.select({
  target: 'band-select', value: '14.200 MHz · 20 m',
  groups: [
    { label: '常用波段', items: [
      ['7.050 MHz · 40 m', '', false],
      ['14.200 MHz · 20 m', 'checked', false],
      ['21.200 MHz · 15 m', '', false],
    ] },
    { label: '其他', items: [['430.000 MHz · UHF', '', true]] },
  ],
})}
```

### 8.3 日期 / 时间浮层

```js
${F.pkPop({
  target: 'pk-date', key: 'date', icon: 'calendar',
  text: `${S9.state.pkYear}-${pad(S9.state.pkMonth + 1)}-${pad(S9.state.pkDay)}`,
  triggerAttrs: 'aria-haspopup="dialog" aria-expanded="false"',
  panelAttrs: 'role="dialog" aria-label="选择日期"',
  content: F.pkCalendar(),
})}
${F.pkPop({
  target: 'pk-time', key: 'time', icon: 'clock',
  text: `${pad(S9.state.pkHour)}:${pad(S9.state.pkMinute)}`,
  triggerAttrs: 'aria-haspopup="dialog" aria-expanded="false"',
  panelAttrs: 'role="dialog" aria-label="选择时间"',
  content: F.pkClock(),
})}
```

> 只要由 `F.pkPop` 生成 `data-act="pk-toggle"` 触发器与面板，`S9.onAction` 会自动完成挂载、定位、开合、回填与退场，无需另写 JS。

### 8.4 新页面板

```js
// 1) 注册表加一行（sig 取已有页面签名）
S9.pages.push({ id: 'demo', group: '全局与组件', title: '示例板',
  route: '/demo', sig: 'board', note: '…' });

// 2) 板函数：S9.render 负责 F.head(p) 与 .s9-board-body 外壳
S9.B.demo = () => `
  ${F.frame('示例')}
  <div class="s9-page">
    ${F.panel('读数', '副标题', `
      ${F.readout('中心频率', '14.200', 'MHz')}
    `, { attrs: S9.attrI(1) })}
  </div>`;
```

### 8.5 新增动画

```css
/* :root 先取号，组件不写裸毫秒 */
--dur-new: 480ms;

.my-block { animation: s9-new var(--dur-new) var(--ease) both; }
@keyframes s9-new { from { opacity: 0; transform: scaleY(.96); } }
```

```js
// 纯 JS 计时登记到 S9.motion，不在逻辑里写数字
S9.motion.newDwell = 900;
```

> 循环动画：只改 `transform` / `opacity`；多元素用负相位；滚出视口加 `animation-play-state: paused`；打印与 REDUCE 归零。

### 8.6 新增状态

```js
// DOM 值小写 slug；显示文案大写仪器字经 stateToken 转换
el.dataset.state = 'selected';
label.textContent = 'SELECTED';
el.setAttribute('aria-pressed', 'true');   // 三通道里的 ARIA
// CSS 只认小写选择器
// .s9-chip[data-state='selected'] { … }
```

---

## 9. 验收清单

**提交前逐项打勾：**

- [ ] `【必须】` 语法：抽取 `<script>` 后 `node --check` 通过。

  ```bash
  python3 -c "s=open('s9-console-design.html',encoding='utf-8').read(); open('/tmp/s9-check.js','w',encoding='utf-8').write(s.split('<script>')[1].split('</script>')[0])" \
    && node --check /tmp/s9-check.js
  ```

- [ ] `【必须】` 源码扫描零裸值：无 `z-index: <数字>`、无 CSS 裸毫秒、无 `[data-state='<大写>']`、无硬编码 `--scrim` / `--shadow-pop` 值、无 `!important` 声明、无原生 `<select>` 标签、无 `repeating-linear-gradient` 刻度（一键脚本见下）。

- [ ] `【必须】` 单一来源：11 档色阶数组只有 `S9.SCALE` 一份；时长只在 `:root` / `S9.motion`；`CLOSED` 不出现在 DOM 状态赋值里。
- [ ] `【必须】` 全部页面板（`S9.pages.length`，当前 30）都渲染出非空 `.s9-page`，控制台 `0 errors` / `0 pageerror`。
- [ ] `【必须】` 全 `data-act` 逐一点一遍，0 报错；移动端 360px × 全部板无横向溢出；打印页数等于板数。
- [ ] `【必须】` 浮层：Picker 四角 `--r3`、与触发器 6px 间隙、不重叠、10 层祖先 `overflow:hidden` 仍可命中；四个桌面浮层关闭都播 120ms 退场且 `pointer-events:none`；REDUCE 立即卸载。
- [ ] `【必须】` 顶栏「更多」（§4.7）：`S9.ROUTES` 与注册表路径集合逐条一致、每条都能落板、筛选 / 清空 / 无命中三态、**滚动时头尾 rect 不动且只有 `.s9-frame-more-body` 的 `scrollTop` 变化、无横向溢出**、焦点在筛选框时方向键不被抢、↑↓ / Enter / Esc / 点面板外。
- [ ] `【必须】` 移动端「更多」（§4.7）：360 / 390 / 480px 三档下**顶栏没有「更多」**、只有 dock 最右那一个能打开**同一份 184 行**目录；面板挂 `.s9-mobile-shell` 且不进 body 层、单列、头尾固定在滚动时不动、无横向溢出、× / Esc 可收起且焦点归还 dock 按钮；30 张板（含 12 张无 `F.frame` 的规格板）逐一打开无报错、`#frame-more-panel` 在每板恰好一份。
- [ ] `【必须】` 移动端顶栏搜索 icon（§4.7）：三档下 `.s9-frame-nav` 为 `display:none`、`.s9-frame-search` 可见且有 `aria-label`，点击打开的是屏内 `#mobile-search`（全站搜索：静态内容 + 页面）而不是路由目录；桌面该 icon 隐藏、`⌘K` 键帽保留。
- [ ] `【必须】` 状态与表单：`Field` 的 `for`/`id` 全配对；错误态只由 `aria-invalid` 驱动；Select 选中后清除错误态与 `aria-invalid`；`aria-expanded` 随开合同步。
- [ ] `【必须】` 双主题：亮色非纯白、暗色近黑；遮罩 / 阴影 / FAB / 数据柱随主题换肤；主题切换不重建 DOM（打标记验证）、过渡 180ms。
- [ ] `【推荐】` 亮 / 暗关键区域截图目检；改动动效时按发布构建口径测量帧率与首帧。

**一键自检脚本**（在仓库根目录执行，可整段复制）：

```bash
python3 - <<'PY'
import re
s = open('docs/design/s9-console-design.html', encoding='utf-8').read()
css = re.sub(r'/\*.*?\*/', '', s.split('<style>')[1].split('</style>')[0], flags=re.S)
js = s.split('<script>')[1].split('</script>')[0]
assert not re.search(r'z-index:\s*\d', css), '裸 z-index'
assert not re.search(r'(animation|transition)[^;]*\b\d+(\.\d+)?m?s\b', css), '裸毫秒'
assert not re.search(r"\[data-state='[A-Z]", s), '大写 DOM 状态'
assert '!important' not in css and '<select ' not in js
assert 'repeating-linear-gradient' not in css

# 顶栏「更多」的路由快照必须与能力注册表逐条一致（§4.7）
reg = open('crates/core/src/registry.rs', encoding='utf-8').read()
mod = reg[reg.index('pub const MODULES'):reg.index('pub fn', reg.index('pub const MODULES'))]
reg_paths = set(re.findall(r'path:\s*"([^"]+)"', mod))
start = js.index('S9.ROUTES = [')
snap = js[start:js.index('];', start)]
html_paths = re.findall(r"\['([^']+)',\s*'[^']*',\s*'[A-Za-z]+'\]", snap)
assert len(html_paths) == len(set(html_paths)) == len(reg_paths), '路由快照条数 / 去重不一致'
assert set(html_paths) == reg_paths, '路由快照与注册表不一致：' + ','.join(sorted(reg_paths ^ set(html_paths))[:5])
print(f'源码扫描通过 · 路由快照 {len(html_paths)} 条与 registry.rs 一致')
PY
```

> 本地预览（在仓库根目录）：`python3 -m http.server 8765 --bind 127.0.0.1`，打开 `http://127.0.0.1:8765/docs/design/s9-console-design.html`。Playwright 可从 `e2e/node_modules` 加载（`@playwright/test`），断言覆盖上面的渲染 / 交互 / 层级 / 主题 / 移动端 / 打印项。

---

## 10. 更新与维护

### 10.1 单一来源清单

| 事实 | 唯一权威 | 其它地方 |
| --- | --- | --- |
| 主题色、层级、时长、遮罩、阴影、圆角、字体 | `:root` + `html[data-theme='light']` | 组件只 `var(--…)` |
| JS 计时 | `S9.motion` | `S9.presence.exitMs()` 引用 `exit` |
| 卸载延迟 | `S9.presence` | 四个浮层 / Picker / Toast 共用 |
| DOM 状态词表 | `S9.stateToken` + §5.2 | 显示文案可大写 |
| 控件实现 | `F.*` / `S9.frag` | 页面只调用 |
| 图标路径 | `ICON_PATHS` | `F.icon*` 引用 |
| 数据色阶 | `S9.SCALE` | `F.scaleSwatches` 引用 |
| 页面清单 | `S9.pages` | `F.head` / 打印栈 / 导航都读它 |
| 动作分发 | `S9.onAction` | `data-act` 只声明名字 |
| 规则全文 | **本文件** | 设计板 `kitall` 09 表是摘要 + 指针 |

### 10.2 改动同步矩阵

| 改了什么 | 必须同步 |
| --- | --- |
| 新增 / 改 token 值 | `:root`、亮色覆盖、`kitall` 09 契约表、本文档 §3、截图基准 |
| 新增 / 改动画档 | `:root` 或 `S9.motion`、`components` 状态矩阵表、本文档 §3.4 / §6.1 |
| 新增 / 改浮层 | `--z-*` 取号、`S9.closeAll` / `open*` 的 `CLOSED` 处理、`hidePortals` 清状态、本文档 §4.4 / §6.2 |
| 新增 / 删 `F.*` 工厂 | 工厂区、页面调用点、`kitall` 08 实时扫描表（自动）、本文档 §4.2 |
| 新增状态值 | `S9.stateToken` 映射（若来自状态机）、CSS 小写选择器、本文档 §5.2 |
| 新增页面板 | `S9.pages`、`S9.B.<id>`、页面签名、打印栈（自动）、本文档 §4.6 若规则变化 |
| 改顶栏「更多」路由目录 | `crates/core/src/registry.rs::MODULES` → `S9.ROUTES` 快照、`MORE_GROUPS`、`F.frameMore*`、`.s9-frame-more*` / `.s9-frame-search` 样式、`S9.pkPlace()` / `S9.pkMount()` / `S9.pkMounted()` 的 `nav` 分支、`S9.moreToggle()`、`.s9-more-portal`、§9 路径集合比对、本文档 §4.7 |
| 改主题 / 视觉语言 | `:root` + 亮色块、本文档 §3.1、`s9-console-ui-prompts.md` 的**意图描述**（不要求同步取值，见 §1.3） |
| 改 modern UI 首屏主视觉 | `F.tower()` 的 SVG、`--dur-ring` / `--dur-sat` / `--dur-eme` / `--dur-moon` / `--dur-aurora`、五条循环关键帧、`s9-hero-outer` / `s9-aurora` 样式、`components` 状态矩阵、本文档 §3.4 / §4.2 / §6.1 / §7 场景 13 |

### 10.3 新增流程

1. 先查 §4.2 / §0，确认没有同形态工厂与同语义 token。
2. 在**唯一来源**处登记（不是先写页面再补 token）。
3. 页面只调用工厂、只引用 token、只写小写 `data-*`。
4. 补齐三通道：视觉 + 文字 + ARIA（§6.4）。
5. 跑 §9 验收清单；涉及动效的补测帧率与降级。
6. 同一次改动里更新本文档与 `kitall` 09 摘要表 —— **改代码的同一次提交里改文档**。

### 10.4 修订方式

- 本文档只写「现在该怎么做」；不记录改前 / 改后快照、批次流水账、试错过程（那些写进提交说明）。被推翻的做法若仍有教训价值，抽成 §7 的一条反例，不保留时间线。
- 修订时在 §1.3 的裁决顺序内确认新事实源；改变边界 / 取舍时同时更新 §1.1、§1.4、§2 相关原则。
- 数字与清单尽量交给源码对账：工厂数、板数、色阶数、层级值都能从 `Object.keys(S9.frag)` / `S9.pages.length` / `S9.SCALE.length` / `getComputedStyle` 取到，不在文档里手数后长期不更新。

### 10.5 落点说明

本文件与 `s9-console-design.html` / `s9-console-ui-prompts.md` 同在 `docs/design/`，同 `s9-console-` 前缀，便于检索；索引在 `docs/README.md` 的「design / 设计稿」一节（`cargo make readme-check` 会校验每份 `.md` 都在索引里有一行）。

不放进 `docs/ui/`：`ui/` 只写**产品代码**（`crates/app/`）的现行规范，本规范约束的是独立的设计板，两者适用范围不同（§1.1），混放会与 `ui/modern.md` 互相套用取值。

---

## 附录 A · 符号索引（grep 用）

| 想找 | 搜索锚点 |
| --- | --- |
| 主题 token | `:root {`、`html[data-theme='light'] {`、`--tr-theme` |
| 层级 / 时长 | `--z-`、`--dur-`、`S9.motion = {` |
| 遮罩 / 阴影 | `--scrim`、`--shadow-pop`、`--fab-shadow` |
| 运行时契约 | `S9.presence = {`、`S9.stateToken =`、`S9.SCALE =` |
| 组件工厂 | `const F = S9.frag`、`^F\.`、`dataAttrs` |
| 图标 | `ICON_PATHS`、`F.iconSvg` |
| 页面结构 | `S9.pages = [`、`S9.B = {}`、`S9.render = (`、`F.head = ` |
| 顶栏「更多」路由目录 | `S9.ROUTES = [`、`MORE_GROUPS = [`、`F.frameMore = `、`data-pk-kind="nav"`、`S9.pkPlace = (`、`data-more-search`、`S9.moreToggle = (`、`.s9-more-portal`、`#board[data-view='mobile'] .s9-mobile-shell > .s9-pk-panel` |
| 移动端顶栏搜索 icon | `.s9-frame-search`、`F.icon('search')`、`#board[data-view='mobile'] .s9-frame-nav`、`#mobile-search` |
| 动作分发 | `S9.onAction = `、`case 'pk-toggle'` |
| 浮层 | `S9.closeAll = (`、`S9.hidePortals = (`、`S9.pkLayer = `、`S9.pkMount = ` |
| 动效表 / 契约表 | `09 统一化契约`、`交互状态 × 过渡口径` |
| 塔台主视觉（对齐 modern UI） | `F.tower`、`s9-hero-outer` / `s9-aurora` / `s9-ring-out` / `s9-sat` / `s9-eme` / `s9-moon-disc`、`--dur-*`；来源 `kit_modern/home/hero.rs` 与 `style/modern/kit/{01-base,03-atmosphere,05-menu}.css` |
| 无障碍 | `aria-invalid`、`focus-visible`、`setAttribute('tabindex'` |
| 降级 / 打印 | `html[data-reduce='1']`、`data-anim='off'`、`@media print` |
