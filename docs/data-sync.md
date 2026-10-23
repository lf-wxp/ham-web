# 外部数据的同步机制

> **这份文档解决什么**：项目里有三份数据是从外部拉来的，它们会不定期更新。这份文档说明
> 「数据从哪来、怎么更新、怎么发现过期、什么能自动跑、什么必须人看」。

---

## 1. 三个数据源

| 数据 | 产物 | 上游 | 默认地址 |
| --- | --- | --- | --- |
| DXCC 前缀表 | `crates/core/data/dxcc.txt` | AD1C 的 cty.csv | `https://www.country-files.com/bigcty/cty.csv` |
| DXCC 实体边界 | `public/dxcc-entities.bin` | Natural Earth 1:110m | `.../ne_110m_admin_0_countries.geojson` |
| 题库 | `public/questions/{A,B,C,full}.json` | 第三方 CSV 仓库 | `.../crac-amateur-radio-exam-questions-2025-csv/main` |

**它们的区别很重要**：

- `dxcc.txt` 是上游文件的**逐行转写**，所以「上游内容摘要」可以直接和产物比对。
- `.bin` 和题库 JSON 是**派生**的（几何被简化量化、CSV 被编译成题目），上游摘要与产物摘要
  没有可比性，只能靠 HTTP 的 `ETag` / `Last-Modified` 判断上游动没动。

这个区别决定了检查逻辑：拿派生产物的摘要去比上游，会永远报「有更新」——这是实现时踩过的坑，
`data_sync.rs` 里有回归测试钉住。

---

## 2. 来源戳（provenance）

每个产物都记一份「我是从哪来的」：

**文本产物**（`dxcc.txt`）写在文件头注释里：

```
# 由 `ham-web-tools dxcc` 从 https://www.country-files.com/bigcty/cty.csv 生成，请勿手工编辑。
# 摘要: sha256:c758e804…
# 抓取: 2026-10-22T08:28:59Z
# 上游版本: VER20261017
# 输入: crates/core/data/dxcc.txt sha256:…
```

**二进制 / JSON 产物**写边车 `data/provenance/<产物相对路径>.source.json`
（例如 `public/questions/config.json` → `data/provenance/public/questions/config.json.source.json`）。

**边车刻意不放在 `public/` 里**，理由见 §2.1。

字段含义：

| 字段 | 作用 |
| --- | --- |
| `source` | 实际抓取地址；离线 `--input` 时是 `file://<文件名>`（据此跳过联网比对）。**只记文件名、不记绝对路径**：戳会入库，绝对路径会泄露本机用户名 |
| `digest` | 上游原始字节的 sha256。题库由多份 CSV + 图片编译而成，没有单一上游文件，这里是本次读过的全部上游文件的**合并摘要**（路径排序后的「路径 + 各自摘要」再取摘要） |
| `fetched_at` | 抓取时间（UTC） |
| `version` | 上游期号，如 `VER20261017` / `CTY-3632`；**取不到就不写**，不猜 |
| `etag` / `last_modified` | 下载时服务端给的校验头；`data-freshness` 对派生产物据此发条件 `HEAD` |
| `inputs` | **仓库内**输入：相对路径 → 摘要（口径见下） |
| `backfilled` | `stamp-data` 补写的戳（见 §3）；摘要是产物自身的，不是上游的 |

`inputs` 是关键。它解决了一个原本看不见的问题：

> `dxcc-entities.bin` 的实体名来自 `dxcc.txt`。`dxcc.txt` 一变，`.bin` 就该重建，
> 但两者之间原本没有任何记录。现在 `.bin` 的边车里记着它依据的 `dxcc.txt` 摘要。

题库同理，记着 `data/explanations.json` 的摘要——解析改了却忘了重建题库，会被抓出来。

**输入摘要的口径**（`provenance::input_digest`）：`.txt` 数据文件只摘**数据行**，跳过 `#` 注释头；
其余文件按原始字节。原因：`dxcc.txt` 的注释头里有抓取时间，上游没变、只是重跑了一遍
`cargo make dxcc`，文件也会变；摘要若把注释头算进去，依赖它的 `.bin` 就会被误判过期。


### 2.1 为什么边车放 `data/provenance/` 而不是产物旁边

`public/questions/` 在 `index.html` 里是整目录 `copy-dir` 进产物的。边车如果放在
`public/questions/config.json.source.json`：

