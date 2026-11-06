# 在线账号与同步通道

> 写给**要动同步逻辑的人**：站点现在有**两条并列的同步通道**——「自定义远端」（用户自己的两个接口）与
> 「在线账号」（本站服务器上的 SQLite）。本文讲它们怎么共存、各自存什么、冲突怎么判断、数据怎么持久化。
> 接口契约见 [`reference/api.md`](../reference/api.md)，本地存储 key 见 [`reference/storage.md`](../reference/storage.md)，
> 自定义远端自己的接口约定与自动同步细则见 `storage.md`「自动同步（两条通道）」。

## 1. 一句话模型

两条通道**交换同一份内容**——不含设备专属 key 的备份 JSON（`ham_web_core::remote_sync::is_device_local_key`
划线），区别只在「去哪」：

| | 自定义远端 | 在线账号 |
| --- | --- | --- |
| 去处 | 用户填的 `GET` / `PUT\|POST` 两个接口 | 同源 `/api/account/*`，存进服务器 SQLite 的多张表（见 §6.1） |
| 鉴权 | 用户自填请求头（令牌明文存在本机） | `HttpOnly` 会话 Cookie（页面脚本碰不到令牌） |
| 谁来搭 | 用户自己 | 站点部署方（设 `ACCOUNT_DB`） |
| 同步粒度 | 整份内容（只能比整份指纹，不一样就整份覆盖） | **按 key**：只交换有变化的那几条，各改各的互不打扰（§5） |
| 并发保护 | 没有（对方服务说了算） | 有：每条改动带着「我看到的那一版」，云端对不上就整批拒绝（`412`） |

它们**不互相替代**：不想自建接口的人用账号，已经有自己服务的人继续用自定义远端；两个都配也行。

## 2. 共存规则：自动同步一次只跑一条

自动同步靠**基线**（上次同步成功时对端是什么样）加一个**脏位**判断「覆盖安不安全」。
两条通道同时自动跑，从 A 拉下来的内容对 B 来说就是「本机变了」，B 写完 A 又觉得变了……基线互相作废，
谁也说不清两边是否分叉。所以：

- **自动**：任一时刻至多一条在跑（`ham_web_core::sync_channel::resolve`）；
- **手动**：两条通道的「上传 / 从云端 / 远端恢复（覆盖）/ 合并」按钮始终可用，互不影响；
- 每条通道**各存各的状态**（`remote-sync-state` / `account-sync-state`），互不覆盖。

谁在自动同步，由 `sync-channel` 偏好决定：

| `sync-channel` | 含义 |
| --- | --- |
| 没有这个 key | 没选过：配了自定义接口就用自定义远端（**与升级前的行为一致**），没配就不自动同步 |
| `custom` / `account` | 用户在面板里打开了对应的「用 … 自动同步」开关 |
| `off` | 用户明确关掉了自动同步（两个开关都关）；登录也不会替他打开 |

界面上是两个开关（各在自己的面板里），**同一时间至多一个是开的**：打开一个，另一个自然关掉；
开关反映的是 `auto_sync::active_channel()`，不是各自存的布尔值，所以不会出现「都开着」。

登录 / 退出对偏好的影响（`pref_after_login` / `pref_after_logout`，都带单测）：

| 事件 | 结果 |
| --- | --- |
| 登录，且没选过、也没配自定义接口 | 自动选上账号（登录就是想用它同步） |
| 登录，已配自定义接口 / 已选过 / 已关掉 | 不抢，让用户在界面上自己开 |
| 退出登录 / 注销账号 | 偏好是 `account` 则回到「没选过」；账号通道的基线作废 |

## 3. 换通道、换账号为什么不沿用旧基线

通道不在跑的时候，本机数据的改动**没人追踪**（写入钩子只盯着正在跑的那条）。留着旧基线再回来，
就会把「期间改过」当成「没改过」而放心用远端覆盖本机——静默丢数据。因此：

- **离开即作废**：切走 / 退出时自定义远端用 `leave()`、账号用 `AccountSync::leave()` 清掉基线（账号还有主人），
  **脏位保留**（它的意思是「可能有没传出去的改动」）。回来时按「从没同步过」处理：自定义远端本机有数据而远端也有
  → `Conflict`；账号则逐个 key 对比（一样的不动，不一样的按 §5 的规则），宁可多问一次，也不静默覆盖；
- **基线有主人**：账号通道的 `AccountSync.owner` 记着基线属于哪个账号 id。
  同一台浏览器换账号登录时 `AccountSync::adopt_owner()` 作废不属于它的基线；
- **异步任务带「代」号**（`auto_sync` 的 `GEN`）：切换通道后，旧通道还在路上的请求回来，
  不许改新通道的状态；
- **另一条通道的手动操作不记基线**：`note_pushed` / `note_pulled` 只在「这条通道正在自动同步」时才改状态。

## 4. 启动时怎么判定

**自定义远端**沿用 `plan_startup`（见 `storage.md`）：对端只有一份整体内容，只能比整份指纹。自定义地址是手敲的，
`404` 可能只是写错，所以从没同步成功过而远端是空的 → `NeedsFirstPush`，**不自动上传**，等用户手动传一次确认。

**在线账号**不比整份指纹，而是跑一轮按 key 的增量同步（§5）。去处是登录态本身，登录成功就说明地址与身份都对，
云端为空只是「还没传过」，直接传，没有 `NeedsFirstPush`。启动时多一步：先 `GET /api/account/session` 问登录态。

| 登录态 | 结果 |
| --- | --- |
| 已登录 | `AccountSync::adopt_owner` 认主人（基线不属于它就作废），再跑一轮 |
| 没登录（含会话过期） | `AutoStatus::SignedOut`，暂停并说明；登录后自动重新判定（`auto_sync::account_signed_in`） |
| 连不上 / 服务器没启用账号 | 当作「获取失败」暂停，不碰本机数据 |

