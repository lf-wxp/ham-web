# i18n 架构重构

> 追踪文档。每完成一步勾选对应条目，并在「进度日志」里补一行日期与结论。
>
> 相关代码：`crates/app/src/i18n/`、`data/i18n/`、`crates/app/build.rs`、`crates/tools/src/i18n.rs`

## 1. 背景：为什么要重构

`crates/app/src/i18n.rs` 曾是一个 11,248 行 / 374 KB 的单文件，里面塞着两份完整词典。它已经到了
「改一行要滚五千行」的程度，并且工具链与这个文件的文本格式强耦合，动结构就得重写工具。

### 重构前实测数据

| 指标 | 数值 |
| --- | --- |
| `i18n.rs` | 11,248 行 / 374,473 字符 |
| EN / ES 词条 | 各 2,975 条（177,282 / 192,225 字符） |
| 分区注释 | 106 个，其中 4 个「批量补齐」块（第 4009 / 4103 / 5004 / 5382 行） |
| `t()` / `tf()` 字面量调用点 | 3,078 处（另有约 30~40 处动态 key） |
| 引用 `i18n` 的文件 | 341 个 |
| `dist/*.wasm` | 7.6 MB（EN+ES 全量编译在内） |

### 四个真实问题

1. **文件不可维护**：`add-i18n` 只会往数组尾部追加，已堆积 4 个「批量补齐」块，与前面的分区注释彻底脱节。
2. **中文原文作 key 的固有脆弱**：改一个中文字符 = 译文静默失效，并留下一条永远命不中的死条目。
   原工具只能查「用了没翻」，查不了「翻了没用」。
3. **词典与 wasm 绑死**：无法按语言/按域裁剪，词典增长直接等于包体增长。
4. **工具链与源码格式强耦合**：`crates/tools/src/i18n.rs` 靠正则解析 Rust 源，
   `slice_array` 依赖 `static EN: &[(&str, &str)] = &[` 与 `\n];`；其中约 100 行
   （`escape_rust` / `display_width` / `is_wide` / `render_entry` / `RUSTFMT_ENTRY_MAX_WIDTH`）
   纯粹是为了对齐 rustfmt 布局。

## 2. 目标架构

```
作者层（人写）                    机器层（校验/索引）              运行层（查表）
───────────────────────────────────────────────────────────────────────────────────────
data/i18n/{lang}/{domain}.json  ──►  check-i18n  ──►  build.rs 代码生成  ──►  t() / tf()
  ├ zh/{shell,exam,…}.json           重复 key            i18n/catalog.rs        （同步、响应式，
  ├ en/{shell,exam,…}.json           占位符一致          内嵌域静态表             API 一字未改）
  └ es/{shell,exam,…}.json           死条目                  ↓ 未命中
                                     按域覆盖率         i18n/pack.rs
                                     语言包新鲜度        运行时语言包（按语言 fetch）
                                          │
                                          └──► i18n-pack ──► public/data/i18n/{lang}.json
```

三层职责分开：**作者只关心自己那个域的 JSON**，机器保证全局一致，运行时只看到一张表。

- `crates/app/src/i18n/mod.rs` —— 运行时 API（`Locale` / `t` / `tf` / `bank_class`），不含任何词条；
- `crates/app/src/i18n/catalog.rs` —— 内嵌域的查表索引，`include!` 构建期生成的静态表；
- `crates/app/src/i18n/pack.rs` —— 运行时语言包：非内嵌域的译文，按语言 fetch 一次（见 P3-B）；
- `crates/app/build.rs` —— 把 `data/i18n/` 编成静态表，写进 `OUT_DIR`（不入库）。

### 与知识库 i18n 的关系

`data/knowledge-i18n/{lang}/{module}.json`（知识库正文）与 `data/i18n/{lang}/{domain}.json`
（界面文案）现在**同构**：都是按语言 / 模块（域）拆分的 JSON，都能逐步推进翻译覆盖率。
两者的交付方式不同：正文走 `postbuild` 合并进 `dist/`，界面文案走 `i18n-pack` 生成到
`public/`（开发与发布共用同一份，见 P3-B）。

## 3. 阶段与进度

### P0 · 目录化分层

- [x] `i18n.rs` → `i18n/` 目录：`mod.rs`（对外 API）+ `catalog.rs`（索引聚合）
- [x] 对外 API 一字未改：`t` / `tf` / `Locale` / `locale()` / `set_locale()` / `bank_class`
- [x] 341 个引用文件的 `use crate::i18n::…` 零改动

### P1 · 词典资产化（单一事实源 = JSON）

- [x] 词典从 Rust 字面量迁出到 `data/i18n/{lang}/{domain}.json`（一次性脚本迁移，
      迁移前后按「EN/ES key 集合与条数完全相同」断言校验）
- [x] `crates/app/build.rs` 构建期代码生成静态表（`OUT_DIR/i18n_catalog.rs`，按域文件名排序、
      域内按 key 排序，输出稳定可复现）
- [x] `t()` 运行时行为不变（同步、响应式、中文零查表）
- [x] `Trunk.toml` 的 `watch` 加入 `../../data/i18n`，改译文能触发重建

**为什么用「构建期代码生成」而不是「运行时 fetch」**：词典只占 7.6 MB wasm 的约 5%，
而运行时 fetch 会让非中文用户首屏先渲染中文再翻转（可见闪烁）。构建期生成既拿到了
「作者改 JSON、不用转义 / 不用对齐 rustfmt」的全部工作流收益，又保持零网络、零运行时
开销。实测重构前后 `dist/*.wasm` 持平（7.66 MB），符合预期。

这一取舍在 P3-B 里被收紧了一层：中文（默认语言）仍是全量内嵌，en / es 只保留内嵌域，
其余域改为运行时按语言拉一次 —— 但拉包发生在**挂载之前**，因此仍然零闪烁，
`t()` 也仍是同步查表（见 P3-B）。

### P2 · 工具链补齐

- [x] `check-i18n`：扫描 `data/i18n/` 资源目录，新增「死条目」检查与按域词条数统计
- [x] `add-i18n`：回写到 `data/i18n/{lang}/{domain}.json`；目标域由 **key 前缀**决定
      （`key.split('.')` 的第一段，见 `crates/tools/src/i18n.rs` 的 `add`）——
      与 `build.rs` 的域划分同源，因此不硬编码页面→域映射表
- [x] 移除为对齐 rustfmt 而存在的 ~100 行格式适配代码（`escape_rust` / `display_width` /
      `is_wide` / `render_entry` / `RUSTFMT_ENTRY_MAX_WIDTH`）
- [x] 清理死条目 25 条（源码里已无该中文原文，永远命中不了）

### P3-A · 语义化 key

