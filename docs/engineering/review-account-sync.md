# 在线账号改动审查：问题与整改追踪

> 写给**接手整改 / 复核这批改动的人**。对象是「在线账号与同步通道」那批改动
> （设计见 [`domain/account-sync.md`](../domain/account-sync.md)）；口径沿用
> [`code-quality.md`](code-quality.md) §1（证据 E1–E3、优先级 P0–P3、§1.3 记录模板）。
> **这是一份有生命周期的追踪表，不是规范**：全部条目关闭（✅ / ❎）后删除本文件，
> 结论已经沉淀在 `domain/account-sync.md`、`reference/api.md`、`engineering/deployment.md` 里
> （[`docs/README.md`](../README.md) 的文档约定第 2 条：规范与历史不混写）。

- 审查日期：2026-11-03；基线：暂存区 50 个文件（+6300 / −160 行），无未暂存改动。
- 方法：逐文件读码（E2）+ 跑门禁（E1）；整改后对关键行为做了变异验证（把实现改坏，对应测试必须变红，见「验证记录」）。
- 条目 ID 沿用 `code-quality.md` 的检查项编号；没有对应检查项的用范围名（`E2`＝备份与同步，`文档`）。
- 「位置」写审查时的行号（整改后已移位），用函数名定位更稳。

## 进度总览

| 优先级 | 总数 | ✅ 已修 | ❎ 不修 | ⬜ 未做 |
| --- | --- | --- | --- | --- |
| P0 | 1 | 1 | 0 | 0 |
| P1 | 8 | 8 | 0 | 0 |
| P2 | 6 | 6 | 0 | 0 |
| P3 | 4 | 3 | 1 | 0 |

| ID | 优先级 | 一句话 | 状态 |
| --- | --- | --- | --- |
| B1.1-1 | P0 | 工具链版本不一致，`cargo make clippy` 红 | ✅ |
| F1.5-1 | P1 | 登录名无长度上限就被拿去当限流 key；小 JSON 接口没有请求体上限 | ✅ |
| E2-1 | P1 | 会话"滑动续期"只续了库，浏览器 Cookie 仍按登录时的 `Max-Age` 过期 | ✅ |
| F1.5-2 | P1 | 配置解析"写错即放行"（`close` 被当成开放注册） | ✅ |
| F1.3-1 | P1 | Docker 默认开放注册，且账号数没有上限 | ✅（见遗留风险） |
| F1.5-3 | P1 | 按账号名限流且成败都计数，别人几次失败就能把本人锁在门外 | ✅ |
| F1.3-2 | P1 | 最大 64 MiB 的备份在异步线程上解析，且未登录也先读完请求体 | ✅ |
| A4.4-1 | P1 | 同通道重启时没复位 `PUSHING`，自动写入可能永久卡死 | ✅ |
| E2-5 | P1 | 账号面板新增的 `role="status"` 让 `backup.spec.ts` 里 7 条自动同步用例命中两个元素而红（**整改验证时才发现**） | ✅ |
| A4.4-2 | P2 | `refresh()` 把 5xx / 429 当"未启用"；并发刷新无序号保护 | ✅ |
| E2-2 | P2 | 账号通道的自动同步提示仍说"远端"，网络错误提示去查 CORS | ✅ |
| E2-3 | P2 | 文档承诺提醒"没有找回口令"，界面里没有 | ✅ |
| A3.1-2 | P2 | 唯一约束判断过宽；指纹解析失败静默变成 0 | ✅ |
| F2-1 | P2 | 库文件（口令哈希 + 全部备份）按默认 umask 创建 | ✅ |
| A3.5-1 | P2 | 用常量断言冒充"不缓存"的测试；多处关键路径无测试 | ✅ |
| A3.1-1 | P3 | `Precondition::from_headers` 返回 `Result<_, ()>` 并压掉 clippy | ✅ |
| A1.6-1 | P3 | `core` 里 `remote_sync` ↔ `sync_channel` 互相引用 | ✅ |
| 文档-1 | P3 | 为一个测试字面量把全角词加进 cspell 词典 | ✅ |
| E2-4 | P3 | 登录 / 注册共用一个表单，口令框 `autocomplete` 只能二选一 | ❎ |

---

## P0

