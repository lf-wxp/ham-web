# 动画 / 动效规范

本规范适用于 `crates/app` 前端（Leptos CSR + Tailwind CSS 4）的所有 CSS 动画与交互动效。**一至三节是像素风；现代风见第四节；弹层的进出场（两套皮肤共用的「在场」实现）见第五节；控件状态矩阵见第六节。**目标是保证动画在移动端优先的设备上稳定 60fps、省电，且不损害可访问性。

相关文件：

- `style/input.css` — 只做 `@import` 与 `@source`；样式按 `style/pixel/*.css` 分工
- `style/pixel/tokens.css` — 节奏 token（`--motion-*` / `--ease-*`）
- `style/pixel/motion.css` — 像素动画 keyframes、弹层进出场、全局降级规则
- `style/tw-animate.css` — Tailwind 动画工具类（自动生成，勿手改）

## 一、核心原则

1. **合成器友好**：动画只触发合成（compositor），不触发布局（layout）或重绘（paint）。
2. **可停**：无限循环动画与定时更新在后台 / 离屏时不应空转。
3. **尊重用户偏好**：所有动画必须自动继承 `prefers-reduced-motion` 降级。

## 二、规则清单

### 1. 只动 `transform` 与 `opacity`

禁止用 `left` / `top` / `width` / `height` / `margin` 等布局属性做逐帧动画——它们每帧都会触发 layout / reflow。
像素主题另外允许 `background-position`（只重绘、不排版的底纹位移），其余仍只走
`transform` / `translate` / `opacity`。

```css
/* 反例：left 触发 reflow */
@keyframes bad {
  from { left: -28%; }
  to   { left: 100%; }
}

/* 正例：translateX 只走合成器 */
@keyframes good {
  to { transform: translateX(357.14%); }
}
```

项目实例：换台读条 `.route-sweep` 的指针用独立的 `translate`（`calc(100vw - 4px)`，不依赖自身宽度）扫过，
整条只动 `opacity` 淡出（`motion.css`）。
像素动效一律配 `steps()` 逐格跳，不用平滑插值 —— 「跳格」既省合成器时间，也是风格本身。

### 2. 复用节奏 token

缓动与时长统一走 `:root` 中定义的 token（像素主题里缓动也是 `steps()`），不要新写 `cubic-bezier` 魔法值：

```css
animation: hero-rise var(--motion-slow) var(--ease-out-expo) forwards;
transition: transform var(--motion-fast) var(--ease-out-expo);
```

### 3. 无限循环动画需可停

氛围类动画保持长周期 + 低复杂度。项目已有全局降级（见 `style/pixel/motion.css` 末尾 `@layer base` 里的两个块：
`@media (prefers-reduced-motion: reduce)` 与 `html[data-pixel-motion="off"]`），新增动画自动继承，无需单独处理。

**滚出视口也要停。** 转自定义属性、动 `stroke-dashoffset` 的循环动画（现代风首页的圆锥描边、电波、卫星）不走合成线程，
每帧都要主线程重算样式、重绘，看不见了还在跑就是白占主线程和电量：

- **CSS 循环动画**：容器带 `data-pause-offscreen`，`motion::pause_offscreen_in` 在它滚出视口时打上 `data-offscreen`，
  `style/modern/kit.css`「首屏主视觉」一节里**只点名**无限循环的那几条（`[data-offscreen] .md-aurora { animation-play-state: paused }` 这样）。
  **不要写 `[data-offscreen] *`**：它连一次性的入场动画与过渡一起暂停，它们停在起始帧、`finished` 永远不兑现
  （e2e 的 `settleFrames` 会等到超时）。首屏 / 功能卡里新加循环动画，要把它的选择器补进这条规则；
- **逐帧的 JS / canvas 循环**（`kit_modern/spectrum.rs`）：滚出视口时**停掉自调度**，回到视口由观察器再挂上。
  只「不画」、每帧仍排下一帧不算停；
