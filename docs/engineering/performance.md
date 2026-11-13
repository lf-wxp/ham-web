# 性能：测量口径、规则与守卫、常驻缓存

> **管什么**：主线程响应（点按 → 画面变化）怎么测、阈值是多少；性能规则由哪条用例守着；评估后决定不做的项；
> 常驻缓存清单；主包体积预算与分包裁定。
>
> **不管什么**（只给指针）：检查项见 [`code-quality.md`](code-quality.md) §8 的 F3；写法细则见 [`docs/ui/motion.md`](../ui/motion.md) §7；
> 「点击反馈先画出来、再做重活」见 [`docs/ui/interaction.md`](../ui/interaction.md) §2。
>
> **历史**：2026-11-07 之前的问题台账（逐页「修前 → 修后」数字、根因、修法）与复核台账已从本文移出，
> 要翻旧数字用 `git show 7ee9e6a:docs/engineering/performance.md`（§2 总表与条目、§3 复核台账）。

---

## 1. 测量口径

### 1.1 档位

| 维度 | 取值 | 为什么 |
| --- | --- | --- |
| 构建 | 发布构建（`cargo make build-web`）+ release 服务器 | 开发构建慢一个量级，热点比例可参考，绝对值不可用 |
| 设备 | Playwright `iPhone 13` 视口；Chromium **4 倍 CPU 降速**为主，WebKit 不降速交叉验证 | 4 倍降速接近中端手机；WebKit 是 iOS Safari 的内核 |
| 进入方式 | 站内导航（点链接，不整页刷新），**首次**（本会话第一次进该页）与**再次**各测一遍 | 首次含代码首次执行；再次才是「页面本身的渲染」 |
| 数据 | **空数据**；**重度用户**：错题本 1000 条（`MAX_RECORDS`）+ 通联日志 5000 条 | 空数据测不出「用得越久越慢」 |
| 重复 | 每页每构建至少 7 次取中位数 | 单次被机器负载左右 |

开发构建保留函数名，用来做 CPU 采样（`Profiler.start`，100µs 间隔）找热点；发布构建只用来量时间。

**浏览器必须带真 GPU 启动**：`channel: "chromium"` + `--use-angle=metal --ignore-gpu-blocklist --enable-gpu-rasterization`
（Linux 换对应的 ANGLE 后端）。Playwright headless 默认走 SwiftShader（CPU 模拟 GPU），创建 WebGL 上下文、提交画布在里面极慢：
`/nec` 每次进页面都有约 100 ms 的主线程 `Commit`（在等 GPU 线程 83 ms 的 `GPUTask`），真 GPU 下是 0。
不带参数，含 WebGL / 大画布的页面会被测成假的长任务；别去「修」它。

### 1.2 指标与阈值

| 指标 | 定义 | 阈值（4 倍降速） |
| --- | --- | --- |
| 首帧 | 点击 → 地址变化后的第一帧 | — （用来定位，不设门槛） |
| 长任务合计 | 点击后 1.5 s 内超过 50 ms 的任务时长之和（`longtask`） | 再次进入 = 0；首次进入 ≤ 1 个 |
| 交互延迟 | 一次交互从输入到下一帧（Event Timing `duration`，≈ INP） | 高频交互（作答、翻题、输入一个字）≤ 50 ms |

长任务合计带 50 ms 的阈值，几十毫秒的改进常被噪声淹没。比较两个构建时另取 CDP `Performance.getMetrics` 在 1.5 s 窗口内的
`TaskDuration` / `ScriptDuration` / `LayoutDuration` / `RecalcStyleDuration` 增量，并放几张**没改过的页面当对照**（差在 ±8 ms 内才算稳）。
布局、样式不变而脚本降了，是「DOM 形状没变、只是造得更省」的直接证据。

### 1.3 工具

