use leptos::prelude::*;

use crate::components::common::Loading;
use crate::data;
use crate::util::set_title;

use super::qcode_view::QCodeView;
use crate::i18n::t;

/// 简语速查页面：加载词条数据后交由 [`QCodeView`] 渲染。
#[component]
pub fn QCodePage() -> impl IntoView {
  set_title("shell.q-codes-2");
  let glossary = LocalResource::new(data::load_glossary);
  move || match glossary.get() {
    Some(g) => view! { <QCodeView entries=g.entries() /> }.into_any(),
    None => {
      view! { <Loading label=t("knowledge.loading-abbreviations") class="py-20" /> }.into_any()
    }
  }
}
