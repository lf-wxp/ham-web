//! 同步通道的 I/O 分派：手动按钮只认 [`SyncChannel`]，不关心去处是谁。
//!
//! 两条通道的**手动**动作（写入 / 恢复 / 合并）交换的是同一份整份内容、返回同一套结果类型
//! （[`Pushed`] / [`PullResult`]），区别只在这里的 `match`：自定义远端走用户配的两个接口
//! （`crate::remote_sync`），在线账号走同源的 `/api/account/backup`（`crate::account_sync`）。
//!
//! **自动**同步不在这里：两条通道的判定办法不同（自定义远端只有一个整份指纹，在线账号按 key 对比），
//! 分别在 `crate::auto_sync` 的两条分支里，见该模块文档。

use ham_web_core::sync_channel::SyncChannel;

use crate::account_sync;
use crate::remote_sync::{self, PullResult, Pushed};

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