- **真机**：任意地址加 `?perf=1` 打开测量面板（`crates/app/src/perf_hud.rs`），显示每次点按的「首帧 / 换页 / 最长卡帧」。
- **回归**：`e2e/tests/responsiveness.spec.ts` 守结构性判据（重活是否推到首帧之后、同一份数据是否只解析一次），不断言毫秒数；规则与用例的对照见 §2。
- **全站扫描**：按 `crates/app/src/app/main_content.rs` 的路由表逐条站内进入；脚本不入库（一次性测量），口径以本节为准。
- **独立目录构建**：测量用独立目录，别和 `cargo make e2e` / `dev-full` 抢 `dist/`（并行构建同一目录会写出混版本产物）。
  `trunk build --release --dist <目录>` → `cargo run -p ham-web-tools --release -- postbuild --dist <目录> --site-url https://ham.onlyxp.me`
  → `DIST_DIR=<目录> HOST=127.0.0.1 PORT=<端口> target/release/ham-web-server`。
  采样用的开发产物是 `trunk build --dist <目录>`，再把 `dist/data/knowledge-i18n` 拷进去（那是 `postbuild` 才生成的）。
- **定位手法**：采样只告诉你「哪个函数」，下面两招告诉你「是什么」。① Chrome timeline 追踪（`Tracing.start`，`devtools.timeline`）：
  看 `Layout` / `UpdateLayoutTree` 是否嵌在脚本调用里（强制布局）、脏节点比例，以及主线程在等时其它线程在做什么；
  ② A/B 注入 CSS（`addInitScript` 里在 `DOMContentLoaded` 后塞 `<style>`）：把字体、`text-wrap`、滤镜、`contain` 逐个关掉，看最大那次 `Layout` 怎么变。

### 1.4 基线与未验证项

2026-11-09 在 §1.1 的档位下扫过全站 180 条路由，作为以后对照的水位：

- **再次进入**：全站长任务合计均为 0（重度用户的错题本、日志、统计、题库浏览、术语表也是 0）。
- **首次进入**：约 25 条路由仍有 1–2 个 50–140 ms 的任务，没有单一热点。较重的：`/nec`（约 200，含首次矩量法求解）、
  地图类（110–180，布局占大头）、`/waveform-lab` `/morse` `/gear` `/`（100–150）；重度用户的日志页首次约 345。
- **未验证**：① 以上都是模拟降速，**真机没验证**；② `ime_gate` 的事件顺序只在 Chromium 里用合成事件验过，
  合成的 `isComposing` 与真实输入法的时序可能有出入，上线后在 iOS Safari 与桌面 Firefox 上用拼音各手测一次「天线」。

---

## 2. 规则与守卫

写法细则在 [`docs/ui/motion.md`](../ui/motion.md) §3、§7，检查项在 [`code-quality.md`](code-quality.md) §8 的 F3；
下表只回答**哪条规则由什么在守**。「—」表示没有自动化守卫，靠评审。用例标题省略了后半句，按前缀搜得到。

