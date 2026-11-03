# 全站交互体验升级（现代风）

> 这篇文档回答四件事：**动效有哪几类、各在什么场景触发**；**页面之间怎么过渡**；
> **每个页面靠什么与别的页面不同**；**版式上改了什么**。最后给出「统一设计语言」与
> 「逐页个性」之间的取舍原则，以及落点、门禁与验收。
>
> 范围：只作用于现代风（`html[data-ui-style="modern"]`）。像素风按 D3 冻结，本文任何一条
> 都不改变它的外观与手感。数据、路由、URL、i18n 词条、无障碍语义一律不动。

## 0. 一句话结论

升级不是「换一套颜色」，而是把全站 180 条路由收进**一套动作语法**（7 类动作 + 12 个页面签名），
让每条路由既能被认出「这是哪一类页面」，又共享同一套曲线、时长与降级口径。
落点是三层：**路由层**（`#route-content` 上的 `data-sig` / `data-nav-dir`）、
**原语层**（`kit_modern/interaction.rs` 的按压反馈与 `kit_modern/rail.rs` 的拖拽轨道）、
**样式层**（`style/modern/interaction.css` 只按属性分支）。

## 1. 交互类型与触发场景

七类动作，全部只用 `--md-ease*` / `--md-dur*` 这一套曲线与时长；
「每页不一样」靠**动作不同**，不靠各页自创缓动。

| # | 动作 | 触发 | 表现 | 时长 | 落点 | 降级 |
|---|---|---|---|---|---|---|
| 1 | **悬停**（hover） | 指针进入可点表面（`@media (hover:hover)` 才启用） | 卡片上抬 ≤3px、描边加深、聚光跟手（`--mx/--my`）、列表行长出左侧标尺、索引条目右移 3px | 120–180ms | `kit.css` §动效层 + `interaction.css` §4 | 触屏无 hover；`prefers-reduced-motion` 去掉位移 |
| 2 | **按压**（press） | `pointerdown`（全局一次事件委托） | 控件：从落点扩散的**声呐涟漪**；整块表面：边缘**声呐环** | 560ms | `kit_modern/interaction.rs` + `.md-ping` / `.md-sonar`（类名必须全局唯一，曾因与首页旧 `.md-ring` 撞名让选项行按压时缩成圆） | 动效关闭时不注入任何节点 |
| 3 | **点击 / 状态**（click / state） | 选中选项、判分、切题、跳目录、答题卡点亮 | 选项「掐一下」（1→0.988→1）、格子弹一下（0.9→1.04→1）、工具页目标面板闪一圈蓝环、数字滚动（expo-out 760ms / 16 步） | 240–900ms | `kit.css` 签名段 + `motion::use_count_up` | 直接落到终态 |
| 4 | **拖拽**（drag） | 窄屏横向轨道上按住鼠标拖动 | 1:1 跟手、松手吸附到最近一张；拖完吞掉那次 click；键盘 ←/→/Home/End 等效 | 实时 | `kit_modern/rail.rs` + `.md-track` | 触屏交还原生滚动（已有跟手与惯性）；无指针事件时退回滚动条 |
| 5 | **滚动**（scroll） | IntersectionObserver 进入视口 / `animation-timeline: scroll()` | 内容块错峰浮现（60ms×≤6 档）、顶栏阅读进度、日志页时间轴脊柱随滚动生长、工具页目录落点高亮 | 520/640ms + 滚动进度 | `motion::reveal_modern_page` + `use_scroll_spy` + `interaction.css` §4.3 | 不支持 `animation-timeline` 时**整条脊柱不画**，而不是画一条满格假进度 |
| 6 | **键盘**（keyboard） | Tab / `⌘K` / `←` `→` / `Esc` / `Home` `End` | 焦点环（`focus-visible`）、调谐台、侧边翻题、轨道滚动、弹层关闭 | — | `shortcuts.rs`、`palette.rs`、`presence.rs`、`rail.rs` | 无 JS 时仍是普通链接与表单 |
| 7 | **换页**（route） | 路径变化 | 见 §2 | 240–360ms | `motion::RouteTransition` | 首屏与同页筛选不播 |

**为什么按压要区分「涟漪」与「边缘环」**：形状规整的控件（按钮、胶囊、导航链接）可以在内部
插一个节点；整块卡片内部常有绝对定位的装饰，插节点会把布局挤歪，所以改成描边外扩。