1. 会被发布到线上（构建元数据本不该是发布物）；
2. 会进入 Service Worker 的预缓存清单（`postbuild.rs` 的 `is_excluded` 原本没有排除它）；
3. 后果最重的是——戳里的 `fetched_at` 是抓取时刻的墙上时间，**每次重新生成都会变**，
   于是内容没变也会改掉 `__CACHE_VERSION__`，让所有用户重下整个预缓存。

实测（2026-10-22，注入边车后跑 `postbuild`）：

```
无 source.json：Generated sw.js (version ccbce065d2586c40, 20 precached files)
有 source.json：Generated sw.js (version 1eaabfb3fa43c61e, 21 precached files)
```

现在改了两处，互为表里：

- **主修**：边车落在 `data/provenance/`，`data/` 不进 `public/`，也就不进产物。
- **兜底**：`is_excluded` 增加 `rel.ends_with(".source.json")`，即使将来有人把它放回
  `public/`，也不会污染预缓存。已验证：强行注入后仍是 20 个文件、版本号不变。

---

## 3. 四个命令

| 命令 | 联网 | 作用 |
| --- | --- | --- |
| `cargo make data-sync` | 要 | 三个数据源全部重新生成 |
| `cargo make data-sync-check` | 不要 | 戳齐不齐、依赖摘要对不对（已在 `cargo make check` / `ci` 里） |
| `cargo make data-freshness` | 要 | 比对上游有无新版；连不上只报告 |
| `cargo make data-stamp` | 不要 | 给旧产物补戳，**内容不动** |

### 改了解析库之后

`data/explanations.json` 是题库边车的输入。改了它之后的离线流程是：

```bash
cargo make explanations-apply   # 把解析写回 public/questions/*.json（离线）
```

`explanations-apply` 在四个题库都写完后，会把边车里记的解析库摘要刷新到当前值
（`provenance::refresh_inputs`），所以随后的 `cargo make check` 不会再要求联网重跑
`cargo make dataset`。只有四个题库文件缺了一个时才不刷新——那时「题库与解析库一致」并不成立。

### 什么时候用 `data-stamp`

产物是在引入来源戳之前生成的（`dxcc.txt` 就是这种情况）。此时拿不到当初那份 `cty.csv`，
**不能编一个假的「上游摘要」**——那会让新鲜度检查永远认为已最新。

`data-stamp` 记的是「既有产物自身的摘要」，并写上 `backfilled` 标记（文本头里是 `# 补戳: 是`）。
新鲜度检查见到这个标记，**不去比摘要，也不联网**，直接报「来源戳是补写的，需要重新生成」：
补戳产物的摘要不是上游的，比了只会误报「有更新」。下次真正执行 `cargo make dxcc` /
`dxcc-map` / `dataset` 时会写入真戳，标记随之消失。

---

## 4. 自动同步：`data-sync.yml`

每周一 03:17 UTC 跑一次，也可手动触发。两个 job，权限分开：

```
check（只读）
  └─ 检查上游 → ┬─ 无更新          → 结束（绿）
                ├─ 上游不可达      → 失败（红），提示换环境
                ├─ 检查工具自己崩溃 → 失败（红）
                └─ 有更新          → ┬─ 没勾 regenerate → 失败（黄），提示再跑一次
                                      └─ 勾了 regenerate → 交给 regenerate job
regenerate（contents + pull-requests 写权限，仅此 job 有）
  └─ cargo make data-sync → data-sync-check → 开 PR（正文是新鲜度明细）
```

**为什么不直接推 main**：上游改动需要人看一眼。比如某个前缀归属变了（`BB`、`BK` 在
CTY-3630 里被划归中国），这会直接影响日志、奖状、稀有度等功能的结果。

**为什么区分「不可达」「无更新」「工具崩溃」**：`check-data-freshness` 默认对连不上的数据源
只报告、退出码 0，定时任务若据此判定「没问题」，就会把网络故障当成「上游没变」——沉默的假好消息。
所以工作流用 `--strict` 跑，并按输出里的标记分类；**退出码非 0 却既没有「有更新」标记也没有
「连不上」标记**，说明是编译失败、panic、参数错误这类工具自身的问题，一律判红，不会落进
「无更新」。三种分类已用一个假 `cargo` 在本地逐一演练过（clean / updates / unreachable /
补戳 / 崩溃 / 参数错误）。

**仓库设置前置条件**（workflow 文件头里也写了）：