| 规则 | 落点 | 守卫 |
| --- | --- | --- |
| 首屏以下的重块在首帧之后才挂 | `components::common::Deferred` | `responsiveness.spec.ts`「日志页：首屏以下的统计明细…」「日志页：记录列表在首帧之后才挂…」「现代风换页：首屏内容不会先显示再变暗…」「「更多」目录：打开的第一帧只挂第一块入口…」 |
| 几块都重，各排各的任务 | 每块各包一层 `Deferred` | 「/nec：结果区、三维方向图、扫频、传输线、NVIS、方向表不在同一个任务里挂上」 |
| 长列表键控 + 分批补齐，补齐只追加 | `after_paint::grow_cap`、`<For key=…>`、`components::common::LoadMore` | 「错题本：首帧只建前几张卡…」「术语表：首帧只建前几张卡…」「题库浏览：首帧只建前几行…」 |
| 补齐循环必须能结束（不靠外部状态、页面隐藏时不递归）；数据没到不提前走完 | `after_paint::grow_cap`、core `list_grow::next_cap`；异步数据的页面等数据到了再启动 | 「补齐循环必须能结束」下三条（隐藏页 × 题库在路上、隐藏页 × 空错题本、题库慢到仍分批）；`list_grow` 单元测试（任何步长 / 目标都在有限步内结束） |
| 只建当前宽度 / 当前面板那一套 | `media::use_media_query`、`ui::Tabs` | 「分页签：只挂当前面板…」「/bands：桌面表格与手机卡片只建当前宽度那一套…」 |
| 世界地图三份平铺，首帧只建中间那份 | `map/map_view.rs`、`projection::tile_build_rank` | 「世界地图：首帧只建中间那份…」；`tile_build_order_is_centre_first_then_left_then_right` |
| 地图区域叠加层拼标记、点击委托、节点形状不变 | `pages/map/svg_markup.rs` | `maps.spec.ts`「分区地图：点多边形与点无几何实体的圆点…」「DXCC 世界地图：点实体跳到日志…」「区域叠加层：每块区域各带一个 <title>…」；`svg_markup` 转义测试、`polygon_points` 格式测试（数据里没有名字带 `&` 的实体，转义只靠单元测试守） |
| 搜索框组字期间不派发；非搜索框不挡 | `composition.rs`、core `ime_gate` | 「输入法组字期间不重算搜索…」「Safari 的事件顺序…」「非搜索的输入框不受组字门控…」；`ime_gate` 单元测试 |
| 作答不整本读写错题本与日志 | `study::load_book`、`achievements::track_logbook` | 「作答：答对不在错题本里的题不整本写回错题本…」 |
| store 级派生缓存跟着数据失效；统计口径每次取用时对一次存储（另一标签页改了口径，换页就跟上） | `pages/log/derived.rs`、`stats_scope_select::use_stats_scope` | 「记一条新实体的通联后，站内跳到统计页立刻看到新数字」「统计口径：另一个标签页改了口径…」 |
| 同一份题库只下载、解析一次；空闲预热 | `data::load_url`、`bank_updates::prewarm` | 「同一份题库同时只请求一次…」「空闲时预热 A 类题库…」 |
| 像素风首页题库选择器不为点亮按钮下载 B / C 类 | `data::check_version` | 「像素风首页：题库选择器不为点亮按钮下载 B / C 类题库」（开发构建跳过：没有修订号） |
| 滚出视口暂停循环动画，只点名无限循环的那几条 | `motion::pause_offscreen_in`、`kit.css` | 「首页：首屏滚出视口后暂停循环动画…」；入场动画被误停时 `settleFrames` 会超时 |
| 服务端不启用 `dxcc` 记忆表；超长输入不进表 | `core/dxcc.rs` | `memo_capacity_and_bypass_do_not_change_results`、`native_builds_do_not_memoize` |
| 搜索匹配的 ASCII 路径不分配 | `core/logbook.rs`（`upper_contains`） | `upper_contains_matches_to_uppercase_contains` |
| 帧循环滚出视口停自调度；定时器读存储走缓存 | `kit_modern/spectrum.rs`、`pages/countdown.rs` | — |
| 新缓存登记 §4（上限、失效条件） | 本文 §4 | — |

### `Deferred` 占位高度

`min_height` 取**各皮肤、各宽度、各数据状态下实测高度的下限**（占位比真身高，挂上时下面的内容会往上缩）。
下面是 2026-11-09 的实测（开发构建，不降速，Chromium；桌面 1280 × 800 / 手机 390 × 844；像素风与现代风各一遍；
脚本不入库，量的是 `Deferred` 里所有块的总高：第一块的上沿到最后一块的下沿）：

| 位置 | 取值 | 实测 | 为什么是这个值 |
| --- | --- | --- | --- |
| 统计页，「DXCC 奖项进度」之后到页脚说明之前的全部块（`stats_page.rs`） | `90rem` | 缺口清单在：170.8（像素 · 桌面 · 1 条）— 364（现代 · 手机 · 200 条）；缺口清单没了：92.4 — 105.3 | 缺口清单列的是**没通联过的实体**，随日志增长而缩短，通联满了就整块消失（80.6 rem 桌面 / 260.6 rem 手机）。取「清单没了」时的下限向下凑整。页脚说明在它下面，所以这块**要**占位 |
| 日志页记录列表（`log_page.rs` 的 `list_placeholder`） | `14rem`；日志为空传空串 | 1 条：14.5（现代 · 桌面）— 20.5；2 条：16.9 — 22.9；60 条（一页）：134.8 — 144.6 | 取只有一条记录时的下限；日志为空时列表只是个空态，不占位 |
| 日志页统计明细 / 体检面板（`log_stats_panel.rs`） | `52rem` / `6rem` | 164 — 173 / 6.1（2026-11-07 实测，本次未复测） | 沿用 |

数据状态变了（缺口清单变短、日志条数变化）要回来复测；`Deferred` 随外层闭包重建（如统计页切换口径）时，占位会短暂比真身矮，
下面的内容跟着晃一下，这是「取下限」的代价，不是 bug。

---

## 3. 已评估、决定不做

查过、有数字、有理由的。没有新证据就别重复排查。

