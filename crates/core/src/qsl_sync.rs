//! QSL 确认同步：把 LoTW / eQSL / Club Log / QRZ 等平台下载的「确认报告」（ADIF）
//! 与本地日志做**差异分析**，由人逐条裁决后再落库。
//!
//! # 为什么不是「直接取逻辑或」
//!
//! 最早的实现是把报告里所有 `Y` 标志按位或进本地（[`apply_qsl_report`]）。这在报告
//! **完整**且只关心新增时没问题，但报告一旦不是全量（只导出了某个日期区间、只导出了
//! 某个波段），「报告里没有」就会被误当成「没有确认」。真正需要人看的是两类行：
//!
//! - **远端新增**：报告说「是」、本地还是「否」→ 可以直接应用；
//! - **冲突**：本地说「是」、而**对该渠道有权威**的来源报告里没有 → 可能是本地手抄错了，
//!   也可能是报告不全，必须由人来判断。
//!
//! # 来源决定「这份报告在说哪些渠道的事」
//!
//! 这是本模块的核心概念（[`QslSource::channels`]）：
//!
//! - LoTW 报告只谈 LoTW（`LOTW_QSL_SENT` / `LOTW_QSL_RCVD`），它没提 `EQSL_QSL_RCVD`
//!   不能算冲突，反过来它没提 `LOTW_QSL_RCVD` 就是真冲突；
//! - **eQSL / QRZ 的导出里也有通用的 `QSL_SENT` / `QSL_RCVD`，但那不是「纸质卡寄出/收到」**。
//!   照着应用会把电子确认错标成「纸质已收」（徽章变绿），所以这些渠道一律不采纳；
//! - Club Log 的 OQRS 就是替你寄纸卡，它的 `QSL_SENT` / `QSL_RCVD` 确实代表纸质卡；
//! - QRZ Logbook 的站内确认在 ADIF 里没有独立字段（落在通用 `QSL_RCVD` 上），本工具
//!   **不猜它的归属**，因此该来源不写入任何渠道、只做匹配分析（`analyze_only`）。
//!   要真正落库得先给模型加一个 qrz 渠道并找到可往返的 ADIF 写法，那是独立的一件事。

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::adif::parse_adif_report;
use crate::logbook::{LogEntry, from_adif};

/// 确认渠道。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Channel {
  /// 纸质卡片。
  Paper,
  /// LoTW。
  Lotw,
  /// eQSL。
  Eqsl,
}

impl Channel {
  /// 全部渠道（界面按此顺序展示）。
  pub const ALL: [Self; 3] = [Self::Paper, Self::Lotw, Self::Eqsl];

  /// 稳定短键。
  #[must_use]
  pub const fn id(self) -> &'static str {
    match self {
      Self::Paper => "paper",
      Self::Lotw => "lotw",
      Self::Eqsl => "eqsl",
    }
  }

  /// 由短键还原。
  #[must_use]
  pub fn from_id(id: &str) -> Option<Self> {
    Self::ALL.into_iter().find(|c| c.id() == id)
  }
}

/// 一个 QSL 标志位：渠道 × 寄出 / 收到。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum QslFlag {
  /// 纸卡已寄出 `QSL_SENT`。
  PaperSent,
  /// 纸卡已收到 `QSL_RCVD`。
  PaperRcvd,
  /// LoTW 已上传 `LOTW_QSL_SENT`。
  LotwSent,
  /// LoTW 已确认 `LOTW_QSL_RCVD`。
  LotwRcvd,
  /// eQSL 已寄出 `EQSL_QSL_SENT`。
  EqslSent,
  /// eQSL 已确认 `EQSL_QSL_RCVD`。
  EqslRcvd,
}

impl QslFlag {
  /// 全部标志位（界面按此顺序展示）。
  pub const ALL: [Self; 6] = [
    Self::PaperSent,
    Self::PaperRcvd,
    Self::LotwSent,
    Self::LotwRcvd,
    Self::EqslSent,
    Self::EqslRcvd,
  ];

