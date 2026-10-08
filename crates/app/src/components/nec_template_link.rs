//! 方案库表格里的「模型」列：把 `.nec` 模板交接给 `/nec`。
//!
//! 交接走 localStorage（见 [`crate::util::stash_nec_template`]）：点链接时先写入模板
//! id，再跳到 `/nec`，那边挂载时取走并清除。用普通 `<a href>` 而不是路由跳转，
//! 就算整页刷新也不影响（localStorage 不丢）。

use ham_web_core::nec_templates::nec_template;
use leptos::prelude::*;

use crate::i18n::t;
use crate::util::stash_nec_template;

/// 一个方案的模型列：有模板时给跳转链接，没有时给「暂无可用模型」的说明。
#[component]
pub fn NecTemplateLink(template: &'static str) -> impl IntoView {
  let Some(found) = nec_template(template) else {
    return view! {
      <span class="text-xs text-muted-foreground" title=move || t("tools.nec-no-model")>
        "—"
      </span>
    }
    .into_any();
  };
  let id = found.id;
  let name = found.name;
  view! {
    <a
      href="/nec"
      class="inline-flex items-center gap-1 whitespace-nowrap rounded-md border bg-background px-2 py-0.5 text-xs font-medium hover:bg-accent"
      title=name
      on:click=move |_| stash_nec_template(id)
    >
      {move || t("tools.nec-open-in-solver")}
    </a>
  }
  .into_any()
}
