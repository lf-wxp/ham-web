//! Web Threads 动态背景组件（React Bits `WebThreads` 的 Leptos 版）。
//!
//! 渲染引擎见 [`crate::web_threads`]，这里只负责「把画布挂到 DOM 上」与「跟随主题热更新」。
//!
//! # 接入方式
//!
//! 1. **全站背景（推荐）**：在应用根布局放一个 [`WebThreadsBackground`] 即可，它会
//!    自动跟随 `light` / `dark` 主题切换配色，并以固定定位铺在内容之下：
//!
//!    ```ignore
//!    view! {
//!      <WebThreadsBackground />
//!      <Navigation />
//!      <MainContent />
//!    }
//!    ```
//!
//! 2. **局部 / 自定义**：用一个 `<div class="relative h-96">` 包裹 [`WebThreads`]，
//!    组件会铺满父容器。通过 `config` 传入 [`ThreadsConfig`]（或 `Signal`）控制外观，
//!    传 `Signal` 时可在运行时热更新（只改 uniform，不重建 WebGL 上下文）：
//!
//!    ```ignore
//!    let config = RwSignal::new(ThreadsConfig { thread_count: 9.0, ..Default::default() });
//!    view! {
//!      <div class="h-96 w-full overflow-hidden rounded-xl">
//!        <WebThreads config=config />
//!      </div>
//!    }
//!    ```
//!
//! 两个组件都把 DOM 标记为 `aria-hidden`，纯粹作为装饰；[`WebThreadsBackground`] 的
//! 容器加了 `pointer-events: none`，指针交互在 `window` 上监听，因此不遮挡前景。
//!
//! # 移动端
//!
//! 触屏设备上 [`WebThreadsBackground`] 使用 [`RenderMode::Static`]：只渲染**一帧**
//! 静态画面当背景图用，不启动动画循环 —— 全屏片元着色器持续重绘是实打实的耗电项，
//! 而静止的背景在同一位置看起来完全一样。判定见 [`is_mobile`]。

use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::theme::use_theme;
use crate::util::{document, window};
use crate::web_threads::{RenderMode, ThreadsConfig, WebThreadsRenderer, hex_to_rgb};

/// 是否移动端（触屏优先的设备）。
///
/// 以「主输入是不是触摸」判定，而不是看视口宽度 —— 桌面浏览器把窗口拖窄不该被
/// 当成移动端。触屏设备上：
///
/// 1. 背景只渲染**一帧静态画面**（[`RenderMode::Static`]），不跑动画，
///    全屏片元着色器的持续重绘就此归零；
/// 2. 鼠标扰动分支（着色器里每像素一次 `exp()`）也一并关掉 —— 触屏上它本来就没用。
///
/// 老浏览器不支持该媒体查询时同样按移动端处理：宁可背景不动，也不白耗电。
fn is_mobile() -> bool {
  !window()
    .match_media("(hover: hover)")
    .ok()
    .flatten()
    .is_some_and(|m| m.matches())
}

/// 两套主题共用的基础配置：触屏上关掉无用的鼠标扰动。
fn base() -> ThreadsConfig {
  ThreadsConfig {
    mouse_interaction: !is_mobile(),
    ..ThreadsConfig::default()
  }
}

/// 深色主题预设：透明画布上叠加冷色发光，贴合站点 teal 品牌色。
fn dark_preset() -> ThreadsConfig {
  ThreadsConfig {
    color1: hex_to_rgb("#2EE6A8"),
    color2: hex_to_rgb("#38BDF8"),
    color3: hex_to_rgb("#E8FFF6"),
    background: hex_to_rgb("#060B10"),
    // 压在正文之下的装饰层，亮度刻意低于上游默认：既保留发光质感，
    // 又让叠在上面的浅色文字保持足够对比度。
    brightness: 0.45,
    opacity: 0.75,
    ..base()
  }
}

/// 浅色主题预设：开启 `light_mode`，画布自带底色并输出「墨线」。
///
/// 底色直接读取主题令牌 `--background`（与页面背景一致，切换主题后依旧无缝）；
/// 令牌不可解析时回退到静态值 `#FAFBFC`（与 `index.html` 的 `theme-color` 一致）。
fn light_preset() -> ThreadsConfig {
  ThreadsConfig {
    color1: hex_to_rgb("#0F766E"),
    color2: hex_to_rgb("#0E7490"),
    color3: hex_to_rgb("#FFFFFF"),
    background: resolve_theme_color("--background").unwrap_or(hex_to_rgb("#FAFBFC")),
    // 浅色下「墨线」比深色的发光更含蓄，略微提高覆盖度才能看清。
    brightness: 1.0,
    opacity: 0.85,
    light_mode: true,
    ..base()
  }
}