  /// 所属渠道。
  #[must_use]
  pub const fn channel(self) -> Channel {
    match self {
      Self::PaperSent | Self::PaperRcvd => Channel::Paper,
      Self::LotwSent | Self::LotwRcvd => Channel::Lotw,
      Self::EqslSent | Self::EqslRcvd => Channel::Eqsl,
    }
  }

  /// 是「收到」还是「寄出」。
  #[must_use]
  pub const fn is_received(self) -> bool {
    matches!(self, Self::PaperRcvd | Self::LotwRcvd | Self::EqslRcvd)
  }

  /// 稳定短键（界面状态、持久化用）。
  #[must_use]
  pub const fn id(self) -> &'static str {
    match self {
      Self::PaperSent => "paper-sent",
      Self::PaperRcvd => "paper-rcvd",
      Self::LotwSent => "lotw-sent",
      Self::LotwRcvd => "lotw-rcvd",
      Self::EqslSent => "eqsl-sent",
      Self::EqslRcvd => "eqsl-rcvd",
    }
  }

  /// 由短键还原。
  #[must_use]
  pub fn from_id(id: &str) -> Option<Self> {
    Self::ALL.into_iter().find(|f| f.id() == id)
  }

  /// 读本地记录上的该标志位。
  #[must_use]
  pub fn get(self, e: &LogEntry) -> bool {
    match self {
      Self::PaperSent => e.qsl_sent,
      Self::PaperRcvd => e.qsl_rcvd,
      Self::LotwSent => e.lotw_sent,
      Self::LotwRcvd => e.lotw_rcvd,
      Self::EqslSent => e.eqsl_sent,
      Self::EqslRcvd => e.eqsl_rcvd,
    }
  }

  /// 写本地记录上的该标志位。
  ///
  /// `PaperSent` 清掉时一并清掉寄出方式（[`LogEntry::qsl_sent_via`]）：留着「未寄出 + 直寄」
  /// 这种自相矛盾的组合，下一个读它的人（列表徽章、QSL 标签页）就会得出错误结论。
  pub fn set(self, e: &mut LogEntry, on: bool) {
    match self {
      Self::PaperSent => {
        e.qsl_sent = on;
        if !on {
          e.qsl_sent_via = crate::qsl_status::QslVia::None;
        }
      }
      Self::PaperRcvd => e.qsl_rcvd = on,
      Self::LotwSent => e.lotw_sent = on,
      Self::LotwRcvd => e.lotw_rcvd = on,
      Self::EqslSent => e.eqsl_sent = on,
      Self::EqslRcvd => e.eqsl_rcvd = on,
    }
  }
}

/// 报告来源：决定「这份报告在说哪些渠道的事」。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QslSource {
  /// LoTW（`LOTW_QSL_*`）。
  Lotw,
  /// eQSL（`EQSL_QSL_*`）。
  Eqsl,
  /// Club Log（OQRS 的纸卡寄出 / 收到）。
  ClubLog,
  /// QRZ Logbook：只做匹配分析，不写入（见模块文档）。
  Qrz,
  /// 其他 / 手抄清单 / 未知来源：按 ADIF 原样相信三个渠道。
  Other,
}

impl QslSource {
  /// 全部来源（界面按此顺序展示）。
  pub const ALL: [Self; 5] = [
    Self::Lotw,
    Self::Eqsl,
    Self::ClubLog,
    Self::Qrz,
    Self::Other,
  ];

