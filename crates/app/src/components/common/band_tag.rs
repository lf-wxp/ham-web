use ham_web_core::registry::{self, ModuleKind};
use leptos::prelude::*;
use leptos_router::hooks::use_location;

use crate::i18n::t;
use crate::ui::{Led, LedState, LedTone};

/// 像素风页头的「波段铭牌」：指示灯 + 当前页面所属的一级模块（考试中心 / 知识库 / 工具）。
///
/// 按路由在注册表里查；不在导航里的页面（回合战、打印页…）不挂。纯定位提示（`aria-hidden`）：
/// 读屏用户在导航地标里已经能拿到同样的信息，页头里再念一遍只是噪声。
#[component]
pub(crate) fn BandTag() -> impl IntoView {
  let location = use_location();
  let kind = move || registry::by_path(&location.pathname.get()).and_then(registry::Module::kind);
  move || {
    kind().map(|k| {
      view! {
        <span class="pxl-band-tag" aria-hidden="true">
          <Led tone=LedTone::Info state=LedState::On small=true />
          {move || band_label(k)}
        </span>
      }
    })
  }
}

/// 一级模块的名字；字面量写在这里（`check-i18n` 只认调用点的直接实参）。
fn band_label(kind: ModuleKind) -> String {
  match kind {
    ModuleKind::Exam => t("shell.exam-center"),
    ModuleKind::Knowledge => t("shell.knowledge"),
    ModuleKind::Tool => t("shell.tools"),
  }
}