**「本机有没有用户数据」只用来认「全新浏览器」**：登录发生在页面运行中，此刻的存储里已经有应用自己启动时写的默认值，
拿它判断，全新浏览器也会被当成「用过的」。所以用「打开页面前就有（`snapshot` 在应用写任何东西之前取的）」
加「打开之后用户写过（第一次指针 / 按键之后的存储写入，**不论有没有通道在自动同步都记**）」。
全新浏览器两边都有而不同的 key 以云端为准（那是应用自己的默认值，不是用户的改动，`prefer_remote`）。

## 5. 在线账号怎么同步：按 key 增量

一个 key 就是一条有稳定身份的记录（`bookmarks`、`mistake-book`、每张 `qsl-image:<编号>`、每个 `practice:{版本}:{题库}`……）。
旧协议每次传整份快照：一个 key 变了，几十 MB 的影像也要重传一遍；两台设备各改各的，后写的那台还会被判成冲突。
现在每个 key 有一个**摘要**（规范化之后文本的 SHA-256），云端给出全部 key 的清单，双方只交换有变化的那几条。

算法是纯逻辑，两端共用、带穷举测试：`ham_web_core::delta`（摘要、三方对比、冲突规则）与
`ham_web_core::delta_sync`（把它们串成一轮「取回 → 落到本机 → 上传」，对云端和本机都是泛型的）。
`crates/app` 只提供两个浏览器里的实现：云端是 `account_cloud`（三个同源接口），本机是 `auto_sync/account.rs` 里的
`BrowserDevice`（读写 `localStorage` / IndexedDB，落盘逻辑在 `backup_apply`）。

### 5.1 基线是每个 key 的一对摘要

每个 key 的基线记**两个**摘要（`delta::Synced`）：上次同步成功时**云端**那份、和当时**本机**那份。
「本机有没有改」只拿本机现在的摘要与本机当时的比，「云端有没有改」只拿云端现在的与云端当时的比。

为什么是一对：服务端存的是**规范化**之后的文本（补默认值、认旧字段名、丢未知字段），老版本写出的内容，
本机算的摘要与云端清单里的永远对不上。若把「两边摘要不同」当成「有改动」，这类 key 每一轮都会被重传、永远稳定不下来；
把规范文本写回本机又会让比服务端新的前端丢字段。记一对就没有这个问题——两边语义等价而字节不同，不是任何一边的改动。

### 5.2 三方对比

| 本机 vs 云端 | 本机改了 | 云端改了 | 结果 |
| --- | --- | --- | --- |
| 摘要一样 | | | 不用动 |
| | 没改 | 没改 | 不用动（等价但字节不同） |
| | 没改 | 改了 | 取回云端的（含云端删除） |
| | 改了 | 没改 | 上传本机的（含本机删除） |
| | 改了 | 改了 | 两边都改了：按 §5.3 |

没有基线（从没同步过、换了账号、云端被清空）等于基线是空的：本机独有的 key 上传，云端独有的取回，
两边都有而内容不同的才算「都改了」。不需要墓碑：删除就是「基线里有、现在没有」。
云端是空的时基线一律作废，**不**当成「云端把所有东西都删了」。

### 5.3 两边都改了同一个 key

每个 key 有一个处理方式（`delta::classify`；新增本地存储 key 时必须归类，漏了测试会红，未归类的按「问用户」）：

| 类别 | 哪些 key | 怎么办 |
| --- | --- | --- |
| 合并 | `bookmarks`、`dxcc_wanted_done`、`exam-history`、`mistake-book`、`daily-challenge`、`rpg-stars`、`rpg-bestiary`、`station-book` | 取回云端的与本机合并（并集 / 去重 / 取最优，重复合并结果不变；复用手动「合并」的 `merge_value`），再传上去 |
| 后写为准 | 设置、偏好、断点：`practice:*`、`exam:*`、`learning-plan`、`listen-settings`、`contest-session`…… | 以本机为准上传，不打扰用户（只有最新一份有意义） |
| 问用户 | 累计计数与列表：`study-stats`、`question-stats`、`card-review`、`question-notes`、`countdowns`、`morse-stats`……，以及**通联日志与 QSL 影像**；没归过类的 key | 停下来：合并会重复累加，任选一边会丢另一边的积累 |

有合并规则的 key，一边改一边删：合并无从谈起，也问用户。

### 5.4 删除不一定传播

「本机没有了」可能是用户删的，也可能是浏览器清了站点数据、IndexedDB 被回收。
**设置与进度、QSL 影像**（用完就删、用户会删掉一张扫描件）的删除照传；
**日志、错题、统计这类积累出来的数据**，一边没有、另一边没动，一律当成「丢了，补回来」，而不是「该删」。
再加一道保险：一轮要删的 key 至少 5 个且超过全部 key 的一半（`delta_sync::suspicious`），多半不是用户删的，停下来问
（`AutoStatus::MassDeletion`），用户可「从云端恢复」找回，或「上传到云端」确认。

### 5.5 日志与影像是一个整体

影像靠通联编号挂在日志上，而编号是各设备本地自增的：两边各记了一条编号相同的不同通联，本机那张编号相同的影像
传上去，就会挂到云端的另一条通联上。所以日志这一边不是「只有本机改了」（取回云端的、冲突）时，本机要往外传的影像一律先停下，
当作冲突；要处理就用手动「合并」（它按通联身份重新编号再挪影像）。往里取的影像不受影响，跟着云端的日志走。
`qsl-image-index`（本机有哪些影像的索引）不同步：它由影像本身推出来，各设备带的影像集合可以不同，同步它只会互相覆盖。

### 5.6 一轮的步骤与两种模式

1. 取清单，读本机现在的内容，三方对比；
2. 有要问用户的 key（**启动模式**）就**什么都不动**；
3. （启动模式）把云端的变化落到本机：覆盖、合并、删除。**落之前核对每个要写的 key 本机还是不是读快照时的样子**，
   不是（用户在取回期间动了数据）就整轮重来，用户的改动自然成了「本机改了」的一边；
4. 重新读本机再对比，把本机的改动按 key 上传，每条带着「我看到的云端那一版」，对不上整批作废，整轮重来；
5. 把每个 key 两侧的摘要记进基线。

