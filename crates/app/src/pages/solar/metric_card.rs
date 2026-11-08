use leptos::prelude::*;

/// 展示一个实时指标。
#[component]
pub(super) fn MetricCard(
  #[prop(into)] label: Signal<String>,
  #[prop(into)] value: Signal<Option<f64>>,
  #[prop(into)] unit: Signal<String>,
  #[prop(into)] loading: Signal<bool>,
  #[prop(into)] failed: Signal<bool>,
) -> impl IntoView {
  view! {
    <div class=crate::kit::tile_class("text-center")>
      <div class="text-xs text-muted-foreground">{move || label.get()}</div>
      <div class=crate::kit::stat_text_class("mt-1")>
        {move || {
          match value.get() {
            Some(v) => format!("{v:.1}"),
            None if loading.get() => "…".to_owned(),
            None if failed.get() => "—".to_owned(),
            None => "—".to_owned(),
          }
        }}
      </div>
      <div class="mt-0.5 text-xs text-muted-foreground">{move || unit.get()}</div>
    </div>
  }
}
