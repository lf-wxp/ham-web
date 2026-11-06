//! 把云端的变化按 key 落到本机（在线账号增量同步的「设备」那一半）。
//!
//! 与 [`crate::util::import_backup`] 的区别：那是用户手动恢复一份备份，整份按 key 叠加，不核对本机现状；
//! 这里是自动同步，**每个要写的 key 先核对本机现在还是不是引擎读快照时的样子**，不是就一个都不写
//! （用户在取回云端期间动了数据，覆盖会把他刚做的事吞掉）。核对与写入在同一把备份锁里，页面此时是 `inert` 的，
//! 用户动不了，所以这是最后一次、也是足够的检查点。
//!
//! 写的方式与手动导入一致：普通 key 直接写 `localStorage`，通联日志走 [`crate::kv::restore_large`]
//! （等 IndexedDB 落盘），QSL 影像走 IndexedDB 直写并维护本机的影像索引（索引是各设备自己的，不同步）。

use std::collections::{BTreeMap, BTreeSet};

use ham_web_core::backup_merge::merge_value;
use ham_web_core::delta::{Digest, Manifest, is_image};
use ham_web_core::delta_sync::{Applied, Edits};
use ham_web_core::logbook::Logbook;
use ham_web_core::qsl_image;

use crate::i18n::t;
use crate::util::window;

/// 本机现在这个 key 的内容；没有为 `None`。取值的来源与 [`crate::util::build_sync_backup`] 完全一致，
/// 摘要才能和引擎手里的本机清单直接比较。
async fn current_text(local: &web_sys::Storage, key: &str) -> Result<Option<String>, String> {
  let failed = |_| t("tools.backup-write-failed");
  if key == "logbook" {
    return crate::kv::load_large_strict(key).await.map_err(failed);
  }
  if is_image(key) {
    // 本机只会有编号规范的影像；别的写法的 key 本机不可能有。
    return match qsl_image::entry_id_from_key(key) {
      Some(_) => crate::kv::load_store_only_strict(key).await.map_err(failed),
      None => Ok(None),
    };
  }
  Ok(local.get_item(key).ok().flatten())
}

/// 落盘。返回 [`Applied::LocalChanged`] 时什么都没写。
///
/// 没写成但不该让整轮失败的 key（内容不合格的影像 / 日志、没有合并规则能处理的值）记进 `skipped`，
/// 引擎不会给它们记基线。
pub async fn apply(edits: &Edits, expect: &Manifest) -> Result<Applied, String> {
  let result = apply_inner(edits, expect).await;
  // 无论成败都丢弃收藏 / 分组 / 笔记的内存缓存：直接写 localStorage 不会触发本标签页的 `storage` 事件，
  // 缓存不失效的话，之后的一次收藏就会用旧缓存整表覆盖刚落下的数据（理由同 `import_backup`）。
  crate::store::invalidate_all();
  result
}

async fn apply_inner(edits: &Edits, expect: &Manifest) -> Result<Applied, String> {
  let Some(local) = window().local_storage().ok().flatten() else {
    return Err(t("shell.localstorage-unavailable"));
  };
  for key in edits.keys() {
    let now = current_text(&local, key).await?.map(|v| Digest::of(&v));
    if now != expect.get(key).copied() {
      return Ok(Applied::LocalChanged);
    }
  }

  // 先做完全部「能提前判定」的校验，再动手写：写到一半才发现某条内容不合格，
  // 会留下半份数据——引擎不会记基线，下一轮把半份当成本机改动推上云。
  let mut skipped = Vec::new();
  let mut set: BTreeMap<String, String> = BTreeMap::new();
  for (key, value) in &edits.set {
    let ok = if is_image(key) {
      qsl_image::entry_id_from_key(key).is_some()
    } else if key == "logbook" {
      // 云端内容是服务端规范化过的，仍然核对：解析不了的日志不能覆盖本机。
      serde_json::from_str::<Logbook>(value).is_ok()
    } else {
      true
    };
    if ok {
      set.insert(key.clone(), value.clone());
    } else {
      skipped.push(key.clone());
    }
  }
  // 合并也要先算完：没有合并规则的 key 直接跳过，别写了一半才发现。
  let mut merge: BTreeMap<String, String> = BTreeMap::new();
  for (key, remote) in &edits.merge {
    if key == "logbook" || is_image(key) {
      continue;
    }
    let current = local.get_item(key).ok().flatten();
    match merge_value(key, current.as_deref(), remote) {
      Some(merged) => {
        merge.insert(key.clone(), merged);
      }
      None => skipped.push(key.clone()),
    }
  }
  let mut remove = Vec::new();
  for key in &edits.remove {
    if is_image(key) && qsl_image::entry_id_from_key(key).is_none() {
      skipped.push(key.clone());
      continue;
    }
    remove.push(key.clone());
  }

  let write_failed = || t("tools.backup-write-failed");
  let mut added = BTreeSet::new();
  let mut removed = BTreeSet::new();

  for (key, value) in &set {
    if is_image(key) {
      match qsl_image::entry_id_from_key(key) {
        Some(id) if crate::util::import_qsl_image(key, value).await? => {
          added.insert(id);
        }
        _ => skipped.push(key.clone()),
      }
    } else if key == "logbook" {
      if !crate::kv::restore_large(key, value).await {
        return Err(write_failed());
      }
    } else {
      local.set_item(key, value).map_err(|_| write_failed())?;
    }
  }

  for (key, merged) in &merge {
    local.set_item(key, merged).map_err(|_| write_failed())?;
  }

  for key in &remove {
    if is_image(key) {
      if let Some(id) = qsl_image::entry_id_from_key(key) {
        if !crate::kv::remove_store_only(key).await {
          return Err(write_failed());
        }
        removed.insert(id);
      }
    } else {
      // 删不掉（配额 / 隐私模式）不影响一致性：下一轮三方对比会发现它还在，重新处理。
      let _ = local.remove_item(key);
    }
  }

  update_image_index(&local, &added, &removed)?;
  Ok(Applied::Done { skipped })
}

/// 本机的影像索引（哪些通联有影像）跟着影像增删。索引只在本机，不同步。
fn update_image_index(
  local: &web_sys::Storage,
  added: &BTreeSet<u64>,
  removed: &BTreeSet<u64>,
) -> Result<(), String> {
  if added.is_empty() && removed.is_empty() {
    return Ok(());
  }
  let mut index: BTreeSet<u64> = local
    .get_item(qsl_image::INDEX_KEY)
    .ok()
    .flatten()
    .and_then(|v| serde_json::from_str::<Vec<u64>>(&v).ok())
    .unwrap_or_default()
    .into_iter()
    .collect();
  index.extend(added);
  for id in removed {
    index.remove(id);
  }
  let value = serde_json::to_string(&index).map_err(|e| e.to_string())?;
  local
    .set_item(qsl_image::INDEX_KEY, &value)
    .map_err(|_| t("tools.backup-write-failed"))
}
