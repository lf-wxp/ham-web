# 性能：测量口径与问题台账

> **管什么**：主线程响应（点按 → 画面变化）怎么测、用什么数据测、阈值是多少，以及按
> [`code-quality.md`](code-quality.md) §1.3 字段记录的性能问题台账（带状态，用来追踪整改进度）。
>
> **不管什么**（只给指针，不重复）：检查项与判定标准见 [`code-quality.md`](code-quality.md) §8 的 F3；
> 「首屏先出、其余每帧一批」「空闲预热」这些写法约束见 [`docs/ui/motion.md`](../ui/motion.md) §7；
> 「点击反馈先画出来、再做重活」见 [`docs/ui/interaction.md`](../ui/interaction.md) §2。

---

## 1. 测量口径

### 1.1 档位

| 维度 | 取值 | 为什么 |
| --- | --- | --- |
| 构建 | 发布构建（`cargo make build-web`）+ release 服务器 | 开发构建慢一个量级，热点比例可参考，绝对值不可用 |
| 设备 | Playwright `iPhone 13` 视口；Chromium **4 倍 CPU 降速**为主，WebKit 不降速交叉验证 | 4 倍降速接近中端手机；WebKit 是 iOS Safari 的内核 |
| 进入方式 | 站内导航（点链接，不整页刷新），**首次**（本会话第一次进该页）与**再次**各测一遍 | 首次含题库下载 / 解析、代码首次执行；再次才是「页面本身的渲染」 |
| 数据 | **空数据**；**重度用户**：错题本 1000 条（即上限 `MAX_RECORDS`）+ 通联日志 5000 条 | 只用空数据测不出「用得越久越慢」那一类问题 |

开发构建保留函数名，用来做 CPU 采样（`Profiler.start`，100µs 间隔）找热点；发布构建只用来量时间。

### 1.2 指标与阈值

| 指标 | 定义 | 阈值（4 倍降速） |
| --- | --- | --- |
| 首帧 | 点击 → 地址变化后的第一帧 | — （用来定位，不设门槛） |
| 长任务合计 | 点击后 1.5 s 内超过 50 ms 的任务时长之和（`PerformanceObserver` 的 `longtask`） | 再次进入 = 0；首次进入 ≤ 1 个 |
| 交互延迟 | 一次交互从输入到下一帧（Event Timing `duration`，≈ INP） | 高频交互（作答、翻题、输入一个字）≤ 50 ms |

### 1.3 工具

- **真机**：任意地址加 `?perf=1` 打开测量面板（`crates/app/src/perf_hud.rs`），显示每次点按的「首帧 / 换页 / 最长卡帧」。
- **回归**：`e2e/tests/responsiveness.spec.ts` 守结构性判据（重活是否推到了首帧之后、同一份数据是否只解析一次），不断言毫秒数。
- **全站扫描**：按 `crates/app/src/app/main_content.rs` 的路由表逐条站内进入，记录上面三项指标；脚本不入库（一次性测量），口径以本节为准。

---

## 2. 问题台账

ID 沿用 [`code-quality.md`](code-quality.md) 的检查项编号（F3.2 响应式粒度、F3.3 主线程长任务、F3.5 逐帧页面）。
状态：⬜ 待修 · 🔄 进行中 · ✅ 已修 · ❎ 不修（写理由）。

「修前 → 修后」是 §1.1 档位下的同机 A/B（2026-11-07）：修前是上一轮的发布产物，修后是本轮的发布产物，
同一个脚本、同一台机器交替测。页面一栏是「站内进入后 1.5 s 内的长任务合计，首次 / 再次」（ms，再次取两次的中位）；
交互一栏是 Event Timing 的交互时长。表里的数字和 §2.2 条目里登记问题时的数字口径相同，但不是同一次测量，略有出入。

### 2.1 总表

