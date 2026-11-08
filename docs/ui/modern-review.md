# 现代风界面审核台账

> **这不是规范。** 规范在 [`ui/modern.md`](modern.md)，设计取舍的来历在 [`ui/decisions.md`](decisions.md)。
> 本文件只回答两件事：**现在有哪些已知问题**、**改到哪一步了**。
> 条目按「维度 → 编号」排列，编号稳定不变（提交说明里直接引用编号）。
>
> 台账规矩：
> 1. 每条都带**证据**（文件:行 或 实测数值）—— 没有证据的「感觉不好看」不进台账。
> 2. 状态只取四种：`待开始` / `进行中` / `已完成` / `待裁定`（需要改规范或产品口径，不能只靠写代码）。
> 3. 一条改完之后**不删**，状态改 `已完成` 并在「落点」写清实现位置 —— 下一个人不用重新发现同一件事。
> 4. 规范级结论（「以后都这样」）改 [`ui/modern.md`](modern.md)；本文件不沉淀规范。

## 总览

2026-11-07 首次审核：181 条路由、两套皮肤并行，现代风的设计语言（暖白/近黑 + 单点橙 + 编辑部排版）已成型，
问题集中在**覆盖面**（真迁移的页面少）、**状态三件套缺失**、以及若干**单点硬伤**。
同一天完成 P0 与 P1 全部、C1 桥接页收敛（按可机械验证的判据收尾）、以及 C5–C9 五条跨皮肤 token 泄漏；剩余 P2 多为需要排期或产品裁定的项目。

| 状态 | 条目数 |
|---|---|
| 已完成 | 36 |
| 进行中 | 0 |
| 待开始 | 1 |
| 待裁定 | 0 |
| 合计 | 37 |

## 1. 视觉层次与布局

| 编号 | 问题 | 证据 | 落点 / 改法 | 优先级 | 状态 |
|---|---|---|---|---|---|
| L1 | 工具页的**只读结果条与输入框同形**：同为 `--md-surface-2` 底、同圆角、同字号，分不清哪个能改 | `pages/tools/mod.rs` 的 `RESULT` 常量；`.md-input` 样式 | 新增 `.md-readout` / `.md-readout-inline`（左 2px 行动色规则线 + 等宽数字 + 正文色），`pages/tools/mod.rs` 的 `result_class()` / `result_inline_class()` 按皮肤派发，37 处结果条一次换装 | P0 | 已完成 |
| L2 | 页头「一把尺」在窄屏**断成两截**：128px 色条 + 72px 刻度两端贴边，390px 下中间空 142px | `.md-article-head::after` | `<640` 一档改成 48px + 56px，两段加起来约三成宽，读成同一条线 | P0 | 已完成 |
| L3 | 同一张卡出现**两枚「这里是一块」的记号**：工具索引卡既有 3px 橙顶边，又有 `.md-panel::before` 左上角标 | `.md-tools-index { border-top: 3px … }` + `.md-panel::before` | 索引卡 `::before { content: none }`；约定写进 `modern.md` §4.3「索引卡用顶边、内容卡用角标」 | P1 | 已完成 |

## 2. 组件一致性与复用

