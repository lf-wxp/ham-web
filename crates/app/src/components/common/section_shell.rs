//! 知识库 / 文档内容区块的统一外壳。
//!
//! `BulletSection` / `ConceptsSection` / `StepsSection` / `TableSection` 共用同一层
//! `<section>` + `<h2>`：标题档位、卡片类、标题间距必须与 CSS 对齐，写在四处就会在某次改档位时
//! 漏掉一处。外壳收在这里，四个内容形态组件只负责正文。
//!
//! 标题档位是现代风的 `md-panel-title`（24px），比 `SectionCard` 的 `md-section-title`
//! （20px）大一档 —— 这是两个族**唯一**不能合并的地方，所以外壳独立成一份，不塞进 `SectionCard`。

use leptos::prelude::*;

use crate::data;
use crate::i18n::t;

/// 知识库内容区块的外壳：标题 + 一块面板，正文由 `children` 提供。
///
/// 现代风 `md-panel` + `md-panel-title`；像素风 `kit::card_class()` + `kit::card_title_class()`。
#[component]
pub(super) fn SectionShell(
  /// 词条 key（未翻译），由外壳统一查 `t`。
  #[prop(into)]
  title: String,
  children: Children,
) -> impl IntoView {
  if crate::kit::is_modern_at_mount() {
    return view! {
      <section class="md-panel">
        <h2 class="md-panel-title">{move || t(&title)}</h2>
        {children()}
      </section>
    }
    .into_any();
  }
  view! {
    <section class=crate::kit::card_class()>
      <h2 class=crate::kit::card_title_class()>{move || t(&title)}</h2>
      {children()}
    </section>
  }
  .into_any()
}

/// 按文案来源选词典：`ui` 为真走界面词典 [`t`]，否则走知识库词典 [`data::kt`]。
///
/// `ui` 必须由调用点声明：知识库正文来自 `crates/core`，界面文案写在 `crates/app`，
/// 两者混用会让译文永远命中不到。
pub(super) fn section_text(text: &str, ui: bool) -> String {
  if ui {
    return t(text);
  }
  data::track_knowledge();
  data::kt(text)
}