| 项 | 结论与理由 | 重开条件 |
| --- | --- | --- |
| 启动时同步解析整本日志快照 | 重度用户冷启动多约 140 ms。改成「先给空日志、首帧之后再填」有**数据丢失窗口**：这期间任何一次编辑都会把只含一两条记录的日志写进权威层（IndexedDB）覆盖整本 —— `provide_log_store` 的仲裁按「内存里一开始就是完整日志」写 | 权威层写入改成带版本的追加 |
| 页面里「提前」的同步布局读（`scrollY` / `offsetHeight` / `getBoundingClientRect`） | 采样里自身耗时很大（`/practice` 28 ms、`/exam` 21 ms、首页 `scrollY` 42 ms），但追踪显示那是**新页面第一次布局本来就要做的活**被提前到脚本里，不是重复布局（练习页 6 次 Layout 共 26 ms）。去掉只是把布局挪回渲染管线，总量不变。首页英雄区首帧同步读 `scrollY` 是有意的：刷新后恢复滚动位置时不闪一帧无视差的画面 | 追踪里出现同一帧内多次 Layout，或脏节点比例持续接近 100% |
| 首页首次进入的网页字体开销 | 首次布局约一半来自网页字体本身（最大一次 Layout 45 → 22 ms）：第一次整形时建字体对象、解析字形表。与字体栈长短无关（只留 3 项 37、去掉系统黑体 45），字体已预加载。要省只能换系统字体，等于放弃设计里的字面，且每会话只发生一次 | 设计上放弃自托管展示字体 |
| 地图页的布局（94–125 ms） | 来自上千个 SVG 节点本身；叠加层拼标记后脚本降了 20–35 ms，布局不动。再降只能减少节点数（合并同一实体的多个环、按缩放简化几何），那是改地图内容 | 首次进入的地图页在真机上被反馈卡 |
| `/nec` 首次矩量法求解 | 求解是一个整块（装阻抗矩阵 + 扫 2664 个方向，约 140 ms）。已让它与其余五块各排各的任务，最长任务 193 → 138 ms。再拆要改 core 里的矩量法 | 首次进入 `/nec` 在真机上被反馈卡 |
| 日志页首次进入的体检 / 奖项 / 申请表首算（约 345 ms） | 第一次都要算，只是再次进入不重复。可用 `after_paint::when_idle` 空闲时预算，代价是启动后多占一次主线程 | 重度用户首次进日志页在真机上被反馈卡 |
| 日志搜索，键间隔略大于防抖（约 160 ms） | 下一键撞上防抖后那次重算，单键交互中位 72 ms（键间隔 60 ms 时 32 ms，无长任务）；整段输入的长任务合计仍是最低的 | 防抖时长改动，或搜索改成增量 |
| 按路由拆分 wasm | 收益上限 br 61.5%，但拆不动的框架地板占 56%；Trunk 也没有插 `wasm-split` 的位置。Leptos 0.8.5 自带的 `#[lazy_route]` 2026-11-09 按两种粒度实测后结论不变 | 见 §5.5 |

---

## 4. 常驻缓存清单

会话级缓存在内存里，页面卸载后也不释放（`thread_local!` 或挂在应用根的 store 上，与应用同寿）。体积按随包文件大小**估算**（未实测堆占用）。
新增同类缓存时在这里补一行，写明上限与失效条件，并配一条「数据变了，数字跟着变」的用例（见 §2）：