| 编号 | 问题 | 证据 | 落点 / 改法 | 优先级 | 状态 |
|---|---|---|---|---|---|
| C1 | **真迁移的页面太少**：形状 / 间距 / 字号仍是像素风，只换了颜色 | `grep -rl ModernPageHead crates/app/src/pages`（7）/ `PageHeader`（118） | **口径更正（2026-11-07 实施时发现）**：`PageHeader` / `PageContainer` **本身已按皮肤派发** —— `PageHeader` 在现代风直接返回 `ModernPageHead`，`PageContainer` 的 `max-w-5xl` 被 `kit.css` 覆盖成 1200 且现代风一律做滚动浮现。所以桥接页的页头与版心**本来就是现代风**，C1 的真实范围是**内嵌 markup**（表格 / 内嵌小块 / 读数 / 分隔线），不是重写外壳。已迁：`/morse`（整页：`md-page` + `flex flex-col gap-5` + `.md-tile`）、`/formulas`（48 个内嵌块 + 公式读数）、`/backup`（去掉外层卡片改为「页面底 + 4 张 `.md-panel`」，登录按钮降为描边）、`/stats`（去掉混进来的 `pxl-window`，实体 / 波段标签换 `.md-badge`）、`/solar`（内嵌 inset 块换 `.md-tile`，波段条件表的单元格改走 `table_cell_class`）、`/dx-spots`（提示设置块换 `.md-tile`；波段标签的图表色阶改走现代 token）。**教训**：`section#backup` 的**标签 + id** 是三份 e2e 的定位契约（`backup.spec.ts` / `account_sync.spec.ts` 用 `section#backup input[type=file]`），把它改成 `<div>` 会让四条同步用例超时 —— 换外壳时先 `rg` 一遍 id 所在的标签。**收尾（同日）**：把共享页面里 60 处无条件的 `class="… border bg-inset …"` 全部改成按皮肤派发（现代 `.md-tile`），并给 `check-structure` 加了「像素式内嵌块」守卫（变异验证：写回一处即红）；紧排版表格的单元格四边框改由 `.md-panel table:not(.md-table)` 统一清零（一处规则覆盖 30+ 张表的硬编码单元格）。**完成判据**（本条的「完成」按这个定义）：共享代码里不再有①无条件的像素内嵌块 ②未按皮肤派发的像素色 token ③硬编码的单元格边框 —— 三条都有机械守卫或 e2e 断言。剩 3 处刻意例外：`callsign_structure` / `dx_spots` 的数据表色带类名（只换底不换形）与 `reference` 的选中态单元格，记在此处不迁移。**坑**：`#route-content .md-page > section` 会给文章式「章节编号 + 刻度尺」，面板类 `<section>` 直接做 `.md-page` 子节点会多出一个「01」—— 必须再包一层 | P2 | 已完成 |
| C2 | **同权重动作三种形态**：`/log` 工具条 6 个平级动作 = 3×`md-btn-ghost` + 1×`FileInput(Outline)` 药丸 + 1×`md-btn-quiet` | `pages/log/log_page.rs` 的现代分支 | 现代分支的 `FileInput` 改用 `Variant::Ghost` 且不降尺寸，整排同权重同高 | P1 | 已完成 |
| C3 | 共享 `Skeleton` 在**现代风没有样式**：`.skeleton` 只有像素版，用 `--pxl-dither`（现代风没这个 token）+ `steps(1)`，渲染成硬切灰块 | `style/pixel/motion.css:408`；`--pxl-dither` 只在 `style/pixel/tokens.css` | `kit.css` 补 `html[data-ui-style="modern"] .skeleton`：与 `.md-skel` 同源的平滑扫光 + `prefers-reduced-motion` 降级 | P0 | 已完成 |
| C4 | `kit.css` **约 9900 行 / 294KB** 单文件承载整站组件（553 个类），新增组件只能往后追加 | `wc -l crates/app/style/modern/kit.css` | **已拆成「入口 + 11 个分域文件」**：`kit.css` 只留约定与 `@import`，规则进 `style/modern/kit/01…11-*.css`（base / question / atmosphere / overlay / menu / polish / bridge / motion / page-head / narrow / tables）。**只做连续切片、不按主题重排** —— `@layer components` 的同名块按源顺序拼接，重排会悄悄改掉同权重规则的胜负。**验证**：①切片是连续的，同层同名规则的源顺序与原文件一致（拼接结果**不是**逐字节复刻：各分片有自己的文件头，尾部另带新增规则；去空行后与原文件的 diff 只剩文件头与新增 / 删除的规则）；②把单体版与拆分版各构建一次，导出整份 CSSOM 逐行比对，**4337 条规则完全一致**（截图对比不可靠：这类页面本身有入场/懒挂载抖动）；③`check-structure` 的「阅读正文规范」原本硬读 `kit.css`，已改为读「入口 + 分片」，否则会退化成空转；④**审核时发现**：`kit_modern/monster_art.rs` 的单测用 `include_str!("…/kit.css")` 读盘找配色块，拆分后入口文件里没有规则 → 该测试静默变红（86 条里挂 1 条）。已改成运行时读「入口 + 整个 `kit/` 目录」；教训：**凡是按路径读样式/数据的测试，移动文件时都要连带检查** | P2 | 已完成 |

