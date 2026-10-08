//! 特殊传播方式速查。

use ham_web_core::special_prop::SPECIAL_MODES;
use leptos::prelude::*;

use crate::components::common::{KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn SpecialPropPage() -> impl IntoView {
  set_title("shell.special-propagation-2");
  view! {
    <KnowledgePage title=t("shell.special-propagation-2") subtitle=t("radio.eme-meteor-scatter-aurora")>
      <TableSection title="特殊传播方式" headers=&["方式", "原理", "特点"] rows=SPECIAL_MODES min_width=640 />
    </KnowledgePage>
  }
}