- [x] 2968 条 ×2 语言的「中文原文 → 译文」翻转为「语义 key → 文案」：key 形如
      `<domain>.<slug>`，slug 由英文译文派生；源码里不再有中文原文字面量
- [x] 中文降级为「一种译文」：新增 `data/i18n/zh/{domain}.json`，与 en / es 同构
      （每语言 2979 条，含 15 条语言无关的格式 / 单位串）
- [x] 中文反向索引：`crates/core` registry 里导航分组名、页面标题这类**运行时**传入
      `t()` 的中文，继续按原文命中词条 —— 不必为了迁移一次性改完 `crates/core`
- [x] codemod 344 个文件 / 3077 处调用点；`check-i18n` 覆盖率 2418/2418（100%）

### P3-B · 按需加载（分包）

- [x] `build.rs` 只把 **zh 全量 + en/es 的 `common` / `shell` 域**编进 wasm；其余域的译文
      由 `i18n/pack.rs` 在运行时按语言 fetch 一次
- [x] `ham-web-tools i18n-pack` 生成 `public/data/i18n/{lang}.json`，由 `index.html` 的
      `copy-dir` 进 `dist/`（`trunk serve` 与 `trunk build` 共用，开发不缺包）
- [x] 无闪烁：包在**挂载之前**（`main.rs`）与**切语言之前**（`set_locale`）载入，
      `t()` 仍是同步查表，调用点一字未改
- [x] 版本信号复用 `sw.js`：包进预缓存清单，内容一变 → 清单哈希变 → `__CACHE_VERSION__`
      变 → 离线缓存自动刷新，不需要额外的 `?v=`
- [x] `check-i18n` 增加语言包新鲜度校验（改了译文忘跑 `i18n-pack` 会红）；
      `add-i18n` 与 `build-web` 自动刷新

### P3-C · 复数规则（`tp()`）

`tf` 只有位置占位符 `{}`，一个 key 只对应一条译文 —— 英文 / 西班牙文下 `n = 1` 会输出
`1 days` / `1 questions` 这类语法错误。实测：`data/i18n/zh/` 含占位符词条 348 条，
其中计数型候选 144 条，`tf(` 调用点约 388 处。中文不受影响（`{} 天` 对 0 / 1 / 2 都成立），
因此这是**只影响 en / es 的正确性问题**，不牵动全站。

**取舍**：不上 ICU MessageFormat / Fluent —— 它们要引入运行时解析器（约 +100 KB wasm）
并把全部 2,979 条改写成新语法，会破坏 P3-B「`t()` 同步查表、包在挂载前载入 → 零闪烁」
这条核心保证，也让作者从「填 JSON 文案」退化成「手写控制结构」。本项目只有 zh / en / es
三种语言、实际只用 `one` / `other` 两类，自建内核更可控。

#### 设计

词条值允许两种形态，一个 key 仍是一条文案（死条目 / 覆盖率 / 命名空间 / 占位符四项现有
校验语义不变）：

```jsonc
// data/i18n/zh/common.json —— 中文不写复数，写了会在构建期直接报错
"common.days": "{} 天"

// data/i18n/en/common.json
"common.days": { "one": "{} day", "other": "{} days" }
```

- 变体名限定 CLDR 六类 `zero` / `one` / `two` / `few` / `many` / `other`，**`other` 必填**；
- `build.rs` 生成**双表**：扁平表 `EN` + 只含复数词条的 `EN_PLURAL`（约 100~150 条，
  且只有落在内嵌域的才进 wasm，体积增量 < 5 KB）；
- 运行时语言包升级为 `{ "flat": {…}, "plural": {…} }`；
- 查表链：`PLURAL[lang][key][category]` → `PLURAL[lang][key].other` → 现有 `t()` 链
  （内嵌扁平 → 语言包 → 中文）。复数词条被老调用点 `t(key)` 查到时返回 `other` 变体，
  不会漏出 key；
- `tp()` 在中文下直接退化为 `tf()`：中文全量扁平内嵌，**零复数开销**。

```rust
// crates/app/src/i18n/mod.rs
pub fn tp(key: &str, count: impl Into<PluralCount>, args: &[&str]) -> String;
```

`count` **单独传、不占用 args**：决定复数的量未必是第一个占位符（如
`common.questions-left-unseen-in` = `{} 类还有 {} 题没做过`，决定复数的是第 2 个占位符）。

规则内核进 `crates/core/src/plural.rs`（不依赖浏览器，配单测）：

| lang | 类别 | 规则 |
| --- | --- | --- |
| `zh` | `Other` | 恒定（`tp` 在中文侧根本不查复数表） |
| `en` | `One` / `Other` | `One`：整数部分为 1 且无小数位 |
| `es` | `One` / `Other` | `One`：整数 1 |

`zero` / `two` / `few` / `many` 在**存储与查表通路里已预留**（词条可写、能命中），
只是当前三种语言的规则函数不产出 —— 将来加俄语 / 阿拉伯语只改这张表 + 加一个
`data/i18n/<lang>/` 目录。

#### 分期

- [x] **P3-C1 内核**：`crates/core/src/plural.rs`（CLDR 类别 + 单测）、`i18n/mod.rs` 的
      `tp()` 与共用 `substitute`、`build.rs` 双表、`catalog.rs` 复数索引、`pack.rs` 的
      `Pack { flat, plural }`、`i18n-pack` 产物结构。对**没有复数变体**的词条 `tp()` 与
      `tf()` 完全等价，因此未迁移的调用点零行为变化
      - 随内核落地顺带把 `common.days`（4 处调用）作为**样条**迁成复数对象：既是端到端
        验证（内嵌域 → 内嵌复数表 → `tp`），也让 `tp` 不至于是死代码。其余词条仍在 C3
      - 单测：`core/plural.rs` 8 条、`app/i18n` 3 条（`tp` 按数量选 `1 day` / `2 days`、
        无复数变体时 `tp` ≡ `tf`、`t()` 回退 `other`）
- [x] **P3-C2 工具链**：`check-i18n` 复数校验（变体名合法 / 必含 `other` / 各变体占位符
      与中文一致 / zh 不得用复数对象）、`collect_call_sites` 认 `tp(`（否则 `tp` 调用的
      key 会被判成死条目）、`add-i18n` 支持对象值、`check-i18n --plural-candidates`
      导出候选与回填骨架
      - `scan_calls` 一次扫出 `(偏移, 函数名, key)`：`tp(` 与 `t(` / `tf(` 同等算「在用」，
        偏移换来 `文件:行号`，候选报告据此人工确认 count 对应哪个占位符
      - `check-i18n --plural-candidates <path> --lang en|es`（`cargo make i18n-plural-candidates`
        默认导 en；导 es 直接跑 `cargo run -q -p ham-web-tools -- check-i18n
        --plural-candidates tmp/plural-candidates-es.json --lang es`）：
        导出 `{key, zh, current, text:{one,other}, calls, uses_tp}`，
        `text` 就是回填骨架 —— `other` 沿用现译文、`one` 留空，填好直接 `add-i18n`
      - 对账（只提示、不判失败，迁移是增量的）：`tp()` 调用但词典没配变体 / 词典配了变体
        但源码没有 `tp()` 调用，两条都会在 `check-i18n` 里列出来
      - 顺手修掉两处工具链噪声：扫源码前把 `#[cfg(test)] mod …` 涂白（保留换行，行号不变），
        否则单测里构造的 `tp("shell.home", 1, &[])` 会让对账误报；`add-i18n` 改为
        **保序 + 只写内容变了的域文件**，一次回填不再带出上千行键重排 diff
