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
| `remote-sync` | 自定义远端同步配置（获取 / 写入接口地址、写入方法，以及获取、写入**各自一份**请求头，读写可用不同令牌；是否去掉 QSL 影像）；令牌明文只存本机，**不进备份**，见下 |
| `remote-sync-state` | 自动同步状态：`base`（上次同步成功时远端内容的指纹）、`dirty`（本机有没写出去的改动）；**不进备份** |

> 通联日志（`logbook`）可能超过 localStorage 5MB 配额：日志改由 `IndexedDB` 权威存储 + `localStorage` 静默快照双写（见 `crates/app/src/kv.rs` / `idb.rs`），跨标签页可见性由 `storage` 事件保证。数据备份仍以 localStorage 快照为准；日志极大（快照被静默丢弃）时建议同时用「通联日志 → 导出 ADIF」留底。

## 自定义远端的自动同步

配了获取 / 写入接口后（`/backup` 页），应用会自动同步；实现见 `crates/app/src/auto_sync.rs`，决策逻辑与重试策略见 `crates/core/src/remote_sync.rs`（`plan_startup` / `retry_delay_ms`，带单测）。

- **接口约定**：
  - **获取**：`GET <get_url>`，响应体是与「导出备份」同格式的 JSON 文本，原样返回（不包装、不 base64）。
  - **写入**：`<method> <put_url>`（`PUT` 或 `POST`）提交 `multipart/form-data`，**只有一个字段 `file`**：文件名 `backup.json`、类型 `application/json`、内容是那份备份 JSON。任何 2xx 都算成功。
  - **`Content-Type` 由浏览器生成**（值里要有 boundary）。写入请求头里填的 `Content-Type` 会被**忽略**——它会盖掉 boundary，服务端就读不到 `file`。旧版本的写入是裸 JSON、允许用这一行改类型，升级后留在本机的这类配置不会报错，只是不再生效。GET 没有请求体，用户的请求头原样发出。
  - **接口契约变更**：写入从 `application/json` 裸 JSON 改成 `multipart/form-data`（0.4.14）。按旧约定搭的服务端要改成读 `file` 字段。
  - **内存**：构造 `FormData` 的过程中，Rust 侧的 JSON 字符串与 JS 侧的 `Blob` 各占一份；备份上限 64 MiB，低内存的手机偏紧。所以构造完立刻释放 Rust 侧的 JSON 与键值表，上传期间只留 `FormData` 里那一份。

- **打开页面时自动获取**：
  - 远端内容没变（指纹等于基线）就什么都不做；
  - 远端变了而本机自上次同步后没有改动，直接**覆盖**本机；
  - 远端是空的、这组接口**以前同步成功过**、本机有数据：自动补传本机；
  - 其余情况**暂停自动写入并提示**，由用户处理：两边都有改动、本机已有数据却从没同步过、获取失败，以及——
  - **这组接口从没同步成功过而远端是空的（404 / 204 / 空内容）**：不自动上传。404 可能只是获取地址写错了，要用户先手动点一次「写入远端」确认地址，之后才自动同步。
- **取回远端期间用户的操作也算本机改动**：取远端要等网络，这段时间用户可能已经在答题 / 记通联。第一次指针按下 / 按键**之后**发生的存储写入会并进「本机有未同步改动」，因此落到冲突而不是被覆盖；应用自己在启动时的写入（迁移标记、每日打卡）发生在用户碰页面之前，不算。
- **数据变化后自动写入**：任何非设备专属 key 的存储写入都会触发，约 5 秒后写出完整数据，两次写入至少间隔 30 秒；内容与基线一致就不发请求；切走标签页时会尽力立即写。写出的范围**与「导出备份」的模块勾选无关**（远端是整体替换的快照，漏掉日志等于把它从远端抹掉），唯一能去掉的是体积最大的 QSL 影像（面板里的「同步时不含 QSL 影像」，配置项 `exclude_qsl_images`）。
- **写入失败分两类**：网络 / 超时 / HTTP 5xx（以及 408 / 425 / 429）是暂时的，按 60 秒起指数退避重试（封顶 30 分钟）；其余（401 / 403 令牌不对、404 / 405 地址或方法不对、413 超限、地址或请求头不合格、数据超过 64 MiB、设备内存不足无法生成上传内容）重试无益，**暂停自动写入并说明原因**，用户处理后手动点一次「写入远端」即可恢复。
- **拉取之后会补写**：覆盖导入是按 key 叠加的，远端没有、本机独有的 key 会留下；所以手动恢复 / 合并以及自动快进之后（配了写入接口时）都会置脏位，让下一次自动写入把差异补回远端——写出前会再比一次指纹，没有差异就不发请求。只配了获取接口时无处可写，覆盖后清掉脏位。
- **为什么自动获取不用合并**：合并对答题统计是累加的（`merge_study_stats`），同一份远端数据合并两次计数就翻倍，不幂等。
- **远端指纹与本机同口径**：远端内容求指纹前先去掉设备专属 key，所以远端是一份手工导出的完整备份（带主题、接口缓存…）也不会因此永远对不上本机。
- **设备专属、不同步的 key**（`is_device_local_key`）：`theme`、`locale`、`ui:*`、`fab-position`、`cat-settings`（电台串口参数跟着这台设备连的那台电台走）、`api-cache:*`、`*:kv-version`、`bank-digest:*`、`push-subscription`、`app:changelog-seen`、`sat-*-notified`、`nec.pending-template`，以及 `remote-sync` / `remote-sync-state` 本身。**新增本地存储 key 时必须在上面的表里登记，并在 `crates/core/src/remote_sync.rs` 的 `every_documented_key_is_classified` 测试里归类（同步 / 设备专属）**——漏归类测试会红，而不是悄悄变成「每次写入都触发一次同步」。
- **手动导入直接写 `localStorage`**，不经过存储钩子，要自己调 `auto_sync::mark_dirty()`。
