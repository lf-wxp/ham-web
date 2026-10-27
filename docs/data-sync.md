# 外部数据同步规范

> **这份文档解决什么**：项目里有四份数据从外部拉来，会不定期更新。本文规定它们的来源、产物、
> 来源戳的结构与口径、新鲜度判定矩阵、四个命令、CI 流程与验收方式。
>
> **它是规范，不是教程**：每条都对应代码里的实现（正文标注文件与函数）。行为改了必须同步改这里；
> 两者不一致时**以代码为准**，并把文档补齐。

---

## 1. 模型

```
上游（HTTP） ──► 生成器（ham-web-tools dxcc / dxcc-map / cn-map / dataset）
                      │
                      ├─► 产物（提交入库）
                      └─► 来源戳（产物头部注释 / data/provenance/*.source.json）
                                  │
        ┌─────────────────────────┴──────────────────────────┐
        │                                                    │
check-data-sync（离线，挂 cargo make check）      check-data-freshness（联网）
判「戳齐不齐、依赖变没变」                       判「上游动没动」
                                                             │
                                                  data-sync.yml（每周一 + 手动）
```

分工的硬边界：

- **离线检查**只读本地文件，可以放进 `cargo make check` / `ci`，永远不因为网络抖动变红。
- **联网检查**只回答「上游是否变化」，连不上只报告（`--strict` 时才算失败）。
- **重新生成**（`data-sync`）永远需要人触发，结果走 PR 让人看一眼，不直接推 main。

---

## 2. 数据源与产物

### 2.1 四个源

| 数据 | 生成器 | 上游 | 默认地址 | 关系 |
| --- | --- | --- | --- | --- |
| DXCC 前缀表 | `dxcc`（`dxcc.rs`） | AD1C 的 cty.csv | `https://www.country-files.com/bigcty/cty.csv` | 逐行转写 |
| DXCC 实体边界 | `dxcc-map`（`dxcc_map.rs`） | Natural Earth 1:110m | `https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_110m_admin_0_countries.geojson` | 派生 |
| 中国省级边界 | `cn-map`（`cn_map.rs`） | Natural Earth 1:50m 州省 | `https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_50m_admin_1_states_provinces.geojson` | 派生 |
| 题库 | `dataset`（`dataset.rs`） | 第三方 CSV 仓库 | `https://raw.githubusercontent.com/xiedada05/crac-amateur-radio-exam-questions-2025-csv/main` | 派生（多文件） |

### 2.2 两种关系决定判定方式

| 关系 | 含义 | 新鲜度怎么判 |
| --- | --- | --- |
| `Verbatim` | 产物是上游文件的**逐行转写**（`dxcc.txt`） | 直接比上游内容的 sha256 |
| `Derived` | 产物由上游**计算**出来（几何被简化量化、CSV 被编译成题目） | 只能靠校验头发条件 `HEAD`；摘要没有可比性 |

拿派生产物的摘要去比上游，会永远报「有更新」——这是实现时踩过的坑，`data_sync.rs` 的
`derived_target_is_never_verbatim` 钉住分类，`derived_judgement_uses_the_conditional_probe`
钉住判定。

题库是 `Derived` 的特例：上游是**目录**，没有「那一个文件的 `ETag`」，所以戳里记**逐文件**校验头
（见 §3.4），检查时逐个文件探活。

### 2.3 产物落点与写入者

| 产物 | 写入者 | 戳载体 |
| --- | --- | --- |
| `crates/core/data/dxcc.txt` | `dxcc` | 文件头 `#` 注释 |
| `public/dxcc-entities.bin` | `dxcc-map` | `data/provenance/public/dxcc-entities.bin.source.json` |
| `public/cn-provinces.bin` | `cn-map` | `data/provenance/public/cn-provinces.bin.source.json` |
| `public/questions/{A,B,C,full}.json`、`public/questions/images/*`、`public/questions/search-index.json` | `dataset` | `data/provenance/public/questions/config.json.source.json` |

两点容易误会：

- 题库的戳**挂在 `public/questions/config.json` 上**（它是题库入口，`data-freshness` 的
  `Target.artifact` 也是它），而不是挂在每个题库 JSON 上。
- `config.json` 本身是提交入库的题库配置，`dataset` 只更新它的边车；`postbuild` 在构建时把
  `rev` 写进 `dist/` 里的那份副本（不回头改仓库里的源文件）。
- `dxcc.txt` 由 `include_str!` 编进 `ham-web-core`，被单测直接消费——所以「数据写坏了」与
  「解析不出来」只能靠 `check` job 抓，`data-sync-check` 抓不到。

### 2.4 谁进这套机制、谁不进

仓库里带「外部来源」的东西不止上面四份，但**不该用同一套办法**。按「上游能不能对拍」分四桶：

