# README 审核与整改记录

> 审核日期：2026-10-23　｜　对象：仓库根目录 [`README.md`](../README.md)（当时 1179 行）
> 本文是这次审核的**追踪表**：每条问题的证据、处置方式与状态都写在这里。
> 后续再发现 README 与代码不一致，先补一条记录，再改文档；机械可查的项已交给
> `cargo make readme-check`（见文末「门禁」）。

## 1. 审核方法

1. 通读 README 全文，按「事实 / 结构 / 缺失 / 可维护性」四类记录问题；
2. 每条结论都在代码里核实证据（`Makefile.toml`、`crates/*`、`.github/workflows/`、`docs/*`）；
3. 修完后用 `cargo make readme-check` 与 `cargo make check` 复核，机械项不再靠人记。

## 2. 问题清单

状态：`[x]` 已修复　`[~]` 部分修复（留了后续项）　`[ ]` 未修（附理由）

### P0 事实性错误（读者会照着做错）

| # | 问题 | 证据 | 处置 | 状态 |
|---|---|---|---|---|
| P0-1 | Worker 数量写成「十一个」，实际 12 个（漏了 `afsk1200-worker`） | `ls crates/*-worker` = 12；改前有 6 处写成「十一个」 | 全部改为「十二个」；新增 `readme-check` 断言数量 | [x] |
| P0-2 | `build-*-worker` 任务清单漏 `build-afsk1200-worker` | `Makefile.toml` 有 `[tasks.build-afsk1200-worker]` | 任务表与常见问题都补上；`readme-check` 断言「Makefile 里每个 `build*-worker` 都在 README 出现」 | [x] |
| P0-3 | `cargo make check` 的构成过期（漏 5 项守卫） | `Makefile.toml` 的 `[tasks.check]` 依赖含 `i18n-pack`、`knowledge-i18n-check`、`explanations-check`、`check-ui`、`check-structure`、`data-sync-check`、`data-staleness` | 任务表与「代码规范」按 `check` / `ci` 的真实依赖重写 | [x] |
| P0-4 | 备份入口自相矛盾：一处说已迁到 `/backup`，另一处仍写「`/tools` 页底部」 | `crates/app/src/pages/tools/tools_page.rs` 里 `#backup` 只做重定向 | 统一为 `/backup`（顶栏设置 / 移动菜单 / 页脚），并删掉重复列出的「数据备份」特性 | [x] |
| P0-5 | 项目结构树缺一整层目录 | 缺 `docs/`、`scripts/`、`.github/`、`LICENSES/`、`AGENTS.md`、`perf-budget.json`、`crates/{hell,wefax,dsp,wav,afsk1200-worker}`、`public/{hell,wefax,afsk1200}-worker`、`public/vendor`、`public/data` | 结构树重写 | [x] |

### P1 结构与可读性

| # | 问题 | 证据 | 处置 | 状态 |
|---|---|---|---|---|
| P1-1 | 手写「目录」51 条，已漏 8 个标题（开放 API、已知的失败与本地排障、各类贡献的入口…） | 目录 vs `grep -nE '^#{2,4} '` 比对 | **删掉手写目录**，改用 GitHub 大纲；顶部只留 4 个入口链接 | [x] |
| P1-2 | 「主要特性」157 行、多条单行 400–1400 字符，GitHub 上几乎不可读 | 旧 README `:157`、`:228` | 全文搬到 [`docs/features.md`](features.md)，README 改为按分组的「功能一览」（每条 1–2 行） | [x] |
| P1-3 | 「使用示例」与主要特性重复，且样例命令任务表里已有 | 旧 README `:420`–`:461` | 折叠进 `docs/features.md` 的「上手路径」；任务表已覆盖样本命令 | [x] |
| P1-4 | 快速开始排在第 384 行，前面压着 150 行特性与整张 API 表 | 旧 README 章节顺序 | 重排为「简介 → 快速开始 → 功能一览 → 文档地图 → 技术栈 → …」 | [x] |
| P1-5 | 参考型表格（本地存储 key 60+ 行、路由表 180 行）挤在 README 里 | 旧 README `:852`–`:901`、`:1054`–`:1074` | 搬到 [`docs/storage.md`](storage.md) 与 [`docs/routes.md`](routes.md)（含带参数页面表），README 各留一段说明 + 链接 | [x] |
| P1-6 | 没有徽章、没有图片，demo 链接藏在一行正文里 | 旧 README `:15` | 加 CI / 许可徽章，顶部一行放 demo 与四个入口链接 | [x] |
| P1-8 | 拆完仍是 973 行：任务表 78 行、端到端测试 97 行、项目结构 64 行、解析维护流程 165 行都压在 README 里 | `python3` 按标题统计各节行数（见下方「结构变更」） | README 只留摘要与索引（302 行），逐节内容搬进 `docs/architecture.md`、`tasks.md`、`deployment.md`、`testing.md`、`data-pipeline.md`、`explanations.md`、`development.md`、`api.md`、`pwa.md`、`licensing.md` | [x] |
| P1-7 | 缺少界面截图 | 仓库无截图资产 | 用 Playwright 截本地 release 构建的真实页面（首页浅色 / 深色、闯关地图、手机首页），放 `docs/images/` 并嵌在简介下方；生成脚本 `e2e/scripts/screenshots.mjs` 保证可重拍 | [x] |