| ID | 场景 | 修前 → 修后 | 优先级 | 状态 |
| --- | --- | --- | --- | --- |
| F3.3-1 | 练习页作答后点「下一题」（重度用户） | 中位 112 → 48 ms / 次（最长 152 → 64） | P1 | ✅ |
| F3.3-2 | 进入错题本页（重度用户） | 471 / 366 → 249 / 199 | P1 | 🔄 再次进入仍超阈值，见条目 |
| F3.3-3 | 进入日志页（重度用户） | 550 / 352 → 372 / 142 | P1 | 🔄 再次进入仍超阈值，见条目 |
| F3.3-4 | 日志页搜索框逐字输入（重度用户，「BG3TA」） | 整段输入的长任务合计 263 → 51 ms（键间隔 160 ms）、266 → 0 ms（键间隔 60 ms）；每键中位 32 → 32 ms（60 ms 间隔） | P1 | ✅ 见条目里的保留项 |
| F3.3-5 | 进入统计页（重度用户） | 149 / 147 → 131 / 66 | P2 | 🔄 再次进入略超阈值 |
| F3.3-6 | 首页与冷启动（重度用户） | 首页 148 / 116 → 89 / 63；启动时同步解析整本日志未改 | P2 | 🔄 首页 ✅；启动解析 ❎（见条目） |
| F3.2-1 | 地图页 | `/callsign` 243 / 90 → 94 / 0；`/zone-map` 172 / 136 → 0 / 0；`/dxcc-map` 74 / 73 → 0 / 0；`/grayline` 55 / 60 → 0 / 0；`/grid-map` 64 / 60 → 0 / 0 | P1 | ✅ |
| F3.3-7 | `/nec` | 362 / 193 → 241 / 0 | P1 | ✅ 首次进入的余量见条目 |
| F3.3-8 | 分页签把非活动面板也挂上（`/morse` 等） | `/morse` 121 / 93 → 131 / 0 | P2 | ✅ 首次进入的余量见条目 |
| F3.3-9 | `/bands`：两套布局都建 | 88 / 97 → 0 / 0 | P2 | ✅ |
| F3.3-10 | `/browse`：首屏 40 道题一次建完 | 143 / 114 → 132 / 56 | P2 | 🔄 再次进入略超阈值 |
| F3.3-11 | `/formulas`：全部公式在一个任务里渲染 | 73 / 69 → 67 / 52 | P2 | 🔄 再次进入略超阈值 |
| F3.3-12 | 其余超阈值页面 | `/cheat-sheet` 68 / 58 → 55 / 0；`/muf` 78 / 80 → 64 / 0；`/progress` 62 / 0 → 0 / 0（重度用户 135 / 108 → 0 / 0）；`/radio-law` 64 / 64 → 0 / 0；`/rig-reviews` 50 / 0 → 0 / 0；`/waveform-lab` 90 / 74 → 0 / 0；`/prefixes` 53 / 0 → 0 / 0 | P2 | ✅ |
| F3.3-13 | 搜索框不防抖、不处理输入法组字 | 站内搜索、题库浏览、术语表、日志均改为防抖；输入法组字期间不派发 | P2 | ✅ |
| F3.3-14 | 像素风首页：题库选择器解析 A/B/C 三个题库 | 首次进首页不再下载 B / C 类（发布构建） | P2 | ✅ |
| F3.3-15 | 搜索框每敲一个字都整页重建列表（复测时发现） | 见 F3.3-4 | P1 | ✅ |
| F3.5-1 | 后台常驻：倒计时每秒解析存储；首页动画 / 频谱滚出视口仍在跑 | 倒计时按原串缓存；频谱滚出视口即停；首屏循环动画滚出视口即暂停 | P3 | ✅ |

**首次进入的余量**：`/nec` `/morse` `/browse` `/callsign` 首次进入仍有 90–240 ms 的长任务，再次进入为 0 或接近 0。
差值主要是这一页的代码第一次执行（wasm 函数首次调用的编译与初始化）与题库 / 数据的首次解析，不在本轮范围内。
按路由拆分 wasm 评估过、没有做：trunk 0.22 不支持，而且会把下载与编译挪到每一页的第一次点击上。

### 2.2 条目

「修法」写的是实际落地的做法；「余量」是修完仍超阈值的部分与下一步。

#### F3.3-1 作答链路

```
位置：      crates/app/src/study.rs（record_items / load_book / update_book）；crates/app/src/achievements.rs（build_stats）；
            crates/core/src/mistake_book.rs（MistakeBook::rekey）；crates/app/src/util.rs（storage::get_json_shared）
优先级：    P1
现象：      重度用户每答一题、点「下一题」要 112 ms（空数据 32 ms），刷题越久越卡
证据：      E3：发布构建 4 倍降速 Event Timing；E2：开发构建 CPU 采样，record_items 中 detect_new 占 41%、load_book 占 37%
根因：      ① load_book 每次都解析整本错题本 JSON 并执行 rekey（逐条归一化题面、重算指纹，合并去重是 O(n²)）；
            ② 成就检测每次都从 localStorage 解析整本通联日志，对每条记录查 DXCC 前缀；
            ③ 答对且不在错题本里的题，错题本没变也整本序列化写回
修法：      ① 错题本按原串缓存 rekey 之后的结果（`study::load_book` 返回共享引用）；学习统计 / 单题统计 / 每日记录走
            `storage::get_json_shared`（按原串比对，备份导入、跨标签页写入不会读到旧值）；改写走「取出来改、改完写回」；
            ② rekey 去重改哈希索引（单元测试锁住顺序与合并口径）；
            ③ 成就判定读日志 store 上的摘要 Memo（`achievements::track_logbook`，空闲时预先算好）；
            ④ 错题本没变（只有「答对且不在本里」）就不写回
回归锚点：  responsiveness.spec.ts「作答：答对不在错题本里的题不整本写回错题本，也不再去读整本日志」（上一轮产物上失败）
状态：      ✅ 112 → 48 ms
```

#### F3.3-2 错题本页

