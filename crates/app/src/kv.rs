//! 统一离线存储门面：对外暴露一套 KV API，内部按数据规模自动分流。
//!
//! - 小数据 → [`crate::util::storage`]（localStorage，同步、简单、天然跨标签页同步）。
//! - 大数据 → [`crate::idb`]（IndexedDB，异步 + 内存缓存，突破 5MB 配额）。
//!
//! 大数据采用「localStorage 静默快照 + IndexedDB 权威」双写：
//! - 快照保证同步读可用、无闪烁，写满时静默丢弃，不弹「存储已满」；快照里放的是**原样的
//!   payload** —— `backup_merge` 等既有逻辑直接读 localStorage 并解析 JSON，不能包一层；
//! - IndexedDB 权威落盘承载大体积，`idb` 的进程内缓存保证写后立即可读。
//!
//! 新增数据时：小数据直接走 [`crate::util::storage`]，大数据调用 [`save_large`] /
//! [`load_large`]，无需在两套 API 之间手写组合。
//!
//! # 写入版本：决定「哪一层说了算」
//!
//! 两层写入的完成语义不同 —— 快照是**同步**写（要么整笔成败），IndexedDB 是**异步**落盘
//! （页面在事务提交前卸载就会被取消）。于是存在一个真实窗口：用户改完数据立刻按 F5 /
//! 关标签页，快照已是新值、IndexedDB 还是旧值；若加载时一律以 IndexedDB 为准，最后一笔
//! 改动就被静默回滚（实测在 QSL 标签页「标记为已寄出」后立刻跳转复现过）。反过来，
//! localStorage 配额写满时快照会静默丢写，此时又必须以 IndexedDB 为准。
//!
//! 所以每次写入都带一个**单调递增的写入版本**（见 [`bump`]），加载时取版本更高的一侧：
//!
//! - **快照侧**：版本号写在同一 key 的兄弟项 `"<key>:kv-version"` 里，顺序是「先值后版本」。
//!   值写失败（配额）就不动版本，留下一对自洽的旧值；版本写失败则把两者一起删掉 ——
//!   宁可退化成「只有权威层」，也不要留一对互相矛盾的状态。
//! - **权威侧**：`"{版本}\n{payload}"` 放在**同一条 IndexedDB 记录**里，单条记录天然原子。
//!   拆成两条写会出现「payload 事务被取消、版本事务却提交」这种更糟的不一致。
//!
//! 两侧版本并列时以权威层为准（含旧数据的两侧都是 0）：那是历史行为，而且并列通常意味着
//! 两侧内容本就相同。

use std::cell::Cell;

use crate::util::storage;

/// 快照版本号的 key 后缀。
///
/// 它是门面内部约定，但会随 [`crate::util::export_backup`] 一起进出备份文件 —— 恢复了
/// 带版本号的备份时，[`mirror_snapshots`] 正是靠它认出「哪些 key 由门面托管」。
pub const VERSION_SUFFIX: &str = ":kv-version";

thread_local! {
  /// 上一次分配的版本号：同一毫秒内连续写入也要严格递增，否则会出现并列。
  static LAST_VERSION: Cell<u64> = const { Cell::new(0) };
}

/// 由「当前毫秒」与「上一次的版本」算出下一个版本：时钟回拨也保证严格递增。
#[must_use]
fn bump(last: u64, now_ms: u64) -> u64 {
  now_ms.max(last.saturating_add(1))
}

/// 分配下一个写入版本。
fn next_version() -> u64 {
  let now = js_sys::Date::now().max(0.0) as u64;
  LAST_VERSION.with(|last| {
    let next = bump(last.get(), now);
    last.set(next);
    next
  })
}

/// 权威层记录的编码：`"{版本}\n{payload}"`。
///
/// 版本与内容写在**同一条记录**里，是为了让它们要么一起新、要么一起旧：分成两个 key
/// 就会出现「内容写了、版本没写」的中间态，而版本正是快照/权威层之间选谁更可信的依据。
///
/// 代价是回滚到改动前的旧前端时，旧代码会把版本头当成 payload 解析（表现为读不出数据）。
/// 那是**一次性**的：旧前端已不再发布，而换一种编码同样要迁移一次存量数据，收益为负。
fn encode(version: u64, value: &str) -> String {
  format!("{version}\n{value}")
}

