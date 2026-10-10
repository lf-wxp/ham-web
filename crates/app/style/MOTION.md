# 动画 / 动效规范

本规范适用于 `crates/app` 前端（Leptos CSR + Tailwind CSS 4）的所有 CSS 动画与交互动效。目标是保证动画在移动端优先的设备上稳定 60fps、省电，且不损害可访问性。

相关文件：

- `style/input.css` — 自定义 keyframes、节奏 token、全局降级规则
- `style/tw-animate.css` — Tailwind 动画工具类（自动生成，勿手改）

## 一、核心原则

1. **合成器友好**：动画只触发合成（compositor），不触发布局（layout）或重绘（paint）。
2. **可停**：无限循环动画与定时更新在后台 / 离屏时不应空转。
3. **尊重用户偏好**：所有动画必须自动继承 `prefers-reduced-motion` 降级。

## 二、规则清单

### 1. 只动 `transform` 与 `opacity`

禁止用 `left` / `top` / `width` / `height` / `margin` 等布局属性做逐帧动画——它们每帧都会触发 layout / reflow。

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

项目实例：`input.css` 中 `.spectrum-bar::after` 的扫掠高光已从 `left` 动画改为 `translateX`。

### 2. 复用节奏 token

缓动与时长统一走 `::root` 中定义的 token，不要新写 `cubic-bezier` 魔法值：

```css
animation: hero-rise var(--motion-slow) var(--ease-out-expo) forwards;
transition: transform var(--motion-fast) var(--ease-out-expo);
```

### 3. 无限循环动画需可停

氛围类动画保持长周期 + 低复杂度。项目已有全局 `prefers-reduced-motion` 降级（见 `input.css` 末尾 `@layer base` 块），新增动画自动继承，无需单独处理。

### 4. `filter: blur` 只用于小元素

`blur-in` / `blur-out` 仅用于弹层、图标等小元素；整页或大卡片进入动画禁用 blur——低端 Android 上代价过高。

**`backdrop-filter`（玻璃模糊）同理克制**：只给「盖在别的内容上、必须看清字」的元素
（顶栏、弹层、对话框、首页的信号面板、贴边吸附的卡片）。普通卡片只用半透明底 + 顶部反光，
不模糊——一页几十张卡片同时开模糊，低端机滚动会掉帧。材质类（`.glass` / `.edge-glow` /
`.spotlight` / `.btn-solid` …）的说明见 `input.css` 的「材质语言」一节。

### 4a. 装饰性循环动画清单

全局极光（`body::after`，36s）、首页光团（`.hero-orb`，18s）、标题渐变平移（`.hero-gradient-text`，14s）
都是长周期、低复杂度的氛围动画：前两者只动 `transform`，后者平移的是文字裁剪的渐变背景，
所以**只允许用在首页那一个标题上**。窄屏（≤ 640px）极光静止；`prefers-reduced-motion`
下全部由全局降级停住。顶栏的阴影与阅读进度线用 CSS 滚动驱动动画
（`animation-timeline: scroll()`），在合成器线程上跑，没有 JS 滚动监听，不支持的浏览器整块隐形。

### 5. 闪烁频率 ≤ 3 次/秒

遵守 WCAG 2.3.1，避免光敏风险。现有 `caret-blink`（1.25s）与频谱扫掠（4.5s）均安全。

### 6. 触屏反馈走 `:active`

桌面 `hover` 态在触屏无效，用 `@media (hover: none)` 下的 `:active` 提供按压反馈（已在 `input.css` 末尾实现）。

### 7. 长列表优先分页

长列表用增量渲染控制 DOM 数量（参见 `src/pages/glossary/glossary_page.rs` 的 `visible` / `PAGE` 分页），不要依赖 `content-visibility` 补性能——它会引入滚动条跳动、`Ctrl+F` 失效等副作用。

## 三、token 速查

| Token | 值 | 用途 |
|---|---|---|
| `--ease-out-expo` | `cubic-bezier(0.16, 1, 0.3, 1)` | 出场 / 入场缓动 |
| `--motion-fast` | `0.15s` | 微交互、hover / active 反馈 |
| `--motion-normal` | `0.3s` | 常规过渡 |
| `--motion-slow` | `0.6s` | 页面入场 |
