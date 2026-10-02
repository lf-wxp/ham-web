use leptos::prelude::*;

use super::INPUT;
use crate::i18n::{t, tf};

/// 变压器阻抗变换：匝数比 Np:Ns = √(Zp/Zs)。
#[component]
pub(super) fn TransformerCalculator() -> impl IntoView {
  let zp = RwSignal::new(200.0);
  let zs = RwSignal::new(50.0);

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("初级阻抗 Zp（Ω）")}</span>
        <input
          type="number"
          prop:value=move || zp.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              zp.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("次级阻抗 Zs（Ω）")}</span>
        <input
          type="number"
          prop:value=move || zs.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              zs.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <div class="sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let p = zp.get();
          let s = zs.get();
          if p <= 0.0 || s <= 0.0 {
            t("请输入正的初级与次级阻抗。")
          } else {
            let n = (p / s).sqrt();
            tf(
              "匝数比 Np:Ns = {}:1　·　阻抗比 Zp:Zs = {}:1",
              &[&format!("{n:.3}"), &format!("{:.3}", p / s)],
            )
          }
        }}
      </div>
      <p class="sm:col-span-2 text-xs text-muted-foreground">
        {move || t("理想变压器的匝数比平方等于阻抗比。实际需考虑磁芯损耗、漏感与频率范围，宽带应用需选用合适的磁材与绕法。")}
      </p>
    </div>
  }
}
