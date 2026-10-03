use ham_web_core::QuestionItem;
use leptos::prelude::*;

use crate::cn::cn;
use crate::i18n::t;
use crate::icons::{Icon, IconKind};
use crate::speech;
use crate::ui::card_class;

/// 可折叠的「答案解析」卡片；题目切换时由调用方重建，自动恢复折叠状态。
/// 题目没有解析时显示轻提示，并引导到仓库补充。
#[component]
pub fn ExplanationCard(question: QuestionItem) -> impl IntoView {
  let open = RwSignal::new(false);
  let text = question
    .explanation
    .as_deref()
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .map(ToOwned::to_owned);

  let Some(text) = text else {
    return view! {
      <div data-slot="card" class=card_class("gap-0 py-0")>
        <div class="flex flex-wrap items-center gap-x-2 gap-y-1 px-6 py-4 text-sm text-muted-foreground">
          <Icon kind=IconKind::BookOpen class="h-4 w-4 shrink-0" />
          <span>{move || t("此题暂无解析")}</span>
          <a
            href="https://github.com/lf-wxp/ham-web"
            target="_blank"
            rel="noopener noreferrer"
            class="inline-flex items-center gap-1 font-medium text-primary underline-offset-2 hover:underline"
          >
            {move || t("欢迎补充")}
            <Icon kind=IconKind::ExternalLink class="h-3 w-3" />
          </a>
        </div>
      </div>
    }
    .into_any();
  };

  let speak_text = text.clone();
  // 折叠面板：外层 grid 行高 0fr → 1fr 做高度过渡（`.ui-collapse` 负责裁剪与收起后隐藏），
  // 内层再叠一层淡入 + 轻微上移，避免只有高度变化显得生硬。
  let panel = Signal::derive(move || {
    cn(&[
      "ui-collapse",
      if open.get() {
        "grid-rows-[1fr]"
      } else {
        "grid-rows-[0fr]"
      },
    ])
  });
  let inner = Signal::derive(move || {
    cn(&[
      // Tailwind v4 的 `translate-y-*` 用的是 `translate` 属性而不是 `transform`，
      // 过渡属性要写成 `translate` 才动得了。
      "px-6 pb-5 transition-[opacity,translate] duration-[var(--motion-normal)] ease-[var(--ease-out-expo)]",
      if open.get() {
        "translate-y-0 opacity-100"
      } else {
        "-translate-y-1 opacity-0"
      },
    ])
  });

  view! {
    <div data-slot="card" class=card_class("gap-0 py-0")>
      <div class="flex items-center">
        <button
          type="button"
          on:click=move |_| open.update(|v| *v = !*v)
          aria-expanded=move || open.get().to_string()
          class="flex flex-1 items-center justify-between gap-2 px-6 py-4 text-left cursor-pointer transition-colors hover:bg-accent/50 outline-none focus-visible:ring-2 focus-visible:ring-ring/50 rounded-xl"
        >
          <span class="text-base font-semibold leading-none">{move || t("答案解析")}</span>
          <Icon
            kind=IconKind::ChevronDown
            class=Signal::derive(move || {
              cn(&[
                "h-4 w-4 shrink-0 transition-transform duration-[var(--motion-normal)] ease-[var(--ease-out-expo)]",
                if open.get() { "rotate-180" } else { "" },
              ])
            })
          />
        </button>
        <button
          type="button"
          class="mr-4 flex size-8 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
          title=move || t("朗读解析")
          aria-label=move || t("朗读解析")
          on:click=move |_| speech::speak_zh(&speak_text)
        >
          <Icon kind=IconKind::Volume2 class="h-4 w-4" />
        </button>
      </div>
      <div class=panel data-open=move || open.get().to_string()>
        <div>
          <div class=inner>
            <div class="whitespace-pre-line border-t pt-4 text-sm leading-6 text-muted-foreground">
              {text.clone()}
            </div>
          </div>
        </div>
      </div>
    </div>
  }
  .into_any()
}
