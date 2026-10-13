//! 备份合并导入：把备份 JSON 中的各 key 与当前本地数据做类型感知合并。
//!
//! 跨设备多份数据应取「并集 / 累加 / 保留更优」，避免整体覆盖导致丢失任一设备的记录。
//! 这里只对核心数据类型做合并；其余 key（主题、各类设置、练习 / 考试进度等）由调用方按
//! 「本机优先」处理（返回 `None` 表示不覆盖）。

use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::daily_challenge::DailyResults;
use crate::exam_history::ExamRecord;
#[cfg(test)]
use crate::logbook::LogEntry;
use crate::logbook::Logbook;
use crate::mistake_book::{
  CategoryStats, MAX_RECORDS, MistakeBook, MistakeRecord, StudyStats, Tally,
};
use crate::qsl_image;
use crate::rpg::{Bestiary, StageStars};
use crate::station::{StationBook, StationProfile};

/// 备份导出与文本导入共用的字节上限，禁止生成本站不能恢复的 JSON。
pub const MAX_BACKUP_BYTES: usize = 64 * 1024 * 1024;

/// 使用序列化后的 UTF-8 字节数，而不是条目数或字符数判断可恢复性。
#[must_use]
pub fn backup_size_is_supported(bytes: usize) -> bool {
  bytes <= MAX_BACKUP_BYTES
}

/// JSON 字节数下界：插入时提前限制驻留数据，最终仍按序列化后的精确字节数检查。
#[must_use]
pub fn backup_payload_bytes(values: &BTreeMap<String, String>) -> usize {
  values.iter().fold(0usize, |total, (key, value)| {
    total.saturating_add(key.len()).saturating_add(value.len())
  })
}

/// 学习统计里每个题库的「做过的题」上限。真实题库每个不到两千题；备份是外部输入，
/// 并集不设上限的话，合法的 64 MiB 备份合并后能把单个 key 撑过 localStorage 的 ~5 MiB 配额。
const MAX_SEEN_PER_BANK: usize = 20_000;
/// 图鉴「已击败」集合与字符串集合（收藏等）的同类上限。
const MAX_SET_ITEMS: usize = 50_000;

/// 集合超过 `max` 时从末尾丢弃多余项（`BTreeSet` 有序，结果是确定性的）。
fn cap_set<T: Ord + Clone>(set: &mut BTreeSet<T>, max: usize) {
  while set.len() > max {
    let Some(last) = set.last().cloned() else {
      break;
    };
    set.remove(&last);
  }
}

/// 考试历史最多保留的条数（与 `ham-web-app` 的 `exam_history` 一致）。
const EXAM_HISTORY_LIMIT: usize = 50;
/// 每日挑战成绩保留天数（与 [`crate::daily_challenge`] 内部一致）。
const DAILY_KEEP_DAYS: usize = 90;

/// 合并单个 key 的备份值。
///
/// * `current` 为本机已有的 JSON（`None` 表示本机没有该 key）；
/// * `backup` 为备份文件里的 JSON。
///
/// 返回值：
/// * 本机没有该 key → 已知类型先校验再采用（半截 / 被篡改的 JSON 不写入本地，返回 `None`），
///   未知 key（如主题这类纯字符串）无从校验，原样采用；
/// * 已知类型 → `Some(合并后的 JSON)`（写回）；
/// * 其余 key → `None`（保留本机，不覆盖）。
#[must_use]
pub fn merge_value(key: &str, current: Option<&str>, backup: &str) -> Option<String> {
  let Some(cur) = current else {
    // 本机没有该 key：已知类型先校验再采用，但 logbook / station-book 还要走与合并路径
    // 相同的归一化 —— 模块化备份允许「只恢复日志、不恢复台站档案」，原样采纳会把
    // 另一台机器上的 `station_id` / 旧格式形状带回本机，违反「宁可少标，不要错标」。
    return match key {
      "logbook" => merge_logbook_with_ids(None, backup).map(|(json, _)| json),
      "station-book" => adopt_migrated_station_book(backup),
      _ => known_value_is_valid(key, backup).then(|| backup.to_owned()),
    };
  };
  match key {
    "logbook" => merge_logbook(cur, backup),
    "bookmarks" | "dxcc_wanted_done" => merge_string_set(cur, backup),
    "exam-history" => merge_exam_history(cur, backup),
    "mistake-book" => merge_mistake_book(cur, backup),
    "study-stats" => merge_study_stats(cur, backup),
    "daily-challenge" => merge_daily_challenge(cur, backup),
    "rpg-stars" => merge_rpg_stars(cur, backup),
    "rpg-bestiary" => merge_rpg_bestiary(cur, backup),
    "station-book" => merge_station_book(cur, backup),
    _ => None,
  }
}