- [x] **P3-C3 迁移**：先迁 `common` 域（内嵌域，首屏可见、收益最大，约 40~60 条），
      再迁 `learning` / `exam` / `log`；源码侧按候选白名单把 `tf(` 改成 `tp(` 并**人工确认
      count 对应哪个占位符** —— 这正是不能全自动 codemod 的原因
      - [x] `common` 域已迁 **35 条**（候选 111 条里挑的，其余是百分比 / 单位 / 日期 /
            纯数值这类英文不变形的文案）；调用点 38 处 `tf(` → `tp(`，跨 18 个文件
      - [x] `exam` / `learning` / `log` 已迁 **27 条**（候选 23 / 31 / 34 条里挑的：
            exam 10、learning 7、log 10）；调用点 27 处 `tf(` → `tp(`，跨 16 个文件；
            候选 308 → 281 条
      - [x] 第三批（`tools` / `radio` / 其余域）已迁 **27 条**：radio 7、common 3、
            log 4、learning 4、contest / home 各 2、knowledge / settings 各 2、tools 2；
            调用点 22 处 `tf(` → `tp(`，跨 22 个文件；候选 277 → 250 条
            - 其中 **B 类（多数量共用一个名词）** 一并纳入：`common.correct-2`、
              `common.wrong`、`log.records-2`、`log.grid-qsos-confirmed`、
              `learning.class-mistakes-total` 等 —— `count` 传给决定词形的那个数，
              en 无变化时 `one` / `other` 同形，西语侧才真正分叉
      - [x] 第四批（**静态扫描盲区**）已迁 **3 条**：`settings.merged-items`
            （`Merged {} items`）、`tools.restored-records`（`Restored {} records`）、
            `common.questions-2`（`{} “{}” ({} questions)`）
            - 这三条在源码里是 `tf("已合并 {} 条数据")` —— 走「中文原文 → key」的
              反向索引，`--plural-candidates` 只认**语义 key 调用点**，因此一条都没进候选表。
              是拿「词典侧反查」补出来的：直接扫 en / es 词典里 `{}` 后紧跟复数名词的
              普通文案，再回头找调用点
            - `settings.merged-items` / `tools.restored-records` 顺手把调用点从中文原文
              改回语义 key（反向索引是给 `crates/core` registry 兜底的，业务代码不该依赖它）
            - 同一处还揪出 `mistake_diagnosis.rs` 的**中文硬编码**：诊断结论是
              `format!("{}「{}」（{} 道）", …)` 拼出来的，`{}` 里填的是中文标点，
              整句在 en / es 下原样显示中文。已改成 `tf("common.questions-2", …)` /
              `tf("common.correct", …)`，连标点都按 `locale()` 切换（zh `；。` / 其它 `; .`）
      - **筛掉的三类**（不是漏迁，是 `tp` 管不了 / 管了没收益）：
        1. 英文不变形：百分比（`{}%`）、单位（`{} min` / `{} km`）、序号
           （`Question {} / {}`）、纯数值（en 文案就一个 `{}`，如 `log.entry`）；
        2. **count 不是数字**：`learning.show-only-mistakes` 的 `{}` 其实是专题名
           （`只看「{}」的错题`），`log.time-notes` 的 `{}` 是备注文本；
        3. **一句话里有两个以上可数名词**：`log.labels-qsos-pages`、
           `log.imported-records-skipped-duplicates` 等。`tp` 只吃一个 `count`，
           改一半会比不改更难读 —— 已在 **P3-C5** 用嵌套 / 改写解决（不引入新机制）。
           ⚠️ 当初列在这里的 `learning.class-mistakes-total` / `log.grid-qsos-confirmed`
           后来确认是 B 类（名词只有一个），已在第三批迁掉
      - ⚠️ 上表第 3 点当时把「多个数量**共用一个**名词」也一并排除了（`common.correct-2`
        等），那是**误伤**：名词只有一个时 `tp` 完全够用。定量后已在 P3-C5 纠正，
        后续批次应把这类纳入迁移
      - 迁移时揪出 **4 条既有 en / es 参数顺序 bug**：`common.days-until-the-class`、
        `-2`、`questions-left-unseen-in`、`of-the-last-attempts` / `only-of-the-last`
        的译文按英文语序写（数量在前），而调用点按中文语序传参（类别在前），
        线上实际渲染成 `A days until the Class 3 exam`。占位符是按位置替换的，
        改译文语序即修复（改调用点会反过来弄坏中文）
- [x] **P3-C4 验收**：`e2e/tests/i18n_plural.spec.ts` —— `addInitScript` 把 locale 切成
      en / es 并种入「日志 1 条通联 + 错题本 1 条已到期错题」，扫 8 个路由的全部可见文案
      与 `title` / `aria-label`，断言**不出现 `1 <复数名词>`**；反向再断言至少命中一次
      `1 <单数名词>`（否则说明种子没生效、断言等于没跑）。旅程已进
      `e2e/coverage-targets.json`（导航与全局，en / es 各一条）
      - 首跑即抓到 3 处真 bug：`radio.grids-worked`（`1 grids worked`）当场迁成复数；
        `learning.challenge-streak-days-best` 与 `log.labels-qsos-pages` 是多计数文案，
        `tp` 只吃一个 count，暂列用例内 `KNOWN_MULTI_COUNT` 白名单并注明原因
      - P3-C5 实施后白名单已**摘除**（两条分别按 L3 / L2 修掉），复跑又抓出
        `home.day-streak`（es `Racha de {} días`）并顺手迁复数；现 en / es × 8 路由
        全绿、0 findings
      - `ROUTES` 已从 8 个扩到 **33 个**（补上 `stats` / `dxcc-map` / `zone-map` /
        `print` / `notifications` / `contest` / `cheat-sheet` / `morse` 等），名词表同步补了
        `entities` / `passes` / `months` / `points` / `stages` / `countdowns` / `callsigns`
        等；实测各路由确实渲染出了 count = 1 的文案（`/stats` 的 `339 entities to reach 100`、
        `/dxcc-map` 的 `Worked 1 / 340 entities`、`/qsl-labels` 的 `1 label · 1 QSO · 1 page`）
      - 仍不覆盖需要交互才出现的文案（答完 1 题、导入 1 条记录后的提示），种子造不出来；
        这类只能靠词典侧反查（见 P3-C3 第四批）兜住