  /// 稳定短键。
  #[must_use]
  pub const fn id(self) -> &'static str {
    match self {
      Self::Lotw => "lotw",
      Self::Eqsl => "eqsl",
      Self::ClubLog => "clublog",
      Self::Qrz => "qrz",
      Self::Other => "other",
    }
  }

  /// 由短键还原；认不出时按 [`QslSource::Other`]（信任报告原样）处理。
  #[must_use]
  pub fn from_id(id: &str) -> Self {
    Self::ALL
      .into_iter()
      .find(|s| s.id() == id)
      .unwrap_or(Self::Other)
  }

  /// 该来源对哪些渠道有权威 —— 也是「它的缺席能不能算冲突」的唯一依据。
  #[must_use]
  pub const fn channels(self) -> &'static [Channel] {
    match self {
      Self::Lotw => &[Channel::Lotw],
      Self::Eqsl => &[Channel::Eqsl],
      // Club Log 只谈纸卡：它的导出是上传日志的副本，LoTW / eQSL 标记可能是旧快照，
      // 拿它当权威会造出「本地已确认、报告（旧副本）里没有」的假冲突。
      Self::ClubLog => &[Channel::Paper],
      // QRZ 的确认没有独立 ADIF 字段，不猜归属。
      Self::Qrz => &[],
      Self::Other => &[Channel::Paper, Channel::Lotw, Channel::Eqsl],
    }
  }

  /// 该来源是否只做匹配分析（不写入任何渠道）。
  #[must_use]
  pub const fn analysis_only(self) -> bool {
    self.channels().is_empty()
  }
}

/// 报告中一条与本地对不上的记录（带定位信息，便于人工判断为什么没匹配上）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingQso {
  /// 对方呼号。
  pub callsign: String,
  /// 日期。
  pub date: String,
  /// 时间（UTC）。
  pub time: String,
  /// 波段。
  pub band: String,
  /// 模式。
  pub mode: String,
}

/// 一条匹配上的 QSO 的差异。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QsoDiff {
  /// 本地记录 id。
  pub entry_id: u64,
  /// 对方呼号。
  pub callsign: String,
  /// 日期。
  pub date: String,
  /// 波段标签。
  pub band: String,
  /// 模式。
  pub mode: String,
  /// 本地为否、报告为「是」的标志位（可以直接补上）。
  pub added: Vec<QslFlag>,
  /// 本地为「是」、而该来源有权威的报告里没有的标志位（需要人裁决）。
  pub conflicts: Vec<QslFlag>,
}

impl QsoDiff {
  /// 有没有需要人看的东西（没有就不必出现在差异表里）。
  #[must_use]
  pub fn needs_attention(&self) -> bool {
    !self.added.is_empty() || !self.conflicts.is_empty()
  }
}

/// 一次「来源报告 ↔ 本地日志」的差异。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QslDiff {
  /// 报告里的记录条数。
  pub total: usize,
  /// 对上本地日志的通联条数（按**本地记录**去重，报告里的重复行不会重复计数）。
  pub matched: usize,
  /// 对不上本地的报告行数（明细见 [`QslDiff::missing`]）。
  pub missing_count: usize,
  /// 对上本地、且本地已是最新的条数。
  pub in_sync: usize,
  /// 需要处理的条数（明细见 [`QslDiff::records`]）。
  pub records: Vec<QsoDiff>,
  /// 对不上本地的报告行明细。
  pub missing: Vec<MissingQso>,
  /// 该来源只做匹配分析（不写入任何渠道）。
  pub analyze_only: bool,
  /// 报告文本在中途被截断（有 `<` 却找不到 `>`）：上面的条数与明细都可能不完整。
  pub truncated: bool,
}

/// 对一条差异的裁决。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QsoDecision {
  /// 把「远端新增」的标志位补上。
  pub apply_added: bool,
  /// 「冲突」以远端为准（**会清掉本地的标志位**，唯一的破坏性操作）。
  pub prefer_remote: bool,
}

impl QsoDecision {
  /// 只补新增，不动本地已有的（安全默认）。
  pub const ADD: Self = Self {
    apply_added: true,
    prefer_remote: false,
  };
  /// 完全以远端为准（补新增 + 清掉冲突的本地标志）。
  pub const MIRROR: Self = Self {
    apply_added: true,
    prefer_remote: true,
  };
  /// 保持本地不变。
  pub const KEEP: Self = Self {
    apply_added: false,
    prefer_remote: false,
  };
}

