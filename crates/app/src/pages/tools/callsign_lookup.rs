use ham_web_core::dxcc::lookup;
use ham_web_core::most_wanted::wanted_prefix;
use leptos::prelude::*;

use super::RESULT;
use crate::i18n::{t, tf};
use crate::ui::{Field, Input};
use crate::util::unique_id;

/// 呼号查询：前缀 → DXCC 实体 / 稀有度。
#[component]
pub(super) fn CallsignLookup() -> impl IntoView {
  let call = RwSignal::new(String::new());
  let call_id = unique_id("callsign-lookup");
  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("log.callsign")) r#for=call_id.clone()>
        <Input
          id=call_id
          value=call
          on_change=Callback::new(move |v: String| call.set(v.to_uppercase()))
          placeholder=Signal::derive(move || t("tools.e-g-p5abc-ja1abc"))
        />
      </Field>
      <div class=RESULT>
        {move || {
          let c = call.get().trim().to_owned();
          if c.is_empty() {
            t("tools.enter-a-callsign-to")
          } else {
            let base = match lookup(&c) {
              Some(e) => tf(
                "tools.dxcc-entity-cq-itu",
                &[
                  e.name,
                  &e.dxcc.to_string(),
                  e.continent,
                  &e.cq.to_string(),
                  &e.itu.to_string(),
                ],
              ),
              None => t("tools.dxcc-entity-unknown"),
            };
            match wanted_prefix(&c) {
              Some(p) => tf("tools.rare-entity-most-wanted", &[&(base).to_string(), (p)]),
              None => base,
            }
          }
        }}
      </div>
    </div>
  }
}
