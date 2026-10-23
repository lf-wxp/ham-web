# 端到端测试

> 单元测试跟着 crate 走（`cargo make test`）；这份管 Playwright：断言口径、覆盖范围、
> 覆盖率统计、CI 集成与本地排障。

**框架**：[Playwright Test](https://playwright.dev)（`@playwright/test` + `@axe-core/playwright`），单 project（Chromium），`fullyParallel: true`，每个用例一个全新浏览器上下文（`localStorage` 为空）。

**断言方式**：以可访问性语义为主（`getByRole` / `getByLabel` / `getByPlaceholder` + `toBeVisible` / `toHaveText` / `toHaveAttribute`），配合 `expect.poll` 等待异步落盘；数据落点直接读 `localStorage`（`page.evaluate`）校验；文件导入 / 导出用 `setInputFiles` + `waitForEvent("download")`；原生 `alert` / `confirm` 用 `waitForEvent("dialog")` 或预先挂 `page.on("dialog")` 自动确认（否则渲染进程被阻塞）。

**覆盖范围**（`e2e/tests/`）：

| 模块 | 用例文件 |
| --- | --- |
| 练习流程 | `practice_flow.spec.ts`（题库切换、只看本类新增 / 只练没做过、题内搜索跳题、题序切换确认、收藏、专项链接）、`study.spec.ts`、`swipe.spec.ts` |
| 模拟考试 | `exam_flow.spec.ts`（A/B 类规则与倒计时、标记、答题卡筛选 / 跳转、交卷确认、交卷后对错、中断恢复、薄弱项与自定义组卷、多选题勾选）、`exam_review.spec.ts`（考后复盘） |
| 复习与题库 | `study_modes.spec.ts`（闪卡、每日挑战、打印版）、`browse.spec.ts`（分类浏览搜索 / 筛选 / 分页）、`glossary.spec.ts`（术语表）、`mistake_topics.spec.ts`（易错知识点）、`notes.spec.ts`、`cards.spec.ts`、`listen.spec.ts` |
| 知识库专题 | `china_ham.spec.ts`（国内卫星频率表、409 MHz 合规对照）、`lora.spec.ts`（LoRa / Meshtastic / AREDN 专题与 `/tools` 的 LoRa 链路计算）、`radiogram.spec.ts`（NTS Radiogram / ICS-213 生成）、`modern_digital_modes.spec.ts` |
| 日志与竞赛 | `log_form.spec.ts`（表单提交、必填校验、搜索筛选、编辑、清空二次确认、QSL 同步、本台信息）、`log.spec.ts`（ADIF 导入去重 / 导出）、`qsl_labels.spec.ts`、`qsl_image.spec.ts`（卡片影像）、`qsl_lotw.spec.ts`（LoTW 导入）、`qsl_sync.spec.ts`（多源三向 diff）、`qsl_designer.spec.ts`（卡片版式 / 导出）、`log_qsl.spec.ts`（日志侧 QSL 徽章与标记）、`contest_log.spec.ts` |
| 工具与实时 | `tools.spec.ts`（计算器）、`backup.spec.ts`（数据备份导出 / 导入 / 合并 / 非法文件）、`waveform_lab.spec.ts`（调制波形与频谱、滤波响应、星座与眼图、香农容量、Smith 圆图、传播热力图）、`photo_processor.spec.ts`、`apt_decoder.spec.ts`、`psk_decode.spec.ts`、`ft8_decoder.spec.ts`（真实 20m 录音黄金标准、FT4 协议跑通）、`cw_decoder.spec.ts`、`cat.spec.ts`、`morse.spec.ts` |
| 天线建模 | `nec.spec.ts`（预设与几何编辑、地面与材质、`.nec` 导入导出、扫频与可用带宽、Yagi 向导、模板加载、拖拽与阵列复制、NVIS 覆盖） |
| 计划与全局 | `planning.spec.ts`（倒计时边界、备考计划）、`progress` 相关、`navigation.spec.ts`（导航跳转、主题、语言）、`search.spec.ts`（全站搜索）、`notifications.spec.ts`（权限与推送降级）、`storage_facade.spec.ts`（快照与权威层版本仲裁）、`i18n_plural.spec.ts`（en / es 复数形态回归） |
| 异常与降级 | `resilience.spec.ts`（题库加载失败、本地数据损坏、接口失败、存储写满）、`fatal.spec.ts`（wasm 兜底页）、`updates.spec.ts`、`new_features.spec.ts`、`a11y.spec.ts`、`knowledge_i18n.spec.ts` |

`smoke.spec.ts` 从路由表读取全部页面，在浅色 / 深色主题下逐一检查运行时错误、资源 404 与 axe（WCAG 2.1 AA）—— **新增页面会自动纳入**，无需改测试。`i18n_layout.spec.ts` 对全部路由做中 / 英 / 西三语布局回归（横向溢出、文本裁剪、越界、浮点未取整）。`mobile_overflow.spec.ts` 再把全部路由按 320 / 390px 两支手机宽度扫一遍，断言整页不出现横向滚动（`scrollWidth - innerWidth <= 1`，落在 `overflow-x: auto` 容器里的元素不算）。

```bash
cd e2e && npm ci && npx playwright install chromium   # 首次
cargo make e2e                                         # 构建 dist/ 并由 Playwright 启动 release 服务器（4173 端口）
cd e2e && E2E_BASE_URL=http://127.0.0.1:3000 npx playwright test   # 直接测正在运行的 trunk dev
cargo make e2e-coverage                                # 只统计覆盖率，不跑用例
```

- `E2E_CHANNEL=chrome` 改用本机已安装的 Chrome；`E2E_PORT` 修改 release 服务器端口；
- 每个测试使用全新浏览器上下文（`localStorage` 为空），fixture 默认把练习 / 考试的快捷键说明标记为已看过；
- 需要「已有数据」的场景用 `page.addInitScript` 直接写 `localStorage`，不依赖真实操作累积。

### 已知的失败与本地排障

**先分清三种红**，再决定要不要改代码：

**① `trunk serve` 的产物缺 `public/` 资产**（本地最常见、也最容易被误判成「抖动」）

`cargo make dev` / `trunk serve` 会把 `public/` 拷进产物目录；一旦这次拷贝没发生或产物被覆盖，dev 服务器对 `/data/i18n/*.json`、`/questions/*`、`/*-worker/*` 会返回 `index.html`（SPA 回退），于是：语言包解析失败 → 英文 / 西语界面回退中文、题库数据缺失 → 页面空白或整页挂载失败。**自检一条命令**：

```bash
curl -s http://127.0.0.1:3030/data/i18n/en.json | head -c 1   # 正常是 `{`，返回 `<` 就是缺资产
ls target/dev-dist/                                          # 正常应有 data/ questions/ fonts/ 等
```

缺了就重来一次（`rm -rf target/dev-dist && cd crates/app && cargo make dev`，或 `trunk serve --dist ../../target/dev-dist`）。这类症状包括 `waveform_lab.spec.ts:67`（英文阶数限定语）、`a11y.spec.ts`（练习页内容为空）、`i18n_layout.spec.ts` 的 `page.goto: Page crashed`。

**② CPU 密集用例在 dev 构建下需要余量**

未 `wasm-opt` 的 dev 构建比 release 慢一个量级，`nec.spec.ts:305`（Yagi 优化循环，60s 超时）在并行跑时容易踩线。**用 `--workers=1` 复跑**通过即可，不是回归。

**③ 真正与本仓库当前改动无关的既有失败**

| 现象 | 结论 |
| --- | --- |
| `knowledge_i18n.spec.ts:5` | 英文界面缺 `/milliwatt levels/` 译文，属知识库译文数据侧 |

**④ 快照型断言踩到防抖求解**（曾经被误记成上面的「既有失败」，已修）

`nec.spec.ts:104` 里「删掉负载后结果条应回到无负载时的值」一度稳定失败：求解是 250ms 防抖的，
用例删完导线**立刻**快照 `noLoad`，拿到的是**上一次（两根导线）**的结果，后面拿它当基准自然
永远对不上 —— 应用本身是对的（删负载后确实回到了无负载值）。修法是快照前先
`await expect(summary(page)).toHaveText(shifted)` 等结果落定。
**教训：对防抖 / 异步刷新的输出做快照，要先等它落定再读，别把「上一次的值」当基准。**

> 本地逐页调试用 `trunk serve` 方便，但要**先确认资产完整**；跑整批回归或提交前请用 `cargo make e2e`（release 构建 + Playwright 自带服务器，口径与 CI 一致）。

两条改 UI 时的经验（动交互语义尤其重要）：

- **改了 `role` / `aria-*`，要按「断言里出现过这些属性」全仓搜 spec，不要按页面名找** —— Smith 圆图的用例住在 `waveform_lab.spec.ts` 里，按页面名找就会漏；
- `getByLabel` / `getByRole` 依赖的标签关联与 role 是**测试契约**：换组件时若动了它们，必须同步 spec（细则见 [`docs/ui-components.md`](ui-components.md)）。

### 覆盖率统计口径

`cargo make e2e-coverage`（`e2e/scripts/coverage.mjs`）输出三条互不替代的指标，报告写入 `e2e/test-results/coverage.{md,json}`：

| 指标 | 定义 | 用途 |
| --- | --- | --- |
| 路由冒烟覆盖率 | `smoke` / `i18n_layout` / `mobile_overflow` 从路由表遍历全部路由做渲染 + 无障碍 + 桌面 / 手机宽度体检，恒为 100% | 保证「新增页面不漏测」 |
| 交互覆盖率 | 至少有一条功能用例通过 `page.goto` 显式访问过的路由数 / 路由总数（不含点击链接间接到达，口径保守可复现） | 衡量有多少页面有专属功能用例 |
| 关键流程覆盖率 | `e2e/coverage-targets.json` 中列出的核心用户旅程，已有用例标题命中的比例 | 真正反映业务场景覆盖，CI 门禁看这个 |

关键旅程清单（`coverage-targets.json`）覆盖：核心用户路径（练习 → 错题 → 复习 → 考试 → 复盘）、异常场景（题库加载失败、本地数据损坏、接口失败、存储写满、wasm 兜底）、边界条件（空呼号提交、空标题倒计时、多选题取消勾选、超大文件、非法备份文件）、权限校验（通知被拒、Web Push 不支持）、页面跳转（导航下拉、空态引导链接、404）、表单提交与数据交互（日志录入 / 编辑 / 清空、QSL 同步、备份导入导出、ADIF / Cabrillo 导入导出）。

新增功能时：先补用例，再把旅程追加到 `coverage-targets.json`（`grep` 为用例标题子串，需唯一命中；脚本会校验没有歧义）。

### 集成到 CI

`.github/workflows/check.yml` 的 `e2e` job 在构建 release 站点后执行 `npx playwright test`，随后运行覆盖率脚本：

1. 报告同时写入 job summary（PR 里直接可见趋势）与 `e2e-coverage` artifact（保留 30 天）；
2. 用仓库变量 `E2E_MIN_JOURNEY` / `E2E_MIN_ROUTE` 设阈值即可变成硬门禁（低于阈值时 job 失败）；当前默认不设阈值，只做趋势观测；
3. 失败时上传 `playwright-report`（含 trace，保留 7 天）。

本地提效：`cargo make check` 只做静态检查；改了 UI 再跑 `cargo make e2e`，只关心覆盖率变化则跑 `cargo make e2e-coverage`。

