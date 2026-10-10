use leptos::prelude::*;

use crate::i18n::t;
use crate::icons::{Icon, IconKind};
use crate::theme::{Theme, use_theme};
use crate::ui::{Button, Size, Variant};

/// 明暗主题切换按钮，可通过 `class` 追加定位等样式。
#[component]
pub fn ThemeToggle(#[prop(optional)] class: &'static str) -> impl IntoView {
  let theme = use_theme();
  let toggle_label = move || {
    if theme.is_dark() {
      t("shell.switch-to-light-mode")
    } else {
      t("shell.switch-to-dark-mode")
    }
  };
  view! {
    <Button
      variant=Variant::Ghost
      size=Size::Icon
      class=class
      aria_label=Signal::derive(move || toggle_label().to_owned())
      title=Signal::derive(move || toggle_label().to_owned())
      on_click=Callback::new(move |_| theme.set(if theme.is_dark() { Theme::Light } else { Theme::Dark }))
    >
      {move || {
        if theme.is_dark() {
          view! { <Icon kind=IconKind::Sun class="h-4 w-4" /> }
        } else {
          view! { <Icon kind=IconKind::Moon class="h-4 w-4" /> }
        }
      }}
    </Button>
  }
}
