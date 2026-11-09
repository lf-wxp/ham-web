# 给 AI agent 的入口

动这个仓库的代码之前先读这份：**几条不能违反的硬规则**，加上「改完怎么验」。
每条规则只写结论，原因、取值与例外都在落点文档里 —— 这里不重复它们。

**找文档先看 [`docs/README.md`](docs/README.md)**（唯一的文档索引，含「常见任务直达」）。

## 1. 硬规则

### 界面

- **先确定改的是哪套皮肤**：现代风（`md-*` 类、`kit_modern/`、`style/modern/`）还是像素风（`pxl-*` 与工具类、`components/`、`style/pixel/`）。
  两者不共享类名与 token。→ [`docs/ui/decisions.md`](docs/ui/decisions.md) §3
- **界面控件只有一套来源：`crates/app/src/ui/`。** 页面里不写原生 `<input>` / `<select>` / `<textarea>`，不新增本地 `const INPUT` / `CHIP_ON`，
  不直接调类名工厂；`grep -rn "刻意的例外" crates/app/src` 列全仅有的例外。→ [`docs/ui/pixel.md`](docs/ui/pixel.md) §10
- **页面宽度只有一个来源**：`PageHeader` 与 `PageContainer` 里的 `max-w-5xl`，值由 `style/input.css` 的 `--page-max` 决定（现代风的 `--md-page-max` 取同一个变量，顶栏也用它 —— 两套皮肤版心同宽）。页面里不写 `max-w-*` / `container` / 自造的 `mx-auto` 宽度容器。
  → [`docs/ui/pixel.md`](docs/ui/pixel.md) §4 · [`docs/ui/modern.md`](docs/ui/modern.md) §3
- **风格派发不许在组件体里订阅信号**：组件体用 `kit::is_modern_at_mount()`，`kit::is_modern()` 只留给 `view!` 属性闭包 / `move ||`，
  且那个闭包要交给 `view!` 渲染（写 `{f}`，不写 `{f()}`）。dev 构建的 `you access a RwSignal<UiStyle> outside a reactive tracking context`
  告警直接指向调用点，不在 `theme.rs`。→ [`docs/ui/decisions.md`](docs/ui/decisions.md) §7
- **模态的焦点与滚动锁各只有一份实现**：每个弹层都调 `crate::focus::use_modal_focus` 与 `crate::overlay::use_scroll_lock`，不自己写 `body.style.overflow`。
  → [`docs/ui/decisions.md`](docs/ui/decisions.md) §7
- **弹层退场是动画、时长读 token、卸载延迟只用 `crate::presence`、关闭态不拦截点击**；新按钮变体的悬停 / 按下规则带 `:not(:disabled)`；
  表单控件的错误态只认 `aria-invalid`。→ [`docs/ui/motion.md`](docs/ui/motion.md) §五、§六
- **页面级动效只有一条来源**：逐页动作写在 `interaction.css` 的 `#route-content[data-sig="…"]` 分支；新增路由必须在
  `PageSignature::paths` 里挑一个签名（漏了 `cargo test` 会红）。→ [`docs/ui/interaction.md`](docs/ui/interaction.md) §6.1
- **页内第二根吸顶栏只有一个来源**：`StickyPanel`；页面里不手写 `sticky top-[…px]`。→ [`docs/ui/pixel.md`](docs/ui/pixel.md) §4、§11
- **现代风 CSS**：视口断点只有五档（480 / 640 / 720 / 900 / 1024）外加外壳的两个 —— dock 与页内菜单按钮的 768、顶栏品牌文字折叠的 360，窄屏档写 640 不写 639；同一底边只允许一层固定层；
  状态色只走 token 不写字面色值；`:has()` 不能决定版式。→ [`docs/ui/modern.md`](docs/ui/modern.md) §2.1、§3、§6.5
- **加 / 删现代风 `md-*` 类名要同步改 [`docs/ui/modern-classes.md`](docs/ui/modern-classes.md)**（`check-structure` 精确双向校验）。

### 代码与文案