| 桶 | 例子 | 判新旧的方式 | 现状 |
| --- | --- | --- | --- |
| A 可下载、可逐字比对 | `dxcc.txt`（cty.csv） | 摘要 | 已纳入 `data-freshness` |
| B 可下载、只能比校验头 | `dxcc-entities.bin`、`cn-provinces.bin`、题库 | 条件 `HEAD`（题库逐文件） | 已纳入 |
| C 人工转录、上游是 HTML / 法规页 | `crates/core/data/gear-rx.txt`、`crates/core/data/radio-law.txt`、`toroid::CORES`、`provenance::PROVENANCES` 的 8 个知识域、`diy_projects::COST_CHECKED_ON` | **只能看核对日期的年龄** | 已纳入 `data-staleness`（§6.2） |
| D 第三方 vendored、版本固定 | `public/vendor/katex`、`public/fonts/*`、`pixel/sprite_art.rs`（源自 pixelarticons） | 记版本 + 人工升级 | **不自动同步**（版本记录见下） |

C 桶为什么不能套 A / B：这些数据是人对着页面抄进仓库的，抄完就没有可比对象了；上游页面的
`Last-Modified` 变了也可能只是模板改版。唯一可靠的信号是「多久没人复核」——所以口径是**年龄**，
不是「上游变没变」。

D 桶为什么不进周检：升级会同时改体积预算（`cargo make size-check`）与视觉，必须人看。要的只是
「现在用的是哪一版」：

| 资源 | 版本记在哪 |
| --- | --- |
| KaTeX | `public/vendor/katex/VERSION`（含来源链接与升级步骤） |
| Geist / Geist Mono / Fusion Pixel / Press Start 2P / Silkscreen | `public/fonts/LICENSES`（来源与许可证；上游按 commit 发布、没有稳定版本号，升级时人工登记） |

**明确不进任何同步机制**（无上游、纯内部派生）：`public/data/i18n/*.json`、
`dist/data/knowledge-i18n/*.json`、`public/questions/search-index.json`、`public/questions/config.json`、
worker bundles / wasm / `sw.js` / `sitemap.xml` / PWA 图标 / `style/pixel/schemes.css`。它们由仓库内
输入确定性生成，改输入就重新生成，没有「上游是否变化」这个维度。

---

## 3. 来源戳（provenance）

### 3.1 载体与位置

| 产物类型 | 载体 |
| --- | --- |
| 文本（`dxcc.txt`） | 文件头 `#` 注释块 |
| 二进制 / JSON | 边车 `data/provenance/<产物相对路径>.source.json` |

边车按**产物相对路径镜像**（`provenance::sidecar_path`），避免不同目录下同名产物互相覆盖。

**硬规则：边车只能落在 `data/provenance/`。** 理由见 §3.6。

### 3.2 文本产物的头部（真实形状）

```
# 由 `ham-web-tools dxcc` 从 https://www.country-files.com/bigcty/cty.csv 生成，请勿手工编辑。
# 摘要: sha256:c758e804…
# 抓取: 2026-10-22T08:28:59Z
# 上游版本: VER20261017
# Last-Modified: Sat, 17 Oct 2026 10:44:30 GMT
# ETag: "cbbe-5f8f3a"
```

补戳产物多一行 `# 补戳: 是（摘要是既有产物自身的，不是上游的；下次 `cargo make dxcc` 会写入真实戳）`。

`dxcc.txt` 是**单文件来源**，头里**不会**出现 `# 输入:`；`inputs` 与 `files` 只存在于边车 JSON。

### 3.3 字段

| 字段 | 作用 |
| --- | --- |
| `source` | 实际抓取地址；离线输入时是 `file://<文件名>`（据此跳过联网比对）。**只记文件名、不记绝对路径**：戳会入库，绝对路径会泄露本机用户名 |
| `digest` | 上游原始字节的 sha256。题库由多份 CSV + 图片编译而成，这里是本次读过的全部上游文件的**合并摘要**（路径排序后「路径 + 各自摘要」再取摘要） |
| `fetched_at` | 抓取时间（UTC，RFC 3339） |
| `version` | 上游期号，形如 `VER20261017`；**取不到就不写**，不猜 |
| `etag` / `last_modified` | 下载时服务端给的校验头；单文件派生产物据此发条件 `HEAD` |
| `inputs` | **仓库内**依赖：相对路径 → 摘要（口径见 §3.5） |
| `files` | **多文件来源**的逐文件戳：上游文件相对路径 → { 摘要, `etag`, `last_modified` }（见 §3.4） |
| `backfilled` | 由 `data-stamp` 补写（见 §5.4）；摘要是产物自身的，不是上游的 |

### 3.4 `files`：多文件来源的逐文件戳

题库的上游地址是目录，没有「一个文件的校验头」。所以 `dataset` 建库时把 **4 份题库 CSV +
`images.csv`** 的摘要与下载时拿到的校验头逐个记下；检查时对每个文件各发一次条件 `HEAD`，
任何一个变了就是有新版本。

**图片不逐张探活**：新题一定伴随 CSV 变化，几百个 `HEAD` 换来的只是更慢的检查。

边界：本地 `DATASET_DIR` 建库没有校验头可记（整个产物本来就是「本地来源」，不参与联网比对），
此时 `files` 为空，判定退回单份校验头路径。