/// 按明暗取预设。
fn preset(dark: bool) -> ThreadsConfig {
  if dark { dark_preset() } else { light_preset() }
}

/// 读取主题 CSS 变量并解析成 sRGB 分量。
///
/// 借一个 1×1 的 2D canvas 把颜色真正**光栅化**成 sRGB：`fillStyle` 的字符串序列化对
/// 现代色彩空间（`oklch()` / `color(display-p3)` 等）会按原空间返回，直接解析字符串会
/// 失败；而 `getImageData` 读到的是浏览器已完成色彩空间转换后的 sRGB 分量，因此能覆盖
/// 所有 CSS 颜色写法，让浅色画布的底色与页面背景严丝合缝，不依赖硬编码。
fn resolve_theme_color(name: &str) -> Option<(f32, f32, f32)> {
  let root = document().document_element()?;
  let computed = window().get_computed_style(&root).ok()??;
  let raw = computed.get_property_value(name).ok()?;
  let raw = raw.trim();
  if raw.is_empty() {
    return None;
  }
  let probe: HtmlCanvasElement = document().create_element("canvas").ok()?.dyn_into().ok()?;
  probe.set_width(1);
  probe.set_height(1);
  let ctx: CanvasRenderingContext2d = probe.get_context("2d").ok()??.dyn_into().ok()?;
  // 先写入一个哨兵色并取像素：若 `raw` 无法解析，`set_fill_style_str` 会静默忽略，
  // 光栅化出的仍是哨兵色，据此判定失败。
  ctx.set_fill_style_str("#010203");
  ctx.fill_rect(0.0, 0.0, 1.0, 1.0);
  ctx.set_fill_style_str(raw);
  ctx.fill_rect(0.0, 0.0, 1.0, 1.0);
  let px = ctx.get_image_data(0.0, 0.0, 1.0, 1.0).ok()?.data();
  if px.len() < 4 {
    return None;
  }
  let (r, g, b) = (px[0], px[1], px[2]);
  if (r, g, b) == (1, 2, 3) {
    return None;
  }
  Some((
    f32::from(r) / 255.0,
    f32::from(g) / 255.0,
    f32::from(b) / 255.0,
  ))
}

/// 铺满父容器的 Web Threads 画布。
///
/// - `config`：外观参数，可直接传 [`ThreadsConfig`] 或任意 `Signal`（运行时热更新）；
/// - `class`：追加到容器 `div` 上的类名（默认铺满父容器，由父级控制尺寸与裁剪）。
///
/// 浏览器不支持 WebGL2 时静默降级为空白容器，不影响页面其余部分。
#[component]
pub fn WebThreads(
  /// 外观参数；传 `Signal` 时可在运行时热更新（只改 uniform，不重建上下文）。
  #[prop(into)]
  config: Signal<ThreadsConfig>,
  /// 渲染模式，默认 [`RenderMode::Animated`]（静止背景传 [`RenderMode::Static`]）。
  #[prop(optional)]
  mode: RenderMode,
  /// 容器类名。
  #[prop(optional)]
  class: &'static str,
) -> impl IntoView {
  let node = NodeRef::<html::Div>::new();
  let renderer = StoredValue::new_local(None::<WebThreadsRenderer>);

  node.on_load(move |el| {
    renderer.set_value(WebThreadsRenderer::new(&el, &config.get_untracked(), mode));
  });

  // 配置变化（含主题切换）只更新 uniform，不重建 WebGL 上下文。
  Effect::new(move |_| {
    let config = config.get();
    renderer.update_value(|renderer| {
      if let Some(renderer) = renderer.as_ref() {
        renderer.apply(&config);
      }
    });
  });

  on_cleanup(move || renderer.update_value(|renderer| *renderer = None));

  view! {
    <div node_ref=node class=class aria-hidden="true"></div>
  }
}

/// 全站动态背景层：固定铺满视口、置于内容之下，并自动跟随明暗主题。
///
/// 作为页面背景使用，不影响前景内容与交互（`pointer-events: none` + 负 `z-index`）。
/// 只应在应用根布局挂载一次。
///
/// 桌面端为动画（受帧率上限约束）；移动端只渲染一帧静态画面当背景图（见 [`is_mobile`]）。
#[component]
pub fn WebThreadsBackground() -> impl IntoView {
  let theme = use_theme();
  let config = Memo::new(move |_| preset(theme.is_dark()));
  // 是否动画在挂载时定下即可：判定依据是「主输入是否为触摸」，
  // 它不会因为旋转屏幕或缩放窗口而改变。
  let mode = if is_mobile() {
    RenderMode::Static
  } else {
    RenderMode::Animated
  };
  view! {
    <div class="web-threads-layer" aria-hidden="true">
      <WebThreads config=config mode=mode class="block h-full w-full" />
    </div>
  }
}
