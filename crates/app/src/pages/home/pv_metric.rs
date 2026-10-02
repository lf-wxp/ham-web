use leptos::prelude::*;

/// 传播指标卡片。
#[component]
pub(super) fn PvMetric(
  /// 指标名称（中文原文，同时作为 i18n key）。
  #[prop(into)]
  label: String,
  #[prop(into)] value: Signal<Option<f64>>,
  #[prop(into)] loading: Signal<bool>,
) -> impl IntoView {
  view! {
    <div class="rounded-lg border bg-muted/40 p-3 text-center">
      <div class="text-xs text-muted-foreground">{label}</div>
      <div class="mt-1 text-2xl font-semibold tabular-nums">
        {move || match value.get() {
          Some(v) => format!("{v:.0}"),
          None if loading.get() => "…".to_owned(),
          None => "—".to_owned(),
        }}
      </div>
    </div>
  }
}
