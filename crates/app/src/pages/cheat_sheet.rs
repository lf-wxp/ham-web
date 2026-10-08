//! 考点速查手册：按 10 大分类整理高频考点，考前冲刺浏览，支持一键打印。

use ham_web_core::cheat_sheet::sections;
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::{t, tp};
use crate::ui::{Button, Size, Variant};
use crate::util::{set_title, window};

#[component]
pub fn CheatSheetPage() -> impl IntoView {
  set_title("exam.exam-points-handbook");
  let secs = sections();
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader title=t("exam.exam-points-handbook") subtitle=t("exam.high-frequency-exam-points-2") />
      <PageContainer>
        <div class="print-hide flex flex-wrap items-center justify-between gap-3">
          <p class="text-xs text-muted-foreground">{move || t("exam.high-frequency-exam-points")}</p>
          <Button
            variant=Variant::Default
            size=Size::Sm
            on_click=Callback::new(move |_| {
                        let _ = window().print();
                      })
          >
            {move || t("radio.print-save-as-pdf")}
          </Button>
        </div>

        {secs
          .into_iter()
          .map(|sec| {
            let color = sec.color;
            let count = sec.items.len();
            view! {
              <section class="print-avoid-break rounded-xl border bg-card print:border-zinc-300">
                <h2 class="flex items-center gap-2 border-b px-4 py-3 text-sm font-semibold">
                  <span class="h-2.5 w-2.5 shrink-0 rounded-full" style=format!("background: {color}")></span>
                  {sec.name}
                  <span class="ml-auto text-xs font-normal text-muted-foreground">
                    {move || tp("knowledge.points", count as u32, &[&count.to_string()])}
                  </span>
                </h2>
                <div class="grid gap-2 p-4 sm:grid-cols-2">
                  {sec
                    .items
                    .into_iter()
                    .map(|it| {
                      view! {
                        <div class="print-avoid-break rounded-lg border bg-muted/30 p-3">
                          <div class="flex items-baseline gap-2">
                            <span class="text-sm font-medium">{it.name}</span>
                            <span class="font-mono text-[10px] text-muted-foreground">{it.code}</span>
                          </div>
                          <p class="mt-1 text-xs leading-5 text-muted-foreground">{it.note}</p>
                        </div>
                      }
                    })
                    .collect_view()}
                </div>
              </section>
            }
          })
          .collect_view()}
      </PageContainer>
    </div>
  }
}
