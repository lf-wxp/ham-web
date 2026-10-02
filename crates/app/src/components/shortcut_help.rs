//! 快捷键帮助面板：按 `?` 或从设置里唤起，键位来自 [`crate::shortcuts::SHORTCUT_HELP`]。

use leptos::prelude::*;

use crate::components::exam::ShortcutRow;
use crate::i18n::t;
use crate::shortcuts::SHORTCUT_HELP;
use crate::ui::Dialog;

/// 快捷键帮助对话框。
///
/// 过去的键位说明只在「首次进入」时自动弹一次，之后再也找不回来；这里做成随时可查，
/// 列表直接渲染 `SHORTCUT_HELP`，新增键位只需改那一处。
#[component]
pub fn ShortcutHelpDialog(open: RwSignal<bool>) -> impl IntoView {
  view! {
    <Dialog open=open class="sm:max-w-sm" label=t("快捷键")>
      <h2 class="text-base font-semibold">{move || t("快捷键")}</h2>
      <p class="text-xs text-muted-foreground">{move || t("答题时按 ? 可随时打开本帮助。")}</p>
      <div class="space-y-2 text-sm">
        {SHORTCUT_HELP
          .iter()
          .map(|(keys, desc)| {
            view! { <ShortcutRow label=Signal::derive(move || t(desc)) keys=(*keys).to_owned() /> }
          })
          .collect_view()}
      </div>
    </Dialog>
  }
}
