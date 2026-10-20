use crate::i18n::t;
use crate::ui::Tooltip;
use leptos::prelude::*;

/// 太阳通量近 12 个月柱状趋势。
#[component]
pub(super) fn FluxTrend(history: Vec<(String, f64)>) -> impl IntoView {
  if history.len() < 2 {
    return view! { <div></div> }.into_any();
  }
  let max = history.iter().map(|(_, v)| *v).fold(f64::MIN, f64::max);
  view! {
    <section class="pxl-card border bg-card">
      <h2 class="pxl-section-title">{move || t("radio.solar-flux-trend-last")}</h2>
      <div class="flex items-end gap-1 p-4">
        {history
          .iter()
          .map(|(m, v)| {
            let h = (v / max * 120.0).max(4.0);
            let label = m.get(2..).unwrap_or(m).to_owned();
            let tip = format!("{m}：{v:.0}");
            view! {
              <Tooltip class="flex-1" label=tip.clone()>
                <div
                  class="flex w-full flex-col items-center gap-1 outline-none focus-visible:outline-2 focus-visible:outline-dashed focus-visible:outline-ring"
                  tabindex="0"
                  role="img"
                  aria-label=tip
                >
                  <div class="w-full bg-primary" style=format!("height: {h:.1}px")></div>
                  <span class="text-[9px] tabular-nums text-muted-foreground">{label}</span>
                </div>
              </Tooltip>
            }
          })
          .collect_view()}
      </div>
    </section>
  }
  .into_any()
}
