# 贡献者指南：开发规范与提交前检查

> 动代码前先读 [`AGENTS.md`](../../AGENTS.md)（硬规则与验收命令）。本文放操作层的东西：代码规范的具体约定、
> 提交前流程、CI 概览。界面规范分两份：[现代风](../ui/modern.md)、[像素风](../ui/pixel.md)。

## 代码规范

- Rust 2024 edition，格式遵循根目录 `rustfmt.toml`（2 空格缩进，行宽 100）；
- workspace 统一开启 `unsafe_code = "forbid"` 与 Clippy `correctness/suspicious/style/complexity/perf`（错误级，`-D warnings`）；
- 不依赖浏览器的逻辑放在 `ham-web-core` 并附单元测试，分层边界见 [`architecture.md`](architecture.md)「分层与依赖方向」；
- 修改界面文案时同步补 `data/i18n/{lang}/{domain}.json`，改完跑一次 `cargo make i18n-pack`；
  规则（语义 key、复数、门禁）与知识库正文译文（以中文原文为 key，改了 `crates/core` 里的中文句子要同步改 key）见 [`../data/i18n.md`](../data/i18n.md)；
- 改动任一解码 Worker 时，注意 `wasm-bindgen` CLI 版本必须与 `Cargo.lock` 一致（`cargo make setup` 会按锁文件安装，CI 亦有版本漂移校验）；
- 新增或删除 Worker、路由、`cargo make` 任务、`docs/` 文件时，同一提交里改 README 与 [`../README.md`](../README.md)（文档索引）——
  `cargo make readme-check` 会对账这些数字、清单与索引。

## 提交前

```bash
cargo make check   # 日常自检：不构建前端
cargo make ci      # 正式提交：check + release 前端构建 + 体积预算
```

`check` 由哪些门禁组成、各自覆盖什么、红了意味着什么，**以 [`code-quality.md`](code-quality.md) §2 为准**（这里不再另列一份，
否则每加一个门禁就要改多处）。仅前端样式改动时可先用 `check`；动了页面组件再跑相关 e2e，口径见 [`testing.md`](testing.md)。

## 快捷键

键位的**唯一数据源**是 [`crates/app/src/shortcuts.rs`](../../crates/app/src/shortcuts.rs) 的 `SHORTCUT_HELP`：应用内「快捷键帮助」面板直接渲染它。
文档不再手抄一份——手抄的版本已经漏过 `?`（显示帮助）与 `Shift / Cmd + 数字`（多选题只选这一项）。

## CI

工作流在 `.github/workflows/`：

| 工作流 | 触发 | 做什么 |
| --- | --- | --- |
| `check.yml` | push 到 `main`、PR、手动 | `cargo make check`；另起 job 构建 release 站点后跑 Playwright（失败时上传报告）；再构建一次 Docker 镜像并跑 `scripts/smoke-image.sh`（依次验证 `/healthz`、首页是 `dist/index.html`、SPA 回退、缺失资源 404、语言包与 Worker 真的能读到），让 Dockerfile 的改动在 PR 阶段就失败 |
| `build-docker.yml` | push 到 `main` 或 `v*` tag、手动 | **先本地构建并冒烟，通过了才推送**镜像（先推后测的话，坏镜像已经覆盖了 `latest`，远端 `docker pull` 就会拿到它） |
| `data-sync.yml` | 每周一（`cron`）、手动 | 重拉外部数据并提交；产物来源戳、新鲜度与「人工转录事实多久没复核」的口径见 [`../data/data-sync.md`](../data/data-sync.md) |

e2e 在 CI 里的覆盖率门禁与报告见 [`testing.md`](testing.md)「集成到 CI」；镜像参数与运行时环境变量见 [`deployment.md`](deployment.md)。
