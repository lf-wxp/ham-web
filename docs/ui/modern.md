# 现代 UI 规范（Modern）

> 本站有**两套设计系统**：本文件是**现代风（Modern）**的规范；像素风见 [`docs/ui/pixel.md`](./pixel.md)，
> 设计取舍与视觉语言的来历见 [`docs/ui/decisions.md`](./decisions.md)
> （**不是规范**；冲突时以本文件为准）。
>
> 现代风的类名一律 `md-*`、变量一律 `--md-*`，全部落在
> `crates/app/src/kit_modern/`（组件）与 `crates/app/style/modern/`（样式）。
> 两套系统**不共享类名、不共享 token、不共享 DOM 结构**，唯一共享的是 `crates/app/src/ui/`
> 里那层「按风格分叉」的判断。
>
> 从哪读起：改取值看 §2；改布局看 §3；写组件看 §4；写页面看 §5；
> 收工前过 §7 的门禁。

## 目录

1. 边界：什么算现代风
2. 取值（tokens）
3. 布局与页面骨架
4. 组件目录
5. 写页面的规则
6. 无障碍（含 6.5 浏览器基线与 `:has()` 的写法）
7. 门禁与自检
8. 反例清单

---

## 1. 边界：什么算现代风

| 归属 | 目录 | 类名 | 变量 |
|---|---|---|---|
| 现代风 | `crates/app/src/kit_modern/`、`crates/app/style/modern/` | `md-*` | `--md-*` |
| 像素风 | `crates/app/src/components/`、`crates/app/style/pixel/` | `pxl-*` / 工具类 | `--*`（无前缀） |
| 共用入口 | `crates/app/src/ui/` | 按风格分叉 | — |

三条硬边界（`check-structure` 机械守着）：

1. `kit_modern/` 与 `style/modern/` 里**不许出现 `pxl-`**（双系统隔离）；
2. 页面上「只在像素风渲染」的文件里**不许出现 `md-*`**（反向隔离）；
3. `style/modern/` 的**层外规则**必须带 `md-` 或 `html[data-ui-style="modern"]` 门禁
   （层外优先级高于工具类，漏门禁会污染像素风）。

页面里判断风格的唯一入口是 `crate::kit::is_modern()` / `is_modern_at_mount()`；
**不要自己读 `ui:uiStyle` 或 `data-ui-style`**（唯一例外是 `theme.rs` 内部的
`DisplayPrefs::style_is_modern_dom()`，供根 Owner 之外的闭包使用，例如 tooltip 气泡）。
原因有两个：`use_display_prefs()` 是 `expect_context`，在根 Owner 之外没有上下文，会直接 panic；
路由兜底里渲染的 404 页拿到的 `DisplayPrefs` 是**默认实例**（不是页面树里的那个），`is_modern()` 一律返回像素，
所以 `pages/not_found.rs` 也读它。

---

## 2. 取值（tokens）

全部定义在 `crates/app/style/modern/tokens.css`。**页面里不要写死数值**，取变量。

### 2.1 颜色（只有五个角色）

| 角色 | 变量 | 浅色 | 深色 | 用法 |
|---|---|---|---|---|
| 页面底 | `--md-bg` | `#f4f3ee` | `#04050a` | 整页背景 |
| 表面 | `--md-surface` / `-2` / `-3` | `#fffefb` / `#ecebe4` / `#e0dfd6` | `#0b0d13` / `#12151f` / `#1b2030` | 卡片 / 输入 / 凹陷 |
| 文字 | `--md-fg` / `--md-fg-muted` | `#0a0b0d` / `#565b66` | `#f2f4fa` / `#8a92a6` | 正文 / 次要 |
| 线 | `--md-border` / `--md-border-strong` / `--md-rule` | `#e2e0d7` / `#c3c1b6` / 18% 前景 | `#222839` / `#3a4460` | 描边 / 悬停描边 / 分隔线 |
| 行动 | `--md-accent`（`= --md-orange` `#ff4d1f`）/ `--md-accent-ink` / `--md-accent-line` | — | `--md-accent-ink` `#ff8a63` | **实心只给全页唯一的主行动**；文字（`-ink`）、描边 / 骨架（`-line`）、浅底（`-wash`）各一档，品牌橙本身在浅色底上做文字对比度不够，文字一律走 `-ink` |

语义色：`--md-blue`（**选中**）、`--md-green`（成功）、`--md-violet`（点缀）、`--md-ring`（焦点环）。

> 规矩：**蓝 = 选中，橙 = 行动**。任何「当前项 / 已勾选 / 已订阅」都用蓝，
> 任何「去做什么」的按钮用橙。这条在复选框、胶囊、导航、页签、下拉选中项上一致。

#### 状态色（危险 / 警示 / 金）：按承载面取，组件里不写字面色值

红 / 琥珀 / 金不是一个颜色，而是**每个承载面一个颜色**。浅色底上能当文字用的红是深砖红，能当描边用的红更亮，
实心按钮底又是另一个 —— 写死一个 `#ff7a7a` 到处用，深色下好看，浅色下红字只有 2.1:1、金字 1.2:1。
所以组件只引用 token，字面色值只许出现在 `tokens.css`（`check-structure` 的「状态色字面值」守着）：

| 承载面 | token | 浅色 | 深色 | 最低对比度 |
|---|---|---|---|---|
| 图形：描边、`color-mix` 浅底、渐变端点 | `--md-danger` | `#d62f2f` | `#ff5d5d` | 3:1（非文本） |
| 文字：错误提示、判错标记、告急倒计时 | `--md-danger-ink` | `#9b1c14` | `#ff7a7a` | 4.5:1，**含压在 22% 红底上的最差情形** |
| 实心按钮 | `--md-danger-fill` / `-hover` / `-fill-fg` | `#d93b3b` / `#bf2f2f` / 白 | 同左 | 4.5:1 |
| 小块实心标记（题号块、错误角标） | `--md-danger-face` / `-face-fg` | = `fill` / 白 | `#ff5d5d` / `#1a0808` | 4.5:1 |
| 琥珀：文字 / 实心旗标 | `--md-warn-ink` / `--md-warn-fill` + `-fill-fg` | `#8a5200` / `#ffb02e` + `#241a00` | `#ffb02e` / 同左 | 4.5:1 |
| 金：图形（辉光、星标填充）/ 文字（星标描线、连击） | `--md-gold` / `--md-gold-ink` | `#c98a00` / `#7f5400` | 均 `#ffcf6b` | 图形 3:1 / 文字 4.5:1 |
| 纹章（emblem）图标前景 | `--md-blue-ink` / `-violet-ink` / `-green-ink` | 各自的深色（= 对应主色） | `#8fb4ff` / `#b79dff` / `#7fe3a8` | 3:1（图形） |
| 绿色实心面上的字 | `--md-green-fg` | 白 | `#08130c` | 4.5:1 |

- 深色那一列就是改造前写死在组件里的数值，**深色外观基本不变**，只是收回到 token
  （同一语义的几个相近色值合并成了一个：`#ff6b6b` / `#ff8a8a` / `#ff9a9a` 等并入 `--md-danger-ink` 或 `--md-danger`）。
