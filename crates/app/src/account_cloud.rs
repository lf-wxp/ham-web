//! 在线账号的「云端」：按 key 增量同步用到的三个同源接口。
//!
//! 引擎（`ham_web_core::delta_sync`）对云端是泛型的，这里是浏览器里的那个实现：
//! `GET /sync/manifest`（清单）、`POST /sync/pull`（按 key 取值）、`POST /sync/push`（按 key 上传，
//! 每条带着「我看到的云端那一版」，对不上服务端回 `412`，整批不生效）。
//! 请求、超时、错误分类与手动同步共用一套（`account_sync`），鉴权靠 `HttpOnly` 会话 Cookie，
//! 并带上 `X-Account-Id`：另一个标签页换了账号时，这个页面的自动同步不能把数据写进别人的云端。

use std::collections::BTreeMap;

use ham_web_core::account::{
  Change, ErrorCode, Precondition, PullRequest, PullResponse, PushRequest, PushResponse,
  SYNC_MANIFEST_PATH, SYNC_PULL_PATH, SYNC_PUSH_PATH, SyncManifest,
};
use ham_web_core::delta_sync::Cloud;

use crate::account_sync::{TIMEOUT_MS, call_within, failure, is_success};
use crate::remote_sync::SyncError;

/// 正在做的是哪一步：失败时界面要说「获取失败」还是「上传失败」。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Op {
  Manifest,
  Pull,
  Push,
}

/// 账号云端。
pub(crate) struct AccountCloud {
  /// 取清单的超时：启动时的自动获取不能被一个挂住的服务拖太久。取值与上传仍用宽松的超时（备份可达几十 MB）。
  manifest_timeout_ms: i32,
  /// 最近一次请求属于哪一步（失败时用）。
  pub(crate) last_op: Op,
}

impl AccountCloud {
  pub(crate) fn new(manifest_timeout_ms: i32) -> Self {
    Self {
      manifest_timeout_ms,
      last_op: Op::Manifest,
    }
  }
}

/// 响应体解析失败：服务端回了不是它自己的 JSON（反代出错页之类），按服务端内部错误算。
fn malformed() -> SyncError {
  SyncError::Account(ErrorCode::Internal)
}

impl Cloud for AccountCloud {
  type Error = SyncError;

  async fn manifest(&mut self) -> Result<Option<SyncManifest>, SyncError> {
    self.last_op = Op::Manifest;
    let (status, body) = call_within(
      "GET",
      SYNC_MANIFEST_PATH,
      None,
      true,
      Precondition::None,
      self.manifest_timeout_ms,
    )
    .await?;
    match status {
      204 => Ok(None),
      s if is_success(s) => serde_json::from_str(&body)
        .map(Some)
        .map_err(|_| malformed()),
      s => Err(failure(s, &body)),
    }
  }

  async fn pull(&mut self, keys: &[String]) -> Result<BTreeMap<String, String>, SyncError> {
    self.last_op = Op::Pull;
    let request = serde_json::to_string(&PullRequest {
      keys: keys.to_vec(),
    })
    .map_err(|_| malformed())?;
    let (status, body) = call_within(
      "POST",
      SYNC_PULL_PATH,
      Some(request),
      true,
      Precondition::None,
      TIMEOUT_MS,
    )
    .await?;
    if !is_success(status) {
      return Err(failure(status, &body));
    }
    serde_json::from_str::<PullResponse>(&body)
      .map(|r| r.values)
      .map_err(|_| malformed())
  }

  async fn push(&mut self, changes: Vec<Change>) -> Result<PushResponse, SyncError> {
    self.last_op = Op::Push;
    let request = serde_json::to_string(&PushRequest { changes }).map_err(|_| malformed())?;
    let (status, body) = call_within(
      "POST",
      SYNC_PUSH_PATH,
      Some(request),
      true,
      Precondition::None,
      TIMEOUT_MS,
    )
    .await?;
    if !is_success(status) {
      return Err(failure(status, &body));
    }
    serde_json::from_str(&body).map_err(|_| malformed())
  }

  fn is_stale(error: &SyncError) -> bool {
    matches!(error, SyncError::Account(ErrorCode::PreconditionFailed))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn only_a_failed_precondition_means_the_cloud_moved_on() {
    assert!(AccountCloud::is_stale(&SyncError::Account(
      ErrorCode::PreconditionFailed
    )));
    // 其余错误（没登录、换了账号、限流、网络……）重来也一样，要如实报告而不是悄悄重试。
    for error in [
      SyncError::Account(ErrorCode::Unauthorized),
      SyncError::Account(ErrorCode::AccountChanged),
      SyncError::Account(ErrorCode::TooLarge),
      SyncError::Network("x".into()),
      SyncError::Timeout,
      SyncError::Http(412),
    ] {
      assert!(!AccountCloud::is_stale(&error));
    }
  }
}
