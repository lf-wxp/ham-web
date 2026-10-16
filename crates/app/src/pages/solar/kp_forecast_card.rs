use ham_web_core::kp_forecast::KpForecast;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::data;
use crate::i18n::t;

use super::kp_outlook::KpOutlook;

/// Kp 三日预报卡片：昨天加未来三天，每 3 小时一格；预测期最高值与 NOAA 磁暴级别。
#[component]
pub(super) fn KpForecastCard(#[prop(into)] refresh: Signal<u32>) -> impl IntoView {
  let data = RwSignal::new(None::<KpForecast>);
  let loading = RwSignal::new(true);
  let poll = RwSignal::new(0u32);
  let clock = RwSignal::new((js_sys::Date::now() / 60_000.0) as i64);
  if let Ok(handle) = set_interval_with_handle(
    move || poll.update(|p| *p = p.wrapping_add(1)),
    std::time::Duration::from_secs(600),
  ) {
    on_cleanup(move || handle.clear());
  }
  if let Ok(handle) = set_interval_with_handle(
    move || clock.set((js_sys::Date::now() / 60_000.0) as i64),
    std::time::Duration::from_secs(60),
  ) {
    on_cleanup(move || handle.clear());
  }
  Effect::new(move |_| {
    let ticket = (refresh.get(), poll.get());
    loading.set(true);
    spawn_local(async move {
      let fetched = data::fetch_external_json_live::<KpForecast>("/api/kp-forecast")
        .await
        .ok();
      if refresh.try_get_untracked().zip(poll.try_get_untracked()) != Some(ticket) {
        return;
      }
      data.set(fetched);
      loading.set(false);
    });
  });

  view! {
    <section class="pxl-card border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.kp-forecast-heading")}</h2>
      <div class="space-y-3 p-4 text-sm">
        {move || {
          if loading.get() {
            return view! { <p class="text-muted-foreground">{move || t("radio.kp-forecast-loading")}</p> }.into_any();
          }
          let now = clock.get();
          let Some(out) = data.get().and_then(|f| ham_web_core::kp_forecast::live_outlook(&f.points, now).map(|o| (f, o))) else {
            return view! { <p class="text-muted-foreground">{move || t("radio.kp-forecast-unavailable")}</p> }.into_any();
          };
          let (f, o) = out;
          view! { <KpOutlook forecast=f outlook=o /> }.into_any()
        }}
      </div>
    </section>
  }
}