- 选 token 看「这块颜色画在什么上」：文字用 `-ink`，描边 / 浅底 / 渐变用基色，实心面用 `-fill` / `-face` 并配它的 `-fg`。
  不要拿 `-ink` 当描边、也不要拿基色当文字 —— 两者在浅色下差着一档对比度。
- 新增一个状态色时，在 `tokens.css` 的浅 / 深两个块里同时声明，并把它加进
  `e2e/tests/modern_a11y.spec.ts` 的「状态色 token 的对比度」（它直接量 token 对页面四个真实底面的对比度，浅深各一遍）。
- 品牌橙的实心面（焦点卡的 `--md-accent-face-*`）是明暗共用的有意字面值，在行尾写 `/* color-ok: 理由 */`。

### 2.2 字体（三个声部）

| 变量 | 字族 | 用途 |
|---|---|---|
| `--md-font` | Geist + 中文系统黑体（HarmonyOS Sans / PingFang / 雅黑回退） | 正文、控件 |
| `--md-font-display` | Bricolage Grotesque + Noto Sans SC Display（中文展示字） | 标题、大号读数 |
| `--md-font-mono` | IBM Plex Mono | 频率、呼号、坐标、代码 |

正文**不压负字距**（中文会糊）。展示层的字距按类写在各规则里，不是一个统一区间：页头标题 `.md-article-title` 为 0，
`.md-display` 为 `-0.01em`，大号读数与序号收得更紧（`.md-strip-value` `-0.045em`、`.md-mod-index` `-0.06em`）。
新增展示类先对照同类现有规则取值，别凭印象。

**中文展示字是分片的**：`Noto Sans SC Display`（900）按 `unicode-range` 切成 `fonts-cjk.css` 里的若干片，第 0 片只含展示层实际用到的汉字
（约 700 字）。新增大量中文标题后，语料外的字会逐字回落到系统黑体 —— PingFang 最高 Semibold、雅黑最高 Bold，给不出 900，
标题里个别字的字重会掉下来。重新生成的步骤写在 `fonts-cjk.css` 文件头（仓库里没有生成工具）。

**换展示 / 等宽字族时，同步改回落字体的度量覆盖**：`fonts.css` 里的 `Bricolage Fallback` / `Plex Mono Fallback` 用
`size-adjust` / `ascent-override` / `descent-override` 让「字体还没到」时的行盒与最终一致，否则 `font-display: swap` 换字时标题会上下跳；
数值的算法写在该处注释里。

### 2.3 字号阶梯

```
3xs 10 · 2xs 11 · xs 12 · sm 13 · compact 15(≤640) / 16 · base 16 · md 17 · lede 19 · lg 22 · xl 24 · 2xl 28
display-4 clamp(22,2.2vw,26) · -3 clamp(24,3vw,34) · -2 clamp(30,4.2vw,52) · -1 clamp(36,5.6vw,72)
prose clamp(15,0.36vw+13.4,17.5) · measure 36em（阅读正文，见下）
```

控件内文字固定 `--md-text-sm`（13），字段标签 / 提示 / 错误固定 `--md-text-xs`（12）。

**正文分两档，按「这一屏在做什么」选，不按长度选：**

| 档 | 变量 | 值 | 用在哪 |
|---|---|---|---|
| 界面正文 | `--md-text-base` | 16px 固定 | 卡片、表单、列表、控件周边 —— 屏幕上还夹着可点的东西 |
| 阅读正文 | `--md-text-prose` | `clamp(15px, 0.36vw + 13.4px, 17.5px)` | `.md-article-page` 的子树：知识库文章、参考页、术语表、收藏页 —— 一屏以成段阅读为主 |

- **阅读正文随视口连续变化，不用断点**：断点处字号一跳，整段换行位置跟着跳（30 字 / 行 → 22 字 / 行），
  读者会觉得「换了个页面」。`clamp` 把 360 → 15px、768 → 16.2px、1024 → 17.1px、≥1200 → 17.5px 拉成一条曲线。
- **行宽按 `em` 限，不按 `px` / `ch`**：`--md-measure: 36em`（36 个汉字 / 行，西文约 62 字符），
  成段文本（`p` / `li` / `dd` / `blockquote`）都带这个 `max-width`。用 `em` 是因为字号在变 ——
  行宽跟着字号缩放，**每行字数**才恒定；`ch` 是「0」的宽度（Geist 约 0.66em），中文方块字是 1em，68ch 会变成 45 字 / 行。
- **阅读正文的行高是 1.75**（界面的 1.6 只适合短文案，长段落回行会串行）；
  表格、卡片标题、徽标、按钮**不跟**这一档 —— 它们是块，不是段落。
- 相对档只允许出现在正文族内部（如 `.md-concept dd` 的 `0.9em`）：术语要大一号、释义要小一号，
  但两者都得跟着正文一起放大，写死 13px 会在桌面版留下一行比正文小 4px 的释义。

守卫：`cargo make check-structure` 的「阅读正文规范」逐项核对 —— 两个 token 的形态（`clamp(…vw…)` / `em`）、
`.md-article-page` 是否真的落了它们、以及本文档表格里的 clamp 值与 CSS 是否逐字一致。
回归用例：`e2e/tests/modern_interact.spec.ts` 的「现代风阅读页（流体正文）」（360 / 1440 两档量字号单调递增与每行字数）。

### 2.4 间距 / 圆角 / 阴影 / 动效

```
space   2 4 6 8 10 12 14 16 20 24 32 48 64（对应 --md-space-0-5 … -16）
radius  xs 4 · sm 6 · ctl 8 · md 10 · lg 10 · xl 14 · pill 999
shadow  1 静止卡片 · 2 悬浮态 · 3 弹层（对话框 / 抽屉 / 气泡）
dur     fast 120ms · base 180ms · slow 320ms
ease    --md-ease（标准）· --md-ease-out（入场）
```

### 2.5 控件尺寸（一行里只允许这几种高度）

| 变量 | 值 | 用在哪 |
|---|---|---|
| `--md-control-h` | **40px** | 输入 / 下拉 / 文本域 / 默认按钮 |
| `--md-control-h-sm` | **30px** | 小号按钮、`.md-chip-sm` |
| `--md-control-r` | 8px | 控件圆角 |
| `--md-control-pad` | 12px | 控件左内距 |

紧凑场景用 `.md-input-sm`（32px，内联在工具条里的小字段），不要用 `h-7` 压抠。

---

另有两组 token 把常用的内距与掺色比例收成了名字，新增组件优先用名字、不写裸数字：
`--md-pad-card`（卡片内距，= `--md-space-6`）、`--md-pad-block`（= `--md-space-4`）、`--md-pad-inline`（= `--md-space-2-5`）；
`--md-tint-weak` / `-soft` / `-strong`（`color-mix` 的掺色比例 8% / 14% / 22%）。

### 2.6 手机与触屏的底线

这些是 `kit.css` 文件末尾的「窄屏 / 触屏追加约束」，桌面版式不受影响：