/// 权威层记录的解码。没有版本头（旧数据）时按版本 0 处理，整串当 payload。
///
/// payload 是 `serde_json` 的紧凑输出、本身不含换行，所以「第一行是不是纯数字」就是
/// 可靠的判别依据 —— 旧数据（整串是一个 JSON 对象）里根本没有换行，会被原样当 payload。
fn decode(raw: &str) -> (u64, &str) {
  match raw.split_once('\n') {
    Some((head, rest)) => head.parse::<u64>().map_or((0, raw), |v| (v, rest)),
    None => (0, raw),
  }
}

/// 快照是否比权威记录更新；并列时以权威层为准。
#[must_use]
fn snapshot_wins(snapshot_version: u64, stored_version: u64) -> bool {
  snapshot_version > stored_version
}

/// 快照版本号的存取 key。
fn version_key(key: &str) -> String {
  format!("{key}{VERSION_SUFFIX}")
}

/// 读快照版本号；没有（旧数据）按 0 处理。
fn snapshot_version(key: &str) -> u64 {
  storage::get(&version_key(key))
    .and_then(|s| s.trim().parse::<u64>().ok())
    .unwrap_or(0)
}

/// 同步读快照（首屏用，不做版本比较）。
///
/// 版本比较在异步的 [`load_large`] 里完成，其后会校正首屏内容；这样首屏立刻有内容可渲染，
/// 又不至于把「还没落盘的旧权威值」当成真相。
#[must_use]
pub fn load_snapshot(key: &str) -> Option<String> {
  storage::get(key)
}

/// 写入大体积数据：localStorage 静默快照 + IndexedDB 权威（异步 fire-and-forget）。
///
/// 返回前已同步更新 IndexedDB 进程内缓存（[`crate::idb::get`] 立即可读），落盘在
/// 后台完成，失败静默（数据仍有 localStorage 快照兜底）。两侧各带一个新版本号，
/// 见模块文档。
pub fn save_large(key: &str, value: &str) {
  let version = next_version();
  // 快照：先值后版本。值没写进去（配额）就连版本都不动；版本没写进去则整对作废，
  // 以免留下「新值配旧版本」这种会被误判成过期的组合。
  if storage::set_silent(key, value)
    && !storage::set_silent(&version_key(key), &version.to_string())
  {
    storage::remove(key);
    storage::remove(&version_key(key));
  }
  // 权威层：版本与 payload 必须同一条记录（见模块文档）。
  let record = encode(version, value);
  crate::idb::cache_put(key, &record);
  let key = key.to_owned();
  wasm_bindgen_futures::spawn_local(async move {
    if let Err(e) = crate::idb::set(&key, &record).await {
      // 快照那侧已经静默（`set_silent`，避免配额满时反复弹警告）；权威层再静默的话，
      // 两侧都失败就变成「刷新后数据凭空消失、且没有任何线索」。这里只记一条 console，
      // 不打断用户操作，但排障时能看见。
      web_sys::console::error_1(
        &format!("[kv] {key} 写入 IndexedDB 失败（localStorage 快照也可能没写进去）：{e:?}").into(),
      );
    }
  });
}

/// 读取大体积数据（异步）：比较两侧的写入版本，返回更新的那一份。
pub async fn load_large(key: &str) -> Option<String> {
  let snapshot = storage::get(key);
  let stored = crate::idb::get(key).await.ok().flatten();
  match (snapshot, stored) {
    (None, None) => None,
    (Some(snapshot), None) => Some(snapshot),
    (None, Some(stored)) => Some(decode(&stored).1.to_owned()),
    (Some(snapshot), Some(stored)) => {
      let (stored_version, stored_value) = decode(&stored);
      Some(if snapshot_wins(snapshot_version(key), stored_version) {
        snapshot
      } else {
        stored_value.to_owned()
      })
    }
  }
}

/// 只写**权威层**（不碰 localStorage 快照）：给天生装不进快照配额的数据，例如 QSL 卡片扫描件。
///
/// 与 [`save_large`] 的区别是**没有快照可以回退**，所以调用方应当 `await` 它、成功后再更新
/// 界面 —— 写入期间页面被卸载就是永久丢失，宁可让用户看到「保存中」，也不要假装成功。
/// 返回是否落盘成功。
pub async fn save_store_only(key: &str, value: &str) -> bool {
  crate::idb::cache_put(key, value);
  crate::idb::set(key, value).await.is_ok()
}