/// 已知类型的备份值是否能按其结构解析；未知 key 无结构可查，视为合法。
///
/// 本机有值时合并路径会走 [`adopt_backup`] 反序列化校验；本机没有该 key 时也必须同样严格，
/// 否则「本机空 + 备份损坏」会比「本机有 + 备份损坏」更宽松，损坏 JSON 原样写进 localStorage。
fn known_value_is_valid(key: &str, backup: &str) -> bool {
  fn parses<T: serde::de::DeserializeOwned>(raw: &str) -> bool {
    serde_json::from_str::<T>(raw).is_ok()
  }
  match key {
    "logbook" => parses::<Logbook>(backup),
    "bookmarks" | "dxcc_wanted_done" => parses::<Vec<String>>(backup),
    "exam-history" => parses::<Vec<ExamRecord>>(backup),
    "mistake-book" => parses::<MistakeBook>(backup),
    "study-stats" => parses::<StudyStats>(backup),
    "daily-challenge" => parses::<DailyResults>(backup),
    "rpg-stars" => parses::<StageStars>(backup),
    "rpg-bestiary" => parses::<Bestiary>(backup),
    "station-book" => parses::<StationBook>(backup),
    _ => true,
  }
}

/// 读本机值；解析不出来时采纳备份（见 [`adopt_backup`]）。
///
/// 返回 `Err(None)` 表示「本机坏了、备份也没法用」—— 两者都无法采纳时才保留本机。
fn local_or_backup<T: serde::de::DeserializeOwned>(
  cur: &str,
  backup: &str,
) -> Result<T, Option<String>> {
  match serde_json::from_str::<T>(cur) {
    Ok(v) => Ok(v),
    Err(_) => Err(adopt_backup::<T>(backup)),
  }
}

/// 本机值解析不出来时的兜底：整份采纳备份；备份自己也解析不出来时返回 `None`。
///
/// 合并本身就是「数据出问题」时的恢复路径，这时代码里的本机值损坏概率最高（手改过、
/// 写到一半的 JSON、旧版本写入的形状）。各个 `merge_*` 原来都用 `.ok()?` 直接返回
/// `None`，而 `None` 在调用方那里的语义是「保留本机、不覆盖」—— 备份里那一项被静默
/// 丢掉，用户看到的却还是「合并成功」。
fn adopt_backup<T: serde::de::DeserializeOwned>(backup: &str) -> Option<String> {
  serde_json::from_str::<T>(backup).ok()?;
  Some(backup.to_owned())
}

/// 合并日志：按 [`LogEntry::dedupe_key`] 去重，保留本机已有记录，新记录重新分配 ID。
fn merge_logbook(cur: &str, backup: &str) -> Option<String> {
  merge_logbook_with_ids(Some(cur), backup).map(|(json, _)| json)
}

/// 本机没有台站档案册时整份采纳备份：与 [`merge_station_book`] 同一口径先 `migrate()`，
/// 旧格式备份不能原样写回本机。
fn adopt_migrated_station_book(backup: &str) -> Option<String> {
  let mut book: StationBook = serde_json::from_str(backup).ok()?;
  book.migrate();
  serde_json::to_string(&book).ok()
}

/// 合并日志并返回备份 ID → 本机 ID 映射，供同一备份的 QSL 影像同步迁移。
/// 无法唯一对应的重复备份 ID 不进入映射，避免把扫描件关联到错误通联。
#[must_use]
pub fn merge_logbook_with_ids(
  current: Option<&str>,
  backup: &str,
) -> Option<(String, BTreeMap<u64, u64>)> {
  let mut backup: Logbook = serde_json::from_str(backup).ok()?;
  for entry in &mut backup.entries {
    entry.station_id = 0;
  }
  let Some(mut local) = current.and_then(|raw| serde_json::from_str::<Logbook>(raw).ok()) else {
    let mut ids = BTreeMap::new();
    let mut duplicates = BTreeSet::new();
    for entry in &backup.entries {
      if ids.insert(entry.id, entry.id).is_some() {
        duplicates.insert(entry.id);
      }
    }
    for id in duplicates {
      ids.remove(&id);
    }
    return Some((serde_json::to_string(&backup).ok()?, ids));
  };
  let mut seen: BTreeMap<String, u64> = local
    .entries
    .iter()
    .filter_map(|e| e.dedupe_key().map(|key| (key, e.id)))
    .collect();
  let mut next = local.entries.iter().map(|e| e.id).max().unwrap_or(0);
  let mut ids = BTreeMap::new();
  let mut duplicate_ids = BTreeSet::new();
  for mut entry in backup.entries {
    let original = entry.id;
    let key = entry.dedupe_key();
    let target = if let Some(id) = key.as_ref().and_then(|k| seen.get(k)).copied() {
      id
    } else {
      next = next.checked_add(1)?;
      entry.id = next;
      // 外部 station_id 在本机可能指向另一台站，未映射时按当前台站回落。
      entry.station_id = 0;
      if let Some(key) = key {
        seen.insert(key, next);
      }
      local.entries.push(entry);
      next
    };
    if ids.insert(original, target).is_some() {
      duplicate_ids.insert(original);
    }
  }
  for id in duplicate_ids {
    ids.remove(&id);
  }
  Some((serde_json::to_string(&local).ok()?, ids))
}

