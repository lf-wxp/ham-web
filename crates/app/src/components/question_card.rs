//! 题目卡片：题干、附图、选项（单选/多选），可选即时显示答案。

use ham_exam_core::QuestionItem;
use leptos::prelude::*;

use crate::components::common::PreviewableImage;
use crate::ui::{
  BadgeVariant, CARD_HEADER, Checkbox, Label, RadioGroup, RadioGroupItem, badge_class, card_class,
  card_content_class, card_title_class,
};
use crate::util::unique_id;

#[component]
pub fn QuestionCard(
  index: usize,
  total: usize,
  question: QuestionItem,
  #[prop(into)] selected: Signal<Vec<String>>,
  on_change: Callback<Vec<String>>,
  #[prop(into)] show_answer: Signal<bool>,
  #[prop(optional, into)] read_only: Signal<bool>,
) -> impl IntoView {
  let base_id = unique_id("q");
  let is_multiple = question.is_multiple();
  let answer_keys = question.answer_keys.clone();
  let j_code = question.j_code().map(ToOwned::to_owned);
  let pages = question.pages.and_then(|p| {
    let start = p.start?;
    Some(match p.end {
      Some(end) if end != start => format!("P.{start}-{end}"),
      _ => format!("P.{start}"),
    })
  });

  let image = question.image().map(|src| {
    let (alt, title) = match &j_code {
      Some(j) => (format!("题号 {j} 附图"), format!("题号 {j} 题图")),
      None => ("题目附图".to_owned(), "题目附图".to_owned()),
    };
    view! {
      <div class="mt-2">
        <PreviewableImage src=src.to_owned() alt=alt title=title />
      </div>
    }
  });

  let options = if is_multiple {
    let items = question
      .options
      .clone()
      .into_iter()
      .map(|opt| {
        let key = StoredValue::new(opt.key.clone());
        let checked = Signal::derive(move || selected.with(|s| key.with_value(|k| s.contains(k))));
        let toggle = Callback::new(move |on: bool| {
          if read_only.get_untracked() {
            return;
          }
          let k = key.get_value();
          let mut next = selected.get_untracked();
          next.retain(|x| *x != k);
          if on {
            next.push(k);
          }
          on_change.run(next);
        });
        view! {
          <label class="flex items-start gap-2">
            <Checkbox class="mt-1" checked=checked on_change=toggle disabled=read_only />
            <span class="whitespace-pre-line leading-6">
              <strong class="mr-2">{opt.key} "."</strong>
              {opt.text}
            </span>
          </label>
        }
      })
      .collect_view();
    view! { <div class="grid gap-2">{items}</div> }.into_any()
  } else {
    let options = question.options.clone();
    let base_id = base_id.clone();
    // 选项必须在 RadioGroup 内部（其 context 提供之后）再构建
    let items = move || {
      options
        .clone()
        .into_iter()
        .map(|opt| {
          let id = format!("{base_id}-{index}-{}", opt.key);
          let label_class = Signal::derive(move || {
            if read_only.get() {
              "whitespace-pre-line leading-6 cursor-default"
            } else {
              "whitespace-pre-line leading-6 cursor-pointer"
            }
            .to_owned()
          });
          view! {
            <div class="flex items-start gap-2">
              <RadioGroupItem class="mt-1" value=opt.key.clone() id=id.clone() disabled=read_only />
              <Label r#for=id class=label_class>
                <strong class="mr-2">{opt.key} "."</strong>
                {opt.text}
              </Label>
            </div>
          }
        })
        .collect_view()
    };
    let value = Signal::derive(move || selected.with(|s| s.first().cloned().unwrap_or_default()));
    let on_value = Callback::new(move |v: String| {
      if !read_only.get_untracked() {
        on_change.run(if v.is_empty() { vec![] } else { vec![v] });
      }
    });
    view! {
      <RadioGroup value=value on_change=on_value disabled=read_only>
        {items()}
      </RadioGroup>
    }
    .into_any()
  };

  let answer_line = move || {
    show_answer.get().then(|| {
      let sel = selected.get();
      let correct = !sel.is_empty() && ham_exam_core::question::same_set(&sel, &answer_keys);
      let verdict = (!sel.is_empty()).then(|| {
        let (class, text) =
          if correct { ("ml-2 text-green-600 dark:text-green-400", "已答对".to_owned()) } else { ("ml-2 text-red-600 dark:text-red-400", format!("作答：{}", sel.join(", "))) };
        view! { <span class=class>{text}</span> }
      });
      view! {
        <div class="text-sm text-muted-foreground">"正确答案：" {answer_keys.join(", ")} {verdict}</div>
      }
    })
  };

  view! {
    <div data-slot="card" class=card_class("")>
      <div data-slot="card-header" class=CARD_HEADER>
        <div data-slot="card-title" class=card_title_class("flex items-center gap-2")>
          <span>"第 " {index + 1} " / " {total} " 题"</span>
          <span data-slot="badge" class=badge_class(BadgeVariant::Secondary, "")>
            {question.kind.label()}
          </span>
          {pages.map(|p| view! { <span data-slot="badge" class=badge_class(BadgeVariant::Outline, "")>{p}</span> })}
          {j_code
            .clone()
            .map(|j| {
              view! {
                <span data-slot="badge" class=badge_class(BadgeVariant::Outline, "") title="题号">
                  {j}
                </span>
              }
            })}
        </div>
      </div>
      <div data-slot="card-content" class=card_content_class("space-y-3")>
        <div class="whitespace-pre-line">{question.question.clone()}</div>
        {image}
        <div class="space-y-2">{options}</div>
        {answer_line}
      </div>
    </div>
  }
}