- **≤640px 时 `.md-input` / `.md-select` / `.md-textarea` 的字号提到 16px**（`--md-text-base`）：iOS Safari 在字号小于 16px 的表单控件
  获得焦点时会把整页放大一次，用户得手动缩回去。桌面的 13px 不变。
- **`pointer: coarse` 下命中区抬档**：`.md-input` / `.md-btn` / `.md-icon-btn` / `.md-text-btn` 最小 44px；贴着输入框右缘的清除按钮、
  步进按钮、`.md-icon-btn-xs` 与 `.md-select-sm` 抬到 32px（再大会吃掉字段右侧内距）；≤900px 的筛选胶囊 `.md-chip` 抬到 44px。
  **嵌在字段里的胶囊不跟着抬**（`.md-menu-search .md-chip` 收到 32）：目录搜索框是 40px 的字段，44 的胶囊会从上下边框各顶出 2px。
  WCAG 2.5.8 的下限是 24×24，20×20 明确不达标。
- **中文界面把最小两档字号各抬 1px**：`--md-text-3xs` 10→11、`--md-text-2xs` 11→12（`tokens.css` 的 `[lang^="zh"]` 分支）。
  这两档是按西文 kicker 定的，汉字方块在 10 / 11px 会糊。
- **≤640px 的练习 / 考试题卡按手机密度排**：卡内距 24→14、卡头 14/18→6/14、卡内容 18→8/12、选项行 10→8、
  选项间距 6→4，题干与选项统一到 `--md-text-compact`（窄屏 15）与 1.45 / 1.4 行高，
  题号 17→13、卡头徽标收到 `--md-text-2xs`。
  朗读按钮走 `data-slot="card-action"`，在卡头里**绝对定位**到右上：盒子仍是 44×44（`pointer: coarse` 的无障碍硬线），
  但视觉缩到 18px 的裸图标（透明底、透明描边），与 13px 的题号同一量级；卡头高度因此由标题行决定（约 37px），
  不再被按钮撑成 56px。它也不再挤在标题行里 —— 挤在里面时 320px 下放不下就整行换行。
  判据是「一屏能扫完全部选项」，回归用例见 `modern_interact.spec.ts` 的「现代风练习页（移动端密度）」。
  触控目标（`.md-icon-btn` 44 / 选项整行 ≥ 44）与焦点环不受影响。
- **≤640px 的阅读页（`.md-article-page`）内距收一档**：卡片内距 24→16、块间距 22→16、
  章节标题 `.md-panel-title` 24→22（`--md-text-lg`）。24 的页面内距 + 24 的卡片内距在 390px 上吃掉 96px，
  正文只剩 294px 宽 —— 相当于每行少两个字。正文档位本身不额外改：它已经是 `--md-text-prose` 的 15px 下限。
  流体正文档位**只属于现代风**：像素风的正文字号是点阵字体的整数倍档（12 / 18），
  非整数倍放大会把点阵笔画糊成灰边，所以它在两档断点上保持固定，不跟这条规范。

## 3. 布局与页面骨架

- 页宽：`--md-page-max` = **1200px**（`kit.css` 把 `.max-w-5xl` 覆盖成它，页头 / 正文 / 页脚 / 底部工具条一起换）。
- 页头：`kit_modern::ModernPageHead`（标题 + 副标题 + 右侧操作区 + 下方工具条），**不要页面里手写 `<h1>`**。
- 断点：见下面「断点与固定层」。
- 滚动：整页一条滚动条；页内第二根吸顶栏用 `common::StickyPanel`，**页面里不要手写 `sticky top-[…]`**。
- **页内菜单（侧栏 / 分类行 / 目录）窄屏只有一种交互**：`kit_modern::ModernMenuDrawer` —— `<768` 右下角悬浮按钮
  （`.md-drawer-fab`）→ 右侧抽屉（`.md-sheet` + `.md-drawer-list`）。动作与像素风 `MobileNavDrawer` 同一套，
  形状按现代风的语言（圆形行动色按钮、行式列表、选中态用行动色）。`≥768` 按钮由 CSS 整颗隐藏，桌面侧栏 / 胶囊行接管。
  按钮可以按住拖动改位置（轻点才是打开抽屉）：位移阈值与收拢在 `ham_web_core::fab`，位置存在 `fab-position`，
  与像素风共用同一个 key —— 换皮肤不会让按钮跳回原处；没拖过时位置交给 CSS（右下角 + `safe-area-inset`）。
  页面只提供抽屉里的行式列表，不自己写 `position: fixed`、断点或抽屉三件套；题库分类、术语分类、工具目录三处必须长得一样。
  多级分类不另起一段：子类缩进挂在**被选中那一类**下面（`.md-drawer-sublist`），展开时淡入 + 下滑 6px
  （`md-drawer-sub-in`，时长走 `--md-dur`；收起是卸载，不播退场，否则整列要抖两下），主类与子类不能脱节，
  也不要叠第二个菜单。
- 吸顶 / 固定元素要避开顶栏时用 `var(--md-nav-h)`（= `--md-nav-inner-h` 60px + 1px 底边），不要写 `60px`：少算那 1px 边框，
  页头第一行会往上钻进顶栏、被裁掉一条。
- **页内吸顶条（`.md-glossary-bar`）的底色必须不透明、横向不越界**：
  半透明 + `backdrop-filter` 会让底下的卡片透上来，整条比页面亮一档，看起来是贴在页面上的一块异物；
  底色既然与页面同色，就不要再靠负外边距铺到版心 / 屏幕边缘 —— 那只会让下边框比下面的卡片宽出一截，
  多出一条两侧探头的横线。底色取页面底色（`--md-bg`，面板类取 `--md-surface`），横向宽度与内容同宽。
- 区块：`section` + `.md-panel`（自带 24px 内距）或 `.md-panel-flush`（内距交给页面，用于旧页面换装）。

### 断点与固定层

**视口断点**：`480 / 640 / 720 / 900 / 1024` 五档，外加**外壳的 768**（底部 dock 与页内菜单按钮）。`check-structure`
的「现代风断点」按这六个值做白名单（识别小数、`em` / `rem`、范围语法）。

| 档位 | 用途 |
|---|---|
| 480 | 手机横屏 / 极窄屏的最后一次降列 |
| 640 | 手机与平板的分界（`max-width` 的窄屏档一律写 **640**，不写 639） |
| 720 | 窄平板的排版档 |
| 768 | **只给外壳**：`<768` 显示底部 dock 与页内菜单按钮（悬浮按钮 + 抽屉）；`≥768` 顶栏有导航、按钮与抽屉一起退场、交给桌面侧栏；不用于别的排版 |
| 900 | 平板与桌面的分界（顶栏完整、bento / 模块三列、练习底栏隐藏） |
| 1024 | 桌面档 |

- **窄屏档写 640，不写 639**（899 / 767 同理）：两个相邻数字并存时，改一处漏一处，且 640px 宽会同时命中「窄屏」与 `min-width: 640` 两套规则。
- **窄屏档的上界一律写成范围语法 `(width < N)`**：`kit.css` 里所有「窄屏」档都是这个写法（`check-structure` 的
  「同值上下界重叠」守卫兜底）。它是**硬性要求**的浏览器特性（Chrome 104 / Firefox 102 / Safari 16.4），
  站点的支持下限（Chrome 111 / Firefox 113 / Safari 16.4）已经覆盖，见 §6.5。