**存量边车要刷新一次才生效**：`files` 是新增字段，引入它之前生成的边车里没有（`serde(default)`
读成空），`check-data-freshness` 会退回单份校验头路径，对题库仍报「无法判断上游是否变化」。
联网执行一次 `cargo make dataset`（或 CI 的 `regenerate`）重新抓取，边车里才有逐文件校验头，
之后逐文件探活才真正开始工作。

### 3.5 `inputs`：仓库内依赖，与摘要口径

`inputs` 记录「这个产物依据了仓库里的哪些文件」。它解决的是「上游没变、但产物该重建」：

- `dxcc-entities.bin` 的实体名来自 `dxcc.txt`，所以边车记着它的摘要。`dxcc.txt` 一变，`.bin`
  就该重建——此前两者之间没有任何记录，看不出来。
- 题库的解析来自 `data/explanations.json`，所以边车记着它的摘要。解析改了却忘了重建题库，会被抓出来。

`cn-provinces.bin` **刻意没有 `inputs`**：它的几何纯粹由上游 GeoJSON 决定，省份名与分区号
的对应关系在 `reference::CHINA_PROVINCES` 里，但改那张表（比如调措辞）不该触发一次联网重建。
「产物里的地名码集合 == 省份表」由 `cn_map` 的单测 `committed_artifact_covers_exactly_the_province_table`
离线守着 —— 改了表却没重建，`cargo make test` 会直接红。

**摘要口径**（`provenance::input_digest`）：`.txt` 数据文件只摘**数据行**、跳过 `#` 注释头；
其余文件按原始字节。原因：`dxcc.txt` 的注释头里有抓取时间，上游没变、只重跑了一遍
`cargo make dxcc`，文件也会变；摘要若把注释头算进去，依赖它的 `.bin` 就会被误判过期。

### 3.6 为什么边车放 `data/provenance/` 而不是产物旁边

`public/questions/` 在 `index.html` 里是整目录 `copy-dir` 进产物的。边车如果放在
`public/questions/config.json.source.json`：

1. 会被发布到线上（构建元数据本不该是发布物）；
2. 会进入 Service Worker 的预缓存清单；
3. 后果最重的是——戳里的 `fetched_at` 是抓取时刻的墙上时间，**每次重新生成都会变**，
   于是内容没变也会改掉 `__CACHE_VERSION__`，让所有用户重下整个预缓存。

实测（2026-10-22，注入边车后跑 `postbuild`）：

```
无 source.json：Generated sw.js (version ccbce065d2586c40, 20 precached files)
有 source.json：Generated sw.js (version 1eaabfb3fa43c61e, 21 precached files)
```

现在有两处，互为表里：

- **主修**：边车落在 `data/provenance/`，`data/` 不进 `public/`，也就不进产物。
- **兜底**：`postbuild` 的 `is_excluded` 含 `rel.ends_with(".source.json")`，即使将来有人把边车
  放回 `public/`，也不会污染预缓存（单测 `excludes_non_precache_files`）。已验证：强行注入后仍是
  20 个文件、版本号不变。

### 3.7 写戳的四条硬规则

1. **不许编上游摘要**：拿不到上游就记产物自身摘要并标 `backfilled`，见 §5.4。
2. **`version` 取不到就不写**，不猜、不补。
3. **空校验头不写**（避免 `"etag": ""` 这种既占地方又判不出新旧的值）。
4. **不记绝对路径**，只记文件名 / 相对路径。

---

## 4. 判定矩阵（`data_sync::judge`）

### 4.1 七种结论

| 结论 | 触发条件 | 联网 | `data-freshness` 退出码 | 输出标记 |
| --- | --- | --- | --- | --- |
| `UpToDate` | Verbatim：上游摘要相同；Derived：条件 `HEAD` 得 `304`；多文件：每个文件都是 `304` | 是 | 0 | `✓` |
| `Changed` | 上游摘要不同 / 条件 `HEAD` 得 `200` / 任一文件 `200` | 是 | **1** | `↑` |
| `NoStamp` | 产物没有戳（引入戳之前的旧产物，或被手改） | 否 | **1** | `?` |
| `Backfilled` | 戳带 `backfilled` | 否 | **1** | `?` |
| `LocalOnly` | `source` 以 `file://` 开头（`--input` / `DATASET_DIR`） | 否 | 0 | `·` |
| `Unknown` | Derived 且戳里没有可用的校验头（多文件时：没校验头的文件跳过不探） | 有校验头的照探，一个都没有则不联网 | 0 | `·` |
| `Unreachable` | 下载失败 / 探测失败 / 服务端 4xx | 是（失败） | `--strict` 时 **1**，否则 0 | `!` |

### 4.2 判定顺序（不能调换）

```
1. 没有戳                    → NoStamp        （不联网）
2. source 是 file://         → LocalOnly       （不联网）
3. backfilled                → Backfilled      （不联网）
4. Verbatim                  → 比上游摘要
5. Derived 且 files 非空     → 逐文件条件 HEAD
6. Derived                   → 单份条件 HEAD
```

