//! 界面语言切换（中文 / English / Español）。

use leptos::prelude::*;

use crate::i18n::{self, Locale};
use crate::icons::{Icon, IconKind};

/// 语言切换控件：地球图标 + 下拉框，常驻导航栏右侧。
#[component]
pub fn LocaleToggle() -> impl IntoView {
  let locale = i18n::locale();
  view! {
    <div class="inline-flex items-center gap-1" title=move || i18n::t("切换语言")>
      <Icon kind=IconKind::Globe class="h-4 w-4 shrink-0 text-muted-foreground" />
      <select
        class="h-8 max-w-[8rem] rounded-md border bg-background px-1.5 text-xs text-muted-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
        aria-label=move || i18n::t("切换语言")
        prop:value=move || locale.get().code()
        on:change=move |e| i18n::set_locale(Locale::from_code(&event_target_value(&e)))
      >
        {Locale::ALL
          .iter()
          .map(|&l| view! { <option value=l.code()>{l.label()}</option> })
          .collect_view()}
      </select>
    </div>
  }
}
