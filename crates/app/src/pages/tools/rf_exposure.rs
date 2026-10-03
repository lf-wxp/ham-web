use ham_web_core::rf_exposure::assess;
use leptos::prelude::*;

use super::{INPUT, fmt_num};
use crate::i18n::{t, tf};

/// 射频暴露合规评估：按 FCC OET-65 估算功率密度与最小安全距离。
#[component]
pub(super) fn RfExposure() -> impl IntoView {
  let power = RwSignal::new(100.0);
  let freq = RwSignal::new(14.0);
  let gain = RwSignal::new(2.15);
  let distance = RwSignal::new(10.0);

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("发射功率（W）")}</span>
        <input
          type="number"
          prop:value=move || power.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              power.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("频率（MHz）")}</span>
        <input
          type="number"
          prop:value=move || freq.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              freq.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("天线增益（dBi）")}</span>
        <input
          type="number"
          prop:value=move || gain.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              gain.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("评估距离（m）")}</span>
        <input
          type="number"
          prop:value=move || distance.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              distance.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <div class="sm:col-span-2 space-y-1 rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
        {move || {
          match assess(power.get(), freq.get(), gain.get(), distance.get()) {
            Some(r) => {
              let ok_public = r.power_density <= r.mpe_uncontrolled;
              view! {
                <div class="tabular-nums">
                  {tf("功率密度：{} mW/cm²", &[&fmt_num(r.power_density)])}
                </div>
                <div class="tabular-nums">
                  {tf(
                    "公众限值 {} mW/cm²，受控限值 {} mW/cm²",
                    &[&fmt_num(r.mpe_uncontrolled), &fmt_num(r.mpe_controlled)],
                  )}
                </div>
                <div class="tabular-nums">
                  {tf(
                    "最小安全距离：公众 {} m，受控 {} m",
                    &[
                      &fmt_num(r.safe_distance_uncontrolled_m),
                      &fmt_num(r.safe_distance_controlled_m),
                    ],
                  )}
                </div>
                <div>
                  {move || {
                    if ok_public {
                      t("当前距离下满足公众环境限值。")
                    } else {
                      t("当前距离下超过公众环境限值，请增大距离或降低功率。")
                    }
                  }}
                </div>
              }
              .into_any()
            }
            None => view! { <span>{move || t("请输入正的功率、频率与距离。")}</span> }.into_any(),
          }
        }}
      </div>
      <p class="sm:col-span-2 text-xs text-muted-foreground">
        {move || t("远场近似：S = P·G/(4πd²)。近场区实际场强可能更高，结果仅供合规自检参考。")}
      </p>
    </div>
  }
}
