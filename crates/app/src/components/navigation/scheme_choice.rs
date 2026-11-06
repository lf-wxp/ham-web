//! 设置对话框里的一个配色方案选项：色块预览 + 名称。

use ham_web_core::color_scheme::Scheme;
use leptos::prelude::*;

use crate::i18n::t;

use super::choice_row::ChoiceRow;

/// 方案名。key 必须以字面量写在调用点（`i18n-check` 靠它判断词条有没有被用到）。
fn scheme_label(id: &str) -> String {
  match id {
    "classic" => t("shell.scheme-classic"),
    "forest" => t("shell.scheme-forest"),
    "ocean" => t("shell.scheme-ocean"),
    "sunset" => t("shell.scheme-sunset"),
    "graphite" => t("shell.scheme-graphite"),
    "pico" => t("shell.scheme-pico"),
    "gameboy" => t("shell.scheme-gameboy"),
    "nes" => t("shell.scheme-nes"),
    "commodore" => t("shell.scheme-commodore"),
    "cga" => t("shell.scheme-cga"),
    "spectrum" => t("shell.scheme-spectrum"),
    // 新增方案忘了在这里登记名字：回退成 id，至少界面上看得出是哪个。
    other => other.to_owned(),
  }
}

/// 一个配色方案选项。`dark` 决定色块预览取亮色还是暗色那一套种子色。
///
/// 单选行本身（id、`<label for>`、皮肤类名）在 [`ChoiceRow`]；这里只提供配色方案特有的
/// 「id → 词条」映射与色块预览。
#[component]
pub(super) fn SchemeChoice(scheme: &'static Scheme, dark: Signal<bool>) -> impl IntoView {
  // 色块：底色 / 主色 / 强调面 / 墨色。全部来自种子色，所以预览就是方案真实的样子。
  let swatch = move || {
    let s = scheme.seeds(dark.get());
    [s.bg, s.primary, s.accent, s.ink]
      .into_iter()
      .map(|c| view! { <span class="size-4" style=format!("background-color:{}", c.hex())></span> })
      .collect_view()
  };
  view! {
    <ChoiceRow
      id_prefix="settings-scheme"
      value=scheme.id
      label=Signal::derive(move || scheme_label(scheme.id))
    >
      <span aria-hidden="true" class="flex shrink-0 border-2 border-ink">
        {swatch}
      </span>
    </ChoiceRow>
  }
}
