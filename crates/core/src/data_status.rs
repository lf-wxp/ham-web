//! 实时数据的可用性口径：一份快照还能不能用、一页的多次请求该怎么对外描述。
//!
//! 快照的存取在 `crates/app/src/data.rs`，这里只放与浏览器无关的判定。把 TTL 与聚合规则
//! 放在 core，是为了让缓存层和提示组件共用同一个「7 天」与同一套分档，不会各写一份、
//! 也不会出现「缓存还认这份数据、提示却说数据全没了」的错位。

/// 离线快照的最大可用时长（7 天，毫秒）。
pub const SNAPSHOT_MAX_AGE_MS: i64 = 7 * 24 * 60 * 60 * 1000;

/// 一次同源 API 请求的数据来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiOrigin {
  /// 这次真的拿到了实时响应。
  Live,
  /// 请求失败，用的是本机最后一次成功的响应。
  Snapshot { cached_at_ms: i64 },
  /// 请求失败，本机也没有可用快照。
  Unavailable,
}

/// 一页实时数据的聚合状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageDataState {
  /// 全部实时，或这一页没有实时请求。
  Live,
  /// 至少一块用了离线快照：页面仍有内容，缺的只是实时性。
  /// `incomplete` 为真表示同页还有别的块既没拿到实时数据、也没有快照（那几块是空的）。
  Snapshot { cached_at_ms: i64, incomplete: bool },
  /// 有的块实时、有的块取不到：缺失是局部的。
  Partial,
  /// 本页所有实时请求都没拿到：整块实时数据不可用。
  Unavailable,
}

/// 快照是否还在有效期内。
///
/// 用户把系统时间往前调时 `now - cached_at` 会是负数，此时仍按「刚存下」处理：
/// 否则一次时钟回拨会让离线页面突然丢掉本来能看的数据。
#[must_use]
pub fn is_snapshot_fresh(cached_at_ms: i64, now_ms: i64) -> bool {
  now_ms.saturating_sub(cached_at_ms) <= SNAPSHOT_MAX_AGE_MS
}

/// 校正从本机读出的快照时间：损坏（非正数、超出 JS `Date` 的表示范围）返回 `None`，
/// 写在「未来」（时钟回拨）的按 `now_ms` 截断。提示里显示的时间因此不会是 `0000-…` 或未来。
#[must_use]
pub fn sanitize_snapshot_time(cached_at_ms: i64, now_ms: i64) -> Option<i64> {
  /// JS `Date` 能表示的最大毫秒数（±8.64e15）。
  const MAX_DATE_MS: i64 = 8_640_000_000_000_000;
  if cached_at_ms <= 0 || cached_at_ms > MAX_DATE_MS {
    return None;
  }
  Some(cached_at_ms.min(now_ms))
}

/// 把本页各接口的来源合成一条提示。
///
/// 有快照就报快照，不因为同页还有别的失败而升级成「整页不可用」；同页若另有块两头都落空，
/// 用 `incomplete` 如实标出，免得「有快照」被读成「都有数据」。多份快照取最近写入的那一份，
/// 提示里的时间才是用户能核对的那次。没有快照时再按「局部 / 整体」分档。
#[must_use]
pub fn summarize_origins(origins: &[ApiOrigin]) -> PageDataState {
  let mut latest_snapshot: Option<i64> = None;
  let mut any_live = false;
  let mut any_unavailable = false;
  for origin in origins {
    match origin {
      ApiOrigin::Live => any_live = true,
      ApiOrigin::Snapshot { cached_at_ms } => {
        latest_snapshot = Some(latest_snapshot.map_or(*cached_at_ms, |ts| ts.max(*cached_at_ms)));
      }
      ApiOrigin::Unavailable => any_unavailable = true,
    }
  }
  match (latest_snapshot, any_live, any_unavailable) {
    (Some(cached_at_ms), _, incomplete) => PageDataState::Snapshot {
      cached_at_ms,
      incomplete,
    },
    (None, true, true) => PageDataState::Partial,
    (None, false, true) => PageDataState::Unavailable,
    (None, _, false) => PageDataState::Live,
  }
}

#[cfg(test)]
mod tests {
  use super::{
    ApiOrigin, PageDataState, SNAPSHOT_MAX_AGE_MS, is_snapshot_fresh, sanitize_snapshot_time,
    summarize_origins,
  };

  #[test]
  fn snapshot_expires_after_seven_days() {
    assert!(is_snapshot_fresh(1_000, 1_000 + SNAPSHOT_MAX_AGE_MS));
    assert!(!is_snapshot_fresh(1_000, 1_000 + SNAPSHOT_MAX_AGE_MS + 1));
  }

  #[test]
  fn clock_rollback_keeps_the_snapshot() {
    assert!(is_snapshot_fresh(10_000, 1_000));
  }

  #[test]
  fn all_live_and_empty_are_live() {
    assert_eq!(summarize_origins(&[]), PageDataState::Live);
    assert_eq!(
      summarize_origins(&[ApiOrigin::Live, ApiOrigin::Live]),
      PageDataState::Live
    );
  }

  #[test]
  fn corrupt_snapshot_time_is_rejected_and_future_is_clamped() {
    assert_eq!(sanitize_snapshot_time(0, 1_000), None);
    assert_eq!(sanitize_snapshot_time(-5, 1_000), None);
    assert_eq!(sanitize_snapshot_time(i64::MAX, 1_000), None);
    assert_eq!(sanitize_snapshot_time(500, 1_000), Some(500));
    assert_eq!(sanitize_snapshot_time(5_000, 1_000), Some(1_000));
  }

  #[test]
  fn snapshot_with_live_neighbours_is_complete() {
    assert_eq!(
      summarize_origins(&[ApiOrigin::Snapshot { cached_at_ms: 42 }, ApiOrigin::Live]),
      PageDataState::Snapshot {
        cached_at_ms: 42,
        incomplete: false
      }
    );
  }

  #[test]
  fn snapshot_next_to_unavailable_is_marked_incomplete() {
    assert_eq!(
      summarize_origins(&[
        ApiOrigin::Unavailable,
        ApiOrigin::Snapshot { cached_at_ms: 42 },
        ApiOrigin::Live,
      ]),
      PageDataState::Snapshot {
        cached_at_ms: 42,
        incomplete: true
      }
    );
  }

  #[test]
  fn latest_snapshot_wins() {
    assert_eq!(
      summarize_origins(&[
        ApiOrigin::Snapshot { cached_at_ms: 100 },
        ApiOrigin::Snapshot { cached_at_ms: 900 },
        ApiOrigin::Snapshot { cached_at_ms: 500 },
      ]),
      PageDataState::Snapshot {
        cached_at_ms: 900,
        incomplete: false
      }
    );
  }

  #[test]
  fn missing_data_is_local_or_whole_page() {
    assert_eq!(
      summarize_origins(&[ApiOrigin::Live, ApiOrigin::Unavailable]),
      PageDataState::Partial
    );
    assert_eq!(
      summarize_origins(&[ApiOrigin::Unavailable, ApiOrigin::Unavailable]),
      PageDataState::Unavailable
    );
  }
}
