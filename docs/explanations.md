# 题目解析维护流程

> 写作规范（结构、格式硬约束、质量红线）见 [`explanations-style.md`](explanations-style.md)；
> 本文只管「怎么统计、合并、增强、写入」这套操作流程。指纹算法在 `crates/core/src/fingerprint.rs`。

### 核心概念：内容指纹

每道题的解析都以**内容指纹**为 key 存储在 `data/explanations.json`：

```text
指纹 = 规整后的题干 || A:选项A|B:选项B|… || 排序后的答案
```

例如：`我国专门针对无线电管理的行政法规及其制定机构是：||A:《中华人民共和国无线电管理条例》|B:…|D:工业和信息化部||AC`

- 与题号、题目顺序、所属题库**无关**：题库重排、增删题目不会导致解析错位；
- A/B/C 中内容相同的题目**共享同一条解析**，只需写一次；
- 题干、选项或答案发生**任何变化**都会产生新指纹 —— 官方修订过的题目会自动显示为「缺失解析」，避免旧解析误用。

> ⚠️ 指纹算法实现于 `crates/core/src/fingerprint.rs`，与历史数据逐字节兼容，请勿修改。

### 维护流程总览

```text
 ┌──────────────────────────┐
 │ 1. 统计缺失               │  cargo make explanations-missing
 │    → tmp/batch.json      │  （{ "B-12": "" , … } 待填模板）
 │    → tmp/context.json    │  （题干 / 选项 / 答案，供撰写参考）
 └────────────┬─────────────┘
              ▼
 ┌──────────────────────────┐
 │ 2. 撰写解析               │  编辑 tmp/batch.json，填入解析文本
 └────────────┬─────────────┘
              ▼
 ┌──────────────────────────┐
 │ 3. 合并 + 去重 + 增强 + 对齐 + 写入 │  BATCH=tmp/batch.json cargo make explanations
 └────────────┬─────────────┘
              ▼
 ┌──────────────────────────┐
 │ 4. 本地预览并提交          │  cargo make dev → git commit data/ public/questions/
 └──────────────────────────┘
```

> 所有参数（`BANK`、`LIMIT`、`BATCH`）均通过**环境变量**传入：`VAR=值 cargo make 任务` 或 `cargo make -e VAR=值 任务`。

### 第 1 步：统计缺失解析

```bash
cargo make explanations-missing              # 全部题库
BANK=C LIMIT=50 cargo make explanations-missing   # 仅 C 类，最多导出 50 题
```

输出每个题库（A/B/C 以及 full 完整题库）的解析覆盖情况，并生成：

- `tmp/batch.json`：待填模板（已按指纹去重，跨题库重合的题只出现一次）
  ```json
  { "B-12": "", "C-305": "" }
  ```
- `tmp/context.json`：题目上下文，便于人工或借助 AI 批量撰写
  ```json
  [{ "id": "B-12", "J": "LK0123", "P": "3.3.2", "type": "单选",
     "question": "…", "options": ["A. …", "B. …"], "answer": "B" }]
  ```

也可直接调用：`cargo run -p ham-web-tools -- missing-explanations --help`。

### 第 2 步：撰写解析

> 完整写作规范（结构模板、分类写法、格式硬约束、质量红线、审查清单）见 [`docs/explanations-style.md`](explanations-style.md)。

在 `tmp/batch.json` 中为每个 ID 填写解析。建议遵循现有风格：

1. **先给结论**：点明正确选项及依据，如「故 A、C 正确」；
2. **逐项排除**：简述错误选项错在哪里；
3. **引用依据**：法规题写明条款（如「《业余无线电台管理办法》第二十条」），技术题给出公式或原理；
4. **控制篇幅**：通常 1–4 句，面向初学者，避免堆砌术语；
5. **专业术语不必手动解释**：第 3 步会用术语表自动注入通俗解释；
6. 留空（`""`）的条目会被跳过，可分多批完成。