```
位置：      crates/app/src/pages/mistakes/{mistakes_page,mistake_diagnosis,mistake_card}.rs
优先级：    P1
现象：      重度用户进入错题本页一个 400+ ms 的长任务
证据：      E3 + E2：load_book 占 107 ms（开发构建），MistakeDiagnosis 40 ms
根因：      一次进入调 6 次 load_book（每次解析 + rekey）、其中 2 次再整份克隆排序；每张卡各读一次学习统计（40 次）；
            分类分布按「每个分类 × 每条错题」各数一遍，页面与诊断面板各做一次
修法：      load_book 走缓存（F3.3-1）；分类分布一遍计数（`category_counts`），页面与诊断面板共用；
            卡片读学习统计的共享引用，不再每张卡克隆一份；`reveal_children` 现代风下不读视口高度（强制布局）
余量：      再次进入 199 ms。剩下的是 1000 条里排序与首屏 40 张卡片（题干、选项、错因标注）的构建：
            下一步把首屏卡片改成与题库浏览相同的「先建一屏、下一帧补齐」
状态：      🔄 471 / 366 → 249 / 199
```

#### F3.3-3 日志页

```
位置：      crates/app/src/pages/log/（log_stats_panel / log_stats_detail / pending_qsl_list / grid_map/state /
            awards_panel / log_page）；crates/core/src/dxcc.rs
优先级：    P1
现象：      重度用户进入日志页长任务 627 / 366 ms
证据：      E3；E2：同一份 5000 条记录在挂载时被整份克隆 10 次以上（`.get()` 读 Vec 信号）；
            DXCC 前缀查找是日志页、统计页、日志搜索的共同热点
根因：      ① 读日志信号用 `.get()`（克隆整份）而不是 `.with()`；② 待回卡列表不分页、DXCC / 网格芯片全量渲染；
            ③ 网格地图、奖项、体检面板都在首屏以下，却和首屏一起同步挂载；④ 每次 DXCC 查找都转大写、拆斜杠、逐级查表
修法：      统计概览挂成 Memo、借用读取；网格地图的筛选条件在循环外读一次、派生统计全部借用；
            DXCC 实体、奖项、待回卡、网格地图拆进 `LogStatsDetail`，体检面板也一样，都包在 `Deferred` 里首帧之后挂载；
            待回卡先列 20 条，其余收在「显示全部 N 条」；`dxcc::lookup_with_prefix` 记忆化，按编号 / 中文名查实体改哈希索引
余量：      再次进入 142 ms：首屏的概览统计（5000 条的计数与去重）与记录列表一页 50 行。下一步把概览计数挂到 store 级 Memo，
            跨访问复用
回归锚点：  responsiveness.spec.ts「日志页：首屏以下的统计明细在首帧之后挂载，待回卡先列 20 条」（上一轮产物上失败）
状态：      🔄 550 / 352 → 372 / 142
```

#### F3.3-4 日志搜索（含 F3.3-15）

```
位置：      crates/app/src/pages/log/entry_list.rs；crates/core/src/logbook.rs（matches_query）；crates/app/src/debounced.rs
优先级：    P1
现象：      重度用户在日志搜索框每输入一个字卡 128 ms
证据：      E3
根因：      输入不防抖，每键克隆全部命中条目再排序；`matches_query` 每条记录给 9 个字段各分配一次大写串
修法：      筛选只产出下标、不克隆记录；整表排序次序单独挂 Memo（日志变了才重排），关键词只做筛选；
            `matches_query` 的 ASCII 路径不分配（单元测试与原实现逐字对比）；输入防抖 150 ms
F3.3-15：   复测时发现：「筛选条件变了回第一页」的 Effect 订阅的是未防抖的关键词，且 `page.set(0)` 不比较新旧值，
            每敲一个字都让当前这一页 50 行整体重建。改为跟防抖后的关键词、值不变不通知（`debounced::set_if_changed`）；
            术语表、题库浏览、错题本、收藏的 `visible.set(PAGE)` 同一问题，一并改掉
保留项：    键间隔恰好略大于防抖时长（约 160 ms）时，下一键会撞上防抖后那一次重算，单键交互时长中位 72 ms；
            整段输入的长任务合计仍从 263 降到 51 ms。键间隔 60 ms 时单键 32 ms、长任务 0
回归锚点：  log 搜索相关 e2e；responsiveness.spec.ts「输入法组字期间不重算搜索」
状态：      ✅
```

#### F3.3-5 统计页

```
位置：      crates/app/src/pages/stats/stats_page.rs
优先级：    P2
现象：      重度用户进入统计页 193 / 146 ms
证据：      E3；E2：每条记录查两次 DXCC 实体（5000 条 → 1 万次前缀查找），日志克隆两次
根因：      实体查找没复用；信号读取克隆整份
修法：      借用读取；每条记录只查一次实体、按 DXCC 编号计数，最后才给出现过的实体取本地化名；查找本身记忆化（F3.3-3）
余量：      再次进入 66 ms，是热力图、每日计数与奖项进度三份 5000 条的统计：下一步挂成跨访问的 Memo
状态：      🔄 149 / 147 → 131 / 66
```

#### F3.3-6 首页与冷启动

