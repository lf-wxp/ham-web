use ham_web_core::fresnel::{
  FULL_CLEARANCE_RATIO, K_STANDARD, MIN_CLEARANCE_RATIO, PathInput, Verdict, analyze,
};
use leptos::prelude::*;

use super::{RESULT, fmt_num};
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 菲涅尔区与视距：障碍物是否侵入第一菲涅尔区，以及地球曲率的影响。
#[component]
pub(super) fn FresnelZone() -> impl IntoView {
  let freq = RwSignal::new(145.0);
  let dist = RwSignal::new(30.0);
  let tx_height = RwSignal::new(100.0);
  let rx_height = RwSignal::new(100.0);
  // 取 4 位小数，输入框里才不会显示 1.3333333333333333。
  let k = RwSignal::new((K_STANDARD * 1e4).round() / 1e4);
  let obstacle_km = RwSignal::new(15.0);
  let obstacle_height = RwSignal::new(0.0);

  let freq_id = unique_id("fresnel-freq");
  let dist_id = unique_id("fresnel-dist");
  let tx_id = unique_id("fresnel-tx-height");
  let rx_id = unique_id("fresnel-rx-height");
  let k_id = unique_id("fresnel-k");
  let obstacle_km_id = unique_id("fresnel-obstacle-km");
  let obstacle_height_id = unique_id("fresnel-obstacle-height");

  let report = move || {
    analyze(&PathInput {
      freq_mhz: freq.get(),
      dist_km: dist.get(),
      tx_height_m: tx_height.get(),
      rx_height_m: rx_height.get(),
      k: k.get(),
      obstacle_km: obstacle_km.get(),
      obstacle_height_m: obstacle_height.get(),
    })
  };

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("log.frequency-mhz")) r#for=freq_id.clone()>
        <NumberField
          id=freq_id
          value=Signal::derive(move || freq.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              freq.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.distance-km")) r#for=dist_id.clone()>
        <NumberField
          id=dist_id
          value=Signal::derive(move || dist.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              dist.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.fresnel-antenna-a-height-m")) r#for=tx_id.clone()>
        <NumberField
          id=tx_id
          value=Signal::derive(move || tx_height.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              tx_height.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.fresnel-antenna-b-height-m")) r#for=rx_id.clone()>
        <NumberField
          id=rx_id
          value=Signal::derive(move || rx_height.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              rx_height.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.fresnel-obstacle-distance-km")) r#for=obstacle_km_id.clone()>
        <NumberField
          id=obstacle_km_id
          value=Signal::derive(move || obstacle_km.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              obstacle_km.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.fresnel-obstacle-height-m")) r#for=obstacle_height_id.clone()>
        <NumberField
          id=obstacle_height_id
          value=Signal::derive(move || obstacle_height.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              obstacle_height.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.fresnel-k-factor")) r#for=k_id.clone()>
        <NumberField
          id=k_id
          value=Signal::derive(move || k.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              k.set(v);
            }
          })
          controls=false
        />
      </Field>
      <div class=format!("{RESULT} space-y-1 tabular-nums")>
        {move || match report() {
          Some(r) => {
            let verdict = match r.verdict {
              Verdict::Full => t("tools.fresnel-verdict-full"),
              Verdict::Sufficient => t("tools.fresnel-verdict-sufficient"),
              Verdict::Partial => t("tools.fresnel-verdict-partial"),
              Verdict::Blocked => t("tools.fresnel-verdict-blocked"),
            };
            let required_pct = format!("{:.0}", MIN_CLEARANCE_RATIO * 100.0);
            let full_pct = format!("{:.0}", FULL_CLEARANCE_RATIO * 100.0);
            view! {
              <div role="status" class="font-medium text-foreground">{verdict}</div>
              <div>
                {tf(
                  "tools.fresnel-zone-result",
                  &[
                    &fmt_num(r.wavelength_m),
                    &fmt_num(r.fresnel1_at_obstacle_m),
                    &fmt_num(r.fresnel1_mid_m),
                  ],
                )}
              </div>
              <div>
                {tf(
                  "tools.fresnel-bulge-result",
                  &[&fmt_num(r.bulge_m), &fmt_num(r.los_height_m), &fmt_num(r.obstacle_effective_m)],
                )}
              </div>
              <div>
                {tf(
                  "tools.fresnel-clearance-result",
                  &[
                    &fmt_num(r.clearance_m),
                    &format!("{:.0}", r.clearance_ratio * 100.0),
                    &required_pct,
                    &full_pct,
                  ],
                )}
              </div>
              <div>
                {if r.margin_60_m >= 0.0 {
                  tf("tools.fresnel-margin-spare", &[&fmt_num(r.margin_60_m)])
                } else {
                  tf("tools.fresnel-margin-needed", &[&fmt_num(-r.margin_60_m)])
                }}
              </div>
              <div>
                {tf(
                  "tools.fresnel-horizon-result",
                  &[&fmt_num(r.horizon_km), &fmt_num(k.get())],
                )}
              </div>
            }
            .into_any()
          }
          None => view! { <span>{move || t("tools.fresnel-enter-valid-values")}</span> }.into_any(),
        }}
      </div>
      <p class="sm:col-span-2 text-xs text-muted-foreground">
        {move || t("tools.fresnel-assumptions")}
      </p>
    </div>
  }
}