/// 一次同步的结果统计。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct QslSyncResult {
  /// 报告里的记录总数。
  pub total: usize,
  /// 对上本地日志的通联条数。
  pub matched: usize,
  /// 报告里有、本地未匹配到的条数。
  pub unmatched: usize,
  /// 本次实际置真的标志位个数。
  pub flags_applied: usize,
  /// 本次「以远端为准」清掉的标志位个数。
  pub flags_cleared: usize,
}

/// 报告里一条记录在**该来源说话的渠道**上的标志位集合。
fn report_flags(report: &LogEntry, source: QslSource) -> BTreeSet<QslFlag> {
  let mut out = BTreeSet::new();
  for &channel in source.channels() {
    for flag in QslFlag::ALL.into_iter().filter(|f| f.channel() == channel) {
      if flag.get(report) {
        out.insert(flag);
      }
    }
  }
  out
}

/// 分析报告与本地日志的差异（**不修改**日志）。
///
/// 报告里的多行命中同一条本地通联时（重复行、按波段分次导出）会聚合成一条差异，
/// 否则界面上会出现两行一模一样的通联。
#[must_use]
pub fn diff_report(entries: &[LogEntry], report_adif: &str, source: QslSource) -> QslDiff {
  let (records, truncated) = parse_adif_report(report_adif);
  let mut diff = QslDiff {
    total: records.len(),
    analyze_only: source.analysis_only(),
    truncated,
    ..Default::default()
  };
  // 先给本地日志建两张索引表：报告动辄上万行，逐行 `entries.iter().find(...)` 是 O(n·m)，
  // 而 `qso_key()` 每条都要现算（含字符串拼接），代价不低。
  // 重复 key 保留**第一条**，与原来的线性查找语义一致。
  let mut by_key: HashMap<String, usize> = HashMap::with_capacity(entries.len());
  let mut by_id: HashMap<u64, usize> = HashMap::with_capacity(entries.len());
  for (i, e) in entries.iter().enumerate() {
    by_key.entry(e.qso_key()).or_insert(i);
    by_id.entry(e.id).or_insert(i);
  }
  let mut reported: BTreeMap<u64, BTreeSet<QslFlag>> = BTreeMap::new();
  for rec in records {
    let report = from_adif(rec);
    match by_key.get(&report.qso_key()).map(|&i| &entries[i]) {
      Some(local) => {
        reported
          .entry(local.id)
          .or_default()
          .extend(report_flags(&report, source));
      }
      None => {
        diff.missing_count += 1;
        diff.missing.push(MissingQso {
          callsign: report.callsign.clone(),
          date: report.date.clone(),
          time: report.time.clone(),
          band: report.band_label(),
          mode: report.mode.clone(),
        });
      }
    }
  }
  diff.matched = reported.len();
  for (entry_id, remote) in reported {
    let Some(&idx) = by_id.get(&entry_id) else {
      continue;
    };
    let local = &entries[idx];
    let mut qso = QsoDiff {
      entry_id,
      callsign: local.callsign.clone(),
      date: local.date.clone(),
      band: local.band_label(),
      mode: local.mode.clone(),
      added: Vec::new(),
      conflicts: Vec::new(),
    };
    for flag in QslFlag::ALL {
      let (local_on, remote_on) = (flag.get(local), remote.contains(&flag));
      if remote_on && !local_on {
        qso.added.push(flag);
      } else if local_on && !remote_on && source.channels().contains(&flag.channel()) {
        // 只有「来源说话的渠道」才算冲突：LoTW 报告没提 eQSL，不是 eQSL 的结论。
        qso.conflicts.push(flag);
      }
    }
    if qso.needs_attention() {
      diff.records.push(qso);
    } else {
      diff.in_sync += 1;
    }
  }
  diff
}