1–3 必须在联网之前短路：补戳产物若去比摘要，第一次比对就必然误报「有更新」；本地来源根本不该
拿上游去比。`backfilled_stamps_are_marked_and_never_compared_with_upstream` 与
`no_stamp_and_local_sources_never_touch_the_network` 用「碰一下就 panic 的上游」钉住了这一点。

### 4.3 多文件来源的优先级

`变了 > 连不上 > 不知道 > 最新`。只要有一个文件确实变了就该重新生成，别让另一个文件的网络故障
把它盖成「不可达」而白等一周（`changed_file_wins_over_an_unreachable_one`）。

### 4.4 退出码

- 任一源 `Changed` / `NoStamp` / `Backfilled` → 退出码 1（`有数据源需要更新`）。
- 有源 `Unreachable` 且传了 `--strict` → 退出码 1（`上游不可达（--strict）`）。
- 其余 → 0。`cargo make data-freshness` **不带** `--strict`（本地网络受限时不该红）；CI 带。

---

## 5. 命令

### 5.1 data-\* 任务

| 命令 | 联网 | 读 | 写 | 用在 |
| --- | --- | --- | --- | --- |
| `cargo make data-sync` | 要 | 上游 | 四个产物 + 四份真戳 | 人触发重新生成（CI 由 `regenerate` job 跑） |
| `cargo make data-sync-check` | 不要 | 产物 + 边车 + `inputs` 指向的本地文件 | 无 | **已在 `cargo make check` / `ci` 里**（§6.1） |
| `cargo make data-freshness` | 要 | 戳 + 上游 | 无 | 本地想知道「上游动没动」 |
| `cargo make data-stamp` | 不要 | 既有产物 | 只补戳，**内容一个字不动** | 引入戳之前的旧产物（一次性） |
| `cargo make data-staleness` | 不要 | 各处核对日期 | 无 | **已在 `cargo make check` / `ci` 里**，到期只告警（§6.2） |
| `cargo make data-staleness-strict` | 不要 | 各处核对日期 | 无 | 到期也判失败；CI 周检 `staleness` job 用，本地复核前也可跑 |

对应的子命令：`check-data-sync` / `stamp-data` / `check-data-freshness [--only dxcc,dxcc-map,cn-map,dataset]
[--strict]` / `check-data-staleness [--max-months 6] [--warn-only]`。

### 5.2 生成器与本地输入

| 生成器 | 本地输入 | 远程地址覆盖 | 备注 |
| --- | --- | --- | --- |
| `dxcc` | `--input <cty.csv>` | `--url` | 本地输入时 `source` 记成 `file://cty.csv` |
| `dxcc-map` | `--input <geojson>`（也接受旧版 JSON，走 `convert_json_to_bin`） | `--url` | 任何情况下都记 `dxcc.txt` 为输入 |
| `cn-map` | `--input <geojson>` | `--url` | 只取 `adm0_a3 = CHN` 的 31 个要素；地名码与 `CHINA_PROVINCES` 对齐，对不上会打印报告 |
| `dataset` | `DATASET_DIR` / `--dataset-dir` | `DATASET_REMOTE` / `--remote` | 本地目录时 `source` 记成 `file://<目录名>` |

本地输入意味着「这批产物不来自上游」，`data-freshness` 会归为 `LocalOnly`（绿、不比对）；
但 `inputs`（`dxcc.txt` / `data/explanations.json`）照记，离线一致性检查照样生效。

### 5.3 改了解析库之后

`data/explanations.json` 是题库边车的输入。改完它的离线流程是：

```bash
cargo make explanations-apply   # 把解析写回 public/questions/*.json（离线）
```

`explanations-apply` 在四个题库都写完后，把边车里记的解析库摘要刷新到当前值
（`provenance::refresh_inputs`），所以随后的 `cargo make check` 不会再要求联网重跑
`cargo make dataset`。**只有四个题库文件缺了一个时才不刷新**——那时「题库与解析库一致」并不成立。

`refresh_inputs` 只刷新**已有**的输入键，不新增；没有边车时什么也不做（那是 `data-stamp` 的职责）。

### 5.4 什么时候用 `data-stamp`

产物是在引入来源戳之前生成的（`dxcc.txt` 就是这种情况）。此时拿不到当初那份 `cty.csv`，
**不能编一个假的「上游摘要」**——那会让新鲜度检查永远认为已最新。

`data-stamp` 记的是「既有产物自身的摘要」，并写 `backfilled` 标记（文本头里是 `# 补戳: 是`）。
新鲜度检查见到这个标记，**不去比摘要，也不联网**，直接报「来源戳是补写的，需要重新生成」。
下次真正执行 `cargo make dxcc` / `dxcc-map` / `cn-map` / `dataset` 时写入真戳，标记随之消失。

两个已知副作用：

- 补戳记的 `source` 是工具的**默认地址**（当初真实抓取地址已不可考）。
- `data-stamp` 是幂等的：已有戳的产物会被跳过。

---

## 6. 离线检查（都挂 `cargo make check`）

