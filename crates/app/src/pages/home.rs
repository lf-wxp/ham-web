//! 首页：选择题库版本与类别，进入练习 / 模拟考试。

use ham_web_core::Bank;
use leptos::prelude::*;
use leptos::task::spawn_local;

use super::{DEFAULT_TITLE, bank_href};
use crate::components::bank_selector::QuestionBankSelector;
use crate::components::bubble::Bubble;
use crate::data;
use crate::ui::{
  CARD_HEADER, Size, Variant, button_class, card_class, card_content_class, card_title_class,
};
use crate::util::set_title;

#[component]
pub fn HomePage() -> impl IntoView {
  set_title(DEFAULT_TITLE);
  let version = RwSignal::new(None::<String>);
  let bank = RwSignal::new(Bank::A);
  let checking = RwSignal::new(false);
  let available = RwSignal::new(false);
  let warn_open = RwSignal::new(false);
  let warn_text = RwSignal::new(String::new());
  let generation = StoredValue::new(0u32);

  Effect::new(move |_| {
    let (Some(v), b) = (version.get(), bank.get()) else {
      return;
    };
    generation.update_value(|g| *g += 1);
    let current = generation.get_value();
    checking.set(true);
    spawn_local(async move {
      let ok = data::bank_available(Some(&v), b).await;
      if generation.try_get_value() == Some(current) {
        available.set(ok);
        checking.set(false);
      }
    });
  });

  let guard = move |e: web_sys::MouseEvent| {
    if !available.get_untracked() || version.with_untracked(Option::is_none) {
      e.prevent_default();
      warn_text.set(format!(
        "题库 {} 暂不可用或为空，请先构建数据集",
        bank.get_untracked()
      ));
      warn_open.set(true);
    }
  };
  let practice_href = move || bank_href("/practice", version.get().as_deref(), bank.get());
  let exam_href = move || bank_href("/exam", version.get().as_deref(), bank.get());

  view! {
    <main class="container mx-auto px-4 py-10 max-w-3xl animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <div data-slot="card" class=card_class("")>
        <div data-slot="card-header" class=CARD_HEADER>
          <div data-slot="card-title" class=card_title_class("")>
            "业余无线电执照考试模拟练习"
          </div>
        </div>
        <div data-slot="card-content" class=card_content_class("space-y-4")>
          <QuestionBankSelector selected_version=version selected_bank=bank disabled=checking />
          <div class="flex flex-wrap gap-3 relative">
            <a
              href=practice_href
              data-slot="button"
              class=button_class(Variant::Default, Size::Default, "")
              on:click=guard
            >
              "开始练习"
            </a>
            <a href=exam_href data-slot="button" class=button_class(Variant::Secondary, Size::Default, "") on:click=guard>
              "开始模拟考试"
            </a>
            <Bubble open=warn_open text=warn_text />
          </div>
        </div>
      </div>
    </main>
  }
}