| C5 | **徽标样式的作用域泄漏**：hero kicker 的 `.md-badge` 没限定祖先，把 `text-transform: uppercase` + 0.16em 字距漏给全站徽标 —— 统计页的「20m」被渲染成「20M」，`ui::Badge` 的每个标签都跟着大写 | `kit.css` 的 `.md-badge`（hero 块）与文件末尾的通用徽标块同名 | 收进 `.md-hero-copy .md-badge`；两条同名规则从此各归各位 | P1 | 已完成 |
| C6 | **视图里的 `({expr})` 被当成整段文本**：统计页「缺口清单」的分组标题写的是 `({ents.len()})`，页面上真的印出「北美 (ents.len())」 | `pages/stats/stats_page.rs` 的缺口清单 | 拆成 `"(" {ents.len()} ")"`。**判据**：Leptos 只认独立的 `{…}`，`(` 开头的 token 树会被当文本 —— 同目录再遇到 `({` 一律按这个改 | P1 | 已完成 |

| C7 | **状态文字色的变量层映射从来没生效**：`--color-hp-text` / `-win-` / `-xp-` / `-gold-` 四个变量在 `legacy-utilities.css` 里改了，但像素风那批色彩 token 写在 Tailwind 的 `@theme inline` 里 —— 生成工具类时值已内联，运行时不再读变量。实测现代风深色底上拿到的是像素的 `#f7a98f` / `#8cc9ff`，而对比度只按像素底面量过 | `legacy-utilities.css` vs `style/pixel/tokens.css` 的 `@theme inline`；`/solar` 的传播等级是第一个暴露点 | **两版**：先按类名覆盖（`html[data-ui-style="modern"] .text-hp-text { … }` 四条）止血；同日收尾改成**别名层**（见下方「三种修法」第 3 种）—— 像素侧 `--color-{hp,xp,gold,win}[-text]` 改为引用 8 个中立别名（`--hp-fill` / `--hp-ink` …），现代侧在 `legacy-utilities.css` 覆盖这 8 个别名。别名才是 `@theme inline` 内联的东西，所以一次覆盖全部档位（含 `bg-win/20`），**类名覆盖已删除** | P1 | 已完成 |

| C8 | **图表色阶 `--chart-1..5` 在现代风没有映射**：`bg-chart-N` / `text-chart-N`（太阳耀斑等级、DX 热点波段标签）拿到的是像素风 8 套配色方案里的某一套。**修法与 C7 不同**：这里的 `@theme inline` 是 `--color-chart-N: var(--chart-N)`，工具类内联的是 `var(--chart-N)`，所以覆盖**底层**的 `--chart-N` 有效、覆盖 `--color-chart-N` 无效 | `style/pixel/tokens.css` 的 `@theme inline` + `schemes.css`；用法在 `pages/dx_spots.rs` 与 `pages/solar/mod.rs` | 在 `legacy-utilities.css` 里把 `--chart-1..5` 映射到 蓝 / 绿 / 紫 / 琥珀 / 橙（实测现代风 `text-chart-4` = `#ffb02e`，像素风不变） | P1 | 已完成 |

| C9 | **四族语义填充色（`bg-hp` / `bg-xp` / `bg-gold` / `bg-win` 及其 `/NN` 透明度档）在现代风是像素色**：题卡对错、进度条、状态点、奖牌处都在用，共 100+ 处，且透明度档无法逐个类名枚举 | `style/pixel/tokens.css` 的 `@theme inline`；用在 `components/question_card.rs`、`components/topic_quiz/quiz_question.rs`、`exam_history/exam_trend.rs`、`pages/cards.rs` 等共享组件 | 在像素风加一层中立别名（`--hp-fill` / `--hp-ink` …），`@theme inline` 改指别名，现代风在 `legacy-utilities.css` 覆盖别名 → 一处覆盖覆盖所有透明度档。**像素取值逐字节不变**（实测 `bg-win` 仍是 `#38b764`）。顺带把 C7 的 4 条类名覆盖并入这一层，只留一处来源 | P1 | 已完成 |

