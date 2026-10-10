# 动画 / 动效规范

本规范适用于 `crates/app` 前端（Leptos CSR + Tailwind CSS 4）的所有 CSS 动画与交互动效。目标是保证动画在移动端优先的设备上稳定 60fps、省电，且不损害可访问性。

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
