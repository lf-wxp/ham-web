# UI 审核报告与修复追踪

> 2026-10-15 全仓 UI 审核。结论与证据见下；修复计划与进度在本文件末尾追踪。
> 审核方法：全仓扫描 `crates/app/src` 的页面 / 组件 / 样式，对照
> `docs/ui-components.md` 与 `crates/app/src/ui/README.md` 的规范逐项核对。

## 一、审核结论

### 1.1 已经做好的（不需要投入）

| 项 | 状态 | 证据 |
|---|---|---|
| 表单控件收敛 | 完成 | 原生 `<input>` / `<select>` / `<textarea>` 已清零，唯一来源 `ui/` |
| 页面宽度收敛 | 完成 | `PageHeader` / `PageContainer` 唯一来源，`check-ui` 机械守着（10 行 / 8 文件封闭清单） |
| 配色方案 + 深色模式 | 完成 | `color_scheme.rs` 11 套方案（classic / forest / ocean / sunset / graphite / pico / gameboy / nes / commodore / cga / spectrum）× 亮暗，设置面板完整暴露 |
| 动效 token 化 | 完成 | 全部动画集中在 `style/pixel/motion.css`，页面无自造 `@keyframes` |
| 内联样式纪律 | 完成 | 全库仅 12 处 `style=`，各有理由（画布尺寸、touch-action、打印比例等） |
| 像素规范门禁 | 完成 | 零圆角 / 零模糊 / 零渐变由 `check-ui` 持续守着 |
| 全局搜索 | 完成 | `components/search_dialog.rs` + `practice_search_dialog/` 已存在 |

### 1.2 待优化（本报告的修复范围）

按「重复面大小 × 用户可见收益」排序：

| # | 方向 | 现状 | 证据 |
|---|---|---|---|
| 1 | **Table + Pagination 组件缺失** | `<table>` 在 77 个页面文件手写，样式各拼各的；22 个页面用 `.slice()` / `.take()` 截断列表充当翻页；移动端策略不一（53 文件有 `overflow-x-auto`，`gear/mod.rs` 还有 `min-width: 640px` 硬编码） | `pages/gear/mod.rs:685` 等 |
| 2 | **反馈通道不统一** | 成功 / 失败提示散布 24 个页面的内联文字 + 单按钮 `MessageDialog`；唯一 toast 是成就专用；`EmptyState` 只铺了 browse / bookmarks / bestiary 三页（7 处），dx_spots、rbn、psk_reporter 等数据页空态空白 | `components/common/empty_state.rs` |
| 3 | **加载态随意** | 全站 0 处 `Suspend`，`Skeleton` 只有 `browse_page` 在用；17 个解码器 / 数据页等待时只有一行文字或小 spinner，无布局占位 | `components/common/loading.rs` / `skeleton.rs` |
| 4 | **Tooltip 与键盘可达性** | 62 个文件用原生 `title=` 兜底（视觉割裂、触屏不可用）；`study_heatmap.rs` 热图格是普通 div，无 tabindex / role，键盘用户拿不到数值 | `components/common/study_heatmap.rs:79-82` |
| 5 | **Tabs / Breadcrumb 缺位**（次要） | 页内视图切换用 `ChipGroup`（40 文件）或手写 `active_tab` 信号，0 处 `role="tablist"`；log/、tools/ 层级深但无面包屑 | — |

## 二、修复方案（逐方向）

### 方向 1：Table + Pagination

- 新增 `ui/table/` 文件夹模块（`table_root.rs` / `table_row.rs` / … + `shared.rs`），
  家族同 `chip/`：`mod.rs` 只放声明与重导出。
- 表格视觉基线走 `.pxl-table`（`controls.css`）：2px 描边、行高 4 的倍数、表头底色、
  斑马纹（可选）；移动端策略内置于根组件（`overflow-x-auto` + 最小宽度提示）。
- 新增 `ui/pagination.rs`：页码 + 上一页 / 下一页，`page` / `total_pages` / `on_change`
  受控式 API；`role="navigation"` + `aria-label`。