## 2. 页面之间的过渡

一条路径、三层时序，任何新页面都不许再叠第四层：

| 层 | 承载 | 表现 | 时长 |
|---|---|---|---|
| ① 换页帧 | `#route-content.motion-page`（所有页面共用，含桥接页） | 按方向三选一，见下表 | 240–360ms |
| ② 内容浮现 | `[data-reveal]`（由 `motion::reveal_modern_page` 登记） | 18px 上浮 + 淡入，同批错峰 60ms | 520/640ms |
| ③ 共享元素 | 原生 View Transitions，**仅调谐台跳转** | 旧页淡出、新页上浮 | 240ms |

换页方向由 `ham_web_core::interaction::nav_direction` 判定，写在 `#route-content[data-nav-dir]` 上：

| 方向 | 什么时候 | 表现 | 语义 |
|---|---|---|---|
| `initial` | 首屏 | 沿用整页淡入（`md-route-enter`） | 不播换页动画，交给页面自己的入场编排 |
| `forward` | 层级变深（`/` → `/stats`） | 自下 14px 浮入 | 「往里走了一层」 |
| `back` | 浏览器后退 / 前进键，或层级变浅 | 自上方 10px 落回，300ms | 「退回去了」，比 forward 更快 |
| `lateral` | 同层级横跳、同页只改查询串 | 只交叉淡入 + `scale(0.995)`，240ms | 同级之间没有「深入」语义 |

三条硬约束（都是踩过的坑）：

1. **高频操作禁用 View Transitions**：VT 会先冻结一帧拍快照，练习翻题第二次起会卡到数百毫秒。
   题目切换用的是 `.md-practice .md-q` / `.md-exam .md-q` 的 CSS 动画，方向相反、动作相同。
2. **位移只用 `translate` / `scale` 属性，不用 `transform`**：入场动画（`[data-reveal]`、`md-q`）
   已经占着 `transform`，两套写在同一个属性上会互相覆盖（表现是「悬停上抬把入场打断」）。
3. **换页动画只有一层**：历史上的「像素网点溶解 + `.md-root` 淡入 + 119 个页面的
   `animate-in`」叠成了「先淡一次再淡一次」，现在只剩 `#route-content` 这一条。

## 3. 逐页签名：12 个动作族

签名按**动作语义**分，不按模块归属分（`/antenna-theory` 与 `/antenna-analyzer` 只差一个词，
一个读、一个算，必须分开）。完整的「路由 → 签名」表是
`crates/core/src/interaction.rs::PageSignature::paths`，单测保证 180 条路由**每条都有且只有
一个签名**、且表里没有不存在的路由。

| 签名 | 覆盖（举例） | 独有动作 | 与邻居的差异 |
|---|---|---|---|
| `landing` | `/` | 首屏视差三层、数字起跳、模块卡甲板；开台清单逐行入位、7 日瀑布反向点亮（`interaction.css` 4.8） | 唯一有「编排式首屏 + 清单入位」的页面 |
| `practice` | `/practice` `/flashcards` `/listen` `/cards` `/daily-challenge` `/battle` `/morse` | 题卡**竖向**浮入、选项掐一下、左右滑切题 | 与 `exam` 同一动作、相反方向 |
| `exam` | `/exam` `/exam-review` `/exam-comparison` | 题卡**横向**推入、答题卡格子弹跳、倒计时危险呼吸 | 时间压力是这个族的特征 |
| `browse` | `/browse` `/mistakes` `/mistake-topics` `/bookmarks` `/most-wanted` | 行长出左侧标尺、筛选条吸附 | 面向「扫一屏再挑一条」 |
| `index` | `/glossary` `/q-code` `/prefixes` `/bands` `/cheat-sheet`… | 条目悬停右移 3px、筛选条吸附 | 面向「按字母/序号找」 |
| `tools` | `/tools` `/vna` `/nec` `/qsl-designer`… | 目录落点蓝环、目录项右移、结果数字滚动 | 面向「算一个值」 |
| `lab` | `*-decode` `/waveform-lab` `/signal-id` `/sdr-waterfall` | 面板扫描带（命中迁移页的 `.md-panel` 与桥接页的 `section.bg-card`） | 面向「看波形」 |
| `map` | `/map` `/grid-map` `/dxcc-map` `/grayline` `/satellites` | 可点标记（带 `<title>` 的多边形 / 圆 / 矩形）悬停描边提亮并变手形；抓手光标只由 `MapView` 内联样式管 | 面向「在空间里定位」 |
| `log` | `/log` `/contest-log` `/cabrillo` `/awards` `/countdown` | 时间轴脊柱随滚动生长 | 面向「按时间排」 |
| `read` | `/reference` `/radio-law` `/history` 及全部知识库页 | 章节编号、链接下划线擦出、页头短线 | 面向「读长文」 |
| `board` | `/stats` `/dashboard` `/progress` `/achievements` | 卡片上抬、进度增长、数字滚动 | 面向「横向比较」 |
| `quiet` | 兜底（当前无路由命中，单测守着） | 只保留基础手感 | 新增页面忘了选签名时不会崩，但测试会红 |

