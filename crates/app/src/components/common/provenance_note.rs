//! 溯源标注：页面底部一行「依据 XX，YYYY-MM 核对」。
//!
//! 依据文本走 `kt()`（知识库文案），只有核心表里有条目的域才渲染 —— 来源没把握就不标。

use ham_web_core::provenance;
use leptos::prelude::*;

use crate::data;
use crate::i18n::tf;

#[component]
pub fn ProvenanceNote(domain: &'static str) -> impl IntoView {
  // 宿主页面不一定套了 KnowledgePage（法规 / 波段 / 卫星页都没套），依据译文得自己确保在拉。
  Effect::new(move |_| data::ensure_knowledge_i18n());
  let p = provenance::for_domain(domain);
  // 「没条目就不标」是有意的，但 domain 拼错也会表现成不标 —— 开发构建里喊一声。
  #[cfg(debug_assertions)]
  if p.is_none() {
    web_sys::console::warn_1(&format!("ProvenanceNote：溯源表里没有域「{domain}」").into());
  }
  view! {
    {p.map(|p| {
      view! {
        <p class="text-xs text-muted-foreground">
          {move || tf("common.provenance-line", &[&data::kt_tracked(p.source), p.checked_on])}
        </p>
      }
        .into_any()
    })}
  }
}