- **定时器里读本地存储**（倒计时每秒检查）：走 `util::storage::get_json_shared`（按原串缓存解析结果），不要每秒重新解析。

### 4. `filter: blur` 只用于小元素

`blur-in` / `blur-out` 仅用于弹层、图标等小元素；整页或大卡片进入动画禁用 blur——低端 Android 上代价过高。

**`backdrop-filter`（毛玻璃）在像素主题里一律禁用**：`surfaces.css` 顶部有一条全局兜底，
把存量页面里残留的 `backdrop-blur*` 与 `--blur-*` 全部归零。层次感靠 2px 硬描边 + 硬投影
（`.pxl-window`）表达，而不是半透明模糊 —— 一页几十张卡片同时开模糊，低端机滚动会掉帧。
新加的材质不要再引入模糊。

### 4a. 装饰性循环动画清单

长周期、低复杂度，且都在 `prefers-reduced-motion` / `data-pixel-motion="off"` 下停住：

- 暗色星点轮换 `.dark body::after`（`pxl-twinkle`，6s，`steps(1, end)`）；
- 骨架屏两档明暗跳动 `.skeleton`（`skeleton-sweep`，1.2s）；
- 指示灯闪烁 `.pxl-led[data-state="blink"]`（`pxl-blink`，1s，`steps(1, end)`；8–16px 的小方块，降级时变常亮）；
- 精灵 / 图标上下浮动 `.pxl-bob`、`.pxl-bob-sprite`（1s）。

背景网点（`body::before`）、首页网格（`.hero-grid`）与屏幕扫描线（`.pxl-display`）是静态底纹，不耗动画预算。
路由切换的「换台」读条 `.route-sweep` 是纯装饰，降级时直接 `display: none`（见 `motion.css`）。

### 5. 闪烁频率 ≤ 3 次/秒

遵守 WCAG 2.3.1，避免光敏风险。现有 `pxl-blink`（0.6s / 1s，`steps(1, end)` 一格一跳）、
`caret-blink`（1.25s）、骨架 `skeleton-sweep`（1.2s）、星点 `pxl-twinkle`（6s）均安全。

### 6. 触屏反馈走 `:active`

桌面 `hover` 态在触屏无效，用 `:active` 提供按压反馈：`.pxl-btn:active`（`controls.css`）
把按钮下沉 2px 并让外投影归零，`.motion-lift` / `.motion-press`（`motion.css`）同理。

### 7. 长列表优先分页

长列表用增量渲染控制 DOM 数量（参见 `src/pages/glossary/glossary_view.rs` 的 `visible` / `PAGE` 分页），不要依赖 `content-visibility` 补性能——它会引入滚动条跳动、`Ctrl+F` 失效等副作用，
而且它自带的绘制隔离会裁掉卡片内 `absolute` 定位的下拉面板。

不能分页的长页面（工具页 60 个计算器）、一打开就要铺满的弹层（「更多」目录约 180 个入口），
以及渲染之后还有成批计算的（术语表卡片的题库出现次数），首屏先出，其余用 `src/after_paint.rs` 的 `each_frame`
每画完一帧推进一批，单批控制在一帧的预算内（不形成 50ms 以上的长任务）；
要按真实位置定位之前（锚点直达、点目录）先一次做完。实现见 `src/pages/tools/tool_mount.rs`、
`src/kit_modern/menu.rs` 与 `src/pages/glossary/hits.rs`。

「用户很可能马上要用、但现在不急」的数据（常用题库）在空闲时预热（`after_paint::when_idle` + `bank_updates::prewarm`），
同一份题库同时只下载、解析一次（`data::load_url`）。

现成的工具与写法，新页面直接用：

- **首屏以下的重块包进 `components::common::Deferred`**：新页面第一帧画出来之后再挂（地址带锚点时它会补上定位）。
  它**下面**还有内容时给 `min_height`，值取各皮肤、各宽度实测高度的下限，否则下面的内容在挂上那一帧整体下跳；
  块里的东西可能整个不出现（如日志为空时的统计明细）就传随状态变空的 `Signal`，别让空页面留白；
