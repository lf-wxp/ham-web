# 现代风 UI 规范页与首页题库入口：审核记录（2026-11-27）

> **一次性记录**，不是长期规范：界面怎么写仍以 [`../ui/modern.md`](../ui/modern.md)、
> [`../ui/pixel.md`](../ui/pixel.md) 为准。本文只回答「这一轮改了什么、当时发现了什么、
> 怎么修的、回归跑了什么」，供后续追踪。
>
> 状态图例：**已完成** / **已评估不做** / **未开始**。

## 1. 范围

- 基线：`main` @ `f23302c`，工作区未提交的变更。
- 规模：39 个文件，+1944 / −147。
- 内容：现代风 `/ui-spec` 七板块（新增 `pages/ui_spec/modern/` 11 个文件）、现代风首页题库版本入口、
  共享组件按皮肤分叉、CSS / 三语词条 / 文档 / e2e 配套、workspace 版本 0.7.5 → 0.7.6。
- 审核方式：读全部 diff + `cargo make` 同款门禁逐项跑 + Playwright 在真实浏览器里量计算样式与交互。

## 2. 结论

首轮审核发现 **4 项阻断提交**、**4 项合并前应修**与若干清理项。截至本记录，**全部已处理**
（其中 1 项评估后决定不做，见 §5）；门禁与相关 e2e 全绿，回归记录见 §6。

## 3. 问题与处理

| 编号 | 级别 | 位置 | 问题 | 处理 | 状态 |
| --- | --- | --- | --- | --- | --- |
| R1 | 阻断 | `kit_modern/home/bank_picker.rs` | 题库卡片直接挂在 `.md-page` 下，章节编号与 `.md-panel::before` 抢同一个伪元素 | 外层包普通 `<section>` 承接章节壳 | 已完成 |
| R2 | 阻断 | `pages/ui_spec/modern/radio.rs` | 滑杆存 0–1、`Slider` 默认 0–100：旋钮贴在左端，拖到 50 时读数 5000% | 统一为 0–100 百分比量纲，补 `aria_valuetext` 与 clamp | 已完成 |
| R3 | 阻断 | `pages/ui_spec/modern/feedback.rs`、`style/modern/kit/01-base.css` | 4 档灯恒为同一个灰点，ON/OFF 无任何视觉变化（`data-tone` / `data-lit` 无人消费） | 属性移到灯本体，补 4 档色相与灭灯规则 | 已完成 |
| R4 | 阻断 | `pages/ui_spec/modern/icons.rs`、`pages/ui_spec/icons_section.rs`、`after_paint.rs` | `grow_cap` 补满即结束；搜索一次再清空后列表永久停在首批 24 个 | 抽 `grow_cap_per_filter`，筛选变化时复位并重新登记 | 已完成 |
| R5 | 应修 | `pages/ui_spec/modern/colors.rs`、`icons.rs` | 复制提示永不消失、无 `aria-live`，且借用 `learning.*` 词条 | 改 `push_success` toast，文案回到 `design.*` | 已完成 |
| R6 | 应修 | `kit_modern/home/{mod,hero,bento}.rs` | 现代首页缺「题库不可用」守卫，与像素首页行为不一致 | 迁入 `available/checking/generation` 与点击拦截 | 已完成 |
| R7 | 应修 | `radio.rs`、三语 `design.json`、`docs/data/i18n.md` | 可见文本 `level-N` 硬编码；西语 `sónar` 重音错；词条计数与合计过期 | 走 `design.md-radio-level`；改 `sonar`；刷新整表计数 | 已完成 |
| R8 | 应修 | `e2e/tests/modern_ui_spec.spec.ts`、`modern_a11y.spec.ts` | 交互缺陷全部漏测；两处空洞断言；`/ui-spec` 不在 axe 路线里 | 补滑杆 / 灯 / 图标重置断言，去掉下限断言，新增七标签 a11y 用例 | 已完成 |
| R9 | 清理 | `modern/data.rs`、`colors_section.rs` | `computed()` 两份逐行相同的实现 | 提到 `spec_data.rs` 共用 | 已完成 |
| R10 | 清理 | `modern/icons.rs` | `size` 与 `icon_class` 两份状态存同一事实 | 类名改为由档位派生 | 已完成 |
| R11 | 清理 | `modern/typography.rs` | 同一面板 `h2` / `h3` 同名，读屏念两遍 | `h3` 改用「字号阶梯」词条 | 已完成 |
| R12 | 清理 | `modern/components.rs`、`modern_ui_spec.rs` | `view!` 内联 `super::super::…`；面板外层与内层重复 `space-y-5` | 顶部导入；纵向节奏只留一层 | 已完成 |
| R13 | 清理 | `modern/data.rs`、`component_index.rs` | 原则卡联动的速查行与卡片取值不对应；组件族缺稳定锚点 | 补「控件高度 / 阴影」两行并重新映射；索引加 `data-spec-list` | 已完成 |
| R14 | 清理 | `components/bank_selector/mod.rs` | 刷新失败块两套皮肤都用 2px 硬边框 | 边框宽度按皮肤分叉 | 已完成 |
| R15 | 可选 | `pages/ui_spec/modern/motion.rs` | 两条无限循环动画不随离屏暂停 | 评估后不做 | 已评估不做 |