| C10 | **调谐台跳转把整页冻住数秒**：点选一条后「卡一会」—— 跳转套了原生 View Transition，它先拍一张旧状态快照，而那一刻面板还开着，等于把整屏弹层（`blur(26px)` 遮罩 + 181 行结果）一起光栅化 | `kit_modern/palette.rs` 的 `go` 回调 + `kit_modern/view_transition.rs` | **去掉 VT**，先关面板再 `navigate`；删掉已无调用方的 `view_transition.rs` 与 `::view-transition-*` 样式。实测（同机、点选到新页首帧）：**4038 ms → 25–39 ms**；与 `interaction.md` 硬约束 1 一致（高频操作禁用 VT，换页只留 `#route-content` 淡入） | P0 | 已完成 |

## 3. 交互反馈与动效

| 编号 | 问题 | 证据 | 落点 / 改法 | 优先级 | 状态 |
|---|---|---|---|---|---|
| I1 | 桌面「更多」**没有展开态**：面板开着，触发器与普通导航项一样（dock 里的同款有洗底） | `.md-menu-anchor` 只声明 `anchor-name`；dock 有 `[aria-expanded="true"]` | 加 `.md-nav-link.md-menu-anchor[aria-expanded="true"]`：浅橙底 + 行动色文字，不加下划线（下划线是「当前页」的记号） | P0 | 已完成 |
| I2 | 目录面板**滚动没有线索**：内部滚动、无渐隐、无滚动条，115 条知识库只露出 2 条就到底部操作条 | `kit_modern/menu.rs` 的 `.md-menu-body` | **做法撤回了（反向修正）**：曾用 `background-attachment: local` 在滚动区顶 / 底各画一条 14px 渐隐，但上沿那条正好贴在搜索框下方 —— 看起来就是「搜索框自带一个阴影」，一个说不清来路的装饰。现在只靠**内容在边缘被切断**（列表在底栏上方露出半行）+ 滚动条提示，不再画渐变。教训：为了「暗示可以滚」而加的装饰，如果落在某个控件正下方，就会被读成那个控件的样式 | P1 | 已完成（含撤销） |
| I3 | 调谐台只有 ⌘K / Ctrl K，触屏没有入口 | `kit_modern/palette.rs` | **先加了「跳转页面」chip，后又撤掉（反向修正）**：目录面板自带的搜索框已按页名 / 路由 / 译文过滤整份目录，再放一颗跳调谐台的 chip 是把同一件事做两遍，还多一层弹层切换。现在触屏路径 = 面板搜索框，⌘K 只服务键盘用户（不开面板、↑↓ 选、覆盖全部 181 条路由）。残留：`/licenses` 无任何界面入口（只差这一个，与调谐台无关） | P2 | 已完成（含撤销） |

## 4. 响应式与多端适配

| 编号 | 问题 | 证据 | 落点 / 改法 | 优先级 | 状态 |
|---|---|---|---|---|---|
| R1 | **320px 顶栏已无余量**：英语品牌字下右侧操作区与内容边界间隙为 0 | 实测 `en@320`：操作区右缘 299 / 内容边界 296 | 新增 **360 断点档**（白名单第 7 个值）：`<360` 收掉品牌文字、只留站标，链接靠 `aria-label` 保住可访问名。实测 320 下右侧余量 **0 → 24px**，三语无溢出；360 / 390 文字照常。e2e 两条守住（320 折叠 / 390 不折叠） | P2 | 已完成 |
| R2 | 触屏平板（768–1024，coarse）顶栏导航仍是 **34px** 高，与 `modern.md` §2.6 自定的 44px 不符 | 实测 1024 + `hasTouch`：`.md-nav-link` 全 34px | coarse 下 `.md-nav-link` 抬到 44（行高居中，导航行仍 60px，版式不动） | P1 | 已完成 |

## 5. 无障碍

