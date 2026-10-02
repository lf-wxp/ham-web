use ham_web_core::dxcc::lookup;
use ham_web_core::most_wanted::wanted_prefix;
use leptos::prelude::*;

use super::{INPUT, RESULT};
use crate::i18n::{t, tf};

/// 呼号查询：前缀 → DXCC 实体 / 稀有度。
#[component]
pub(super) fn CallsignLookup() -> impl IntoView {
  let call = RwSignal::new(String::new());
  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("呼号")}</span>
        <input
          type="text"
          placeholder=move || t("如 P5ABC、JA1ABC")
          prop:value=move || call.get()
          on:input=move |e| call.set(event_target_value(&e).to_uppercase())
          class=INPUT
        />
      </label>
      <div class=RESULT>
        {move || {
          let c = call.get().trim().to_owned();
          if c.is_empty() {
            t("输入呼号查询其 DXCC 实体与稀有度。")
          } else {
            let base = match lookup(&c) {
              Some(e) => tf(
                "DXCC 实体：{}（#{}，{}）　CQ {} 区 / ITU {} 区",
                &[
                  e.name,
                  &e.dxcc.to_string(),
                  e.continent,
                  &e.cq.to_string(),
                  &e.itu.to_string(),
                ],
              ),
              None => t("DXCC 实体：未识别"),
            };
            match wanted_prefix(&c) {
              Some(p) => tf("{}　稀有实体：{}（Most Wanted）", &[&(base).to_string(), (p)]),
              None => base,
            }
          }
        }}
      </div>
    </div>
  }
}