### 3.1 R1 首页题库卡片的章节编号与角标冲突

**现象**：现代首页「快速开始练习」卡片左上角渲染成竖排的「0」「4」，卡片上沿还多出一条带刻度的
章节分隔线（真实浏览器截图确认）。

**根因**：`#route-content .md-page > section:not(.md-closing)::before`（`style/modern/kit/08-motion.css`）
负责章节编号，`.md-panel::before`（`style/modern/kit/02-question.css`）负责左上角三角。两条规则写在
同一个伪元素上；此前首页没有「`.md-panel` 直接挂在 `.md-page` 下」的写法，所以从未叠加。
计算样式实测：兄弟区块是 `position:static; width:1136px`，这张卡是
`position:absolute; width:18px; height:18px` —— 编号被塞进 18px 盒子里换行，章节尺的
`box-shadow` / `background-image` 也一起画到了卡片顶边。

**处理**：`ModernBankPicker` 外层改成普通 `<section>`（承接编号与上沿线），`SectionCard` 放进里面，
与首页其他区块结构一致。**验证**：卡片恢复三角角标与圆角，章节编号回到卡片上方一行。

### 3.2 R2 电台面板滑杆量纲

**现象**：5 个旋钮全在轨道最左端，读数却是 30% / 60% / 45% / 80% / 20%；拖到 50 时读数
`5000%`、填充宽 25950px（轨道 519px）。

**根因**：`levels` 存 `0.0..1.0`，`ui::Slider` 默认 `min=0 / max=100 / step=1`，且调用点没传范围。

**处理**：`levels` 改为 0–100，`on_change` 夹紧到 `0..=100`，补 `aria_valuetext`。
**验证**：e2e 把第一档设为 50，断言读数 `50%`、填充宽度不超过轨道。

### 3.3 R3 指示灯与 ON/OFF

**现象**：4 个灯实测都是 `rgb(58, 68, 96)`（`--md-border-strong`），点 OFF 前后完全一致。

**根因**：`data-tone` / `data-lit` 只写在包裹 `<span>` 上，而 `style/modern/` 里没有任何规则消费它们；
像素风用的是真正的 `Led` 组件（`LedTone` + `LedState`），现代风这一份只是空壳。

**处理**：属性移到 `.md-led` 本体，在 `01-base.css` 补 `md-green` / `md-danger` / `md-gold` / `md-blue`
四档色相与 `.md-led[data-lit="off"]` 的熄灭色；选择器只作用于 `.md-led` 自身，不影响
`.md-onair-row` / `.md-live-level` 两处按父级着色的既有用法。**验证**：e2e 断言 ON 时 4 色互不相同、
OFF 后回到同一种熄灭色。

### 3.4 R4 图标分批在搜索清空后停摆

**现象**：初始 58 → 搜索 `ar` 得 8 → 清空后 24，再等 1.5s 仍是 24。

**根因**：`after_paint::grow_cap` 补满目标后把 `cap` 设为 `usize::MAX` 并返回 `false`，循环结束；
搜索时把 `cap` 重置为 24 不会重新登记，于是永久停在第一批。像素风同名板块是同一套写法，一并修。

**处理**：新增 `after_paint::grow_cap_per_filter(cap, step, first, total: Memo<usize>)`：在 `Effect` 里
订阅筛选结果、复位上限并重新登记一轮（循环本身仍是有限步结束）。两个图标板块都改用它，
删掉 `set_if_changed` 的重置。**验证**：e2e 搜索后清空，断言回到 58 个。

### 3.5 R5 复制反馈

**现象**：点「复制变量名」后「已复制到剪贴板」永久留在卡片里；提示没有 `aria-live`，读屏不播报；
用的是 `learning.*` 词条（设计页应走 `design.*`）。

**处理**：改为 `crate::ui::push_success`（自动消失的 toast），色板用既有的 `design.colors-copied`，
图标新增 `design.md-icon-copied`。与像素风同一位置的写法对齐。

### 3.6 R6 现代首页的题库守卫

**处理**：把像素首页的 `available / checking / generation` 三段搬进 `ModernHome`：版本或类别变化时
异步确认可用性，过期结果按 generation 丢弃；hero 与快速入口的两个 CTA 接同一个
`Callback<MouseEvent>`，不可用时 `prevent_default` 并 `push_info` 说明原因；检查期间选择器禁用
（与像素首页同口径）。错题本不依赖题库，不接守卫。

### 3.7 R7 文案与文档

- `radio.rs` 的 `level-N` → `design.md-radio-level`（zh「档位 {}」/ en「Level {}」/ es「Nivel {}」）。
- 西语 `design.md-demo-sonar`：`Onda de sónar` → `Onda de sonar`（该名词无重音）。
- `docs/data/i18n.md` 的域规模表：`design` 258 → 304，并顺带刷新了已经过期的其余各行与合计
  （5408 → 5746，复数词条 163 → 164）。