> 已有解析需要修改时，同样把对应题目 ID 写进 batch 文件即可覆盖；ID 可在「题库分类浏览」页每道题左上角看到。

### 第 3 步：合并、增强、写入

```bash
BATCH=tmp/batch.json cargo make explanations
# 或：cargo make -e BATCH=tmp/batch.json explanations
```

等价于依次执行：

| 步骤 | 命令 | 作用 |
| --- | --- | --- |
| 合并 | `BATCH=… cargo make explanations-add` | 按 ID 找到题目、计算指纹，写入 `data/explanations.json`（未知 ID 会告警） |
| 去重 | `cargo make explanations-dedupe` | 清理解析里重复注入的术语括注（同一条只保留首次出现），并删除注进更长词语内部的错位括注 |
| 增强 | `cargo make explanations-enhance` | 用 `data/glossary/` 中 `inject: true` 的词条，为每条解析中**首次出现**的术语追加 `（通俗解释）`，幂等可重复执行 |
| 对齐 | `cargo make explanations-sync` | 同一道题因题库间空格写法不同而有多个指纹时，保留最详细的一条解析并写回该组所有指纹（明细见 `tmp/explanations-sync-report.json`） |
| 写入 | `cargo make explanations-apply` | 把解析写入 `public/questions/*.json`（离线完成，无需重新拉取 CSV） |

术语增强规则：长术语优先（「对流层散射」先于「散射」）；术语后已有括号、或处于原文括号内时跳过；每个术语每条解析只注入一次；落在更长词语内部（复合词阻止表，如「超视距」「副载波」「必要带宽」「亚音调静噪」）时跳过；「发射频率」中的「射频」不视为术语。

### 第 4 步：预览与提交

```bash
cargo make dev    # 在练习模式 / 分类浏览页检查解析显示
git add data/ public/questions/
git commit -m "docs(explanations): 补充 C 类解析 50 条"
```

### 维护术语表

术语表按一级分类拆分为 `data/glossary/` 目录下的多个文件（`law.json`、`frequency.json`、`operation.json`、`slang.json`、`modulation.json`、`equipment.json`、`antenna.json`、`propagation.json`、`basics.json`、`safety.json`），前端编译期嵌入（离线可用），`check-glossary` 会读取并合并目录下全部文件后校验。每个文件为 JSON 对象，key 为术语（中文名称，或 `QRM`、`73` 这类缩写本身），词条顺序即页面同分类内的展示顺序：

```json
{
  "驻波比": {
    "abbr": "SWR",
    "en": "Standing Wave Ratio",
    "category": "天线",
    "desc": "衡量天线和连接线配合得好不好的指标；配合越好能量浪费越少，数值越接近 1:1 越好",
    "aliases": ["电压驻波比", "VSWR"],
    "inject": true
  },
  "PEP": {
    "en": "Peak Envelope Power",
    "category": "设备",
    "desc": "峰包功率，即发射信号在最高点那瞬间的功率",
    "see": "峰包功率",
    "inject": true
  }
}
```

| 字段 | 必填 | 说明 |
| --- | --- | --- |
| `desc` | 是 | 通俗解释，一句话、面向初学者；**不要包含全角括号**（与注入格式冲突） |
| `category` | 建议 | 一级分类 key：`法规` `频率` `操作` `用语` `调制` `设备` `天线` `传播` `基础` `安全`，缺省归入「其他」 |
| `abbr` | 否 | 英文缩写，如 `SWR`；术语本身是 ASCII 时自动视为缩写，无需重复填写 |
| `en` | 否 | 英文全称 |
| `aliases` | 否 | 别名 / 同义词，参与搜索并显示为「又称」 |
| `see` | 否 | 参见的其他术语（必须是已存在的 key），页面上可点击跳转 |
| `inject` | 否 | 默认 `false`；为 `true` 时参与解析术语注入，此时解释中不能含任何括号 |
| `common` | 否 | 默认 `true`；`false` 表示「全量补充」的冷门词条。简语页（`/q-code`）据此区分「常用 / 全部」两个视图 |