| 缓存 | 位置 | 内容 | 上限 / 估算体积 | 失效 |
| --- | --- | --- | --- | --- |
| 站内搜索的题目检索文本 | `components/search_dialog.rs`（`QUESTION_TEXTS`） | 约 3500 条「题干 + 解析」小写拼接串 | 一份；≈ `search-index.json` 的 1.7 MB | 索引换新（`Arc::ptr_eq`）就重建 |
| 题库浏览的检索文本 | `pages/browse/browse_page.rs`（`search_texts` Memo） | 当前加载题库每题「题干 + 选项 + 解析」小写串 | 随页面卸载释放；三个题库全载时 ≈ 3.5 MB | 题库数据变就重算；第一次输入关键词才建 |
| 题库覆盖率的题目 key | `data.rs`（`BANK_KEYS`） | 每个题库一份 key 集合（约 3500 个短串） | 最多 6 份；< 0.5 MB | 题库重载是另一个 `Arc`，旧的按先进先出挤掉 |
| 地图几何解码结果 | `data.rs`（`BINARY`） | `dxcc-entities.bin`、`cn-provinces.bin` 解码后的 `Arc` | 两份；随包 12 KB / 7 KB，解码后更大，**未测** | 不失效（随包文件） |
| 错题本 / 学习统计 / 单题统计 / 每日记录 | `study.rs`（`BOOK`）、`util::storage`（`PARSED`） | 解析结果 + 原串 | 每个 key 一份；错题本上限 1000 条，原串约 600 KB，解析结果同量级 | 原串变了就未命中（备份导入 / 跨标签页写入也一样） |
| DXCC 查找记忆 | `core/dxcc.rs`（`MEMO`） | 呼号 → 实体下标 | 仅 `wasm32`；≤ 5 万条，键 ≤ 32 字节 | 满了整表清空 |
| 公式渲染结果 | `index.html`（KaTeX `cache`） | TeX 原文 → HTML | 公式是静态的，上百条 | 不失效 |
| NEC 最近一次求解 | `pages/nec/nec_page.rs`（`LAST_SOLVE`） | 输入、传输线、结果 | 一份；含方向图采样，**未测** | 输入变了就重算 |
| 日志的派生统计 | `pages/log/derived.rs`（`LogDerived`，挂在 `LogStore` 上） | 统计口径、按口径过滤后的日志（**一份日志的完整副本**）、概览、奖项进度、DXCC 申请表、体检报告 | 惰性：没进过日志 / 统计 / 地图页就不占；5000 条日志时过滤副本 ≈ 日志本身（解析后几 MB），**未测** | 日志、台站档案或口径变了就标脏，下次读重算；体检报告带「算它时的时间」，有「时间在未来」的条目时过期自动重算 |

---

## 5. 主包体积：现状、预算与分包评估（2026-11-08，2026-11-09 补测 §5.5）

### 5.1 实测

| 项 | 值 | 口径 |
|---|---|---|
| 主包 wasm（raw） | **8.44 MiB**（8,846,892 B，2026-11-12） | `trunk build --release`，与 `dist/` 里的产物逐字节一致 |
| 主包 wasm（br） | **2.20 MiB**（2,301,656 B，2026-11-12；预算 2,500,000 B，余量约 7.9%） | postbuild 预压缩，线上实际传输 |
| 主包 JS 胶水（br） / 样式表（br） | 16 KB / 53 KB（54,575 B） | 样式表预算 55 KB |
| 首访字体（现代风 / 中文） | 6 个文件 / 234 KB | `performance.getEntriesByType("resource")` 实测 |
| 首访字体（现代风 / 英西） | 5 个文件 / 133 KB | 中文展示字按语言预载后省掉 101 KB |

构成：数据段 1.64 MB raw / br 0.45 MB（占整包 br 的 22%），是**编译期嵌进 wasm 的静态表**（中文词典、DXCC 实体表、法规原文、知识库索引），
首屏同步查表要用，不是「可以拖到后面的代码」；代码段 6.43 MB raw / br 1.59 MB。归因做法与结果见 §5.4。

### 5.2 已落地的预算与预载

- **主包预算 2,500,000 B 收紧到 2,260,000 B**（收紧当时实测 2,128,947 B，余量 ~6%；当前实测见 §5.1；本节的 MB 指 MiB，预算文件里是字节）。原来 23% 的余量等于允许悄悄长 400 KB 而无人发现。
- **2026-11-12 又放回 2,500,000 B**：P.372 大气噪声的系数表（`f64` 原样，原始 306 KiB、br 后约 91 KiB）在不损精度的前提下压不动，验收口径是与官方程序逐位一致；放宽后余量约 7.9%，不是回到「悄悄长 400 KB」的口径 —— 仍然每次上调前先问「能不能拆进 Worker」，记录见 ROADMAP「工程门禁 · 体积」。
- 字体预算进 `perf-budget.json`（`raw: true` 口径：woff2 已压缩，postbuild 不生成 `.br`）。
- 中文展示字（100 KB）**按语言预载**：英 / 西语界面首访字体从 241 KB 降到 133 KB。

### 5.3 路由分包：三条路与裁定

