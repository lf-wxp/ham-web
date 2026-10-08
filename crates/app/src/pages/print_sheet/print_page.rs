use ham_web_core::Bank;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_query_map;

use crate::components::common::Loading;
use crate::ui::{Button, Checkbox, ControlSize, NativeSelect, SelectOption, Size, Variant};
use crate::util::{local_today, set_title_with_args};

use super::question_block::QuestionBlock;
use super::{AnswerMode, Item, Source, explanation, load_items};
use crate::i18n::{t, tf, tp};

/// 错题 / 收藏打印页。
#[component]
pub fn PrintPage() -> impl IntoView {
  let query = use_query_map();
  let source = RwSignal::new(
    if query.with_untracked(|q| q.get("src").as_deref() == Some("bookmarks")) {
      Source::Bookmarks
    } else {
      Source::Mistakes
    },
  );
  let bank = RwSignal::new(
    query
      .with_untracked(|q| q.get("bank"))
      .and_then(|b| b.parse::<Bank>().ok()),
  );
  let answers = RwSignal::new(AnswerMode::End);
  let explain = RwSignal::new(true);
  let items = RwSignal::new(None::<Vec<Item>>);

  Effect::new(move |_| {
    // 传 key + 实参（不传拼好的译文），切语言时标题才会跟着重算。
    set_title_with_args("radio.print", &[source.get().title()]);
    let (s, b) = (source.get(), bank.get());
    items.set(None);
    spawn_local(async move { items.set(Some(load_items(s, b).await)) });
  });

  let print = move |_| {
    let _ = window().print();
  };
  let seg = |active: bool| {
    if active {
      "rounded-md bg-background px-2.5 py-1 text-xs font-medium shadow-sm"
    } else {
      "rounded-md px-2.5 py-1 text-xs text-muted-foreground hover:text-foreground"
    }
  };

  view! {
    <div class="min-h-screen bg-muted/40 pb-10 print:bg-white print:pb-0">
      <div class="print-hide sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-[210mm] flex-wrap items-center gap-x-4 gap-y-2 px-4 py-3">
          <h1 class="mr-auto text-base font-semibold">{move || t("radio.print-version")}</h1>
          <div class="inline-flex rounded-lg bg-muted p-0.5" role="group" aria-label=move || t("radio.source")>
            {[Source::Mistakes, Source::Bookmarks].into_iter().map(|s| view! {
              <button type="button" class=move || seg(source.get() == s) aria-pressed=move || (source.get() == s).to_string() on:click=move |_| source.set(s)>
                {move || t(s.title())}
              </button>
            }).collect_view()}
          </div>
          <div class="inline-flex rounded-lg bg-muted p-0.5" role="group" aria-label=move || t("exam.bank")>
            {[None, Some(Bank::A), Some(Bank::B), Some(Bank::C)].into_iter().map(|b| view! {
              <button type="button" class=move || seg(bank.get() == b) aria-pressed=move || (bank.get() == b).to_string() on:click=move |_| bank.set(b)>
                {b.map_or(t("exam.all"), |b| tf("learning.class", &[&(b).to_string()]))}
              </button>
            }).collect_view()}
          </div>
          <label class="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
            {move || t("radio.answer")}
            <NativeSelect
              value=Signal::derive(move || {
                match answers.get() {
                  AnswerMode::Hidden => "hidden",
                  AnswerMode::Inline => "inline",
                  AnswerMode::End => "end",
                }
                .to_owned()
              })
              on_change=Callback::new(move |v: String| {
                answers.set(match v.as_str() {
                  "hidden" => AnswerMode::Hidden,
                  "inline" => AnswerMode::Inline,
                  _ => AnswerMode::End,
                })
              })
              options=vec![
                SelectOption::new("end", Signal::derive(move || t("radio.collected-at-the-end"))),
                SelectOption::new("inline", Signal::derive(move || t("radio.show-with-each-question"))),
                SelectOption::new("hidden", Signal::derive(move || t("radio.don-t-show"))),
              ]
              size=ControlSize::Sm
              aria_label=Signal::derive(move || t("radio.answer"))
              class="w-auto"
            />
          </label>
          <label class="inline-flex cursor-pointer items-center gap-1.5 text-xs text-muted-foreground">
            <Checkbox
              checked=explain
              on_change=Callback::new(move |v: bool| explain.set(v))
              disabled=Signal::derive(move || answers.get() == AnswerMode::Hidden)
            />
            {move || t("radio.with-explanations")}
          </label>
          <Button
            variant=Variant::Default
            size=Size::Sm
            on_click=Callback::new(move |_| print(()))
          >{move || t("radio.print-save-as-pdf")}</Button>
        </div>
      </div>

      <div class="print-sheet mx-auto mt-6 max-w-[210mm] bg-white px-[14mm] py-[12mm] text-zinc-900 shadow-sm print:mt-0 print:max-w-none print:p-0 print:shadow-none">
        {move || match items.get() {
          None => view! { <Loading label=t("exam.loading-questions") class="py-16" /> }.into_any(),
          Some(list) if list.is_empty() => view! {
            <p class="py-16 text-center text-sm text-zinc-500">
              {tf("radio.is-empty-practise-first", &[(source.get().title())])}
            </p>
          }.into_any(),
          Some(list) => {
            let mode = answers.get();
            let with_explain = explain.get() && mode != AnswerMode::Hidden;
            let bank_label = bank.get().map_or(t("radio.all-banks"), |b| tf("radio.class-bank", &[&(b).to_string()]));
            let key_list = list.clone();
            view! {
              <header class="mb-2 flex items-end justify-between border-b-2 border-zinc-900 pb-2">
                <div>
                  <div class="text-lg font-bold">{tf("radio.amateur-radio-licence", &[(source.get().title())])}</div>
                  <div class="text-[11px] text-zinc-500">
                    {tp(
                      "radio.questions",
                      list.len() as u32,
                      &[&(bank_label).to_string(), &(list.len()).to_string(), &(local_today()).to_string()],
                    )}
                  </div>
                </div>
                <div class="text-[11px] text-zinc-500">{move || t("radio.name-score")}</div>
              </header>
              {list.into_iter().enumerate().map(|(i, item)| view! {
                <QuestionBlock index=i + 1 item=item answers=mode explain=with_explain />
              }).collect_view()}
              {(mode == AnswerMode::End).then(|| view! {
                <section class="print-break-before mt-6 border-t-2 border-zinc-900 pt-3">
                  <h2 class="mb-2 text-sm font-bold">{move || t("radio.reference-answer")}</h2>
                  <ol class="grid grid-cols-5 gap-x-4 gap-y-1 text-[12px] tabular-nums print:grid-cols-6">
                    {key_list.iter().enumerate().map(|(i, it)| view! {
                      <li>{format!("{}. {}", i + 1, it.q.answer_keys.join(""))}</li>
                    }).collect_view()}
                  </ol>
                  {with_explain.then(|| view! {
                    <div class="mt-4 space-y-1.5 text-[12px] leading-relaxed text-zinc-700">
                      {key_list.iter().enumerate().filter_map(|(i, it)| explanation(&it.q).map(|t| view! {
                        <p class="print-avoid-break"><span class="font-semibold text-zinc-900">{format!("{}. ", i + 1)}</span>{t}</p>
                      })).collect_view()}
                    </div>
                  })}
                </section>
              })}
            }.into_any()
          }
        }}
      </div>
    </div>
  }
}