旧写法 `"术语": "解释"` 仍然兼容，等价于 `{ "desc": "解释", "inject": true }`。

> **关于 `inject`**：注入是按子串匹配的，短缩写（如 `AM`、`CW`）或常见词会在解析中被大量误注入，因此新词条默认不参与注入。只有确实需要在解析中自动附加解释的中文专业术语才建议设为 `true`，并在执行后抽查效果。

修改后：

```bash
cargo make glossary-check                                    # 校验：分类是否存在、参见目标、重复词条、括号等
cargo make explanations-enhance && cargo make explanations-apply   # 仅当新增/修改了 inject=true 的词条时需要
cargo make dev                                               # 在 /glossary 预览
```

术语表页面功能：按分类筛选、搜索（术语 / 缩写 / 英文 / 别名 / 解释，精确匹配优先）、「只看英文缩写」、参见跳转，并统计该术语在 A/B/C 各题库中出现的题数（与分类浏览页的关键词搜索一致），点击即跳转到 `/browse?bank=…&q=…`。长度小于 3 的纯 ASCII 术语（如 `K`、`73`）误匹配过多，不做统计。也支持 `/glossary?q=驻波比` 直接定位。全站搜索面板同样会检索术语表与简语条目。

### 题库更新后的解析迁移

官方更新题库后：

```bash
cargo make dataset                 # 重新生成题库（内容未变的题目自动沿用解析）
cargo make explanations-missing    # 查看因题目修订/新增而缺失的解析
```

旧解析保留在 `explanations.json` 中不会自动删除，不影响使用。

### 存档快照里的解析

界面里的解析只有**一个口径：题库当前版本**（`public/questions/*.json` 里的 `explanation`）。
两处存档会带一份题目快照，快照里的解析是存档当时的版本，解析订正后它不会自己更新：

| 存档 | 谁在读 | 怎么保证读到现在的 |
|---|---|---|
| 错题本 `mistake-book` | 错题集（列表 / 重练 / 闪卡复习）、复仇战 | 展示时经索引取题库当前版本的解析；快照与存档原样不动 |
| 考后复盘 `exam:last-review` | 考后复盘页 | 每题的解析按题库当前版本显示 |

规则集中在 `crates/core/src/explanation_index.rs`（带单测）。前端分两步用它：
`data::spawn_explanation_index(banks, 信号)` 在后台把题库并入索引，展示处用
`data::snapshot_explanation` / `data::review_explanation` 拿一个随索引到达自动更新的
`Signal<Option<String>>`，交给 `ExplanationCard`：

- **以稳定 ID 且题干、选项、答案集合都一致认题**（空白按内容指纹同口径规整）。只看 ID 不够：
  题库重排 / 增删后同一个 ID 可能指向另一道题；只看 ID + 题干也不够：勘误只改了答案或选项时，
  旧答案配上针对新答案的解析会自相矛盾。对不上就不换，沿用快照自己的解析 —— 宁可旧，
  也不能把别的题（或别的答案）的解析安上来。
- **认得出就以题库为准**，包括题库已撤下解析的情况（不再展示快照里的旧文字）。
- **认不出才用快照兜底**，题库没加载出来（离线、失败）时同样如此，所以这些页面不会因此变成依赖网络。
- **从不改写快照**：题干 / 选项 / 答案是错题 key（内容指纹）的输入，不能跟着动；换的只是
  展示出来的那份文字。注意错题**再次答错**时，`MistakeBook::record_result` 会用当次的题目覆盖
  快照 —— 那是正常的存档更新，与本节的展示规则无关。
- **按需、不阻塞**：只拉快照 id 涉及的题库（`banks_for`，没有快照就一个都不拉），各题库并发拉、
  到一个并入一个；页面从不等它 —— 索引没到之前显示快照里的，到了只换卡片里的文字，用户已经
  展开的卡片不会因此收起。

新增读取解析的页面时：题目直接来自 `data::load_bank` 的不用管；题目来自存档快照的，必须过上面这一步。