```
位置：      crates/app/src/kit_modern/home/{onair,bento}.rs；crates/app/src/pages/home/wanted_card.rs；
            crates/app/src/pages/log/mod.rs（provide_log_store）
优先级：    P2
现象：      重度用户首页 183 / 107 ms；冷启动到首页内容出现 384 ms（空数据 242 ms）
证据：      E3；E2：首页读两次错题本；像素风「通缉台」整份克隆日志；启动时同步解析整本日志快照
修法：      错题本走缓存（F3.3-1）；「通缉台」改读成就判定那份日志摘要 Memo
不修：      启动时同步解析日志快照。改成「先给空日志、首帧之后再填」会开一个数据丢失窗口：这期间任何一次编辑
            都会把只含一两条记录的日志写进权威层（IndexedDB），覆盖掉整本 —— `provide_log_store` 的快照 / 权威层仲裁
            就是按「内存里一开始就是完整日志」写的。重度用户冷启动多出的约 140 ms 换不来这个风险
状态：      🔄 首页 ✅（148 / 116 → 89 / 63）；启动解析 ❎
```

#### F3.2-1 地图页

```
位置：      crates/app/src/data.rs（ready_signal / load_binary）；crates/app/src/pages/map/（map_view / zone_map /
            zone_overlay / dxcc_overlay）
优先级：    P1
现象：      含世界地图的页面再次进入仍有 150–280 ms 长任务
证据：      E3；E2：脚本只占 10–25 ms，大头是建 DOM：分区 / DXCC 地图约 3100 个 SVG 节点，因为世界在 x 方向平铺三份；
            几何数据每次 `.get()` 深拷贝
根因：      三份世界同步建完；ready_signal 的 derive 每读一次克隆一次；几何解码结果不跨访问缓存
修法：      几何解码结果在会话内缓存并以 `Arc` 分发（命中缓存也等首帧画完再交付）；叠加层的派生结果改 Memo；
            中间那份世界随页面建，左右两份首帧之后每帧补一份（初始视口只看得见中间那份）
回归锚点：  responsiveness.spec.ts「世界地图：首帧只建中间那份，左右两份随后补上」（上一轮产物上失败）
状态：      ✅ 见总表
```

#### F3.3-7 `/nec`

```
位置：      crates/app/src/pages/nec/nec_page.rs
优先级：    P1
现象：      进入 `/nec` 284 / 184 ms 长任务
证据：      E3；E2：settled_input 起手就是默认输入，result Memo 被首屏组件立即读取，同步跑完整求解与方向图扫描
修法：      求解结果按（输入，传输线）在会话内缓存；读结果的区块（结果、三维方向图、扫频、NVIS、方向表）包在
            `Deferred` 里，首次求解发生在首帧之后
状态：      ✅ 362 / 193 → 241 / 0（首次进入的余量见总表下的说明）
```

#### F3.3-8 分页签

```
位置：      crates/app/src/ui/tabs/tabs_panel.rs
优先级：    P2
现象：      `/morse` 首次 154 ms：四个训练器全部挂载，只是隐藏
修法：      面板内容第一次被激活才挂载，之后常驻（切回来不重建、保留状态）；面板外壳始终在，`aria-controls` 不悬空
回归锚点：  responsiveness.spec.ts「分页签：只挂当前面板，切到别的面板才挂、切回来状态还在」（上一轮产物上失败）
状态：      ✅ 再次进入 93 → 0
```

#### F3.3-9 `/bands`

```
位置：      crates/app/src/pages/bands/bands_page.rs；crates/app/src/media.rs
优先级：    P2
现象：      进入 `/bands` 约 100 ms
修法：      按 `md` 断点只建一套（`media::use_media_query`，跨断点时切换）；脚注与小测在首屏以下，包进 `Deferred`
            （它会补上锚点定位）
回归锚点：  responsiveness.spec.ts「/bands：桌面表格与手机卡片只建当前宽度那一套」（上一轮产物上失败）
状态：      ✅ 88 / 97 → 0 / 0
```

#### F3.3-10 `/browse`

```
位置：      crates/app/src/pages/browse/browse_page.rs
优先级：    P2
现象：      再次进入 133 ms
修法：      进入的第一帧只建 8 道题，下一帧补到 40；关键词检索文本挂成惰性 Memo（第一次输入时才建）；搜索防抖
余量：      再次进入 56 ms：补齐 32 道题的那一帧。下一步改成每帧补一批
状态：      🔄 143 / 114 → 132 / 56
```

#### F3.3-11 `/formulas`

```
位置：      crates/app/index.html（KaTeX）；crates/app/src/pages/formulas.rs
优先级：    P2
现象：      首次 234 ms、再次 87 ms
修法：      KaTeX 按 TeX 原文缓存渲染结果，每帧最多花 8 ms、没排完的下一帧接着排；公式分组首帧建两组，其余每帧一组
余量：      再次进入 52 ms，在阈值边缘
状态：      🔄 73 / 69 → 67 / 52
```

#### F3.3-12 其余超阈值页面

```
位置：      `/muf`：pages/muf/heatmap_card.rs；`/progress` `/achievements`：pages/{progress,achievements}.rs、data.rs
            （bank_question_keys）、core mistake_book（covered_keys）；其余页面靠上面几条的公共修复（分页签、DXCC 查找、
            日志摘要）一起落下来
修法：      MUF 热力图 240 格的类名建一次，选中态只切两个类（此前每格是读选中信号、每次都 `cn` 合并的闭包）；
            题库覆盖率的题目 key 集合按题库数据缓存（此前每次进页面对三个题库做约 3500 次内容指纹）；
            进度页 / 成就页读日志摘要 Memo，不再克隆整本日志
状态：      ✅ 见总表
```