页面刚打开时（**启动模式**）可以把云端的变化落到本机；之后的**自动写入只上传**：页面里的信号已经读过存储，
中途改写存储，用户下一次操作就会拿旧值盖掉它。需要写本机的变化留到下次打开，需要合并的 key 报冲突。
落盘时拿备份锁（页面此时 `inert`，核对与写入之间没有人能改数据）。一轮最多重来 3 次，云端一直在变就放弃（`Busy`），
按退避稍后再试。落盘时内容不合格的 key（影像格式不对、没有合并规则能处理）记为 `skipped`，**不记基线**，
否则下一轮会把「本机没有」当成「用户删了」传出去；它们的个数进 `Outcome::skipped`（跨重来累计、同一个 key 只算一次），
启动那一轮据此提示用户一次（`tools.account-auto-skipped`），而不是每次打开都悄悄重试。

**基线分步存**：引擎在每次落盘本机之后、每一批上传生效之后都调用 `Device::checkpoint(&base)`，浏览器侧
（`auto_sync::checkpoint_keys`）立刻存进 `account-sync-state`，不等整轮结束。否则上传分批时，第一批已经写进云端、
第二批还没发出去，这时关页：已写进云端的 key 在本机没有基线，本机文本与云端规范文本的摘要往往不同，
下次启动会被当成「两边都改了」，累计类的 key 因此误报冲突。存之前核对基线纪元（`KEYS_EPOCH`）没变，
手动上传 / 恢复、登出、换账号在路上替换了基线，这一轮手里的旧基线就不能再盖掉它们。

用户在取回期间动了数据，这台设备就不再是「只有应用默认值的全新设备」：重来时不再无条件以云端为准。

### 5.7 接口与并发保护

三个同源接口（契约见 [`reference/api.md`](../reference/api.md)）：

| 接口 | 用在 |
| --- | --- |
| `GET /sync/manifest` | 云端全部 key 的摘要，读目录、不读内容 |
| `POST /sync/pull` | 按 key 取值（规范形态） |
| `POST /sync/push` | 按 key 上传改动；每条带 `base`（我看到的这个 key 在云端的摘要，没有为 `null`），整批要么全生效要么全不生效；写之前按净增量预估，账号总量会超过 64 MiB 就整批拒绝（`413`，拒绝时没有写过任何东西）；同一个 key 在一批里出现两次回 `400` |

检查与写入在**同一个写事务里**，两台设备同时改同一个 key，只有一个成功；各改各的 key 谁也不会挡住谁。
整份的 `GET` / `PUT /backup` 仍在，给手动的「上传 / 恢复 / 合并」用（用户明确要覆盖，不设条件）。

云端的**版本号**（`ETag` / `If-Match` / `backup_meta.fingerprint`）是对 `(key, 摘要)` 清单求的指纹
（`delta::manifest_fingerprint`），**不用读出任何内容**，所以只改一个 key 的增量写入也能在事务里算出新版本。
整份接口与增量接口共用同一个版本号；服务端与前端对「哪些 key 参与同步」只有一处定义（`delta::in_scope`：
排除设备专属 key、`qsl-image-index`、超过 512 字节的 key；账号选项可去掉影像）。

**手动操作之后的基线**（纯函数，带测试）：手动上传后云端恰好等于本机，基线是回执里的清单配上传的文本摘要
（`base_after_push`）；手动恢复 / 合并后，每个云端有、本机也有的 key 记成「两边当时都停在云端那一版」
（`base_after_pull`）——这样合并出的结果、本机独有的改动，下一轮是「本机改了、云端没动」，直接上传，
而不是被当成「两边都改了」卡在累计类的 key 上。

另一个串写场景：两个标签页登录了不同账号（Cookie 共享），旧标签页还在自动写。
前端在每个请求里带 `X-Account-Id`，服务端核对与当前会话不一致就回 `409 account_changed`。

## 6. 数据结构

### 6.1 服务端 SQLite（`crates/server/src/account/`）

账号与会话各一张表；账号名下的同步数据**拆进多张表**（`relational/`），不再是「一行一整份 JSON」：

