# 文档索引

> 这是 `docs/` 的**唯一**目录：[`README.md`](../README.md) 与 [`AGENTS.md`](../AGENTS.md) 只指向这里，不各自再列一遍。
> 守卫：`cargo make readme-check` 保证 `docs/` 下每份文档在本页都有一行链接；
> `cargo make check-structure` 保证文档内链和 `docs/…md §N` 引用都能解析（见[文档约定](#文档约定)）。

## 常见任务直达

只列**章节级**的跳转（具体内容只在目标文档里写，这里不重复）；找不到的任务看下面的分类表。

| 要做什么 | 去哪 |
| --- | --- |
| 改界面 | 先确定是哪套皮肤：现代风 → [`ui/modern.md`](ui/modern.md)，像素风 → [`ui/pixel.md`](ui/pixel.md)；找组件与 prop → [`crates/app/src/ui/README.md`](../crates/app/src/ui/README.md) |
| 页面骨架 / 内容宽度 / 页头 | [`ui/pixel.md`](ui/pixel.md) §4 · [`ui/modern.md`](ui/modern.md) §3 |
| 取值：圆角 / 间距 / 字号 / 状态色 | [`ui/modern.md`](ui/modern.md) §2 |
| 弹层怎么进出场、控件的悬停 / 禁用 / 错误态 | [`ui/motion.md`](ui/motion.md) §五、§六 |
| 新增路由要选哪个页面签名 | [`ui/interaction.md`](ui/interaction.md) §3 |
| 加 / 删 `md-*` 类名 | [`ui/modern-classes.md`](ui/modern-classes.md)（手工维护，漏登记会被门禁拦） |
| 没走组件的原生元素（「刻意的例外」） | [`ui/pixel.md`](ui/pixel.md) §10 |
| 改界面文案 / 译文 | [`data/i18n.md`](data/i18n.md) |
| 改完界面怎么验、e2e 红了怎么排障 | [`engineering/testing.md`](engineering/testing.md) |
| 提交前要跑什么 | [`engineering/code-quality.md`](engineering/code-quality.md) §2 |
| 新增解码器 / worker 错误码 | [`domain/decode-worker-protocol.md`](domain/decode-worker-protocol.md) |
| 新增本地存储 key | [`reference/storage.md`](reference/storage.md)（并在 `core/remote_sync.rs` 的 `every_documented_key_is_classified` 里归类） |
| 动同步（自定义远端 / 在线账号）、账号接口、数据库表 | [`domain/account-sync.md`](domain/account-sync.md) |
| 为什么这么设计 / 这条规则怎么来的 | [`ui/decisions.md`](ui/decisions.md)（含颜色 / 几何 / 字体这些视觉语言的来历，§11） |

## 文档分类

目录按**读者的问题**分，不按文档类型分。

| 目录 | 回答什么问题 | 性质 |
| --- | --- | --- |
| [`ui/`](#ui--界面两套皮肤) | 界面怎么写 | 规范 |
| [`engineering/`](#engineering--工程) | 怎么开发、测试、构建、部署；代码质量的标准是什么 | 规范 + 指南 |
| [`data/`](#data--数据与内容) | 外部数据、题库、解析、文案怎么维护 | 规范 + 流程 |
| [`reference/`](#reference--速查) | 有哪些路由 / 存储 key / 接口 / 功能 | 速查（多由代码派生，工具对账） |
| [`domain/`](#domain--领域功能) | 某个领域功能的协议、口径与验收 | 规范 |

### ui / 界面（两套皮肤）

| 文档 | 内容 | 何时看 |
| --- | --- | --- |
| [`ui/modern.md`](ui/modern.md) | **现代 UI 规范**：取值（token）、布局、组件目录、写页面的规则、无障碍、门禁、反例清单 | 改现代风界面 |
| [`ui/pixel.md`](ui/pixel.md) | **像素 UI 规范**：视觉基线、决策表（要什么 → 用哪个）、页面骨架、组件 API 约定、例外清单、自检命令与验收清单 | 改像素风界面 |
| [`ui/decisions.md`](ui/decisions.md) | **设计取舍（现行）**：为什么是两套独立 Kit、Kit 契约、允许差多远、风格开关、门禁口径、两套互不串的验证方法、参考实现、为什么不引入第三方组件库、颜色 / 几何 / 字体的来历 | 想知道「为什么这么定」 |
| [`ui/motion.md`](ui/motion.md) | **动效规范**：原则与规则清单、两套皮肤的 token、弹层进出场（「在场」实现）、控件状态矩阵 | 写动画 / 弹层 / 控件状态 |
| [`ui/interaction.md`](ui/interaction.md) | **页面级交互（现代风）**：七类动作与触发场景、换页方向、12 个页面签名、「统一 vs 个性」 | 加页面动效 / 新增路由选签名 |
| [`ui/modern-classes.md`](ui/modern-classes.md) | `md-*` 类名索引（**手工维护**，`check-structure` 精确双向校验） | 查「这个类归谁 / 还有没有人用」 |

代码旁的文档：[`crates/app/src/ui/README.md`](../crates/app/src/ui/README.md)（组件 API 清单——「有什么」；放在代码旁是为了改组件时一眼能看到）。

### engineering / 工程

| 文档 | 内容 | 何时看 |
| --- | --- | --- |
| [`engineering/architecture.md`](engineering/architecture.md) | 技术栈、完整目录树、crate 分层与依赖方向 | 想知道代码放哪 |
| [`engineering/development.md`](engineering/development.md) | 贡献者指南：代码规范、提交前检查、CI 概览 | 第一次动代码 |
| [`engineering/testing.md`](engineering/testing.md) | Playwright：断言口径、覆盖范围、运行档位、写法约定、覆盖率口径、本地排障 | 写 / 跑 / 排查 e2e |
| [`engineering/tasks.md`](engineering/tasks.md) | `cargo make` 任务全集 | 找某个任务 |
| [`engineering/deployment.md`](engineering/deployment.md) | 本地开发细节、生产构建、Docker、环境变量、Web Push | 构建与部署 |
| [`engineering/code-quality.md`](engineering/code-quality.md) | **代码质量规范**：分层边界、单一来源、**门禁一览（§2）**、问题分级与证据口径、分范围检查清单 | 提交前 / 做审查 |
| [`engineering/performance.md`](engineering/performance.md) | **性能**：测量口径（构建 / 降速 / 真 GPU / 数据量）与阈值、基线水位、规则与守卫它们的用例对照、已评估后决定不做的项、常驻缓存清单、主包体积预算与分包裁定 | 页面卡顿 / 要不要优化某处 / 加缓存 / 想拆包 |
| [`engineering/licensing.md`](engineering/licensing.md) | 许可、第三方声明、GPL 例外与部署方义务 | 引入依赖 / 字体 / 分发 |

### data / 数据与内容

| 文档 | 内容 | 何时看 |
| --- | --- | --- |
| [`data/data-sync.md`](data/data-sync.md) | 外部数据同步：来源、产物、来源戳 schema、新鲜度判定矩阵、四个命令、CI 分类位 | 改数据源 / 同步红了 |
| [`data/data-pipeline.md`](data/data-pipeline.md) | 题库数据集构建与新增题库版本 | 官方换版 |
| [`data/explanations.md`](data/explanations.md) | 题目解析的维护流程：统计、合并、增强、写入 | 批量改解析 |
| [`data/explanations-style.md`](data/explanations-style.md) | 题目解析的写作规范 | 写一条解析 |
| [`data/typography.md`](data/typography.md) | 题库题面排版（空格、全角半角）与内容指纹 | 改题库文本 |
| [`data/i18n.md`](data/i18n.md) | 界面多语言：词条规范、复数、三语门禁与验收 | 改任何界面文案 |

### reference / 速查

| 文档 | 内容 | 何时看 |
| --- | --- | --- |
| [`reference/features.md`](reference/features.md) | 完整功能清单（README「功能一览」的逐条版） | 查某个功能有没有 / 限制是什么 |
| [`reference/routes.md`](reference/routes.md) | 完整路由表与带参数页面（由注册表派生） | 查路径 |
| [`reference/storage.md`](reference/storage.md) | 本地存储 key 一览与自动同步（两条通道）的约束 | 查 / 加存储 key |
| [`reference/api.md`](reference/api.md) | 后端接口：同源代理与开放 API（`/api/v1/*`） | 对接后端 |

### domain / 领域功能

| 文档 | 内容 | 何时看 |
| --- | --- | --- |
| [`domain/decode-worker-protocol.md`](domain/decode-worker-protocol.md) | 解码 Worker 协议：统一信封、错误码表、wasm 胶水契约、新增清单 | 动解码器 |
| [`domain/hardware-verification.md`](domain/hardware-verification.md) | 真机验收：单测与假串口只算「待验」，真机记录才叫「已验证」 | 动串口 / 外部格式 |
| [`domain/antenna-coverage.md`](domain/antenna-coverage.md) | 天线方向增益 × DXCC 实体的口径与实现（口径是「方向增益」，不是覆盖 / 可达） | 动天线分析 |
| [`domain/pwa.md`](domain/pwa.md) | PWA：Service Worker 生成、缓存策略、更新提示、图标 | 动离线 / 缓存 |
| [`domain/account-sync.md`](domain/account-sync.md) | 在线账号与同步通道：与自定义远端怎么共存、基线与冲突、SQLite 表结构、安全、Docker 持久化 | 动同步 / 账号 / 部署数据卷 |

另有：[`ROADMAP.md`](../ROADMAP.md)（还没做的、刻意不做的、新增内容必须过的门禁）、[`images/`](images/)（README 截图）。

## 文档约定

1. **一件事一个家。** 每个事实只在一处写全，其它地方给一行摘要加链接。能由代码对账的数字与清单交给工具，别在文档里手数
   （README 的 Worker / 路由 / 计算器数量由 `readme-check` 对账）。同一件事要在三个文件里各写一遍时，先想清楚谁是权威、其余改成链接。
2. **规范与历史不混写。** `ui/` `engineering/` `data/` `domain/` 只写「现在该怎么做」；时序、试错、被推翻的结论不进规范 ——
   能抽成教训的写进规范的反例清单（如 `ui/modern.md` §8），「为什么这么定」写进 `ui/decisions.md`，
   其余不保留（批次流水账、改前 / 改后快照写在提交说明里）。仓库里没有 `archive/` 目录，也不再新增。
3. **目录是读者的问题。** 新文档先问「谁在什么时候来读它」，再选目录；放不进任何一个目录，多半说明它在讲两件事，该拆。
4. **命名。** 小写加连字符；目录名已表达的前缀不再重复（`ui/modern.md`，不是 `ui/ui-modern.md`）。
5. **引用写法。** 链接用相对路径；文字里引用写仓库根路径 `docs/ui/modern.md`；指向小节写 `docs/ui/modern.md §3`
   （编号小节才能被校验：门禁会检查被引用的文件里真有这一节，代码注释里的 `docs/…md §N` 同样会被检查）。
6. **新增 / 移动 / 删除文档：** 放对目录 → 在本页加（改、删）一行 → 用 `git mv` 移动 → 跑 `cargo make readme-check` 与 `cargo make check-structure`。
7. **改代码的同一次提交里改文档。** 下表是谁维护什么：

| 文档 | 怎么维护 | 什么时候改 |
| --- | --- | --- |
| `ui/modern.md` `ui/pixel.md` | 手写 | 改了取值、组件或写法规则时 |
| `ui/modern-classes.md` | 手写（没有生成命令） | 往 `style/modern/*.css` 加 / 删 `md-*` 类名后；漏登记或留死条目都会被 `check-structure` 拦住 |
| `ui/decisions.md` | 手写 | 只有「设计取舍变了」才动（换方案、改边界、改门禁口径） |
| `crates/app/src/ui/README.md` | 手写 | 组件增减或 prop 变化时 |
| `reference/routes.md` | 随注册表（`crates/core/src/registry.rs`）更新 | 新增 / 改路由；数量由 `readme-check` 对账 |
| `reference/features.md` | 手写，与 README「功能一览」成对 | 改功能时两处一起改 |
| `reference/storage.md` | 手写 | 新增 / 改本地存储 key |

## 守卫对照

文档里的哪些断言是机器在看的（其余靠人读）：

| 守卫 | 保证什么 |
| --- | --- |
| `cargo make readme-check` | README 的 Worker / 路由 / 计算器数量与任务清单；`docs/` 每份文档在本页有链接；README 与 AGENTS 指向本页；`README「章节」` 引用仍找得到；截图存在 |
| `cargo make check-structure`（文档内链） | 所有相对链接可解析；`docs/…md §N` 的 N 在目标文件里存在（只证明「这一节存在」，证明不了「这一节讲的就是你要的」） |
| `cargo make check-structure`（类名索引） | `ui/modern-classes.md` 与 `style/modern/*.css` 精确双向一致 |
| `cargo make check-ui` | `ui/pixel.md` §11 里「应为 0」的机械条目 |
| `cargo make i18n-check` · `knowledge-i18n-check` | 三语词条齐全；知识库译文 key 不漂移（[`data/i18n.md`](data/i18n.md)） |
| `cargo make data-sync-check` · `data-staleness` | 数据产物来源戳一致；人工转录的事实多久没复核（[`data/data-sync.md`](data/data-sync.md)） |

完整的门禁清单与「失败意味着什么」见 [`engineering/code-quality.md`](engineering/code-quality.md) §2。

## 术语

| 术语 | 含义 |
| --- | --- |
| 现代风 / 像素风 | 两套并存的界面皮肤。现代风 = `md-*` 类 + `--md-*` 变量 + `kit_modern/` + `style/modern/`；像素风 = `pxl-*` 与工具类 + 无前缀变量 + `components/` + `style/pixel/`。两者不共享类名与 token，唯一共享的是 `crates/app/src/ui/` 里按风格分叉的那层判断（[`ui/decisions.md`](ui/decisions.md) §3） |
| Kit | 一套皮肤的组件集合（`kit_modern/` 与 `components/`），各自对外满足同一份「Kit 契约」（[`ui/decisions.md`](ui/decisions.md) §4） |
| 桥接页 | 还没有现代风专属实现、用像素风 DOM 加桥接样式（`legacy-utilities.css`）适配的页面 |
| 页面签名 | 每个路由在 `interaction.css` 里选定的那一族页面级动作（[`ui/interaction.md`](ui/interaction.md) §3） |
| 在场（presence） | 弹层关闭后仍留在 DOM 里播完退场动画再卸载的统一机制（`crate::presence`；[`ui/motion.md`](ui/motion.md) §五） |
| 门禁 | `cargo make check` 里的机械守卫；红了先当 P0 处理 |
| 来源戳 | 外部数据产物上记录来源与依赖摘要的标记（[`data/data-sync.md`](data/data-sync.md)） |
| 内容指纹 | 题面归一化后的哈希，解析 / 错题 / 收藏都靠它认题（[`data/typography.md`](data/typography.md)） |
| SSOT | 单一事实来源：同一件事只有一处权威定义，其它地方引用它 |