#### F3.3-13 搜索框

```
位置：      crates/app/src/ui/input.rs；crates/app/src/debounced.rs；components/search_dialog.rs；
            pages/{browse/browse_page,glossary/glossary_view,log/entry_list,practice/view}.rs
优先级：    P2
修法：      搜索框（`ui::Input` 的 `kind=Search`、现代风 `ModernInput` 的 `kind="search"`）在输入法组字期间（`isComposing`）
            不派发，`compositionend` 时补派一次；判定在 core 的 `ime_gate`（单元测试钉住 Chrome / Safari 两种事件顺序），
            DOM 侧是 `crates/app/src/composition.rs`；其余类型的输入框逐字回调不变（范围见 F3.3-16 / F3.3-17）；
            站内搜索、题库浏览、术语表、日志的筛选按防抖副本算（地址带来的关键词、点相关词条跳转两份一起写，立即生效）；
            站内搜索的题目检索文本按索引缓存；练习页跳题搜索 `Signal::derive` 改 `Memo`（此前每键扫两遍题库）
回归锚点：  responsiveness.spec.ts「输入法组字期间不重算搜索，选字结束才算一次」（两套皮肤各一遍；上一轮产物上失败）
状态：      ✅（首版只做了 `ui::Input`、却整档标 ✅，漏了现代风，2026-11-07 复核补齐）
```

#### F3.3-14 像素风首页题库选择器

```
位置：      crates/app/src/data.rs（check_version）
优先级：    P2
修法：      配置里带修订号的题库直接按「可用」算（修订号由构建后处理按文件内容写入，文件缺失时构建失败）；
            开发构建没有修订号，照旧逐个加载判断
回归锚点：  responsiveness.spec.ts「像素风首页：题库选择器不为点亮按钮下载 B / C 类题库」（上一轮产物上失败；开发构建跳过）
状态：      ✅
```

#### F3.5-1 后台常驻

```
位置：      crates/app/src/pages/countdown.rs；crates/app/src/kit_modern/spectrum.rs；crates/app/src/motion.rs
            （pause_offscreen_in）；crates/app/style/modern/kit.css；kit_modern/home/{hero,bento}.rs
优先级：    P3
修法：      倒计时的每秒检查按原串缓存解析结果；频谱滚出视口就停掉帧循环（此前只是不画、每帧仍自调度），回到视口再挂上；
            首屏与功能卡容器带 `data-pause-offscreen`，滚出视口时打 `data-offscreen`，CSS 暂停里面点名的几条循环动画
踩坑：      第一版写成 `[data-offscreen] *`，把一次性的入场动画与过渡也暂停了 —— 它们停在起始帧、`finished` 永远不兑现，
            e2e 的 `settleFrames` 等在上面超时。现在只点名无限循环的那几条，新加循环动画时要补进这条规则
回归锚点：  responsiveness.spec.ts「首页：首屏滚出视口后暂停循环动画，滚回来继续」（上一轮产物上失败）
状态：      ✅
```

---

## 3. 复核台账：对本轮性能改动的代码审查（2026-11-07）

> **来源**：对 §2 那批未提交改动（67 个文件）做的一次代码审查，加上落实整改时补跑 e2e 暴露出来的问题。
> 状态符号与 §2 相同；ID 接着 §2 往下排。证据级别见 [`code-quality.md`](code-quality.md) §1.1。
> 条目只写**问题与落点**；改法与取舍在表里一句话交代，重要的几条在 §3.2 按模板展开。

### 3.1 总表