```
范围/ID：   B1.1-1（对应 A1.7）
位置：      Cargo.toml:40（workspace.package.rust-version）· .tool-versions · README.md:53,136 · docs/engineering/architecture.md:9,52 · crates/core/src/filter_design.rs:63,65
优先级：    P0（门禁红）
现象：      `rust-version` 抬到 1.99.1 后，clippy 按新版本启用 `approx_constant`，`1.618_033_988_7` 被判成 `GOLDEN_RATIO`
            的近似值（deny 级）→ `cargo make clippy` / CI 红；`.tool-versions` 仍是 1.98.1，低于声明的 MSRV；
            README 与 architecture 仍写 1.88。
证据：      E1 `cargo clippy -p ham-web-core` 报 2 处 error；对比 `CLIPPY_CONF_DIR` 把 msrv 设成 1.89 时全部通过。
根因：      抬 MSRV 时只改了 `Cargo.toml` 与 Dockerfile 注释，没有对齐「四处一致」（B1.1）；clippy 的版本相关 lint 因此变严，
            而代码里手写的常量没跟着换。
修法：      保持 1.99.1（Dockerfile 与 CI 本来就是它）：`filter_design.rs` 两处改用 `std::f64::consts::GOLDEN_RATIO`
            （值相同，精度只会更高）；`.tool-versions` 改 1.99.1；README / architecture 的 1.88 全部改 1.99.1。
            不选"把 MSRV 降到 1.89"：用户明确要求 1.99.1，且 CI / 镜像已经在用。
回归锚点：  `cargo make clippy`（-D warnings）；`ham-web-core` 的滤波器单测（g 值表数值不变）。
状态：      ✅
```

## P1

```
范围/ID：   F1.5-1
位置：      crates/server/src/account/handlers.rs:274-282（login）· mod.rs（router）
优先级：    P1
现象：      登录请求的 `username` 没有长度检查就进了限流 map 的 key，也被拿去查库；路由只有 axum 默认 2 MiB 的体积上限。
            匿名请求可以把任意长的字符串塞进限流 map 与日志。
证据：      E2 读码。
根因：      长度校验排在限流之后；小 JSON 接口没有单独的请求体上限。
修法：      `login` 里先判 `name.len() > USERNAME_MAX` / 口令 > `PASSWORD_MAX` / 空名，直接当错误凭据（不查库、不跑 Argon2、
            不占限流桶）；登录 / 注册 / 改口令 / 注销的请求体上限 16 KiB（`AUTH_BODY_LIMIT`）。
回归锚点：  `account::tests::absurd_account_names_and_passwords_are_rejected_without_a_lookup`、
            `small_json_endpoints_refuse_huge_bodies`。
状态：      ✅
```

```
范围/ID：   E2-1
位置：      crates/server/src/account/store.rs:292-297（find_session_user）· session.rs:40-45 · mod.rs:14
优先级：    P1
现象：      文档说"会话有效期滑动续期"，但续期只更新库里的 `expires_at`，服务端从不重发 `Set-Cookie`：
            浏览器那份 Cookie 仍按登录那天起算的 `Max-Age` 过期，天天在用的人 30 天后照样被登出。
证据：      E2 读码 + 因果链：登录（Max-Age=30d）→ 第 29 天访问（库续到 59d）→ 第 31 天浏览器丢弃 Cookie → 401。
根因：      续期只发生在存储层，调用方不知道"刚续期"。
修法：      `find_session_user` 改为 `find_session`，返回 `SessionHit { user, renewed }`；`authenticate` 带出 `renewed`，
            会话 / 备份 / 改口令等所有已鉴权响应在 `renewed` 时重发同值的 `Set-Cookie`（`Authed::renewal_cookie`）。
            不选"改文案为固定有效期"：滑动续期才是想要的体验。
回归锚点：  `store::tests::sessions_expire_and_slide`（`renewed` 标志）、
            `account::tests::renewing_a_session_also_renews_the_browser_cookie`（把 `renewal_cookie` 改成恒 `None` 即红，已验证）。
状态：      ✅
```