- 迁移顺序：先 `gear` / `reference` / `digital_comms` 这类多表重灾区，再逐页铺开；
  每页迁移后过相关 e2e。

### 方向 2：反馈统一（Toast + EmptyState）

- 新增 `ui/toast/`：全局 Toast 通道（成功 / 失败 / 信息，自动消退，`role="status"` /
  `aria-live="polite"`）；挂在 `app/mod.rs`，页面只调 `toast::push(…)`。
- 约定：轻量成功 / 已复制 → Toast；需要用户确认的失败 → 保留 `MessageDialog`。
- `EmptyState` 铺到所有数据列表页（dx_spots / rbn / psk_reporter / repeater_lookup 等）。

### 方向 3：加载态收口

- 约定：资源型页面（异步拉数据 / 解码）用 `Suspend` + `Skeleton`；短操作保留 `Loading`。
- 优先收口 17 个「一行加载中」页面。

### 方向 4：Tooltip + 键盘可达性

- 新增 `ui/tooltip.rs`（复用 `popover.rs` 基座）：hover / focus-visible 都触发，
  纯 CSS 定位 + `role="tooltip"`。
- 迁移热图 / 图表类 hover-only 信息；给 `study_heatmap` 格子补 tabindex + 焦点可达。

### 方向 5：Tabs / Breadcrumb（次要，本轮暂缓）

- 视图切换语义化与面包屑纳入 backlog，待方向 1-4 完成后再评估。

## 三、进度追踪

| 方向 | 状态 | 组件 | 迁移页面 | 验收 |
|---|---|---|---|---|
| 1. Table + Pagination | 完成（常规表格已清零） | `ui/table/`（Table / TableHead / TableBody / TableRow / TableHeader / TableCell；已补 `colspan` / `aria_hidden` / `title` / `tabindex` / `role` / `wrapper_class` 通道）、`ui/pagination.rs`、`core/pagination.rs`（6 单测） | 44 个手写表格文件全部迁移（组 A 25 + 组 B 14 + 中断后接手 5）；`TableSection` 收口（≈30 处）、`entry_list` 分页 → `Pagination`。**刻意保留**：`nec/{lines,wire_table,load_table,design}` 4 张（`data-slot` 是 e2e 定位契约）、`qsl_labels/label_view.rs`（打印纸面 mm 坐标）、`components/exam/category_compare.rs`（无边框极简行列表，视觉会变）、`pages/bands/bands_page.rs`（tbody 大量动态 `rowspan`/`colspan`，`TableCell` 的静态 `colspan: u32` 表达不了；thead 曾迁成组件又整体回退） | 已过：`cargo make check` 全绿；`satellites`/`contest_log`/`morse`/`repeater_chirp`/`live_feeds`/`tools_backup` 等 spec 全绿；迁移页 320/390px 无溢出 |
| 2. Toast + EmptyState | 进行中 | `ui/toast.rs`（ToastRoot + push/push_success/push_error），挂 `app/mod.rs`；`EmptyState` / `Loading` 文案升级为 `TextValue`（切语言响应式） | 已完成：`util::alert` 清零（37 处 → toast，含 3 处硬编码中文词条化），`util::alert` 删除；e2e 契约同步（`log`/`live_feeds`/`calendars`/`qsl_lotw`/`qsl_sync`/`tools_backup` 等 7 个 spec）；`dx_spots` 三态（加载/降级/筛选空）迁到 `Loading` / `EmptyState`；`rbn` / `psk_reporter` / `repeater_lookup` 的「查询失败 / 查无记录」提示块迁到 `EmptyState`（词条与 e2e 断言文字不变） | 已过：`cargo make check` + 上述 spec 全绿 |
| 3. Suspend + Skeleton | 完成（骨架策略落地） | `Skeleton` / `Loading`（label 已升 `TextValue`） | 首屏资源列表页铺骨架（页头先行 + 轮廓占位 + `sr-only` 状态播报）：`qcode_page` / `glossary_page`（6 行）、`cards`（骨架卡）、`bookmarks`（4 行）、`browse`（既有 6 行）；查询型短等待保留 `Loading`（`dx_spots` 等）。**不引入 `Suspense`**：数据获取是 `LocalResource` + `match` / `spawn_local` + 信号两套成熟模式，`Suspense` 需重构为 `Resource` 流，收益不抵成本 | 已过：`cargo check`；e2e 待跑 |
| 4. Tooltip + 键盘可达性 | 完成（原生 `title=` 清零） | `ui/tooltip.rs` + `.pxl-tip*`（controls.css） | 全仓原生元素 `title=` **47 处全部迁移**：B 类热图/图表格 12 处（+tabindex +role="img" +aria-label，柱状图 Tooltip 包整列防百分比高度塌陷，`history_bars` 用 `self-stretch`）；C 类说明型 32 处（截断补全 5、说明文本、行为提示按钮/链接）；A 类图标按钮 3 处（补 aria-label / 删冗余 title）。删除死词条 `shell.hud-profile`。**不动**：SVG `<title>` 子元素（a11y 名）、组件 prop 的 `title`（`PageHeader` 等 165 处语义） | 已过：`cargo make check` 全绿；`morse`/`cards`/`browse`/`study_modes`/`calendars`/`live_feeds` 22 用例全绿；17 个迁移页 320/390px 无溢出；Tooltip 悬停行为 e2e 验证通过 |
| 5. Tabs / Breadcrumb | 暂缓 | — | — | — |

