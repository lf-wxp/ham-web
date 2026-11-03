use leptos::prelude::*;

#[component]
pub(super) fn Stat(
  #[prop(into)] label: Signal<String>,
  #[prop(into)] value: Signal<usize>,
) -> impl IntoView {
  view! {
    <div class=crate::kit::card_class_with("p-3")>
      <div class=crate::kit::stat_class()>{move || value.get()}</div>
      <div class="text-xs text-muted-foreground">{move || label.get()}</div>
    </div>
  }
}
