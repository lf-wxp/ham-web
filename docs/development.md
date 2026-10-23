# 开发规范与样式体系

> 硬规则（单一来源、一文件一组件、文案走 `t()`）的**入口**是 [`../AGENTS.md`](../AGENTS.md)；
> 本文放操作层的东西：快捷键、代码规范、改 UI 的注意点、像素风与动效降级。

## 快捷键

- `/`：打开全站搜索（不在输入框内时生效）
- `← / →`：上一题 / 下一题
- `1-9`：选择对应选项；多选题为切换，`Shift` / `Cmd` + 数字为仅选该项
- `Enter`：打开搜索（练习 · 顺序模式；焦点在按钮 / 链接上时为默认操作）
- `Esc`：关闭搜索面板 / 对话框

## 代码规范

- Rust 2024 edition，格式遵循根目录 `rustfmt.toml`（2 空格缩进，行宽 100）；
- workspace 统一开启 `unsafe_code = "forbid"` 与 Clippy `correctness/suspicious/style/complexity/perf`；
- 提交前执行 `cargo make ci`（仅前端样式改动时可先用 `cargo make check` 快速自检）。两者都包含 `cargo make check` 这一整套门禁：格式、Clippy、单元测试、语言包与文案词典、知识库译文、解析表、e2e 覆盖对账、界面规范、结构守卫、README 对账、像素风生成物、数据来源戳与时效性；
- 新增或删除 Worker、路由、`cargo make` 任务、`docs/` 文件时，同一提交里改 README —— `cargo make readme-check` 会对账这些数字与清单；
- 不依赖浏览器的逻辑（日志、奖状、DX 通知匹配、Koch、错题本、考试判定、CAT 协议解析、CW 解码、标签排版等）放在 `ham-web-core`，并附单元测试；
- 修改界面文案时同步补 `data/i18n/{lang}/{domain}.json`（按域拆分、语义 key `域.词条`，`cargo make i18n-check` 会校验重复 key、占位符数量、死条目与语言包新鲜度）；改完译文跑一次 `cargo make i18n-pack`；知识库正文译文放在 `data/knowledge-i18n/`（以中文原文为 key，改动 `crates/core` 里的中文句子后要同步改 key，`cargo make knowledge-i18n-check` 会校验，否则译文静默失效、页面上仍是中文）；
- 改动任一解码 Worker（清单取自 `crates/*-worker`）时，注意 `wasm-bindgen` CLI 版本必须与 `Cargo.lock` 一致（`cargo make setup` 会按锁文件安装，CI 亦有版本漂移校验）。

CI 位于 `.github/workflows/`：`check.yml` 在 push 到 `main` 与 PR 时运行 `cargo make check`，并另起一个 job 构建 release 站点后运行 Playwright 端到端测试（失败时上传报告）；同一份 workflow 还会构建一次 Docker 镜像并跑 `scripts/smoke-image.sh`（依次验证 `/healthz`、首页是否是 `dist/index.html`、SPA 回退、缺失资源 404、语言包与 Worker 是否真的能读到），让 Dockerfile 的改动在 PR 阶段就失败。`build-docker.yml` 在 push 到 `main` 或 `v*` tag 时构建并推送镜像，推送后再从 registry 拉回同一份镜像跑一遍相同的冒烟。`data-sync.yml` 按计划（以及手动触发）重拉外部数据并提交：产物来源戳、新鲜度与「人工转录事实多久没复核」的口径见 [`docs/data-sync.md`](data-sync.md)。


## 修改 UI 的注意事项

> 写 / 改界面前先读 **[`docs/ui-components.md`](ui-components.md)**（通用 UI 组件使用规范：
> 决策表「要什么用哪个」、硬规则、常见坑、验收清单），组件清单与示例见
> [`crates/app/src/ui/README.md`](../crates/app/src/ui/README.md)。

- 页面里不写原生表单元素，统一用 `crates/app/src/ui/` 的组件；基础组件的 Tailwind 类名与原 shadcn/ui 完全一致；类名合并使用 `cn(&[...])`（语义同 `tailwind-merge`）；
- Tailwind 会扫描 `crates/app/src` 下的 Rust 源码，新类名需以**完整字面量**出现（不要字符串拼接类名片段）；
- 在 `leptos` 中向依赖 context 的子组件（如 `RadioGroupItem`、`SelectItem`）传递子元素时，需在父组件的 children 内构建，而不是提前 `collect_view()`；
- 知识库表格统一使用「卡片 + 内部网格线」的样式：卡片内的 `table.border-collapse` 会自动去掉与卡片边框重合的最外圈边框（见 `style/input.css`），因此单元格只需写 `border`，无需手动处理外边线。

## 像素风外观与动效降级

样式集中在 `crates/app/style/pixel/`（调色板与 token、字体、控件原语、动效），组件与页面只用语义 token 与 `pxl-*` 类，规范见 [`docs/ui-components.md` §2](ui-components.md)。

| 关注点 | 实现 |
| --- | --- |
| 背景 | 纯 CSS：亮色是带细抖动点阵的纸张，暗色是深蓝夜空 + 稀疏星点（`steps()` 慢闪）；没有 WebGL、没有渐变 / 模糊，打印时不渲染 |
| 字体 | 标题 / HUD 用 Press Start 2P / Silkscreen；正文与中文用点阵中文体（Fusion Pixel），按 `unicode-range` 切片，只加载命中的分片；`cargo make fonts-pixel`（纯 Rust，`crates/tools/src/pixel/fonts.rs`）生成，许可证见 `public/fonts/LICENSES` |
| 对比度 | 调色板里「会被当文字用」的色阶由 `cargo make pixel-palette` 按真实表面（含徽章色调底与每个配色方案的亮 / 暗表面）校正；`cargo make pixel-check` 只审计，已接入 `check` / `ci`。填充色（血条 / 金币）画在深色槽里，写字请用 `text-hp-text` / `text-gold-text` 等文字专用 token |
| 配色方案 | 设置里可选 11 套方案（经典像素 / 森林 / 海洋 / 晚霞 / 石墨，以及 PICO-8 / Game Boy / 红白机 / C64 / CGA / ZX Spectrum 这几套经典像素调色板），**与明暗正交**：每个方案都有亮 / 暗两套。落在 `<html data-scheme>`（`localStorage` 键 `ui:colorScheme`）。方案表在 `crates/core/src/color_scheme.rs`，每个方案只写 5 个种子色，其余变量按固定规则派生并保证字色对比度；`style/pixel/schemes.css` 由 `cargo make pixel-schemes` 生成。经典方案手写在 `tokens.css`。状态色（血条 / 金币 / 通关绿 / 危险红）与字体、圆角、动效不随方案变 |
| 减少动态效果 | 系统开启「减少动态效果」时，所有逐帧动画降为静态帧 |
| 像素动效开关 | 「外观与动效」设置里可单独关闭（`localStorage` 键 `ui:pixelMotion`，落在 `<html data-pixel-motion>`），不影响系统设置 |
| 易读字体开关 | 同一设置里可把正文切回抗锯齿字体（`ui:readableFont`，`<html data-readable-font>`）；标题与数字仍是像素字体 |