**桥接页同样有个性**：签名写在 `#route-content` 上，尚未迁移到现代组件的那 160 多个页面
也拿到同一套 `[data-sig]` 分支（阅读页的章节编号、日志页的脊柱就作用在它们身上），
不需要逐页改 DOM。

## 4. 布局创新点

不是「同一版式换颜色」，而是四处在**结构层面**动过的点：

| # | 页面 | 改动 | 为什么是结构改动 | 退化路径 |
|---|---|---|---|---|
| L1 | 首页 | 三张模块卡在 **640–899px** 从纵向堆叠变成**可拖拽的卡片甲板**（横向吸附、键盘可走；`<640` 回退纵向列表，保证入口可发现） | 窄屏不再是「一条长列表」，一屏能看到 1.3 张卡，暗示还有内容 | 无指针事件 → 常规横向滚动；桌面仍是三列网格 |
| L2 | 检索页（`browse` / `index`） | 搜索 + 筛选条**吸附**在顶栏下沿，结果列表从它下面穿过 | 长列表里改条件不必先滚回顶部 | 不支持 `backdrop-filter` 时退回实心底色 |
| L3 | 长文页（`read`） | 章节标题前出现**悬挂式两位编号**（`01` `02`…，`≥1024px`） | 用 CSS counter 在正文之外长出一层「手册目录」，不改任何正文与翻译 | 窄屏不显示；`h2` 结构不变 |
| L4 | 日志页（`log`） | 版心左侧一条**时间轴脊柱**，进度随滚动生长 | 把「一堆卡片」读成「一条时间线」 | 不支持 `animation-timeline` 时整条不画 |

这四条共同点：**零新增文案、零新增数据、零新增依赖**，全部由既有 DOM + 属性驱动，
所以 i18n、SEO、打印排版、e2e 定位契约都不受影响。

## 5. 统一与个性的平衡原则

1. **曲线与时长只有一套**：`--md-ease` / `--md-ease-out` / `--md-dur(-fast/-slow)`。
   个性体现在「动作」（竖翻 / 横推 / 标尺 / 脊柱 / 编号），不体现在缓动。
2. **一页只有一个主动作**：练习是竖翻，就不再叠横向推入；阅读页有编号，就不再给卡片加入场位移。
   同时播两个主动作，页面会显得「到处在动」，反而认不出签名。
3. **幅度有上限**：位移 ≤14px（换页）/ ≤18px（浮现）/ ≤3px（悬停），缩放 ≥0.97。
   超过这个量级，动效就从「方向提示」变成「转场特效」。
4. **反馈不改信息结构**：不加轮播、不藏入口、不改文案、不改 DOM 语义。
   所有交互原语都是**附加层**：删掉 `interaction.css`，页面仍然完整可用。
5. **降级优先于炫技**：三层开关（系统「减少动态效果」、设置里的动效开关、`@supports`）任一
   不满足时，都要退到「静态但可读」，而不是「卡在半路」或「内容隐身」。
6. **像素风零影响**：新样式全部以 `html[data-ui-style="modern"]` 或 `#route-content[data-sig]` 开头；
   `kit_modern/` 与 `style/modern/` 里不出现任何像素风标识（`check-structure` 守着）。

## 6. 工程落点