```text
users             id PK AUTOINCREMENT · username UNIQUE COLLATE NOCASE · password_hash · created_at · password_changed_at
sessions          token_hash BLOB PK · user_id → users(id) ON DELETE CASCADE · created_at · last_seen_at · expires_at · user_agent

backup_meta       user_id PK → users · fingerprint TEXT · size · updated_at          ← 云端备份的概况；fingerprint 是对 key 清单求的版本号（§5.7）
user_kv           (user_id, key) PK · value TEXT NULL · storage 'inline'|'table' · digest BLOB(32) · bytes   ← 目录：每个同步 key 一行；digest 是规范文本的 SHA-256，也是清单里这个 key 的摘要

── 通联与台站 ──
qsos              (user_id, id) PK · pos · 40 个通联字段各一列                         ← logbook
station_books     user_id PK · next_id · active_id                                    ← station-book
station_profiles  (user_id, id) PK · pos · label · callsign · operator · gridsquare · rig · antenna
station_info      user_id PK · callsign · operator · gridsquare · rig · antenna        ← station-info（当前台站的镜像）
qsl_images        (user_id, qso_id) PK · mime · data BLOB                             ← qsl-image:<编号>（一族 key，一图一行）
qsl_image_index   (user_id, pos) PK · qso_id                                          ← qsl-image-index

── 题库学习 ──
wrong_questions   (user_id, bank, pos) PK · 题面各一列                               ← exam:wrong:{A|B|C}（一族 key，一个题库一份）
wrong_question_options (user_id, bank, pos, opt_pos) PK → wrong_questions · opt_key · text
wrong_question_answers (user_id, bank, pos, ans_pos) PK → wrong_questions · value
mistakes          (user_id, key) PK · pos · 题面 / 元数据各一列 · 计数、时间、ease …    ← mistake-book
mistake_options   (user_id, key, pos) PK → mistakes · opt_key · text
mistake_choices   (user_id, key, kind, pos) PK → mistakes · value   （kind = answer / mine）
mistake_banks     (user_id, key, bank) PK → mistakes   （错过的题库，多对多）
exam_results      (user_id, pos) PK · bank · correct · total · timestamp · weak        ← exam-history
exam_reviews      user_id PK · bank · finished_at_ms                                   ← exam:last-review
exam_review_items (user_id, pos) PK → exam_reviews · id · code · question · explanation · category
exam_review_options (user_id, pos, opt_pos) PK → exam_review_items · text
exam_review_choices (user_id, pos, kind, choice_pos) PK → exam_review_items · value   （kind = answer / given）
practice_progress (user_id, key) PK · 题库 / 版本 / 位置 · order_indices / answers（JSON 列，全库仅有的两处之一）   ← practice:{版本}:{题库}（一族 key）
exam_progress     (user_id, key) PK · … · answers / flags / question_ids / questions_snapshot    ← exam:savedState:{版本}:{题库}（一族 key）
study_scopes      (user_id, bank) PK · answered · correct   （bank 为空 = 全部）       ← study-stats
study_tallies     (user_id, bank, kind, key) PK → study_scopes · answered · correct   （kind = category / sub）
study_seen        (user_id, bank, question_key) PK → study_scopes
question_stats    (user_id, question_key) PK · attempts · correct · last_ms · duration_ms   ← question-stats
study_daily       (user_id, day) PK · answered · new_count · duration_ms               ← study-daily
daily_challenge   (user_id, day) PK · correct · total                                  ← daily-challenge
bookmarks         (user_id, pos) PK · value                                           ← bookmarks
bookmark_groups   (user_id, pos) PK · name                                             ← bookmark-groups
bookmark_group_items (user_id, group_pos, pos) PK → bookmark_groups · stable_id
question_notes    (user_id, pos) PK · stable_id UNIQUE · note                          ← question-notes（pos 保住前端写出的键序）
card_schedule     user_id PK · new_day                                                 ← card-review
card_states       (user_id, id) PK → card_schedule · ease · interval_days · due_ms · last_ms · reps · lapses
card_new_today    (user_id, deck) PK → card_schedule · count
stage_stars       (user_id, stage) PK · stars                                          ← rpg-stars
bestiary          (user_id, question_key) PK                                           ← rpg-bestiary
achievements_seen (user_id, pos) PK · value                                           ← achievements-seen

── 莫尔斯与电台 ──
koch_progress     user_id PK · level · char_wpm · effective_wpm · level_sent · level_correct   ← morse-koch
koch_chars        (user_id, ch) PK → koch_progress · sent · correct
runner_stats      user_id PK · wpm · adaptive · … · top_wpm                            ← morse-runner
runner_missed     (user_id, ch) PK → runner_stats · missed
morse_stats       user_id PK · correct · wrong · best_streak · top_wpm · today …      ← morse-stats
morse_mistakes    (user_id, ch) PK → morse_stats · count
morse_daily       (user_id, day) PK → morse_stats · correct · wrong
morse_trainer     user_id PK · mode · scope · target（CHECK 逐个变体）· wpm · eff_wpm · adaptive   ← morse-trainer
morse_send        user_id PK · correct · wrong · best_streak                          ← morse-send
morse_settings    user_id PK · tone_hz · volume                                       ← morse-settings
dxcc_wanted_done  (user_id, pos) PK · value                                           ← dxcc_wanted_done

── 工具与器材 ──
gear_measurements (user_id, gear_id) PK · 四项实测值（可为 NULL）· measured_on        ← gear-measurements
diy_progress      (user_id, project) PK · done                                        ← diy-progress
diy_bought        (user_id, project, item) PK → diy_progress
noise_samples     (user_id, pos) PK · label · at_ms · floor_db · occupied_ratio       ← noise-floor-samples
nec_designs       (user_id, id) PK · pos · name · saved_at · freq_hz · ground · eps_r · sigma · conductivity   ← nec-designs
nec_wires / nec_feeds / nec_loads / nec_lines   (user_id, design_id, pos) PK → nec_designs   （按 pos 保序：馈电等用导线下标引用导线）

── 页面自己的小文档 ──
countdown_events  (user_id, id) PK · pos · title · target_ms                          ← countdowns
daily_checkin     user_id PK · last_date · streak                                     ← daily-checkin
contest_session   user_id PK · contest_id · start · end_at · station_id · …           ← contest-session
grid_filters      user_id PK · band · mode · dxcc · year · callsign · qsl · paths     ← grid-map-filters
listen_settings   user_id PK · source · rate · think_secs · explain · shuffle         ← listen-settings
listen_positions  (user_id, source) PK → listen_settings · position

── 设置 ──
study_plan        user_id PK · target_date · bank · reminder_time                      ← learning-plan
dx_alerts         user_id PK · new_dxcc · new_band                                     ← dx-alerts
dx_alert_calls    (user_id, pos) PK → dx_alerts · pattern
sat_watch         user_id PK · lat · lon · min_elev · alerts · lead_min · apt_alert    ← sat-watch
sat_favorites     (user_id, pos) PK → sat_watch · norad · name
```

专表全部外键级联到 `users`（子表再级联到自己的父表）：注销账号时一次清干净。

**导入即规范化——任何一份合法的备份都能正确进表**：

- 同步内容是 `{ key: 文本 }`，文本不一定是 JSON（`theme`、`practice:lastMode`、损坏时留下的原文……），
  也不一定是今天的形态（老版本应用导出的文件、手改过的）；
- 有专表的 key，按 `ham_web_core` 里的类型**宽松解析**：缺的字段补默认值、旧字段名按别名认（`sota_ref` → 自己所在的山峰）、
  多出来的未知字段丢弃，然后存进各张表；取回时由类型序列化成当前形态。**不要求取回的与上传的逐字节相同**——
  老版本的备份就该按今天的结构进表，而不是躲在 `user_kv` 里；
- 不进专表的只有三种：这个 key 没有专表；值解析不了（不是 JSON、类型不对、缺必填字段）；
  违反行级约束（日志编号 / 台站 id / 错题 key 在账号内重复、进度里的题库与 key 对不上……）。
  它们原样放在 `user_kv.value`，一个字节都不丢，也不拦住同一份备份里别的 key——一个坏 key 不该让整个账号同步不了；