| 编号 | 问题 | 证据 | 落点 / 改法 | 优先级 | 状态 |
|---|---|---|---|---|---|
| A1 | **浅色焦点环对比度 2.33:1**（要求 ≥3:1）：`--md-ring` 是 50% 透明度的蓝 | `tokens.css` 的 `--md-ring`；实测合成后 2.33:1 | 浅色档抬到 70%（实测 3.45:1，深色档同为 70% / 3.69:1） | P0 | 已完成 |
| A2 | **设置图标画成准星**：`MIcon::Settings` 是「圆 + 四根短线」，放大后读不出「设置」 | `kit_modern/icons.rs` 的 `Self::Settings` | 改成齿轮（内圈 + 齿根圈 + 八齿），20px 下齿与圈连成一体 | P0 | 已完成 |
| A3 | **空题库被当成错误播报**：`role="alert"` 会把「题库不可用」作为紧急告警念出来 | `pages/exam/view.rs`、`pages/practice/view.rs` | 改用 `EmptyState`（`plain` 之外的默认卡片）；「全做完了」那个空态用 `actions` 给回全部题目的按钮 | P0 | 已完成 |
| A4 | a11y 覆盖不均：`modern_a11y.spec.ts` 只扫 9 个主页面 + 8 个打开态，桥接页几乎没被 axe 扫过 | `e2e/tests/modern_a11y.spec.ts` | 新增「桥接页抽样」用例：`/backup`（表单密集）、`/stats`（图表）、`/nec`（Canvas + 表单）、`/solar`（多卡 + 外部数据），深色档全扫 | P2 | 已完成 |

## 6. 主题与配色

| 编号 | 问题 | 证据 | 落点 / 改法 | 优先级 | 状态 |
|---|---|---|---|---|---|
| T1 | **橙色语义在文档与实现之间已分裂**：`tokens.css` 写「橙=行动，不许标选中」，但顶栏当前页与 dock 当前项都用橙标「我在哪」 | `tokens.css` 行动色阶梯注释、`decisions.md` §5 | **裁定（三分）**：行动 = 实心橙、一屏一个；**位置**（我在哪）= 允许橙，但只走 `accent-ink` 文字 + 细轨道 / 8% 底，不铺实心；**选中**（勾选、筛选 chip）= 一律蓝。实现无需改（现有导航高亮正是「位置」那一档），两份文档已同步 | P2 | 已完成 |
| T2 | 桥接页的错误底色有两档：手写 `bg-destructive/5`（映射到 `--md-danger-fill`）与 `.md-input[aria-invalid]` 的 7% 掺色不一致 | 解码页手写块 vs `aria-invalid` 规则 | 由 `ErrorState` 统一：`.md-error` 用 `--md-danger` 7% 掺色，与表单错误态同一档 | P1 | 已完成 |

## 7. 字体与排版

| 编号 | 问题 | 证据 | 落点 / 改法 | 优先级 | 状态 |
|---|---|---|---|---|---|
| F1 | **中文展示字没有度量对齐回退**：拉丁三款都有度量覆盖，中文展示字直接落到 PingFang / 雅黑，`swap` 后标题位移 —— 实测行盒 **Noto 1.50em vs 系统黑体 1.40em**（36px 标题每行差 3.6px） | `style/modern/fonts.css`；`fonts-cjk.css` 共 7 个 `@font-face` | 补 `"Noto Sans SC Display Fallback"`（ascent 116% / descent 34% = 1.50em）并接进 `--md-font-display`。**坑**：macOS 上 `local("PingFang SC")` 匹配不到（静默失效），必须写 PostScript 名 `local("PingFangSC-Regular")` —— 实测阻断字体加载后标题高度 74px = 74px，位移为 0 | P2 | 已完成 |
| F2 | 最小两档字号（10/11px）仍用在章节编号、路径提示、kbd 上，中文 11px 在 1200 版心里偏小 | `tokens.css` 的 `[lang^="zh"]` 分支 | 中文下 `-3xs` → **12**、`-2xs` → **13**（与 `sm` 同值，有意：中文只需一个「最小正文级」）。回归：181 路由 × 三语 × 320/390 溢出审计 + 三语桌面布局审计全过 | P2 | 已完成 |

## 8. 性能与加载体验