Leptos CSR 只有一个 wasm 实例，路由是客户端路由。0.8.5 之前没有「进到某页才下载那页 wasm」的能力；0.8.5 起框架自带了
（`LazyRoute` + `#[lazy_route]`，底层是 wasm-split）。2026-11-09 按「单页」与「10 块聚合」两种粒度都量过，**结论仍是不做**，数字与坑见 §5.5。
除它之外还有三条路：

| 方案 | 做法 | 代价 | 什么时候值得做 |
|---|---|---|---|
| **A. 多 wasm 模块** | 重功能区（解码器、地图、考试引擎）编成独立 `cdylib`，进对应路由时由 JS 实例化 | 跨模块传 Leptos 信号与 `web_sys` 对象几乎不可行，等于拆成微前端；**月级**。收益上限实测 br 61.5%，见 §5.4 | 主包涨到 2.6 MB 以上且首屏 LCP 被带宽卡住 |
| **B. 重活挪进 Worker** | 解码已经这么做；可继续挪题库检索 / 统计聚合 / 地图投影 | 中等；只减少**主线程长任务**，**不减少字节** | 长任务超标时（阈值见 §1.2） |
| **C. SSR / 岛屿** | Leptos SSR + 按路由 islands | 要服务端渲染管线 + 首屏 HTML 缓存，重构量最大；首屏字节数是**数量级**下降 | 有稳定后端与运维预算 |

**裁定：不做 A。** 拆代码带不走框架地板，也带不走默认语言词典。框架自带的 wasm-split（§5.5）同理，只是把「拆不动的地板」换成了「碎片化的请求」。
重开时先做 C —— 它是唯一能把框架地板一起砍掉的路。

### 5.4 分包测量的结果（2026-11-08）

> 一次性测量，脚本不入库。br 一律指 `brotli -q 11`（与 postbuild 的预压缩相差约 4 KB），只用于 A/B，不与 §5.1 的数混用。

**crate 级归因。** `cargo bloat` 在本仓库不可用（wasm 报 `unsupported file format`，它只解析 ELF），改用 `twiggy top` 按 `crate[hash]` 前缀汇总。
release 产物是 strip 过的，先造带符号的：

```bash
CARGO_PROFILE_WASM_RELEASE_STRIP=false CARGO_PROFILE_WASM_RELEASE_DEBUG=1 \
  cargo build --profile wasm-release --target wasm32-unknown-unknown -p ham-web-app
twiggy top target/wasm32-unknown-unknown/wasm-release/ham-web-app.wasm > /tmp/top.txt
```

浅层字节：`ham_web_app` 3.28 MB（181 个路由页面 2.17 MB + 共享外壳 1.11 MB）、`alloc` 1.69、`reactive_graph` 1.03、`core` 0.96、`tachys` 0.44、
`ham_web_core` 0.38；**非本仓库的框架地板 4,733,302 B，占 56%，与路由无关**。56% 的分母是这份带符号构建（浅层共 8,390,015 B），
不是 §5.1 的生产代码段（6,431,937 B），两个数不可相除。

**数据段。** 生产产物 5,727 段 / 1,640,513 B / br 472,426 B。最大一块是**中文词典**（469,722 B raw / 170,910 B br，`build.rs` 对默认语言全量内嵌），
其次是法规原文（61,575 B）和 DXCC 实体表（47,771 B）。词典属于「默认语言同步查表」的设计前提。

**删减实验**（同一工作区三次 `trunk build --release --dist target/rel-*`）：

| 构建 | raw wasm | br | code 段 | data |
|---|---|---|---|---|
| 基线 | 8,234,910 | 2,126,177 | 6,431,937 | 1,640,513 |
| 去掉 `/nec` 一页 | 7,926,393 | 2,046,153（**−3.8%**） | 6,149,829 | 1,622,060 |
| 只留外壳（摘掉 179 条路由） | 2,681,727 | 819,239（**−61.5%**） | 1,696,708 | 939,618 |

两个反直觉的点：

1. **符号归因会低估一半以上。** `/nec` 在 `twiggy top` 里只挂 140,455 B，真删掉少了 282,108 B 代码（2.0 倍）：泛型实例化与被内联的辅助函数记在**定义它们的 crate**
   名下，只有删掉调用方才真消失。「某页值多少字节」只能靠删减实验量。
2. **数据段也有一部分是路由本地的。** 摘掉 179 条路由后数据段从 1.64 MB 掉到 0.94 MB（术语表、法规原文、DXCC 表只有特定页面用）。