- 所以表里的 `CHECK` 只收「解析阶段一定会产出」的取值。约束比解析更严，就会让一份合法的备份在落库时才失败
  （整次导入返回 500）：例如影像类型 `image/jpg`、大小写不同的类型名，解析阶段就统一成 `image/jpeg` 等规范写法；
- `user_kv` 是目录，**每个 key 恰好一行**：专表里没有行时，「`[]`（有这个 key、内容为空）」与「根本没这个 key」
  分不开，目录把它们分开；
- 每个 key 记着上传原文的 SHA-256（`digest`）：没变的 key 不重写，一次只改一条通联不会把图片之类的大值重写一遍。
  行很多的专表（通联日志）更进一步：`Table::replace` 读出旧行按编号对照，只删没了的、只写变了的、
  内容没变只是位置变了的只改 `pos`，加一条通联不会把全部行删光重写。
  写入在**同一个事务**里完成「检查条件 → 让库里恰好等于新备份（多的 key 与行删掉）→ 更新 `backup_meta`」；
- 全表 `STRICT`（列类型是硬约束）、外键级联、`serde_json` 开 `float_roundtrip`（浮点数在「解析 → 存库 → 读回 → 序列化」
  这一圈里位数不变，否则规范化就不幂等）。

**一族 key 的专表**：`practice:{版本}:{题库}`、`exam:savedState:{版本}:{题库}`、`qsl-image:<编号>` 各认领一个前缀，
每个 key 是表里自己的一行。认领只是候选：key 与内容对不上（进度里的题库与 key 不符）、不是进度本身
（`practice:lastMode`、`practice:noResumePrompt:*`）、影像类型不在白名单或数据不是 base64，都原样留在目录里。

**特殊处理**：

- **QSL 影像存成 `BLOB`**：拆成类型 + 二进制，省掉 base64 的 1/3 体积；缺 padding 的 base64 照样收，取回时补齐；
- **列表类的 key 用 `pos` 保住前端写出的顺序**：`bookmarks`、`question-notes`、`achievements-seen` 前端用 `HashSet` /
  `HashMap` 写出，顺序每次都可能不同。要是服务端取回时按字典序重排，前端本机算的指纹就永远对不上云端的，
  退出再登录就会被判成「两边不一样」去问用户。所以按出现顺序存、按同一顺序取；同一个 id 出现两次的 `question-notes` 留在目录里；
- **可选字段分得清「没有」与「空」**：`exam:savedState` 的 `questionIds` / `questionsSnapshot` 缺省时是 `NULL`，
  `[]` 时是 `'[]'`，原文里它们是两种写法；
- **全库仅有的 JSON 列**：练习 / 考试断点进度里按题位对齐的数组（题序、作答、标记、题目快照）——整体读写、
  与别的行没有关系，拆成行只会把一份进度变成几千行，查询上没有任何好处。其余结构化数据（错题的题面和选项、
  复盘的选项和作答……）都拆成了行。

**类型搬进了 core**：`countdowns`、`daily-checkin`、`morse-stats` / `morse-trainer` / `morse-send` / `morse-settings`、
`contest-session`、`grid-map-filters`、`listen-settings` 这些类型原来只定义在 `crates/app` 的页面里，服务端依赖不到。
现在它们在 `ham_web_core`（`countdown` / `checkin` / `morse_store` / `contest::ContestSession` / `grid_filter` / `listen`），
页面和服务端用同一份定义，页面加字段服务端才不会悄悄丢掉；表里的列和代码里的解构都不带 `..`，
加了字段而忘了建列，**会直接编译失败**。每个类型在 core 里有一条「存储格式固定」的测试，字段名和顺序不能悄悄变。

两处顺带的改动：

- **`MorseStats` 的映射从 `HashMap` 改成 `BTreeMap`**：`HashMap` 的键序随机，同一份数据每次写出的文本都可能不同，
  同步指纹就会一直对不上；
- **竞赛场次的「开始时间」由页面补**：core 不读时钟，所以 `ContestSession` 没有 `Default`，页面用 `starting(start)` 构造。

**仍然没有专表的 key**（每个 key 一行存在 `user_kv`，内容照样完整可取）——它们本身就是**一个字符串或开关**，
一行 key / value 已经是最合适的关系形态，拆成表没有任何好处：

- 值不是 JSON 的单个设置：`theme`、`locale`、`practice:lastMode`、`practice:noResumePrompt:*`、`exam:answerCardFilter:*`、
  `exam:showExplanation:*`、`exam:weighted`、`exam:strict`、`qsl-card-template`、`qsl-card-imposition`、`qsl-label-layout`、
  `study-reminder-last`、`log-stats-scope`、`mistake-book:seeded` 等；
- 设备专属的：`ui:*`、`api-cache:*`、`cat-settings`、`fab-position`、`push-subscription`（手动导出会带上，所以照收）；
- 损坏时留下的原文：`nec-designs-corrupt`；
- 比服务端新的前端新加的 key——服务端不认识它，也不会丢它。

新增一张专表：在 `relational/` 里实现 `Table`、在 `codecs()` 登记、在 `schema/content.sql` 建表，并在
`relational/tables_tests.rs`（核心类型）或 `local_tests.rs`（页面小文档 / 工具器材）的 `cases()` 里加一条
「规范值进表、取回一致」的用例——不改接口，也不改前端。类型还在页面里的，先搬进 core。

**没做的**：①合并粒度是一个 key，不是 key 里的一行（比如两台设备各给 `study-stats` 加了一个题，仍要问用户）——
行级合并需要给每条记录加变更版本与删除墓碑；②手动的「上传 / 恢复 / 合并」仍是整份（用户明确要覆盖，语义就是整份）；
③自动写入每次仍要把本机全部内容读出来算摘要（含 IndexedDB 里的影像），只是不再整份上传。

设计取舍：

- **账号名 `COLLATE NOCASE`**：`BG4XXX` 与 `bg4xxx` 是同一个账号；账号名只收 ASCII（`validate_username`），
  所以 NOCASE 就够，不会出现「看起来一样的两个名字」；
- **口令存 Argon2id 的 PHC 字符串**：盐与参数都在串里，将来调参只影响新哈希；并发运算数有上限（占内存）；
  账号不存在时也做一次等价的假校验，响应时间不泄露账号是否存在；