### P2 缺失内容

| # | 问题 | 证据 | 处置 | 状态 |
|---|---|---|---|---|
| P2-1 | 没有文档地图；`AGENTS.md`、`docs/i18n.md`、`docs/data-sync.md`、`docs/code-quality.md`、`docs/hardware-verification.md` 从未被 README 链接 | 旧 README 的 `docs/` 链接只有 5 个文件 | 新增「文档地图」表（12 行），并指回 `AGENTS.md` 作为总入口 | [x] |
| P2-2 | 环境要求未提 e2e 需要的 Node | `e2e/package.json`、`.github/workflows/check.yml` 用 Node 22 | 环境要求补 Node ≥ 22（注明仅 e2e 需要），构建链仍不需要 Node | [x] |
| P2-3 | 浏览器能力边界散落在长篇特性句里 | Web Serial / WebGL2 / PushManager / Service Worker 各自的降级逻辑 | 新增「浏览器支持」表（6 行） | [x] |
| P2-4 | `e2e/` 同时存在 `package-lock.json` 与 `pnpm-lock.yaml`，README 只写 `npm ci` | CI 用 `npm ci`，全仓无 pnpm 引用 | 删除未被引用的 `e2e/pnpm-lock.yaml`；README 口径保持 `npm ci` | [x] |
| P2-5 | `data-sync.yml` 在 README 里没有介绍 | `.github/workflows/data-sync.yml`（周计划 + 手动触发 + 开 PR） | 「代码规范 → CI」段落补一句并链到 `docs/data-sync.md` | [x] |
| P2-6 | 界面已三语，README 只有中文 | — | 未修：英文版需要整篇翻译并长期维护，收益与成本需再评估；先从功能页三语入手 | [ ] |

### P3 可维护性（防再次漂移）

| # | 问题 | 处置 | 状态 |
|---|---|---|---|
| P3-1 | README 里的数量、任务清单全靠手写，改代码后必然漂移 | 新增 `cargo make readme-check`（`crates/tools/src/readme_check.rs`），对账 7 条，并挂进 `cargo make check` 与 `cargo make ci` | [x] |

## 3. 门禁：`cargo make readme-check`

`ham-web-tools check-readme` 现在对账七条（任一不符即失败）：

1. README 里每处「N 个 Worker」= `crates/*-worker` 的目录数（支持中文数字，1–99）；
2. `Makefile.toml` 里每个 `build*-worker` 任务都在 README 出现过；
3. README 的「共 N 个路径」= `crates/core/src/registry.rs` 的 `path:` 条数；
4. README 的「N 项计算器」= `crates/app/src/pages/tools/tools_page.rs` 里 `TOOL_NAV` 的条目数（页面自己也把这份清单当唯一来源）；
5. `docs/*.md` 每个文件至少被 README 或 `AGENTS.md` 链接一次（不留孤儿文档）；
6. `docs/*.md`、`ROADMAP.md` 与 `Makefile.toml` 里以「README」+ 中文书名号引用的章节名，在 README 仍能找到这段文字（改标题时不会静默断链）；
7. README 引用的 `docs/images/*` 截图真实存在（重拍改了文件名、或临时删图排查后忘了恢复，渲染出来只有一个碎图标）。