/// 读 [`save_store_only`] 写的数据。
pub async fn load_store_only(key: &str) -> Option<String> {
  crate::idb::get(key).await.ok().flatten()
}

/// 删除 [`save_store_only`] 写的数据，返回是否删除成功。
pub async fn remove_store_only(key: &str) -> bool {
  crate::idb::cache_remove(key);
  crate::idb::remove(key).await.is_ok()
}

/// 删除大体积数据：快照、版本号与权威记录一起删（异步 fire-and-forget）。
///
/// 当前业务尚无删除「快照 + 权威」双写数据的场景，保留作为门面的完整 API。
#[allow(dead_code)]
pub fn remove_large(key: &str) {
  storage::remove(key);
  storage::remove(&version_key(key));
  // 同 [`save_large`]：先同步移除进程内缓存，保证立即可见。
  crate::idb::cache_remove(key);
  let key = key.to_owned();
  wasm_bindgen_futures::spawn_local(async move {
    let _ = crate::idb::remove(&key).await;
  });
}

/// 把快照层的内容回灌权威层（备份 / 合并导入之后调用）。
///
/// [`crate::util::import_backup`] 与 `import_backup_merge` 只写 localStorage；不同步回灌的
/// 话，下次加载时权威层的版本号可能更高（甚至只是旧数据），用户就会看到「提示导入成功、
/// 数据却没变」。这里把所有**带版本号**的 key（即门面托管的大数据）重新走一遍
/// [`save_large`]：内容不变，只是把两侧版本对齐到同一个新版本，于是快照侧不再被判过期。
///
/// 幂等：多回灌一次只是把同样的内容再写一遍。
pub fn mirror_snapshots() {
  for (key, _) in storage::usage() {
    let Some(base) = key.strip_suffix(VERSION_SUFFIX) else {
      continue;
    };
    if let Some(value) = storage::get(base) {
      save_large(base, &value);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::{bump, decode, encode, snapshot_wins};

  #[test]
  fn authority_record_round_trips() {
    let record = encode(1_700_000_000_000, "{\"entries\":[]}");
    let (version, payload) = decode(&record);
    assert_eq!(version, 1_700_000_000_000);
    assert_eq!(payload, "{\"entries\":[]}");
  }

  #[test]
  fn payload_without_a_version_header_counts_as_version_zero() {
    // 旧数据（本次改动之前写进 IndexedDB 的记录）是裸 payload，没有版本头。
    for legacy in ["{\"entries\":[]}", "", "{\"a\":\"b\\nc\"}"] {
      let (version, payload) = decode(legacy);
      assert_eq!(version, 0, "旧记录应按版本 0 处理：{legacy}");
      assert_eq!(payload, legacy, "旧记录整串就是 payload");
    }
    // 「第一行像数字、但整串不是合法版本头」的情况不能被误判：紧凑 JSON 不含换行，
    // 所以只有形如 `数字\n...` 的记录才会被当成带版本头。
    let (version, payload) = decode("12abc\nrest");
    assert_eq!((version, payload), (0, "12abc\nrest"));
  }

  #[test]
  fn versions_are_strictly_increasing_even_within_one_millisecond() {
    let mut last = 0u64;
    for _ in 0..5 {
      last = bump(last, 1_700_000_000_000);
    }
    assert_eq!(last, 1_700_000_000_004, "同一毫秒内必须逐次 +1");
    // 时钟回拨：仍要严格递增，否则「版本更高的一侧」会出现并列而退回权威层。
    assert_eq!(bump(last, 1_000), last + 1);
    // 时钟前进：直接采用更大的时间戳。
    assert_eq!(bump(last, 2_000_000_000_000), 2_000_000_000_000);
  }

  #[test]
  fn the_newer_side_wins_and_ties_fall_back_to_the_authority() {
    // 快照刚写完、权威层还停在上一笔（事务被卸载取消）→ 快照胜。
    assert!(snapshot_wins(1_700_000_000_001, 1_700_000_000_000));
    // 快照写满被静默丢弃、权威层是最新的 → 权威层胜。
    assert!(!snapshot_wins(1_699_999_999_999, 1_700_000_000_000));
    // 并列（含两侧都是旧数据的 0 / 0）→ 权威层胜。
    assert!(!snapshot_wins(0, 0));
    assert!(!snapshot_wins(7, 7));
  }
}
