//! 6 m 方向分布：把现有 6 m spot 的方位按八方位数一遍，回答「现在 6 m 开向哪几个方向」。
//!
//! 数据全由调用方算好（`ham_web_core::spot_bearing`）；本台网格没填时只给提示，不显示一张空的分布。

use ham_web_core::spot_coverage::SECTORS;
use leptos::prelude::*;

use crate::i18n::t;

/// 八方位字母，下标同 `spot_coverage::sector_of_bearing`。
const NAMES: [&str; SECTORS] = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];

#[component]
pub fn SixMeterSummary(
  /// 本台网格是否可用：不可用就没有原点，算不出任何方位。
  home_known: Signal<bool>,
  /// 各方位的 spot 数。
  counts: Signal<[usize; SECTORS]>,
) -> impl IntoView {
  view! {
    <div class="space-y-1 border-t pt-2" data-testid="six-summary">
      {move || {
        if !home_known.get() {
          return view! {
            <p class="text-xs text-muted-foreground">{t("radio.six-no-home")}</p>
          }
            .into_any();
        }
        let c = counts.get();
        if c.iter().all(|&n| n == 0) {
          return view! {
            <p class="text-xs text-muted-foreground">{t("radio.six-summary-empty")}</p>
          }
            .into_any();
        }
        view! {
          <p class="flex flex-wrap items-baseline gap-x-3 gap-y-1 text-xs">
            <span class="text-muted-foreground">{t("radio.six-summary-title")}</span>
            {NAMES
              .iter()
              .zip(c)
              .filter(|(_, n)| *n > 0)
              .map(|(name, n)| {
                view! {
                  <span class="font-mono tabular-nums" data-sector=*name>
                    {format!("{name} {n}")}
                  </span>
                }
              })
              .collect_view()}
          </p>
        }
          .into_any()
      }}
      <p class="text-xs text-muted-foreground">{t("radio.six-note")}</p>
    </div>
  }
}