- **互斥的两档要让边界只属于一边**：dock 与顶栏导航（`<768` / `≥768`）、页内菜单按钮与桌面侧栏（`<768` / `≥768`）、
  紧凑与完整导航（`[768, 900)` / `≥900`）、练习页底部操作条与浮动「下一题」（`<900` / `≥900`）
  用范围语法 `width < N`，不要写 `max-width: N`（含边界，N 本身会同时命中两边：768 是 iPad 竖屏宽度，
  会让顶栏与 dock 一起出现；900 写成 `max-width` 则练习页恰好 900px 时既无底栏也无浮钮）。
  其余不互斥的版式档位沿用 `max-width`，1px 重叠无害。
- **同一组件只允许一套阈值**：网格分列只认视口断点，不要把同一组件的分列同时写进
  `@media` 与 `@container`（`.md-bento` 曾因此让 808px 变成「卡片三列、导航还是手机 dock」）。
  容器查询只用于面板内部重排。
- `.md-page` 声明了 `container-type: inline-size`，它是 `fixed` 后代的包含块：
  所有 fixed 弹层 / 底栏必须走 `<Portal>`。
- **Portal 出来的表面不在 `.md-root` 里**（弹层 / 调谐台 / 抽屉 / dock / 气泡 / toast / 固定条都挂在 `body` 上）。新增这类表面要同步三处：
  ① 覆盖它们的规则**不能带 `.md-root` 限定**，否则整条落空（`legacy-utilities.css` 头部有同样的规矩）；
  ② 把它的根类加进 `kit.css` 的焦点环选择器列表，漏一个，键盘用户在那个表面里看到的仍是像素虚线环
  （规则里不要写 `border-radius`：`outline` 本来就跟随元素自身的圆角，写死 4px 会让 10px 的图标按钮一聚焦就变方）；
  ③ 加进 `docs/ui/motion.md` 里的减少动态名单。字体不用逐个声明：`legacy-utilities.css` 已对 `body` 钉死 `--md-font`。

**固定层（z-index 阶梯，取值在 `tokens.css` 的 `--md-z-*`）**：

| 层 | z-index | 出现条件 | 让位方式 |
|---|---|---|---|
| `.md-bar`（考试底栏）/ `.md-bar-practice` | 40（`--md-z-bar`） | 考试：所有宽度；练习：`<900` | 页面 `padding-bottom` 叠 `env(safe-area-inset-bottom)` |
| `.md-next-float` | 41 | 练习页答完题后，**仅 `≥900`** | `<900` 不出现（底部操作条已有「下一题」，同屏只留一个主行动） |
| `.md-rail` | 45 | `≥900` 两侧翻页 | 与 dock 互斥 |
| `.md-dock` | 55（`--md-z-dock`） | `<768`，且**不在练习 / 考试页** | `.md-root` 底部留白 |
| `.md-drawer-fab` | 56（dock + 1） | `<768`，带分类 / 目录的页面 | 抬到 dock 之上 16px；开抽屉时被 `.md-sheet` 遮罩盖住 |
| 抽屉 `.md-sheet` | 60（`--md-z-sheet`） | 打开时 | — |
| 模态 `.md-modal` / `ui::Dialog` 现代分支 | 70（`--md-z-modal`） | 打开时 | — |
| 气泡 `.md-tip-popup` / toast | 80（`--md-z-top`） | 悬停 / 反馈时 | 必须盖过模态 |

规则：

1. **同一底边只允许一层**：dock 与考试 / 练习底栏必须互斥（练习页与考试页收起 dock）。
   新增 fixed 表面时同时写清「谁给它让位、窄屏与 dock 是否互斥」，否则手机上会出现「点交卷跳到导航页」这类遮挡。
2. 所有让位留白都叠 `env(safe-area-inset-bottom)`；底栏自己的 `padding-bottom` 含这一项，
   专注模式（dock 收起）下要把它还回来。
3. 新增 fixed 表面时同时写三件事：z-index 档位（用上表的变量）、谁给它让位、窄屏是否与 dock 互斥。
4. 模态必须压过 dock，气泡与 toast 必须压过模态；不要写 `z-50` / `z-[60]` 这类游离数值。

---

## 4. 组件目录

类名与组件的对应关系由门禁保证同步（见 §7），完整清单见 [`docs/ui/modern-classes.md`](./modern-classes.md)。

### 4.1 按钮与链接

| 类 | 用途 |
|---|---|
| `.md-btn` + `.md-btn-accent / -line / -quiet / -ghost / -text / -danger` | 六种变体；实心（accent）**一屏只给一个主行动** |
| `.md-btn-sm`（30）/ `.md-btn-xs`（26）/ `.md-btn-lg`（48）/ `.md-btn-icon`（40 正方） | 尺寸档 |
| `.md-link` | 正文行内链接（行动色 + 悬停加深，**不下划线**；全站 `a` 免下划线，见 `style/input.css`），由 `kit::link_class()` 换装 |
| `.md-link-row` | 首页入口列表（`.md-link-list`）里的一行：`flex` + 36px 高 + `margin-inline: -8px` 的悬停「出血」。**与 `.md-link` 无关**，不要混用 |

### 4.2 表单

| 类 | 用途 |
|---|---|
| `.md-input` / `.md-input-sm` / `.md-textarea` | 文本、数字、多行；40 / 32 / 自适应高度 |
| `.md-field` + `.md-field-label` / `-hint` / `-error` / `-required` | 表单行：标签 12px、间距 6px、错误行带圆形感叹号 |
| `.md-input[aria-invalid="true"]` | 错误态：红描边 + 底色掺 7% 红 + 标签同步转红（`:has` 联动） |
| `.md-check` / `.md-check.md-radio` / `.md-radio-dot` / `.md-check-mark-icon` | 自绘复选 / 单选（16px，选中蓝底白勾） |
| `.md-check-row` | 「控件 + 文字」的一行（8px 间距，`kit::check_row_class()`） |
| `.md-switch` / `.md-switch-thumb` | 开关 36×20（`role="switch"` + `aria-checked`） |

> **错误态的唯一事实来源是 `aria-invalid`**：常态绝不能写 `aria-invalid="false"`。
> 有具体文案传 `Field(error=…)`；只判断「能否解析」时传 `Field(invalid=…)`，
> 组件会渲染默认提示（`common.check-this-field`）。

### 4.3 数据展示

