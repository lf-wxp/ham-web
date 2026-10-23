# 本地存储 key 一览

> 所有 key 都只存在浏览器本地，不上传服务器。权威数据在 `localStorage`；
> 通联日志等大体积数据由 `IndexedDB` 权威承载、`localStorage` 只留快照（见 `crates/app/src/kv.rs`）。
> 导出 / 导入走站内「数据备份」（`/backup`）。

题库相关的 key 与数据结构与旧版（Next.js）完全一致，升级后用户已有进度与偏好可继续使用：

| key | 内容 |
| --- | --- |
| `practice:{版本}:{题库}` | 练习进度（顺序模式） |
| `practice:lastMode` | 上次练习题序 |
| `practice:noResumePrompt:{版本}:{题库}` | 本题库不再提示恢复 |
| `exam:savedState:{版本}:{题库}` | 未完成的考试 |
| `exam:answerCardFilter:{题库}` / `exam:showExplanation:{题库}` | 考试偏好 |
| `ui:shortcutsHelpSeen:{practice,exam}` | 快捷键说明是否已展示 |
| `theme` | `light` / `dark` / `system` |
| `locale` | `zh` / `en` / `es`（界面语言） |

后续新增功能的 key（同样只存在本地）：

| key | 内容 |
| --- | --- |
| `bookmarks` | 收藏题目的 stable_id 集合 |
| `exam-history` | 模拟考试成绩历史（最多 50 次；薄弱项组卷带 `weak` 标记，不计入备考状态） |
| `logbook` | 通联日志（`/stats`、`/grid-map`、`/progress`、`/most-wanted` 共用） |
| `station-info` | 本台信息（呼号 / 操作员 / 网格 / 设备 / 天线，写入 ADIF 台站字段） |
| `daily-checkin` | 首页每日打卡与连续天数 |
| `countdowns` | 倒计时条目 |
| `morse-stats` | 摩尔斯解码练习统计 |
| `dxcc_wanted_done` | 已通联的 DXCC 稀有前缀集合 |
| `mistake-book` | 错题本（题目快照、错误次数、连续答对次数、难度系数、复习间隔、下次复习时间，最多 1000 题） |
| `study-stats` | 按分类累计的作答数与正确数（含 A/B/C 分题库统计与已做过题目集合） |
| `mistake-book:seeded` | 是否已从旧版练习 / 考试存档迁移错题 |
| `study-stats:seen-seeded` | 是否已从旧版存档补齐「已做过」题目集合 |
| `morse-koch` | Koch 法抄收训练级别、速度与逐字符统计 |
| `dx-alerts` | DX 热点通知设置（新 DXCC / 新波段 / 关注呼号） |
| `learning-plan` | 备考计划（考试日期与类别） |
| `study-daily` | 每天的作答量（保留 60 天） |
| `morse-runner` | CW 呼号抄收设置与统计（最高分、累计通联、常错字符） |
| `contest-session` | 竞赛录入当前场次（竞赛、开始时间、本方交换信息、频率 / 模式 / 功率） |
| `sat-watch` | 卫星过境设置（位置、最低仰角、收藏卫星、提醒开关与提前量） |
| `sat-watch-notified` | 已提醒过的过境，避免重复通知 |
| `grid-map-filters` | 日志网格地图筛选条件（含是否显示通联路径） |
| `listen-settings` | 听题模式设置（题源、语速、思考时间、是否读解析、各题源听到的位置） |
| `cat-settings` | 电台 CAT 协议、波特率与 CI-V 地址 |
| `qsl-label-layout` | QSL 标签纸版式 |
| `card-review` | 知识卡片复习进度（每张卡的难度系数、间隔、下次复习时间，及今日已学新卡数） |
| `app:changelog-seen` | 已看过的更新说明日期 |
| `bank-digest:{A\|B\|C}` | 上次加载的题库修订号与题目摘要，用于提示题库变化 |
| `question-notes` | 题目私人笔记（题目 ID → 笔记文本） |
| `bookmark-groups` | 收藏分组 |
| `question-stats` | 题目级作答统计 |
| `achievements-seen` | 已提示过的成就解锁 id |
| `push-subscription` | Web Push 订阅的本地快照 |
| `morse-trainer` / `morse-send` | Koch 抄收与按键发报训练设置 |
| `sat-apt-notified` | 已提醒过的 APT 录制机会 |
| `exam:last-review` | 上次考后复盘的题目与作答 |

> 通联日志（`logbook`）可能超过 localStorage 5MB 配额：日志改由 `IndexedDB` 权威存储 + `localStorage` 静默快照双写（见 `crates/app/src/kv.rs` / `idb.rs`），跨标签页可见性由 `storage` 事件保证。数据备份仍以 localStorage 快照为准；日志极大（快照被静默丢弃）时建议同时用「通联日志 → 导出 ADIF」留底。
