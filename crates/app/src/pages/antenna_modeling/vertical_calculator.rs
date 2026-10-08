//! 垂直天线尺寸计算器。

use ham_web_core::antenna_design::vertical_dims;
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 垂直天线尺寸计算器。
#[component]
pub(super) fn VerticalCalculator() -> impl IntoView {
  let freq = RwSignal::new(14.2);
  let k = RwSignal::new(0.95);

  // `Field` 的标签与控件是兄弟节点，`r#for` / `id` 必须配对才能点击标签聚焦输入框
  //（e2e 与读屏都按「标签 → 控件」的关联来定位）。
  let freq_id = unique_id("vertical-freq");
  let k_id = unique_id("vertical-k");

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
        label=Signal::derive(move || t("knowledge.velocity-factor-k-0"))
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
          let Some((radiator, radial)) = vertical_dims(freq.get(), k.get()) else {
            return t("knowledge.enter-a-valid-frequency");
          };
          // 先按固定小数位格式化数值，再交给 `tf` 填占位符 —— 译文里只需一个 `{}`。
          tf(
            "common.radiator-m-radials-each",
            &[&format!("{radiator:.2}"), &format!("{radial:.2}")],
          )
        }}
      </div>
    </div>
  }
}
