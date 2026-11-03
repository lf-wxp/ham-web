//! 页脚：按界面风格派发（像素风虚线上沿 / 现代风细分隔线）。

use leptos::prelude::*;

use super::pixel_footer::PixelFooter;

#[component]
pub(super) fn Footer() -> impl IntoView {
  view! {
    {move || {
      if crate::kit::is_modern() {
        view! { <crate::kit_modern::ModernFooter /> }.into_any()
      } else {
        view! { <PixelFooter /> }.into_any()
      }
    }}
  }
}