- **会话只存令牌的 SHA-256**：库文件泄露（备份、误传）时拿不到可用的登录态；令牌 32 字节随机数；
  滑动续期（至多每小时写一次，**续期时同时给浏览器重发 `Set-Cookie`**——只续库里的有效期、不续 Cookie 的
  `Max-Age`，天天在用的人到了登录后第 N 天照样被登出），每个账号最多 10 个会话，超出淘汰最久没用的；
- **每个账号一份云端备份**，不做历史版本；体积上限同导入（64 MiB），按各 key 规范文本的字节数之和算；
  另有一道**行数上限**（40 万行，全部专表合计）：字节数挡不住行数，几十字节一行的统计表能把 64 MiB
  展开成上百万行，逐行 INSERT 会把全局写锁占上十几秒。两道上限在整份上传与按 key 增量写入里都核，
  且都在**写完之后、提交之前**核对，超了就整体回滚（`413 too_large`，等于一行没写）。
  规范化会补齐缺省字段，拆出来的内容可能比原文大得多，所以整份上传还是边拆边数、一超上限立刻停
  （`Split::from_map_within`），按 key 的增量上传同理（`Delta::from_changes_within`，超了回 `413`），
  `Store::put_backup` 入库前再核一次；
  key 的口径与增量接口一致（非空、≤ 512 字节、≤ 20000 个），不合规回 `400 invalid_backup`；
  单张 QSL 影像超过 `MAX_STORED_CHARS`（24 MiB）不进影像表，原样留在目录里（仍计入总量）；
- **单个 key 的展开也有上限**（`MAX_TABLE_RAW_BYTES`，32 MiB）：累计预算是「一个 key 处理完之后」才加的，
  而展开一个 key 本身就可能占很多内存——实测最短合法通联 74 字节，解码后每条 744 字节（约 10 倍），
  规范文本还要再占一份。所以原文超过这个数的 key **不展开**，原样留在目录里（仍计入总量、照样同步、取回逐字节一致）。
  真实日志每条约 600 字节，10 万条约 58 MiB，超过 32 MiB 的日志在服务端只是当作一整段文本存放，不丢数据；
  解析完键值表 / 改动列表之后原始请求体立刻释放，不陪着规范化出来的内容一起占内存；
- **读取走只读连接池，而且整次读取在一个读事务里**：`get_backup` / `sync_manifest` / `pull_keys` 是多条语句，
  WAL 下自动提交的每条各取一个快照，中间夹一次别人的提交就会读到「目录是旧的、专表是新的」；读事务让它们共用一个快照
  （有并发读写的用例守着，去掉事务它会红）。池里有 4 个连接，一次几十 MB 的下载只占一个，不挡别人的清单与取值；
  内存库（测试）没有第二个连接，退回写连接并同样包进事务；
- **账号总数有上限**（`ACCOUNT_MAX_USERS`，默认 1000，`0` = 不限）：计数与插入在同一个写事务里；满了回
  `403 registration_closed`（对用户就是"本站不收新账号了"，不另造错误码）；
- **`fingerprint` 存十进制文本**：`u64` 超出 SQLite `INTEGER`（`i64`）的范围；
- **外键级联**：删用户即删会话与全部同步数据（每个连接都要 `PRAGMA foreign_keys = ON`）；
- **结构定义在 `account/schema/*.sql`**，迁移用 `PRAGMA user_version` + 只追加的 `MIGRATIONS`；
  库版本比程序新就**拒绝启动**，不拿旧程序改写新库。目前只有一条（初版）：改成多表时没有任何线上数据，
  所以没有背着旧结构的迁移链，也没有 `backups` 旧表。**从有库在线上跑起来之后才开始只追加**；开发期改了结构，
  删掉本地的库文件（如 `tmp/accounts.db*`）重建即可；
- **版本号对得上不等于结构对得上，启动时还要自检**：迁移链压成一条之前，`user_version = 1` 的库是带 `backups` 表的旧结构，
  只看版本号会放行它，服务照常启动、登录照常成功，之后每个同步接口才报 `no such table`（500）——
  比启动失败难查得多。所以已有版本的库必须真有 `users` / `sessions` / `backup_meta` / `user_kv`，缺了就拒绝启动，
  报出缺哪几张表，并说清怎么办（没有要保留的数据就删库重建）。**程序不升级旧结构**，这是有意的取舍；
- **「导出备份」的文件可以直接导入**：备份文件就是 `{ key: 值 }`（含设备专属的主题 / 接口缓存、`qsl-image:*` 影像、
  损坏的原文），`PUT /backup` 照单全收：有专表的 key 规范化后进专表，其余每个 key 一行进 `user_kv`；
  重复导入同一份是幂等的；
- **`GET /backup` 返回的是从各表拼回的规范 JSON**（紧凑、键按字典序）。每个 key 的值是当前形态：当前版本应用写出的
  与上传的逐字节相同，老版本的会被规范化。每个 key 的摘要与 `ETag` 对的就是这份内容；
- **WAL 模式**：读写互不阻塞，崩溃后以日志恢复。库文件旁会有 `-wal` / `-shm`（见 §8）；
- **库文件新建时权限 `0600`**（unix）：里面有口令哈希与全部用户的备份，不该对同机其他用户可读；
  `-wal` / `-shm` 沿用主文件的权限。已存在的库**权限过宽时**收紧到 `0600`（从别处复制、旧版本创建、数据卷挂载过来的
  可能是 `0644`）；权限本来就对的不去碰。权限过宽而又改不了（文件不属于本进程）时启动失败，这是有意的：
  里面有口令哈希，宁可让运维看到错误去修，也不带着一个别人可读的库悄悄跑起来；
- **指纹、摘要读不出来按数据库错误报**，不悄悄当成 `0`（`0` 也是合法指纹，会让条件写入在错误的基线上通过；摘要不是 32 字节同理）；
  目录与专表对不上（被改坏）同样按数据库错误报，不返回缺了一块的备份。

### 6.2 浏览器本地（`localStorage`，都不进备份）