### 已迁移文件清单（方向 2：Toast）

`components/exam/exam_result_dialog.rs`、`pages/weekly.rs`、`pages/log/{log_page,log_health_panel,station_panel,entry_form,mod,qsl_sync_dialog}.rs`、
`pages/eqsl/lotw_station_location.rs`、`pages/tools/backup_tool.rs`、`pages/{callsign,events,gear,portable_lookup,zone_map,contest_calendar,psk_reporter,rbn,repeater_lookup,dx_spots,qsl_designer,report}.rs`、
`pages/photo/photo_uploader.rs`、`pages/muf/voacap_card.rs`、`pages/practice/view.rs`；`util::alert` 函数已删除。

> 进度表随修复逐项更新；每个方向完成时在「迁移页面」列写明实际迁移的文件清单。
>
> **外部变更注记（2026-10-15）**：工作区同时存在未跟踪的进行中改动（`crates/core/src/contest_analysis.rs`、
> `sat_doppler.rs`、`cat.rs`、`contest.rs`、`lib.rs` 的 `contest_analysis` 行），非本报告改动。
> `contest_analysis.rs:293` 的 `sort_by` → `sort_by_key`（clippy）会把 `cargo make check` 判红，
> 待该文件合入时修复；本报告的全部改动在它出现前已通过完整 `cargo make check`。

### 剩余待办（下一轮）

- 方向 1（完成）：常规表格已清零；刻意保留的 6 处见上表（nec ×4、label_view、category_compare），
  要收编需先补 `data-slot` 通道 / 接受视觉变化，不值得为个别页面做。
- 方向 2（完成）：`EmptyState` / `Loading` 已铺到 dx_spots / rbn / psk_reporter / repeater_lookup。
- 方向 3（完成）：骨架策略已落地；解码器页保留「图标 + 文案 + 取消」占位（任务型等待，非列表加载）。
- 方向 4（完成）：原生 `title=` 已清零；组件 prop 的 `title`（`PageHeader` / `Button` / `Input` / `FileDropZone` / `ChipToggle` / `TableCell` 等）是 prop 语义而非悬停提示，不在迁移范围。
- 全量 e2e 回归建议用 release 口径（`cargo make e2e`）：dev 构建连续导航 170 路由会在 ~100 路由处
  Page crash（内存累积，单路由抽检全部正常，非本报告引入）。