两个方向，都不联网、都不会因为网络抖动变红：

| 命令 | 回答的问题 | 对应桶（§2.4） |
| --- | --- | --- |
| `cargo make data-sync-check` | 产物有没有戳、依赖变没变 | A / B |
| `cargo make data-staleness` | 人工转录的外部事实多久没复核 | C |

### 6.1 来源戳一致性：`check-data-sync`（`data_sync::run`）

| 检查 | 抓什么 | 修复提示 |
| --- | --- | --- |
| 产物有没有戳 | `dxcc.txt` 的头部、两个 `.bin` / 题库的边车 | `cargo make dxcc` / `dxcc-map` / `cn-map` / `dataset` |
| 戳字段齐不齐 | `dxcc.txt` 的摘要长度、抓取时间 | `cargo make dxcc` |
| 依赖变没变 | 边车 `inputs` 里每个文件的当前摘要（`cn-provinces.bin` 没有 `inputs`，见 §2.3） | `dxcc-entities.bin` → `dxcc-map`；题库 → `explanations-apply`（离线）或 `dataset`（重新拉上游） |

输出与退出码：

```
✓ 数据产物来源戳齐全，依赖摘要与现状一致        # 退出码 0
✗ 数据产物的来源戳有问题 N 处：               # 退出码 1
    <产物>：<问题>
      → 修复：<命令>
```

它**不判断上游新旧**，也**不联网**：重建 `.bin` / 题库需要出网，把网络依赖塞进本地检查会让离线
环境无故变红。它也不校验 `files` 里的上游摘要（那些文件不在仓库里）。

### 6.2 时效性数据是否过期：`check-data-staleness`（`data_sync::staleness`）

登记表在 `ham_web_core::staleness::curated()`：`provenance::PROVENANCES` 的 8 个知识域，加四个
数据面 `gear-rx` / `radio-law-text` / `toroid` / `diy-cost`。每一项的核对日期都写在**它自己的
数据文件或常量**里（条文库读文件头的 `# 核对日期：`，磁环取 `CORES[].verified` 里最旧的一条），
这里只做判定，不另存一份日期。

判定规则：

- `今天 - checked_on > 6 个月` → **到期**（`--max-months` 可改阈值）；
- 日期读不出来或格式不对 → **数据错误**（不知道就不能当新鲜）；
- 核对日期晚于今天 → **数据错误**（写了还没发生的复核，记录不可信）。

两类的处置不同（`ham_web_core::staleness::Age`）：

| 情形 | 默认（`data-staleness-strict`） | `--warn-only`（`cargo make check` / `ci` 用） |
| --- | --- | --- |
| 到期 | 失败 | 只告警，退出码 0 |
| 读不出来 / 在未来 | 失败 | **仍然失败** |

为什么 `check` 里只告警：到期随时间自然出现，若在每个 PR 都跑的 `check` 里判失败，某天起所有无关
改动（包括 `chore/data-sync` 的自动 PR）会同时变红。到期的红灯放在 CI 每周一的 `staleness` job
（`data-sync.yml`，独立于 `regenerate`，红了不挡数据重新生成）。数据错误是确定性的，所以任何模式
都失败。

输出会指名到「改哪个文件 / 常量」：

```
时效性数据的核对日期（今天 2026-10，阈值 6 个月）：
✓ bands：2026-10（0 个月）—— 《中华人民共和国无线电频率划分规定》
⚠ gear-rx：2025-11（11 个月）—— Sherwood (NC0B) Receiver Test Data
→ 复核后把日期改成当前月份，改在这些地方：
    gear-rx：`gear_rx::CHECKED_ON` 与 `crates/core/data/gear-rx.txt` 文件头
Error: 1 项时效性数据超过 6 个月未复核
```

口径边界：它判的是「**该回去看了**」，不是「上游真的变了」——C 桶没有可比的摘要，能给的只有
时间信号。所以复核之后要做的不是「让检查变绿」，而是真去对一遍页面再改日期。

为什么是 6 个月：这些数据里最易变的是卫星状态与器材实测，半年一次复核与页面上「依据 XX，
YYYY-MM 核对」的承诺对得上。

---

## 7. CI：`data-sync.yml`

### 7.1 触发与 job

每周一 03:17 UTC 定时跑，也可手动触发（`workflow_dispatch`，带 `regenerate` 勾选框）。三个 job，
权限分开：

```
check（contents: read）
  └─ check-data-freshness --strict → 分类 → 写 job summary
regenerate（contents: write + pull-requests: write，仅此 job）
  └─ cargo make data-sync → data-sync-check → 开 PR（正文是新鲜度明细）
staleness（contents: read，独立于上面两个）
  └─ check-data-staleness（严格）→ 写 job summary；到期才红，不挡 regenerate
```

### 7.2 `check` job 的分类位与失败顺序

工具输出只有五种标记（`✓ ↑ ? ! ·`），工作流按标记 grep 出五个布尔位，再加一个兜底的
`crashed`：