#### 风险与对策

| 风险 | 对策 |
| --- | --- |
| 复数变体漏翻（`one` 有、`other` 没有） | `check-i18n` 强制 `other` 必填；查表链回退 `other` → `t()` → 中文，最差也只是回到今天的行为 |
| `count` 传错占位符 | `tp` 的 count 与 args 解耦，迁移靠 `--plural-candidates` 报告人工确认；e2e 用 `count = 1` 场景钉住 |
| 死条目误判 | `scan_calls` 认 `tp(`（单测模块先涂白，测试里构造的调用不干扰）；复数词条仍是**一个** key，不新增死条目判定分支 |
| 语言包结构变化被旧 SW 缓存住 | 与 P3-B 同款机制：包在 `sw.js` 预缓存清单里，内容变则 `__CACHE_VERSION__` 变 |
| 将来加语言时规则表不准 | 规则集中在 `core/plural.rs` 一张表 + 单测；`zero/two/few/many` 通路已预留 |

### P3-C5 · 多计数文案（已实施）

`tp(key, count, args)` 只吃一个 `count`，一句话里塞了多个「数量 + 可数名词」就管不了
（如 `log.labels-qsos-pages` = `{} labels · {} QSOs · {} pages`：count 给谁？）。
C3 期间先把这类条目挂起，这里定量后定方案。

**规模实测**（`tmp/plural-candidates-en.json` 281 条候选，按「`{}` ≥ 2 且可数名词 ≥ 2」筛）：

| 类别 | 条数 | 说明 |
| --- | --- | --- |
| 真·多名词 | **4** | `learning.challenge-streak-days-best`、`learning.correct-in-a-row`、`log.imported-records-skipped-duplicates`、`log.labels-qsos-pages` |
| 常量（恒 > 1） | 2 | `learning.questions-a-day-in` / `-minute-limit-counts`（`CHALLENGE_COUNT` / `CHALLENGE_MINUTES` 编译期常量，永远走 `other`） |
| 单位串误报 | 4 | `tools.*` 的 `{} MB` / `{} MHz` / `{} km`，英文缩写不变形，本就不该迁 |

**结论：为 4 条文案引入机制不划算 —— 不引入。**

**纠正一处过度保守**：另一类「多个数量共用一个名词」（`common.correct-2` = en `{} / {} correct` /
es `{} / {} correctos`）**不是障碍**。名词只有一个，把 `count` 传给决定词形的那个数（通常是
总数 / 后一个数）即可，现有 `tp` 直接覆盖。西语侧这类是真会变形的（`1 / 1 correcto`），
前两批把它们一并排除了，属于误伤，后续批次应纳入。

**方案（三层，按优先级）**

| 层级 | 做法 | 适用 | 成本 |
| --- | --- | --- | --- |
| L1 写作规范 | 一条文案只放一个「数量 + 可数名词」，并列句拆开写 | 所有新增文案（源头治理） | 0，靠评审与下面的检查 |
| L2 嵌套格式化 | 外层模板只留 `{}` 承接，内层片段各自 `tp`：<br>`tp("log.imported-records-skipped-duplicates", n, &[&tp("log.n-records", added, …), &tp("log.n-duplicados", dupes, …)])` | 语序一致、可拆的并列句 | 0 机制改动（内层片段**自己**也是带 `{}` 的模板，但它在作为实参传进外层之前就已由自己的 `tp` 填完，外层 `substitute` 因此不会误伤；占位符校验按 `{}` 计数，内外层各自成立）；代价是 key 变多、文案要拆 |
| L3 改写 | 只留一个可数名词，其余改成无量词表达（如 `best: 7`、`最长：7`） | 拆不动的短句 | 0 机制改动；要改文案，需译文侧确认 |

**已否决的方案**

| 方案 | 否决理由 |
| --- | --- |
| `tp` 接多个 count / 组合变体（`{"one_other": …}`） | 类别组合按 2ⁿ 爆炸（3 个名词 = 8 组变体），存储、翻译、校验三方成本都失控 |
| ICU MessageFormat / Fluent | 与 P3-C 定案时的判断一致：引入运行时解析器约 +100 KB wasm，为 4 条文案不值 |

**落地步骤**

1. [x] `log.labels-qsos-pages` → L2：拆出 `log.n-labels` / `log.n-qsos` / `log.n-pages`
      三个片段 key，外层模板退成 `{} · {} · {}`；调用点
      `qsl_labels_page.rs` 用 `tf` 套三个 `tp`
2. [x] `log.imported-records-skipped-duplicates` → L2：拆出 `log.imported-n-records` /
   `log.skipped-n-duplicates`，外层模板退成 `{}，{}`；调用点 `log_page.rs`
   - 复审修正（2026-10-06）：内层片段改为**直接复用** `log.imported-records`，删掉同义的
     `log.imported-n-records`。中文释义必须唯一 —— `catalog::zh_reverse()` 是「中文原文 →
     key」的**单值**映射，两条 key 共用同一句中文时后写的会覆盖先写的，另一条静默查不到；
     这里两条译文恰好逐字相同才没露症状，机制上却是隐患。`check-i18n` 现已加「中文释义
     唯一」硬校验
3. [x] `learning.challenge-streak-days-best` → L3：改写为
      `Challenge streak: {} day · best: {}` / `Racha: {} día · mejor: {}`（只留一个可数名词）；
      调用点 `daily_challenge.rs` 两处，`tp` 传 `cur_streak`
4. [x] `learning.correct-in-a-row` → B 类：名词只有 `days` 一个，`tp` 传第三个 count
      （`mistakes_page.rs`，`u32::from(next_days)`）
5. [x] 摘掉 `e2e/tests/i18n_plural.spec.ts` 里的 `KNOWN_MULTI_COUNT` 白名单 —— 复跑
      en / es 各 8 路由全绿（0 findings）
