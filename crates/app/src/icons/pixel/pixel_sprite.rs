//! 像素精灵：角色 / Boss / 怪物 / 成就徽章 / 地图节点。
//!
//! 数据由 `cargo make pixel-sprites`（`crates/tools/src/pixel/sprites.rs`）从字符画生成（`sprite_data.rs`），
//! 渲染时每种颜色一个 `<path>`，整张精灵只有十来个节点，开销可以忽略。
//!
//! 清晰度：`shape-rendering="crispEdges"` 关掉抗锯齿，`image-rendering: pixelated` 对内联
//! SVG 无效，所以不靠它；尺寸取「像素格数」的整数倍（16 格 → 32 / 48 / 64px）才不会出现
//! 半个像素的糊边，调用方用 `size` 传**每格的屏幕像素数**而不是总边长，从接口上杜绝非整数倍。

use leptos::prelude::*;

use super::sprite_data::{SpriteData, sprite_by_name};

/// 渲染一张精灵。
///
/// - `name`：精灵名，见 `sprite_data.rs` 里 `sprite_by_name` 的列表（如 `hero` / `boss` /
///   `badge_trophy` / `mon_noise`）。名字不存在时什么都不渲染 —— 数据是编译期常量，
///   传错名字是编码错误，由 [`super::sprite_data`] 的单元测试覆盖，不在运行时刷屏报错。
/// - `scale`：每个像素格占多少屏幕像素（默认 3：16 格 → 48px）。
/// - `label`：给读屏的名字；缺省视为纯装饰（`aria-hidden`）。
#[component]
pub fn PixelSprite(
  #[prop(into)] name: Signal<&'static str>,
  #[prop(default = 3)] scale: u8,
  #[prop(optional, into)] label: Option<Signal<String>>,
  #[prop(optional, into)] class: Signal<String>,
) -> impl IntoView {
  view! {
    {move || {
      let data: &'static SpriteData = sprite_by_name(name.get())?;
      let px = u32::from(data.size) * u32::from(scale.max(1));
      let paths = data
        .layers
        .iter()
        .map(|l| view! { <path fill=l.fill d=l.d /> })
        .collect_view();
      let aria_label = label.map(|l| l.get());
      Some(
        view! {
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width=px
            height=px
            viewBox=format!("0 0 {0} {0}", data.size)
            shape-rendering="crispEdges"
            class=move || format!("pxl-sprite shrink-0 {}", class.get())
            role=aria_label.as_ref().map(|_| "img")
            aria-label=aria_label.clone()
            aria-hidden=aria_label.is_none().then_some("true")
          >
            {paths}
          </svg>
        },
      )
    }}
  }
}