| ID | 问题 | 优先级 | 状态 | 修法（落点） |
| --- | --- | --- | --- | --- |
| F3.3-16 | 组字门控只做了 `ui::Input`，现代风搜索框 `ModernInput` 没有；F3.3-13 却整档标 ✅ | P2 | ✅ | 判定抽到 core 的 `ime_gate`，DOM 侧 `composition.rs`，两个控件共用 |
| F3.3-17 | 门控对所有 `ui::Input` 生效，表单字段（字数计数、`maxlength`、逐字校验）组字期间会滞后；Safari 先 `compositionend` 后 `input`，选字确认会派发两次 | P2 | ✅ | 只对搜索框启用；尾随的同值 `input` 在时间窗内吞掉（`ime_gate`） |
| F3.3-18 | `dxcc` 记忆表在服务端（`/api/v1/dxcc/lookup`、呼号回退）也启用，每个工作线程最多 5 万条 | P2 | ✅ | 容量只在 `wasm32` 非零；超长输入不进表；内部函数带容量参数，单元测试两条路径都走 |
| F3.3-19 | `Deferred` 挂载前不占位，它下面的内容在挂上时整体下跳 | P3 | ✅ | `Deferred` 加 `min_height`（可随状态变空）；日志页两处按实测下限填值，见 §3.2 |
| F3.3-20 | `Deferred` 补滚动直接拿 `location.hash` 当元素 `id`，编码过的锚点（中文）找不到 | P3 | ✅ | `util::decode_uri_component`（与 `encode_uri_component` 对称，带单元测试），格式坏了原样保留 |
| F3.3-21 | `upper_contains`：字段是 ASCII、查询词含非 ASCII（中文查询）时仍给每个字段分配一次大写串 | P3 | ✅ | 这种组合直接判否（ASCII 转大写仍是 ASCII）；原有的逐字对比测试覆盖 |
| F3.3-22 | `TabsPanel` 靠「闭包只会跑一次」保证内容不丢，多触发一次就把已挂上的面板换成空白 | P3 | ✅ | `children` 改 `ChildrenFn`：最坏重建一份，不会静默变空 |
| F3.3-23 | `update_book` 约定「改了必须返回 `true`」，写错就静默丢改动，没有护栏 | P3 | ✅ | 文档写明契约；调试构建对照改前快照断言（`[update_book] …`） |
| F3.3-24 | 地图三份平铺的建造次序按下标写死（`1 => 0, 0 => 1`），改了 `WORLD_OFFSETS` 就会悄悄先建看不见的那份 | P3 | ✅ | `projection::tile_build_rank` 按偏移值算（离视口那份越近越先），带单元测试 |
| F3.3-25 | 待回卡「显示全部 N 条」手写 `<button>` + `link_class`，不走 `ui::Button` | P3 | ✅ | `Button variant=Text`，文字色与下划线显式给 |
| F3.3-26 | 新增的会话级缓存占多少内存没有登记 | P3 | ✅ | 见 §4 |
| F3.3-27 | 灰线地图 e2e 稳定红：三份世界逐帧补齐后，用例在第一份就快照，之后永远对不上（审查时没跑 e2e，没发现） | P2 | ✅ | `maps.spec.ts` 快照前先等三份都在；见 `testing.md`「已知的失败」⑤ |
| F3.3-28 | 现代风站标换图标（`md-brand-dot` → `md-brand-mark`）混在性能改动里 | P3 | ⬜ | 提交时拆成独立提交；这是工作区操作，不是代码缺陷，未代拆 |
| F3.3-29 | §2 里「修前 → 修后」的数字依赖不入库的测量脚本，别人复现不了 | P3 | ❎ | §1.3 已把「脚本不入库」定为刻意边界，数字只作量级参考；回归以 `responsiveness.spec.ts` 的结构性判据为准。要入库得先定死「机器 / 降速 / 数据」口径，不在本轮 |

### 3.2 条目

#### F3.3-16 / F3.3-17 搜索框的组字门控

```
位置：      crates/app/src/ui/input.rs；crates/app/src/kit_modern/input.rs；crates/app/src/composition.rs；
            crates/core/src/ime_gate.rs
优先级：    P2
现象：      现代风下用拼音搜索「天线」，t / ti / tia / tian 每个中间串都触发一次整份搜索重算（像素风没有）；
            Safari 下选字确认后同一个值派发两次
证据：      E2 读码：`browse_page.rs` 现代风分支用的是 `ModernInput(on_input=…)`，没有 `isComposing` 判断；
            E3：把 `ModernInput` 的门控临时关掉，「输入法组字期间不重算搜索（modern）」红，还原后绿
根因：      门控写在 `ui::Input` 的事件处理里，现代风的搜索框是另一个控件，漏了；
            门控同时挡了所有类型的 `ui::Input`，而表单字段依赖逐字回调
修法：      判定（组字中吞掉、结束时补派、吞掉 Safari 尾随的同值 `input`）放 core 的 `CompositionGate`，不依赖浏览器、
            有单元测试；`composition.rs` 只负责从事件里读 `isComposing` 与时间戳。`ui::Input` 只在 `kind=Search` 时启用，
            `ModernInput` 只在 `kind="search"` 时启用。为什么不全局生效：字数计数、`maxlength`、逐字校验在组字期间
            不回调会滞后，而这些字段没有「每个字符整份重算」的代价，没必要挡
回归锚点：  responsiveness.spec.ts「输入法组字…（pixel / modern）」「Safari 的事件顺序…（pixel / modern）」
            「非搜索的输入框不受组字门控」；`ime_gate` 的 7 条单元测试
状态：      ✅
```

#### F3.3-18 `dxcc` 记忆表

```
位置：      crates/core/src/dxcc.rs（MEMO_CAP / lookup_memoized）；调用方 crates/server/src/api_v1.rs、callsign.rs
优先级：    P2
现象：      服务端每个工作线程都会攒一张最多 5 万条的线程局部表，而服务端是「查一个、查完即走」
证据：      E1：`grep -rn "dxcc::" crates/server/src`
根因：      记忆表是为浏览器里「同一批呼号反复查几遍」写的，放在了 core 里无条件启用
修法：      `MEMO_CAP` 只在 `wasm32` 取 5 万、其余目标为 0（不记忆）；超过 32 字节的输入不进表。
            查找拆成 `lookup_memoized(callsign, cap)`，单元测试用容量 2 的小表让「满了清空」真的发生，并核对三条路径
            （不记忆 / 小表 / 超长输入）结果与直接查表一致
回归锚点：  `memo_capacity_and_bypass_do_not_change_results`、`native_builds_do_not_memoize`
状态：      ✅
```

