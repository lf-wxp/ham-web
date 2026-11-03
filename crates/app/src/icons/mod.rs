//! 像素图标：[`IconKind`] 的对外 API 保持不变，内部统一画成 24×24 网格的像素图。
//!
//! 图标来源与生成方式见 `pixel/`。`IconKind` 沿用 Lucide 的名字：`name()` / `icon_of()` 的
//! 反向映射被注册表（`ham_web_core::registry`）依赖，所以枚举与名字都不动，只换了渲染。

mod badge;
mod icon_kind;
mod icon_lookup;
mod modern_map;
mod pixel;

pub use badge::badge_sprite;
pub use icon_kind::IconKind;
pub use icon_lookup::icon_of;
pub use pixel::PixelSprite;

use leptos::prelude::*;

/// 精灵名是否存在（供其它模块的单元测试校验自己的映射表）。
#[cfg(test)]
pub fn sprite_exists(name: &str) -> bool {
  pixel::sprite_data::sprite_by_name(name).is_some()
}

/// 渲染一个像素图标。
///
/// 路径是整数坐标的方块，`crispEdges` 保证任何缩放比例下边缘都不糊；
/// 默认尺寸 24px = 1 格 1px，调用方的 `h-4 w-4`（16px）会按 2/3 缩小，边缘会有轻微的
/// 抗锯齿过渡，所以推荐用 `size-6`（24px）与 `size-12`（48px）这类整数倍。
#[component]
pub fn Icon(kind: IconKind, #[prop(optional, into)] class: Signal<String>) -> impl IntoView {
  // 现代风：换成线性图标（`ui/` 的日历、chevron、勾选等控件都用这个组件）。
  if crate::kit::is_modern_at_mount()
    && let Some((icon, extra)) = modern_map::to_modern(kind)
  {
    // `class` 是响应式的（下拉箭头靠它在展开时加 `rotate-180`）：以信号传下去，
    // 在 `get_untracked()` 里取值会把旋转冻死在首帧。
    return view! {
      <crate::kit_modern::ModernIcon
        kind=icon
        class=Signal::derive(move || format!("{extra} {}", class.get()))
      />
    }
    .into_any();
  }
  let class = move || format!("pxl-icon pxl-icon-{} {}", kind.name(), class.get());
  view! {
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="currentColor"
      shape-rendering="crispEdges"
      class=class
      aria-hidden="true"
    >
      <path d=pixel::icon_path(kind) />
    </svg>
  }
  .into_any()
}