/// 覆盖日志时只有同 ID、同可靠通联身份且两边 ID 唯一的旧影像可以保留。
#[must_use]
pub fn retained_image_ids(current: Option<&str>, incoming: &str) -> BTreeSet<u64> {
  let Some(local) = current.and_then(|v| serde_json::from_str::<Logbook>(v).ok()) else {
    return BTreeSet::new();
  };
  let Ok(incoming) = serde_json::from_str::<Logbook>(incoming) else {
    return BTreeSet::new();
  };
  let unique = |book: &Logbook| {
    let mut entries = BTreeMap::new();
    let mut repeated = BTreeSet::new();
    for entry in &book.entries {
      if entries.insert(entry.id, entry.dedupe_key()).is_some() {
        repeated.insert(entry.id);
      }
    }
    for id in repeated {
      entries.remove(&id);
    }
    entries
  };
  let old = unique(&local);
  unique(&incoming)
    .into_iter()
    .filter_map(|(id, key)| {
      key
        .filter(|key| old.get(&id).and_then(Option::as_ref) == Some(key))
        .map(|_| id)
    })
    .collect()
}

/// 仅影像备份不新增日志：只匹配本机已有的、身份明确且唯一的通联。
#[must_use]
pub fn image_ids_for_existing_log(current: Option<&str>, backup: &str) -> BTreeMap<u64, u64> {
  let Some(local) = current.and_then(|v| serde_json::from_str::<Logbook>(v).ok()) else {
    return BTreeMap::new();
  };
  let Ok(backup) = serde_json::from_str::<Logbook>(backup) else {
    return BTreeMap::new();
  };
  let mut seen = BTreeMap::new();
  let mut ambiguous = BTreeSet::new();
  for entry in &local.entries {
    if let Some(key) = entry.dedupe_key()
      && seen.insert(key.clone(), entry.id).is_some()
    {
      ambiguous.insert(key);
    }
  }
  let mut ids = BTreeMap::new();
  let mut source_ids = BTreeSet::new();
  let mut repeated = BTreeSet::new();
  for entry in &backup.entries {
    if !source_ids.insert(entry.id) {
      repeated.insert(entry.id);
    }
    if let Some(key) = entry.dedupe_key()
      && !ambiguous.contains(&key)
      && let Some(&target) = seen.get(&key)
      && ids.insert(entry.id, target).is_some()
    {
      repeated.insert(entry.id);
    }
  }
  for id in repeated {
    ids.remove(&id);
  }
  ids
}

/// 合并台站档案册：按「呼号 + 网格」取并集，**保留本机的 id 与当前台站**。
///
/// 备份里那些通联的归属（`station_id`）指的是它在**另一台机器**上的 id，跨设备没有意义：
/// 硬按 id 对齐会把记录错配到别的台站上，所以合并进来的通联会回落到当前台站
/// （见 [`crate::station`] 的回落规则）—— 宁可少标一个归属，也不要错标。
fn merge_station_book(cur: &str, backup: &str) -> Option<String> {
  let mut merged = match local_or_backup::<StationBook>(cur, backup) {
    Ok(v) => v,
    Err(fallback) => return fallback,
  };
  let Ok(backup) = serde_json::from_str::<StationBook>(backup) else {
    return None;
  };
  merged.migrate();
  for profile in backup.profiles {
    // 呼号 + 网格就能认出同一个台站（与 `StationBook::claim` 同一把尺子）。
    if merged
      .profiles
      .iter()
      .any(|p| p.matches(&profile.callsign, &profile.gridsquare))
    {
      continue;
    }
    merged.upsert(StationProfile { id: 0, ..profile });
  }
  serde_json::to_string(&merged).ok()
}

/// 合并字符串集合（收藏、DXCC 稀有度完成清单）：保留本机顺序，备份里的新项按其顺序追加。
///
/// 此前用 `BTreeSet` 并集再排序，收藏列表的原有顺序会被永久打乱。
fn merge_string_set(cur: &str, backup: &str) -> Option<String> {
  let mut merged = match local_or_backup::<Vec<String>>(cur, backup) {
    Ok(v) => v,
    Err(fallback) => return fallback,
  };
  let Ok(backup) = serde_json::from_str::<Vec<String>>(backup) else {
    return None;
  };
  let mut seen: HashSet<String> = HashSet::with_capacity(merged.len());
  merged.retain(|item| seen.insert(item.clone()));
  for item in backup {
    if merged.len() >= MAX_SET_ITEMS {
      break;
    }
    if seen.insert(item.clone()) {
      merged.push(item);
    }
  }
  serde_json::to_string(&merged).ok()
}

/// 合并考试历史：按（题库、对题数、总题数、时间戳）去重，按时间升序，最多保留 [`EXAM_HISTORY_LIMIT`] 条。
fn merge_exam_history(cur: &str, backup: &str) -> Option<String> {
  let mut merged = match local_or_backup::<Vec<ExamRecord>>(cur, backup) {
    Ok(v) => v,
    Err(fallback) => return fallback,
  };
  let Ok(backup) = serde_json::from_str::<Vec<ExamRecord>>(backup) else {
    return None;
  };
  // 去重键把 `weak`（是否为薄弱项组卷）并入：同一题库、同一分数、同一时间戳的普通练习
  // 与薄弱项组卷是两次不同的考试，不能并成一条。
  let mut seen: HashSet<(String, usize, usize, i64, bool)> = merged
    .iter()
    .map(|r| (r.bank.clone(), r.correct, r.total, r.timestamp, r.weak))
    .collect();
  for r in backup {
    if seen.insert((r.bank.clone(), r.correct, r.total, r.timestamp, r.weak)) {
      merged.push(r);
    }
  }
  merged.sort_by_key(|r| r.timestamp);
  if merged.len() > EXAM_HISTORY_LIMIT {
    merged.drain(..merged.len() - EXAM_HISTORY_LIMIT);
  }
  serde_json::to_string(&merged).ok()
}

