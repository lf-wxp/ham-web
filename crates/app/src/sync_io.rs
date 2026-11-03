//! 同步通道的 I/O 分派：自动同步与手动按钮只认 [`SyncChannel`]，不关心去处是谁。
//!
//! 两条通道交换的是同一份内容、返回同一套结果类型（[`Fetched`] / [`Pushed`] / [`PushError`] /
//! [`PullResult`]），区别只在这里的 `match`：自定义远端走用户配的两个接口
//! （`crate::remote_sync`），在线账号走同源的 `/api/account/backup`（`crate::account_sync`）。
//! 新增第三条通道时，改这一个文件加一个分支，`auto_sync` 与面板都不用动。

use ham_web_core::remote_sync::Fingerprint;
use ham_web_core::sync_channel::SyncChannel;

use crate::account_sync;
use crate::remote_sync::{self, Fetched, PullResult, PushError, Pushed};

/// 这条通道配好了「获取」的去处。
///
/// 账号通道恒为真：去处是同源接口，「没登录」是运行时状态（请求时回 `unauthorized`），
/// 不是配置缺失。
pub fn can_pull(channel: SyncChannel) -> bool {
  match channel {
    SyncChannel::Custom => remote_sync::with_config(|c| c.can_pull()),
    SyncChannel::Account => true,
  }
}

/// 这条通道配好了「写入」的去处。
pub fn can_push(channel: SyncChannel) -> bool {
  match channel {
    SyncChannel::Custom => remote_sync::with_config(|c| c.can_push()),
    SyncChannel::Account => true,
  }
}

/// 取回远端内容，不动本机数据。错误已是界面文案。
pub async fn fetch(channel: SyncChannel, timeout_ms: i32) -> Result<Fetched, String> {
  match channel {
    SyncChannel::Custom => remote_sync::fetch(&remote_sync::config(), timeout_ms).await,
    SyncChannel::Account => account_sync::fetch(timeout_ms).await,
  }
}

/// 取回内容的指纹；不是备份时给出带通道措辞（「远端」/「云端」）的错误。
pub fn fingerprint_of(channel: SyncChannel, body: &str) -> Result<Fingerprint, String> {
  match channel {
    SyncChannel::Custom => remote_sync::fingerprint_of(body),
    SyncChannel::Account => account_sync::fingerprint_of(body),
  }
}

/// 按这条通道的选项生成同步内容后的本机指纹；生成失败（如超过体积上限）为 `None`。
pub async fn local_fingerprint(channel: SyncChannel) -> Option<Fingerprint> {
  match channel {
    SyncChannel::Custom => remote_sync::local_fingerprint(&remote_sync::config()).await,
    SyncChannel::Account => account_sync::local_fingerprint().await,
  }
}

/// 自动写入：本机内容与基线一致就不发请求。
pub async fn push_if_changed(
  channel: SyncChannel,
  base: Option<Fingerprint>,
) -> Result<Option<Pushed>, PushError> {
  match channel {
    SyncChannel::Custom => remote_sync::push_if_changed(&remote_sync::config(), base).await,
    SyncChannel::Account => account_sync::push_if_changed(base).await,
  }
}

/// 手动写入：一定发，不设条件。错误已是界面文案。
pub async fn push(channel: SyncChannel) -> Result<Pushed, String> {
  match channel {
    SyncChannel::Custom => remote_sync::push(&remote_sync::config()).await,
    SyncChannel::Account => account_sync::push().await,
  }
}

/// 手动恢复（`merge` 为真走合并导入）。错误已是界面文案。
pub async fn pull(channel: SyncChannel, merge: bool) -> Result<PullResult, String> {
  match channel {
    SyncChannel::Custom => remote_sync::pull(&remote_sync::config(), merge).await,
    SyncChannel::Account => account_sync::pull(merge).await,
  }
}