/// 按逐条裁决应用差异，返回统计。
///
/// 找不到本地记录（差异算出来之后又被删掉）的条数跳过，不报错。
pub fn apply_diff(
  entries: &mut [LogEntry],
  diff: &QslDiff,
  decide: impl Fn(&QsoDiff) -> QsoDecision,
) -> QslSyncResult {
  let mut result = QslSyncResult {
    total: diff.total,
    matched: diff.matched,
    unmatched: diff.missing_count,
    ..Default::default()
  };
  // `diff.records` 可能有上千条，逐条 `entries.iter_mut().find(...)` 是 O(差异数 × 日志数)；
  // 先建一张 id → 下标表（重复 id 保留第一条，与原来的 `find` 语义一致）。
  let mut by_id: HashMap<u64, usize> = HashMap::with_capacity(entries.len());
  for (i, e) in entries.iter().enumerate() {
    by_id.entry(e.id).or_insert(i);
  }
  for qso in &diff.records {
    let decision = decide(qso);
    if !decision.apply_added && !decision.prefer_remote {
      continue;
    }
    let Some(&idx) = by_id.get(&qso.entry_id) else {
      continue;
    };
    let target = &mut entries[idx];
    if decision.apply_added {
      for &flag in &qso.added {
        if !flag.get(target) {
          flag.set(target, true);
          result.flags_applied += 1;
        }
      }
    }
    if decision.prefer_remote {
      for &flag in &qso.conflicts {
        if flag.get(target) {
          flag.set(target, false);
          result.flags_cleared += 1;
        }
      }
    }
  }
  result
}