#### F3.3-19 `Deferred` 占位

```
位置：      crates/app/src/components/common/deferred.rs；pages/log/{log_page,log_stats_panel}.rs
优先级：    P3
现象：      日志页的统计明细、体检面板在首帧之后才挂上，它们下面的添加表单与记录列表在那一帧整体下跳
证据：      E3：实测（桌面 1280 与手机 390 宽，像素 / 现代，不降速，60 条日志）：统计明细 164–173 rem、体检 6.1 rem
            （空日志也是 6.1 rem，3000 条是 15.6 rem）；明细里奖状进度 49.5–54.8 rem、实体清单 5.4–6.3 rem，
            待回卡约 52 rem、网格地图约 46–57 rem，后两块随数据有无而变
修法：      `Deferred` 加 `min_height`（`Signal<String>`，空串 = 不占位）。取各皮肤、各宽度实测高度的**下限**：
            体检 `6rem`；统计明细 `52rem`，且日志为空时传空串（那时明细整块不出现，占位只会留白）。
            `/bands`、`/nec` 的 `Deferred` 在页面最底部、下面没有内容可推，不加
余量：      占位只到下限（52 rem 对 164–173 rem），有数据时下面的内容仍会跳一截，只是少了近三分之一。
            要彻底消除得让待回卡与网格地图也有固定高度，那是版式改动，不在本轮
回归锚点：  无（版面跳动不是结构性判据，e2e 不断言像素高度）
状态：      ✅
```

#### 3.3 验证记录

整改后补跑的检查（审查时只跑了门禁，没跑 e2e、两皮肤布局审计与体积预算）：

| 检查 | 口径 | 结果 |
| --- | --- | --- |
| `cargo make check` | 提交前门禁（fmt / clippy / test / i18n / 结构守卫 / 界面规范 / README 对账 / 像素风生成物 / 数据来源与时效） | 通过 |
| `cargo make size-check` | release 产物，br 压缩后 | 29 项均在预算内 |
| 全量 e2e | release 口径（`cargo make build-web` 产物，3 worker） | 1191 通过 · 3 跳过 · 0 失败 |
| `mobile_overflow` + `i18n_layout` | release；像素风与现代风（`E2E_UI_STYLE=modern`）各一档 | 两档各 6 项通过 |
| `responsiveness.spec.ts` + 相关 spec | dev（`trunk serve`） | `modern_ui` 两条、`modern_a11y` 一条在 dev 下红，原因与处理见 `testing.md`「已知的失败」⑤，release 口径通过；灰线地图一条是真回归（F3.3-27），已修 |
| 变异验证 | 临时关掉 `ModernInput` 的门控 | 「输入法组字期间不重算搜索（modern）」红，还原后绿 |

**没有覆盖的**：`ime_gate` 的事件顺序只在 Chromium 里用合成事件验过，没有在真实的 Safari / Firefox 与真实输入法上跑
（`E2E_CROSS=1` 只跑 `cross_browser.spec.ts`，不含这条）。合成的 `isComposing` 事件与真实输入法的时序可能有出入，
上线后在 iOS Safari 与桌面 Firefox 上用拼音输入各手测一次「天线」。

---

## 4. 常驻缓存清单

本轮新增的会话级缓存都在内存里，页面卸载后也不释放（`thread_local!`，与应用同寿）。体积按随包文件大小**估算**（未实测堆占用），
新增同类缓存时在这里补一行，并写明上限与失效条件：

| 缓存 | 位置 | 内容 | 上限 / 估算体积 | 失效 |
| --- | --- | --- | --- | --- |
| 站内搜索的题目检索文本 | `components/search_dialog.rs`（`QUESTION_TEXTS`） | 约 3500 条「题干 + 解析」小写拼接串 | 一份；≈ `search-index.json` 的 1.7 MB | 索引换新（`Arc::ptr_eq`）就重建 |
| 题库浏览的检索文本 | `pages/browse/browse_page.rs`（`search_texts` Memo） | 当前加载题库每题「题干 + 选项 + 解析」小写串 | 随页面卸载释放；三个题库全载时 ≈ 3.5 MB | 题库数据变就重算；第一次输入关键词才建 |
| 题库覆盖率的题目 key | `data.rs`（`BANK_KEYS`） | 每个题库一份 key 集合（约 3500 个短串） | 最多 6 份；< 0.5 MB | 题库重载是另一个 `Arc`，旧的按先进先出挤掉 |
| 地图几何解码结果 | `data.rs`（`BINARY`） | `dxcc-entities.bin`、`cn-provinces.bin` 解码后的 `Arc` | 两份；随包二进制 12 KB / 7 KB，解码后的多边形点集更大，**未测** | 不失效（随包文件，不会变） |
| 错题本 / 学习统计 / 单题统计 / 每日记录 | `study.rs`（`BOOK`）、`util::storage`（`PARSED`） | 解析结果 + 原串 | 每个 key 一份；错题本上限 1000 条，原串约 600 KB，解析结果同量级 | 原串变了就未命中（备份导入 / 跨标签页写入也一样） |
| DXCC 查找记忆 | `core/dxcc.rs`（`MEMO`） | 呼号 → 实体下标 | 仅 `wasm32`；≤ 5 万条，键 ≤ 32 字节 | 满了整表清空 |
| 公式渲染结果 | `index.html`（KaTeX `cache`） | TeX 原文 → HTML | 公式是静态的，上百条 | 不失效 |
| NEC 最近一次求解 | `pages/nec/nec_page.rs`（`LAST_SOLVE`） | 输入、传输线、结果 | 一份；结果含方向图采样，体积**未测** | 输入变了就重算 |