- **几块都很重就每块各包一层 `Deferred`**，不要共用一个：同一个 `Deferred` 里的内容在同一个任务里挂完，
  各排各的才落在各自的任务里，块与块之间主线程才能响应点按（`/nec` 的结果区、三维方向图、扫频等）；
- **重活落在绘制之后**：`requestAnimationFrame` 的回调跑在这一帧绘制**之前**，在里面做重活等于算进首帧。
  用 `after_paint::after_next_paint`（rAF 里再排一个 0 ms 定时器；页面已隐藏、rAF 暂停时它直接同步执行）。
  页面外的脚本同理（`index.html` 里 KaTeX 的首轮排版）；
- **「桌面一套、手机一套、CSS 藏一套」** 改用 `media::use_media_query`，只建当前宽度那一套；
- **分页签** `ui::Tabs` 的面板内容第一次被切到才挂载，不必再自己做惰性；
- **长列表（含「加载更多」）按下标 / 主键键控**（`<For key=…>`），并配 `after_paint::grow_cap` 分批补齐：补齐只追加新项。
  整体 `collect_view` 每补一批都把前面建好的全部丢掉重建。三条要点：**数据异步到达的页面，等数据到了再调用 `grow_cap`**
  （在数据落地的那段 `spawn_local` 里启动；先启动的话上限在列表出现前就走完，分批等于没做）；
  **不要在 `step` 里「数据没到就原地等」**：那样的循环靠外部状态结束，加载失败时永不收工，页面隐藏时
  （`after_next_paint` 同步执行）还会变成无终止的同步递归 —— `each_frame` 的 `step` 每次调用都必须有进展（`grow_cap`
  的步进是 core 的 `list_grow::next_cap`，单元测试钉住了任何取值下都会结束）；
  读 `visible` 的闭包不要包住 `<For>`，否则每次加载更多整块重跑、键控白做 —— 「加载更多」按钮用
  `components::common::LoadMore`（它自己读 `visible` 与 `total`，`total` 可以是会变的信号；按钮间距默认 `mt-4 w-full`，
  列表容器已带间距的页面传 `class="w-full"`），不要在页面里再手写一份；
  行里有本地状态的列表，先确认复用已建好的行是安全的；
  **`<For>` 的键是下标时，要写明「数据只写一次」这个前提**（数据会被整体替换的列表改用稳定主键）；
- **搜索框**：筛选挂到 `debounced::use_debounced` 的防抖副本上，「回第一页」之类的重置写 `debounced::set_if_changed`
  （`RwSignal::set` 写同一个值也照样通知，每敲一个字都会让整页列表重建一遍）；输入法组字期间不派发（选字确认才补一次）：
  `ui::Input` 的 `kind=Search` 与 `ModernInput` 的 `kind="search"` 已内置（`composition.rs`，判定在 core 的 `ime_gate`），
  页面不必再写；非搜索的输入框刻意不挡，表单字段要逐字回调；
- **大体积的本地数据**用 `util::storage::get_json_shared` 读（按原串缓存解析结果），不要每次 `get_json`；
  读大集合（日志、错题本、题库）一律借用（`.with`），不要 `.get()` 把整份克隆一遍；
- **页面间共用、算一遍就很贵的派生结果**（日志的概览 / 奖项进度 / 申请表 / 体检）挂成 store 上的 `Memo`
  （`pages/log/derived.rs` 的 `LogDerived`）：惰性（第一次读才算）、数据变了才重算、离开页面不释放，所以再进页面是直接取值。
  规矩：每个这类缓存登记进 `docs/engineering/performance.md` §4（上限与失效条件）；配一条「数据变了，别的页面的数字跟着变」的
  e2e（缓存最容易错在过期）；结果会随时间过期的（体检里「时间在未来」的条目）要带上算它时的时间、读之前判一下；
