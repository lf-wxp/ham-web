use leptos::prelude::*;

#[component]
pub(super) fn Stat(label: &'static str, #[prop(into)] value: Signal<usize>) -> impl IntoView {
  view! {
    <div class="rounded-xl border bg-card p-3">
      <div class="text-2xl font-semibold tabular-nums">{move || value.get()}</div>
      <div class="text-xs text-muted-foreground">{label}</div>
    </div>
  }
}
