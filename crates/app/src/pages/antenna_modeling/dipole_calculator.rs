use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;
use leptos::prelude::*;

/// 偶极天线尺寸估算计算器。
#[component]
pub(super) fn DipoleCalculator() -> impl IntoView {
  let freq = RwSignal::new(14.2);
  let k = RwSignal::new(0.95);

  // `Field` 的标签与控件是兄弟节点，`r#for` / `id` 必须配对才能点击标签聚焦输入框
  //（e2e 与读屏都按「标签 → 控件」的关联来定位）。
  let freq_id = unique_id("dipole-freq");
  let k_id = unique_id("dipole-k");

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("log.frequency-mhz")) r#for=freq_id.clone()>
        <NumberField
          id=freq_id
          step=0.01
          value=Signal::derive(move || freq.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              freq.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field
        label=Signal::derive(move || t("knowledge.velocity-factor-k-0-2"))
        r#for=k_id.clone()
      >
        <NumberField
          id=k_id
          step=0.01
          min=0.5
          max=1.0
          value=Signal::derive(move || k.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              k.set(v);
            }
          })
          controls=false
        />
      </Field>
      <div class="sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let f = freq.get();
          let kk = k.get().clamp(0.5, 1.0);
          if f <= 0.0 {
            t("tools.enter-a-positive-frequency")
          } else {
            let half = 150.0 / f * kk;
            // 建议架高约半波长（150/f），不是 1/4 波长（75/f）。
            tf("common.half-wave-dipole-m", &[&format!("{half:.2}"), &format!("{:.2}", half / 2.0), &format!("{:.2}", 75.0 / f * kk), &format!("{:.2}", 150.0 / f)])
          }
        }}
      </div>
    </div>
  }
}
