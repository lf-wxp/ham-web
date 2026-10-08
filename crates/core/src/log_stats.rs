//! 通联日志的可视化统计：UTC 时段 × 波段的通联密度热力图，以及**统计口径**
//! （按台站 / 操作员分面，见 [`StatsScope`]）。

use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::logbook::{LogEntry, split_hhmm};
use crate::station::StationBook;
use crate::study_plan::{day_number, format_day};

/// 日志波段的展示顺序（低频到高频）。
pub const BAND_ORDER: &[&str] = &[
  "160m", "80m", "60m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m", "2m", "70cm",
];

/// 统计每个波段在 24 个 UTC 小时内的通联数量。
///
/// 返回按 [`BAND_ORDER`] 排序的 `(波段, [24 小时计数])` 列表，未在排序表内出现的波段
/// （如「其他」）按字母序排在其后。忽略时间或波段不合法的记录。
#[must_use]
pub fn hour_band_heatmap(entries: &[LogEntry]) -> Vec<(String, [u32; 24])> {
  let mut map: BTreeMap<String, [u32; 24]> = BTreeMap::new();
  for e in entries {
    let band = e.band_label();
    if band.is_empty() {
      continue;
    }
    // 与体检共用同一个 `HH:MM` 解析：各写一份会容忍不同的写法，于是「体检查不出问题、
    // 热力图却少一条」（原来这里用 `get(..2)`，`8:30` 的单数字小时会被静默丢掉）。
    let Some((hour, _)) = split_hhmm(&e.time) else {
      continue;
    };
    map.entry(band).or_insert([0; 24])[hour as usize] += 1;
  }

  let mut out: Vec<(String, [u32; 24])> = BAND_ORDER
    .iter()
    .filter_map(|b| map.remove(*b).map(|v| ((*b).to_owned(), v)))
    .collect();
  out.extend(map);
  out
}

/// 每个波段的通联条数，key 为 [`LogEntry::band_label`]。
///
/// 波段推不出来的记录归到空串（调用方按需过滤）—— 比赛页的「本波段 N 条」要拿它查表。
#[must_use]
pub fn band_counts(entries: &[LogEntry]) -> BTreeMap<String, usize> {
  let mut map: BTreeMap<String, usize> = BTreeMap::new();
  for e in entries {
    *map.entry(e.band_label()).or_default() += 1;
  }
  map
}

/// 日志里出现过的波段与模式（各自去重、升序），供筛选下拉用。
///
/// 波段忽略空值（筛选项里出现一个空选项没有意义），模式不忽略 —— 与原来界面上那版
/// 保持一致，免得下拉里凭空少一项。
#[must_use]
pub fn distinct_bands_and_modes(entries: &[LogEntry]) -> (Vec<String>, Vec<String>) {
  let bands: BTreeSet<String> = entries
    .iter()
    .map(LogEntry::band_label)
    .filter(|b| !b.is_empty())
    .collect();
  let modes: BTreeSet<String> = entries.iter().map(|e| e.mode.clone()).collect();
  (bands.into_iter().collect(), modes.into_iter().collect())
}

/// 日志统计概览（日志页顶部那几块数字与两个分布条）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogOverview {
  /// 参与统计的通联条数。
  pub total: usize,
  /// 不同呼号数。
  pub callsigns: usize,
  /// DXCC 实体名（升序去重）。
  pub dxcc: Vec<String>,
  /// 对方网格（升序去重）。
  pub grids: Vec<String>,
  /// 模式分布，按条数降序（条数相同时按字母序，保证结果稳定）。
  pub modes: Vec<(String, usize)>,
  /// 波段分布，按条数降序（同上）。
  pub bands: Vec<(String, usize)>,
  /// 已收到纸质确认的条数。
  pub confirmed: usize,
}