- **一屏上千个同构 SVG 节点**（地图的区域叠加层）：整层拼成一段标记，`inner_html` 一次解析，点击在外层 `<g>` 上委托
  （`pages/map/svg_markup.rs`：转义、`data-key`、`clicked_key`）。节点形状不变，样式与 e2e 都不受影响；
  逐个 `view!` 建每个节点要好几次 wasm → JS 调用。

这些规则各由哪条用例守着，见 `docs/engineering/performance.md` §2；测量口径、已评估后决定不做的项与常驻缓存清单也在那里。
真机上要看实际耗时，在地址后加 `?perf=1` 打开测量面板（`src/perf_hud.rs`）。

### 8. 电台动作（像素风）

像素风的反馈只用下面八个动作（`docs/ui/pixel.md` §2 硬规则 17）：缓动一律 `steps()`，时长取 `--motion-*`。
新动效先在这张表里找，找不到再加一行，不另起名字。

| 动作 | 用在哪 | 时长 / 帧 | 降级（减少动效 / 像素动效关） | 落点 |
|---|---|---|---|---|
| 按压 | 按键、导航波段键 | `--motion-instant` · `steps(2)` | 保留（本来就是瞬时） | `.pxl-btn` 的 `translate` |
| 换台 | 路由切换 | 溶解 `--motion-normal`；桌面档顶栏刻度尺的指针走到新分组 `--motion-normal` · `steps(8)`，手机档是顶端刻度条上扫过的指针 `--motion-slow` · `steps(8)` | 溶解归零；指针直接到位，顶端刻度条不渲染 | `.motion-page`、`.pxl-scale-track`（`TuningScale`）、`.route-sweep`（手机档） |
| 闪烁 | 只给指示灯（8–16px） | 1s 周期 · `steps(1)` | 常亮 | `.pxl-led[data-state="blink"]` |
| 调谐 | 推子、旋钮 | 不补间：每次输入直接跳到新的一格（刻度灯 / 指针吸附在格上） | 同左 | `ui::Knob`（`pxl-knob-*`）、`.ui-slider` 的刻度与红指针 |
| 扫描 | 站内搜索 | 输入未落定时刻度尺指针来回扫（1.6s · `steps(16)`）+ 标题栏琥珀灯闪 + 读数屏「SCAN」 | 指针停在左端、灯常亮，文字照常 | `.pxl-scale[data-sweep]`、`components/search_dialog.rs` |
| 锁定 | 答对、搜索命中 | 题卡内沿亮两下绿：`--motion-normal` · `steps(1)` 四段；练习读数屏的 LOCK 灯亮；搜索命中转绿灯 + 「LOCK」+ 七段条数，指针停在第一组 | 不播，结论在判分文字与绿灯 | `.pxl-window[data-signal="lock"]::after`、`PracticeRig` |
| 干扰（QRM） | 答错；抗干扰 / 路线上的干扰形象 | 答错：网点噪声跳三帧（`--motion-normal`）+ 练习读数屏的 QRM 灯亮；干扰频谱：两帧轮换 0.6s · `steps(1)`，滚出视口暂停 | 答错不播；频谱停在一帧静图 | `.pxl-window[data-signal="qrm"]::after`、`.pxl-qrm-frame` |
| 衰减 | S 表峰值保持回落 | 每格 80ms · `steps(1)` | 不画峰值保持 | 峰值保持已有（`SignalMeter` 的 `peak`，琥珀格），但它是真实记录的最好成绩、不会「回落」，所以没有衰减动画 |

闪烁频率守规则 5（≤ 3 次 / 秒）；**循环闪烁只给指示灯**，读数、文字与大面积色块永远不闪（倒计时告急时闪的是旁边的灯，不是数字）。
判分的「锁定」是一次性的两下 2px 内框，干扰频谱的两帧轮换是形状在变、不是明暗闪 —— 都不算例外。

## 三、token 速查

| Token | 值 | 用途 |
|---|---|---|
| `--ease-out-expo` | `steps(4, end)` | 出场 / 入场缓动（像素主题里缓动也是跳格，值在 `tokens.css`） |
| `--motion-instant` | `0.08s` | 按下 / 释放这种「立刻」的反馈 |
| `--motion-fast` | `0.12s` | 微交互、hover / active 反馈 |
| `--motion-normal` | `0.24s` | 常规过渡、弹层进出场 |
| `--motion-slow` | `0.48s` | 页面入场、路由读条 |