/// 一次性把报告里的确认全部并进日志（只补新增，**不清本地**）。
///
/// 这是本模块最早的接口，保留给「报告完整且可信」的场景（例如自己手抄的纸质卡清单）；
/// 多来源、可能不全的报告请走 [`diff_report`] + [`apply_diff`] 的人工裁决路径。
#[must_use]
pub fn apply_qsl_report(entries: &mut [LogEntry], report_adif: &str) -> QslSyncResult {
  let diff = diff_report(entries, report_adif, QslSource::Other);
  apply_diff(entries, &diff, |_| QsoDecision::ADD)
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 本地一条通联。
  ///
  /// `id` 必须互不相同：差异按本地记录 id 聚合，两条都用 id 1 会互相覆盖。
  fn qso(id: u64, call: &str, freq: &str, mode: &str, time: &str) -> LogEntry {
    LogEntry {
      id,
      callsign: call.into(),
      date: "2026-09-30".into(),
      time: time.into(),
      freq: freq.into(),
      mode: mode.into(),
      ..Default::default()
    }
  }

  /// 报告里的一条记录：波段由频率反推，保证与本地那条的 `qso_key` 对得上。
  fn record(call: &str, freq: &str, mode: &str, time: &str, extra: &str) -> String {
    let freq: f64 = freq.parse().expect("测试里的频率");
    let band = crate::frequencies::band_of(freq).to_ascii_uppercase();
    format!(
      "<CALL:{}>{call}<QSO_DATE:8>20260930<TIME_ON:4>{time}<BAND:{}>{band}<MODE:{}>{mode}{extra}<EOR>\n",
      call.len(),
      band.len(),
      mode.len()
    )
  }

  fn report(rows: &[String]) -> String {
    format!("<EOH>\n{}", rows.concat())
  }

  #[test]
  fn sources_speak_only_for_their_own_channels() {
    assert_eq!(QslSource::Lotw.channels(), &[Channel::Lotw]);
    assert_eq!(QslSource::Eqsl.channels(), &[Channel::Eqsl]);
    assert_eq!(QslSource::ClubLog.channels(), &[Channel::Paper]);
    assert_eq!(QslSource::Qrz.channels(), &[]);
    assert_eq!(
      QslSource::Other.channels(),
      &[Channel::Paper, Channel::Lotw, Channel::Eqsl]
    );
    // QRZ 那一份是「只分析不写库」，别把它当成写库来源。
    assert!(QslSource::Qrz.analysis_only());
    assert!(!QslSource::Lotw.analysis_only());
  }

  #[test]
  fn source_ids_round_trip() {
    for s in QslSource::ALL {
      assert_eq!(QslSource::from_id(s.id()), s);
    }
    assert_eq!(QslSource::from_id("nope"), QslSource::Other);
  }

  #[test]
  fn flag_ids_and_accessors_agree_with_the_model() {
    for flag in QslFlag::ALL {
      assert_eq!(QslFlag::from_id(flag.id()), Some(flag));
      let mut e = LogEntry::default();
      assert!(!flag.get(&e), "默认应为未置位");
      flag.set(&mut e, true);
      assert!(flag.get(&e), "{} 置位后应读得到", flag.id());
      // 每个标志位只影响自己那一位。
      for other in QslFlag::ALL.into_iter().filter(|f| *f != flag) {
        assert!(
          !other.get(&e),
          "{} 不该被 {} 连带置位",
          other.id(),
          flag.id()
        );
      }
      flag.set(&mut e, false);
      assert!(!flag.get(&e));
    }
    assert!(QslFlag::PaperRcvd.is_received());
    assert!(!QslFlag::LotwSent.is_received());
    assert!(QslFlag::from_id("").is_none());
  }

  #[test]
  fn clearing_paper_sent_also_clears_the_route() {
    // 「未寄出 + 直寄」是自相矛盾的组合，清标志时必须一起清。
    let mut e = LogEntry::default();
    QslFlag::PaperSent.set(&mut e, true);
    e.qsl_sent_via = crate::qsl_status::QslVia::Direct;
    QslFlag::PaperSent.set(&mut e, false);
    assert!(!e.qsl_sent);
    assert_eq!(e.qsl_sent_via, crate::qsl_status::QslVia::None);
  }

  #[test]
  fn additions_are_split_from_conflicts() {
    let mut local = qso(1, "JA1X", "14.074", "FT8", "12:34");
    local.qsl_rcvd = true; // 本地：纸质已收
    let entries = vec![local];
    // LoTW 报告：新增 LoTW 确认；纸质卡不归它管，不该算冲突。
    let lotw = report(&[record(
      "JA1X",
      "14.074",
      "FT8",
      "1234",
      "<LOTW_QSL_RCVD:1>Y",
    )]);
    let d = diff_report(&entries, &lotw, QslSource::Lotw);
    assert_eq!(d.matched, 1);
    assert_eq!(d.records.len(), 1);
    assert_eq!(d.records[0].added, vec![QslFlag::LotwRcvd]);
    assert!(d.records[0].conflicts.is_empty(), "纸质确认不是 LoTW 的事");

    // 同一份报告按「其他来源」解释：纸质确认就成了冲突（本地说有、报告没提）。
    let d = diff_report(&entries, &lotw, QslSource::Other);
    assert_eq!(d.records[0].conflicts, vec![QslFlag::PaperRcvd]);
  }

  #[test]
  fn an_eqsl_export_never_marks_a_paper_card_as_received() {
    // eQSL 的导出里也带通用 `QSL_RCVD`，但那不是纸质卡。
    let entries = vec![qso(1, "JA1X", "14.074", "FT8", "12:34")];
    let eqsl = report(&[record(
      "JA1X",
      "14.074",
      "FT8",
      "1234",
      "<QSL_SENT:1>Y<QSL_RCVD:1>Y<EQSL_QSL_RCVD:1>Y",
    )]);
    let d = diff_report(&entries, &eqsl, QslSource::Eqsl);
    assert_eq!(
      d.records[0].added,
      vec![QslFlag::EqslRcvd],
      "只采纳 eQSL 渠道"
    );
    assert!(!d.records[0].added.contains(&QslFlag::PaperRcvd));
    assert!(!d.records[0].added.contains(&QslFlag::PaperSent));
  }

  #[test]
  fn club_log_speaks_for_paper_cards() {
    let entries = vec![qso(1, "JA1X", "14.074", "FT8", "12:34")];
    let clublog = report(&[record(
      "JA1X",
      "14.074",
      "FT8",
      "1234",
      "<QSL_SENT:1>Y<QSL_SENT_VIA:1>B",
    )]);
    let d = diff_report(&entries, &clublog, QslSource::ClubLog);
    assert_eq!(d.records[0].added, vec![QslFlag::PaperSent]);
  }

  #[test]
  fn qrz_reports_are_analysis_only() {
    let entries = vec![qso(1, "JA1X", "14.074", "FT8", "12:34")];
    let qrz = report(&[record("JA1X", "14.074", "FT8", "1234", "<QSL_RCVD:1>Y")]);
    let d = diff_report(&entries, &qrz, QslSource::Qrz);
    assert!(d.analyze_only);
    assert_eq!(d.matched, 1, "匹配情况仍然要报给用户");
    assert!(d.records.is_empty(), "不猜归属，所以没有可应用的差异");
    // 报告里的 QSL_RCVD 没有被当成纸质已收。
    let r = apply_diff(&mut entries.clone(), &d, |_| QsoDecision::MIRROR);
    assert_eq!(r.flags_applied, 0);
  }

  #[test]
  fn duplicate_report_rows_collapse_into_one_diff() {
    let entries = vec![qso(1, "JA1X", "14.074", "FT8", "12:34")];
    let dup = report(&[
      record("JA1X", "14.074", "FT8", "1234", "<LOTW_QSL_RCVD:1>Y"),
      record("JA1X", "14.074", "FT8", "1234", "<LOTW_QSL_SENT:1>Y"),
    ]);
    let d = diff_report(&entries, &dup, QslSource::Lotw);
    assert_eq!(d.total, 2);
    assert_eq!(d.matched, 1, "同一本地通联只算一条");
    assert_eq!(d.records.len(), 1);
    assert_eq!(
      d.records[0].added,
      vec![QslFlag::LotwSent, QslFlag::LotwRcvd],
      "两行的标志位要合并"
    );
  }

  #[test]
  fn unmatched_rows_are_listed_with_their_identity() {
    let entries = vec![qso(1, "JA1X", "14.074", "FT8", "12:34")];
    let r = report(&[record("DL1A", "14.074", "FT8", "1400", "")]);
    let d = diff_report(&entries, &r, QslSource::Lotw);
    assert_eq!((d.total, d.matched, d.missing_count), (1, 0, 1));
    assert_eq!(d.missing[0].callsign, "DL1A");
    assert_eq!(d.missing[0].date, "2026-09-30");
    // `from_adif` 把 `TIME_ON` 统一成 `HH:MM`，明细照抄这个格式。
    assert_eq!(d.missing[0].time, "14:00");
    assert_eq!(d.missing[0].mode, "FT8");
  }

  #[test]
  fn in_sync_rows_stay_out_of_the_table() {
    let mut local = qso(1, "JA1X", "14.074", "FT8", "12:34");
    local.lotw_rcvd = true;
    let entries = vec![local];
    let r = report(&[record(
      "JA1X",
      "14.074",
      "FT8",
      "1234",
      "<LOTW_QSL_RCVD:1>Y",
    )]);
    let d = diff_report(&entries, &r, QslSource::Lotw);
    assert_eq!((d.matched, d.in_sync, d.records.len()), (1, 1, 0));
  }

  #[test]
  fn decisions_control_what_changes() {
    let mut local = qso(1, "JA1X", "14.074", "FT8", "12:34");
    local.eqsl_rcvd = true; // 本地多了一条 eQSL 确认
    let mut entries = vec![local];
    let r = report(&[record(
      "JA1X",
      "14.074",
      "FT8",
      "1234",
      "<LOTW_QSL_RCVD:1>Y",
    )]);
    let d = diff_report(&entries, &r, QslSource::Other);
    let qso_diff = &d.records[0];
    assert_eq!(qso_diff.added, vec![QslFlag::LotwRcvd]);
    assert_eq!(qso_diff.conflicts, vec![QslFlag::EqslRcvd]);

    // 保持本地：什么都不动。
    let keep = apply_diff(&mut entries, &d, |_| QsoDecision::KEEP);
    assert_eq!((keep.flags_applied, keep.flags_cleared), (0, 0));
    assert!(!entries[0].lotw_rcvd && entries[0].eqsl_rcvd);

    // 只补新增：不动本地已有的。
    let add = apply_diff(&mut entries, &d, |_| QsoDecision::ADD);
    assert_eq!((add.flags_applied, add.flags_cleared), (1, 0));
    assert!(entries[0].lotw_rcvd && entries[0].eqsl_rcvd);

    // 以远端为准：清掉冲突的那一位。
    let mirror = apply_diff(&mut entries, &d, |_| QsoDecision::MIRROR);
    assert_eq!((mirror.flags_applied, mirror.flags_cleared), (0, 1));
    assert!(entries[0].lotw_rcvd && !entries[0].eqsl_rcvd);
    // 统计里带上报告侧的计数，界面不必再自己算。
    assert_eq!((mirror.total, mirror.matched, mirror.unmatched), (1, 1, 0));
  }

  #[test]
  fn applying_skips_records_that_disappeared() {
    let mut entries = vec![qso(1, "JA1X", "14.074", "FT8", "12:34")];
    let r = report(&[record(
      "JA1X",
      "14.074",
      "FT8",
      "1234",
      "<LOTW_QSL_RCVD:1>Y",
    )]);
    let d = diff_report(&entries, &r, QslSource::Lotw);
    entries.clear(); // 差异算完之后通联被删了
    let result = apply_diff(&mut entries, &d, |_| QsoDecision::ADD);
    assert_eq!(result.flags_applied, 0);
    assert_eq!(result.matched, 1, "报告侧的计数照旧");
  }

  #[test]
  fn updates_matching_qsos_and_counts_unmatched() {
    let mut log = vec![
      qso(1, "JA1X", "14.074", "FT8", "12:34"),
      qso(2, "W1AW", "7.010", "CW", "13:00"),
    ];
    let r = report(&[
      record("JA1X", "14.074", "FT8", "1234", "<LOTW_QSL_RCVD:1>Y"),
      record("DL1A", "7.010", "CW", "1400", "<LOTW_QSL_RCVD:1>Y"),
    ]);
    let result = apply_qsl_report(&mut log, &r);
    assert_eq!((result.total, result.matched, result.unmatched), (2, 1, 1));
    assert!(log[0].lotw_rcvd);
    assert!(!log[1].lotw_rcvd);
  }

  #[test]
  fn merges_multiple_confirmations_without_losing_flags() {
    let mut log = vec![qso(1, "JA1X", "14.074", "FT8", "12:34")];
    log[0].qsl_rcvd = true;
    let r = report(&[record(
      "JA1X",
      "14.074",
      "FT8",
      "1234",
      "<LOTW_QSL_RCVD:1>Y",
    )]);
    let _ = apply_qsl_report(&mut log, &r);
    assert!(log[0].qsl_rcvd, "纸卡确认应保留");
    assert!(log[0].lotw_rcvd, "LoTW 确认应累加");
  }

  #[test]
  fn the_legacy_apply_only_ever_adds() {
    // 报告里一条通用 `QSL_*` 都没有：按「其他来源」解释时「已寄出 / 纸卡已收」都算冲突，
    // 但老接口只做并集 —— 绝不能把本地已有的清掉（那是破坏性操作，只能由人勾）。
    let mut log = vec![qso(1, "JA1X", "14.074", "FT8", "12:34")];
    log[0].qsl_sent = true;
    log[0].qsl_sent_via = crate::qsl_status::QslVia::Bureau;
    log[0].qsl_rcvd = true;
    let r = report(&[record(
      "JA1X",
      "14.074",
      "FT8",
      "1234",
      "<LOTW_QSL_RCVD:1>Y",
    )]);
    let result = apply_qsl_report(&mut log, &r);
    assert!(log[0].qsl_sent && log[0].qsl_rcvd, "本地标志位应原样保留");
    assert_eq!(log[0].qsl_sent_via, crate::qsl_status::QslVia::Bureau);
    assert!(log[0].lotw_rcvd, "新增照常补上");
    assert_eq!(result.flags_cleared, 0);
  }
}
