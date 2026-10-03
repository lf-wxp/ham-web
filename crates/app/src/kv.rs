//! 统一离线存储门面：对外暴露一套 KV API，内部按数据规模自动分流。
//!
//! - 小数据 → [`crate::util::storage`]（localStorage，同步、简单、天然跨标签页同步）。
//! - 大数据 → [`crate::idb`]（IndexedDB，异步 + 内存缓存，突破 5MB 配额）。
//!
//! 大数据采用「localStorage 静默快照 + IndexedDB 权威」双写：
//! - 快照保证同步读可用、无闪烁，写满时静默丢弃，不弹「存储已满」；
//! - IndexedDB 权威落盘承载大体积，`idb` 的进程内缓存保证写后立即可读。
//!
//! 新增数据时：小数据直接走 [`crate::util::storage`]，大数据调用 [`save_large`] /
//! [`load_large`]，无需在两套 API 之间手写组合。

use crate::util::storage;

/// 写入大体积数据：localStorage 静默快照 + IndexedDB 权威（异步 fire-and-forget）。
///
/// 返回前已同步更新 IndexedDB 进程内缓存（[`crate::idb::get`] 立即可读），落盘在
/// 后台完成，失败静默（数据仍有 localStorage 快照兜底）。
pub fn save_large(key: &str, value: &str) {
  storage::set_silent(key, value);
  // 先同步更新 IndexedDB 进程内缓存，保证本函数返回后 [`crate::idb::get`] 立即读到
  // 新值（否则 `cache_put` 会随下方 spawn 的异步任务延迟执行，写后立即读存在未命中窗口）。
  crate::idb::cache_put(key, value);
  // 异步闭包需持有 owned 数据，避免函数返回后 `key`/`value` 变成悬垂引用。
  let key = key.to_owned();
  let value = value.to_owned();
  wasm_bindgen_futures::spawn_local(async move {
    let _ = crate::idb::set(&key, &value).await;
  });
}

/// 读取大体积数据（异步）：优先 IndexedDB 进程内缓存，未命中回退 IndexedDB。
pub async fn load_large(key: &str) -> Option<String> {
  crate::idb::get(key).await.ok().flatten()
}

/// 删除大体积数据：移除 localStorage 快照 + IndexedDB（异步 fire-and-forget）。
///
/// 当前业务尚无删除大数据的场景，保留作为门面的完整 API。
#[allow(dead_code)]
pub fn remove_large(key: &str) {
  storage::remove(key);
  // 同 [`save_large`]：先同步移除进程内缓存，保证立即可见。
  crate::idb::cache_remove(key);
  let key = key.to_owned();
  wasm_bindgen_futures::spawn_local(async move {
    let _ = crate::idb::remove(&key).await;
  });
}
