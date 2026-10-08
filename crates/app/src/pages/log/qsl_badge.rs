//! QSL 状态徽章：把 [`QslStatus`] 映射成四色档位 + 文案。
//!
//! 状态与优先级的推导全在 `ham_web_core::qsl_status`（可单测），这里只做两件事：
//! 把语义档位 [`QslTone`] 翻成配色类名，把状态翻成文案。四色与 `bands` 页的
//! `usage_class(Usage)` 是同一套写法：`bg-*-100` 浅色底 + `dark:bg-*-900/40` 深色底，
//! 保证明暗两套主题下对比度都够。

use ham_web_core::qsl_status::{QslConfirm, QslStatus, QslTone, QslVia};
use leptos::prelude::*;

use crate::i18n::t;

/// 语义档位 → 徽章配色。
pub(super) fn tone_class(tone: QslTone) -> &'static str {
  match tone {
    QslTone::Muted => "bg-muted text-muted-foreground",
    QslTone::Amber => "bg-amber-100 text-amber-800 dark:bg-amber-900/40 dark:text-amber-300",
    QslTone::Sky => "bg-sky-100 text-sky-800 dark:bg-sky-900/40 dark:text-sky-300",
    QslTone::Emerald => {
      "bg-emerald-100 text-emerald-800 dark:bg-emerald-900/40 dark:text-emerald-300"
    }
  }
}

/// 状态文案。`t()` 的**字面量必须写在调用点**：静态扫描靠它认条目，走
/// `fn key() -> &'static str` 会被判成死条目、CI 直接红。
pub(super) fn status_label(status: QslStatus) -> String {
  match status {
    QslStatus::NotSent => t("log.not-sent"),
    QslStatus::Sent(_) => t("log.sent"),
    QslStatus::Confirmed(QslConfirm::Paper) => t("log.qsl-received"),
    QslStatus::Confirmed(QslConfirm::Lotw) => t("log.lotw-confirmed"),
    QslStatus::Confirmed(QslConfirm::Eqsl) => t("log.eqsl-confirmed"),
  }
}

/// 四色档位的筛选文案（与徽章同口径，颜色即「走到哪一步」）。
pub(super) fn tone_label(tone: QslTone) -> String {
  match tone {
    QslTone::Muted => t("log.not-sent"),
    QslTone::Amber => t("log.sent"),
    QslTone::Sky => t("log.qsl-electronic-confirmed"),
    QslTone::Emerald => t("log.qsl-received"),
  }
}

/// 寄出方式短文案（只作为「已寄出」徽章的后缀）。
pub(super) fn via_label(via: QslVia) -> String {
  match via {
    QslVia::Bureau => t("log.qsl-via-bureau"),
    QslVia::Direct => t("log.qsl-via-direct"),
    QslVia::None => String::new(),
  }
}

/// QSL 状态徽章。文案包在闭包里（而不是在组件构造时取一次），这样切换语言时
/// 徽章会跟着变 —— 列表行本身不依赖语言信号，不会因为换语言而重建。
#[component]
pub(super) fn QslBadge(status: QslStatus) -> impl IntoView {
  // 只有「已寄出」才附带方式后缀；确认状态下方式是无关信息（看的是确认途径）。
  let via = match status {
    QslStatus::Sent(via) => via,
    _ => QslVia::None,
  };
  view! {
    <span class=format!(
      "inline-flex items-center gap-1 whitespace-nowrap rounded-md px-2 py-0.5 text-xs font-medium {}",
      tone_class(status.tone()),
    )>
      {move || status_label(status)}
      {(via != QslVia::None)
        .then(|| {
          view! {
            <span class="font-normal opacity-70">{move || format!("·{}", via_label(via))}</span>
          }
        })}
    </span>
  }
}