/// 合并错题本：按 [`MistakeRecord::key`] 去重，合并 `banks`，重复时保留错得更多的一条。
fn merge_mistake_book(cur: &str, backup: &str) -> Option<String> {
  let cur_book = match local_or_backup::<MistakeBook>(cur, backup) {
    Ok(v) => v,
    Err(fallback) => return fallback,
  };
  let Ok(backup_book) = serde_json::from_str::<MistakeBook>(backup) else {
    return None;
  };
  let mut by_key: BTreeMap<String, MistakeRecord> = cur_book
    .records
    .into_iter()
    .map(|r| (r.key.clone(), r))
    .collect();
  for mut r in backup_book.records {
    match by_key.get_mut(&r.key) {
      Some(existing) => {
        // banks 取并集（同一道题可能出现在多个题库）。
        existing.banks.extend(r.banks.iter().copied());
        // 保留错得更多的一条；替换前把并集后的 banks 赋回给 incoming，
        // 避免用 incoming 的局部 banks 覆盖掉本机已有的题库标记。
        if r.wrong_count > existing.wrong_count {
          r.banks = existing.banks.clone();
          *existing = r;
        }
      }
      None => {
        by_key.insert(r.key.clone(), r);
      }
    }
  }
  let mut records: Vec<MistakeRecord> = by_key.into_values().collect();
  records.sort_by_key(|r| std::cmp::Reverse(r.last_wrong_ms));
  records.truncate(MAX_RECORDS);
  serde_json::to_string(&MistakeBook { records }).ok()
}

/// 合并累计答题统计：分类计数累加、做过的题目集合取并集。
fn merge_study_stats(cur: &str, backup: &str) -> Option<String> {
  let mut merged = match local_or_backup::<StudyStats>(cur, backup) {
    Ok(v) => v,
    Err(fallback) => return fallback,
  };
  let Ok(backup) = serde_json::from_str::<StudyStats>(backup) else {
    return None;
  };
  merge_category(&mut merged.total, &backup.total);
  for (bank, bs) in backup.banks {
    let dst = merged.banks.entry(bank).or_default();
    merge_category(&mut dst.stats, &bs.stats);
    dst.seen.extend(bs.seen);
    cap_set(&mut dst.seen, MAX_SEEN_PER_BANK);
  }
  serde_json::to_string(&merged).ok()
}

/// 累计统计合并：两个计数都走饱和加法。
///
/// 备份是外部输入，`answered` 可以被文件拉到 `u32::MAX`：直接 `+=` 在 debug 下 panic、
/// 在 release（含浏览器用的 `wasm-release`）静默回绕，学习统计会变成垃圾，`correct`
/// 还可能大于 `answered`（正确率 > 100%）。饱和后再夹取，保证 `correct <= answered`。
fn merge_category(dst: &mut CategoryStats, src: &CategoryStats) {
  dst.answered = dst.answered.saturating_add(src.answered);
  dst.correct = dst.correct.saturating_add(src.correct).min(dst.answered);
  merge_tally_map(&mut dst.categories, &src.categories);
  merge_tally_map(&mut dst.subs, &src.subs);
}

fn merge_tally_map(dst: &mut BTreeMap<String, Tally>, src: &BTreeMap<String, Tally>) {
  for (k, v) in src {
    let d = dst.entry(k.clone()).or_default();
    d.answered = d.answered.saturating_add(v.answered);
    d.correct = d.correct.saturating_add(v.correct).min(d.answered);
  }
}

/// 合并每日挑战成绩：同一天保留得分更高的一次，最多保留 [`DAILY_KEEP_DAYS`] 天。
fn merge_daily_challenge(cur: &str, backup: &str) -> Option<String> {
  let mut merged = match local_or_backup::<DailyResults>(cur, backup) {
    Ok(v) => v,
    Err(fallback) => return fallback,
  };
  let Ok(backup) = serde_json::from_str::<DailyResults>(backup) else {
    return None;
  };
  for (date, r) in backup.days {
    match merged.days.get(&date) {
      // 同分时保留题量更大的一次（题量更大是更难的一档）；分数、题量都相同时保留本机。
      Some(existing) if (existing.correct, existing.total) >= (r.correct, r.total) => {}
      _ => {
        merged.days.insert(date, r);
      }
    }
  }
  while merged.days.len() > DAILY_KEEP_DAYS {
    merged.days.pop_first();
  }
  serde_json::to_string(&merged).ok()
}

/// 合并闯关星级：每一关取两边的历史最佳。
///
/// 星级只增不减，取最大值就是正确的并集 —— 不会因为导入一份旧备份而「掉星」。
fn merge_rpg_stars(cur: &str, backup: &str) -> Option<String> {
  let mut merged = match local_or_backup::<StageStars>(cur, backup) {
    Ok(v) => v,
    Err(fallback) => return fallback,
  };
  let Ok(backup) = serde_json::from_str::<StageStars>(backup) else {
    return None;
  };
  for (stage, stars) in backup.best {
    merged.record(&stage, stars);
  }
  serde_json::to_string(&merged).ok()
}

