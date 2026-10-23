//! 考试页顶部：进度头、薄弱项标识、考试元信息（类别 / 题数 / 限时 / 剩余）与设置入口。

use ham_web_core::text::format_ms;
use ham_web_core::{Bank, ExamRule};
use leptos::prelude::*;

use crate::components::common::QuestionProgressHeader;
use crate::i18n::{bank_class, t, tf};
use crate::icons::{Icon, IconKind};
use crate::ui::{Button, ButtonLink, Size, Variant};

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
  // 倒计时是考试页最该被读到的数字：走 HUD 字体（`pxl-label`）+ 正文档字号，
  // 而不是混在 12px 的灰色元信息里（像素规范：数字读数用 `pxl-title` / `pxl-label`）。
  let remaining_view = move || {
    let ms = remaining.get();
    let class = if ms <= 60_000 {
      "pxl-label text-base text-red-600 dark:text-red-400"
    } else {
      "pxl-label text-base text-foreground"
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
          <ButtonLink
            href=Signal::derive(href)
            variant=Variant::Outline
            size=Size::Sm
            title=Signal::derive(move || t("exam.weighted-by-topic-accuracy"))
          >
            {move || if weak.get() { t("exam.standard-exam") } else { t("exam.weak-area-exam") }}
          </ButtonLink>
          <Button
            variant=Variant::Outline
            size=Size::Icon
            aria_label=Signal::derive(move || t("exam.settings"))
            title=Signal::derive(move || t("exam.settings"))
            on_click=Callback::new(move |_| on_open_settings.run(()))
          >
            <Icon kind=IconKind::Settings class="h-4 w-4" />
          </Button>
        }
      }
      meta=ViewFn::from(move || {
        view! {
          {move || weak.get().then(|| view! { <span class="mr-1 bg-amber-500/15 px-1.5 py-0.5 text-amber-800 dark:text-amber-300">{move || t("exam.weak-area-exam")}</span> })}
          {move || t("exam.class-prefix")} {move || bank_class(bank.get().as_str())} "｜"
          {move || {
            let r = rule.get();
            tf("exam.questions-single-multiple", &[&r.total.to_string(), &r.singles.to_string(), &r.multiples.to_string()])
          }}
          "｜" {move || tf("exam.time-limit-min", &[&rule.get().minutes.to_string()])} "｜"
          {move || t("exam.remaining")} {remaining_view}
        }
      })
    />
    // 窄屏只留三项元信息：剩余时间已经是上方的 HUD 读数（`BossStage`），
    // 这里再写一遍既重复又会把「试题数（单选 32，多选 8）」挤成折行。
    // `flex-wrap` 而不是两列网格：三项在 390px 下正好一行，320px 下自然折到第二行。
    <div class="flex flex-wrap items-baseline gap-x-4 gap-y-1 text-xs text-muted-foreground sm:hidden">
      <span>{move || t("exam.class-prefix")} {move || bank_class(bank.get().as_str())}</span>
      <span>
        {move || tf("exam.questions-count", &[&rule.get().total.to_string()])}
      </span>
      <span>{move || tf("exam.time-limit-min", &[&rule.get().minutes.to_string()])}</span>
    </div>
  }
}