```
范围/ID：   F1.5-2
位置：      crates/server/src/account/mod.rs:100-110（AccountConfig::from_env）
优先级：    P1
现象：      只有字面值 `closed` 才关闭注册：`ACCOUNT_REGISTRATION=close` / `false` / `off` 全部静默当成"开放"；
            `ACCOUNT_COOKIE_SECURE=TRUE`（大小写）静默回退 `auto`，可能让 Cookie 缺 `Secure`；
            `ACCOUNT_SESSION_DAYS=30d` 静默回退 30。运维以为关了、实际全网可注册。
证据：      E2 读码。
根因：      "解析失败就用默认"在安全开关上是失败即放行。
修法：      `from_env` 改返回 `Result`，值不分大小写解析；不认识的值让进程**启动失败**，错误里点名变量与可用的值。
            不选"只打警告"：警告没人看，而错误的方向是危险的那边。
回归锚点：  `account::tests::a_mistyped_value_stops_startup_instead_of_silently_opening_things_up`、
            `recognised_values_are_accepted_in_any_case`、`an_empty_environment_means_the_documented_defaults`。
状态：      ✅
```

```
范围/ID：   F1.3-1
位置：      Dockerfile（ENV ACCOUNT_DB）· crates/server/src/account/mod.rs · docs/domain/account-sync.md §8
优先级：    P1
现象：      镜像里 `ACCOUNT_DB` 已设好 → 已有的 Docker 部署升级后自动启用账号并对公网开放注册；账号数与总占盘没有上限；
            文档却写"既有部署升级后行为不变"。
证据：      E2 读码 + 文档对照。
根因：      "默认启用"的取舍没有配套的容量上限，文档只讲了非 Docker 的情形。
修法：      新增 `ACCOUNT_MAX_USERS`（默认 1000，`0` = 不限）；计数与插入在同一个写事务里（`create_user_capped`），
            满了回 `403 registration_closed`（不另造错误码）；文档如实写清 Docker 镜像的升级行为与关闭办法
            （`ACCOUNT_REGISTRATION=closed` 或 `-e ACCOUNT_DB=`）；`docker-compose.yml` 透传该变量。
            不选"镜像默认不启用"：账号功能是这批改动的卖点，且 e2e / 文档都按"镜像默认启用"写；改默认是产品决定，不在整改范围。
回归锚点：  `store::tests::the_user_count_is_capped_atomically`、`account::tests::registration_stops_when_the_account_cap_is_reached`。
状态：      ✅（残留风险见文末）
```

```
范围/ID：   F1.5-3
位置：      crates/server/src/account/handlers.rs:275-277（login）、325-330（change_password）、359-364（delete_account）
优先级：    P1
现象：      每次尝试（无论成败）都消耗该账号名 10 次/10 分钟的额度：任何人只要知道账号名，发 10 次错误口令就能让本人
            10 分钟登不上；旧测试甚至断言"正确口令也被拦"。
证据：      E3 旧测试 `too_many_wrong_guesses_for_one_account_are_throttled` 即该行为。
根因：      限流计"请求数"而不是"失败数"，且只有账号名一个维度。
修法：      只数失败，分两层（`RateLimiter` 新增 `is_blocked_by_key` / `record_by_key` / `clear_by_key`）：
            ①（账号名, 来源 IP）每 10 分钟 10 次失败，成功清零；②账号名（不分来源）每 10 分钟 100 次失败，成功不清。
            ②刻意定高：只在分布式猜口令时才碰到，单个来源攻击者先被①拦住，碰不到本人。
            来源 IP 由路由层 `tag_client_ip` 解析一次放进请求扩展。登录 / 改口令 / 注销共用同一份额度。
            不选"只按账号名、额度调大"：那样仍是随手可锁的开关，只是门槛高一点。
回归锚点：  `correct_logins_never_use_up_the_failure_budget`（把成功分支改成记失败即红，已验证）、
            `one_source_failing_does_not_lock_the_owner_out_from_elsewhere`、
            `guessing_from_many_sources_is_stopped_by_the_per_account_total`、
            `changing_the_password_shares_the_failure_budget_with_login`、`rate_limit::tests::*`。
状态：      ✅
```

```
范围/ID：   F1.3-2
位置：      crates/server/src/account/handlers.rs:405-429（put_backup）
优先级：    P1
现象：      模块文档说"重活都进 spawn_blocking"，但 `put_backup` 在 tokio worker 上对至多 64 MiB 的内容做 UTF-8 校验 + JSON 解析
            + 指纹；并发上传没有限制；`Bytes` 提取器在鉴权**之前**就把整个请求体读进内存（未登录也能让服务读 64 MiB）。
证据：      E2 读码。
根因：      提取器的顺序决定了"先读体、后鉴权"；解析没放进阻塞线程。
修法：      `put_backup` 改收 `Request`：先鉴权 → 校验头 → 声明长度超限直接 `413` → 拿上传名额（`UPLOAD_SLOTS`＝4）→ 才读体
            （`to_bytes` 带 `BODY_LIMIT`）→ 解析与指纹放进 `blocking`。备份路由不再挂 `DefaultBodyLimit`（读体自己带上限）。
回归锚点：  `account::tests::an_oversized_backup_is_refused_with_413`、`an_anonymous_upload_is_turned_away_before_its_body_is_read`。
状态：      ✅
```

