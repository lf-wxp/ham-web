//! 现代风首页的题库版本入口。
//!
//! 与像素版共用同一个 `QuestionBankSelector`：控件、键盘契约与无障碍名只有一份实现，
//! 这里只负责把它放进现代风的卡片外壳。选择结果由 `ModernHome` 提升到 hero 与快速入口的链接。

use ham_web_core::Bank;
use leptos::prelude::*;

use crate::components::bank_selector::QuestionBankSelector;
use crate::components::common::SectionCard;
use crate::i18n::t;

#[component]
pub fn ModernBankPicker(
  version: RwSignal<Option<String>>,
  bank: RwSignal<Bank>,
  #[prop(into)] disabled: Signal<bool>,
) -> impl IntoView {
  view! {
    // 外层必须是普通 `<section>`：`.md-panel` 一旦直接挂在 `.md-page` 下，会同时命中章节编号与
    // `.md-panel::before` 两条伪元素规则 —— 编号被 18px 角标盒子挤成两行，章节尺也会画到卡片顶边。
    // 包一层普通 section 后，编号归章节壳、三角归卡片，两边各管各的。
    <section>
      <SectionCard title=Signal::derive(move || t("home.quick-start"))>
        <div class="p-4">
          <QuestionBankSelector
            selected_version=version
            selected_bank=bank
            disabled=disabled
          />
        </div>
      </SectionCard>
    </section>
  }
}