/// 合并怪物图鉴：已击败集合取并集。
fn merge_rpg_bestiary(cur: &str, backup: &str) -> Option<String> {
  let mut merged = match local_or_backup::<Bestiary>(cur, backup) {
    Ok(v) => v,
    Err(fallback) => return fallback,
  };
  let Ok(backup) = serde_json::from_str::<Bestiary>(backup) else {
    return None;
  };
  merged.defeated.extend(backup.defeated);
  cap_set(&mut merged.defeated, MAX_SET_ITEMS);
  serde_json::to_string(&merged).ok()
}

/// 备份的模块归类：导出时按模块勾选（QSL 影像这类大数据单独分组，
/// 其余本地数据全量兜底）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupModule {
  /// 通联日志、台站档案、QSL 影像索引等通联域数据。
  Log,
  /// QSL 影像本体（IndexedDB 里的 `qsl-image:<id>`，数据量大）。
  QslImage,
  /// 其余所有本地数据（设置、练习进度、考试历史……）。
  Other,
}

/// 把存储 key 归入备份模块。
///
/// 边界情况（如实记录）：`log-stats-scope` 是日志页的统计范围偏好，归「其他」还是
/// 「通联」都说得通 —— 它跟日志数据强绑定，恢复日志时必须一起恢复才一致，
/// 因此归「通联」。
#[must_use]
pub fn classify_key(key: &str) -> BackupModule {
  if key.starts_with(qsl_image::KEY_PREFIX) {
    return BackupModule::QslImage;
  }
  match key {
    "logbook" | "station-info" | "station-book" | qsl_image::INDEX_KEY | "log-stats-scope" => {
      BackupModule::Log
    }
    _ => BackupModule::Other,
  }
}

#[cfg(test)]
mod classify_tests {
  use super::*;

