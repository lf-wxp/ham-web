# 动画 / 动效规范

本规范适用于 `crates/app` 前端（Leptos CSR + Tailwind CSS 4）的所有 CSS 动画与交互动效。**一至三节是像素风；现代风见第四节；弹层的进出场（两套皮肤共用的「在场」实现）见第五节；控件状态矩阵见第六节。**目标是保证动画在移动端优先的设备上稳定 60fps、省电，且不损害可访问性。

相关文件：

- `style/input.css` — 只做 `@import` 与 `@source`；样式按 `style/pixel/*.css` 分工
- `style/pixel/tokens.css` — 节奏 token（`--motion-*` / `--ease-*`）
- `style/pixel/motion.css` — 像素动画 keyframes、弹层进出场、全局降级规则
- `style/tw-animate.css` — Tailwind 动画工具类（自动生成，勿手改）

## 一、核心原则

1. **合成器友好**：动画只触发合成（compositor），不触发布局（layout）或重绘（paint）。
2. **可停**：无限循环动画与定时更新在后台 / 离屏时不应空转。
3. **尊重用户偏好**：所有动画必须自动继承 `prefers-reduced-motion` 降级。

## 二、规则清单

### 1. 只动 `transform` 与 `opacity`

禁止用 `left` / `top` / `width` / `height` / `margin` 等布局属性做逐帧动画——它们每帧都会触发 layout / reflow。
像素主题另外允许 `background-position`（骨架 / 扫掠高光这类只重绘的动效，见 `pxl-scroll-x`），其余仍只走
`transform` / `opacity`。

```css
/* 反例：left 触发 reflow */
@keyframes bad {
  from { left: -28%; }
  to   { left: 100%; }
}

/* 正例：translateX 只走合成器 */
@keyframes good {
  to { transform: translateX(357.14%); }
}
```

项目实例：`.spectrum-bar` 的扫掠高光从 `left` 动画改成了 `background-position-x`（`pxl-scroll-x`，`surfaces.css`）。
像素动效一律配 `steps()` 逐格跳，不用平滑插值 —— 「跳格」既省合成器时间，也是风格本身。

### 2. 复用节奏 token

缓动与时长统一走 `:root` 中定义的 token（像素主题里缓动也是 `steps()`），不要新写 `cubic-bezier` 魔法值：

```css
animation: hero-rise var(--motion-slow) var(--ease-out-expo) forwards;
transition: transform var(--motion-fast) var(--ease-out-expo);
```

### 3. 无限循环动画需可停

氛围类动画保持长周期 + 低复杂度。项目已有全局降级（见 `style/pixel/motion.css` 末尾 `@layer base` 里的两个块：
`@media (prefers-reduced-motion: reduce)` 与 `html[data-pixel-motion="off"]`），新增动画自动继承，无需单独处理。

### 4. `filter: blur` 只用于小元素

`blur-in` / `blur-out` 仅用于弹层、图标等小元素；整页或大卡片进入动画禁用 blur——低端 Android 上代价过高。

**`backdrop-filter`（毛玻璃）在像素主题里一律禁用**：`surfaces.css` 顶部有一条全局兜底，
把存量页面里残留的 `backdrop-blur*` 与 `--blur-*` 全部归零。层次感靠 2px 硬描边 + 硬投影
（`.pxl-window`）表达，而不是半透明模糊 —— 一页几十张卡片同时开模糊，低端机滚动会掉帧。
新加的材质不要再引入模糊。

### 4a. 装饰性循环动画清单

长周期、低复杂度，且都在 `prefers-reduced-motion` / `data-pixel-motion="off"` 下停住：

- 暗色星点轮换 `.dark body::after`（`pxl-twinkle`，6s，`steps(1, end)`）；
- 骨架屏两档明暗跳动 `.skeleton`（`skeleton-sweep`，1.2s）；
- 频谱条扫掠高光 `.spectrum-bar`（`pxl-scroll-x`，1.6s，`steps(16, end)`）；
- 精灵 / 图标上下浮动 `.pxl-bob`、`.pxl-bob-sprite`（1s）。

