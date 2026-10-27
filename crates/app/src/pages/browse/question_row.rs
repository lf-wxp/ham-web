use std::collections::HashSet;

use ham_web_core::QuestionItem;
use ham_web_core::categories::{self, RefKind};
use leptos::prelude::*;

use crate::cn::cn;
use crate::i18n::{t, tf, tp};
use crate::icons::{Icon, IconKind};
use crate::store;
use crate::tone;
use crate::ui::ChipToggle;

#[component]
pub(super) fn QuestionRow(
  q: QuestionItem,
  /// 列表页持有的收藏快照：本行从里面读自己的状态，切换时就地改它。
  /// 不直接读 `store::is_bookmarked()`：那是普通函数、不参与响应式，点了不会变色。
  bookmarks: RwSignal<HashSet<String>>,
) -> impl IntoView {
  // 没有稳定编号（`id` 与 `J` 都缺）的题不给收藏入口：位置兜底键 `pos:N` 随列表顺序变，
  // 写进存储后练习页对不上、收藏集也列不出来，用户还取消不了。
  let bookmark_toggle = q.stable_id().map(|id| {
    // 每行一个 Memo：快照整体变了，只有本题那一位翻转的行才继续通知下游绑定。
    let marked = Memo::new({
      let id = id.clone();
      move |_| bookmarks.with(|set| set.contains(&id))
    });
    // 名称恒定、带题号：切换状态只交给 `aria-pressed`，读屏也分得清是哪一题。
    let label = Signal::derive({
      let id = id.clone();
      move || tf("exam.bookmark-question", &[&id])
    });
    let title = Signal::derive(move || {
      if marked.get() {
        t("exam.remove-bookmark")
      } else {
        t("exam.bookmark-this-question")
      }
    });
    // 以存储为准：`toggle_bookmark` 返回切换后的真实状态，不信按钮传来的目标值。
    let on_change = Callback::new(move |_: bool| {
      let added = store::toggle_bookmark(&id);
      bookmarks.update(|set| {
        if added {
          set.insert(id.clone());
        } else {
          set.remove(&id);
        }
      });
    });
    view! {
      <ChipToggle active=marked on_change=on_change aria_label=label title=title>
        {view! {
          {move || if marked.get() { t("exam.bookmarked") } else { t("exam.bookmark") }}
        }
          .into_any()}
      </ChipToggle>
    }
  });
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
    tp(
      "common.correct-answers-items",
      answers.len(),
      &[&(answers.len()).to_string()],
    )
  } else {
    t("exam.correct-answer")
  };

  view! {
    <article class="pxl-card border bg-card p-4 sm:p-5">
      <div class="mb-3 flex flex-wrap items-center gap-2">
        <span class="bg-muted px-2 py-0.5 font-mono text-xs text-muted-foreground">
          {q.id_str().or(q.j_code()).map(ToOwned::to_owned)}
        </span>
        {top_cat
          .map(|c| {
            let chip = tone::chip(c.tone);
            view! {
              <span class=format!("border-2 px-2 py-0.5 text-xs text-foreground {chip}")>
                {move || t(c.name)}
              </span>
            }
          })}
        {sub.map(|s| view! { <span class="bg-accent px-2.5 py-0.5 text-xs text-muted-foreground">{s.name}</span> })}
        {is_multiple
          .then(|| {
            view! {
              <span class="bg-gold/10 px-2 py-0.5 text-xs text-gold-text">
                {move || t("exam.multiple")}
              </span>
            }
          })}
        // 编号与收藏同组：窄屏换行时两者一起靠右，不会让收藏单独落在一行的最左边。
        <span class="ml-auto flex items-center gap-2">
          <span class="font-mono text-xs text-muted-foreground">{q.p_code().map(ToOwned::to_owned)}</span>
          {bookmark_toggle}
        </span>
      </div>

      <div class="mb-3 leading-relaxed">{q.question.clone()}</div>
      {q.image().map(|src| view! { <img src=src.to_owned() alt="题目附图" loading="lazy" class="mb-3 max-h-64 border" /> })}

      <div class="mb-3 border border-win/40 bg-win/5 p-3">
        <div class="mb-1.5 flex items-center gap-1.5 text-xs text-win-text">
          <Icon kind=IconKind::CheckCircle2 class="size-6 w-3.5" />
          {answer_title}
        </div>
        {answers
          .into_iter()
          .map(|a| {
            view! {
              <div class="flex items-start gap-2 py-0.5 text-sm">
                <span class="mt-0.5 flex size-5 shrink-0 items-center justify-center border-2 border-win bg-card text-xs text-win-text">
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
        .map_or_else(
          || {
            view! {
              <div class="mb-3 border-t border-dashed pt-3">
                <div class="mb-1 text-xs text-muted-foreground">{move || t("exam.explanation")}</div>
                <div class="text-sm leading-6 text-muted-foreground">{move || t("knowledge.no-explanation-yet-for")}</div>
              </div>
            }
            .into_any()
          },
          |e| {
            view! {
              <div class="mb-3 border-t border-dashed pt-3">
                <div class="mb-1 text-xs text-muted-foreground">{move || t("exam.explanation")}</div>
                <div class="whitespace-pre-line text-sm leading-6 text-muted-foreground">{e}</div>
              </div>
            }
            .into_any()
          },
        )}

      {note
        .map(|n| {
          view! {
            <div class="mb-3 border-t border-dashed pt-3">
              <div class="mb-1 text-xs text-muted-foreground">
                {move || t("knowledge.topic")} {sub.map(|s| s.name)}
              </div>
              <div class="text-sm leading-6 text-muted-foreground">{n}</div>
            </div>
          }
        })}

      {(!refs.is_empty())
        .then(|| {
          view! {
            <div class="border-t border-dashed pt-3">
              <div class="mb-1 text-xs text-muted-foreground">{move || t("exam.references")}</div>
              <ul class="space-y-1">
                {refs
                  .iter()
                  .map(|r| {
                    let badge = cn(&[
                      "mt-0.5 shrink-0 px-1.5 py-0.5 text-xs",
                      if r.kind == RefKind::Law {
                        "bg-xp/10 text-xp-text"
                      } else {
                        "bg-win/10 text-win-text"
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
                          <Icon kind=IconKind::ExternalLink class="size-6 w-3" />
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
