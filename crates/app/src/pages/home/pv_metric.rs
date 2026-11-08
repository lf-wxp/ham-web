use leptos::prelude::*;

/// 传播指标卡片。
#[component]
pub(super) fn PvMetric(
  /// 指标名称（中文原文，同时作为 i18n key）。
  #[prop(into)]
  label: Signal<String>,
  #[prop(into)] value: Signal<Option<f64>>,
  #[prop(into)] loading: Signal<bool>,
) -> impl IntoView {
  view! {
    <div class=crate::kit::tile_class("text-center")>
      <div class="text-xs text-muted-foreground">{move || label.get()}</div>
      <div class=crate::kit::stat_text_class("mt-1")>
        {move || match value.get() {
          Some(v) => format!("{v:.0}"),
          None if loading.get() => "…".to_owned(),
          None => "—".to_owned(),
        }}
      </div>
    </div>
  }
}