## 5. 主包体积：现状、预算与分包方案（2026-11-07）

### 5.1 实测

| 项 | 值 | 口径 |
|---|---|---|
| 主包 wasm（raw） | **7.85 MB** | `target/rel-a/*_bg.wasm`（`trunk build --release`） |
| 主包 wasm（br） | **2.03 MB**（2,128,947 B） | postbuild 预压缩，线上实际传输 |
| 主包 JS 胶水（br） | 16 KB | |
| 样式表（br） | 52 KB | 预算 55 KB，余量健康 |
| 首访字体（现代风 / 中文） | 6 个文件 / 234 KB | `performance.getEntriesByType("resource")` 实测 |
| 首访字体（现代风 / 英西） | 5 个文件 / 133 KB | 中文展示字按语言预载后省掉 101 KB |

体积构成（`twiggy top`，release 已 strip，只能看到段级）：

- **数据段占大头**：单个 `data[...]` 最大 346 KB，其后还有 122 / 61 / 61 / 54 / 53 / 49 KB 若干个。
  这些是**编译期嵌进 wasm 的静态表**：中文词典（4827 条，默认语言全量内嵌）、DXCC 实体表（44 KB）、
  业余无线电法规原文（52 KB）、知识库索引等。它们不是「可以拖到后面的代码」，而是首屏同步查表要用的数据。
- **代码段**：单函数最大 ~36 KB，`cargo bloat` 的 crate 级归因在 strip 后不可用；
  要精确归因需用带符号的 `--profiling` 构建。

### 5.2 已经做掉的（本轮）

- **主包预算从 2,500,000 B 收紧到 2,260,000 B**（实测 2,128,947 B ≈ 2.03 MiB，余量 ~6%；本节的 MB 指 MiB，预算文件里是字节）。原来 23% 的余量等于允许悄悄长 400 KB 而无人发现。
- 字体预算进 `perf-budget.json`（新增 `raw: true` 口径，字体是已压缩的 woff2，postbuild 不生成 `.br`）。
- 中文展示字（100 KB）改为**按语言预载**：英 / 西语界面首访字体从 241 KB 降到 **133 KB**。

### 5.3 为什么没做「路由分包」，以及要做的三条路

Leptos CSR 只有一个 wasm 实例，路由是 `leptos_router` 的客户端路由 —— **代码在同一个模块里**，
不存在「进到某页才下载那一页的 wasm」这种现成能力。真正要拆，只有三条路：

| 方案 | 做法 | 代价 | 什么时候值得做 |
|---|---|---|---|
| **A. 多 wasm 模块** | 把重功能区（解码器、地图、考试引擎）编成独立 crate → 独立 `cdylib`，由 JS 在进入对应路由时实例化，状态经 `wasm-bindgen` 的共享内存 / 序列化传递 | 跨模块传递 Leptos 信号与 `web_sys` 对象几乎不可行；等价于把这几页拆成微前端。**月级**工作量，收益是首屏少 300–600 KB | 主包涨到 2.6 MB 以上、且首屏 LCP 被带宽卡住时 |
| **B. 把重活挪进 Worker** | 解码已经这么做（`*-worker`）；可继续把「题库检索 / 统计聚合 / 地图投影」这类纯计算挪进 Worker | 中等；只减少**主线程长任务**，**不减少字节**（Worker wasm 另算） | 长任务超标时（见 §2 台账 F3.3） |
| **C. SSR / 岛屿** | 换成 Leptos SSR + 按路由做 islands | 需要服务端渲染管线 + 首屏 HTML 缓存，**重构量最大**，但首屏字节数是**数量级**下降 | 有稳定后端与其运维预算时 |

**结论**：以当前 2.03 MB、且数据段占大头（拆代码也带不走）来看，A 的收益/成本比最差 —— 除非数据能改成
运行时拉取（那要先解决「同步查表」这个前提，属于数据层改造，不是打包问题）。所以本轮把它记为
**需要独立立项的方案**，不做半成品。

### 5.4 若以后要动手，先做这三步测量

1. `trunk build --release --dist target/rel-x` + `cargo bloat --release --crate-type cdylib -n 30`（**带符号**构建）拿到 crate 级归因；
2. 用 `twiggy top -n 50` 对照数据段：确认哪些 `data[...]` 是 i18n / 题库 / 地理数据；
3. 对候选（比如「只在 `/nec`、`/antenna-modeling` 用到的电磁计算」）做「删掉它体积掉多少」的实验，
   再决定是否值得为它引入模块边界。