| key | 内容 |
| --- | --- |
| `sync-channel` | 自动同步偏好：`custom` / `account` / `off` |
| `remote-sync-state` | 自定义远端的 `{ base, dirty }` |
| `account-sync-state` | 账号通道的 `AccountSync`：`{ owner, dirty, keys }`，`keys` 是 `{ key: { r: 云端摘要, l: 本机摘要 } }`（§5.1）；旧版存的整份指纹 `base` 读成「从没同步过」，不报错 |
| `account-sync-options` | 账号通道的选项（是否去掉 QSL 影像） |
| `remote-sync` | 自定义远端的配置（地址与请求头，含令牌） |

**账号的令牌不在这里**：会话在 `HttpOnly` Cookie 里，页面脚本读不到，XSS 偷不走。

### 6.3 两端共用的契约（`ham_web_core::account`）

账号名 / 口令规则、请求与响应结构、错误码（`ErrorCode`，前端按码翻成三语文案）、路径与头名都在 core，
服务端与前端引用同一份常量。指纹是 `u64`，**必须用 `serde_json` 解析**，不要走 JS 的 `JSON.parse`（超过 2^53 丢精度）。

## 7. 安全要点

| 威胁 | 做法 |
| --- | --- |
| 口令泄露（库文件被拿走） | Argon2id；会话令牌只存哈希 |
| XSS 偷登录态 | `HttpOnly` Cookie |
| CSRF | `SameSite=Strict`；JSON 接口一律要求 `Content-Type: application/json`（跨站表单发不出） |
| 撞库 / 暴力猜口令 | **只数失败**（口令对了不耗额度），登录 / 改口令 / 注销共用，分三层：①每个 IP 的登录 / 注册尝试（成败都算）每 10 分钟 20 次；②每个（账号名，来源 IP）的口令失败每 10 分钟 10 次；③每个账号名的口令失败总数（不分来源）每 10 分钟 100 次。②先拦住单个攻击者；③刻意定得高，只在分布式猜口令时才会碰到——额度太低，别人只要知道账号名就能随手把本人锁在门外。成功登录清掉这个来源的②，不清③ |
| 匿名请求占内存 / CPU | 登录 / 注册 / 改口令 / 注销的请求体上限 16 KiB；超长账号名（> 32 字节）/ 口令（> 128 字节）在查库、跑 Argon2 之前就当错误凭据，也不占限流桶；备份上传**先鉴权、先校验头，再拿上传名额（同时 4 份），最后才读请求体**（未登录的请求读不到一个字节；按 key 取值的请求体也是先鉴权再读，上限 1 MiB），解析与算指纹放进 `spawn_blocking`；展开单个 key 的内存有上限（见 §6.1「单个 key 的展开」） |
| 开放注册被刷满磁盘 | `ACCOUNT_MAX_USERS`（默认 1000）。注意最坏情况仍是 人数 × 64 MiB：对公网开放的站点按磁盘预算调这个值，或设 `ACCOUNT_REGISTRATION=closed` |
| 配置写错（`close`、`TRUE`、`30d`） | 启动时严格解析，**不认识的值让进程起不来**，不悄悄回退成默认（`ACCOUNT_REGISTRATION=close` 若被当成"开放"，运维以为关了、实际全网可注册） |
| 探测账号是否存在 | 登录失败不区分「账号不存在」与「口令不对」，耗时也对齐 |
| 超长口令拖慢服务 | 口令上限 128 字节，超长直接当错误口令，不跑哈希 |
| 改口令后旧会话还能用 | 改口令后其他设备的会话全部删除，当前设备保持登录 |
| 非 HTTPS 反代下 Cookie 被截获 | 经 `X-Forwarded-Proto: https` 自动加 `Secure`；`ACCOUNT_COOKIE_SECURE=1` 强制 |

**不做的事**：不发邮件（所以没有「找回口令」——忘记口令只能由部署方改库；请在界面上提醒用户保存好）；
不做第三方登录；不做邮箱 / 手机验证；不做多因素。这些要引入外部依赖或运维面，超出「同步备份」的范围。

## 8. 部署与持久化

没设 `ACCOUNT_DB` 时整组接口回 `503 unavailable`（`GET /session` 例外，仍是 `200` + `enabled: false`），
前端据此显示「未启用」：非 Docker 的既有部署升级后行为不变，也不会在没人知情的地方多出一个数据库文件。

**Docker 镜像是例外**：镜像里 `ACCOUNT_DB` 已设好，所以**升级到这个版本的镜像用户会自动启用账号，并默认开放注册**
（上限 `ACCOUNT_MAX_USERS`，默认 1000）。不想对公网开放注册：设 `ACCOUNT_REGISTRATION=closed`；
想完全关掉账号：运行时传空值 `-e ACCOUNT_DB=`（空值等于没设）。

Docker 镜像里 `ACCOUNT_DB=/app/data/accounts.db`，并声明了 `VOLUME /app/data`。要点：

- **挂整个 `/app/data` 目录，不要只挂 `accounts.db` 一个文件**：WAL 模式在库文件旁生成 `accounts.db-wal` / `accounts.db-shm`，
  只挂单个文件，容器内的写入会落进容器可写层，重建容器就丢；
- **命名卷**（推荐）：`-v ham-web-data:/app/data` 或 `docker compose up -d`（仓库根的 `docker-compose.yml`）。
  删容器、重建镜像后数据仍在；只有 `docker volume rm` / `docker compose down -v` 才会删；
- **宿主目录**：目录要对 UID 65532（镜像里的 `nonroot`）可写；
- 备份：`docker run --rm -v ham-web-data:/data -v "$PWD":/out busybox tar czf /out/ham-data.tgz -C /data .`
  （最好先 `docker stop`，保证 WAL 已落盘）。

环境变量见 [`engineering/deployment.md`](../engineering/deployment.md)。

## 9. 怎么验