| 类 | 用途 |
|---|---|
| `.md-table` + `.md-table-compact` | 标准 8/12 内距；**行数 ≥ 15 用紧凑档**（5/8）。量过 46 张表后的现状：知识页规格表（3–8 行）用标准档；`/tools` 下的参考表（3–15 行）历史上就是紧凑档，保持不动；`/modes`（19 行）是唯一因这条判据改过的。判据不再逐张讨论 |
| `.md-panel` / `.md-panel-flush` / `.md-panel-title` / `.md-section-title` | 卡片与区块标题（`kit::card_class*()` / `card_title_class()`） |
| `.md-badge` / `.md-chip` / `.md-chip-sm` | 徽标（1px 描边胶囊）/ 可点胶囊（32 / 28） |
| `.md-stat` / `.md-stat-value` / `.md-stat-label` / `.md-stat-text` | 统计卡与大号读数（等宽 + tabular-nums） |
| `.md-tablist` / `.md-tab` | 页签（底边一条线，激活项渐变下划线） |
| `.md-pagination` / `.md-page-btn` | 分页（32px 方块，`data-current="true"` 实心） |
| `.md-q-row*` | 分类浏览的题目卡片（`.md-q-row-head` 装标签行、`.md-q-row-text` 是题干、`.md-q-row-tag` 是分类胶囊）。**标签行与题干必须是两个块级行**：标签与题干同处一个 flex 行时，题干首行被胶囊推到右边、换行后回到卡片内沿，左边缘就成了锯齿。判据见 `modern_ui.spec.ts` 的「题目行」 |

### 4.4 弹层与导航

| 类 | 用途 |
|---|---|
| `.md-modal` / `.md-modal-panel` / `-head` / `-body` | 模态框（`kit_modern::ModernDialog`） |
| `.md-dialog-veil` / `.md-dialog-panel` | 共用弹层 `ui::Dialog` 的现代皮肤（全站搜索等） |
| `.md-sheet` / `.md-sheet-panel` | 抽屉（移动端菜单、答题卡） |
| `.md-popover` / `.md-option` / `.md-option-group` | 下拉弹层与选项（`ui::popover`） |
| `.md-tip-popup` | 悬停气泡（`ui::tooltip`，挂在 `body` 上） |
| `.md-nav` / `.md-nav-link` / `.md-menu*` / `.md-dock` / `.md-palette` | 顶栏、主导航面板、底部 dock、调谐台 |
| `.md-drawer-fab` / `.md-drawer-list` / `.md-drawer-item` / `.md-drawer-sublist` | 窄屏页内菜单（`kit_modern::ModernMenuDrawer`）：可拖动的悬浮按钮 + 抽屉里的行式列表（子类缩进挂在主类下），见 §3 |
| `.md-menu-chips` | 桌面版分类胶囊行的容器：`<768` 整块隐藏（那份分类在抽屉里），避免同一屏两份菜单 |

---

## 5. 写页面的规则

### 5.1 换装，而不是分叉

页面里**尽量不写 `if is_modern()`**。优先用 `kit::` 里的换装函数：

```rust
kit::card_class()            // 现代 md-panel md-panel-flush / 像素 pxl-card …
kit::card_class_with("p-4")  // 附加类两套共用
kit::card_title_class()      // md-section-title / pxl-section-title
kit::link_class()            // md-link / pxl-link
kit::check_row_class()       // md-check-row / 工具类
ui::badge_skin()             // md-badge / pxl-badge
ui::control_class(size, …)   // md-input / pxl-field
ui::table_base()             // md-table / pxl-table
kit::table_cell_class("align-top") // 现代：只有 extra / 像素：border px-3 py-2 + extra
```

只有在「结构本身不同」（例如弹层、表格密度、题卡）时才在组件里分叉。

### 5.2 不要做的事

| 反例 | 为什么 |
|---|---|
| 页面里写 `sticky top-[…]` | 第二根吸顶栏唯一来源是 `StickyPanel` |
| 组件体顶层读 `kit::is_modern()` | 那里只求值一次，订阅无效；用 `is_modern_at_mount()` |
| 在组件体里 `{f()}` 立即调用读了 `kit::is_modern()` 的闭包 | 跑在构造期、没有跟踪上下文（dev 告警）；写 `{f}` 交给 `view!`，或让闭包读 `_at_mount` |
| 自己读 `data-ui-style` / `ui:uiStyle` | 风格开关只在 `theme.rs` 读写 |
| 在像素分支里写 `md-*` 类 | 反向隔离门禁会红（现代类名漏进像素模式） |
| 新增无对应组件的 `md-*` 类不登记 | 类名索引门禁会红 |
| 层外（不在 `@layer` 里）写不带 `md-` 的规则 | 会污染像素风（层外优先级高于工具类） |
| 用 `!important` 而不写理由 | 必须同行补 `/* important-ok: 理由 */` |

---

## 6. 无障碍

现代风的 a11y 是**回归用例守着**的（`e2e/tests/modern_a11y.spec.ts`）：

- 6 个主页面（深色）+ 3 个主页面（浅色）+ 8 种「打开态」表面（设置 / 搜索 / 调谐台 / 快捷键帮助 /
  练习设置 / 移动端菜单 / 答题卡 / 确认弹层）零违规（WCAG 2.1 A/AA）。
- 对比度：正文 ≥ 4.5:1；大字（≥ 24px 或 ≥ 19px 粗体）≥ 3:1。用 `color-mix` 把纯品牌色
  掺前景色压暗是常用手法（`--md-orange` 在浅色底上只有 2.98）。
- 装饰性元素（大号序号、分组计数）也要过对比度 —— 它们是文字节点。
- 图标按钮必须有 `aria-label`；`aria-label` 的文案要**包含**可见文字（`label-content-name-mismatch`），分隔用空格（`搜索 /`）：
  全角括号的 `搜索（/）` 在 axe 里仍判不匹配。
- **无 role 的 `div` 不能挂 `aria-label`**（axe `aria-prohibited-attr`）：`ui::Table` 的滚动容器只有调用方明确传了 `role` 才带名字，
  否则名字留给内层 `<table>`。
- `<main>` 之外的固定条（考试 / 练习底栏、`BottomBar`）要写 `role="region"` 与名称（axe 的 `region` 规则：页面内容都应在地标内）。
- **axe 的「零违规」不覆盖渐变底、背景图与伪元素上的文字**：这类节点只进 `incomplete`（需人工复核），不算 violation，
  迁移时曾有 1272 处。`.md-quiz-score .md-mono` 就是人工按「向上找第一个实色祖先、再做 alpha 合成」估算后才发现只有 3.29:1
  （`.md-mod-index`、`.md-menu-count` 是同类）。仓库里没有留存这个自检脚本，改这类文字的颜色时要自己算一遍。
- 弹层：`role="dialog"` + `aria-modal` + `data-modal-root`，焦点用 `focus::use_modal_focus` 锁在面板内。

---

## 6.5 浏览器基线与 `:has()` 的写法

版本取自 MDN browser-compat-data（`mdn/browser-compat-data`，2026-11-01 取数）。