| 标记 | grep 关键字 | 位 | 含义 | 结果 |
| --- | --- | --- | --- | --- |
| `↑` | 上游有新版本 | `upstream` | 上游真发版 | 红，要 regenerate |
| `?` | 没有来源戳 / 来源戳是补写的 | `stamped` | 没戳或补戳（摘要取自产物自身） | 红，要 regenerate |
| `!` | 连不上上游 | `unreachable` | 出网失败 | 红，换环境 |
| `·` | 无法判断上游是否变化 | `unknown` | 派生产物没校验头 | 绿，但结论点名「不猜」 |
| — | （以上都没有且退出码非 0） | `crashed` | 编译失败 / panic / 参数错误 | 红 |

`✓`（上游无变化）与本地来源那条 `·`（`file://`，措辞是「跳过上游比对」）不产生任何布尔位——
它们不需要处置。`updates = upstream || stamped`，只有它作为 job 输出传给 `regenerate`。

后续步骤按这个顺序跑，各自的 `if` 互斥；前面的失败会挡住后面的：

1. `Fail if the freshness check itself broke`（`crashed`）→ 红
2. `Fail if upstream unreachable`（`unreachable`）→ 红
3. `Report no regeneration needed`（`updates` / `unreachable` / `crashed` 全假）→ 绿
4. `Keep the report`（`updates` 真）：把 `/tmp/freshness.txt` 作为 artifact `freshness-report` 上传
5. `Fail if regeneration is needed but not requested`（`updates` 真且没勾 `regenerate`）→ 红

`regenerate` job 取 `freshness-report` 拼 PR 正文，并只在 `needs.check.outputs.updates == 'true' &&
inputs.regenerate` 时运行。

### 7.3 为什么不直接推 main

上游改动需要人看一眼。比如某个前缀归属变了（`BB`、`BK` 在 CTY-3630 里被划归中国），这会直接影响
日志、奖状、稀有度等功能的结果。机械判断只能说明「变了」，说明不了「变得对」。

### 7.4 为什么区分「有更新 / 戳不可用 / 不可达 / 不知道 / 崩溃」

- **不可达 ≠ 没更新**：默认对连不上的源只报告、退出码 0；定时任务若据此判「没问题」，就把网络
  故障当成了「上游没变」——沉默的假好消息。所以 CI 用 `--strict`。
- **补戳 ≠ 上游发版**：两者都要重新生成，但性质完全不同。`check` 分开输出 `upstream` / `stamped`，
  summary 与失败提示按实际原因说话：既不会把补戳说成「上游有新版本」，也不会把「没有戳」说成
  「补写的」。
- **不知道 ≠ 可信**：派生产物没校验头时工具退出码 0，这类不判红（不是故障），但也绝不写成
  「戳可信」——结论里单列一条「有产物的新旧无法判断」。把「不知道」混进绿灯，正是这套检查最想
  避免的假好消息。

几种分类已用一个假 `cargo` 在本地逐一演练过（clean / updates / unreachable / 补戳 / 无法判断 /
崩溃 / 参数错误）。

### 7.5 混合情况与补戳期间

- **混合**：一个源有新版本、另一个连不上时 `updates` 与 `unreachable` 同时为真；`check` 因不可达
  变红，`regenerate` **不会**运行（勾了也一样）。这是有意的：上游拉不到时重新生成同样会失败。
- **补戳期间**（仅当仓库里**存在**补戳产物时；现状没有，见 §11 第 5 条）：
  - **定时任务会一直红**。`regenerate` 只有 `workflow_dispatch` 有，`schedule` 触发时恒为 false，
    所以每周都会落到「需要重新生成但没勾 regenerate → 失败」。要消掉只能手动 dispatch 一次
    （勾 `regenerate`）并把 PR 合掉；不想每周收红灯就先注释掉 `schedule`。
  - **「上游不可达」检测是空转**。补戳在联网之前就短路（§4.2），所以这个阶段的红与绿跟出网能力
    无关，别拿它当作网络已通的证据。

### 7.6 开 PR 失败

`Open pull request` 是 `continue-on-error`，失败时下一步 `Explain pull request failure` 把出路写成
annotation，并给出人工开 PR 的 compare 链接——此时分支已经推上去了，不用重跑十分钟的数据重生成。

---

## 8. 仓库设置与 `check.yml` 的跳过规则

**一次性配置**：

- Settings → Actions → General → 勾选「Allow GitHub Actions to create and approve pull requests」。
- 若分支保护把 `Check / docker`、`Check / e2e` 列为 required，未配 `DATA_SYNC_TOKEN` 时
  `check.yml` 根本不触发、这两个 check 不会上报，PR 会停在「Expected — Waiting for status to be
  reported」。想让数据 PR 自动通过，就得把这两个 job 排除在 required 之外（合并到 main 的 push
  仍会补跑一遍）。

**`check.yml` 的 `scope` job**：默认 `GITHUB_TOKEN` 开出的 PR 不会触发本仓库的 `check.yml`
（GitHub 防递归规定）；走 `DATA_SYNC_TOKEN`（PAT，`contents` + `pull-requests` 写权限）会触发，
但对同时满足下面两条的 PR 主动跳过 `docker` / `e2e`：