/// 汇总一批通联的概览。
///
/// 去重一律用集合而不是 `Vec::contains`：原来那版对每条记录都线性扫一遍已有的 DXCC 与
/// 网格清单，几千条日志就是 O(n²)（每次日志变动都要重算一遍）。分布表用 `BTreeMap`
/// 还顺带让「条数相同」的两项按字母序排 —— `HashMap` 的迭代顺序随进程随机，界面上
/// 两个条目的先后会莫名其妙地变。
#[must_use]
pub fn overview(entries: &[LogEntry]) -> LogOverview {
  let mut by_mode: BTreeMap<String, usize> = BTreeMap::new();
  let mut dxcc: BTreeSet<&'static str> = BTreeSet::new();
  let mut grids: BTreeSet<String> = BTreeSet::new();
  let mut calls: HashSet<String> = HashSet::new();
  let mut confirmed = 0;
  for e in entries {
    *by_mode.entry(e.mode.clone()).or_default() += 1;
    if let Some(entity) = e.entity() {
      dxcc.insert(entity.name);
    }
    if !e.gridsquare.is_empty() {
      grids.insert(e.gridsquare.clone());
    }
    // 与 `qso_key` 同一口径（trim + 大写）：不 trim 的话「 BG4XXX」与「BG4XXX」会被
    // 算成两个呼号，「不同呼号数」偏大。
    calls.insert(e.callsign.trim().to_uppercase());
    confirmed += usize::from(e.qsl_rcvd);
  }
  // 波段分布复用 `band_counts`：推不出波段的记录不进分布表。
  let bands = band_counts(entries)
    .into_iter()
    .filter(|(band, _)| !band.is_empty())
    .collect();
  LogOverview {
    total: entries.len(),
    callsigns: calls.len(),
    dxcc: dxcc.into_iter().map(str::to_owned).collect(),
    grids: grids.into_iter().collect(),
    modes: sorted_by_count(by_mode),
    bands: sorted_by_count(bands),
    confirmed,
  }
}

/// 分布表：按条数降序，条数相同时按名字升序（保证同一份日志每次渲染的次序一致）。
fn sorted_by_count(map: BTreeMap<String, usize>) -> Vec<(String, usize)> {
  let mut v: Vec<(String, usize)> = map.into_iter().collect();
  v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
  v
}

/// 按日期统计 QSO 数量，key 为 `YYYY-MM-DD`（忽略日期不合法或缺失的记录）。
///
/// 返回按日期升序的映射，供「日历热力图」按天着色。
#[must_use]
pub fn daily_qso_counts(entries: &[LogEntry]) -> BTreeMap<String, u32> {
  let mut map: BTreeMap<String, u32> = BTreeMap::new();
  for e in entries {
    // 按解析结果归一化，而不是截前 10 个字符：`get(..10)` 返回的切片长度恒为 10
    // （那个 `filter` 是恒真的），`abcdefghij` 也会被当成一天；`2026-1-5` 与
    // `2026-01-05` 还会被当成两天，热力图上就多出一格。
    let Some(days) = day_number(&e.date) else {
      continue;
    };
    *map.entry(format_day(days)).or_default() += 1;
  }
  map
}

/// 统计口径：只看某个台站 / 某个操作员，或全都看。
///
/// 「分账」在业余无线电里是实打实的需求：俱乐部台的日志里既有若干操作员，也可能有若干个
/// 台站（固定台 + 野外），奖状进度按谁 / 按哪个台站算得分开看 —— 否则「我们俱乐部快够
/// DXCC 100 了」这句话没人说得清是谁的 100。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatsScope {
  /// 只看这个台站档案（`None` = 全部）。
  pub station: Option<u64>,
  /// 只看这个操作员（空 = 全部；比较时不区分大小写）。
  pub operator: String,
}

impl StatsScope {
  /// 全都看。
  #[must_use]
  pub fn all() -> Self {
    Self::default()
  }

  /// 只看某个台站。
  #[must_use]
  pub fn of_station(id: u64) -> Self {
    Self {
      station: Some(id),
      operator: String::new(),
    }
  }

  /// 只看某个操作员。
  #[must_use]
  pub fn of_operator(op: &str) -> Self {
    Self {
      station: None,
      operator: op.trim().to_ascii_uppercase(),
    }
  }

