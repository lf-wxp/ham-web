use leptos::prelude::*;

/// 展示一个实时指标。
#[component]
pub(super) fn MetricCard(
  label: &'static str,
  #[prop(into)] value: Signal<Option<f64>>,
  unit: &'static str,
  #[prop(into)] loading: Signal<bool>,
  #[prop(into)] failed: Signal<bool>,
) -> impl IntoView {
  view! {
    <div class="rounded-lg border bg-muted/40 p-3 text-center">
      <div class="text-xs text-muted-foreground">{label}</div>
      <div class="mt-1 text-2xl font-semibold tabular-nums">
        {move || {
          match value.get() {
            Some(v) => format!("{v:.1}"),
            None if loading.get() => "…".to_owned(),
            None if failed.get() => "—".to_owned(),
            None => "—".to_owned(),
          }
        }}
      </div>
      <div class="mt-0.5 text-[11px] text-muted-foreground">{unit}</div>
    </div>
  }
}
