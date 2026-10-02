//! 天线调试实操速查。

use ham_web_core::antenna_tuning::{TUNING_STEPS, TUNING_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, KnowledgePage, StepsSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn AntennaTuningPage() -> impl IntoView {
  set_title(&t("天线调试"));
  view! {
    <KnowledgePage title=t("天线调试") subtitle=t("天线分析仪 · 修剪流程")>
      <StepsSection title="调试步骤" items=TUNING_STEPS />
      <BulletSection title="调试要点" items=TUNING_TIPS />
    </KnowledgePage>
  }
}
