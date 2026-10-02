//! 考试页顶部：进度头、薄弱项标识、考试元信息（类别 / 题数 / 限时 / 剩余）与设置入口。

use ham_web_core::text::format_ms;
use ham_web_core::{Bank, ExamRule};
use leptos::prelude::*;

use crate::components::common::QuestionProgressHeader;
use crate::i18n::{bank_class, t, tf};
use crate::icons::{Icon, IconKind};
use crate::ui::{Size, Variant, button_class};

#[component]
pub(super) fn ExamHeader(
  #[prop(into)] percent: Signal<i64>,
  #[prop(into)] weak: Signal<bool>,
  #[prop(into)] version: Signal<Option<String>>,
  #[prop(into)] bank: Signal<Bank>,
  #[prop(into)] rule: Signal<ExamRule>,
  remaining: RwSignal<i64>,
  on_open_settings: Callback<()>,
) -> impl IntoView {
  let remaining_view = move || {
    let ms = remaining.get();
    let class = if ms <= 60_000 {
      "text-red-600 dark:text-red-400"
    } else {
      ""
    };
    view! {
      <span class=class aria-live="polite">
        {format_ms(ms)}
      </span>
    }
  };

  view! {
    <QuestionProgressHeader
      percent=percent
      right=move || {
        let href = move || {
          let base = crate::pages::bank_href("/exam", version.get().as_deref(), bank.get());
          if weak.get() { base } else { format!("{base}&mode=weak") }
        };
        view! {
          <a
            class=button_class(Variant::Outline, Size::Sm, "")
            href=href
            title=move || t("按分类正确率与错题加权抽题，不计入备考状态")
          >
            {move || if weak.get() { t("常规模考") } else { t("薄弱项组卷") }}
          </a>
          <button
            class=button_class(Variant::Outline, Size::Icon, "")
            aria-label=move || t("设置")
            title=move || t("设置")
            on:click=move |_| on_open_settings.run(())
          >
            <Icon kind=IconKind::Settings class="h-4 w-4" />
          </button>
        }
      }
      meta=ViewFn::from(move || {
        view! {
          {move || weak.get().then(|| view! { <span class="mr-1 rounded bg-amber-500/15 px-1.5 py-0.5 text-amber-800 dark:text-amber-300">{move || t("薄弱项组卷")}</span> })}
          {move || t("考试类别：")} {move || bank_class(bank.get().as_str())} "｜"
          {move || {
            let r = rule.get();
            tf("试题数：{}（单选 {}，多选 {}）", &[&r.total.to_string(), &r.singles.to_string(), &r.multiples.to_string()])
          }}
          "｜" {move || tf("限时：{} 分钟", &[&rule.get().minutes.to_string()])} "｜"
          {move || t("剩余时间：")} {remaining_view}
        }
      })
    />
    <div class="sm:hidden grid grid-cols-2 gap-x-3 gap-y-1 text-xs text-muted-foreground">
      <div>{move || t("考试类别：")} {move || bank_class(bank.get().as_str())}</div>
      <div>
        {move || {
          let r = rule.get();
          tf("试题数：{}（单选 {}，多选 {}）", &[&r.total.to_string(), &r.singles.to_string(), &r.multiples.to_string()])
        }}
      </div>
      <div>{move || tf("限时：{} 分钟", &[&rule.get().minutes.to_string()])}</div>
      <div>{move || t("剩余：")} {remaining_view}</div>
    </div>
  }
}