  /// 是不是「全都看」。
  #[must_use]
  pub fn is_all(&self) -> bool {
    self.station.is_none() && self.operator.trim().is_empty()
  }

  /// 稳定短串（界面持久化用）：`""` / `s:3` / `o:WXP` / `s:3|o:WXP`。
  #[must_use]
  pub fn key(&self) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(id) = self.station {
      parts.push(format!("s:{id}"));
    }
    if !self.operator.trim().is_empty() {
      parts.push(format!("o:{}", self.operator.trim().to_ascii_uppercase()));
    }
    parts.join("|")
  }

  /// 由短串还原；认不出的片段忽略（宁可变回「全都看」，也不要让界面崩）。
  ///
  /// 切法不是「按 `|` 逐段解析」：操作员名是自由文本，可能自带 `|`，逐段切会把
  /// `o:A|B` 的后半截当成「认不出的片段」丢掉 —— 往返就不一致了。`s:` 段是纯数字、
  /// 不含 `|`，所以「第一段是台站、其余整段都是操作员」没有歧义，且与旧值的写法兼容。
  #[must_use]
  pub fn from_key(key: &str) -> Self {
    let mut scope = Self::all();
    // 第一段是 `s:` 开头才当台站段；否则整串都按操作员段看（`o:…`、旧值、手写的串）。
    let (station, operator) = match key.split_once('|') {
      Some((head, tail)) if head.starts_with("s:") => (Some(head), tail),
      // 没有 `|` 时整串只有一个片段：`s:3` 是台站段，其余按操作员段看。
      None if key.starts_with("s:") => (Some(key), ""),
      _ => (None, key),
    };
    if let Some(id) = station
      .and_then(|s| s.strip_prefix("s:"))
      .and_then(|s| s.trim().parse::<u64>().ok())
    {
      scope.station = Some(id);
    }
    if let Some(op) = operator.strip_prefix("o:") {
      scope.operator = op.trim().to_ascii_uppercase();
    }
    scope
  }

  /// 这条通联是否落在口径内。
  #[must_use]
  pub fn matches(&self, e: &LogEntry, book: &StationBook) -> bool {
    if let Some(id) = self.station
      && book.of_entry(e).id != id
    {
      return false;
    }
    let want = self.operator.trim();
    if !want.is_empty() && operator_of(e, book) != want.to_ascii_uppercase() {
      return false;
    }
    true
  }
}

/// 一条通联的操作员：记录里填了 `OPERATOR` 就用它，否则回落到所属台站档案的操作员。
///
/// 回落到台站档案而不是「空」：老日志与本工具自己记的记录都不逐条写操作员，
/// 它们显然属于那个台站的操作员 —— 按空字符串分面会把它们全丢进一个没有名字的桶里。
#[must_use]
pub fn operator_of(e: &LogEntry, book: &StationBook) -> String {
  let own = e.operator.trim();
  if !own.is_empty() {
    return own.to_ascii_uppercase();
  }
  book.of_entry(e).operator.trim().to_ascii_uppercase()
}

/// 日志里出现过的操作员（去重；按条数降序，同数按字母序）。
#[must_use]
pub fn operators(entries: &[LogEntry], book: &StationBook) -> Vec<String> {
  let mut counts: BTreeMap<String, usize> = BTreeMap::new();
  for e in entries {
    let op = operator_of(e, book);
    if !op.is_empty() {
      *counts.entry(op).or_default() += 1;
    }
  }
  let mut out: Vec<(String, usize)> = counts.into_iter().collect();
  out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
  out.into_iter().map(|(op, _)| op).collect()
}

