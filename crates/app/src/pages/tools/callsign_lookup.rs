use ham_web_core::dxcc::lookup;
use ham_web_core::most_wanted::wanted_prefix;
use leptos::prelude::*;

use super::{INPUT, RESULT};

/// 呼号查询：前缀 → DXCC 实体 / 稀有度。
#[component]
pub(super) fn CallsignLookup() -> impl IntoView {
  let call = RwSignal::new(String::new());
  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">"呼号"</span>
        <input
          type="text"
          placeholder="如 P5ABC、JA1ABC"
          prop:value=move || call.get()
          on:input=move |e| call.set(event_target_value(&e).to_uppercase())
          class=INPUT
        />
      </label>
      <div class=RESULT>
        {move || {
          let c = call.get().trim().to_owned();
          if c.is_empty() {
            "输入呼号查询其 DXCC 实体与稀有度。".to_owned()
          } else {
            let base = match lookup(&c) {
              Some(e) => format!(
                "DXCC 实体：{}（#{}，{}）　CQ {} 区 / ITU {} 区",
                e.name, e.dxcc, e.continent, e.cq, e.itu
              ),
              None => "DXCC 实体：未识别".to_owned(),
            };
            match wanted_prefix(&c) {
              Some(p) => format!("{base}　稀有实体：{p}（Most Wanted）"),
              None => base,
            }
          }
        }}
      </div>
    </div>
  }
}