## 四、现代风（`style/modern/`）

本文前三节写的是像素风（`steps()` 逐格跳）。现代风是另一套节奏，规则同样要守：

| Token | 值 | 用途 |
|---|---|---|
| `--md-dur-fast` | `120ms` | 控件的悬停 / 状态过渡（描边、底色、勾选）、悬停气泡、下拉面板 |
| `--md-dur` | `180ms` | 对话框 / 抽屉 / Toast / 菜单的进出场、按钮的悬停 |
| `--md-dur-slow` | `320ms` | 较大的位移 / 版式变化 |
| `--md-ease` / `--md-ease-out` | `cubic-bezier(…)` | 过渡 / 入场用 ease-out，退场用 ease（**例外**：抽屉的退场用 `--md-ease-out`，见下一条） |

- **抽屉退场用先快后慢的 `--md-ease-out`。** 在目录里点链接时，退场与新页面挂载同时进行，主线程会被新页面占住一两百毫秒，
  抽屉的不透明度只能停在某一帧上：缓入曲线那时还剩约 60%，新旧两张页面半透明地叠在一起；缓出曲线第一帧就降到三成以下，
  被冻住的也只是一层很淡的残影。卸载时机仍由 `presence` 按 `--md-dur` 决定，不受曲线影响（实现见 `kit.css` 的 `.md-sheet[data-state="closed"]`）。

- **进出场的时长一律走 token**，不写 `180ms` 这样的字面量。装饰性的长动画（氛围循环、一次性演出、涟漪）可以写字面量。
- 现代风**不用 `steps()`**，像素风**不用平滑缓动**；两套皮肤的动画不要互相借用（现代风的单选框曾借用像素风的
  `motion-check`，逐格跳的动画漏进了平滑的界面里）。
- 减少动态效果：`tokens.css` 的 `@media (prefers-reduced-motion: reduce)` 把 `.md-root` 与各个 Portal 出来的表面
  （对话框 / 抽屉 / 调谐台 / 气泡 / Toast / 提示气泡 / 页内菜单的悬浮按钮）的时长压到 1ms。
  **新增一个 Portal 表面，要把它的根类加进这份名单。**

## 五、弹层的进出场：一份「在场」实现，一个时长 token

两套皮肤的弹层（下拉 / 日期 / 时间 / 对话框 / 抽屉 / Toast / 提示气泡 / 现代风的六个表面）共用 `src/presence.rs`：
关闭后元素先留在 DOM 里播完退场动画，再卸载。几条硬规矩，每一条都踩过：

1. **退场是动画，不是过渡。** 入场动画与 `data-state` 在同一次样式变更里被移除时，浏览器不会从动画值起一个过渡，
   `transition: opacity` 因此从来没播过 —— 像素风的面板在关闭后第一帧就是透明的（逐帧实测）。
   写 `[data-state="closed"] { animation: …-out <token> … both/forwards }`。
2. **一个 token，两个使用者。** 退场动画的时长与 Rust 里的卸载延迟读**同一个** CSS 变量：

   | 档位 | 现代风 | 像素风 | 谁用 |
   |---|---|---|---|
   | `ExitSpeed::Normal` | `--md-dur` | `--motion-normal` | 对话框 / 抽屉 / 下拉 / 日期 / 时间 / Toast / 现代风六个表面 |
   | `ExitSpeed::Fast` | `--md-dur-fast` | `--motion-fast` | 悬停气泡 |

   `presence::exit_ms(speed)` 在每次关闭时现读（用户运行中切风格，两套 token 的值不同），再加一帧。
   退场动画的时长只能是这两档的 token（`check-structure` 的「退场时长走 token」守着，写死 `180ms` 会红）。
   **不要**在 Rust 里再写 `EXIT_MS` 常量（「延迟卸载只有一份」守着）。
