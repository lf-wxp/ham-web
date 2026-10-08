use leptos::prelude::*;

use super::{RESULT, fmt_num};
use crate::i18n::t;
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 频率 ↔ 波长换算（λ = 300 / f(MHz)）。
#[component]
pub(super) fn FreqWavelength() -> impl IntoView {
  let freq = RwSignal::new(145.0);
  let wl = RwSignal::new(300.0 / 145.0);

  // `Field` 的标签与控件是兄弟节点，`r#for` / `id` 必须配对才能点击标签聚焦输入框
  //（e2e 与读屏都按「标签 → 控件」的关联来定位）。
  let freq_id = unique_id("freq-wavelength-freq");
  let wl_id = unique_id("freq-wavelength-wl");

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("log.frequency-mhz")) r#for=freq_id.clone()>
        <NumberField
          id=freq_id
          value=Signal::derive(move || freq.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(f) = v.trim().parse::<f64>()
              && f > 0.0
            {
              freq.set(f);
              wl.set(300.0 / f);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.wavelength-m")) r#for=wl_id.clone()>
        <NumberField
          id=wl_id
          value=Signal::derive(move || wl.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(w) = v.trim().parse::<f64>()
              && w > 0.0
            {
              wl.set(w);
              freq.set(300.0 / w);
            }
          })
          controls=false
        />
      </Field>
      <div class=RESULT>
        {move || format!("{} MHz ≈ {} m", fmt_num(freq.get()), fmt_num(wl.get()))}
      </div>
    </div>
  }
}
