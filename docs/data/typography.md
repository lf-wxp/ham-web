# 题库文本排版规范

> 题面来自上游 CSV，同一道题在不同题库里的空格写法并不一致（「25 瓦」与「25瓦」、
> 「13.8/Ｎ」与「13.8 / Ｎ」）。若不统一，同一道题会得到**不同内容指纹**，带来三类问题：
>
> 1. 解析表 `data/explanations.json` 以指纹为 key，于是同一道题留下多份解析，长期下来互相矛盾；
> 2. 练习模式「只看本类新增」按指纹判重，同一道题会被当成「本类新增」再出一遍；
> 3. 错题本 / 覆盖率按指纹记 key，同一道题可能出现两条记录。
>
> 因此**在生成题库数据时就统一排版**，让同一道题的文本逐字节一致。

## 1. 归一化规则

规则实现在 `crates/core/src/typography.rs`，入口 `ham_web_core::normalize()`（幂等）：

| 规则 | 例子 |
| --- | --- |
| 全角 ASCII（Ａ-Ｚ、ａ-ｚ、０-９）转半角 | `0.091×Ｎ（安）` → `0.091×N（安）` |
| 连续空白折叠为一个空格、去首尾 | `天  线` → `天 线` |
| 中文与 ASCII 字母 / 数字之间补一个空格 | `使用 FT8模式` → `使用 FT8 模式`；`约5瓦` → `约 5 瓦` |
| 中文与全角标点之间不留空格 | `以下哪些 “ITU 分区”` → `以下哪些“ITU 分区”` |
| 斜杠两侧不留空格 | `0.0768 /N` → `0.0768/N` |
| 最后套用 `TEXT_FIXES`（上游已知文本错误） | `andstanding by` → `and standing by` |

刻意**不**处理的：中文 / 英文标点的选用、英文内部逗号后的空格、公式里 `=` `+` `×` 周围的空格 ——
题库里这些本就两种写法并存，强行统一会改动大量与「指纹分叉」无关的文本。

## 2. 生效位置

| 位置 | 说明 |
| --- | --- |
| `cargo make dataset`（`crates/tools/src/dataset.rs`） | 建库时对题干、选项、CSV 兜底解析列调用 `normalize()`，随后的指纹计算基于归一化文本；并刷新 `public/questions/search-index.json` |
| `cargo make questions-normalize` | 一次性迁移：不重新下载 CSV，把已提交的 `public/questions/*.json` 归一化，并同步迁移解析表 key、归一化解析正文与术语表释义，最后刷新搜索索引 |
| `cargo make explanations-add` | 新写的解析在入库时归一化 |
| `MistakeBook::rekey()`（`crates/core/src/mistake_book.rs`） | 前端加载错题本时按题目快照重算 key（幂等），排版变化不会让历史错题失效 |
| `unique_to_bank()`（`crates/core/src/practice.rs`） | 练习模式「只看本类新增」用 `content_key()`（忽略空白）判重，同一道题不会因写法差异再出一遍 |
| 前端全站搜索 | 索引 `/questions/search-index.json` 由 A/B/C 题库派生：题库刷新时一起重新生成，前端按三库修订号带 `?v=` 拉取、每次打开搜索核对修订号，数据更新后自动换新索引（离线由 Service Worker 回退旧副本） |

## 3. 发现新的上游文本错误时

1. 在 `crates/core/src/typography.rs` 的 `TEXT_FIXES` 增加一条 `(错误写法, 修正写法)`（只登记确定能修的）；
2. `cargo make questions-normalize` 应用（幂等，可重复执行）；
3. `cargo make explanations-check` 复核「同一道题多指纹」应为 0。

> 归一化无法解决**词边界**问题（`andstanding by` 该拆在哪需要人判断），所以这类错误只能逐条登记。
> 上游新增此类错误时，`explanations-check` 会以硬错误形式报出多指纹组。

## 4. 一次性迁移（2026-10 已执行）

`cargo make questions-normalize` 的结果：

| 项目 | 结果 |
| --- | --- |
| 题面改写 | 1466 道题（A/B/C/full 共 4483 条记录） |
| 解析表 | 2071 → 1369 条：700 组同题变体合并（保留最详细的一条，明细 `tmp/questions-normalize-report.json`） |
| 解析正文归一化 | 19 条；术语表释义 1 个文件（2 条 desc） |
| 搜索索引 | 重新生成 3108 条，题干与解析中 0 处残留未归一化相邻 |
| 验收 | `content_key` 分组下同题多指纹组 701 → **0**；`cargo make explanations-check` 全部为 0 |

影响面：

- **错题本**：加载时用题目快照重算 key，历史进度保留；同题变体的两条记录会合并（错次数累加、到期时间取早、题库集合取并集）。
- **覆盖率 / 单题统计**（`BankStats.seen`、`QuestionStats`）：只存 key、没有题目快照，无法重算，
  被改写题目的这些统计会重新计数。这是迁移唯一的数据损失，已在应用内更新日志
  （`crates/core/src/changelog.rs`）中说明。

## 5. 设计取舍：为什么不去改指纹算法

`fingerprint`（`crates/core/src/fingerprint.rs`）是**持久化 key**：解析表的 key、错题本与统计的
`question_key` 都由它派生。把它改成忽略空白，确实能让变体在 key 层面合并，但会同时废掉用户浏览器里
已有的一切 key；错题本有题目快照尚可重算，`BankStats.seen` 这类只存 key 的统计则无法恢复。

因此采取的做法是：**指纹保持稳定语义（空白折叠为单个空格），在生成数据时统一排版**，
让同一道题的文本本身一致；另配一个 `content_key()`（去掉全部空白）只用于内存中的同题判定
（练习判重、解析表变体对齐）。这样既消除了分叉，又不需要迁移用户数据。
