//! 瓷片电容丝印代码 → 容值（含容差字母）。

use ham_web_core::component_codes;
use ham_web_core::unit_math::format_capacitance_f;
use leptos::prelude::*;

use super::RESULT_INLINE;
use crate::i18n::t;
use crate::ui::{Field, Input, InputType};
use crate::util::unique_id;

#[component]
pub(super) fn CapacitorCode() -> impl IntoView {
  let code = RwSignal::new(String::from("104"));
  let id = unique_id("capacitor-code");

  view! {
    <div class="grid gap-3">
      <Field
        label=Signal::derive(move || t("tools.marking-code"))
        hint=Signal::derive(move || t("tools.capacitor-code-hint"))
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
        {move || match component_codes::capacitor_code(&code.get()) {
          Some(c) => {
            let mut out = format_capacitance_f(c.farads);
            if let Some(tol) = c.tolerance {
              out.push_str(" · ");
              out.push_str(tol);
            }
            out
          }
          None => t("tools.code-unrecognized"),
        }}
      </div>
    </div>
  }
}
