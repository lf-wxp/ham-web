use leptos::prelude::*;

use crate::components::common::Loading;
use crate::data;
use crate::util::set_title;

use super::glossary_view::GlossaryView;

/// 术语表页面：加载词条数据后交由 [`GlossaryView`] 渲染。
#[component]
pub fn GlossaryPage() -> impl IntoView {
  set_title("术语表");
  let glossary = LocalResource::new(data::load_glossary);
  move || match glossary.get() {
    Some(g) => view! { <GlossaryView entries=g.entries() /> }.into_any(),
    None => view! { <Loading label="加载术语表..." class="py-20" /> }.into_any(),
  }
}
