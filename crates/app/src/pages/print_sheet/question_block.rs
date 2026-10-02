use leptos::prelude::*;

use super::{AnswerMode, Item, explanation};
use crate::i18n::{t, tf};

/// 打印版中的一道题。
#[component]
pub(crate) fn QuestionBlock(
  index: usize,
  item: Item,
  answers: AnswerMode,
  explain: bool,
) -> impl IntoView {
  let q = item.q;
  let inline = answers == AnswerMode::Inline;
  let code = q.j_code().map(ToOwned::to_owned);
  let explain_text = (explain && inline).then(|| explanation(&q)).flatten();
  view! {
    <article class="print-avoid-break border-b border-zinc-200 py-3 last:border-0">
      <div class="flex gap-2 text-[13px] leading-relaxed">
        <span class="shrink-0 font-semibold tabular-nums">{format!("{index}.")}</span>
        <div class="min-w-0 flex-1">
          <p>
            {q.question.clone()}
            {q.is_multiple().then(|| view! { <span class="ml-1 text-[11px] text-zinc-500">{move || t("（多选）")}</span> })}
          </p>
          {q.image_url.clone().map(|src| view! { <img src=src alt="题目附图" class="my-2 max-h-48 max-w-full" /> })}
          <ul class="mt-1 grid gap-x-6 gap-y-0.5 sm:grid-cols-2 print:grid-cols-2">
            {q.options.iter().map(|o| {
              let right = inline && q.answer_keys.contains(&o.key);
              view! {
                <li class=if right { "font-semibold underline decoration-2 underline-offset-2" } else { "" }>
                  {format!("{}. {}", o.key, o.text)}
                </li>
              }
            }).collect_view()}
          </ul>
          <div class="mt-1 flex flex-wrap gap-x-3 text-[11px] text-zinc-500">
            {code.map(|c| view! { <span>{c}</span> })}
            {(item.wrong > 0).then(|| view! { <span>{tf("答错 {} 次", &[&(item.wrong).to_string()])}</span> })}
            {inline.then(|| view! { <span class="font-semibold text-zinc-900">{tf("答案：{}", &[&(q.answer_keys.join("")).to_string()])}</span> })}
          </div>
          {explain_text.map(|t| view! {
            <p class="mt-1.5 rounded bg-zinc-100 px-2 py-1 text-[12px] leading-relaxed text-zinc-700">{tf("解析：{}", &[&(t).to_string()])}</p>
          })}
        </div>
      </div>
    </article>
  }
}