| 编号 | 问题 | 证据 | 落点 / 改法 | 优先级 | 状态 |
|---|---|---|---|---|---|
| P1 | **单包 2.03MB br、181 条路由同一个 wasm**，没有路由级分包 | `perf-budget.json`、`docs/engineering/performance.md` §5 | 本轮做完可做的部分：**量化**（raw 7.85MB / br 2.03MB；数据段占大头 —— 中文词典表单个 346KB、法规 52KB、DXCC 44KB）、**预算 2,500,000 B → 2,260,000 B**（原余量 23% 等于允许悄悄长 400KB）、三条分包路线的成本与触发条件写进 performance.md §5。**真分包**见 P1b | P2 | 已完成 |
| P1b | 路由级分包本身（多 wasm 模块 / SSR 岛屿） | `docs/engineering/performance.md` §5 | 按 §5.3 的三条路线，先做 §5.4 的三步测量（带符号构建 + crate 级归因 + 删减实验）再决定；**需要独立排期** | P2 | 待开始 |
| P2 | 首页 live 面板加载中只有 `—` + `aria-busy`，读数从占位跳到真值会带动整块抖动 | `kit_modern/home/live.rs`、`home/live_metric.rs` | `LiveMetric` 加 `loading`：加载中画按三位数字预留宽度的骨架（`md-live-metric-skel`），文案走 `sr-only` | P1 | 已完成 |
| P3 | 字体总量 1.2MB，现代风首屏预载 4 个文件（CJK-0 约 103KB） | `public/fonts`、`index.html` 首屏脚本、`perf-budget.json` | ①字体进预算（新增 `raw: true` 口径，64 项全过）；②**中文展示字按语言预载** —— 英 / 西语首访字体 **241 KB → 133 KB**（中文仍 234 KB）；③等宽三档移出预载队列 | P2 | 已完成 |

## 9. 状态设计（空 / 加载 / 错误）

| 编号 | 问题 | 证据 | 落点 / 改法 | 优先级 | 状态 |
|---|---|---|---|---|---|
| S1 | **加载态多处只有一行文字**，占位高度与实际内容不一致，内容到位时整页跳 | `pages/dashboard/*_card.rs`、`pages/solar/*`、`pages/muf/heatmap_card.rs` 等 | 新增 `components/common/loading_block.rs`（现代风灰块 / 像素风保留原来那行字），**14 处**已全部换装：仪表盘四卡、太阳活动两卡、most-wanted、ISS 追踪、传播组件、地图三处（`tall=true` 整块灰面）、每日挑战、闪卡、进度、听题 | P1 | 已完成 |
| S2 | **空态一半手写一句话**，与 `EmptyState` 的排版/语气不统一 | `pages/vna_reader/*`、`pages/nec/design_archive.rs`、`pages/log/*`、`pages/sdr_waterfall/*` | 收编 7 处真正的空态（VNA 两处、NEC 存档、SDR 噪底、竞赛分析、奖项申请、听题）到 `EmptyState plain=true`。**注意 `class=NOTE` 多数不是空态而是字段说明**（`smith` / `waveform_lab` 里全是读数标签），不要把这一类改成空态 | P1 | 已完成 |
| S3 | **错误态没有共享组件**：解码页逐字复制同一块 `role="alert"` + `border-destructive` | `pages/rtty_decoder/rtty_decoder_page.rs` 为样板 | 新增 `components/common/error_state.rs`（标题 + 原文 + 可选重试），两套皮肤各一份样式；8 个解码页共 9 处已收编 | P1 | 已完成 |

## 10. 信息架构与可发现性

| 编号 | 问题 | 证据 | 落点 / 改法 | 优先级 | 状态 |
|---|---|---|---|---|---|
| N1 | 现代风外壳**缺两个直达入口**：`/achievements`、`/battle` 只能靠 ⌘K、搜索或页内链接（像素风有 HUD 徽章直达成就页） | `components/navigation/hud_badge.rs`；`core/src/registry.rs` 里两条 `group: None` | 目录面板底部加「成就」入口（不塞进考试段：「段 = registry 分组」这条对应关系不能失真）；战斗页仍只从页内进入 | P1 | 已完成 |
| N2 | 调谐台把 **181 条路由平铺**：无分组标题、无最近访问 | `kit_modern/palette.rs` | 结果先按 registry 的分组排序（否则同组会碎成多段、段标题重复），再切段并渲染 `.md-palette-group`；顺序与 `↑↓` 游标、`aria-activedescendant` 同源 | P1 | 已完成 |
| N3 | 搜索侧「页面」命中硬上限 3 条，长尾页面基本搜不到 | `components/search_dialog.rs` 的 `MAX_PAGE_RESULTS` | 上限 3 → **8**，并补**排序分档**（同名 → 标题前缀 → 标题包含 → 仅路由包含）：只放宽上限不够 —— 旧实现按注册表顺序截断，前缀命中的长尾页会被「正文提到」的条目挤掉。e2e 新增「长尾页面也能直达」，改回 3 时该用例变红（变异验证过） | P2 | 已完成 |
| N4 | 语气不一致：现代首页与调谐台里仍有「复仇到期怪物 / 闯关地图 / 怪物图鉴」这类 RPG 词 | 首页「今日开台」、调谐台首屏 | **裁定**：游戏化页面（`/map`、`/bestiary`、战斗界面）保留世界观语言；**调谐台**是「按概念找页面」的工具，给两条 RPG 路由补**白话别名**（「按专题刷题」/「错题复习」），别名同时参与输入过滤 —— 实测输入「错题」能搜到「怪物图鉴（错题复习）」。首页保留插图化的游戏区（刻意设计） | P2 | 已完成 |