| 特性 | Chrome | Firefox | Safari | 站点怎么用 |
|---|---|---|---|---|
| `color-mix()` | 111 | 113 | 16.2 | **硬性要求**，无回退 |
| `@layer` | 99 | 97 | 15.4 | **硬性要求** |
| 范围媒体查询（`width < 768px`） | 104 | 102 | 16.4 | **硬性要求**：dock 与紧凑导航的边界（让边界只属于一边，见 §3） |
| `:has()` | 105 | 121 | 15.4 | **不是硬性要求**：版式不用它，焦点环有回退，其余是装饰（见下） |
| `@supports selector()` | 83 | 69 | 14.1 | 回退机制本身，比 `:has()` 早得多 |
| `@property` | 85 | 128 | 16.4 | 渐进：缺失时焦点卡 hover 的颜色变化是「切」而不是「过渡」 |
| `animation-timeline` | 115 | 159 | 26 | 渐进：已在 `@supports` 内，缺失时无滚动视差 / 脊柱进度 |
| View Transitions | 109 | 144 | 18 | 渐进：`view_transition.rs` 先检测 `startViewTransition` |

**合起来的下限：Chrome 111 / Firefox 113 / Safari 16.4。** 低于它的浏览器没有支持承诺。
`:has()` 会把 Firefox 的下限从 113 抬到 121，这是**刻意避开的** —— 所以有下面的规矩。

**`:has()` 的三种合法用法**（`check-structure` 的「`:has()` 降级声明」要求每一行都写 `/* has-ok: 理由 */`）：

1. **版式 / 状态判定一律不用**。练习 / 考试页收起 dock、换底部留白这类「丢了就坏」的，由 Rust 在 `<body>` 上写
   `data-page-mode`（`kit_modern/page_mode.rs`），CSS 用普通属性选择器；输入框的图标内边距同理（`data-adorn-left/right`）。
   页面自己知道自己的形态，没必要让浏览器去反查子树 —— 这也省掉 `body:has()` 在每次 DOM 变化时的样式失效开销。
2. **焦点环**：必须加 `@supports not selector(:has(*))` 回退到 `:focus-within`，否则键盘用户在老浏览器上看不到焦点
   （`.md-opt`、`.md-card-link` 是范例）。
3. **纯装饰**：缺失只是少一点视觉提示（禁用行不变淡、非法字段的标签不转红、地图标记不悬停提亮），
   在行尾写明缺失后果即可。控件本身的状态（禁用、`aria-invalid`、红色描边）不能依赖它。

## 7. 门禁与自检

```bash
cargo make check-ui        # 界面规范（「应为 0」的机械条目，条数以工具输出为准）
cargo make check-structure # 结构守卫（含下面的现代风相关项，项数以工具输出为准）
cargo make check-i18n      # 文案词典
cd e2e && npx playwright test tests/modern_ui.spec.ts tests/modern_a11y.spec.ts
```

`check-structure` 里与现代风直接相关的几条：

| 检查 | 判据 |
|---|---|
| 双设计系统隔离 | `kit_modern/` 与 `style/modern/` 里不出现 `pxl-` |
| 反向隔离 | 只在像素风渲染的文件里不出现 `md-*` |
| 现代风样式门禁 | 指向像素风 DOM 的选择器必须带 `html[data-ui-style="modern"]` |
| 层外样式门禁 | 层外规则必须带 `md-` 或 `data-ui-style` |
| `!important` 标注 | 每一条都要同行写 `/* important-ok: 理由 */` |
| `:has()` 降级声明 | 每处 `:has()` 同行写 `/* has-ok: 理由 */`（`@supports selector(:has(*))` 特性检测不算在用） |
| 状态色字面值 | 红 / 琥珀色相（含橙）的十六进制只能写在 `tokens.css`；品牌渐变等例外同行写 `/* color-ok: 理由 */` |
| 退场时长走 token | 退场动画（关键帧名 `*-out` / `*-out-only`、非无限循环）的时长必须是 `var(--…)`，不能写 `180ms`；路由过渡之类不走延迟卸载的同行写 `/* dur-ok: 理由 */`。两套皮肤的 CSS 都扫 |
| 延迟卸载只有一份 | `crates/app/src` 里只有 `presence.rs` 能定义 `*EXIT_MS` 常量 |
| 视口断点 | 只能取 480 / 640 / 720 / 768 / 900 / 1024（识别小数、`em` / `rem`、范围语法） |
| 类名索引 | `style/modern/` 下全部 CSS 声明的 `md-*` 与 `docs/ui/modern-classes.md` **精确双向**一致 |
| 弹层三件套 | 带 `data-modal-root` 的面板必须装齐焦点 / 滚动锁 / Esc |
| 文档内链 | 相对链接与 `docs/xxx.md §N` 小节引用都要真实存在 |

改完界面后的人工判据：

```
同页可见输入框高度 == 下拉高度 == 40
按钮高度档 ≤ 4 种（h-auto 的文本型按钮不计）
整页（含 Portal 出来的表面）里 .pxl-* 计数 == 0（没有例外：404 页与气泡都已按风格分叉）
```

`tests/dual_isolation.spec.ts` 会替你守住这条：**现代风里 `pxl-*` 必须为空、像素风里 `md-*` 必须为空**
（8 条代表路由 × 打开态表面，各自要等到面板真的出现再扫）。全量版：

```bash
node e2e/audit/dual_sweep.mjs modern && node e2e/audit/dual_sweep.mjs pixel
```

两条扫描覆盖 181 条路由（registry 180 + 404）与交互表面（现代风 9 个、像素风 8 个：调谐台是现代风专有），两侧都应为 0。脚本路径与 cwd 无关，
站点地址取 `E2E_BASE_URL`（默认 `http://127.0.0.1:3030`）；有任何串风格 / 报错 / 表面没打开都以非 0 退出。

---

## 8. 反例清单

1. **同一类定义两遍**（`.md-field` 曾出现 4px 与 5px 两份，后者静默生效）。
2. **同名组件两套实现各写一套类**（`.md-toggle` 与 `.md-switch` 并存过 → 已合并）。
3. **层外规则会盖掉层内新规则**：现代风样式若被覆盖，先看 `style/modern/legacy-utilities.css` 有没有同名规则（它是层外的）。
4. **`size-6` 之类的工具类会压过层内规则**：需要精确尺寸时写层外规则或专用类。
5. **在 `@keyframes` 附近用 `[^}]*` 正则删 CSS** 会切坏块结构，Tailwind 直接报 `Missing opening {`。
6. **静态审计只能证明「默认路径不渲染」**：`search_dialog` 只在按 `/` 时出现，
   删桥接前要么补交互态扫描，要么先给组件加分叉。
7. **按深色底调的浅色粉彩直接写进组件**（`#ff7a7a` / `#ffcf6b` / `#8fb4ff`）：深色好看，浅色下对比度 1.2–2.5:1，
   而 axe 扫不到 —— 只有答错 / 告急 / 解锁才会出现。走 §2.1 的状态色 token。
8. **深色墨水字压在会变深的实心面上**：`.md-quiz-key` 曾写死 `#0b1220`，浅色下蓝底（`#2540ff`）上只有 2.9:1、
   绿底（`#00734f`）上 3.2:1。实心面上的字用它自己的 `-fg` token（`--md-blue-fg` / `--md-green-fg`），随明暗一起换。