1. 分支名是 `chore/data-sync`，且 head repo 是本仓库（fork 起个同名分支白蹭不到）；
2. `git diff` 确认改动**全部**落在数据产物路径下：三个文件按**整串**匹配（`crates/core/data/dxcc.txt`、
   `public/dxcc-entities.bin`、`public/cn-provinces.bin`），两个目录按前缀匹配（`public/questions/`、
   `data/provenance/`）。整串匹配是有意的：`public/cn-provinces.bin.js`、`dxcc.txt.bak` 这类「前缀相同」
   的路径不算数据产物，会按普通 PR 全跑。

只认分支名是不够的：本仓库的协作者也能建一个同名分支塞代码（甚至改 `check.yml` 把门禁删掉），
那样 PR 上一个门禁都不跑。内容不符就按普通 PR 全跑——门禁只允许往严格的方向失效。跳过的两个 job
在 PR 上显示为 Skipped，不阻塞合并。

**`check` job 永远照跑**：`dxcc.txt` 被 `include_str!` 进 `ham-web-core`、被单测直接消费，而
`data-sync-check` 只验来源戳与依赖摘要，验不出「数据写坏了、解析不出来」。

**报错 `GitHub Actions is not permitted to create or approve pull requests.`**：是上面那条仓库设置
没勾（组织仓库还可能是组织层策略锁住），不是 workflow 写错了。两条出路二选一：

1. 勾上 Settings → Actions → General 的「Allow GitHub Actions to create and approve pull requests」；
2. 配 `DATA_SYNC_TOKEN`——这条限制只针对 `GITHUB_TOKEN`，走 PAT 不受约束。

---

## 9. 验收

### 9.1 自动门禁

```bash
cargo make check   # 含 data-sync-check 与 data-staleness（都离线；后者到期只告警）
cargo make ci      # 同上 + release 前端构建
```

两个离线检查的分工见 §6。

### 9.2 关键回归测试

| 测试 | 钉住的规范 |
| --- | --- |
| `compare_matrix`、`verbatim_judgement_follows_the_upstream_digest` | `dxcc.txt` 按摘要判新旧 |
| `derived_judgement_uses_the_conditional_probe`、`derived_target_is_never_verbatim` | 派生产物只认校验头，不许拿摘要比 |
| `derived_without_validators_is_unknown_and_stays_offline` | 没校验头就说不知道，且不联网 |
| `multi_file_source_probes_every_recorded_file`、`changed_file_wins_over_an_unreachable_one`、`multi_file_source_without_validators_is_unknown` | 题库逐文件探活与优先级 |
| `backfilled_stamps_are_marked_and_never_compared_with_upstream` | 补戳短路、绝不联网、幂等 |
| `no_stamp_and_local_sources_never_touch_the_network` | 没戳 / 本地来源不联网 |
| `regenerating_dxcc_with_unchanged_data_does_not_stale_the_bin` | `inputs` 摘要只算数据行 |
| `explanations_change_is_cleared_by_refreshing_the_sidecar` | `explanations-apply` 刷新边车 |
| `conditional_head_falls_back_to_last_modified_and_never_sends_both` | `If-None-Match` 优先（RFC 9110 §13.1.3） |
| `probe_reports_http_errors_instead_of_claiming_unchanged` | 4xx 不当成「没变」 |
| `sidecar_path_mirrors_under_data_provenance`、`file_stamps_round_trip_through_the_sidecar` | 边车位置与逐文件戳可往返 |
| `months_are_counted_in_calendar_months`、`staleness_is_strictly_over_the_limit`、`age_separates_expiry_from_broken_data`（core） | 月龄口径、「严格超过阈值才算过期」，以及到期与数据错误（读不出来 / 未来）的区分 |
| `malformed_dates_are_not_guessed`（core） | 日期读不出来就判失败，不当新鲜 |
| `every_registered_entry_has_a_usable_date_and_unique_key`、`file_backed_entries_are_registered`（core） | 登记项齐全、key 唯一、日期来自数据本身 |
| `file_backed_keys_point_at_the_place_to_edit`（tools） | 过期提示要指到「改哪个文件 / 常量」 |

### 9.3 更接近真实的验收（需要出网）

单测里的联网路径跑在本地桩服务器上（绕开代理变量），只能证明协议行为。真正的验收要看一次
真实上游的运行——**引入新数据源或给旧产物补戳之后**尤其要做：

1. `workflow_dispatch` 勾 `regenerate` 跑一次，确认 PR 里每个产物都带真戳（`dxcc.txt` 头有
   `# 上游版本:`，`.bin` 边车有 `etag` 或 `last_modified`，题库边车有 `files`）。
2. 合 PR 后再跑一次定时任务 / 手动触发，确认 `check` 变绿且 summary 里没有「无法判断」。
3. 若上游不给校验头，确认报告是「无法判断」而不是「没更新」。

本机的快速自检：`cargo make data-freshness`。2026-10-26 的一次结果见 §11 第 3 条。