```
范围/ID：   A4.4-1
位置：      crates/app/src/auto_sync.rs:652-681（activate）· 373-383（push_now）
优先级：    P1
现象：      账号通道已是当前通道时再次登录走"同通道 + restart"：`prev == next`，跳过了复位 `PUSHING` 的那段，只 `bump_generation()`。
            此刻若有在途写入，它回来发现过期直接 `return`、不复位 → `PUSHING` 永远为真，之后所有自动写入都被拦下，界面停在"同步中"。
证据：      E2 读码 + 因果链（写入在途 → 会话过期重新登录 → 旧写入返回 → PUSHING 卡真）。
根因：      复位逻辑只写在"通道变了"的分支里。
修法：      重启分支补 `PUSHING.set(false)` 与 `FAIL_STREAK.set(0)`，并写明为什么。
回归锚点：  无专门的自动化锚点（依赖浏览器定时器与网络，原生测试驱动不了）；靠读码确认，e2e `account_sync.spec.ts`
            的重新登录路径会间接走到。
状态：      ✅（修复已做，无专门回归测试）
```

```
范围/ID：   E2-5
位置：      e2e/tests/backup.spec.ts:532,544,560,569,635,694,709 · crates/app/src/pages/backup/remote_sync_panel.rs（根节点）· account_panel/panel_root.rs
优先级：    P1（关键路径 e2e 红；`cargo make e2e` 与 CI 都会红）
现象：      `/backup` 页加了「在线账号」面板后，页面上有两处 `role="status"`：账号面板的"此服务器没有启用在线账号…"与远端面板的自动同步状态行。
            旧用例用 `page.locator("section#backup").getByRole("status")` 取状态行，strict mode 命中 2 个元素 → 7 条自动同步用例失败。
证据：      E3 本地 `trunk serve` + `npx playwright test tests/backup.spec.ts`：整改前 7 failed / 25 passed；整改后 32 passed。
            （首轮审查只读码 + 跑了 `cargo make check`，后者不含 e2e，所以当时没看到。）
根因：      新增面板时按 AGENTS.md 要求"改了 role / aria-*，按断言里出现过这些属性全仓搜 spec，不要按页面名找"这一步没做完整——
            没有搜 `getByRole("status")`。
修法：      远端面板根节点加 `data-testid="remote-sync-panel"`（与 `account-panel` 对称），spec 里统一用 `remotePanel(page)`
            把状态行限定在远端面板内。不选"去掉账号面板的 `role=status`"：那条提示是给读屏用户的状态通知，该保留。
回归锚点：  `backup.spec.ts` 自动同步组（7 条）。
状态：      ✅
```

## P2

```
范围/ID：   A4.4-2
位置：      crates/app/src/account_sync.rs:179-205（refresh）
优先级：    P2
现象：      只有网络错误才保留已知登录态；429 / 502 / 504 等被当成 `Unavailable`，已登录的页面瞬间掉成"服务器未启用"并卸载改口令对话框。
            多个 `refresh` 并发时慢响应可能覆盖刚退出 / 刚登录的新状态。
证据：      E2 读码。
修法：      判定抽成纯函数 `verdict()`（429 / ≥500 / 断网 → `KeepKnown`）；`EPOCH` 序号：每次落定登录态加一，
            在途的 `refresh` 回来发现序号变了就丢弃自己的答案。
回归锚点：  `account_sync::tests::a_hiccup_never_changes_what_we_know` 等 5 条。
状态：      ✅
```

```
范围/ID：   E2-2
位置：      crates/app/src/auto_sync.rs:593-595 · remote_sync.rs（import_fetched / SyncError::message）
优先级：    P2
现象：      账号通道的自动快进成功 / 失败、"不是有效备份"仍用"远端"词条；网络错误提示"检查地址、确认 CORS"，对同源接口是误导；
            体积超限说"远端内容超过 64 MiB"。
修法：      新增 4 条三语词条（`account-auto-updated` / `account-not-a-backup` / `account-network-error` / `account-response-too-large`）；
            `import_fetched` 多收一个 key；`account_sync::describe` 统一账号通道的错误文案；`sync_io::fingerprint_of` 按通道分派。
回归锚点：  `cargo make i18n-check`（三语齐全、无死条目、key 字面量都存在）。
状态：      ✅
```

