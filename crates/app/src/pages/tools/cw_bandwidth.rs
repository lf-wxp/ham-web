use leptos::prelude::*;

use super::{RESULT, fmt_num};
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField, RadioGroup, RadioGroupItem};
use crate::util::unique_id;

/// CW 必要带宽估算：Bn = B × K，B = WPM / 1.2，K 取 5（衰落信道）或 3（非衰落）。
#[component]
pub(super) fn CwBandwidth() -> impl IntoView {
  let wpm = RwSignal::new(25.0);
  let fading = RwSignal::new(true);

  let wpm_id = unique_id("cw-bandwidth-wpm");
  let fading_id = unique_id("cw-bandwidth-fading");
  let non_fading_id = unique_id("cw-bandwidth-non-fading");

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("tools.keying-speed-wpm")) r#for=wpm_id.clone()>
        <NumberField
          id=wpm_id
          value=Signal::derive(move || wpm.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              wpm.set(v);
            }
          })
          controls=false
        />
      </Field>
      <RadioGroup
        value=Signal::derive(move || if fading.get() { "fading" } else { "non-fading" }.to_string())
        on_change=Callback::new(move |v: String| fading.set(v == "fading"))
        class="flex flex-wrap items-end gap-4 pb-2"
      >
        <label class="flex cursor-pointer items-center gap-2 text-sm">
          <RadioGroupItem value="fading" id=fading_id />
          {move || t("tools.fading-k-5")}
        </label>
        <label class="flex cursor-pointer items-center gap-2 text-sm">
          <RadioGroupItem value="non-fading" id=non_fading_id />
          {move || t("tools.non-fading-k-3")}
        </label>
      </RadioGroup>
      <div class=RESULT>
        {move || {
          let k = if fading.get() { 5.0 } else { 3.0 };
          let b = wpm.get() / 1.2;
          tf(
            "tools.baud-rate-b-bd",
            &[&fmt_num(b), &fmt_num(b * k)],
          )
        }}
      </div>
    </div>
  }
}