### 3.8 R8 e2e

`modern_ui_spec.spec.ts` 新增：滑杆拖到 50、灯 4 色与 OFF 换态、图标从 58 → 搜索 → 清空回 58；
`h3 >= 6` 换成组件族精确 6；`if (clearBox)` 改成拿不到布局盒直接失败。
`modern_a11y.spec.ts` 新增「设计规范页：七个标签零 a11y 违规」：逐个打开七个标签与通用组件的
四个子标签各扫一次（动效标签先关掉「减少动态效果」，否则无限循环让 `settle` 等不到静默）。

## 4. 已确认没有问题的方面

- **安全**：新增代码无 `innerHTML` / `eval`，无用户输入进入 `style` 或 `href`；链接参数走
  `encode_uri_component`，题库类别是枚举。
- **依赖**：`Cargo.lock` 除 workspace 版本外零改动。
- **皮肤隔离**：现代风 `/ui-spec` 七个标签 + 五个组件子标签实测 `pxl-*` 为 0；像素风 `/ui-spec`
  实测 `md-*` 为 0。
- **i18n**：新增词条三语齐全，删掉的占位词条无残留引用。
- **性能**：`TabsPanel` 懒挂载成立（首帧只建总览）；图标 24 条一批；45 个 token 只在主题变化时
  读一次计算样式；列表用 `For` 主键键控。

## 5. 已评估、决定不做

**R15 动效演示的离屏暂停**：`md-current`（波形流动）与 `md-skel-shimmer`（骨架扫光）是无限循环，
但它们的 `animation` 是**内联 style**（重播机制要求逐元素写回），样式表里的
`animation-play-state: paused` 压不过内联简写；而仓库禁止 `!important`。面板本身是懒挂载的，
切走标签时 `hidden` 已让动画停止，只在「停在本标签内滚动」时继续跑。
权衡后不做：把 6 个演示改成类驱动会推翻重播机制，收益只是 6 个小方块的合成开销。

## 6. 回归记录

| 检查 | 结果 |
| --- | --- |
| `cargo fmt --all --check` | 通过 |
| `cargo clippy -p ham-web-app --target wasm32-unknown-unknown -- -D warnings` | 通过 |
| `cargo test -p ham-web-app` | 100 passed |
| `check-structure` | 全部通过（557 个 `md-*` 与索引一致） |
| `check-ui` | 通过 |
| `check-e2e` | 495 条 journey 唯一命中 |
| `i18n-pack` + `check-i18n` | zh / en / es 各 5746 条，100% 覆盖，无死条目 |
| `check-readme` / `pixel-check` / `check-data-sync` / `check-data-staleness` / `knowledge-i18n-check` / `check-explanations` | 通过 |
| e2e `modern_ui_spec.spec.ts` | 见下方「执行结果」 |
| e2e `modern_a11y.spec.ts` | 见下方「执行结果」 |
| e2e `select.spec.ts`（双皮肤） | 见下方「执行结果」 |
| e2e `mobile_overflow` + `i18n_layout`（modern / pixel） | 见下方「执行结果」 |
| e2e `dual_isolation` | 见下方「执行结果」 |

### 执行结果

- `modern_ui_spec.spec.ts`：2 passed（含新增的滑杆 / 指示灯 / 图标重置断言）。
- `modern_a11y.spec.ts`：新增「设计规范页」1 passed（25.5s）；深色 8 页 / 亮色 3 页 / 桥接页 4 页 3 passed。
- `select.spec.ts`（双皮肤）：60 passed，2 skipped。
- `mobile_overflow` + `i18n_layout`：modern 6 passed；pixel 6 passed。
- `dual_isolation`：2 passed。
- 浏览器复核（修复前 → 修复后）：
  - 滑杆：`input.value` 与读数一致（30 / 60 / 45 / 80 / 20），填充比例 0.30 / 0.60 / 0.45 / 0.80 / 0.20；
    修复前旋钮贴在 0、拖到 50 时读数 5000%。
  - 指示灯：ON 时 4 色为 `rgb(53,227,159)` / `rgb(255,93,93)` / `rgb(255,207,107)` / `rgb(111,140,255)`，
    OFF 后统一为 `rgb(58,68,96)`；修复前 4 个恒为 `rgb(58,68,96)`。
  - 图标：58 → 搜索 8 → 清空 58；修复前清空后停在 24。
  - 首页：章节壳 `::before` 为 `position:static`（编号单行「04」），卡片角标 `absolute 18px`、
    圆角 10px；可用题库下点 CTA 正常跳到 `/practice?version=2025-10&bank=A`（守卫不误伤）。

## 7. 后续

- 像素风的图标板块已随 R4 一起修好；如果还有其他页面重复「重置 `cap` 但不再登记」的写法，
  应改用 `after_paint::grow_cap_per_filter`。
- `md-led` 的四档色相是规范页的展示契约；站内其他位置要复用时，按同样的 `data-tone` 语义接，
  不要另起一套指示灯。
- 版本号 `0.7.6` 是发布标记，与本次功能无关；若要走独立提交，可在合并前拆出。