9. **新类名与既有类共名**：按压反馈的 `md-ring` 与首页旧的 `.md-ring`（132px 圆环）撞名，按下选项行的 560ms 内整行缩成一个圆，
   `click` 被丢掉。临时挂在元素上的行为类名要全局唯一，加之前 `grep` 一遍。
10. **给可点元素铺覆盖层，会让里面的子元素变得「点不到」**：单选题行里 label 的 `::before` 铺满整行后，
   Playwright 的 `locator.click()` 点行内 `strong` 会因命中的是 label 而一直重试到超时（用户的真实点击不受影响，
   落在 label 上同样选中）。现代风下点选项一律点整行 `.md-opt`；`fixtures.option()` 点的是视觉隐藏的控件本身，在现代风下用不了。
   另外：量点击坐标要放在**所有会引起重排的前置操作之后**，否则坐标过期，点击落到别处，看起来像「功能偶发丢点击」。
11. **同一个控件写两份实现**：`Select` 与 `NativeSelect`（现 `SimpleSelect`）曾各写一套开合 / 触发器 / 箭头 / 面板，
   只共享了类名。结果一份键盘完全不可用、另一份 Tab 离开后全屏遮罩残留，箭头一个渐变一个瞬切，选中行一个多一枚对勾。
   同一个控件有两种**接口形态**（组合式 / 数据式）时，让一个做引擎、另一个做封装，行为只写一遍。
12. **组件体里 `provide_context` 会被并排的兄弟覆盖**：Leptos 的组件不开上下文作用域。凡是"根 + 子项"形态、
   子项在**打开 / 展开之后才构建**的组件（下拉、手风琴、菜单），根必须用 `leptos::context::Provider` 包住子节点。
   症状很隐蔽：只有同一页并排有两个以上才出，且是"选项读到了别人的值"。
13. **退场写成 `transition`**：入场动画与 `data-state` 在同一次样式变更里被移除，浏览器不会从动画值起过渡，
   退场根本没播 —— 像素风的面板在关闭后第一帧就是透明的，现代风的 `ui::Dialog` 则干脆没有 `[data-state="closed"]` 规则。
   退场一律写 `animation`，并用逐帧采样验证（`e2e/tests/ui_motion.spec.ts`），不要只看「最终消失了」。
14. **同一件事的时长写在 Rust 常量里**：卸载延迟曾在六个文件里各写一份（200 / 200 / 240 / 180 / 180 / 180），
   注释都写着「要和 CSS 同步」，没有任何东西保证。让 Rust 读 CSS 的 token（`presence::exit_ms`），两边不可能再各说各话。
15. **退场中的遮罩还在吞点击**：元素在 DOM 里留到动画结束，期间全屏遮罩照样接收指针事件。关闭态一律 `pointer-events: none`。
16. **给变体写悬停规则时漏掉 `:not(:disabled)`**：`.md-btn-accent:hover` 曾没排除禁用，禁用的主按钮悬停会扫光、上抬、投影加深，
   看起来可点、点了没反应。每个变体各写各的悬停，就要各自排除；测试要**遍历全部变体**，只取第一个会恰好取到写对了的那个。
17. **两个不同的组件共用一个类名**：首页入口列表的一行与 `kit::link_class()` 的行内文字链接曾都叫 `.md-link`，
   于是全站 36 个行内链接被一并撑成 `flex` + 36px 高 + `margin-inline: -8px` 的「列表行」（`/radio-law` 一页 22 个）。
   症状不在出问题的组件上，而在**别人**身上：`i18n_layout` 的现代风档把它报成「文本裁剪」才发现。
   列表行现在叫 `.md-link-row`；行内链接的 `.md-link` 只管颜色 / 悬停 / 焦点环，**不碰盒模型**。
   新增类名前先 `grep` 一遍，改名后用 `docs/ui/modern-classes.md` 对账。
18. **覆盖规则要对着原规则的完整选择器算优先级**：原规则是 `html[data-ui-style="modern"] .motion-page`（0,2,1），
   只写 `.motion-page:has(…)`（0,2,0）压不住它，动画照旧生效。凡是「关掉某个动画 / 位移」的覆盖，改完要**量**：
   连续采样目标元素的 `getBoundingClientRect().top`，首屏 `max - min` 应为 0（练习 / 考试页曾叠着多层入场位移：
   路由入场、整页入场、题卡换题上浮、像素的 `animate-in`，表现为首屏「跳一下」）。
19. **窄屏上同一屏不要有两个吸顶横条**：搜索条与分类条各自吸顶，加起来吃掉 100px 以上，下面那条还会被上面的遮住，
   视觉上就是「只剩一个搜索框，菜单不见了」。现行做法是 ≥900 搜索条吸顶；窄屏**一律不留吸顶横条**，
   搜索条随页滚走，分类 / 目录收进抽屉（`interaction.css` 的 `@media (min-width: 900px)` 分支只给桌面吸顶）。
   窄屏上侧栏消失后必须有等价的页内导航入口 —— 三页共用同一个 `ModernMenuDrawer`，不要一页一个花样；
   也不要退回「一行横滚胶囊」：9 个分类横排后仍要划、46 个工具更是划不到头。
20. **带 `transform` 的元素会成为绝对定位后代的包含块**：闯关地图的记号圆点曾放在会 `hover:translateX(3px)` 的卡片**内部**，
   鼠标一上去圆点就以卡片为基准重新定位，跳到文字上。记号现在是卡片的**兄弟**节点（`pages/quest_map/stage_node.rs`）；卡片的 hover 后来也改成只换描边与底色、不再位移，但兄弟结构保留，以后给卡片加任何 transform 都不会再把记号带走。
   判据：hover 前后量几个相关元素的 `getBoundingClientRect().left`；若只有绝对定位的那个变了，就是这类问题。
   （同一机制的另一半在 §3：`.md-page` 的 `container-type` 让 `fixed` 后代的包含块变成它，所以 fixed 表面要走 `<Portal>`。）
21. **弹层面板上无条件 `stop_propagation` 会让面板里的链接整页刷新**：路由的全局 click 拦截挂在 `window` 的冒泡阶段，
   面板为了「点面板内不关闭」把冒泡截断，里面的 `<a href>` 就到不了拦截层，浏览器走默认导航（用户会看到一次启动遮罩）。
   「点外面关闭」一律走 `kit_modern::backdrop::Backdrop`（只认按下与抬起都落在遮罩本身的点击），不在面板上拦冒泡。
   有条件的拦截不在此列：`ModernRail` 只在拖动结束后吞一次 click（防止拖完误点链接），`popover::swallow` 用于弹层里没有链接的选项行与遮罩（防止 `<label>` 把点击转发回触发器）。
   判据（`modern_ui.spec.ts`）：跳转前在 `window` 上写标记，跳转后标记仍在、面板已移除、URL 已变，三者同时成立才算 SPA 导航。
22. **整块比视口还高的容器不要登记进入场观察器**：`reveal_modern_page`（`motion.rs`）的阈值是「自身 8% 可见」，
   一个 20000px 高的 `.md-tools-grid` 永远达不到 —— 登记了就等于把它永久藏起来（工具页曾整页空白）。现在按 `高度 ≤ 视口 × 0.92` 过滤，
   它的子块（`.md-panel`）各自登记。新增入场目标时先想它会不会比一屏还高。
