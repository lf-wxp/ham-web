//! 全站 Web Threads 动态背景层（React Bits `WebThreads` 的 Leptos 版）。
//!
//! 低阶画布见 [`crate::components::web_threads::WebThreads`]，渲染引擎见
//! [`crate::web_threads`]；这里只负责「按主题挑预设、按设备挑渲染模式」。
//!
//! # 接入方式
//!
//! 在应用根布局放一个 [`WebThreadsBackground`] 即可，它会自动跟随 `light` / `dark`
//! 主题切换配色，并以固定定位铺在内容之下：
//!
//! ```ignore
//! view! {
//!   <WebThreadsBackground />
//!   <Navigation />
//!   <MainContent />
//! }
//! ```
//!
//! 容器被标记为 `aria-hidden`，纯粹作为装饰；`pointer-events: none` + 负 `z-index`
//! 使指针交互不遮挡前景。只应在应用根布局挂载一次。
//!
//! # 移动端
//!
//! 触屏设备上使用 [`RenderMode::Static`]：只渲染**一帧**静态画面当背景图用，
//! 不启动动画循环 —— 全屏片元着色器持续重绘是实打实的耗电项，而静止的背景在
//! 同一位置看起来完全一样。判定见 [`is_mobile`]。

use leptos::prelude::*;

use crate::theme::{resolve_theme_color, use_theme};
use crate::util::window;
use crate::web_threads::{RenderMode, ThreadsConfig, hex_to_rgb};

use super::web_threads::WebThreads;

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
