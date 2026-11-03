//! 现代风快捷键帮助：一列「键位 → 说明」，键帽用小圆角 kbd。
//! 键位表仍取自 `shortcuts::SHORTCUT_HELP`，与像素版同源。

use leptos::prelude::*;

use crate::i18n::t;
use crate::shortcuts::SHORTCUT_HELP;

use super::dialog::ModernDialog;

#[component]
pub fn ModernShortcutHelp(open: RwSignal<bool>) -> impl IntoView {
  view! {
    <ModernDialog open=open title=Signal::derive(move || t("exam.shortcuts"))>
      <p class="md-muted md-sm">{move || t("common.press-while-answering-to")}</p>
      <div class="md-kbd-list">
        {SHORTCUT_HELP
          .iter()
          .map(|(keys, desc)| {
            view! {
              <div class="md-kbd-row">
                <kbd class="md-kbd">{(*keys).to_owned()}</kbd>
                <span>{move || t(desc)}</span>
              </div>
            }
          })
          .collect_view()}
      </div>
    </ModernDialog>
  }
}
