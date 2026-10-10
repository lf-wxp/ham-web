//! 备份合并导入：把备份 JSON 中的各 key 与当前本地数据做类型感知合并。
//!
//! 跨设备多份数据应取「并集 / 累加 / 保留更优」，避免整体覆盖导致丢失任一设备的记录。
//! 这里只对核心数据类型做合并；其余 key（主题、各类设置、练习 / 考试进度等）由调用方按
//! 「本机优先」处理（返回 `None` 表示不覆盖）。

use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::daily_challenge::DailyResults;
use crate::exam_history::ExamRecord;
use crate::logbook::{LogEntry, Logbook};
use crate::mistake_book::{
  CategoryStats, MAX_RECORDS, MistakeBook, MistakeRecord, StudyStats, Tally,
};
use crate::rpg::{Bestiary, StageStars};
use crate::station::{StationBook, StationProfile};

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
/// * 本机没有该 key → `Some(backup)`（直接采用备份值）；
/// * 已知类型 → `Some(合并后的 JSON)`（写回）；
/// * 其余 key → `None`（保留本机，不覆盖）。
#[must_use]
pub fn merge_value(key: &str, current: Option<&str>, backup: &str) -> Option<String> {
  let Some(cur) = current else {
    return Some(backup.to_owned());
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
  let mut merged = match local_or_backup::<Logbook>(cur, backup) {
    Ok(v) => v,
    Err(fallback) => return fallback,
  };
  let Ok(backup) = serde_json::from_str::<Logbook>(backup) else {
    return None;
  };
  let mut seen: HashSet<String> = merged
    .entries
    .iter()
    .filter_map(LogEntry::dedupe_key)
    .collect();
  let mut id = merged.next_id();
  for mut entry in backup.entries {
    // 没有 `dedupe_key`（日期或时间缺失）就说不上重不重复，一律并入 —— 见
    // `LogEntry::dedupe_key`：把整批同呼号同波段的记录当成一条吞掉，比多几条记录糟糕。
    if entry.dedupe_key().is_none_or(|key| seen.insert(key)) {
      entry.id = id;
      id += 1;
      merged.entries.push(entry);
    }
  }
  serde_json::to_string(&merged).ok()
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

/// 合并字符串集合（收藏、DXCC 稀有度完成清单）：并集后排序。
fn merge_string_set(cur: &str, backup: &str) -> Option<String> {
  let a = match local_or_backup::<Vec<String>>(cur, backup) {
    Ok(v) => v,
    Err(fallback) => return fallback,
  };
  let Ok(b) = serde_json::from_str::<Vec<String>>(backup) else {
    return None;
  };
  let set: BTreeSet<String> = a.into_iter().chain(b).collect();
  serde_json::to_string(&set.into_iter().collect::<Vec<_>>()).ok()
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
  let mut seen: HashSet<(String, usize, usize, i64)> = merged
    .iter()
    .map(|r| (r.bank.clone(), r.correct, r.total, r.timestamp))
    .collect();
  for r in backup {
    if seen.insert((r.bank.clone(), r.correct, r.total, r.timestamp)) {
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
  }
  serde_json::to_string(&merged).ok()
}

fn merge_category(dst: &mut CategoryStats, src: &CategoryStats) {
  dst.answered += src.answered;
  dst.correct += src.correct;
  merge_tally_map(&mut dst.categories, &src.categories);
  merge_tally_map(&mut dst.subs, &src.subs);
}

fn merge_tally_map(dst: &mut BTreeMap<String, Tally>, src: &BTreeMap<String, Tally>) {
  for (k, v) in src {
    let d = dst.entry(k.clone()).or_default();
    d.answered += v.answered;
    d.correct += v.correct;
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
      Some(existing) if existing.correct >= r.correct => {}
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
  serde_json::to_string(&merged).ok()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::bank::Bank;

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