6. [x] （可选增强）`check-i18n` 增加两句守门检查，把 L1 与「反向索引盲区」都钉住：
      - [x] 「一句话多个可数名词」检查：文案里出现 ≥ 2 处「`{}` 后紧跟可数名词」就报错
        （`ok = false`，存量已清零，因此能当硬门禁）：
        - 词表**自维护**：从本语言已迁的复数变体里取每个变体的末词（`{} days` → `days`），
          迁一条多一词；`QSO` 这类还没迁复数词条的用一份兜底名单（即 e2e
          `i18n_plural.spec.ts` 盯的那批 25 个名词 ×2 语言）
        - 滤噪关键：**只有单复数两形都出现过的词才算可数名词** —— `correct` /
          `answered` / `minute` 这类没有复数形式的词被滤掉，否则
          `{} · {}% correct / {} answered` 会天天误报
        - 只数「`{}` 后紧邻名词」（与 e2e 盯 `1 <复数名词>` 同口径），
          `{} new questions` 这种隔了修饰语的会漏 —— 守门宁可漏报不误报，
          漏了的由 e2e 的 count = 1 扫描兜住
        - 实测：zh / en / es 三语 **0 命中**；把 `log.labels-qsos-pages` 临时改回历史文案
          `{} labels · {} QSOs · {} pages` 做探针，准确报出「3 处」并判失败
      - [x] `--plural-candidates` 把**中文原文被调用**也算作调用点（原来只认语义 key，
        所以第四批那三条一直在盲区里）：`ZhIndex`（中文原文 → key）在
        `collect_call_sites` / `collect_plural_call_sites` / `call_locations`
        三处统一解析字面量，方向就是 `unused()` 已有的「源码字面量包含中文」
        逻辑反过来 —— 先精确匹配整条原文，再对含 CJK 的字面量按**最长**原文做包含
        匹配（`format!` 把原文拼进更长句子的情况）；只有带 `{}` 的原文参与包含匹配，
        否则「条」「次」这类短原文会命中一切
            - 噪声：本次补扫**没有新增候选**（251 条不变）—— 第四批已把业务代码里的
              中文原文调用点都改回语义 key，反向索引现在只剩给 `crates/core`
              registry 兜底用；新逻辑是守门，不是补量
            - 验证：临时探针 `tf("7–20 天 {} 道", …)` 被解析成 `learning.7-20-days`
              并给出 `crates/app/src/tmp_probe.rs:2` 调用点；单测
              `resolves_chinese_literal_calls_to_semantic_keys` 钉住精确 / 包含 /
              无占位符三类行为；`cargo make check` 全绿（覆盖率 100%、无死条目）

**实测**：4 条改完 + 白名单摘除后，e2e 复跑又抓出 `home.day-streak`（es `Racha de {} días`），
说明第二、三批把「数量 + 名词」漏在候选之外的情况确实存在，靠 C4 的纯文本扫描兜住。该条已顺手
迁为复数（`one`: `Racha de {} día` / en 两形同形）。候选 280 → 277 条。

### P3 · 明确不做（或延后）

- [ ] **性别（性数一致）**：西语的性数一致作用在**修饰语**上（`1 pregunta nueva` /
      `2 preguntas nuevas`），取决于名词的语法性别而非数字，需要给词条挂 `gender` 元数据；
      而本项目文案几乎都是「N + 名词」的短串，没有真实痛点。复数部分（`pregunta` /
      `preguntas`）已由 P3-C 的 `one` / `other` 覆盖。**触发条件**：出现「N 个 新的 /
      已完成的 / 可用的 + 名词」这类带形容词的文案时再单独立项，用 `tg(key, gender)` +
      词条级 `gender` 元数据解决，**不并入复数系统**
- [ ] **零值文案**：`共 0 题` 与 `暂无题目` 是措辞差异而非复数，用独立 key 解决，
      不进复数系统
- [ ] **复数 / 性别规则中的小数精确位数 `v`**：当前三种语言只需区分「有无小数部分」，
      `PluralCount` 因此只存布尔；接入需要精确 `v` 的语言（如 `ar`）时再改成 `u32`

## 4. 域划分

沿用原文件里 106 个分区注释的语义，收敛为 12 个域。分区注释能定域的直接定域；
「批量补齐 / 渲染点补齐 / 知识页补充」这类兜底块按「调用点所在文件」路由到域；
跨 ≥3 个域共用的条目（共享组件文案）归入 `common`。

| 域 | 词条数（zh+en+es） | 内嵌进 wasm | 覆盖内容 |
| --- | --- | --- | --- |
| `tools` | 1,554 | — | 小工具目录与各工具组件（最大域） |
| `radio` | 1,371 | — | 传播、卫星、太阳、中继、DXCC / SOTA 地图、SDR、追台 |
| `knowledge` | 1,065 | — | 知识页通用小标题、表头、分类说明 |
| `common` | 975 | ✓ | 错误提示、计算结果模板、共享组件、快捷键帮助 |
| `exam` | 948 | — | 练习、模拟考试、题目卡片、错题、收藏、浏览、闪卡、辨析 |
| `log` | 816 | — | 通联日志、网格地图、统计、奖状、QSL、Cabrillo |
| `learning` | 630 | — | 学习进度、周报、热力图、备考计划、成就、每日挑战 |
| `shell` | 531 | ✓ | 顶部导航、菜单分组、页脚、全站搜索面板、主题 / 语言、页面标题 |
| `morse` | 405 | — | 莫尔斯电码、听题模式、字母解释法 |
| `contest` | 372 | — | 竞赛、竞赛日志、竞赛日历 |
| `settings` | 153 | — | 通知中心、推送、备份、提醒 |
| `home` | 117 | — | 首页与首页卡片 |

`common` / `shell` 之外的域由运行时语言包提供（见 P3-B）；中文不受影响 —— zh 全量内嵌。

## 5. 现在的工作流

源码里写的是语义 key（`t("shell.home")`），中文与其它语言一样只是 `data/i18n/` 里的一份
译文。**改文案 = 改 JSON；新增文案 = 加一个 key 并补各语言译文。**

**新增一句文案**：照常写 `t("域.词条")`，然后导出待译模板并回填

```bash
cargo make i18n-check          # 会报出「待补：域.词条」——写错的 key 在这里现形
cargo run -q -p ham-web-tools -- check-i18n --missing tmp/ui-missing.json --lang en
# 填写 tmp/ui-missing.json 里的 text 字段（key 与 zh 已填好，zh 仅作参考）
cargo run -q -p ham-web-tools -- add-i18n tmp/ui-missing.json --lang en
```

`add-i18n` 按 key 的域前缀定位到 `data/i18n/{lang}/{domain}.json`（并用 `zh` 目录里的中文
原文作参考），回填完会自动刷新运行时语言包。es 侧把 `--lang` 换成 `es` 再跑一遍即可。

**改一句已有文案**：直接改 `data/i18n/{lang}/{domain}.json`（语义 key 保证改中文不会让
别的语言失效），然后跑一次 `cargo make i18n-pack` 刷新语言包；`i18n-check` 会校验产物
是否与 `data/i18n/` 一致。

**新增一个页面**：key 前缀用页面所属域，通常只触碰 1~2 个 JSON。

**删文案**：删掉源码里的 `t()` 调用与该 key 的各语言条目；`i18n-check` 会把「词典里有、
没人用」的条目列为死条目提示清理。

