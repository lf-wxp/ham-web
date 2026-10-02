use ham_web_core::operating::{
  CONTACT_STEPS, GROUNDING_TIPS, LOG_FIELDS, QSL_FIELDS, REPEATER_TIPS,
};
use leptos::prelude::*;

use crate::util::set_title;

use super::field_list::FieldList;
use super::tip_list::TipList;
use crate::i18n::t;

#[component]
pub fn OperatingPage() -> impl IntoView {
  set_title(&t("通联实务"));
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("通联实务")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("通联流程 · 日志 · QSL · 中继台 · 接地防雷")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("标准通联流程")}</h2>
          <FieldList fields=CONTACT_STEPS />
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("通联日志字段")}</h2>
          <FieldList fields=LOG_FIELDS />
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("QSL 卡片信息")}</h2>
          <FieldList fields=QSL_FIELDS />
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("中继台使用要点")}</h2>
          <TipList tips=REPEATER_TIPS />
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("接地与防雷")}</h2>
          <TipList tips=GROUNDING_TIPS />
        </section>
      </div>
    </div>
  }
}
