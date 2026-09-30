use leptos::prelude::*;

use crate::cn::cn;
use crate::icons::{Icon, IconKind};

/// 加载状态：spinner + 可选文案。默认大间距居中，可用 `class` 覆盖间距。
#[component]
pub fn Loading(
  #[prop(optional, into)] label: Option<String>,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  view! {
    <div class=cn(&[
      "flex items-center justify-center gap-2 py-20 text-sm text-muted-foreground",
      &class,
    ])>
      <Icon kind=IconKind::Loader2 class="h-5 w-5 animate-spin" />
      {label.map(|l| view! { <span>{l}</span> })}
    </div>
  }
}