背景网点（`body::before`）与首页网格（`.hero-grid`）是静态底纹，不耗动画预算。
路由切换的「读条」`.route-sweep` 是纯装饰，降级时直接 `display: none`（见 `motion.css`）。

### 5. 闪烁频率 ≤ 3 次/秒

遵守 WCAG 2.3.1，避免光敏风险。现有 `pxl-blink`（0.6s / 1s，`steps(1, end)` 一格一跳）、
`caret-blink`（1.25s）、骨架 `skeleton-sweep`（1.2s）、星点 `pxl-twinkle`（6s）均安全。

### 6. 触屏反馈走 `:active`

桌面 `hover` 态在触屏无效，用 `:active` 提供按压反馈：`.pxl-btn:active`（`controls.css`）
把按钮下沉 2px 并让外投影归零，`.motion-lift` / `.motion-press`（`motion.css`）同理。

### 7. 长列表优先分页

长列表用增量渲染控制 DOM 数量（参见 `src/pages/glossary/glossary_page.rs` 的 `visible` / `PAGE` 分页），不要依赖 `content-visibility` 补性能——它会引入滚动条跳动、`Ctrl+F` 失效等副作用。

## 三、token 速查

| Token | 值 | 用途 |
|---|---|---|
| `--ease-out-expo` | `steps(4, end)` | 出场 / 入场缓动（像素主题里缓动也是跳格，值在 `tokens.css`） |
| `--motion-instant` | `0.08s` | 按下 / 释放这种「立刻」的反馈 |
| `--motion-fast` | `0.12s` | 微交互、hover / active 反馈 |
| `--motion-normal` | `0.24s` | 常规过渡、弹层进出场 |
| `--motion-slow` | `0.48s` | 页面入场、路由读条 |

## 四、现代风（`style/modern/`）

本文前三节写的是像素风（`steps()` 逐格跳）。现代风是另一套节奏，规则同样要守：

| Token | 值 | 用途 |
|---|---|---|
| `--md-dur-fast` | `120ms` | 控件的悬停 / 状态过渡（描边、底色、勾选）、悬停气泡、下拉面板 |
| `--md-dur` | `180ms` | 对话框 / 抽屉 / Toast / 菜单的进出场、按钮的悬停 |
| `--md-dur-slow` | `320ms` | 较大的位移 / 版式变化 |
| `--md-ease` / `--md-ease-out` | `cubic-bezier(…)` | 过渡 / 入场用 ease-out，退场用 ease |

- **进出场的时长一律走 token**，不写 `180ms` 这样的字面量。装饰性的长动画（氛围循环、一次性演出、涟漪）可以写字面量。
- 现代风**不用 `steps()`**，像素风**不用平滑缓动**；两套皮肤的动画不要互相借用（现代风的单选框曾借用像素风的
  `motion-check`，逐格跳的动画漏进了平滑的界面里）。
- 减少动态效果：`tokens.css` 的 `@media (prefers-reduced-motion: reduce)` 把 `.md-root` 与各个 Portal 出来的表面
  （对话框 / 抽屉 / 调谐台 / 气泡 / Toast / 提示气泡）的时长压到 1ms。**新增一个 Portal 表面，要把它的根类加进这份名单。**

## 五、弹层的进出场：一份「在场」实现，一个时长 token

两套皮肤的弹层（下拉 / 日期 / 时间 / 对话框 / 抽屉 / Toast / 提示气泡 / 现代风的六个表面）共用 `src/presence.rs`：
关闭后元素先留在 DOM 里播完退场动画，再卸载。几条硬规矩，每一条都踩过：

1. **退场是动画，不是过渡。** 入场动画与 `data-state` 在同一次样式变更里被移除时，浏览器不会从动画值起一个过渡，
   `transition: opacity` 因此从来没播过 —— 像素风的面板在关闭后第一帧就是透明的（逐帧实测）。
   写 `[data-state="closed"] { animation: …-out <token> … both/forwards }`。