**为什么 A 拿不到上限。** 收益上限 1,306,938 B br（61.5%）是「179 条路由的代码与路由本地数据全不在首包」。但 Leptos CSR 模块间不能共享信号与 `web_sys` 对象，
每拆一个页面模块都要连框架副本一起打（外壳 819,239 B br 里代码占 490,513 B），单页首访要下两份。单页最贵的 `/nec` 只占 3.8%，平均 7.3 KB br。

#### 实施尝试：实测否决（当日撤销）

更便宜的候选是把默认语言词典改成运行时语言包（内嵌首屏五域 `common` / `shell` / `home` / `exam` / `learning` + 一份精简的「中文原文 → key」反向表，构建期扫源码命中 882 条）。
实现后**净亏**：wasm br −65,458 B，新增 `data/i18n/zh.json` +94,013 B br，**中文首访合计 +28,555 B**（en / es 首访 −65,458 B）。

原因是**拆开 brotli 流本身有代价**：现在中文译文与外壳字符串共享同一段压缩历史，挪进独立资源后各流从零建字典，同样的文字多占四成左右。
即便把反向表算成零开销，94 KB 仍大于 65 KB；另有多一次网络请求（`#boot` 期间要等它）和包没到时的「漏语义 key」降级态。

**结论**：默认语言继续全量内嵌，`build.rs` 的 `BUNDLED_DOMAINS` 保持只对非中文生效。只有**换编码**（二进制语言包、或与 wasm 共享压缩字典）时才可能翻正，那已不是「打包」问题。

### 5.5 路由级 wasm-split 实测（2026-11-09）

Leptos 0.8.5 起框架自带懒加载：`leptos_router::LazyRoute` + `Lazy<T>`，`#[lazy_route]` 把 `view` 交给 `#[lazy]`，后者展开成 `wasm_split!`
（本仓库当前锁定 `leptos 0.8.23` / `leptos_router 0.8.17` / `wasm_split_helpers 0.2.3`；单页一段测于 `0.8.21` / `0.8.16`，10 块一段测于当前锁定版本）。它**不是** §5.3 的方案 A：拆出的块与原模块共享同一份线性内存与同一个 Leptos 运行时，
不存在跨模块传信号 / `web_sys` 对象的死结，所以不能拿 A 的结论直接套 —— 这里单独量。

> 一次性测量，脚本不入库（与 §5.4 同一约定）。口径：`wasm-release` profile + 测量时的 `wasm-bindgen 0.2.129`（当前锁定 `0.2.130`；`--target web`）+ `wasm-opt 133 -Oz`
> （与 `index.html` 的 `data-wasm-opt` 同参数）+ `brotli -q 11`。A 组（未拆分）在同一份源码上复算 br **2,126,616 B**，与 §5.4 基线差 0.02%，可用。
> 单页一段在 0.6.5 上测，10 块一段在 0.6.6（`e7e12d3`）上测，两段各自带同版本基线 —— 版本差约 0.1%，不影响结论。

**构建链。** Trunk 的内置步骤是 `cargo build → wasm-bindgen → wasm-opt`，钩子插不进中间，只能自己串；`wasm-split` 必须在 wasm-bindgen **之前**：

```bash
# .cargo/config.toml 的 rustflags 被环境变量覆盖，所以要原样带上；--emit-relocs 是 wasm-split 的硬要求
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="-C debuginfo=1 --cfg erase_components -C link-args=--emit-relocs" \
  cargo build --profile wasm-release --target wasm32-unknown-unknown -p ham-web-app
wasm-split-cli target/wasm32-unknown-unknown/wasm-release/ham-web-app.wasm /tmp/split --target web --out-name ham-web-app
wasm-bindgen --target web --no-typescript --out-dir /tmp/split --out-name ham-web-app /tmp/split/ham-web-app.wasm
```

不加 `--emit-relocs` 直接报 `No linking section found`。代价：中间产物 11.6 MB → 26.3 MB（0.6.6 加回 name 段后 34.7 MB），改 RUSTFLAGS 会触发一次全量重编译（2m23s）。

#### 只拆 `/nec` 一页

