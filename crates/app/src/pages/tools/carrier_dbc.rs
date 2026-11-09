use ham_web_core::decibel::{dbc_between, spur_dbm};
use leptos::prelude::*;

use super::{fmt_num, result_class};
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 载波相对电平（dBc）：载波电平、杂散电平与相对电平三者互为输入，改一个联动另外两个。
#[component]
pub(super) fn CarrierDbc() -> impl IntoView {
  let carrier = RwSignal::new(50.0);
  let relative = RwSignal::new(-60.0);
  let spur = RwSignal::new(spur_dbm(50.0, -60.0));

  let carrier_id = unique_id("dbc-carrier");
  let relative_id = unique_id("dbc-relative");
  let spur_id = unique_id("dbc-spur");

  view! {
    <div class="space-y-4">
      <div class="grid gap-3 sm:grid-cols-3">
        <Field label=Signal::derive(move || t("tools.dbc-carrier-level")) r#for=carrier_id.clone()>
          <NumberField
            id=carrier_id
            value=Signal::derive(move || carrier.get().to_string())
            on_change=Callback::new(move |v: String| {
              // 只接受有限值：换算只是加减，`inf` / `NaN` 一旦写进信号就再也回不来。
              if let Ok(c) = v.trim().parse::<f64>()
                && c.is_finite()
              {
                carrier.set(c);
                spur.set(spur_dbm(c, relative.get_untracked()));
              }
            })
            controls=false
          />
        </Field>
        <Field label=Signal::derive(move || t("tools.dbc-relative-level")) r#for=relative_id.clone()>
          <NumberField
            id=relative_id
            value=Signal::derive(move || relative.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(d) = v.trim().parse::<f64>()
                && d.is_finite()
              {
                relative.set(d);
                spur.set(spur_dbm(carrier.get_untracked(), d));
              }
            })
            controls=false
          />
        </Field>
        <Field label=Signal::derive(move || t("tools.dbc-spur-level")) r#for=spur_id.clone()>
          <NumberField
            id=spur_id
            value=Signal::derive(move || spur.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(s) = v.trim().parse::<f64>()
                && s.is_finite()
              {
                spur.set(s);
                relative.set(dbc_between(carrier.get_untracked(), s));
              }
            })
            controls=false
          />
        </Field>
      </div>

      <div class=result_class()>
        {move || {
          tf(
            "tools.dbc-result",
            &[
              &fmt_num(carrier.get()),
              &fmt_num(spur.get()),
              &fmt_num(relative.get()),
            ],
          )
        }}
      </div>

      <p class="text-xs text-muted-foreground">{move || t("tools.dbc-note")}</p>
    </div>
  }
}