23. **在共享类上挂装饰**：`.md-h2` 同时用于页面区块标题和弹层 / 面板里的小标题。曾给它加过橙色方块，结果「答案解析」「我的设置」
   这类小标题全被顶上一个巨块；装饰现在挂在只属于页面的 `.md-article-title::before` 上。同理 `.md-badge` 也被 `ui::badge_skin` 复用，
   橙色只限定在 `.md-hero-copy > .md-badge`，不整体改色。
24. **像素风的全局默认会渗进现代页面，而 `legacy-utilities.css` 不是过渡桥**：像素风把所有 Tailwind `transition-*` 的默认缓动设成
   `steps(4, end)`（`--default-transition-timing-function`），不改回来，未迁移页里每个 `transition-colors` 都是四帧跳变。
   `legacy-utilities.css` 在 `html[data-ui-style="modern"]` 上把默认缓动与时长改回 `--md-ease` / `--md-dur`。**这个文件是现代风的 token 层，
   不要当遗留清理**：`body` 的字体兜底（没有它，`main` 之外的挂件与 Portal 表面会回落成点阵字）、默认缓动、像素语义色
   （`--primary` / `--card` …）的现代对应，只要像素 CSS 还挂在 `<html>` 上就得在。页面里约 900 处、散在 300 多个文件里的共享语义工具类
   （`font-mono` / `bg-inset` / `text-primary` / `text-hp-text` / `bg-card` …）也不要改写成 `md-*`：量化后是代码变多、样式不变；要减少规则，走变量层。
25. **改变量层之前，先确认工具类读的是哪个变量**：语义色（`--card` / `--primary` / `--color-hp-text` …）可以在 `legacy-utilities.css`
   的变量层一次重映射，不必逐类覆盖；选择器挂在 `html[data-ui-style="modern"][data-scheme] body`（0,2,2），才能稳定压过非经典配色方案的
   `html[data-scheme="x"].dark`（0,2,1），不依赖文件引入顺序。**字体不行**：像素风的 `.font-sans` / `.font-mono` 读的是
   `--font-pixel-body` / `--font-pixel-mono`，不是 `--font-sans` / `--font-mono`，全仓没有任何规则读后者（像素风在 `@theme inline` 里声明了它们，Tailwind 生成工具类时就把值换了进去）。
   `legacy-utilities.css` 里曾有两行重映射这两个变量的声明，实测它们不起作用：只停用逐类覆盖、保留那两行，
   `/tools` 上的 `.font-mono` 立刻回到 Fusion Pixel Mono；删掉它们后 25 个页面变体、7126 个有字元素的计算字体栈与删之前逐一相同。
   字体必须逐类覆盖 `.font-sans` / `.font-mono`。
26. **`mx-auto` 的 flex 子项不再拉伸**：auto 外边距会吃掉 flex 子项的剩余空间，子项于是按 max-content 排，内容宽时顶出版心
   （`/qsl-labels` 在 390px 宽曾溢出 12px）。现代页头已经提供版心与内距，像素页头自带的 `mx-auto max-w-5xl px-4` 放在里面是重复的，
   `kit.css` 的 `.md-page-head > [class~="mx-auto"]` 把它归零 —— 写在 `@layer` 之外，因为 `px-4` / `max-w-5xl` 是工具类，层内压不住。
27. **同一个闭包服务两套皮肤时，在闭包里面分叉，别在外面补一层**：词条卡的「题库出现」脚注是两套皮肤共用的 body，里面的胶囊 / 计数类各自
   按 `modern` 选（`pages/glossary/term_card.rs`）。外层再包一个现代容器，去不掉内层写死的像素类（`border-t border-dashed`、`border px-2`），
   两套描边会叠成双层边或虚线盒；反过来在共用 body 里写死 `md-mono` 之类的现代类名，像素风里会命中它的规则。
28. **挂载后才算出来的状态，组件 prop 要用 `Signal<bool>`**：`bool` 只在构造时取一次值。成就的解锁状态是页面加载后才算出的，
   `ModernEmblem` 的 `locked` 是 `Signal<bool>`、调用方传 `Signal::derive(…)`；第一版写成 `bool`，21 枚徽章永远停在锁形。
29. **直接子 `<section>` 会被自动编号**：`#route-content .md-page > section:not(.md-closing)` 用 CSS 计数器给每个直接子 `<section>` 画「01 / 02」
   章节头与上沿刻度，按标签匹配、不管语义。非章节块（进度条、提示条）不要写成 `.md-page` 的直接子 `<section>`，用 `<div class="md-panel">`
   （成就页的进度条曾因此被渲染成「01」）。
30. **改了组件的类名或皮肤分支后，查一遍还挂在旧类名上的状态选择器**：`Pagination` 的页码按钮早已换成 `md-page-btn`，「当前页高亮」却还挂在
   `.pxl-btn[aria-current="page"]` 上 —— 现代风下当前页没有任何标识，没有报错，也没有门禁变红。现在当前页由 `data-current` 驱动
   （`ui/pagination.rs` + `kit.css`）。改名后 `rg` 一遍这个组件上的 `[aria-*]` / `[data-*]` 状态选择器。
31. **`view!` 属性值里含 `<` / `>` 要加花括号**：写 `disabled={current >= total}`，不要写 `disabled=current >= total`。没加花括号时 `>` 会被当成
   标签结束，后半截被渲染成文本（翻页按钮里曾出现一行 Rust 源码），编译不报错。
32. **跨 Kit 共用的 context 别直接 provide 裸 `RwSignal<bool>`**：`expect_context` 是按**类型**查找的。现代风顶栏曾 provide 过自己的
   `RwSignal<bool>`（设置面板的开关），与站内搜索的开关同类型 —— 切换风格后，像素风顶栏拿到的是前者留下的、已随组件销毁的信号，
   点搜索按钮时 `set(true)` 写在死信号上，弹层永远不出现。现在站内搜索的状态是独立的 newtype（`components/search_dialog.rs` 的 `SearchOpen`），
   设置面板的开关不再走 context（`ModernMenu` 显式接收 `settings_open` prop）。约定：`provide_context` 只留给真正跨层的状态，且要有自己的类型；
   父子之间直接用 prop。判据（`global_search.spec.ts`「切换界面风格后顶栏入口仍可用」）：像素 → 现代 → 像素，每一步都点顶栏按钮，而不是按 `/`。

---

## 附录：相关文件

| 文件 | 内容 |
|---|---|
| [`docs/ui/pixel.md`](./pixel.md) | 像素风规范 |
| [`docs/ui/modern-classes.md`](./modern-classes.md) | `md-*` 类名索引（手工维护，门禁精确双向校验） |
| [`docs/ui/decisions.md`](./decisions.md) | 设计取舍（为什么两套独立 Kit）；规范冲突时以本文件为准 |
| `crates/app/src/ui/README.md` | 组件 API 清单（「有什么」） |
| `crates/app/style/modern/kit.css` | 现代风组件样式（8000+ 行，按类名分区） |