  #[test]
  fn classifies_backup_keys_by_module() {
    assert_eq!(classify_key("logbook"), BackupModule::Log);
    assert_eq!(classify_key("station-book"), BackupModule::Log);
    assert_eq!(classify_key("qsl-image-index"), BackupModule::Log);
    assert_eq!(classify_key("log-stats-scope"), BackupModule::Log);
    assert_eq!(classify_key("qsl-image:7"), BackupModule::QslImage);
    assert_eq!(classify_key("mistake-book"), BackupModule::Other);
    assert_eq!(classify_key("theme"), BackupModule::Other);
    assert_eq!(classify_key("practice:2024"), BackupModule::Other);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::bank::Bank;

  #[test]
  fn backup_limit_matches_text_import_at_the_exact_byte_boundary() {
    assert!(backup_size_is_supported(0));
    assert!(backup_size_is_supported(MAX_BACKUP_BYTES));
    assert!(!backup_size_is_supported(MAX_BACKUP_BYTES + 1));
    assert!(!backup_size_is_supported(usize::MAX));
  }

  #[test]
  fn covering_logs_only_retains_images_of_the_same_unique_qso() {
    let a = LogEntry {
      id: 1,
      callsign: "JA1AAA".into(),
      date: "2026-01-01".into(),
      time: "12:00".into(),
      ..Default::default()
    };
    let local = serde_json::to_string(&Logbook {
      entries: vec![a.clone()],
    })
    .unwrap();
    assert_eq!(
      retained_image_ids(Some(&local), &local),
      BTreeSet::from([1])
    );
    let other = serde_json::to_string(&Logbook {
      entries: vec![LogEntry {
        callsign: "W1AW".into(),
        ..a.clone()
      }],
    })
    .unwrap();
    assert!(retained_image_ids(Some(&local), &other).is_empty());
    let ambiguous = serde_json::to_string(&Logbook {
      entries: vec![a.clone(), a],
    })
    .unwrap();
    assert!(retained_image_ids(Some(&local), &ambiguous).is_empty());
    assert!(retained_image_ids(Some("broken"), &local).is_empty());
    let values = BTreeMap::from([("key".to_owned(), "中文\"".to_owned())]);
    assert_eq!(backup_payload_bytes(&values), 10);
    assert!(backup_payload_bytes(&values) < serde_json::to_string(&values).unwrap().len());
  }

  #[test]
  fn log_id_mapping_preserves_duplicates_and_remaps_new_records() {
    let original = LogEntry {
      id: 1,
      date: "2026-01-01".into(),
      time: "12:00".into(),
      callsign: "JA1AAA".into(),
      ..Default::default()
    };
    let local = serde_json::to_string(&Logbook {
      entries: vec![original.clone()],
    })
    .unwrap();
    let backup = serde_json::to_string(&Logbook {
      entries: vec![
        LogEntry { id: 9, ..original },
        LogEntry {
          id: 1,
          station_id: 2,
          callsign: "W1AW".into(),
          date: "2026-01-02".into(),
          time: "13:00".into(),
          ..Default::default()
        },
      ],
    })
    .unwrap();
    let (merged, ids) = merge_logbook_with_ids(Some(&local), &backup).unwrap();
    assert_eq!(ids.get(&9), Some(&1));
    assert_eq!(ids.get(&1), Some(&2));
    let book: Logbook = serde_json::from_str(&merged).unwrap();
    assert_eq!(book.entries[1].station_id, 0);
    let existing = image_ids_for_existing_log(Some(&local), &backup);
    assert_eq!(existing.get(&9), Some(&1));
    assert!(!existing.contains_key(&1));
    let (adopted_json, adopted) = merge_logbook_with_ids(None, &backup).unwrap();
    assert_eq!(adopted.get(&9), Some(&9));
    let adopted_book: Logbook = serde_json::from_str(&adopted_json).unwrap();
    assert!(
      adopted_book
        .entries
        .iter()
        .all(|entry| entry.station_id == 0)
    );
    let duplicate_source = serde_json::to_string(&Logbook {
      entries: vec![
        LogEntry {
          id: 9,
          ..book.entries[0].clone()
        },
        LogEntry {
          id: 9,
          ..book.entries[1].clone()
        },
      ],
    })
    .unwrap();
    assert!(image_ids_for_existing_log(Some(&local), &duplicate_source).is_empty());
  }

  #[test]
  fn ambiguous_backup_ids_do_not_map_images() {
    let entry = LogEntry {
      id: 7,
      ..Default::default()
    };
    let backup = serde_json::to_string(&Logbook {
      entries: vec![entry.clone(), entry],
    })
    .unwrap();
    assert!(merge_logbook_with_ids(None, &backup).unwrap().1.is_empty());
    assert!(
      merge_logbook_with_ids(Some(r#"{"entries":[]}"#), &backup)
        .unwrap()
        .1
        .is_empty()
    );
  }

  #[test]
  fn a_corrupt_local_value_is_replaced_by_the_backup() {
    // 合并本身就是「数据出问题」时的恢复路径，这时代码里的本机值损坏概率最高。
    // 原来各 `merge_*` 都 `.ok()?` 返回 `None`，而 `None` 的语义是「保留本机」——
    // 备份里那一项被静默丢掉，用户看到的却还是「合并成功」。
    let backup = serde_json::to_string(&Logbook {
      entries: vec![LogEntry {
        id: 1,
        callsign: "JA1AAA".into(),
        date: "2026-10-01".into(),
        time: "12:00".into(),
        ..Default::default()
      }],
    })
    .expect("序列化备份");

    let merged =
      merge_value("logbook", Some("{ 写到一半的 JSON"), &backup).expect("本机损坏时要采纳备份");
    let book: Logbook = serde_json::from_str(&merged).expect("反序列化");
    assert_eq!(book.entries.len(), 1);
    assert_eq!(book.entries[0].callsign, "JA1AAA");

    // 备份自己也解析不出来时不乱写：保留本机。
    assert!(merge_value("logbook", Some("{ 坏的"), "{ 也坏的").is_none());
  }

  #[test]
  fn merging_a_logbook_never_swallows_records_without_a_date() {
    // 日期缺失时 `qso_key` 会退化成「呼号|波段|模式」，按它去重会把整批记录并成一条。
    let local = Logbook {
      entries: vec![LogEntry {
        id: 1,
        callsign: "JA1AAA".into(),
        mode: "FT8".into(),
        ..Default::default()
      }],
    };
    let backup = Logbook {
      entries: vec![
        LogEntry {
          id: 9,
          callsign: "JA1AAA".into(),
          mode: "FT8".into(),
          ..Default::default()
        },
        LogEntry {
          id: 9,
          callsign: "JA1AAA".into(),
          mode: "FT8".into(),
          ..Default::default()
        },
      ],
    };
    let merged = merge_value(
      "logbook",
      Some(&serde_json::to_string(&local).expect("序列化本机")),
      &serde_json::to_string(&backup).expect("序列化备份"),
    )
    .expect("可合并");
    let book: Logbook = serde_json::from_str(&merged).expect("反序列化");
    assert_eq!(book.entries.len(), 3, "没有日期就谈不上重复，一律并入");
  }

  #[test]
  fn station_book_union_keeps_local_ids_and_active() {
    let local = StationBook::from_info(&crate::logbook::StationInfo {
      callsign: "BG4XXX".into(),
      gridsquare: "OM89EW".into(),
      ..Default::default()
    });
    let home = local.active_id;
    let mut backup = StationBook::from_info(&crate::logbook::StationInfo {
      callsign: "BG4XXX".into(),
      gridsquare: "OM89EW".into(),
      ..Default::default()
    });
    // 备份里多一台野外台站（同一台机器的 id 与本地不同）。
    let field = backup.upsert(StationProfile {
      label: "POTA".into(),
      callsign: "BG4XXX".into(),
      gridsquare: "OL99AA".into(),
      ..Default::default()
    });
    backup.set_active(field);

    let merged = merge_value(
      "station-book",
      Some(&serde_json::to_string(&local).unwrap()),
      &serde_json::to_string(&backup).unwrap(),
    )
    .expect("台站档案册应当可合并");
    let book: StationBook = serde_json::from_str(&merged).expect("反序列化");

    assert_eq!(book.profiles.len(), 2, "家里那条去重、野外那条新增");
    assert_eq!(book.active_id, home, "当前台站仍取本机");
    assert_eq!(book.by_id(home).expect("本机那条").gridsquare, "OM89EW");
    assert!(
      book.profiles.iter().any(|p| p.gridsquare == "OL99AA"),
      "备份里的野外台站要带过来"
    );
    // id 全部唯一且本机的不变。
    let mut ids: Vec<u64> = book.profiles.iter().map(|p| p.id).collect();
    ids.sort_unstable();
    let n = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), n);
  }

  #[test]
  fn missing_local_adopts_backup() {
    assert_eq!(
      merge_value("theme", None, "light").as_deref(),
      Some("light")
    );
  }

  #[test]
  fn unknown_keys_keep_local() {
    assert_eq!(merge_value("theme", Some("dark"), "light"), None);
  }

  #[test]
  fn merges_logbook_dedup_by_qso() {
    let cur = r#"{"entries":[{"id":1,"date":"2026-01-01","time":"12:00","freq":"14.074","mode":"FT8","callsign":"JA1X","remark":""}]}"#;
    let backup = r#"{"entries":[{"id":9,"date":"2026-01-01","time":"12:00","freq":"14.074","mode":"FT8","callsign":"ja1x","remark":""},{"id":10,"date":"2026-01-02","time":"12:00","freq":"7.010","mode":"CW","callsign":"W1AW","remark":""}]}"#;
    let merged = merge_value("logbook", Some(cur), backup).expect("merge");
    let lb: Logbook = serde_json::from_str(&merged).expect("parse");
    assert_eq!(lb.entries.len(), 2);
    assert_eq!(lb.entries[1].callsign, "W1AW");
    assert_eq!(lb.entries[1].id, 2);
  }

  #[test]
  fn absent_local_key_still_validates_known_types() {
    // R2-11：本机没有该 key 时，半截 / 篡改的 JSON 不得原样写入。
    for key in [
      "logbook",
      "bookmarks",
      "exam-history",
      "mistake-book",
      "study-stats",
      "daily-challenge",
      "rpg-stars",
      "rpg-bestiary",
      "station-book",
    ] {
      assert_eq!(merge_value(key, None, "{\"entries\": [tru"), None, "{key}");
      assert_eq!(merge_value(key, None, "\"just a string\""), None, "{key}");
    }
    // 合法值与未知 key（纯字符串设置）照旧采用。
    assert_eq!(
      merge_value("bookmarks", None, r#"["a"]"#).as_deref(),
      Some(r#"["a"]"#)
    );
    assert_eq!(merge_value("theme", None, "dark").as_deref(), Some("dark"));
  }

  #[test]
  fn merged_sets_are_capped() {
    // R2-12：并集不能无限增长。
    let many: Vec<String> = (0..MAX_SEEN_PER_BANK + 500)
      .map(|i| format!("q{i:06}"))
      .collect();
    let backup = format!(
      r#"{{"banks":{{"A":{{"seen":{}}}}}}}"#,
      serde_json::to_string(&many).expect("json")
    );
    let merged = merge_value("study-stats", Some("{}"), &backup).expect("merge");
    let stats: StudyStats = serde_json::from_str(&merged).expect("parse");
    assert_eq!(stats.banks[&Bank::A].seen.len(), MAX_SEEN_PER_BANK);
  }

  #[test]
  fn string_set_merge_keeps_local_order() {
    let merged = merge_value("bookmarks", Some(r#"["z","a"]"#), r#"["m","a","b"]"#).expect("merge");
    let v: Vec<String> = serde_json::from_str(&merged).expect("parse");
    assert_eq!(v, ["z", "a", "m", "b"]);
  }

  #[test]
  fn merges_string_sets() {
    let merged = merge_value("bookmarks", Some(r#"["a","b"]"#), r#"["b","c"]"#).expect("merge");
    let v: Vec<String> = serde_json::from_str(&merged).expect("parse");
    assert_eq!(v, ["a", "b", "c"]);
  }

  #[test]
  fn merges_exam_history_dedup_and_caps() {
    let cur = r#"[{"bank":"A","correct":30,"total":40,"timestamp":1}]"#;
    let backup = r#"[{"bank":"A","correct":30,"total":40,"timestamp":1},{"bank":"A","correct":35,"total":40,"timestamp":2}]"#;
    let merged = merge_value("exam-history", Some(cur), backup).expect("merge");
    let h: Vec<ExamRecord> = serde_json::from_str(&merged).expect("parse");
    assert_eq!(h.len(), 2);
    assert_eq!(h[1].correct, 35);
  }

  #[test]
  fn merges_mistake_book_keeping_harder_record_and_banks() {
    let cur = r#"{"records":[{"key":"k1","question":{"question":"q","options":[],"answer_keys":["A"],"type":"single"},"my_answer":["B"],"wrong_count":1,"streak":0,"last_wrong_ms":1,"due_ms":1,"banks":["A"]}]}"#;
    let backup = r#"{"records":[{"key":"k1","question":{"question":"q","options":[],"answer_keys":["A"],"type":"single"},"my_answer":["B"],"wrong_count":3,"streak":0,"last_wrong_ms":5,"due_ms":5,"banks":["B"]},{"key":"k2","question":{"question":"q2","options":[],"answer_keys":["A"],"type":"single"},"my_answer":["B"],"wrong_count":1,"streak":0,"last_wrong_ms":3,"due_ms":3,"banks":["A"]}]}"#;
    let merged = merge_value("mistake-book", Some(cur), backup).expect("merge");
    let book: MistakeBook = serde_json::from_str(&merged).expect("parse");
    assert_eq!(book.records.len(), 2);
    let k1 = book.records.iter().find(|r| r.key == "k1").expect("k1");
    assert_eq!(k1.wrong_count, 3);
    assert!(k1.banks.contains(&Bank::A) && k1.banks.contains(&Bank::B));
  }

  #[test]
  fn merges_study_stats_by_accumulation() {
    let cur = r#"{"categories":{"a":{"answered":3,"correct":2}},"answered":3,"correct":2,"banks":{"A":{"categories":{"a":{"answered":3,"correct":2}},"answered":3,"correct":2,"seen":["k1"]}}}"#;
    let backup = r#"{"categories":{"a":{"answered":1,"correct":1},"b":{"answered":5,"correct":5}},"answered":6,"correct":6,"banks":{"A":{"categories":{"a":{"answered":1,"correct":0}},"answered":1,"correct":0,"seen":["k2"]},"B":{"categories":{},"answered":2,"correct":2,"seen":[]}}}"#;
    let merged = merge_value("study-stats", Some(cur), backup).expect("merge");
    let s: StudyStats = serde_json::from_str(&merged).expect("parse");
    assert_eq!((s.total.answered, s.total.correct), (9, 8));
    assert_eq!(s.total.categories["a"].answered, 4);
    assert_eq!(s.total.categories["b"].answered, 5);
    let a = s.bank(Bank::A).expect("bank A");
    assert_eq!(a.stats.categories["a"].answered, 4);
    assert!(a.seen.contains("k1") && a.seen.contains("k2"));
    assert_eq!(s.bank(Bank::B).expect("bank B").stats.answered, 2);
  }

  #[test]
  fn huge_backup_stats_saturate_instead_of_wrapping() {
    // 备份是外部输入：`answered` / `correct` 拉满时不能让 u32 溢回（debug 下 panic、
    // release 下静默回绕），也不能产出 correct > answered 的「正确率 > 100%」。
    let cur =
      r#"{"answered":1,"correct":1,"categories":{"a":{"answered":1,"correct":1}},"banks":{}}"#;
    let backup = r#"{"answered":4294967295,"correct":4294967295,"categories":{"a":{"answered":4294967295,"correct":4294967295}},"banks":{"A":{"answered":4294967295,"correct":4294967295,"seen":[]}}}"#;
    let merged = merge_value("study-stats", Some(cur), backup).expect("merge");
    let s: StudyStats = serde_json::from_str(&merged).expect("parse");
    assert_eq!(s.total.answered, u32::MAX, "饱和而不是回绕");
    assert!(s.total.correct <= s.total.answered, "正确率不能超过 100%");
    assert!(s.total.categories["a"].correct <= s.total.categories["a"].answered);
    let a = s.bank(Bank::A).expect("bank A");
    assert_eq!(a.stats.answered, u32::MAX);
    assert!(a.stats.correct <= a.stats.answered);
  }

  #[test]
  fn merges_daily_challenge_keeping_better() {
    let cur = r#"{"days":{"2026-10-01":{"correct":8,"total":10}}}"#;
    let backup =
      r#"{"days":{"2026-10-01":{"correct":5,"total":10},"2026-10-02":{"correct":9,"total":10}}}"#;
    let merged = merge_value("daily-challenge", Some(cur), backup).expect("merge");
    let r: DailyResults = serde_json::from_str(&merged).expect("parse");
    assert_eq!(r.days["2026-10-01"].correct, 8);
    assert_eq!(r.days["2026-10-02"].correct, 9);
  }

  #[test]
  fn merges_rpg_stars_by_best_per_stage() {
    let local = r#"{"best":{"A:a":3,"A:b":1}}"#;
    let backup = r#"{"best":{"A:a":1,"A:b":2,"A:c":3}}"#;
    let merged = merge_value("rpg-stars", Some(local), backup).unwrap();
    let stars: StageStars = serde_json::from_str(&merged).unwrap();
    assert_eq!(stars.stars("A:a"), 3, "旧备份不能让本机掉星");
    assert_eq!(stars.stars("A:b"), 2);
    assert_eq!(stars.stars("A:c"), 3);
  }

  #[test]
  fn merges_rpg_bestiary_as_union() {
    let merged = merge_value(
      "rpg-bestiary",
      Some(r#"{"defeated":["A-1","A-2"]}"#),
      r#"{"defeated":["A-2","A-3"]}"#,
    )
    .unwrap();
    let dex: Bestiary = serde_json::from_str(&merged).unwrap();
    assert_eq!(dex.count(), 3);
  }

  #[test]
  fn rpg_state_adopts_backup_when_local_is_missing_or_broken() {
    let backup = r#"{"best":{"A:a":2}}"#;
    assert_eq!(
      merge_value("rpg-stars", None, backup).as_deref(),
      Some(backup)
    );
    let merged = merge_value("rpg-stars", Some("not json"), backup).unwrap();
    assert_eq!(
      serde_json::from_str::<StageStars>(&merged)
        .unwrap()
        .stars("A:a"),
      2
    );
  }
}