| 层 | 测试 | 守什么 |
| --- | --- | --- |
| core | `sync_channel::tests` / `account::tests` / `remote_sync::tests` | 通道解析、登录 / 退出对偏好的影响、离开即作废基线、账号基线认主人、规则校验、条件头解析、旧状态仍读得出（含旧版账号状态读成「从没同步过」） |
| core | `delta::tests` | 摘要是标准 SHA-256 且十六进制往返；清单只含参与同步的 key；**三方对比逐行对照 §5.2 的表**；规范化造成的字节差异不算改动；删除只在设置 / 进度 / 影像上传播；每类 key 的冲突处理方式（含每个文档化的同步 key 都归过类）；**两台设备的对比互为镜像**；日志与影像整体（本机影像不会在日志未定时传出）；手动上传 / 恢复之后的基线 |
| core | `delta_sync::tests` | 假云端 + 假设备跑**同一份**生产算法：首次全传、空闲不再传；**只传改动的 key**；全新设备取回；**各改各的不冲突**；累计类冲突什么都不动；可合并的并集；设置后写为准；进度删除传播而日志丢了会补回；**大量删除先停下**；传输中输掉竞争重来；用户在取回期间动了数据整轮重来且不丢；**重来之前已落到本机的取回 / 合并 / 删除计数跨重来累加**（输掉竞争后仍提示「已更新」）；设备拒绝落盘时不记基线；**没写成的 key 个数跨重来只算一次（`skipped`）**；**自动写入里「被 hold 的影像」与「要取回而推迟的」两个计数累加而不是互相覆盖**；**基线在每次落盘本机之后、每一批上传之后都交出去（`checkpoint`），不等整轮结束**；服务端改写的内容传一次就稳定；只上传模式从不写本机；**40 个随机种子、3 台设备任意改 / 删 / 同步后收敛且稳定后不再有动作** |
| server | `account::store::tests` | 唯一性、会话过期与续期、淘汰、级联删除、**条件写入**、**规范化后超限的整份上传被拒且旧内容不动**、重新打开库后数据仍在、新版库拒绝启动 |
| server | `account::relational::tests` / `tables_tests` | 规范值与不是 JSON / 没有专表的内容**逐字节往返**；**老版本 / 手写的合法内容正确进表并规范化**（旧字段名、缺字段、未知字段、缺 padding 的 base64、`image/jpg`）、**规范化幂等**、**上传时算的指纹 == 取回内容的指纹**；解析不了 / 违反行级约束的内容原样保留且不拦住别的 key；**预算拆分**（`from_map_within`，按规范化后的字节数算）；影像超上限不进影像表；每张专表一条「规范值进表、取回一致」（一次全放 + 单独放）；**每个枚举列的 `CHECK` 覆盖枚举的每一个变体**（加变体时第一个红）；`f32` 放进 `REAL` 不走样；NEC 设计的顺序、id 与导线下标原样回来；错题暂存一个题库一族 key 互不干扰；错题本三条记录逐字段对得上（`pages` 有无、`NULL` 与空串、标准答案与我的作答不串）；一族 key 各占一行且互不干扰；`NULL` 与 `[]` 分得开；影像确实是 `BLOB`；父子表一起清、清空账号无残留；`[]` 与「没有这个 key」分得开；整体替换会删掉没了的 key 与行、没变的 key 不重写；账号间隔离、级联删除 |
| server | `account::relational::delta_tests` | 每个 key 独立核对「看到的是不是现在这一版」；**整批里有一个过期就一个都不写**；以为没有其实有 / 以为有其实没有；删除连专表的行一起清；key 在专表与目录之间来回；摘要是规范文本的、没变的 key 不重写；超限整批拒绝且**拒绝之前不写任何一行**；同一个 key 出现两次不收；**推送批按规范化后的字节边拆边数、超预算就停**；**原文超过展开上限的 key 不展开、原样存放、取回逐字节一致**；**通联日志只改一条只动几行**，中间删 / 插 / 换序后读回逐字节一致；整份写入与逐个增量写入得到同样的清单与指纹；取值只给指定的 key；账号间隔离 |
| server | `account::store::tests` | **旧结构（`user_version = 1` 而缺 `backup_meta` 的库）启动被拒绝并说清怎么办**、自己建的库重开不被误伤；**整次读取不会读到半个并发提交**（一个线程不停写、一个线程不停整份读，读到的内容重算出的指纹必须等于概况里的；去掉读事务它必红）；**一次大读取占着一个连接时别的读取照常完成**；已存在的松权限库被收紧 |
| server | `account::tests`（走完整路由） | 注册 / 登录 / 退出、**按 key 取值的超大请求体先鉴权再 `413`（未登录回 `401` 而不是 `413`）**、不区分账号是否存在、**只数失败的三层限流**（成功不耗额度、一个来源打满不影响本人从别处登录、分布式猜测被账号总额挡住、改口令共用额度）、`Secure`、跨账号隔离、`412`、`409`、改口令踢设备、注销、**人数上限**、**续期重发 Cookie**、过期会话、超大备份 `413`、空 / 超长 / 过多的 key 回 `400 invalid_backup`、未登录上传不读体、**配置写错会报错**、**规范备份经 HTTP 落进各表并完整取回、指纹与前端口径一致、「导出备份」的完整文件（含设备专属 key、QSL 影像、损坏值）导入后行数与内容都对、重复导入幂等、老版本形态的日志整个保留** |
| server | `main::tests` | 账号接口走 `no-store`、不开放跨域；挂上账号路由后 SPA fallback 照旧 |
| app | `account_sync::tests` / `account_cloud::tests` | 登录态判定（5xx / 429 / 断网保留已知状态，不掉成"未启用"）、错误码优先于状态码、哪些账号错误值得重试、只有 `412` 才算「云端已经变了」 |
| e2e | `account_sync.spec.ts` | 两套皮肤下的面板、登录 / 注册 / 退出、两个开关互斥、手动上传 / 恢复 / 合并；**自动写入只传改动的 key、`base` 等于上一次写进去的内容的摘要（摘要口径跨语言一致）**；**两台设备各改各的都成功**；同一项数据两边都改了停下来问、选「从云端恢复」后恢复；**本机一下子少了很多数据时停下、不把删除同步出去**；新设备取回与自动合并。`BrowserDevice` / `backup_apply` 依赖浏览器，只有这一层覆盖 |
| 部署 | `docker build` + 带卷的 `docker run` | 注册后删容器、用同一个卷重建，仍能登录、云端备份还在 |