```
范围/ID：   E2-3
位置：      crates/app/src/pages/backup/account_panel/auth_form.rs · data/i18n/*/tools.json（account-rules-hint）· docs/domain/account-sync.md §7
优先级：    P2
现象：      文档写"忘记口令只能由部署方改库；请在界面上提醒用户保存好"，三语词条里没有任何这类提示。
修法：      在注册表单的口令提示 `tools.account-rules-hint` 末尾补一句"本站不发邮件，没有找回口令，请务必记牢"（zh / en / es）。
回归锚点：  `cargo make i18n-check`；e2e 里对该提示只按"账号名要 3–32 位"的错误文案断言，不受影响。
状态：      ✅
```

```
范围/ID：   A3.1-2
位置：      crates/server/src/account/store.rs:96-98（is_unique_violation）、331、387
优先级：    P2
现象：      `is_unique_violation` 匹配所有约束冲突（外键、非空也会被说成"账号名已被占用"）；指纹文本解析失败用 `unwrap_or(0)`
            静默变成 0——0 也是合法指纹，条件写入会在错误的基线上通过。
修法：      只认 `SQLITE_CONSTRAINT_UNIQUE`；指纹解析失败转成数据库错误（500），不再静默。
回归锚点：  `store::tests::a_damaged_fingerprint_is_an_error_not_a_silent_zero`、`usernames_are_unique_regardless_of_case`。
状态：      ✅
```

```
范围/ID：   F2-1
位置：      crates/server/src/account/store.rs:117-125（Store::open）
优先级：    P2
现象：      库文件与目录按默认 umask 创建（常见 0644），同机其他用户可读到口令哈希与全部用户的备份。
修法：      打开前先创建文件并设 `0600`（unix）；`-wal` / `-shm` 沿用主文件权限；已存在的库不改权限。
            不改目录权限：目录可能是用户自己的共享目录，改它越界。
回归锚点：  `store::tests::a_new_database_file_is_private_to_its_owner`（unix）。
状态：      ✅
```

```
范围/ID：   A3.5-1
位置：      crates/server/src/account/tests.rs:782-788（account_responses_are_not_cacheable_by_the_static_layer）
优先级：    P2
现象：      该测试只断言常量 `API_BASE` 以 `/api/` 开头，没有请求任何东西，却叫"不被静态层缓存"（空洞断言，`testing.md` 明令禁止）。
            另缺：IP 维度限流、会话过期、超大请求体、`failure()` / `retryable()` 的账号分支。
修法：      删掉空洞断言；在 `main.rs` 里走真实的 `app()` 断言 `/api/account/session` 带 `Cache-Control: no-store` 且不开放跨域，
            并断言挂上账号路由后 SPA fallback 照旧；补齐上面列的测试（server 新增 20 余条，app 新增 5 条）。
回归锚点：  `tests::account_api_is_never_cached_and_stays_same_origin`、`tests::mounting_accounts_does_not_break_the_spa_fallback`。
状态：      ✅
```

## P3

```
范围/ID：   A3.1-1
位置：      crates/core/src/account.rs:291-292（Precondition::from_headers）
优先级：    P3
现象：      返回 `Result<Self, ()>` 并用 `#[allow(clippy::result_unit_err)]` 压住告警；调用方可以不假思索地 `.ok()` 成"无条件覆盖"。
修法：      新增 `InvalidPrecondition` 单元结构体作错误类型，去掉 `allow`。
回归锚点：  `preconditions_are_parsed_strictly_from_headers`（断言 `Err(InvalidPrecondition)`）。
状态：      ✅
```

```
范围/ID：   A1.6-1
位置：      crates/core/src/remote_sync.rs ↔ crates/core/src/sync_channel.rs
优先级：    P3
现象：      两个模块互相 `use`（`remote_sync` 引三个 key 常量，`sync_channel` 引一批类型），依赖方向有环。
修法：      三个 key 常量定义下沉到 `remote_sync`（"哪些 key 不进备份"本来就是它的事），`sync_channel` 用 `pub use` 重新导出，
            调用方仍从 `sync_channel` 取，路径不变。