3. **退场期间不拦截点击。** `data-state="closed"` 的遮罩与面板都是 `pointer-events: none`
   （下拉遮罩与面板类里的 `data-[state=closed]:pointer-events-none`、`.md-modal` / `.md-sheet` / `.md-bubble` 的关闭态规则）。
   否则用户松手后紧接着点别处，那一下会被一层看不见的全屏遮罩吞掉约 200ms。
4. **代际号，不是只看 `open`。** 计时器到点时 `open` 恰好又是 `false`（关 → 开 → 关）会把第二次退场截断在半路；
   打开与关闭都要让代际号自增，计时器只在代际号没变时才卸载。
5. **定时器回调里只用 `try_*` 访问信号。** 退场途中跳转页面会销毁信号，`get_untracked` 会 panic 带崩整个 wasm 实例。
6. **退场期间的三个属性成套写，取值只有一份。** 面板**与遮罩**都要写 `data-state` + `inert` + `aria-hidden`
   （`inert` 把元素移出无障碍树和焦点序；Playwright 的角色定位器与部分读屏只认 `aria-hidden`，所以两个一起写）。
   取值收在 `presence.rs`，**不要**在调用点内联 `if closing { "closed" } else { "open" }`。按手里有什么选：
   持有 `Presence` 的（现代风 Kit）用 `presence.state()` / `.inert()` / `.aria_hidden()`，以 `closing` 为判据；
   只有 `open` 信号的（`ui/` 下的下拉 / 日期 / 时间 / 对话框 / 抽屉）用 `presence::state_attr(open)` / `inert_attr(open)` /
   `aria_hidden_attr(open)`，关闭的那一刻立即生效。两族判据不同、取值相同。

## 六、控件状态矩阵

同一个控件在两套皮肤下，每种状态都要有反馈。下表是**实现后在浏览器里量到的**（`e2e/tests/ui_motion.spec.ts` 逐项守着）：

| 状态 | 像素风 | 现代风 |
|---|---|---|
| 悬停 | 80ms `steps(2)`：描边 / 底色 / 位移 | `--md-dur-fast` 平滑：描边 / 底色（输入框、下拉触发器、勾选、分页、页签、胶囊）；按钮 `--md-dur`（含主按钮的扫光，有意的设计） |
| 按压 | 2px 下沉（按钮 / 分页） | 涟漪 `md-ping`（按钮 / 胶囊 / 图标按钮 / 分页，**禁用的不播**）；整块表面是声呐环 |
| 键盘聚焦 | 2px 虚线环 | 2px 实线环 |
| 禁用 | 按钮：网点遮罩 + 降饱和；输入 0.7；勾选 / 开关 / 滑块 0.5 | 一律 `opacity: 0.5` + `not-allowed`，主按钮去掉橙色投影；**悬停 / 按下 / 涟漪全部排除** |
| 错误 | `aria-invalid` → `--destructive` 描边（输入 / 下拉 / 勾选 / 开关 / 单选组） | 同，`--md-danger` |
| 勾选 / 选中 | `check-pop` 120ms 逐格 | `md-check-pop` 120ms 平滑（复选框与单选框同一个） |

- `aria-invalid` 是唯一的事实来源：组件用 `invalid` 入参（`Checkbox` / `Switch` / `Input` / `Select` / …；单选的属性挂在
  `radiogroup` 上、样式往下传），CSS 只认属性。**`Slider` 没有 `invalid`**：连续量没有「校验未通过」这个状态，不是遗漏。
- 每个变体的悬停规则各写各的，**新增按钮变体时悬停 / 按下规则必须带 `:not(:disabled)`**（`.md-btn-accent` 曾漏掉，
  于是禁用的主按钮悬停时仍会扫光、上抬）。
- 现代风的 Tab / 复选框 / 开关 / 滑块**没有**涟漪，这是有意的：它们是「状态切换」而不是「去做某事」，
  涟漪会让一排切换控件显得吵。
