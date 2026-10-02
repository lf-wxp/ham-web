use crate::i18n::t;
use leptos::prelude::*;

/// 太阳通量近 12 个月柱状趋势。
#[component]
pub(super) fn FluxTrend(history: Vec<(String, f64)>) -> impl IntoView {
  if history.len() < 2 {
    return view! { <div></div> }.into_any();
  }
  let max = history.iter().map(|(_, v)| *v).fold(f64::MIN, f64::max);
  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("太阳通量趋势（近 12 个月）")}</h2>
      <div class="flex items-end gap-1 p-4">
        {history
          .iter()
          .map(|(m, v)| {
            let h = (v / max * 120.0).max(4.0);
            let label = m.get(2..).unwrap_or(m);
            view! {
              <div class="flex flex-1 flex-col items-center gap-1" title=format!("{m}：{v:.0}")>
                <div class="w-full rounded-t bg-primary" style=format!("height: {h:.1}px")></div>
                <span class="text-[9px] tabular-nums text-muted-foreground">{label}</span>
              </div>
            }
          })
          .collect_view()}
      </div>
    </section>
  }
  .into_any()
}
