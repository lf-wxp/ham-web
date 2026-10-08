use leptos::prelude::*;

use super::{RESULT, fmt_num};
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 驻波比 ↔ 反射系数 / 回波损耗。
#[component]
pub(super) fn SwrConverter() -> impl IntoView {
  let swr = RwSignal::new(1.5);
  let return_loss = RwSignal::new(-20.0 * ((1.5f64 - 1.0) / (1.5f64 + 1.0)).log10());

  let swr_id = unique_id("swr-value");
  let rl_id = unique_id("swr-return-loss");

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("tools.swr-2")) r#for=swr_id.clone()>
        <NumberField
          id=swr_id
          value=Signal::derive(move || swr.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(s) = v.trim().parse::<f64>()
              && s >= 1.0
            {
              swr.set(s);
              return_loss.set(-20.0 * ((s - 1.0) / (s + 1.0)).log10());
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.s11-db-negative")) r#for=rl_id.clone()>
        <NumberField
          id=rl_id
          value=Signal::derive(move || return_loss.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(rl) = v.trim().parse::<f64>()
              && rl < 0.0
            {
              return_loss.set(rl);
              let gamma = 10f64.powf(rl / 20.0);
              swr.set((1.0 + gamma) / (1.0 - gamma));
            }
          })
          controls=false
        />
      </Field>
      <div class=RESULT>
        {move || {
          let s = swr.get();
          let gamma = (s - 1.0) / (s + 1.0);
          tf(
            "tools.swr-reflection-coefficient-s11",
            &[
              &fmt_num(s),
              &fmt_num(gamma),
              &fmt_num(return_loss.get()),
              &fmt_num(-return_loss.get()),
            ],
          )
        }}
      </div>
    </div>
  }
}