> **`@theme inline` 的三种修法**（C7 / C8 / C9 各用了其中一种，按代价从小到大）：
> 1. 工具类内联的是**中立变量**（`var(--chart-5)`、`var(--hp-fill)`）→ 覆盖那个变量，一次覆盖所有档位（含 `/20`）。
> 2. 工具类内联的是**像素专属 token**（`var(--pxl-gold-text)`）→ 只能按类名覆盖，透明度档要逐个写。
> 3. 想用第 1 种但像素侧写死了 `--pxl-*` → 先加一层别名（像素侧指向 `--pxl-*`，现代侧指向 `--md-*`）。
>
> **C7 最终走的是第 3 种，不是第 2 种**：像素侧那 8 个 `--color-*` 的**定义处**本来就可以改，
> 把值从 `var(--pxl-hp-text)` 换成 `var(--hp-ink)` 即可 —— 一旦内联的是别名，现代侧覆盖别名就覆盖了
> 「填充 + 文字 × 4 族」的全部档位。第 2 种的代价（逐类名、逐透明度档）只在**改不动像素侧定义**时才值得付。
> 判据：`rg 'var\(--' crates/app/style/pixel/tokens.css` 看 `@theme inline` 那一行到底内联了什么。

## 优先级汇总

**P0（本轮全部完成）**：A1 焦点环 · A2 齿轮图标 · A3 空题库告警 · C3 骨架屏 · I1「更多」展开态 · L1 结果条 · L2 页头尺

**P1（全部完成）**：C2 工具条权重 · C5 徽标作用域 · C6 文本插值 · C7 状态文字色 · C8 图表色阶 · C9 语义填充色 · I2 面板滚动线索 · I3 调谐台触屏入口 · L3 卡片记号 · N1 成就入口 · N2 调谐台分组 · P2 live 骨架 · R2 触屏导航 44px · S1 加载占位（14 处）· S2 空态收编 · S3 错误态 · T2 错误底色统一

**P2**：已完成 A4 桥接页 a11y 抽样 · C1（按「完成判据」收尾：6 页真迁移 + 全站 60 处像素内嵌块扫净 + 守卫上线，剩余 3 处刻意例外见 C1 行）· C4 样式拆分 · F1 中文展示字度量回退 · F2 最小字号 · P1 路由分包量化 · P3 字体预算 · N3 搜索命中上限 · N4 文案语气（调谐台白话别名）· R1 窄屏折叠 · T1 橙色语义（三分裁定）。

**仅剩**：P1b 路由级分包本身（多 wasm 模块 / SSR 岛屿），需要独立排期，先做 `performance.md` §5.4 的三步测量。

## 复查方式

- 颜色 / 图标 / 焦点环：`cargo make check-structure`、`cargo make check-ui`，再跑 `e2e/tests/modern_a11y.spec.ts`。
- 布局 / 命中区：`e2e/tests/modern_breakpoints.spec.ts`，外加 `mobile_overflow.spec.ts` 与 `i18n_layout.spec.ts` 的像素 / 现代两档。
- 状态三件套：`e2e/tests/modern_interact.spec.ts`（打开态与交互），新增组件需要配「慢网 + 失败重试」用例。
