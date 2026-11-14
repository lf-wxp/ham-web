//! 桌面顶栏下沿的换台刻度尺（`docs/ui/pixel.md` §3.4、`docs/ui/motion.md` §8「换台」）。

use ham_web_core::registry::{self, ModuleKind};
use leptos::prelude::*;
use leptos_router::hooks::use_location;

use crate::components::radio::TuningScale;
use crate::i18n::t;

/// 三个分组在刻度上的位置（%）：刻度是面板上印死的，不跟着上面按键的实际位置走。
const MARKS: [(ModuleKind, u8); 3] = [
  (ModuleKind::Exam, 18),
  (ModuleKind::Knowledge, 50),
  (ModuleKind::Tool, 82),
];
/// 不在任何分组里的页面（首页、回合战…）：指针停在左端「待命」。
const PARKED: u8 = 1;

/// 顶栏的换台刻度：三个分组的丝印标签，指针停在当前页面所在的分组上、换分组时逐格走过去。
/// 所在分组在导航地标与页头铭牌里都有文字。只在桌面档（`xl`）显示。
#[component]
pub(crate) fn BandScale() -> impl IntoView {
  let location = use_location();
  let pos = move || {
    let kind = registry::by_path(&location.pathname.get()).and_then(registry::Module::kind);
    kind
      .and_then(|k| MARKS.iter().find(|(m, _)| *m == k))
      .map_or(PARKED, |(_, p)| *p)
  };
  let marks = Signal::derive(move || {
    MARKS
      .iter()
      .map(|(kind, at)| (mark_label(*kind), *at))
      .collect::<Vec<_>>()
  });
  view! { <TuningScale marks=marks needle=Signal::derive(pos) class="hidden xl:block" /> }
}

/// 刻度上的丝印标签；字面量写在这里（`check-i18n` 只认调用点的直接实参）。
fn mark_label(kind: ModuleKind) -> String {
  match kind {
    ModuleKind::Exam => t("shell.band-exam"),
    ModuleKind::Knowledge => t("shell.band-knowledge"),
    ModuleKind::Tool => t("shell.band-tools"),
  }
}
