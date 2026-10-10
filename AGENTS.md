# 给 AI agent 的入口

这份文件是**动这个仓库的代码之前先读的东西**：指向各领域的规范，并列出几条不要违反的硬规则。

## 1. 按你要改的东西找文档

| 要改什么 | 先读 |
|---|---|
| **界面 / 表单控件 / 按钮 / 弹层** | [`docs/ui-components.md`](docs/ui-components.md)（使用规范：决策表、硬规则、坑、验收）→ [`crates/app/src/ui/README.md`](crates/app/src/ui/README.md)（组件清单与示例） |
| **页面骨架 / 内容宽度（页头、正文容器）** | [`docs/ui-components.md`](docs/ui-components.md) §1.5：宽度只有 `max-w-5xl` 一种，页头用 `PageHeader`、正文用 `PageContainer` |
| 仓库里那些「刻意的例外」（没走组件的原生元素） | `grep -rn "刻意的例外" crates/app/src`；清单与原因见 `docs/ui-components.md` §5 / §8 |
| 界面文案 / 译文 | [`docs/i18n-refactor.md`](docs/i18n-refactor.md)；词条在 `data/i18n/{zh,en,es}/<域>.json` |
| 题库题面排版（空格、全角半角、指纹） | [`docs/typography.md`](docs/typography.md) |
| 题目解析的写作 | [`docs/explanations-style.md`](docs/explanations-style.md) |
| 项目结构、cargo make 任务、路由 | [`README.md`](README.md) |

## 2. 硬规则

- **界面控件只有一套来源**：`crates/app/src/ui/`。页面里不写原生 `<input>` / `<select>` /
  `<textarea>` / `<input type="range">`，不新增本地 `const INPUT` / `CHIP_ON`，
  不直接调用类名工厂（`button_class` 只给 `Button` / `ButtonLink` 内部与
  `docs/ui-components.md` §8 清单里的几处「刻意的例外」用 ——
  `grep -rn "刻意的例外" crates/app/src` 可列全）。
- **页面宽度只有一个来源**：内容宽度 = `PageHeader`（`components/common/page_header.rs`）与
  `PageContainer`（`components/common/page_container.rs`）里的 `max-w-5xl`（1024px）。
  页面里不写 `max-w-*` / `container` / 自造的 `mx-auto` 宽度容器；间距用 `PageContainer` 的
  `class` 覆盖；多行页头用 `PageHeader` 的 `children`。详见 `docs/ui-components.md` §1.5。
- **一个文件只放一个组件 —— 共享组件与业务组件都是**（`crates/app/src/ui/`、`components/`、
  `pages/` 一视同仁）。「组件」既指 `#[component]`，也指任何**返回视图的函数**
  （`-> impl IntoView` / `-> AnyView`，例如表格单元格、列表行这类视图构造函数）：
  一个文件里出现两个就必须拆开，别往里加第三个。
  自检（应无输出）：
  ```bash
  grep -rEo '\->[[:space:]]*(impl[[:space:]]+[A-Za-z_:]*[[:space:]]*)?(IntoView|AnyView)' \
    crates/app/src --include='*.rs' | cut -d: -f1 | sort | uniq -c | awk '$1>1'
  ```
- **一族 / 主从多个组件用文件夹模块**（`dialog/`、`radio/`、`select/`、`chip/`、
  `pages/bands/`、`pages/log/awards_panel/`、`components/study_plan_card/` 都是这么放的）：
  主组件与共享 helper 放 `mod.rs`，子组件各占一个文件，共享的样式 token / context 放
  `shared.rs`。命名注意：`chip/chip.rs` 这类「模块与父模块同名」会被 Rust 拒绝，照
  `dialog_root.rs` / `select_root.rs` / `chip_item.rs` 起名。
- **文案必须走 `t("域.词条")` 且字面量写在调用点**；新增词条要 zh / en / es 三份齐全，
  再 `cargo make i18n-pack`。不要硬编码界面中文（检查器抓不到，但 en / es 界面会露馅）。
- **标签关联与无障碍名要写全**：`Field` 的 `r#for` 与控件 `id` 配对；没有可见标签的控件 /
  图标按钮必填 `aria_label`。这些同时是 e2e 的定位契约。
- **不依赖浏览器的逻辑放 `ham-web-core` 并带单元测试**（判定、匹配、解析、排版等），
  `crates/app` 只做 DOM 与视图。
- **提交前必须过 `cargo make check`**；动了页面组件再跑相关 e2e。

```bash
cargo make check   # fmt-check + clippy + test + i18n-check + knowledge-i18n-check + explanations-check + pixel-check
cargo make ci      # 上面 + release 前端构建（提交前用这个）
```

## 3. 改完界面后怎么验

```bash
cd crates/app && trunk serve --dist ../../target/dev-dist --port 3030 --no-autoreload
cd e2e && E2E_BASE_URL=http://127.0.0.1:3030 npx playwright test tests/<相关 spec>.spec.ts
```

- 改了 `role` / `aria-*`：按「断言里出现过这些属性」全仓搜 spec，**不要按页面名找**
  （smith 的用例住在 `waveform_lab.spec.ts` 里）。
- 看到「页面空白 / 整页挂载失败 / 英文界面回退中文」这类红，**先怀疑 dev 产物缺资产**：
  `curl -s http://127.0.0.1:3030/data/i18n/en.json | head -c 1` 返回 `<` 就说明 `public/`
  没被拷进产物（`trunk serve` 的拷贝没发生 / 产物被覆盖），重来一次即可。
  `dist/data/knowledge-i18n/{lang}.json` 同理：它由 postbuild 把
  `data/knowledge-i18n/{lang}/*.json` 合并而成，而 `trunk serve` 不跑 postbuild ——
  缺失时 `knowledge_i18n.spec.ts` 与 `i18n_layout.spec.ts` 会红，别急着怀疑代码。
- dev 构建没有 `wasm-opt`，CPU 密集用例（如 `nec.spec.ts:305` 的 Yagi 优化）并行时容易
  超时 —— `--workers=1` 复跑通过就不是回归。整批回归用 `cargo make e2e`（release 口径）。

## 4. 写代码的风格

- Rust 2024，`rustfmt.toml`（2 空格、行宽 100），clippy 的
  `correctness/suspicious/style/complexity/perf` 是**错误级**（`-D warnings`）。
- 注释写「为什么」，不写「做了什么」；只在别人会问「为什么这么写」的地方加注释。
- 注释、文案、文档一律中文（除专有名词与代码标识符）。