2. **一个 token，两个使用者。** 退场动画的时长与 Rust 里的卸载延迟读**同一个** CSS 变量：

   | 档位 | 现代风 | 像素风 | 谁用 |
   |---|---|---|---|
   | `ExitSpeed::Normal` | `--md-dur` | `--motion-normal` | 对话框 / 抽屉 / 下拉 / 日期 / 时间 / Toast / 现代风六个表面 |
   | `ExitSpeed::Fast` | `--md-dur-fast` | `--motion-fast` | 悬停气泡 |

   `presence::exit_ms(speed)` 在每次关闭时现读（用户运行中切风格，两套 token 的值不同），再加一帧。
   退场动画的时长只能是这两档的 token（`check-structure` 的「退场时长走 token」守着，写死 `180ms` 会红）。
   **不要**在 Rust 里再写 `EXIT_MS` 常量（「延迟卸载只有一份」守着）。
3. **退场期间不拦截点击。** `data-state="closed"` 的遮罩与面板都是 `pointer-events: none`
   （下拉遮罩与面板类里的 `data-[state=closed]:pointer-events-none`、`.md-modal` / `.md-sheet` / `.md-bubble` 的关闭态规则）。
   否则用户松手后紧接着点别处，那一下会被一层看不见的全屏遮罩吞掉约 200ms。
4. **代际号，不是只看 `open`。** 计时器到点时 `open` 恰好又是 `false`（关 → 开 → 关）会把第二次退场截断在半路；
   打开与关闭都要让代际号自增，计时器只在代际号没变时才卸载。
5. **定时器回调里只用 `try_*` 访问信号。** 退场途中跳转页面会销毁信号，`get_untracked` 会 panic 带崩整个 wasm 实例。

## 六、控件状态矩阵

同一个控件在两套皮肤下，每种状态都要有反馈。下表是**实现后在浏览器里量到的**（`e2e/tests/ui_motion.spec.ts` 逐项守着）：

| 状态 | 像素风 | 现代风 |
|---|---|---|
| 悬停 | 80ms `steps(2)`：描边 / 底色 / 位移 | `--md-dur-fast` 平滑：描边 / 底色（输入框、下拉触发器、勾选、分页、页签、胶囊）；按钮 `--md-dur`（含主按钮的扫光，有意的设计） |
| 按压 | 2px 下沉（按钮 / 分页） | 涟漪 `md-ping`（按钮 / 胶囊 / 图标按钮 / 分页，**禁用的不播**）；整块表面是声呐环 |
| 键盘聚焦 | 2px 虚线环 | 2px 实线环 |
| 禁用 | 按钮：网点遮罩 + 降饱和；输入 0.7；勾选 / 开关 / 滑块 0.5 | 一律 `opacity: 0.5` + `not-allowed`，主按钮去掉橙色投影；**悬停 / 按下 / 涟漪全部排除** |
| 错误 | `aria-invalid` → `--destructive` 描边（输入 / 下拉 / 勾选 / 开关 / 单选组） | 同，`--md-danger` |
| 勾选 / 选中 | `check-pop` 120ms 逐格 | `md-check-pop` 120ms 平滑（复选框与单选框同一个） |

- `aria-invalid` 是唯一的事实来源：组件用 `invalid` 入参（`Checkbox` / `Switch` / `Input` / `Select` / …；单选的属性挂在
  `radiogroup` 上、样式往下传），CSS 只认属性。**`Slider` 没有 `invalid`**：连续量没有「校验未通过」这个状态，不是遗漏。
- 每个变体的悬停规则各写各的，**新增按钮变体时悬停 / 按下规则必须带 `:not(:disabled)`**（`.md-btn-accent` 曾漏掉，
  于是禁用的主按钮悬停时仍会扫光、上抬）。
- 现代风的 Tab / 复选框 / 开关 / 滑块**没有**涟漪，这是有意的：它们是「状态切换」而不是「去做某事」，
  涟漪会让一排切换控件显得吵。