- Settings → Actions → General → 勾选「Allow GitHub Actions to create and approve pull requests」。
- 用默认 `GITHUB_TOKEN` 开出的 PR **不会触发** `check.yml`（GitHub 防递归的规定）。要让 PR 上自动跑检查，
  建一个有 `contents` + `pull-requests` 写权限的 PAT，存成仓库密钥 `DATA_SYNC_TOKEN`；没配时退回
  `GITHUB_TOKEN`，此时 PR 上要手动重跑检查。

**报错 `GitHub Actions is not permitted to create or approve pull requests.`**：这是上面那条仓库设置
没勾（组织仓库还可能是组织层的策略锁住），不是 workflow 写错了。两条出路二选一：

1. 勾上 Settings → Actions → General 的「Allow GitHub Actions to create and approve pull requests」；
2. 配 `DATA_SYNC_TOKEN`（PAT，`contents` + `pull-requests` 写权限）——这条限制只针对 `GITHUB_TOKEN`，
   走 PAT 不受约束，副作用是 PR 上的 `check.yml` 也会自动跑起来。

`regenerate` job 里 `Open pull request` 是 `continue-on-error`，失败时由下一步
`Explain pull request failure` 把上面两条出路写成 annotation 并给出
`https://github.com/<repo>/compare/<base>...chore/data-sync` 的人工开 PR 链接——此时分支已经推上去了，
不用重跑十分钟的数据重生成。

---

## 5. 网络受限时的行为

本机实测（2026-10-22），这台机器**拉不到**这三份数据：

| 路径 | 结果 |
| --- | --- |
| 系统 `curl` | TLS 被拒（LibreSSL 3.3.6 过旧） |
| Python `ssl` | 缺 OpenSSL 1.1，import 失败 |
| Node `fetch` | `wrong version number` |
| `ureq`（仓库自己用的） | `corrupt message of type InvalidContentType` |
| HTTP | 公司代理返回 502 |

所以：

- **本地**只能跑 `data-sync-check` 和 `data-stamp`（都不联网）。
- **`data-freshness` / `data-sync` 必须换到能出网的环境**（CI runner 或另一台机器）。
- 换环境后第一次跑，请先确认 `check-data-freshness` 能真的取到上游摘要，再依赖定时任务。

代码没有为此做特殊处理，也不需要：`ureq` 的报错会原样显示，`data-freshness` 会把它归为
「不可达」。

---

## 6. 已知限制

1. **上游版本号不一定拿得到**。`dxcc.txt` 的期号靠 cty.csv 里的 `=VER…` 标记提取；取不到时
   戳里就没有 `version` 字段，只能靠摘要比对。**不猜**。
2. **派生产物的新鲜度只能靠校验头**。上游不给 `ETag` / `Last-Modified` 时，检查会如实报
   「无法判断」，而不是假装知道。
3. **联网路径只在本地桩服务器上验证过，没有打过真实上游**（本机连不上，见 §5）。已验证的部分：
   `ETag` → `If-None-Match`、只有 `Last-Modified` → `If-Modified-Since`、两者都有时只发
   `If-None-Match`（RFC 9110 §13.1.3）、`304` / `200` / 4xx 的区分，以及下载时校验头被原样存进戳。
   这些测试起一个只接一次连接的本地 HTTP 桩，并绕开环境里的代理变量。没有验证的部分：真实的
   raw.githubusercontent.com 与 country-files.com 是否真的给出这些校验头——这只能等 CI 第一次跑。
4. **本地 `dxcc.txt` 落后上游一期**（生成于 CTY-3631，上游最新 CTY-3632）。3632 与中国前缀
   无关，所以不影响 `BR` 的结论；等能联网时重新生成即可。
5. **当前三个产物都是补戳**。在第一次联网重新生成之前，`data-freshness` 对它们一律报「来源戳是补写的」
   （退出码 1）。这是预期行为，不是故障：它在如实说「我不知道这些产物对应上游的哪一期」。

---

## 7. 待决：`BR` 前缀缺失

`BR`（官方明确的业余中继台前缀）不在 cty.csv 里，`BR4XX` 因此显示「非中国呼号」。
三个可选方案：**A** 等上游补上（不引入分叉）；**B** 在 `ham-web-tools dxcc` 生成脚本里加补丁规则
（与上游产生差异，需写明理由）；**C** 只在呼号知识页的解析层兜底（影响面仅限本页，
但「DXCC 实体」行仍显示未识别）。机械上现在有了检测手段：上游补上 `BR` 后，
`data-freshness` 会报有更新。
