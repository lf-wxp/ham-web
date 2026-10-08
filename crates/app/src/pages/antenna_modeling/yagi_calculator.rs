//! Yagi 天线振子尺寸计算器。

use ham_web_core::antenna_design::yagi_dims;
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// Yagi 振子尺寸计算器。
#[component]
pub(super) fn YagiCalculator() -> impl IntoView {
  let freq = RwSignal::new(14.2);
  let directors = RwSignal::new(2usize);

  // `Field` 的标签与控件是兄弟节点，`r#for` / `id` 必须配对才能点击标签聚焦输入框
  //（e2e 与读屏都按「标签 → 控件」的关联来定位）。
  let freq_id = unique_id("yagi-freq");
  let directors_id = unique_id("yagi-directors");

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
        label=Signal::derive(move || t("knowledge.number-of-directors-0"))
        r#for=directors_id.clone()
      >
        <NumberField
          id=directors_id
          step=1.0
          min=0.0
          max=5.0
          value=Signal::derive(move || directors.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<usize>() {
              directors.set(v.min(5));
            }
          })
          controls=false
        />
      </Field>
      <div class="sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let Some(d) = yagi_dims(freq.get(), directors.get()) else {
            return t("tools.enter-a-positive-frequency");
          };
          let mut parts = vec![
            tf("common.reflector-m", &[&format!("{:.2}", d.reflector)]),
            tf("common.driven-element-m", &[&format!("{:.2}", d.driven)]),
          ];
          for (i, len) in d.directors.iter().enumerate() {
            parts.push(tf(
              "common.director-m",
              &[&(i + 1).to_string(), &format!("{:.2}", len)],
            ));
          }
          parts.push(tf(
            "common.reflector-driven-spacing-m",
            &[
              &format!("{:.2}", d.refl_spacing),
              &format!("{:.2}", d.dir_spacing),
            ],
          ));
          parts.join("　")
        }}
      </div>
    </div>
  }
}