| 项 | 未拆分 | 拆 `/nec` | 差 |
|---|---|---|---|
| 主包 raw | 8,229,906 | 7,954,075 | −3.35% |
| 主包 br | 2,124,215 | 2,073,878 | −2.37% |
| chunk（raw / br） | — | 320,791 / 105,918 | 压缩率 3.03（主包 3.83） |
| 非 `/nec` 页首访 br | 2,140,498 | 2,090,894 | −2.3% |
| **`/nec` 首访 br** | 2,140,498 | 2,196,812 | **+2.6%** |

拆一页等于让这一页首访更重，只换来其他页少 2.3%。wasm-split 搬走 372,659 B 代码 + 20,452 B 数据（比 §5.4 删减实验的 282,108 B 多：
只被搬走函数调用的整段代码也跟着走）。浏览器验证：`/` 首屏不请求 chunk，站内跳到 `/nec` 才拉那一个文件，页面正常渲染。

#### 按模块聚成 10 块（181 条路由，每块约 244 KB 源码）

| 项 | 未拆分 | 10 块 | 差 |
|---|---|---|---|
| 主包 raw / br | 8,251,493 / 2,126,616 | 2,670,691 / **800,374** | **br −62.4%** |
| 单页首访 br（含胶水） | 2,142,810 | **1,070,087 – 1,417,575（平均 1,218,713）** | −43% |
| 单页首访 wasm 请求数 | 2 | **168 – 273** | ×100 |
| **全部产物 br（PWA 全量预缓存）** | 2,142,810 | **2,945,866** | **+37.5%** |
| wasm 文件数 | 1 | **482** | |

主包确实掉到 800,374 B，与 §5.4「只留外壳」的 819,239 B 预测吻合 —— 剩下的就是框架地板 + 外壳 + 中文词典。**但块不是「一次只拉一块」**：
块之间共享的函数被 CLI 拆成 471 个独立 `chunk_*.wasm`（中位 3 KB，最大 127 KB），`__wasm_split.js` 从 2,192 B 涨到 156,658 B raw / 4,930 br。
浏览器实测一次页面加载触发 237 次 wasm 请求（`performance` buffer 默认 250 条被打满截断，按 `__wasm_split.js` 的依赖图算是 168–273 个）。
功能正常：`/nec` `/morse` `/log` `/exam` 都能渲染，没有 chunk 加载失败。

#### 为什么还是不做

碎片化不是「块切多了」，而是机制本身：**跨块共享的每个函数都要单独成块**。本仓库 179 个页面共享同一套 UI kit / i18n / store，共享面天生最大，
块数再少也压不住文件数。加上每块独立建 brotli 字典（3.83 → 3.03）把省下的字节吃回去，全量 br 反而 +37.5%，而 PWA 的 `install` 会把它们全部预缓存。
只拆 1–2 个孤立重页时文件数没问题（单页那档只有 1 个 chunk、2 个请求），但那一档已经实测净亏 2.6%。

裁定与 §5.3 一致：**不做**。重开条件不变（主包 > 2.6 MB 且首屏 LCP 被带宽卡住，先做 C：SSR / 岛屿）。

#### 重开时先看这三个坑

1. **table 导出**：`wasm_split_cli_support 0.2.3` 生成的 `__wasm_split.js` 调 `initSync(undefined, undefined)`，并从主模块导出里取 `__indirect_function_table`；
   `wasm-bindgen 0.2.129`（测量时的版本，未在当前锁定的 `0.2.130` 上复测）不再导出这张表（bindgen 前的主 wasm 里有），chunk 实例化报 `LinkError: table import requires a WebAssembly.Table`。
   探针里给主模块补了一条 28 B 的 export 才跑通；正式接入要么等 CLI 适配，要么在 bindgen 后补一道导出。
2. **`view!` 不接受内联 turbofish**：`view=Lazy::<XRoute>::new()` 会被当成标签解析失败。先 `let x = Lazy::<XRoute>::new();` 再 `view=x`。
3. **聚合要绕开 `#[lazy_route]`**：拆分函数不能有泛型 / receiver / `impl Trait` / `async`；`#[lazy_route]` 会把 `view` 改写成「自由函数 + impl 里的 async 包装」
   （`Self` 自动换成具体类型，所以 `fn view(this: Self) -> AnyView` 照写）。但它给每个 `#[lazy]` 的模块名都加了 call-site 哈希，等于一个函数一块；
   要按模块聚合得直接用 `#[wasm_split(同名模块, wasm_split_path = ::leptos::wasm_split)]`。
