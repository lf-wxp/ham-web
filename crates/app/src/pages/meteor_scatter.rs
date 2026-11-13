//! 流星散射速查。

use ham_web_core::meteor_scatter::{METEOR_SHOWERS, MS_CONCEPTS, MS_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn MeteorScatterPage() -> impl IntoView {
  set_title("knowledge.meteor-scatter");
  view! {
    <KnowledgePage title=move || t("knowledge.meteor-scatter") subtitle=move || t("knowledge.the-main-weak-signal")>
      <ConceptsSection title="核心概念" items=MS_CONCEPTS />
      <TableSection title="主要流星雨" headers=&["流星雨", "峰值时段", "说明"] rows=METEOR_SHOWERS class="min-w-[640px]" />
      <BulletSection title="操作要点" items=MS_TIPS />
      <section class=crate::kit::card_class()>
        <h2 class=crate::kit::card_title_class()>
          {move || t("knowledge.next-meteor-shower-peaks")}
        </h2>
        <p class="px-4 py-3 text-sm text-muted-foreground">
          {move || t("knowledge.the-shower-timetable-on")}
        </p>
        <div class="px-4 pb-4">
          <a href="/events" class=format!("{} {}", crate::kit::link_class(), "text-sm")>
            {move || t("knowledge.open-the-event-calendar")}
          </a>
        </div>
      </section>
    </KnowledgePage>
  }
}