---

## 10. 新增一个数据源的清单

1. 生成器写戳（文本头或边车），`version` / 校验头能取到就记，取不到别猜。
2. `data_sync.rs` 的 `TARGETS` 加条目：`name` / `artifact` / `url` / `fix` / `relation`
   （`Verbatim` 还是 `Derived`；多文件来源用 `files`）。
3. `data_sync::backfill` 的产物列表（要支持 `data-stamp` 时）。
4. `Makefile.toml` 加 task，并挂进 `data-sync` 的依赖链（这样 `regenerate` 会自动带上它）。
5. `check.yml` 的 `scope` 白名单加它的产物路径，否则数据 PR 只是退回「全跑」——多等一轮，不会漏门禁。
6. 本规范 §2.3 / §2.4 / §4 同步更新，并补一条判定单测。

`data-sync.yml` **不用改**：它按输出标记分类，不认具体数据源。

**反过来的情况**：新数据其实是人工转录（C 桶，上游是 HTML / 法规页）——那就**别**进 `TARGETS`：
把核对日期写进它自己的数据文件或常量，再登记到 `ham_web_core::staleness::curated()`，并在
`data_sync::fix_hint` 里写明「改哪里」。理由见 §2.4。

---

## 11. 已知限制与盲区

1. **上游版本号不一定拿得到**。`dxcc.txt` 的期号靠 cty.csv 里的 `=VER…` 标记提取（`dxcc.rs`）；
   取不到时戳里就没有 `version`，只能靠摘要比对。
2. **派生产物的新鲜度只能靠校验头**。上游不给 `ETag` / `Last-Modified` 时只能报「无法判断」；
   这时跑一次 `cargo make dxcc-map` / `dataset` 把校验头记进戳即可。
3. **联网路径只在 raw.githubusercontent.com 上打过真实上游**。本地桩服务器验证了 `ETag` →
   `If-None-Match`、只有 `Last-Modified` → `If-Modified-Since`、两者都有时只发 `If-None-Match`、
   `304` / `200` / 4xx 的区分，以及下载时校验头被原样存进戳。真实上游方面：2026-10-26 在本机对
   raw.githubusercontent.com 跑 `cargo make data-freshness`，`dxcc-map` 与 `cn-map` 的戳里记着
   `ETag`，条件 `HEAD` 回 `304`，判为「上游无变化」。**仍未验证**：country-files.com 一侧——同一
   次运行里 `dxcc` 的整份下载以 TLS 错误失败（`received corrupt message of type
   InvalidContentType`）。本机没有设置任何代理变量，`curl` 直连同一地址也在握手阶段失败
   （`tlsv1 alert protocol version`），所以问题出在该站点与本机客户端的 TLS 协商，不在工具；
   CI 的 runner 上是否复现要看实际运行。按设计这种情况只报「连不上」而不判新旧。
4. **`dxcc.txt` 的期号有两套，别混**。戳里记的是 cty.csv 自带的 `VER20261017`（头部 `# 上游版本:`），
   `CTY-3631` / `CTY-3632` 是 AD1C 的发布编号，文件里不会出现；所以「落后上游几期」只能按
   `VER…` 日期核对，不能按发布编号。
5. **补戳是一次性的迁移手段，现状里没有补戳产物**。`data-stamp` 与 `backfilled` 标记仍在（工具与
   单测都保留，用于引入戳之前的旧产物）；四个产物现在都带真戳（`dxcc.txt` 头有 `# 上游版本:`，
   两份 `.bin` 的边车有 `etag`，题库边车有 `files`）。以后若再给**新增**的产物补戳，
   在第一次联网重新生成前，`data-freshness` 会对它报「来源戳是补写的」（退出码 1）——那是
   预期行为，不是故障。
6. **数据同步 PR 少两道门禁**。只跳过 `docker` / `e2e`（`check` 照跑），镜像与浏览器回归要等合并后
   push 到 main 才补跑一次。
7. **C 桶的检查只判年龄**。`data-staleness` 说的是「该回去看了」，说不了「上游真的变了」——复核完
   要真去对一遍页面再改日期；只改日期不动内容，等于把红灯擦掉。
8. **D 桶（vendored）不自动同步**。KaTeX 版本记在 `public/vendor/katex/VERSION`；字体只有来源与
   许可证（上游按 commit 发布，没有稳定版本号）。升级走人工 PR，并且要过 `size-check` 与公式页 e2e。

---

## 12. 待决：`BR` 前缀缺失

`BR`（官方明确的业余中继台前缀）不在 cty.csv 里，`BR4XX` 因此显示「非中国呼号」。
三个可选方案：**A** 等上游补上（不引入分叉）；**B** 在 `ham-web-tools dxcc` 生成脚本里加补丁规则
（与上游产生差异，需写明理由）；**C** 只在呼号知识页的解析层兜底（影响面仅限本页，
但「DXCC 实体」行仍显示未识别）。机械上现在有了检测手段：上游补上 `BR` 后，`data-freshness`
会报有更新。
