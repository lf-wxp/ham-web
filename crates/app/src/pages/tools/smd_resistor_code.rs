//! SMD 电阻丝印代码 → 阻值。

use ham_web_core::component_codes;
use leptos::prelude::*;

use super::{RESULT_INLINE, fmt_resistance};
use crate::i18n::t;
use crate::ui::{Field, Input, InputType};
use crate::util::unique_id;

#[component]
pub(super) fn SmdResistorCode() -> impl IntoView {
  let code = RwSignal::new(String::from("472"));
  let id = unique_id("smd-resistor-code");

  view! {
    <div class="grid gap-3">
      <Field
        label=Signal::derive(move || t("tools.marking-code"))
        hint=Signal::derive(move || t("tools.smd-resistor-code-hint"))
        r#for=id.clone()
      >
        <Input
          id=id
          value=Signal::derive(move || code.get())
          on_change=Callback::new(move |v: String| code.set(v))
          kind=InputType::Text
        />
      </Field>
      <div class=RESULT_INLINE>
        {move || match component_codes::smd_resistor(&code.get()) {
          Some(r) => fmt_resistance(r.ohms),
          None => t("tools.code-unrecognized"),
        }}
      </div>
    </div>
  }
}