/// 按口径筛出记录（**克隆**一份子集）。
///
/// 上层那些统计（`AwardProgress::from_entries` / 网格地图 / 模式分布…）都吃 `&[LogEntry]`，
/// 为了几种口径去给它们全换签名不划算；统计面板本来就会把整表 clone 给子组件，
/// 这里多一次过滤 + 克隆可接受（只在口径或日志变化时算一次）。
#[must_use]
pub fn scoped_entries(
  entries: &[LogEntry],
  book: &StationBook,
  scope: &StatsScope,
) -> Vec<LogEntry> {
  if scope.is_all() {
    return entries.to_vec();
  }
  entries
    .iter()
    .filter(|e| scope.matches(e, book))
    .cloned()
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::logbook::{LogEntry, StationInfo};
  use crate::station::{StationBook, StationProfile};

  fn entry(band: &str, time: &str) -> LogEntry {
    LogEntry {
      band: band.to_owned(),
      time: time.to_owned(),
      ..LogEntry::default()
    }
  }

  /// 一本两档案的册子：1 = 家里（操作员 WXP），2 = 野外（操作员 LZ）。
  fn book() -> StationBook {
    let mut book = StationBook::from_info(&StationInfo {
      callsign: "BG4XXX".into(),
      operator: "WXP".into(),
      gridsquare: "OM89EW".into(),
      ..Default::default()
    });
    book.upsert(StationProfile {
      label: "POTA".into(),
      callsign: "BG4XXX".into(),
      operator: "LZ".into(),
      gridsquare: "OL99AA".into(),
      ..Default::default()
    });
    book
  }

  fn qso(id: u64, station_id: u64, operator: &str) -> LogEntry {
    LogEntry {
      id,
      station_id,
      operator: operator.to_owned(),
      callsign: "JA1AAA".into(),
      date: "2026-10-01".into(),
      time: "12:00".into(),
      freq: "14.074".into(),
      mode: "FT8".into(),
      ..Default::default()
    }
  }

  #[test]
  fn scope_keys_round_trip() {
    for scope in [
      StatsScope::all(),
      StatsScope::of_station(3),
      StatsScope::of_operator("wxp"),
      StatsScope {
        station: Some(7),
        operator: "LZ".into(),
      },
    ] {
      assert_eq!(StatsScope::from_key(&scope.key()), scope, "{}", scope.key());
    }
    // 认不出的短串退化成「全都看」，不让界面崩。
    assert!(StatsScope::from_key("nonsense").is_all());
    assert!(StatsScope::from_key("s:abc|x:1").is_all());
    assert!(StatsScope::all().is_all());
    assert!(!StatsScope::of_station(1).is_all());
  }

  #[test]
  fn scope_key_survives_a_pipe_in_the_operator_name() {
    // 操作员名是自由文本，可能自带 `|`。按 `|` 逐段解析会把后半截当成「认不出的片段」
    // 丢掉（`o:A|B` 只剩 `A`），存进 localStorage 再读回来就不是原来那个口径了。
    for operator in ["A|B", "|", "W|X|Y"] {
      let scope = StatsScope {
        station: Some(3),
        operator: operator.into(),
      };
      assert_eq!(StatsScope::from_key(&scope.key()), scope, "{operator}");
    }
  }

  #[test]
  fn operator_falls_back_to_the_station_profile() {
    let book = book();
    let home = book.active_id;
    // 记录里没写操作员 → 用台站档案的。
    assert_eq!(operator_of(&qso(1, home, ""), &book), "WXP");
    // 记录里写了就用它（临时换人操作：值机员与台站归属是两件事）。
    assert_eq!(operator_of(&qso(2, home, "bd1abc"), &book), "BD1ABC");
    // 归属没指定（老日志）也回落到当前台站。
    assert_eq!(operator_of(&qso(3, 0, ""), &book), "WXP");
  }

  #[test]
  fn scoping_by_station_uses_the_attribution_fallback() {
    let book = book();
    let home = book.active_id;
    let field = book.profiles[1].id;
    let entries = vec![
      qso(1, home, ""),
      qso(2, 0, ""), // 老日志：没有归属，按当前台站算
      qso(3, field, ""),
      qso(4, 999, ""), // 指向已删档案 → 同样回落当前台站
    ];
    let scoped = scoped_entries(&entries, &book, &StatsScope::of_station(home));
    assert_eq!(
      scoped.iter().map(|e| e.id).collect::<Vec<_>>(),
      vec![1, 2, 4],
      "只有野外那条被排除"
    );
    assert_eq!(
      scoped_entries(&entries, &book, &StatsScope::of_station(field))
        .iter()
        .map(|e| e.id)
        .collect::<Vec<_>>(),
      vec![3]
    );
  }

  #[test]
  fn scoping_by_operator_is_case_insensitive_and_ignores_station() {
    let book = book();
    let entries = vec![
      qso(1, book.active_id, "wxp"),
      qso(2, book.active_id, ""),         // 回落到家里档案的 WXP
      qso(3, book.profiles[1].id, "WXP"), // 野外台站，但这次是 WXP 操作的
      qso(4, book.profiles[1].id, "LZ"),
    ];
    let scoped = scoped_entries(&entries, &book, &StatsScope::of_operator("wxp"));
    assert_eq!(
      scoped.iter().map(|e| e.id).collect::<Vec<_>>(),
      vec![1, 2, 3],
      "跨台站按人分账"
    );
    assert_eq!(
      scoped_entries(&entries, &book, &StatsScope::of_operator("lz"))
        .iter()
        .map(|e| e.id)
        .collect::<Vec<_>>(),
      vec![4]
    );
  }

  #[test]
  fn both_dimensions_can_be_combined() {
    let book = book();
    let field = book.profiles[1].id;
    let entries = vec![
      qso(1, field, "WXP"),
      qso(2, field, "LZ"),
      qso(3, book.active_id, "WXP"),
    ];
    let scope = StatsScope {
      station: Some(field),
      operator: "WXP".into(),
    };
    assert_eq!(
      scoped_entries(&entries, &book, &scope)
        .iter()
        .map(|e| e.id)
        .collect::<Vec<_>>(),
      vec![1]
    );
  }

  #[test]
  fn all_scope_keeps_everything_and_a_copy() {
    let book = book();
    let entries = vec![qso(1, book.active_id, ""), qso(2, 0, "")];
    let scoped = scoped_entries(&entries, &book, &StatsScope::all());
    assert_eq!(scoped, entries);
    assert_eq!(scoped.len(), 2);
  }

  #[test]
  fn operators_are_listed_by_frequency_then_name() {
    let book = book();
    let entries = vec![
      qso(1, book.active_id, "LZ"),
      qso(2, book.active_id, "LZ"),
      qso(3, book.active_id, "wxp"),
      qso(4, 0, ""),  // → WXP（家里档案）
      qso(5, 0, " "), // 空白也算没填
    ];
    // WXP 共 3 条（显式 1 + 没填回落 1 + 全空白 1），LZ 2 条 → 按条数降序。
    assert_eq!(operators(&entries, &book), vec!["WXP", "LZ"]);
    // 一条都没有时是空表（界面据此决定要不要显示口径选择器）。
    assert!(operators(&[], &book).is_empty());
    let blank = StationBook::default();
    assert!(operators(&[qso(1, 0, "")], &blank).is_empty());
  }

  #[test]
  fn groups_by_band_and_hour() {
    let entries = vec![
      entry("20m", "08:30"),
      entry("20m", "08:45"),
      entry("20m", "09:00"),
      entry("40m", "08:10"),
    ];
    let heat = hour_band_heatmap(&entries);
    // 按 BAND_ORDER：40m 在 20m 之前
    assert_eq!(heat[0].0, "40m");
    assert_eq!(heat[0].1[8], 1);
    assert_eq!(heat[1].0, "20m");
    assert_eq!(heat[1].1[8], 2);
    assert_eq!(heat[1].1[9], 1);
  }

  #[test]
  fn skips_invalid_time() {
    let entries = vec![
      entry("20m", ""),
      entry("20m", "99:00"),
      entry("20m", "07:00"),
    ];
    let heat = hour_band_heatmap(&entries);
    assert_eq!(heat.len(), 1);
    assert_eq!(heat[0].1[7], 1);
  }

  #[test]
  fn overview_counts_dedupes_and_sorts_stably() {
    let mut a = entry("20m", "08:00");
    a.mode = "FT8".into();
    a.callsign = "ja1x".into();
    a.gridsquare = "PM95".into();
    a.qsl_rcvd = true;
    let mut b = entry("20m", "09:00");
    b.mode = "FT8".into();
    b.callsign = "JA1X".into(); // 大小写不同算同一个呼号
    b.gridsquare = "PM95".into(); // 重复网格只算一次
    let mut c = entry("40m", "10:00");
    c.mode = "CW".into();
    c.callsign = "JA2Y".into();
    // 分布表：条数降序；条数相同时按名字升序（不依赖 `HashMap` 的随机顺序）。
    let o = overview(&[a, b, c]);
    assert_eq!(o.total, 3);
    assert_eq!(o.callsigns, 2);
    assert_eq!(o.grids, vec!["PM95".to_owned()]);
    assert_eq!(o.modes, vec![("FT8".to_owned(), 2), ("CW".to_owned(), 1)]);
    assert_eq!(o.bands, vec![("20m".to_owned(), 2), ("40m".to_owned(), 1)]);
    assert_eq!(o.confirmed, 1, "只有 a 标了已确认");
  }

  #[test]
  fn band_counts_and_distinct_options_come_from_the_same_place() {
    let mut with_band = entry("20m", "08:00");
    with_band.mode = "FT8".into();
    let mut no_band = entry("", "09:00");
    no_band.mode = "FT8".into();
    let entries = [with_band, no_band];
    // 计数保留空波段（比赛页要拿它查表），筛选下拉则不该出现空选项。
    assert_eq!(band_counts(&entries).get(""), Some(&1));
    let (bands, modes) = distinct_bands_and_modes(&entries);
    assert_eq!(bands, vec!["20m".to_owned()]);
    assert_eq!(modes, vec!["FT8".to_owned()]);
  }

  #[test]
  fn single_digit_hours_still_land_on_the_heatmap() {
    // 体检那边（`split_hhmm`）认得 `8:30`，热力图也必须认得 —— 两处口径不一致的话，
    // 表现为「体检查不出问题、热力图却少一条」。
    let entries = vec![entry("20m", "8:30"), entry("20m", "08:45")];
    let heat = hour_band_heatmap(&entries);
    assert_eq!(heat[0].1[8], 2);
  }

  #[test]
  fn counts_daily_qso() {
    let mut a = entry("20m", "08:30");
    a.date = "2024-05-01".to_owned();
    let mut b = entry("20m", "09:00");
    b.date = "2024-05-01".to_owned();
    let mut c = entry("40m", "08:10");
    c.date = "2024-05-02".to_owned();
    let mut invalid = entry("40m", "08:10");
    invalid.date = "bad".to_owned();
    let counts = daily_qso_counts(&[a, b, c, invalid]);
    assert_eq!(counts.get("2024-05-01"), Some(&2));
    assert_eq!(counts.get("2024-05-02"), Some(&1));
    assert!(!counts.contains_key("2024-05-03"));
  }

  #[test]
  fn daily_counts_normalize_the_date_and_reject_junk() {
    let mut loose = entry("20m", "08:30");
    loose.date = "2024-5-1".to_owned(); // 与 `2024-05-01` 是同一天
    let mut canon = entry("20m", "09:00");
    canon.date = "2024-05-01".to_owned();
    // 长度够 10 个字符但不是日期：原来的实现按「切开前 10 个字节」判，会把它当成一天。
    let mut junk = entry("20m", "10:00");
    junk.date = "abcdefghij".to_owned();
    let counts = daily_qso_counts(&[loose, canon, junk]);
    assert_eq!(counts.len(), 1, "两种写法归一化到同一天，乱码不算");
    assert_eq!(counts.get("2024-05-01"), Some(&2));
  }
}
