use ham_web_core::sync_channel::SyncChannel;
use leptos::prelude::*;

use crate::auto_sync::AutoStatus;
use crate::i18n::{t, tf};

/// 自动同步的状态行：只在**这条通道正在自动同步**时显示。
///
/// 两条通道共用同一个自动同步状态机（一次只跑一条），所以状态只属于正在跑的那条；
/// 另一条通道的面板不显示它，免得把「自定义远端冲突」读成「账号冲突」。
/// 正常状态是普通正文色，需要用户处理的问题用状态色，并带 `role="status"`。
#[component]
pub(super) fn SyncStatusLine(channel: SyncChannel) -> impl IntoView {
  let status = crate::auto_sync::status();
  let active = crate::auto_sync::active_channel();
  view! {
    {move || {
      if active.get() != Some(channel) {
        return None;
      }
      let account = channel == SyncChannel::Account;
      let (text, is_problem) = match status.get() {
        AutoStatus::Off => return None,
        AutoStatus::Syncing => (t("tools.remote-auto-syncing"), false),
        AutoStatus::Synced(at) => (tf("tools.remote-auto-synced", &[&at]), false),
        AutoStatus::Conflict if account => (t("tools.account-auto-conflict"), true),
        AutoStatus::Conflict => (t("tools.remote-auto-conflict"), true),
        AutoStatus::NeedsFirstPush => (t("tools.remote-auto-first-push"), true),
        AutoStatus::PullFailed(e) if account => (tf("tools.account-auto-pull-paused", &[&e]), true),
        AutoStatus::PullFailed(e) => (tf("tools.remote-auto-pull-paused", &[&e]), true),
        AutoStatus::PushFailed(e) => (tf("tools.remote-auto-push-retry", &[&e]), true),
        AutoStatus::PushPaused(e) if account => (tf("tools.account-auto-push-paused", &[&e]), true),
        AutoStatus::PushPaused(e) => (tf("tools.remote-auto-push-paused", &[&e]), true),
        AutoStatus::SignedOut => (t("tools.account-auto-signed-out"), true),
        AutoStatus::MassDeletion => (t("tools.account-auto-mass-deletion"), true),
      };
      let class = if is_problem { "text-xs text-hp-text" } else { "text-xs" };
      Some(view! { <p role="status" class=class>{text}</p> })
    }}
  }
}