- **一个文件只放一个组件**（含任何返回视图的函数），一族组件用文件夹模块。→ [`docs/engineering/code-quality.md`](docs/engineering/code-quality.md) C3
- **不依赖浏览器的逻辑放 `ham-web-core` 并带单元测试**，`crates/app` 只做 DOM 与视图。→ [`docs/engineering/architecture.md`](docs/engineering/architecture.md)「分层与依赖方向」
- **文案必须走 `t("域.词条")`，字面量写在调用点**；新增词条 zh / en / es 三份齐全，再 `cargo make i18n-pack`。
  页面渲染 `crates/core` 的中文原文（`kt_tracked` / `core_text_tracked`）要挂词典拉取。→ [`docs/data/i18n.md`](docs/data/i18n.md)
- **重活不进新页面的首帧**：首屏以下的块包 `Deferred`（几块都重就每块各一层）；长列表按主键键控（`<For key=…>`）并用 `grow_cap` 分批补齐，
  不整体 `collect_view`；读日志 / 错题本 / 题库这类大集合用 `.with` 借用，不 `.get()` 克隆；大本地数据走 `get_json_shared`，
  页面间共用的昂贵派生结果挂 store 级 `Memo` 并登记 `performance.md` §4；搜索框筛选挂防抖副本，重置分页写 `set_if_changed`；
  滚出视口的循环动画要停（只点名无限循环的那几条）。→ [`docs/ui/motion.md`](docs/ui/motion.md) §3、§7 · [`docs/engineering/performance.md`](docs/engineering/performance.md) §2
- **标签关联与无障碍名要写全**：`Field` 的 `r#for` 与控件 `id` 配对；无可见标签的控件 / 图标按钮必填 `aria_label`。这些同时是 e2e 的定位契约。
- **风格**：Rust 2024，`rustfmt.toml`（2 空格、行宽 100），clippy 的 `correctness/suspicious/style/complexity/perf` 是错误级。
  注释写「为什么」不写「做了什么」；注释、文案、文档一律中文（专有名词与代码标识符除外）。

### 文档

- **一件事一个家**：每个事实只在一处写全，其它地方链接过去。新增 / 移动 / 删除文档先看 [`docs/README.md`](docs/README.md) 的「文档约定」。

## 2. 提交前与改完怎么验

```bash
cargo make check   # 提交前门禁（组成与「失败意味着什么」见 docs/engineering/code-quality.md §2）
cargo make ci      # check + release 前端构建 + 体积预算（正式提交用这个）
```

改了界面再验：

```bash
cargo make i18n-pack   # 生成 public/data/i18n/{en,es}.json（生成物不入库；漏跑 en / es 界面会回退中文）
cd crates/app && trunk serve --dist ../../target/dev-dist --port 3030 --no-autoreload
cd e2e && E2E_BASE_URL=http://127.0.0.1:3030 npx playwright test tests/<相关 spec>.spec.ts
```

- 改了 `role` / `aria-*`：按「断言里出现过这些属性」**全仓搜 spec，不要按页面名找**。
- 改了横向排版（标签 + 控件、`shrink-0`、网格子项、长文案下拉）：跑 `mobile_overflow.spec.ts` 与 `i18n_layout.spec.ts`，
  **像素风与现代风（`E2E_UI_STYLE=modern`）各一档** —— 桌面宽度下看不出来，两档也不能互相代替。
- 红了先分清是不是环境问题（dev 产物缺 `public/` 资产、缺 `postbuild` 才生成的知识库词典、dev 构建没有 `wasm-opt` 导致 CPU 密集用例超时）：
  → [`docs/engineering/testing.md`](docs/engineering/testing.md)「已知的失败与本地排障」。整批回归用 `cargo make e2e`（release 口径）。
- 写 / 改 e2e 用例的约定（空洞断言、`toBeHidden` 与卸载、检测器要配自测、变异验证、抓首帧用 `MutationObserver`）→ 同文档「写用例的约定」。
- 改了性能相关的东西（首帧、列表、缓存、动画）：测量口径（发布构建、4 倍降速、**必须带真 GPU**、重度数据、站内导航、多次取中位）与阈值见
  [`docs/engineering/performance.md`](docs/engineering/performance.md) §1；动之前先看 §3「已评估、决定不做」，别重复排查。