回归锚点：  `every_documented_key_is_classified`、`only_sync_bookkeeping_is_excluded_from_backups`。
状态：      ✅
```

```
范围/ID：   文档-1
位置：      cspell.json:4（"ａｂｃｄ"）· crates/core/src/account.rs（测试里的全角字面量）
优先级：    P3
现象：      为一个测试字面量把全角词加进项目词典（且没按序）。
修法：      测试里改写成 `\u{ff41}…` 转义，词典里删掉。
状态：      ✅
```

```
范围/ID：   E2-4
位置：      crates/app/src/pages/backup/account_panel/auth_form.rs:111
优先级：    P3
现象：      登录与注册共用一个表单，口令框 `autocomplete` 只能写一个值（现为 `current-password`），
            注册时浏览器的"建议强口令"不会出现。
状态：      ❎ 不修。理由：拆成两个表单（或按点击切换属性）会让"我到底注册过没有"的来回切换回到界面上，
            这正是当初合成一个表单要避免的（见 `auth_form.rs` 的组件文档）；影响只是少了一个浏览器的便利功能，不影响安全与正确性。
```

---

## 验证记录

| 项 | 命令 | 结果 |
| --- | --- | --- |
| 门禁（整批） | `cargo make check` | 通过，见「最终结果」 |
| e2e | `E2E_BASE_URL=http://127.0.0.1:3030 npx playwright test tests/account_sync.spec.ts tests/backup.spec.ts` | 通过，见「最终结果」 |
| 变异验证 | 把 `renewal_cookie` 改成恒 `None`；让成功登录也记失败 | 对应测试（`renewing_a_session_also_renews_the_browser_cookie`、`correct_logins_never_use_up_the_failure_budget`）变红，已还原 |

### 最终结果

| 项 | 结果 |
| --- | --- |
| `cargo make check`（fmt-check、clippy 原生 + 12 个 worker + wasm 前端、全部单测、i18n-pack / i18n-check / knowledge-i18n-check / explanations-check、e2e-check、check-ui、check-structure、readme-check、pixel-check、data-sync-check、data-staleness） | **通过**（376 s） |
| 单测数量 | server 135（含 23 条新增）· app 77（含 5 条新增）· core 1538 |
| e2e `account_sync.spec.ts`（dev 构建，像素风 + 现代风） | **18 / 18 通过** |
| e2e `backup.spec.ts` | 整改前 7 红（见 E2-5）→ **32 / 32 通过** |
| e2e `mobile_overflow.spec.ts` + `i18n_layout.spec.ts`（口令提示变长了，按 `AGENTS.md` 该跑） | 像素风、现代风（`E2E_UI_STYLE=modern`）各一档：**6 / 6 + 6 / 6 通过** |

**没有运行的项**（需要你本地补）：

- `docker build` 与"带卷的 `docker run`：注册后删容器、用同一个卷重建仍能登录"的冒烟（`account-sync.md` §9 的"部署"一行）；
- `cargo make ci`（release 前端构建 + `size-check` 体积预算）——新增 `rusqlite(bundled)` / `argon2` 只进服务端二进制，不进 wasm，理论上不影响前端体积，但没有实测；
- `E2E_ACCOUNT_LIVE=1`（同一批断言打真实后端，验证假服务与真服务端没有漂移）；
- `cargo make e2e`（release 口径的整批回归）——这次只跑了与账号 / 备份 / 布局相关的几份 spec，是 dev 构建口径。

## 遗留风险（已知、不在本次整改内）

- **`ACCOUNT_MAX_USERS` 不是占盘上限**：最坏仍是 人数 × 64 MiB。对公网开放的站点要么按磁盘预算调小，要么 `ACCOUNT_REGISTRATION=closed`。
  真正的总容量上限需要另行设计（按账号配额 / 总量检查），属于产品决定。
- **账号名维度的总失败额（100/10 分钟）仍可被分布式来源耗尽**，效果是该账号暂时登不上；这是"挡分布式猜口令"的必然代价，
  比"随手可锁"（原实现）低几个数量级，但不为零。
- **限流器的内存**仍是"条目达到 1 万时才回收过期项"：大量不同来源 × 不同账号名的失败记录在窗口内会占内存（每条很小）。
- **没有"找回口令"**：这是设计决定（不发邮件），整改只补了提示。
