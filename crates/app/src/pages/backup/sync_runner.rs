//! 两条同步通道的三个手动动作（写入 / 恢复覆盖 / 恢复合并）共用的执行器。
//!
//! 自定义远端面板与在线账号面板的按钮行为完全一致，区别只有去处（`crate::sync_io` 按通道分派）
//! 与提示文案的措辞（「远端」/「云端」）；抽成一个 hook，两个面板各自只管排版。

use ham_web_core::sync_channel::SyncChannel;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::i18n::{t, tp};
use crate::remote_sync::PullResult;
use crate::ui::ToastKind;

/// 三个手动同步动作。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SyncAction {
  PullOverwrite,
  PullMerge,
  Push,
}

/// 「已写入 N 项」的提示。词条按通道各一条，字面量写在这里（静态扫描要认得出）。
fn pushed_text(channel: SyncChannel, count: usize) -> String {
  let n = count.to_string();
  match channel {
    SyncChannel::Custom => tp("tools.remote-pushed", count as u32, &[&n]),
    SyncChannel::Account => tp("tools.account-pushed", count as u32, &[&n]),
  }
}

fn no_data_text(channel: SyncChannel) -> String {
  match channel {
    SyncChannel::Custom => t("tools.remote-no-data"),
    SyncChannel::Account => t("tools.account-no-data"),
  }
}

/// 造一个执行器：调用 `run.run(action)` 即发起一次手动同步。
///
/// - `busy`：本地导入 / 导出正在进行（来自父组件），期间不许再开一次；
/// - `syncing`：同步进行中，由父组件持有，好让导入 / 导出按钮一并禁用；
/// - `on_done`：动作结束（无论成败）后的回调，父组件用它刷新存储占用。
///
/// 必须在组件体里调用（要取日志 Store 与挂载守卫）。
pub(super) fn use_sync_runner(
  channel: SyncChannel,
  busy: Signal<bool>,
  syncing: RwSignal<bool>,
  on_done: Callback<()>,
) -> Callback<SyncAction> {
  let alive = StoredValue::new(crate::util::mount_guard());
  let log_store = crate::pages::log::use_log_store();

  Callback::new(move |action: SyncAction| {
    if busy.get_untracked() || syncing.get_untracked() {
      return;
    }
    let Some(task) = crate::util::begin_backup_task(log_store.backup_busy) else {
      return;
    };
    syncing.set(true);
    let alive = alive.get_value();
    spawn_local(async move {
      let (kind, text) = match action {
        SyncAction::Push => match crate::sync_io::push(channel).await {
          Ok(pushed) => {
            // 写出的总是完整快照，对端现在就等于本机，可以当自动同步的基线。
            crate::auto_sync::note_pushed(channel, pushed.fp, pushed.base);
            (ToastKind::Success, pushed_text(channel, pushed.count))
          }
          Err(e) => (ToastKind::Error, e),
        },
        SyncAction::PullOverwrite | SyncAction::PullMerge => {
          let merge = action == SyncAction::PullMerge;
          let outcome = crate::sync_io::pull(channel, merge).await;
          // 恢复可能写了一半才失败，所以无论成败都把日志 Store 对齐到存储（与手动导入一致）。
          if !matches!(outcome, Ok(PullResult::Empty)) {
            log_store.refresh_from_storage().await;
          }
          match outcome {
            Ok(PullResult::Empty) => (ToastKind::Info, no_data_text(channel)),
            Ok(PullResult::Imported { count: n, fp, base }) => {
              crate::auto_sync::note_pulled(channel, fp, base, merge);
              // 与手动导入同一套词条（走语义 key + `tp`，见 `backup_tool.rs` 的同类注释）。
              let msg = if merge {
                tp("settings.merged-items", n as u32, &[&n.to_string()])
              } else {
                tp("tools.restored-records", n as u32, &[&n.to_string()])
              };
              (ToastKind::Success, msg)
            }
            Err(e) => (ToastKind::Error, e),
          }
        }
      };
      drop(task);
      if !alive() {
        return;
      }
      syncing.set(false);
      crate::ui::push(kind, text);
      on_done.run(());
    });
  })
}