## 6. 验收指标

- [x] `i18n/mod.rs` 只剩 API 与运行时逻辑，不再含任何词条
- [x] 词条唯一事实源为 `data/i18n/**`，最大的域文件 `tools.json` 约 518 条
- [x] `cargo make i18n-check` 全绿：zh / en / es 各 2,985 条，无重复、命名空间一致、
      占位符一致、覆盖率 100%、无死条目、无多计数文案、语言包最新
- [x] `cargo make check`（fmt / clippy / test / i18n-check / knowledge-i18n-check /
      explanations-check）全绿
- [x] `cargo make build-web` 构建通过；`dist/*.wasm` 7.53 MB（P3-B 把 en / es 的非内嵌域
      移出后比 P3-A 前的 7.66 MB 少 131 KB），代价是非中文首次访问多一次约 50 KB（br）的
      语言包请求
- [x] `tp()` 可用且 `t` / `tf` / `Locale` / `bank_class` API 一字未改，2,900+ 处 `t()` 调用点
      零改动（P3-C1）
- [x] `core/plural.rs` 的复数规则有单测钉住（0 / 1 / 2 / 21 / 1.5 / 负数 / 极值 / 未知语言）
- [x] 计数型词条迁移完成后，en / es 下 `count = 1` 的页面不再出现 `1 days` / `1 questions`
      类语法错误（P3-C4 的 e2e 断言，en / es 各一条，已加入覆盖目标清单并通过）
- [x] 复数落地的体积增量可控（全部批次迁完后复查：wasm 累计 **+15.6 KB**、en.json
      **+5.2 KB**，相对 7.55 MB 的 wasm 可忽略）。C1 + common + 第二批 + C4 + C5 + 第三批实测：wasm
      7,531,630 → 7,539,093 → 7,545,460 → 7,545,964 → 7,546,187 → 7,546,586 →
      7,546,965 → 7,547,249 B（**共 +15.6 KB**；C1 的 +7.3 KB 是 `tp` 查表链代码，与词条数无关；
      common 域 35 条 +6.4 KB；exam / learning / log 27 条只 **+504 B**；C4 补的
      `radio.grids-worked` +223 B；C5 四条 +396 B、`home.day-streak` +3 B；第三批 27 条
      **+379 B** —— 除 `common` 外都不是内嵌域，变体只进运行时包，wasm 里增长的只是
      调用点代码）；
      语言包 `en.json` 179,989 → 180,013 → 182,400 → 183,495 → 183,528 → 183,953 →
      185,059 → 185,173 B（C1 的 +24 B 是 `{"flat":…,"plural":…}` 一层包装，common 域 35 条
      +2.4 KB，第二批 27 条 +1.1 KB，C5 四条 +393 B、`home.day-streak` +32 B，
      第三批 27 条 +1.1 KB、第四批 3 条 +114 B）。
      复数表只收复数词条、且只有内嵌域进 wasm，预计全部迁完也就再增数 KB —— 每批迁移后复查一次
- [x] 复数迁移覆盖全部「数量 + 可数名词」文案：累计 5 批共 **98 条**（35 + 27 + 27 + 3，
      外加 `radio.grids-worked` / `home.day-streak`、C5 四条与两个 L2 外层模板拆分）；
      `check-i18n` 复数候选 343 → 250 条，剩下的已逐条过目并归类（见 P3-C3「筛掉的三类」）：
      约 123 条是单位 / 百分比 / 时刻（`{} MHz` / `{}%` / `{}:00`），约 98 条是 `{}` 为名称
      或纯序号（`Question {} / {}` / `Grid {}` / callsign），29 条含名词但 count 恒 > 1
      （`CHALLENGE_COUNT`、`CQ_ZONE_MAX`、`MAX_COMPARE`）、是浮点字符串
      （`tools.runtime-hours` = `{:.2}`）或是标签性复数（`Questions: {}`）

> **数字会随迁移漂移**：本节各条记录的是该阶段落地当时的实测值。复审基线（2026-10-06）：
> zh / en / es 各 **2,983** 条、复数候选 251 条、`dist/*.wasm` 7.55 MB 量级。
> 同一次复审把 `check-i18n` 收紧成硬门禁：**漏 key / 死条目 / 三方 key 集合不一致 /
> `tf` 实参含中文 / 新增编号 slug** 一律判失败（此前只 `println!` 提示），
> 并新增「中文释义唯一」校验；清单与修复过程见当时的复审报告（未入库）。

## 7. 风险与对策

| 风险 | 对策 |
| --- | --- |
| 切分过程漏条目 / 串域 | 迁移脚本按「key 集合与条数前后完全相同」断言；`catalog.rs` 的 `bundled_domains_are_embedded_in_every_language` 测试长期守住 |
| 生成代码不可读、IDE 跳转失效 | 生成物只放 `OUT_DIR`，不入库；作者面对的永远是 `data/i18n/` 里的 JSON |
| build.rs 未感知资源变更 | `build.rs` 对每个 JSON 声明 `cargo:rerun-if-changed`；`Trunk.toml` 的 `watch` 同步加入 `../../data/i18n` |
| 死条目误判 | 判据是「源码里（含 `crates/core`）再也搜不到这个 key，且其中文原文不是任何字面量的子串」；清理前已逐条全仓库回查确认 |
| 语言包没到就渲染（先中文后译文） | 包在挂载前与切语言前载入（P3-B），`t()` 保持同步；包拉取有 **3 秒**超时（慢网下 6 秒骨架屏 = 「打开就卡住」），超时则退化为「内嵌域有译文、其余中文」 |
| 包晚于挂载到达（慢网 / 兜底挂载） | 包载入成功会 `bump` `pack::READY`（`ArcRwSignal`），`t()` / `tp()` 订阅它 —— 包一到，已渲染的视图自动重算，不会停在中文；首屏兜底超时取「包超时 + 1s」，不再早于包超时 |
| 首屏拉包失败后再也不重试（整场会话都是混排） | `pack::schedule_retry`：失败后按 3 秒间隔最多重试 3 次，成功即 `bump` `READY` 自愈；有界，离线时不会无限打请求 |
| 中文用户被迫为用不到的 en / es 付流量 | 语言包**不进** SW 预缓存清单（`postbuild::is_excluded`），改走 `RUNTIME.i18n` 按需缓存；`__CACHE_VERSION__` 仍把语言包内容哈希算进去，所以改译文照样换缓存版本 |
| 改了译文忘了刷新语言包 | `check-i18n` 比对 `data/i18n/` 与 `public/data/i18n/` 的字节内容；`add-i18n` 与 `build-web` 会自动刷新；Docker 构建阶段也会重新生成（见 `Dockerfile` 的 `i18n-pack` 步骤） |
| 语言包被浏览器 / SW 缓存住旧版 | 运行时缓存名带版本号（`i18n-<__CACHE_VERSION__>`），`activate` 里清掉旧版本；发布地址不带 hash，故不额外加 `?v=` |

