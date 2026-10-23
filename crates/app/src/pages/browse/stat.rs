use leptos::prelude::*;

#[component]
pub(super) fn Stat(
  #[prop(into)] label: String,
  #[prop(into)] value: Signal<usize>,
) -> impl IntoView {
  view! {
    <div class="pxl-card border bg-card p-3">
      <div class="pxl-stat">{move || value.get()}</div>
      <div class="text-xs text-muted-foreground">{label}</div>
    </div>
  }
}
