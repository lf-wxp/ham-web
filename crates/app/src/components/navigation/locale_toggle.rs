//! 界面语言切换（中文 / English / Español）。

use leptos::prelude::*;

use crate::i18n::{self, Locale};
use crate::icons::{Icon, IconKind};
use crate::ui::{ControlSize, Select, SelectItem};

/// 语言切换控件：地球图标 + 下拉菜单，常驻导航栏右侧。
#[component]
pub fn LocaleToggle() -> impl IntoView {
  let locale = i18n::locale();
  let label = Signal::derive(move || i18n::t("shell.switch-language"));

  let shown = locale.clone();
  let value = Signal::derive(move || Some(shown.get().code().to_owned()));
  // 语言名本身不翻译（「English」在中文界面下也写 English）。
  // `trigger` 每次都在响应式上下文里重跑，这里读到的就是最新语言。
  let current = locale.clone();
  let trigger = move || view! { {current.get().label()} }.into_any();
  let items = move || {
    Locale::ALL
      .iter()
      .map(|&l| view! { <SelectItem value=l.code()>{l.label()}</SelectItem> })
      .collect_view()
  };

  view! {
    <div class="inline-flex items-center gap-1">
      <Icon kind=IconKind::Globe class="h-4 w-4 shrink-0 text-muted-foreground" />
      <Select
        value=value
        on_change=Callback::new(move |v: String| i18n::set_locale(Locale::from_code(&v)))
        placeholder=label
        trigger=trigger
        size=ControlSize::Sm
        aria_label=label
        class="w-auto max-w-[8rem]"
      >
        {items}
      </Select>
    </div>
  }
}
