use leptos::prelude::*;

/// 信号格：由矮到高的几根小柱，点亮 `lit` 根（通联路线的 1–3 格信号、搜索结果的匹配度）。
///
/// 纯装饰（`aria-hidden`）：格数的文字版本由调用方给（「2 / 3 格信号」）。
#[component]
pub fn SignalBars(#[prop(into)] lit: Signal<u8>, #[prop(default = 3)] total: u8) -> impl IntoView {
  view! {
    <span class="pxl-sigbars" aria-hidden="true">
      {(0..total)
        .map(|i| {
          view! {
            <span
              style=format!("height: {}px", (u16::from(i) + 1) * 4)
              data-lit=move || (i < lit.get()).then_some("")
            ></span>
          }
        })
        .collect_view()}
    </span>
  }
}