## 8. 进度日志

| 日期 | 内容 |
| --- | --- |
| 2026-10-06 | 完成现状体检与方案评审，建立本文档 |
| 2026-10-06 | P0 + P1 + P2 落地：`i18n/` 模块拆分、词典资产化到 `data/i18n/`（12 个域）、`build.rs` 构建期代码生成、工具链改为读资源目录并新增死条目检查；清理死条目 25 条 |
| 2026-10-06 | 验证：`cargo make check` 全绿、`cargo make build-web` 通过，wasm 体积与重构前持平 |
| 2026-10-06 | P3-A 落地：2968 条 ×2 语言翻转为语义 key（`域.词条`）、新增 `zh` 目录使三语言同构；codemod 344 文件 / 3077 处调用点；中文反向索引兼容 `crates/core` 的动态中文标题 |
| 2026-10-06 | P3-B 落地：wasm 只内嵌 zh 全量 + en/es 的 `common` / `shell`，其余域走运行时语言包（`public/data/i18n/{lang}.json`）；包在挂载前与切语言前载入，零闪烁；`wasm` 7.66 → 7.53 MB |
| 2026-10-06 | P3-C 方案定稿并写入本文档：复数走「CLDR 类别 + 词条变体对象 + `tp()`」，明确不上 ICU / Fluent；性别与零值文案列为独立议题不做 |
| 2026-10-06 | P3-C1 内核：`core/plural.rs`（CLDR 类别 + 单测）、`tp()` 与共用 `substitute`、`build.rs` 扁平表 + 复数表双表、`catalog.rs` 复数索引、`pack.rs` 的 `Pack { flat, plural }` 与 `i18n-pack` 新结构；`common.days`（4 处调用）作样条迁成复数对象 |
| 2026-10-06 | 验证：`cargo make check` 全绿、`cargo make build-web` 通过；wasm +7.3 KB、语言包 +24 B；`i18n::tests` 断言 en 下 `1 day` / `2 days`、es 下 `1 día` / `2 días` |
| 2026-10-06 | P3-C2 工具链：`scan_calls` 认 `tp(` 并回带行号、`check-i18n --plural-candidates`（`cargo make i18n-plural-candidates`）导出 343 条候选与 `{one,other}` 骨架、`tp()` ↔ 复数变体双向对账（只提示不判失败）；扫源码前涂白 `#[cfg(test)] mod`、`add-i18n` 保序且只写有变化的域文件 |
| 2026-10-06 | P3-C3 第一批：`common` 域（内嵌域）迁 35 条复数词条、38 处 `tf(` → `tp(`（18 个文件）；候选 343 → 308 条；顺带修掉 4 条既有 en / es 参数顺序 bug（译文按英文语序写、调用点按中文语序传参）；`cargo make check` / `build-web` 全绿，wasm 累计 +13.8 KB |
| 2026-10-06 | P3-C3 第二批：`exam` / `learning` / `log` 迁 27 条（10 / 7 / 10）、27 处 `tf(` → `tp(`（16 个文件）；候选 308 → 281 条；筛掉百分比 / 单位 / 序号 / 纯数值、count 实为名称的三条、以及一句话两个以上可数名词的条目（`tp` 只吃一个 count，留作后续议题）；`cargo make check` / `build-web` 全绿，wasm 累计 +14.3 KB（第二批仅 +504 B，非内嵌域变体只进语言包） |
| 2026-10-06 | P3-C5 方案定稿：多计数文案定量后**不引入新机制**（真·多名词仅 4 条，2 条是常量、4 条是单位串误报）；定 L1 写作规范 / L2 嵌套格式化 / L3 改写三层策略，否决「组合变体」与 ICU；并纠正 C3 的过度保守——「多数量共用一个名词」（`common.correct-2` es `{} / {} correctos`）现有 `tp` 就能覆盖，属误伤 |
| 2026-10-06 | P3-C4 验收：`e2e/tests/i18n_plural.spec.ts`（en / es × 8 路由，种 count=1 数据后扫全部文案，断言无 `1 <复数名词>` 且至少命中一次单数形式），旅程进 `coverage-targets.json`；首跑抓出 `radio.grids-worked`（已迁）与 2 条多计数文案（列入用例白名单待后续方案）；候选 281 → 280 条，`cargo make check` / `build-web` 全绿，wasm 累计 +14.6 KB |
| 2026-10-06 | P3-C3 第四批（静态扫描盲区）：用「词典侧反查」补出 3 条走中文反向索引的漏网复数文案（`settings.merged-items` / `tools.restored-records` / `common.questions-2`），并把调用点改回语义 key；顺带修掉 `mistake_diagnosis.rs` 里 `format!` 硬编码中文句式（en / es 下整句显示中文）与随语言切换的标点；e2e `ROUTES` 从 8 扩到 33、名词表补齐，`i18n_plural.spec.ts` 2 passed；`tools_backup.spec.ts` 里一条因文案改写而失效的断言（`/回波损耗 6\.02 dB/` → `/回波损耗.*6\.02 dB/`）同步修正；wasm 7,547,249 B、en.json 185,173 B |
| 2026-10-06 | P3-C3 第三批：`tools` / `radio` 及 `common` / `log` / `learning` / `contest` / `home` / `knowledge` / `settings` 剩余候选过一遍，迁 27 条、22 处 `tf(` → `tp(`（22 个文件）；B 类（多数量共用一个名词）按 P3-C5 的纠正纳入迁移；剩余 250 条候选逐条归类为「单位 / 百分比 / 时刻」「`{}` 是名称或序号」「count 恒 > 1 或浮点」三类，判定无需再迁；候选 277 → 250 条，`cargo make check` / `build-web` 全绿，`i18n_plural.spec.ts` 2 passed，wasm 累计 +15.3 KB（7,546,965 B），en.json 185,059 B |
| 2026-10-06 | P3-C5 第 6 项①落地：`check-i18n` 新增「一句话多个可数名词」硬门禁 —— 词表自维护（从已迁复数变体取末词）+ 25 词 ×2 语言兜底名单，只有单复数两形都在表的词才算可数名词（`correct` / `minute` 之类自动滤掉），文案里 ≥ 2 处「`{}` + 可数名词」即判失败；三语实测 0 命中，`log.labels-qsos-pages` 历史文案探针准确报出 3 处；单测 `flags_sentences_with_multiple_countable_nouns`，`cargo make check` 全绿 |
| 2026-10-06 | 既有 e2e 失败清零（独立专项，清理记录未入库）：全量 434 passed / 6 failed → **440 passed / 0 failed** —— ① `/satellites` 横向滚动容器补 `tabindex="0"` + `role="region"` + `aria-label`（axe `scrollable-region-focusable`，新增 `radio.satellite-frequency-table` ×3 语言）；② `i18n_layout` 审计改为「祖先是横向可滚容器则不判越界、容器自身不判裁剪」，每语言 22 处 → 0；③ `data/knowledge-i18n/en/wspr.json` 的 key 漂移（`WSPR_NOTES` 首条静默回退中文）修正，并新增 `cargo make knowledge-i18n-check`（`knowledge_i18n::drift`）守门，已并入 `cargo make check` |
| 2026-10-06 | P3-C5 第 6 项②落地：`--plural-candidates` 把「源码直接用中文原文当 key」的调用点也算作调用点 —— 新增 `ZhIndex`（中文原文 → 语义 key，精确 + 最长包含匹配，仅带 `{}` 的原文参与包含匹配），`collect_call_sites` / `collect_plural_call_sites` / `call_locations` 三处统一解析，第四批那种静态扫描盲区从此自动现形；本次候选数不变（251 条，业务侧已无中文原文调用点），新增单测 `resolves_chinese_literal_calls_to_semantic_keys`，`cargo make check` 全绿 |
| 2026-10-06 | P3-C5 实施：4 条多计数文案按方案落地（`log.labels-qsos-pages` / `log.imported-records-skipped-duplicates` 走 L2 嵌套、`learning.challenge-streak-days-best` 走 L3 改写、`learning.correct-in-a-row` 归 B 类直接用 `tp`）；摘掉 e2e 白名单后复跑，又抓出 `home.day-streak`（es `Racha de {} días`）并顺手迁复数；候选 280 → 277 条，`check-i18n --lang en` 全绿，`i18n_plural.spec.ts` 2 passed / 0 findings，wasm 累计 +15.0 KB（7,546,586 B），en.json 183,953 B |
| 2026-10-06 | 提交前全量复审（415 文件 / +21,126 −14,099）：发现 1 处漏翻译 + 8 项守门缺口 + 8 项次要问题（报告为一次性产物，未入库） |
| 2026-10-06 | 复审修复落地：①`filter_section.rs` 的 `n 阶` 改走词典 `tools.order-n` / `-2n`；②`check-i18n` 变硬门禁（漏 key / 死条目 / 三方 key 集合 / 中文释义唯一 / `tf` 实参含中文 / 新增编号 slug），并先涂白注释再扫调用点；③删掉同义 key `log.imported-n-records` 与 3 条只在单测里出现的死条目；④`set_locale` 加请求序号、`main.rs` 加挂载兜底、`build.rs` 补目录级 `rerun-if-changed`；⑤`set_title` 改传 key（143 处）；⑥页面复用 `response_curve_log`，core 补边界断言与 9 个单测 |
| 2026-10-07 | 未提交变更全量复审（471 文件 / +37,670 −14,976，报告见 [`code-review-2026-10-07.md`](./code-review-2026-10-07.md)）并逐项修复：**P0** `apply_title` 漏掉 `t()`（143 处改传 key 后标题会显示原始 key）恢复查表，并新增 `set_title_with_args`（`radio.print` 之类带实参的标题也能跟随切语言，`scan_calls` 同步认这个新 API）；**P1** 语言包载入 `bump` 版本信号（`t()` / `tp()` 订阅，慢网不再整站停中文）、首屏兜底超时改「包超时 + 1s」、NEC 求解与 QSL 预览防抖（`util::debounce`）、八木优化改 `YagiSearch` 分帧（数值与同步版逐位一致，新增等价性单测）、删除通联时清理卡片影像（`next_id` 会复用 id）、QSL 报告解析 0 条与读取失败改为显式报错、`ci` 补 `knowledge-i18n-check`、`dev` 补 `i18n-pack`、`NEC` 拒 NaN/±inf 输入；**P2** `pack_is_stale` 出错视为陈旧、`knowledge_i18n` 词典改 `BTreeMap`（键序不再抖动）并校验 `--module`、线性方程组改相对主元阈值 + 结果有限性校验、`voacap` 并列取较早整点、`read_file_text` 加 64 MB 上限、`qsl_sync` 建索引去 O(n·m)；**P3** 47 处静态 `aria_label` / `placeholder` 改 `Signal::derive`（切语言即时更新）、`NativeSelect` 补方向键 / Home / End / Enter 与 `aria-activedescendant`、中文反向索引放开到扩展区与全角标点、`i18n-refactor.md` 更新过时描述与失效引用 |
| 2026-10-07 | 第二轮未提交变更复审（490 文件 / +39,558 −15,244，报告 [`code-review-uncommitted-2026-10-07.md`](./code-review-uncommitted-2026-10-07.md)，含逐项修复进度表）：**C** ①e2e 覆盖率门禁的 journey grep 歧义（`coverage.mjs` 必然 exit 1）收敛为唯一子串；②波形实验室 `Memo` 误用 `get_untracked()` 导致波形/频谱冻结，改回 tracked 读取并补「切调制方式后 `<path d>` 变化」断言；③新增 `util::mount_guard()` 修掉 MUF / NEC 三处「异步续体在组件卸载后碰信号 → wasm 白屏」。**M** ④语言包不再进 SW 预缓存清单（改 `RUNTIME.i18n` 按需缓存，内容哈希仍并入 `__CACHE_VERSION__`），中文用户不再为 en / es 付约 400 KB；⑤包超时 6s → 3s，并新增 3 秒 ×3 的有界重试（首屏拉包失败不再整场混排）；⑥Docker 构建补 `i18n-pack` 与 sstv / wspr worker，与 `cargo make build-web` 对齐；⑦`waveform_lab` 三个公开函数的 `assert!` 改为退化返回空结果（wasm 里 panic = 白屏）；⑧e2e 三处固定 `waitForTimeout` 改「等加载占位消失」（`fixtures.waitSettled`）。**m** ⑨Smith 除零与 NaN 传染、⑩`from_wires` 半径校验、⑪`EX` 卡电压改必填、⑫`nec_mesh` 峰值校验与线框上界、⑬`eye_traces` 降为 `O(n·span·sps)`、⑭`escape_rust` 转义 Cf 类字符、⑮工具链 `unescape` 认 `\xNN` / `\u{…}`、⑯`knowledge_i18n` 扩展名白名单、⑰时间选择器两列补 `aria-label`（新增 `common.hour` / `common.minute`，词典 3166 → 3168 条）、⑱读数行去掉全角空格、⑲方向图数据改 `StoredValue` 共享、⑳`nec.spec.ts` 假绿断言按 `title` 属性定位、㉑文档漂移（ROADMAP 用例/单测条数、README spec 清单、悬空引用）修正 |
