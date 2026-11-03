//! 界面语言切换（中文 / English / Español）。

use leptos::prelude::*;

use crate::i18n::{self, Locale};
use crate::icons::{Icon, IconKind};
use crate::ui::{ControlSize, SelectOption, SimpleSelect};

/// 语言切换控件：地球图标 + 下拉菜单，常驻导航栏右侧。
#[component]
pub fn LocaleToggle() -> impl IntoView {
  let locale = i18n::locale();
  let label = Signal::derive(move || i18n::t("shell.switch-language"));

  let value = Signal::derive(move || locale.get().code().to_owned());
  // 语言名本身不翻译（「English」在中文界面下也写 English），所以选项文案是静态的。
  let options: Vec<SelectOption> = Locale::ALL
    .iter()
    .map(|&l| SelectOption::new(l.code(), l.label()))
    .collect();

  view! {
    <div class="inline-flex min-w-0 items-center gap-1">
      <Icon kind=IconKind::Globe class="size-6 w-4 shrink-0 text-muted-foreground" />
      // 不传 `placeholder`：语言永远有值，不需要「清空」行。没有可见标签，名字与悬停提示都靠 `label`。
      <SimpleSelect
        value=value
        on_change=Callback::new(move |v: String| i18n::set_locale(Locale::from_code(&v)))
        options=options
        size=ControlSize::Sm
        aria_label=label
        title=label
        class="w-auto max-w-[8rem]"
      />
    </div>
  }
}
