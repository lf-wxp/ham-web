//! Web Threads 动态背景画布（React Bits `WebThreads` 的 Leptos 版）。
//!
//! 渲染引擎见 [`crate::web_threads`]，这里只负责「把画布挂到 DOM 上」与「跟随主题热更新」。
//! 全站背景层（自动跟随明暗主题、按设备挑渲染模式）见
//! [`crate::components::web_threads_background::WebThreadsBackground`]。
//!
//! # 接入方式
//!
//! 低阶组件铺满父容器，用一个 `<div class="relative h-96">` 包裹即可。通过 `config`
//! 传入 [`ThreadsConfig`]（或 `Signal`）控制外观，传 `Signal` 时可在运行时热更新
//! （只改 uniform，不重建 WebGL 上下文）：
//!
//! ```ignore
//! let config = RwSignal::new(ThreadsConfig { thread_count: 9.0, ..Default::default() });
//! view! {
//!   <div class="h-96 w-full overflow-hidden rounded-xl">
//!     <WebThreads config=config />
//!   </div>
//! }
//! ```
//!
//! 组件把 DOM 标记为 `aria-hidden`，纯粹作为装饰。

use leptos::html;
use leptos::prelude::*;

use crate::web_threads::{RenderMode, ThreadsConfig, WebThreadsRenderer};

/// 铺满父容器的 Web Threads 画布。
///
/// - `config`：外观参数，可直接传 [`ThreadsConfig`] 或任意 `Signal`（运行时热更新）；
/// - `class`：追加到容器 `div` 上的类名（默认铺满父容器，由父级控制尺寸与裁剪）；
/// - `mode`：渲染模式，默认 [`RenderMode::Animated`]（静止背景传 [`RenderMode::Static`]）。
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