选这几条的理由：本次审查踩中的三处漂移（P0-1、P0-2、P0-3）全部属于这一类「手写摘要 vs 代码」，
而它们恰好都能被机械判定；第 4 条是同一类风险的下一个候选（加一个计算器就要改 README 的「60 项」）。
运行成本只有几次文本读取。反例验证过：把 README 改成「61 项计算器」会直接报错并指出实际条目数。

## 4. 本次结构变更一览

| 原来 | 现在 |
|---|---|
| `README.md` 1179 行，含手写目录 | 第一轮 973 行 → 第二轮 **302 行**：只留摘要、索引与上手命令 |
| 无截图 | 简介下方 4 张真实页面截图（`docs/images/`，由 `e2e/scripts/screenshots.mjs` 生成） |
| 「主要特性」157 行长句 | README「功能一览」8 个分组各 1–2 行；全文 → `docs/features.md` |
| 「使用示例」5 个流程 | 折进 `docs/features.md` 的「上手路径」 |
| localStorage key 表在 README | `docs/storage.md` |
| 180 条路由表在 README | `docs/routes.md`（带参数页面表也一并搬走） |
| 技术栈 + 项目结构 81 行 | 技术栈表留在 README；目录树缩成 10 行，逐目录说明 → `docs/architecture.md` |
| 后端 API 55 行 | 两段摘要 → `docs/api.md` |
| cargo make 任务 78 行 | 10 条常用 → `docs/tasks.md`（全集） |
| Docker + Web Push 92 行 | 三条命令 → `docs/deployment.md` |
| 数据集 + 新增题库版本 + 解析维护流程 199 行 | 6 行摘要 → `docs/data-pipeline.md` / `docs/explanations.md` |
| 端到端测试 97 行 | 4 条规则 → `docs/testing.md` |
| 快捷键 / 代码规范 / UI 注意 / 像素风 144 行 | 5 条要点 → `docs/development.md` |
| 许可 39 行 | 一段结论 → `docs/licensing.md` |

## 5. 未做项与后续

- **英文 README**（P2-6）：若要做，建议只维护一份 `README.en.md` 并在顶部互相链接，避免两套内容各自漂移。
- **截图的时效**：截图不进任何门禁（像素风界面大改后不会自动红），界面有明显变化时手动重拍，
  命令见 §6。四张图合计约 100 KB（webp、1440 / 390 宽，像素图压缩率高）。
- 顺带修掉的跨文档漂移：
  - `ROADMAP.md` 里 `README.md#主要特性` 锚点已失效 → 改指 `docs/features.md`；
  - `ROADMAP.md` 的功能表里「10 个解码器（9 个走 Worker）」是旧数字 → 改为 12 / 11；
  - `ROADMAP.md` 的「相关文档」表补上 `docs/features.md`、`storage.md`、`routes.md`、`code-quality.md`、`readme-review.md`；
  - `AGENTS.md` 的文档索引与 `cargo make check` 组成同步到新口径。
- **文档数量与入口**：二轮拆分后 `docs/` 有 22 份文档，唯一的导航入口是 README 的「文档地图」。
  不再有「某份文档谁也找不到」的情况由门禁第 5 条保证（README 或 `AGENTS.md` 必须链到它）。
- 本轮改动只动文档、`Makefile.toml`、`crates/tools`（新增守卫）、`e2e/scripts/screenshots.mjs`（新增截图脚本）
  与 `e2e/pnpm-lock.yaml`（删除）；页面、路由与数据产物均未改，因此 e2e 无需重跑
  （`cargo make check` 已全量通过）。

## 6. 复核命令

```bash
cargo make readme-check   # 六条对账

cargo make check          # 全量门禁（已包含 readme-check）

# 重拍 README 截图（截的是 release 产物，别对着 trunk dev 拍：字号与 wasm-opt 后的表现不同）
cargo make build-web && cargo make serve      # 另一个终端：http://127.0.0.1:8080
cd e2e && node scripts/screenshots.mjs        # 写入 docs/images/*.webp
# 也可以对已部署站点重拍：E2E_BASE_URL=https://ham.onlyxp.me node scripts/screenshots.mjs
```
