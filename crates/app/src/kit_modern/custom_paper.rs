//! 现代风自定义组卷：三个数字输入 + 分类胶囊多选 + 取消 / 开始。
//! 组卷参数结构与校验与像素版一致（`ham_web_core::exam::CustomPaper`）。

use crate::ui::Field;
use ham_web_core::categories::TOP_CATEGORIES;
use ham_web_core::exam::{self, CustomPaper};
use leptos::prelude::*;

use crate::i18n::t;

use super::dialog::ModernDialog;

#[component]
pub fn ModernCustomPaper(open: RwSignal<bool>, on_confirm: Callback<CustomPaper>) -> impl IntoView {
  let singles = RwSignal::new(32u32);
  let multiples = RwSignal::new(8u32);
  let minutes = RwSignal::new(40u32);
  let cats: RwSignal<Vec<&'static str>> = RwSignal::new(Vec::new());

  let confirm = move || {
    open.set(false);
    on_confirm.run(CustomPaper {
      singles: singles.get() as usize,
      multiples: multiples.get() as usize,
      categories: cats.get(),
      minutes: minutes.get(),
    });
  };

  view! {
    <ModernDialog open=open title=Signal::derive(move || t("exam.custom-paper"))>
      <div class="md-input-grid">
        <Field
          label=Signal::derive(move || t("exam.single-choice-count"))
          r#for="custom-paper-singles"
        >
          <super::ModernInput
            value=Signal::derive(move || singles.get().to_string())
            on_input=Callback::new(move |v: String| singles.set(exam::clamp_count(&v, 300)))
            id="custom-paper-singles"
            kind="number"
            min="0"
            max="300"
            aria_label=Signal::derive(move || t("exam.single-choice-count"))
          />
        </Field>
        <Field
          label=Signal::derive(move || t("exam.multiple-choice-count"))
          r#for="custom-paper-multiples"
        >
          <super::ModernInput
            value=Signal::derive(move || multiples.get().to_string())
            on_input=Callback::new(move |v: String| multiples.set(exam::clamp_count(&v, 100)))
            id="custom-paper-multiples"
            kind="number"
            min="0"
            max="100"
            aria_label=Signal::derive(move || t("exam.multiple-choice-count"))
          />
        </Field>
        <Field
          label=Signal::derive(move || t("exam.time-limit-minutes-0"))
          r#for="custom-paper-minutes"
        >
          <super::ModernInput
            value=Signal::derive(move || minutes.get().to_string())
            on_input=Callback::new(move |v: String| minutes.set(exam::clamp_count(&v, 600)))
            id="custom-paper-minutes"
            kind="number"
            min="0"
            max="600"
            aria_label=Signal::derive(move || t("exam.time-limit-minutes-0"))
          />
        </Field>
      </div>

      <div class="flex flex-col gap-2">
        <span class="md-muted md-sm">{move || t("exam.limit-categories-empty-all")}</span>
        <div class="md-chip-row">
          {TOP_CATEGORIES
            .iter()
            .map(|c| {
              let key = c.key;
              let name = c.name;
              view! {
                <button
                  type="button"
                  class="md-chip"
                  aria-pressed=move || cats.get().contains(&key).to_string()
                  on:click=move |_| {
                    cats.update(|list| {
                      if let Some(pos) = list.iter().position(|&k| k == key) {
                        list.remove(pos);
                      } else {
                        list.push(key);
                      }
                    })
                  }
                >
                  {move || t(name)}
                </button>
              }
            })
            .collect_view()}
        </div>
      </div>

      <div class="md-dialog-actions">
        <button type="button" class="md-btn md-btn-ghost" on:click=move |_| open.set(false)>
          {move || t("exam.cancel")}
        </button>
        <button
          type="button"
          class="md-btn md-btn-accent"
          disabled=move || singles.get() + multiples.get() == 0
          on:click=move |_| confirm()
        >
          {move || t("exam.start-exam")}
        </button>
      </div>
    </ModernDialog>
  }
}
