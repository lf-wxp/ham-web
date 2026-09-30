use ham_web_core::QuestionItem;
use ham_web_core::categories::{self, RefKind};
use leptos::prelude::*;

use crate::cn::cn;
use crate::icons::{Icon, IconKind};

#[component]
pub(super) fn QuestionRow(q: QuestionItem) -> impl IntoView {
  let sub = q.p_code().and_then(categories::sub_category);
  let top_cat = sub.and_then(|s| categories::top_category(s.top));
  let answers: Vec<_> = q
    .options
    .iter()
    .filter(|o| q.answer_keys.contains(&o.key))
    .cloned()
    .collect();
  let note = q.p_code().and_then(categories::sub_note);
  let refs = top_cat.map_or(&[][..], |c| categories::top_refs(c.key));
  let is_multiple = q.is_multiple();
  let answer_title = if is_multiple {
    format!("正确答案（共 {} 项）", answers.len())
  } else {
    "正确答案".to_owned()
  };

  view! {
    <article class="rounded-xl border bg-card p-4 sm:p-5">
      <div class="mb-3 flex flex-wrap items-center gap-2">
        <span class="rounded-md bg-muted px-2 py-0.5 font-mono text-xs text-muted-foreground">
          {q.id_str().or(q.j_code()).map(ToOwned::to_owned)}
        </span>
        {top_cat
          .map(|c| {
            view! {
              <span
                class="rounded-full px-2.5 py-0.5 text-xs font-semibold text-white"
                style=format!("background: {}", c.color)
              >
                {c.name}
              </span>
            }
          })}
        {sub.map(|s| view! { <span class="rounded-full bg-accent px-2.5 py-0.5 text-xs text-muted-foreground">{s.name}</span> })}
        {is_multiple
          .then(|| {
            view! {
              <span class="rounded-full bg-amber-100 px-2 py-0.5 text-xs font-semibold text-amber-700 dark:bg-amber-900/40 dark:text-amber-400">
                "多选"
              </span>
            }
          })}
        <span class="ml-auto font-mono text-xs text-muted-foreground">{q.p_code().map(ToOwned::to_owned)}</span>
      </div>

      <div class="mb-3 font-medium leading-relaxed">{q.question.clone()}</div>
      {q.image().map(|src| view! { <img src=src.to_owned() alt="题目附图" loading="lazy" class="mb-3 max-h-64 rounded-lg border" /> })}

      <div class="mb-3 rounded-lg border border-emerald-200 bg-emerald-50/60 p-3 dark:border-emerald-900/50 dark:bg-emerald-950/30">
        <div class="mb-1.5 flex items-center gap-1.5 text-xs font-semibold text-emerald-700 dark:text-emerald-400">
          <Icon kind=IconKind::CheckCircle2 class="h-3.5 w-3.5" />
          {answer_title}
        </div>
        {answers
          .into_iter()
          .map(|a| {
            view! {
              <div class="flex items-start gap-2 py-0.5 text-sm">
                <span class="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded bg-emerald-600 text-xs font-bold text-white">
                  {a.key}
                </span>
                <span class="flex-1">{a.text}</span>
              </div>
            }
          })
          .collect_view()}
      </div>

      {q
        .explanation
        .clone()
        .filter(|e| !e.is_empty())
        .map(|e| {
          view! {
            <div class="mb-3 border-t border-dashed pt-3">
              <div class="mb-1 text-xs font-semibold text-muted-foreground">"解析"</div>
              <div class="whitespace-pre-line text-sm leading-6 text-muted-foreground">{e}</div>
            </div>
          }
        })}

      {note
        .map(|n| {
          view! {
            <div class="mb-3 border-t border-dashed pt-3">
              <div class="mb-1 text-xs font-semibold text-muted-foreground">
                "知识点 · " {sub.map(|s| s.name)}
              </div>
              <div class="text-sm leading-6 text-muted-foreground">{n}</div>
            </div>
          }
        })}

      {(!refs.is_empty())
        .then(|| {
          view! {
            <div class="border-t border-dashed pt-3">
              <div class="mb-1 text-xs font-semibold text-muted-foreground">"参考依据"</div>
              <ul class="space-y-1">
                {refs
                  .iter()
                  .map(|r| {
                    let badge = cn(&[
                      "mt-0.5 shrink-0 rounded px-1.5 py-0.5 text-[10px] font-bold",
                      if r.kind == RefKind::Law {
                        "bg-indigo-100 text-indigo-700 dark:bg-indigo-900/40 dark:text-indigo-300"
                      } else {
                        "bg-emerald-100 text-emerald-700 dark:bg-emerald-900/40 dark:text-emerald-300"
                      },
                    ]);
                    let body = match r.url {
                      Some(url) => view! {
                        <a
                          href=url
                          target="_blank"
                          rel="noopener noreferrer"
                          class="inline-flex items-center gap-1 underline-offset-2 hover:underline"
                        >
                          {r.text}
                          <Icon kind=IconKind::ExternalLink class="h-3 w-3" />
                        </a>
                      }
                      .into_any(),
                      None => view! { <span>{r.text}</span> }.into_any(),
                    };
                    view! {
                      <li class="flex items-start gap-2 text-sm text-muted-foreground">
                        <span class=badge>{r.kind.label()}</span>
                        {body}
                      </li>
                    }
                  })
                  .collect_view()}
              </ul>
            </div>
          }
        })}
    </article>
  }
}
