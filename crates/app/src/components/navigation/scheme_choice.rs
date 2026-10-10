//! 设置对话框里的一个配色方案选项：单选框 + 四格色块预览 + 名称，点文字也能选中。

use ham_web_core::color_scheme::Scheme;
use leptos::prelude::*;

use crate::i18n::t;
use crate::ui::RadioGroupItem;

/// 方案名。key 必须以字面量写在调用点（`i18n-check` 靠它判断词条有没有被用到）。
fn scheme_label(id: &str) -> String {
  match id {
    "classic" => t("shell.scheme-classic"),
    "forest" => t("shell.scheme-forest"),
    "ocean" => t("shell.scheme-ocean"),
    "sunset" => t("shell.scheme-sunset"),
    "graphite" => t("shell.scheme-graphite"),
    // 新增方案忘了在这里登记名字：回退成 id，至少界面上看得出是哪个。
    other => other.to_owned(),
  }
}

/// 一个配色方案选项。`dark` 决定色块预览取亮色还是暗色那一套种子色。
#[component]
pub(super) fn SchemeChoice(scheme: &'static Scheme, dark: Signal<bool>) -> impl IntoView {
  let id = format!("settings-scheme-{}", scheme.id);
  let for_id = id.clone();
  // 色块：底色 / 主色 / 强调面 / 墨色。全部来自种子色，所以预览就是方案真实的样子。
  let swatch = move || {
    let s = scheme.seeds(dark.get());
    [s.bg, s.primary, s.accent, s.ink]
      .into_iter()
      .map(|c| view! { <span class="size-4" style=format!("background-color:{}", c.hex())></span> })
      .collect_view()
  };
  view! {
    <label for=for_id class="flex cursor-pointer items-center gap-2 text-sm">
      <RadioGroupItem value=scheme.id id=id />
      <span aria-hidden="true" class="flex shrink-0 border-2 border-ink">
        {swatch}
      </span>
      {move || scheme_label(scheme.id)}
    </label>
  }
}
