//! 天线调试实操速查。

use ham_web_core::antenna_tuning::{TUNING_STEPS, TUNING_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, KnowledgePage, StepsSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn AntennaTuningPage() -> impl IntoView {
  set_title("shell.antenna-tuning");
  view! {
    <KnowledgePage title=t("shell.antenna-tuning") subtitle=t("knowledge.antenna-analysers-trimming-workflow")>
      <StepsSection title="调试步骤" items=TUNING_STEPS />
      <BulletSection title="调试要点" items=TUNING_TIPS />
    </KnowledgePage>
  }
}
