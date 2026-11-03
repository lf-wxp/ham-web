# 路由表

> 唯一事实来源是 `crates/core/src/registry.rs` 的能力注册表：路径、标题、图标与分组都在那里，
> 顶部导航、`sitemap.xml` 与本文都由它派生。新增页面时改注册表 + `main_content.rs` 的 `<Route>`，
> 本文随注册表更新（数字与路径由 `cargo make readme-check` 对账）。
>
> 路由表定义在 `crates/app/src/app/main_content.rs`（外层布局见同目录 `mod.rs`）。

| 模块 | 路径 |
| --- | --- |
| 首页 | `/` |
| 考试中心 · 闯关 | `/map` `/bestiary` |
| 考试中心 · 练题 | `/practice` `/daily-challenge` `/browse` `/flashcards` `/listen` |
| 考试中心 · 模拟考 | `/exam` `/exam-review` |
| 考试中心 · 复习 | `/cards` `/mistakes` `/mistake-topics` `/bookmarks` |
| 考试中心 · 学习数据 | `/weekly` `/progress` `/study-calendar` `/report` |
| 备考速查 | `/reference` `/cheat-sheet` `/confusables` `/formulas` `/prefixes` `/glossary` `/q-code` `/phonetic` `/rst` `/morse` `/cw-operating` `/exam-comparison` `/license-classes` `/radio-law` |
| 模式 · 数字 | `/analog-modes` `/atv` `/modern-digital-modes` `/modes` `/modulation` `/digital-comms` `/lora` `/dsp` `/dv-network` `/rtty` `/ft8` `/sdr` `/gnuradio` `/aprs` `/sstv` `/weather-sat` `/packet` |
| 传播 · 频段 | `/frequencies` `/coordination` `/zone-map` `/propagation` `/beacons` `/special-prop` `/meteor-scatter` `/aurora` `/eme` `/muf` `/wspr` `/bands` `/bandplan` |
| 天线系统 | `/antennas` `/antenna-array` `/polarization` `/feedline` `/balun` `/antenna-diy` `/practical-antennas` `/antenna-installation` `/antenna-farm` `/antenna-tuning` `/antenna-theory` `/antenna-modeling` `/nvis` |
| 设备 · 测量 | `/connectors` `/antenna-analyzer` `/vna` `/electronics` `/filters` `/measurement-lab` `/meters` `/power` `/power-supply` `/transceiver` `/receiver` `/rig-reviews` `/gear` `/amplifier` `/microwave` `/mobile` |
| 通联 · 活动 | `/radiogram` `/operating` `/sat-operation` `/china-ham` `/contest` `/cabrillo` `/awards` `/iota` `/dx` `/dxpedition` `/most-wanted` `/qrp` `/eqsl` `/qsl-card` `/ardf` `/emcomm` `/winlink` `/events` `/portable` `/grid` `/callsign` `/repeater` `/repeater-build` `/logging-software` |
| 进阶 · 关于 | `/organizations` `/safety` `/grounding` `/rfi` `/noise` `/beginner` `/learning-path` `/learning-resources` `/swl` `/license` `/spectrum-admin` `/regulations` `/history` `/remote` `/open-source` `/community` `/diy-projects` `/developers` |
| 工具 · 计算 · 解码 | `/kiss-tnc` `/vna-reader` `/navtex-decode` `/lrpt-decode` `/dominoex-decode` `/olivia-decode` `/rtty-decode` `/hell-decode` `/wefax-decode` `/ft8-decode` `/tools` `/nec` `/waveform-lab` `/smith` `/compliance` `/apt-decoder` `/sstv-decode` `/wspr-decode` `/signal-id` `/sdr-waterfall` `/psk-decode` |
| 工具 · 日志 · 竞赛 | `/log` `/qsl-labels` `/qsl-designer` `/contest-log` `/contest-calendar` `/stats` |
| 工具 · 实时数据 | `/dashboard` `/dx-spots` `/psk-reporter` `/rbn` `/solar` |
| 工具 · 地图 · 卫星 | `/grid-map` `/grayline` `/dxcc-map` `/portable-map` `/satellites` `/sdr-map` |
| 工具 · 训练 · 辅助 | `/photo-processor` `/callsign-copy` `/countdown` `/notifications` `/print` |
| 可直链访问（未在导航中） | `/battle` `/license.html` `/achievements` `/backup` |

带参数的页面：

| 路径 | 参数 |
| --- | --- |
| `/practice` `/exam` | `version`（题库版本）、`bank`（A\|B\|C）；`/practice` 另有 `multi=1`（只练多选，页面顶部有同名开关），`/exam` 另有 `mode=weak`（薄弱项组卷） |
| `/map` | `version`、`bank`（A\|B\|C） |
| `/battle` | `stage`（一级分类 key，如 `法规`；闯关）或 `mode=revenge`（复仇，另可带 `key`（单只怪物的错题 key）、`topic`（限定专题））；另有 `version`、`bank`。无目标时给出提示 |
| `/browse` | `bank`、`q`（关键词） |
| `/glossary` | `q`（关键词） |
| `/zone-map` | `q`（呼号 / 实体名，定位分区） |
| `/callsign` | `call`（呼号，打开即自动在线查询） |
| `/print` | `src`（`mistakes` \| `bookmarks`）、`bank`（A\|B\|C） |
| `/cards` | `deck`（`qcode` \| `abbrev` \| `phonetic` \| `morse` \| `glossary`） |
| `/contest-log` | `contest`（预选竞赛模板 ID，如 `CQ-WW-CW`） |

其余路径回退到 404 页。

| 路径 | 参数 |
| --- | --- |
| `/practice` `/exam` | `version`（题库版本）、`bank`（A\|B\|C）；`/practice` 另有 `multi=1`（只练多选，页面顶部有同名开关），`/exam` 另有 `mode=weak`（薄弱项组卷） |
| `/map` | `version`、`bank`（A\|B\|C） |
| `/battle` | `stage`（一级分类 key，如 `法规`；闯关）或 `mode=revenge`（复仇，另可带 `key`（单只怪物的错题 key）、`topic`（限定专题））；另有 `version`、`bank`。无目标时给出提示 |
| `/browse` | `bank`、`q`（关键词） |
| `/glossary` | `q`（关键词） |
| `/zone-map` | `q`（呼号 / 实体名，定位分区） |
| `/callsign` | `call`（呼号，打开即自动在线查询） |
| `/print` | `src`（`mistakes` \| `bookmarks`）、`bank`（A\|B\|C） |
| `/cards` | `deck`（`qcode` \| `abbrev` \| `phonetic` \| `morse` \| `glossary`） |
| `/contest-log` | `contest`（预选竞赛模板 ID，如 `CQ-WW-CW`） |

其余路径回退到 404 页。