| 文件 | 职责 |
|---|---|
| `crates/core/src/interaction.rs` | 路由 → 签名、路由 → 层级 → 换页方向；10 条单测（覆盖率/唯一性/无死条目/方向/归一化） |
| `crates/app/src/kit_modern/interaction.rs` | 把签名与方向写到 `#route-content`；全局按压反馈（事件委托，一个监听覆盖全站控件） |
| `crates/app/src/kit_modern/rail.rs` | `ModernRail`：可拖拽 / 可吸附 / 可键盘的横向轨道 |
| `crates/app/src/motion.rs` | `RouteTransition` 在换页时落位签名与方向，并重启换页动画 |
| `crates/app/style/modern/interaction.css` | 换页方向三条 keyframes、声呐点/环、轨道、逐页签名、降级 |

数据流只有一条：

```
location.pathname
  → core::interaction::{signature_for, nav_direction}
  → #route-content[data-sig][data-nav-dir]
  → style/modern/interaction.css 的 [data-sig=…] 分支
```

### 6.1 新增一个页面怎么接入

1. 在 `PageSignature::paths` 里把新路由登记到**一个**签名下（漏登记 `cargo test` 会红并列出路径）；
2. 页面若需要自己的主动作，只在 `interaction.css` 里加一条 `[data-sig=…]` 规则，不要写进
   `kit.css`，更不要写进 `.rs`；
3. 需要横向列表就包一层 `ModernRail`（`class` 传业务类名），不要自己写拖拽。

### 6.2 门禁

- `cargo make check-structure`：现代风目录里出现 `pxl-` / `--pxl-` 直接红；
  组件体里订阅风格信号直接红；一文件一组件。
- `cargo make check-ui`：页面里 `transition-all` / `duration-<数字>` / `scale-*` /
  `hover:-translate-*` 一律红 —— 这些活都由本层用 token 完成。
- `cargo make check`：含 `cargo test`（core 的签名表）与 `e2e-check`。

## 7. 验收

`e2e/tests/interaction.spec.ts`：

| 用例 | 判据 |
|---|---|
| 每条路由都落位一个交互签名 | 10 条代表路由的 `#route-content[data-sig]` 等于预期 |
| 首屏 initial / 站内 forward / 后退 back | 调谐台站内跳转与 `goBack()` 下的 `data-nav-dir` |
| 按压有涟漪且自清理 | `pointerdown` 后出现 `.md-ping`，600ms 内消失 |
| 窄屏模块卡可拖拽 | 拖动后 `scrollLeft > 0`、拖完不跳转、`Home`/`→` 可滚 |
| 窄屏无整页横滚 | 320 / 390px `body.scrollWidth - innerWidth ≤ 0` |
| 减少动态效果 | 不注入涟漪，且没有元素停在 `opacity: 0` |
| 像素风不装这套交互 | 按压后无 `.md-ping` |

回归口径：`modern_ui.spec.ts`（双设计系统）、`navigation.spec.ts`（`[data-nav]` 选择器与
本层的 `data-nav-dir` 不冲突）、`mobile_overflow.spec.ts`（180 路由 × 三语 × 两宽度）。

## 8. 已知未做（不要当成 bug）

- **触屏拖拽**沿用浏览器原生横向滚动（已有跟手与惯性），本层只接管鼠标；
- **地图 / 解码器的页面级专属动作**（定位脉冲、波形同步高亮）要等这两族页面迁到现代组件后
  才能落在自家 DOM 上；目前 `lab` 只有桥接卡片上的扫描带，`map` 只有可点标记的悬停提亮。
  这两族页面没有 `data-running` / `data-busy` 之类的状态属性可挂钩，要做「运行中」动效得先补属性；
- **签名规则要对着真实 DOM 写**：`.md-panel` 目前只由 `*Section` 系列、`/tools`、浏览 / 收藏 /
  参考页产出；`lab` 与 `map` 族大多仍是 `pxl-card` / `bg-card`。新增规则前先在页面里数一下
  选择器能命中几个节点（`interaction.spec.ts` 的 lab / map 用例就是这么守的）；
- **共享元素过渡**只用于调谐台跳转；列表 → 详情的共享元素要等详情路由出现再做；
- 像素风**一行未改**：它在 `data-sig` 属性写入后不会有任何视觉变化（样式全部限定了现代风），
  这是设计约束，不是遗漏。
