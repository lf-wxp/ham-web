//! 考试日程：排期在各省公告里，这一页只把「去哪里查」与「查到之后做什么」串起来。

use ham_web_core::exam_schedule::{CHANNELS, CONFIRM_ITEMS, NEXT_STEPS};
use leptos::prelude::*;

use crate::components::common::{ConceptsSection, PageContainer, PageHeader};
use crate::i18n::t;
use crate::icons::{Icon, IconKind};
use crate::ui::{ButtonLink, Size, Variant};
use crate::util::set_title;

#[component]
pub fn ExamSchedulePage() -> impl IntoView {
  set_title("learning.exam-schedule");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("learning.exam-schedule")
        subtitle=move || t("learning.exam-schedule-subtitle")
      />

      <PageContainer>
        // 免责与用法放在最前面：用户来这一页是找日期的，先说清楚本站没有日期，
        // 再给渠道，比先给一张「看起来很全」的渠道表更不容易误会。
        <p class="text-xs leading-relaxed text-muted-foreground">
          {move || t("learning.exam-schedule-disclaimer")}
        </p>

        <section class=crate::kit::card_class()>
          <h2 class=crate::kit::card_title_class()>{move || t("learning.exam-schedule-channels")}</h2>
          <div class="divide-y">
            {CHANNELS
              .iter()
              .map(|c| {
                view! {
                  <div class="grid gap-1 px-4 py-3">
                    <div class="flex flex-wrap items-baseline gap-x-2">
                      <span class="text-sm text-primary">{move || t(c.name)}</span>
                      <span class="text-xs text-muted-foreground">{move || t(c.what)}</span>
                    </div>
                    <p class="text-sm text-muted-foreground">{move || t(c.entry)}</p>
                    {c
                      .url
                      .map(|url| {
                        view! {
                          <a
                            href=url
                            target="_blank"
                            rel="noopener noreferrer"
                            class="inline-flex items-center gap-1 text-sm text-primary underline-offset-2 hover:underline"
                          >
                            <Icon kind=IconKind::ExternalLink class="size-6 shrink-0" />
                            {move || t("learning.exam-schedule-open-site")}
                          </a>
                        }
                      })}
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        // `ui = true`：这五项是界面文案（`learning` 域），走 `t()` 而不是知识库词典 `kt()`。
        <ConceptsSection
          title="learning.exam-schedule-checklist"
          items=CONFIRM_ITEMS
          ui=true
        />

        <section class=crate::kit::card_class()>
          <h2 class=crate::kit::card_title_class()>{move || t("learning.exam-schedule-next")}</h2>
          <div class="divide-y">
            {NEXT_STEPS
              .iter()
              .map(|s| {
                view! {
                  <div class="flex flex-col gap-2 px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
                    <div class="min-w-0">
                      <div class="text-sm">{move || t(s.title)}</div>
                      <div class="mt-0.5 text-xs text-muted-foreground">{move || t(s.desc)}</div>
                    </div>
                    <ButtonLink
                      href=s.href
                      variant=Variant::Outline
                      size=Size::Sm
                      class="shrink-0 self-start sm:self-auto"
                    >
                      {move || t(s.action)}
                    </ButtonLink>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>
      </PageContainer>
    </div>
  }
}
