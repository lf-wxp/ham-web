//! 日志体检：把「一眼看不出、却会毁掉奖状进度」的记录问题挑出来，并给出可自动修正的动作。
//!
//! # 为什么要体检
//!
//! 日志是**长期积累**的数据：一条通联录错波段不会报错，只会让 DXCC 少算一个实体；缺了对方
//! 网格不会报错，只会让 VUCC 永远差一格；导入同一份 ADIF 两次也不会报错，只会让统计翻倍。
//! 这些问题的共同点是「不会当场炸」——所以需要一次集中体检。
//!
//! # 检查项与判定依据
//!
//! | 问题 | 判定依据 |
//! | --- | --- |
//! | 缺 / 格式不对的对方网格 | [`lat_lon_from_grid`]（VUCC 与网格奖状要用） |
//! | 日期 / 时间格式不对 | [`crate::logbook::minutes_of`]（`YYYY-MM-DD` + `HH:MM`） |
//! | 时间落在未来 | 与调用方传入的「现在」比较；**很可能是填了本地时**，此时给出减偏移的修正 |
//! | 频率不在业余波段内 | [`AMATEUR_BAND_EDGES`] ∪ 频谱表里的划分 ∪ 卫星业余频段（三处都是本站自己的数据） |
//! | 波段字段与频率不符 | [`band_of`]（频率是更可信的那一侧：`band_label` 也优先用它） |
//! | 话务落在不允许话务的子段 | [`crate::bandplan`] 的模式子段（IARU 三区规划） |
//! | 重复通联 | [`LogEntry::dedupe_key`]（与导入去重同一把尺子） |
//! | 呼号前缀存疑 | [`dxcc::lookup`] 查不出实体 |
//!
//! # 模式那一层只报一种错
//!
//! 「模式与频率不匹配」由 [`crate::bandplan`] 收着的 IARU 三区模式子段判定，但**只报
//! 「话务落在不允许话务的子段」**（例如 30m 整段、20m 的 CW 段、2m 的 144.0–144.1）。
//! 反过来（CW / 数据落在话务段）不报：多数区域规划并未普遍禁止，报了就是误报 ——
//! 这一层宁可漏报也不误报，与体检其余部分的立场一致。
//!
//! # 自动修正只做「不会错」的三件事
//!
//! 频率反推波段、本地时改 UTC、重复项合并。像「缺网格」这种**不能自动补**：用 DXCC 实体中心
//! 猜一个网格写进日志，等于往数据里掺假，统计面板的估算提示里也一直强调那是估算。

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use crate::bandplan;
use crate::bands::{AMATEUR_BAND_EDGES, AMATEUR_SATELLITE_RANGES_MHZ, BANDS, parse_range_mhz};
use crate::dxcc;
use crate::frequencies::band_of;
use crate::grid::lat_lon_from_grid;
use crate::logbook::{LogEntry, minutes_of, stamp};
use crate::qsl_status::QslVia;

/// 判定「时间落在未来」时允许的宽限（分钟）：设备时钟总会差一点，别为两三分钟报错。
const FUTURE_SLACK_MINUTES: i64 = 5;

/// 问题种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IssueKind {
  /// 没填对方网格。
  MissingGrid,
  /// 对方网格格式不对（`lat_lon_from_grid` 解不出）。
  InvalidGrid,
  /// 日期或时间格式不对。
  InvalidDateTime,
  /// 时间落在未来。
  TimeInFuture,
  /// 频率不在业余波段内。
  FrequencyOutOfBand,
  /// 波段字段与频率推算出的不一致。
  BandMismatch,
  /// 话务模式落在不允许话务的模式子段里。
  ModeOutOfSegment,
  /// 重复通联（同一次通联在日志里出现多次）。
  Duplicate,
  /// 呼号前缀查不出 DXCC 实体。
  SuspiciousPrefix,
}

impl IssueKind {
  /// 全部种类（界面按此顺序展示：从「影响奖状」到「可能只是提示」）。
  pub const ALL: [Self; 9] = [
    Self::Duplicate,
    Self::FrequencyOutOfBand,
    Self::BandMismatch,
    Self::ModeOutOfSegment,
    Self::TimeInFuture,
    Self::InvalidDateTime,
    Self::InvalidGrid,
    Self::MissingGrid,
    Self::SuspiciousPrefix,
  ];

  /// 稳定短键。
  #[must_use]
  pub const fn id(self) -> &'static str {
    match self {
      Self::MissingGrid => "missing-grid",
      Self::InvalidGrid => "invalid-grid",
      Self::InvalidDateTime => "invalid-datetime",
      Self::TimeInFuture => "time-in-future",
      Self::FrequencyOutOfBand => "frequency-out-of-band",
      Self::BandMismatch => "band-mismatch",
      Self::ModeOutOfSegment => "mode-out-of-segment",
      Self::Duplicate => "duplicate",
      Self::SuspiciousPrefix => "suspicious-prefix",
    }
  }

  /// 由短键还原。
  #[must_use]
  pub fn from_id(id: &str) -> Option<Self> {
    Self::ALL.into_iter().find(|k| k.id() == id)
  }
}

/// 自动修正动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fix {
  /// 按频率回填波段字段。
  BandFromFreq,
  /// 把日期 / 时间从本地时改成 UTC（减去本机与 UTC 的偏移分钟数）。
  UtcFromLocal {
    /// 本机时区相对 UTC 的偏移（分钟，东八区 = 480）。
    offset_minutes: i32,
  },
  /// 合并重复项：保留第一条，把其余那几条的 QSL 标志位并进来后删掉。
  MergeDuplicates,
}

impl Fix {
  /// 稳定短键（界面用）。
  #[must_use]
  pub const fn id(self) -> &'static str {
    match self {
      Self::BandFromFreq => "band-from-freq",
      Self::UtcFromLocal { .. } => "utc-from-local",
      Self::MergeDuplicates => "merge-duplicates",
    }
  }
}

/// 一条体检发现。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
  /// 问题种类。
  pub kind: IssueKind,
  /// 涉及的通联 id。`Duplicate` 里**第一条是保留项**，其余会被合并掉。
  pub entry_ids: Vec<u64>,
  /// 定位：涉及的第一条通联的呼号。
  pub callsign: String,
  /// 定位：日期。
  pub date: String,
  /// 定位：时间。
  pub time: String,
  /// 定位：波段。
  pub band: String,
  /// 定位：模式。
  pub mode: String,
  /// 补充值（含义随 `kind` 而定）：
  ///
  /// - `InvalidGrid` / `InvalidDateTime` / `FrequencyOutOfBand` / `SuspiciousPrefix` → 记录里的原值；
  /// - `BandMismatch` → 频率推算出的波段；
  /// - `ModeOutOfSegment` → 该频率所属的模式子段（`14.000–14.150 · CW`）；
  /// - `TimeInFuture` → 按本地时解释后换算出的 UTC「YYYY-MM-DD HH:MM」；
  /// - `Duplicate` → 重复条数。
  pub extra: Option<String>,
  /// 可自动修正时给出动作；不能自动修的为 `None`。
  pub fix: Option<Fix>,
}

impl Issue {
  fn new(kind: IssueKind, e: &LogEntry) -> Self {
    Self {
      kind,
      entry_ids: vec![e.id],
      callsign: e.callsign.trim().to_owned(),
      date: e.date.clone(),
      time: e.time.clone(),
      // 定位信息用**记录里的写法**（有的话）：`BandMismatch` 要靠它显示「记录写 40M、
      // 应为 20m」；记录里没填波段时才退回按频率推出来的那个。
      band: if e.band.trim().is_empty() {
        e.band_label()
      } else {
        e.band.trim().to_owned()
      },
      mode: e.mode.clone(),
      extra: None,
      fix: None,
    }
  }

  fn extra(mut self, value: impl Into<String>) -> Self {
    self.extra = Some(value.into());
    self
  }

  fn fixed(mut self, fix: Fix) -> Self {
    self.fix = Some(fix);
    self
  }

  fn ids(mut self, ids: Vec<u64>) -> Self {
    self.entry_ids = ids;
    self
  }

  /// 自动修正动作（没有就是 `None`）。字段 [`Issue::fix`] 的同值访问器，改名以免
  /// 与字段撞名。
  #[must_use]
  pub fn fix_action(&self) -> Option<Fix> {
    self.fix
  }
}

/// 体检上下文：现在几点、本机在哪个时区。由调用方注入，核心不读时钟。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealthContext {
  /// 当前 UTC 时间（自 1970-01-01 起经过的分钟数）。
  pub now_utc_minutes: i64,
  /// 本机时区相对 UTC 的偏移（分钟，东八区 = 480）。
  pub local_offset_minutes: i32,
}

impl HealthContext {
  /// 由「UTC 日期 + 时间」与本地偏移构造（测试与调用方都用它，省得手算分钟数）。
  ///
  /// 日期或时间非法时返回 `None`：这里刻意不给 panic 版本 —— 本 crate 编进 wasm，
  /// 一次 `expect` 就是整页白屏。调用方拿不到合法的「现在」时应自己决定退路
  /// （界面用的是浏览器时钟，正常走不到 `None`）。也不能静默当成 0：那会把「现在」
  /// 塌缩到 1970 年，让每条记录都判成「时间在未来」。
  #[must_use]
  pub fn new(date: &str, time: &str, local_offset_minutes: i32) -> Option<Self> {
    Some(Self {
      now_utc_minutes: minutes_of(date, time)?,
      local_offset_minutes,
    })
  }
}

/// 频率文本 → MHz（只管纯数字；带单位的写法解析不出就跳过，宁可漏报也不误报）。
fn parse_freq(freq: &str) -> Option<f64> {
  let v: f64 = freq.trim().parse().ok()?;
  (v.is_finite() && v > 0.0).then_some(v)
}

/// 频谱表里全部业余业务划分的预解析区间（表是静态的，体检逐条调 [`is_amateur_frequency`]，
/// 每次现解析字符串是纯浪费）。
static PLAN_RANGES: LazyLock<Vec<(f64, f64)>> = LazyLock::new(|| {
  BANDS
    .iter()
    .flat_map(|b| b.allocations)
    .filter_map(|a| parse_range_mhz(a.range))
    .collect()
});

/// 该频率是否落在业余波段内。
///
/// 三个来源都是本站自己的数据：常用波段边界（[`AMATEUR_BAND_EDGES`]，按中国口径）、
/// 频谱表里的全部业余业务划分、以及卫星业余频段。只用前两者会漏掉一部分微波波段
/// （例如 13cm 的 2400–2450 只在卫星频段表里），只用前者会把微波通联误报成越界。
#[must_use]
pub fn is_amateur_frequency(mhz: f64) -> bool {
  let in_edges = AMATEUR_BAND_EDGES
    .iter()
    .any(|&(_, lo, hi)| mhz >= lo && mhz < hi);
  let in_satellite = AMATEUR_SATELLITE_RANGES_MHZ
    .iter()
    .any(|&(lo, hi)| mhz >= lo && mhz < hi);
  let in_plan = PLAN_RANGES.iter().any(|&(lo, hi)| mhz >= lo && mhz < hi);
  in_edges || in_satellite || in_plan
}

/// 体检报告。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HealthReport {
  /// 参与体检的通联条数。
  pub entries: usize,
  /// 发现的问题（按记录顺序）。
  pub issues: Vec<Issue>,
}

impl HealthReport {
  /// 没有发现问题。
  #[must_use]
  pub fn is_clean(&self) -> bool {
    self.issues.is_empty()
  }

  /// 某一类问题的条数。
  #[must_use]
  pub fn count(&self, kind: IssueKind) -> usize {
    self.issues.iter().filter(|i| i.kind == kind).count()
  }

  /// 能自动修正的条数。
  #[must_use]
  pub fn fixable_count(&self) -> usize {
    self.issues.iter().filter(|i| i.fix.is_some()).count()
  }

  /// 某一类里能自动修正的条数（界面按种类给出「修正」按钮）。
  #[must_use]
  pub fn fixable_of(&self, kind: IssueKind) -> usize {
    self
      .issues
      .iter()
      .filter(|i| i.kind == kind && i.fix.is_some())
      .count()
  }
}

/// 体检一遍日志。
#[must_use]
pub fn check(entries: &[LogEntry], ctx: &HealthContext) -> HealthReport {
  let mut issues = Vec::new();
  for e in entries {
    check_entry(e, ctx, &mut issues);
  }
  issues.extend(check_duplicates(entries));
  HealthReport {
    entries: entries.len(),
    issues,
  }
}

fn check_entry(e: &LogEntry, ctx: &HealthContext, out: &mut Vec<Issue>) {
  // 呼号 / DXCC 实体。
  let call = e.callsign.trim();
  if call.is_empty() || dxcc::lookup(call).is_none() {
    out.push(Issue::new(IssueKind::SuspiciousPrefix, e).extra(call));
  }

  // 对方网格。
  let grid = e.gridsquare.trim();
  if grid.is_empty() {
    out.push(Issue::new(IssueKind::MissingGrid, e));
  } else if lat_lon_from_grid(grid).is_none() {
    out.push(Issue::new(IssueKind::InvalidGrid, e).extra(grid));
  }

  // 日期 / 时间。
  match minutes_of(&e.date, &e.time) {
    None => {
      let raw = format!("{} {}", e.date.trim(), e.time.trim());
      out.push(Issue::new(IssueKind::InvalidDateTime, e).extra(raw.trim().to_owned()));
    }
    Some(at) => {
      if at > ctx.now_utc_minutes + FUTURE_SLACK_MINUTES {
        // 填了本地时的话，真正的 UTC = 记录值 − 本机偏移。这个换算结果不在未来，
        // 才敢说是「本地时误填」；否则只是一个凭空的未来时间，不给修法。
        let as_utc = at - i64::from(ctx.local_offset_minutes);
        let mut issue = Issue::new(IssueKind::TimeInFuture, e);
        if as_utc <= ctx.now_utc_minutes + FUTURE_SLACK_MINUTES {
          let (d, t) = stamp(as_utc);
          issue = issue.extra(format!("{d} {t}")).fixed(Fix::UtcFromLocal {
            offset_minutes: ctx.local_offset_minutes,
          });
        }
        out.push(issue);
      }
    }
  }

  // 频率：越界与波段字段不符。
  if let Some(mhz) = parse_freq(&e.freq)
    && !is_amateur_frequency(mhz)
  {
    out.push(Issue::new(IssueKind::FrequencyOutOfBand, e).extra(e.freq.trim()));
  }
  // 模式与波段规划：只报「话务落在不允许话务的子段」（判据见 `bandplan` 的文档）。
  if let Some(mhz) = parse_freq(&e.freq)
    && let Some((_, seg)) = bandplan::phone_out_of_segment(&e.mode, mhz)
  {
    out.push(
      // 用中点分隔而不是套一层括号：子段文字本身可能带括号（如「（WARC）」），
      // 再套一层会读成「…（仅 CW / 窄带数据（WARC））」。
      Issue::new(IssueKind::ModeOutOfSegment, e).extra(format!("{} · {}", seg.range, seg.text)),
    );
  }
  if let Some(mhz) = parse_freq(&e.freq) {
    let derived = band_of(mhz);
    let recorded = e.band.trim();
    if derived != "其他" && !recorded.is_empty() && !recorded.eq_ignore_ascii_case(derived) {
      out.push(
        Issue::new(IssueKind::BandMismatch, e)
          .extra(derived)
          .fixed(Fix::BandFromFreq),
      );
    }
  }
}

/// 重复通联：同一把钥匙（[`LogEntry::dedupe_key`]）出现多次，每组合并成一条发现。
fn check_duplicates(entries: &[LogEntry]) -> Vec<Issue> {
  // key 每条只算一遍（拼字符串有开销），分组与上报两轮都吃这份现成的。
  //
  // 缺日期 / 时间的记录没有 `dedupe_key`（见其文档）：放它们进分组的话，同一台站在
  // 同一波段模式上的多次通联会撞成一条，而这一条挂着「合并」这个**会删记录**的修正
  // 动作 —— 宁可漏报，也不能让用户点一下就丢数据。
  let keyed: Vec<(Option<String>, u64)> = entries.iter().map(|e| (e.dedupe_key(), e.id)).collect();
  let mut groups: HashMap<&str, Vec<u64>> = HashMap::new();
  // 每组只在**位置最靠前**的那条记录上报一次。按位置而不是按 id 判断：日志里可能有
  // 两条同 id 的记录，按 id 比会让同一组报两遍。
  let mut first_at: HashMap<&str, usize> = HashMap::new();
  for (at, (key, id)) in keyed.iter().enumerate() {
    let Some(key) = key.as_deref() else {
      continue;
    };
    groups.entry(key).or_default().push(*id);
    first_at.entry(key).or_insert(at);
  }
  let mut out = Vec::new();
  for (at, e) in entries.iter().enumerate() {
    let Some(key) = keyed[at].0.as_deref() else {
      continue;
    };
    let Some(ids) = groups.get(key) else {
      continue;
    };
    if ids.len() > 1 && first_at.get(key) == Some(&at) {
      out.push(
        Issue::new(IssueKind::Duplicate, e)
          .extra(ids.len().to_string())
          .ids(ids.clone())
          .fixed(Fix::MergeDuplicates),
      );
    }
  }
  out
}

/// 应用一条发现的自动修正，返回被改动（含删除）的记录条数。
///
/// 找不到对应记录（体检之后又被删了）时什么都不做，返回 0。
pub fn apply_fix(entries: &mut Vec<LogEntry>, issue: &Issue) -> usize {
  apply_fixes(entries, std::slice::from_ref(issue))
}

/// 批量应用自动修正（界面的「一键修正」），返回被改动（含删除）的记录条数。
///
/// 逐条调 [`apply_fix`] 会为每条发现重扫一遍全表，一键修正因此是
/// O(发现数 × 条数)；这里先把 `id → 下标` 建**一次**表再改。按下标而不是按 id 扫，
/// 还有一个语义上的好处：日志里两条记录撞 id 时（手改 localStorage、第三方 JSON
/// 都能留下），每条发现只落到它自己那一条上，不会顺手改掉别人。
///
/// 会删记录的合并放到最后单独跑 —— 删除会让下标失效。
pub fn apply_fixes(entries: &mut Vec<LogEntry>, issues: &[Issue]) -> usize {
  let index: HashMap<u64, usize> = entries
    .iter()
    .enumerate()
    .map(|(at, e)| (e.id, at))
    .collect();
  let mut changed = 0;
  for issue in issues {
    match issue.fix {
      None | Some(Fix::MergeDuplicates) => {}
      // 按频率回填波段。
      Some(Fix::BandFromFreq) => {
        changed += rewrite(entries, &index, &issue.entry_ids, |e| {
          let Some(mhz) = parse_freq(&e.freq) else {
            return false;
          };
          let derived = band_of(mhz);
          if derived == "其他" || e.band.trim().eq_ignore_ascii_case(derived) {
            return false;
          }
          e.band = derived.to_owned();
          true
        });
      }
      // 本地时改 UTC：减去体检时记下的本机偏移（不是「现在」的偏移，否则跨时区漂移）。
      Some(Fix::UtcFromLocal { offset_minutes }) => {
        changed += rewrite(entries, &index, &issue.entry_ids, |e| {
          let Some(at) = minutes_of(&e.date, &e.time) else {
            return false;
          };
          let (d, t) = stamp(at - i64::from(offset_minutes));
          e.date = d;
          e.time = t;
          true
        });
      }
    }
  }
  for issue in issues {
    if issue.fix == Some(Fix::MergeDuplicates) {
      changed += merge_duplicates(entries, &issue.entry_ids);
    }
  }
  changed
}

/// 按 `id → 下标` 表逐条改写，返回真正改动的条数。
fn rewrite(
  entries: &mut [LogEntry],
  index: &HashMap<u64, usize>,
  ids: &[u64],
  mut f: impl FnMut(&mut LogEntry) -> bool,
) -> usize {
  let mut changed = 0;
  for id in ids {
    let Some(&at) = index.get(id) else {
      continue;
    };
    if let Some(e) = entries.get_mut(at)
      && f(e)
    {
      changed += 1;
    }
  }
  changed
}

/// 合并一组重复项：保留 `ids[0]` 对应的**那一条记录**，其余的先并进来再删掉。
///
/// 按**下标**定位与删除、而不是按 id 过滤：日志里出现两条同 id 的记录时，按 id 删会把
/// 刚写回合并结果的保留项自己也删掉 —— 整笔通联消失，返回值却照旧报告「已修正」。
fn merge_duplicates(entries: &mut Vec<LogEntry>, ids: &[u64]) -> usize {
  let Some(&keep_id) = ids.first() else {
    return 0;
  };
  let Some(mut keep_at) = entries.iter().position(|e| e.id == keep_id) else {
    return 0;
  };
  // id 撞车时 `position` 只找到第一条，其余同 id 的记录同样算重复项 —— 这正是
  // 按 id 过滤会误删保留项的那个场景。
  let dupe_ids: HashSet<u64> = ids.iter().skip(1).copied().collect();
  let dupe_at: Vec<usize> = entries
    .iter()
    .enumerate()
    .filter(|(at, e)| *at != keep_at && dupe_ids.contains(&e.id))
    .map(|(at, _)| at)
    .collect();
  if dupe_at.is_empty() {
    return 0;
  }
  let mut merged = entries[keep_at].clone();
  let before = merged.clone();
  for &at in &dupe_at {
    merge_into(&mut merged, &entries[at]);
  }
  let changed = dupe_at.len() + usize::from(merged != before);
  // 从后往前删，顺带把保留项的下标往左挪：重复项有前有后时不调整就会写错位置。
  for &at in dupe_at.iter().rev() {
    entries.remove(at);
    if at < keep_at {
      keep_at -= 1;
    }
  }
  entries[keep_at] = merged;
  changed
}

/// 把 `from` 的信息并进 `to`：QSL 标志位取或，文本字段只填空、不覆盖。
///
/// 这里必须列**全部**可合并字段：漏掉的字段等于在合并时被丢掉（重复项随后就被删），
/// 而 `freq` / `operator` / SOTA / POTA 参考号、竞赛交换这些恰恰常常只写在其中一条上
/// —— 频率丢了就再也反推不出波段与子段，操作员丢了会把这条通联算到别人头上。
fn merge_into(to: &mut LogEntry, from: &LogEntry) {
  to.qsl_sent |= from.qsl_sent;
  to.qsl_rcvd |= from.qsl_rcvd;
  to.lotw_sent |= from.lotw_sent;
  to.lotw_rcvd |= from.lotw_rcvd;
  to.eqsl_sent |= from.eqsl_sent;
  to.eqsl_rcvd |= from.eqsl_rcvd;
  to.qrz_rcvd |= from.qrz_rcvd;
  // 寄出方式只在「已寄出但方式未知」时补入：已有方式不被覆盖。
  if to.qsl_sent && to.qsl_sent_via == QslVia::None {
    to.qsl_sent_via = from.qsl_sent_via;
  }
  fill(&mut to.freq, &from.freq);
  fill(&mut to.band, &from.band);
  fill(&mut to.operator, &from.operator);
  fill(&mut to.gridsquare, &from.gridsquare);
  fill(&mut to.name, &from.name);
  fill(&mut to.qth, &from.qth);
  fill(&mut to.rst_sent, &from.rst_sent);
  fill(&mut to.rst_rcvd, &from.rst_rcvd);
  fill(&mut to.tx_pwr, &from.tx_pwr);
  fill(&mut to.time_off, &from.time_off);
  fill(&mut to.prop_mode, &from.prop_mode);
  fill(&mut to.sat_name, &from.sat_name);
  fill(&mut to.sota_ref, &from.sota_ref);
  fill(&mut to.pota_ref, &from.pota_ref);
  fill(&mut to.contest_id, &from.contest_id);
  fill(&mut to.stx, &from.stx);
  fill(&mut to.srx, &from.srx);
  fill(&mut to.state, &from.state);
  fill(&mut to.iota, &from.iota);
  fill(&mut to.dxcc, &from.dxcc);
  fill(&mut to.cqz, &from.cqz);
  fill(&mut to.ituz, &from.ituz);
  fill(&mut to.remark, &from.remark);
}

/// 只填空值，不覆盖已有写法。
fn fill(to: &mut String, from: &str) {
  if to.trim().is_empty() && !from.trim().is_empty() {
    *to = from.to_owned();
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn entry(id: u64, call: &str) -> LogEntry {
    LogEntry {
      id,
      callsign: call.into(),
      date: "2026-10-01".into(),
      time: "12:00".into(),
      freq: "14.074".into(),
      mode: "FT8".into(),
      gridsquare: "PM95".into(),
      ..Default::default()
    }
  }

  /// 体检「现在」：2026-10-08 01:30 UTC，本机东八区。
  fn ctx() -> HealthContext {
    HealthContext::new("2026-10-08", "01:30", 480).expect("测试用的时间是写死的，必须合法")
  }

  fn kinds(report: &HealthReport) -> Vec<IssueKind> {
    report.issues.iter().map(|i| i.kind).collect()
  }

  #[test]
  fn a_clean_entry_has_nothing_to_report() {
    let log = vec![entry(1, "JA1AAA"), entry(2, "BG1AA")];
    let report = check(&log, &ctx());
    assert!(
      report.is_clean(),
      "干净的日志不该有问题：{:?}",
      report.issues
    );
    assert_eq!(report.entries, 2);
    assert_eq!(report.fixable_count(), 0);
  }

  #[test]
  fn issue_ids_and_kinds_round_trip() {
    for kind in IssueKind::ALL {
      assert_eq!(IssueKind::from_id(kind.id()), Some(kind));
    }
    assert_eq!(IssueKind::from_id("nope"), None);
    assert_eq!(Fix::BandFromFreq.id(), "band-from-freq");
    assert_eq!(
      Fix::UtcFromLocal {
        offset_minutes: 480
      }
      .id(),
      "utc-from-local"
    );
    assert_eq!(Fix::MergeDuplicates.id(), "merge-duplicates");
  }

  #[test]
  fn missing_and_invalid_grid_are_distinguished() {
    let mut a = entry(1, "JA1AAA");
    a.gridsquare.clear();
    let mut b = entry(2, "JA1AAA");
    b.time = "12:01".into(); // 换个时间，免得两条记录先被判成重复
    b.gridsquare = "PM95X".into(); // 奇数长度，不是合法网格
    let report = check(&[a, b], &ctx());
    assert_eq!(
      kinds(&report),
      vec![IssueKind::MissingGrid, IssueKind::InvalidGrid]
    );
    // 缺网格**不能**自动补：拿实体中心猜一个写进日志等于掺假。
    assert!(report.issues[0].fix_action().is_none());
    assert_eq!(report.issues[1].extra.as_deref(), Some("PM95X"));
  }

  #[test]
  fn invalid_date_or_time_is_reported_once() {
    let mut a = entry(1, "JA1AAA");
    a.date = "2026/10/01".into();
    let mut b = entry(2, "JA1AAA");
    b.time = "25:00".into();
    let mut c = entry(3, "JA1AAA");
    c.time = "12:60".into();
    let report = check(&[a, b, c], &ctx());
    assert_eq!(report.count(IssueKind::InvalidDateTime), 3);
    assert_eq!(report.issues[0].extra.as_deref(), Some("2026/10/01 12:00"));
    assert_eq!(report.issues[1].extra.as_deref(), Some("2026-10-01 25:00"));
    assert_eq!(report.fixable_count(), 0, "时间格式错没有安全修法");
  }

  #[test]
  fn a_local_time_entry_is_detected_and_can_be_shifted_to_utc() {
    // 本机 09:30（东八区）被当成 UTC 填了进去 —— 以 UTC 看落在未来，减去 8 小时才合理。
    let mut e = entry(1, "JA1AAA");
    e.date = "2026-10-08".into();
    e.time = "09:30".into();
    let mut log = vec![e];
    let report = check(&log, &ctx());
    let issue = &report.issues[0];
    assert_eq!(issue.kind, IssueKind::TimeInFuture);
    assert_eq!(issue.extra.as_deref(), Some("2026-10-08 01:30"));
    assert_eq!(
      issue.fix_action(),
      Some(Fix::UtcFromLocal {
        offset_minutes: 480
      })
    );

    assert_eq!(apply_fix(&mut log, issue), 1);
    assert_eq!(log[0].date, "2026-10-08");
    assert_eq!(log[0].time, "01:30");
    // 修完之后再体检就没有这一条了。
    assert!(check(&log, &ctx()).is_clean());
  }

  #[test]
  fn a_bogus_future_date_gets_no_guess() {
    let mut e = entry(1, "JA1AAA");
    e.date = "2030-01-01".into();
    let report = check(&[e], &ctx());
    assert_eq!(report.count(IssueKind::TimeInFuture), 1);
    // 减 8 小时仍在未来 → 不是「本地时误填」，不给修法。
    assert!(report.issues[0].fix_action().is_none());
    assert!(report.issues[0].extra.is_none());
  }

  #[test]
  fn a_few_minutes_ahead_is_within_the_clock_slack() {
    let mut e = entry(1, "JA1AAA");
    e.date = "2026-10-08".into();
    e.time = "01:33".into(); // 比「现在」晚 3 分钟
    assert!(check(&[e], &ctx()).is_clean());
  }

  #[test]
  fn common_and_microwave_bands_are_all_accepted() {
    for mhz in [
      1.84, 3.573, 7.074, 10.136, 14.074, 18.1, 21.074, 24.915, 28.5, 50.1, 144.39, 432.1,
      // 微波：只在卫星频段表 / 频谱表里出现，仍须放行。
      1296.0, 2400.2, 5760.1, 10368.1, 24048.0, 47088.0, 76000.0, 122250.0,
    ] {
      assert!(is_amateur_frequency(mhz), "{mhz} MHz 应是业余波段");
    }
    for mhz in [12.345, 27.555, 7.5, 100.5, 88.5, 0.05] {
      assert!(!is_amateur_frequency(mhz), "{mhz} MHz 不该被当成业余波段");
    }
  }

  #[test]
  fn out_of_band_frequency_is_reported_without_a_fix() {
    let mut e = entry(1, "JA1AAA");
    e.freq = "27.555".into();
    e.band.clear();
    let report = check(&[e], &ctx());
    assert_eq!(kinds(&report), vec![IssueKind::FrequencyOutOfBand]);
    assert_eq!(report.issues[0].extra.as_deref(), Some("27.555"));
    assert!(
      report.issues[0].fix_action().is_none(),
      "频率错得由人来判断"
    );
  }

  #[test]
  fn band_field_is_corrected_from_the_frequency() {
    let mut e = entry(1, "JA1AAA");
    e.band = "40M".into(); // 频率是 14.074 → 20m
    let mut log = vec![e];
    let report = check(&log, &ctx());
    assert_eq!(kinds(&report), vec![IssueKind::BandMismatch]);
    assert_eq!(report.issues[0].extra.as_deref(), Some("20m"), "修成什么");
    assert_eq!(
      report.issues[0].band, "40M",
      "定位信息用记录里的写法，才读得出「记录写 40M、应为 20m」"
    );

    assert_eq!(apply_fix(&mut log, &report.issues[0]), 1);
    assert_eq!(log[0].band, "20m");
    assert!(check(&log, &ctx()).is_clean());
  }

  #[test]
  fn phone_in_a_cw_only_segment_is_reported() {
    // 20m 的 14.000–14.150 按三区规划只给 CW（`bandplan`）。
    let mut ssb = entry(1, "JA1AAA");
    ssb.mode = "SSB".into();
    ssb.freq = "14.050".into();
    let report = check(&[ssb], &ctx());
    assert_eq!(kinds(&report), vec![IssueKind::ModeOutOfSegment]);
    assert_eq!(
      report.issues[0].extra.as_deref(),
      Some("14.000–14.150 · CW"),
      "要指出落在哪个子段里"
    );
    assert!(
      report.issues[0].fix_action().is_none(),
      "改模式还是改频率得人来定"
    );

    // 同一个频率上的 CW 与 FT8 都不报：这一层只管话务（见模块文档）。
    let mut cw = entry(2, "JA1AAA");
    cw.freq = "14.050".into();
    cw.mode = "CW".into();
    let mut digi = entry(3, "JA1AAA");
    digi.freq = "14.074".into();
    digi.mode = "FT8".into();
    assert!(check(&[cw, digi], &ctx()).is_clean());
  }

  #[test]
  fn thirty_metres_has_no_phone_at_all() {
    // WARC 波段整段只给 CW / 窄带数据 —— 路线图里举的另一个例子。
    let mut e = entry(1, "JA1AAA");
    e.mode = "SSB".into();
    e.freq = "10.130".into();
    let report = check(&[e], &ctx());
    assert_eq!(kinds(&report), vec![IssueKind::ModeOutOfSegment]);
  }

  #[test]
  fn voice_outside_the_plan_is_left_alone() {
    // 没有子段可判（微波 / 越界频率）、或模式归不进话务（空白 / DATA / SSTV）时都不动嘴。
    for (mode, freq) in [
      ("SSB", "2400.100"),
      ("SSB", "27.555"),
      ("", "14.050"),
      ("DATA", "14.050"),
      ("SSTV", "3.550"),
    ] {
      let mut e = entry(1, "JA1AAA");
      e.mode = mode.into();
      e.freq = freq.into();
      let issues = kinds(&check(&[e], &ctx()));
      assert!(
        !issues.contains(&IssueKind::ModeOutOfSegment),
        "{mode}@{freq} 不该报模式与频率不匹配"
      );
    }
  }

  #[test]
  fn records_without_a_band_field_are_not_flagged() {
    // 只填频率、没填 BAND 是常态（`band_label` 会自己推），不该报错。
    let mut e = entry(1, "JA1AAA");
    e.band.clear();
    assert!(check(&[e], &ctx()).is_clean());
  }

  #[test]
  fn duplicates_are_grouped_at_the_first_record() {
    let log = vec![entry(1, "JA1AAA"), entry(2, "BG1AA"), entry(3, "JA1AAA")];
    let report = check(&log, &ctx());
    assert_eq!(report.count(IssueKind::Duplicate), 1);
    let issue = &report.issues[0];
    assert_eq!(issue.entry_ids, vec![1, 3], "保留第一条，其余待合并");
    assert_eq!(issue.extra.as_deref(), Some("2"));
    assert_eq!(issue.fix_action(), Some(Fix::MergeDuplicates));
  }

  #[test]
  fn merging_duplicates_keeps_every_confirmation_and_only_fills_blanks() {
    let mut a = entry(1, "JA1AAA");
    a.remark = "第一条的备注".into();
    a.qsl_rcvd = true;
    let mut b = entry(3, "JA1AAA");
    b.lotw_rcvd = true;
    b.qrz_rcvd = true;
    b.gridsquare.clear();
    b.name = "Yamada".into();
    b.qsl_sent = true;
    b.qsl_sent_via = crate::qsl_status::QslVia::Bureau;
    let mut log = vec![a, entry(2, "BG1AA"), b];
    let report = check(&log, &ctx());
    let issue = report
      .issues
      .iter()
      .find(|i| i.kind == IssueKind::Duplicate)
      .expect("应报出重复项");

    assert_eq!(apply_fix(&mut log, issue), 2, "改一条 + 删一条");
    assert_eq!(log.len(), 2, "重复项已删除");
    let keep = log.iter().find(|e| e.id == 1).expect("保留第一条");
    assert!(
      keep.qsl_rcvd && keep.lotw_rcvd && keep.qrz_rcvd && keep.qsl_sent,
      "确认不能丢"
    );
    assert_eq!(keep.qsl_sent_via, crate::qsl_status::QslVia::Bureau);
    assert_eq!(keep.name, "Yamada", "空字段从重复项补上");
    assert_eq!(keep.remark, "第一条的备注", "已有写法不被覆盖");
    // 顺带把「对方网格为空的重复项」并掉之后，保留项的网格仍在（原本就有）。
    assert_eq!(keep.gridsquare, "PM95");
  }

  #[test]
  fn records_without_a_date_or_time_are_never_reported_as_duplicates() {
    // 缺日期 / 时间时 `qso_key` 会退化成「呼号|波段|模式」：同一台站在同一波段模式上的
    // 多次通联都会撞到同一个 key。这类记录一律不参与重复判定 —— 那条发现挂着会删记录的
    // 「合并」，误报的代价是丢数据。
    let mut a = entry(1, "JA1AAA");
    a.date.clear();
    let mut b = entry(2, "JA1AAA");
    b.date.clear();
    let mut c = entry(3, "JA1AAA");
    c.time.clear();
    let report = check(&[a, b, c], &ctx());
    assert_eq!(report.count(IssueKind::Duplicate), 0);
    // 但不参与去重不等于不体检：它们各自的日期 / 时间问题照旧要报出来。
    assert_eq!(report.count(IssueKind::InvalidDateTime), 3);
  }

  #[test]
  fn merging_duplicates_survives_colliding_ids() {
    // 手改过的 localStorage、第三方 JSON 里都可能出现两条同 id 的记录。按 id 过滤删除
    // 会把刚写回合并结果的保留项自己也删掉 —— 整笔通联消失，返回值却报告「已修正」。
    let mut a = entry(7, "JA1AAA");
    a.lotw_rcvd = true;
    let b = entry(7, "JA1AAA");
    let mut log = vec![a, b];
    let report = check(&log, &ctx());
    let issue = report
      .issues
      .iter()
      .find(|i| i.kind == IssueKind::Duplicate)
      .expect("应报出重复项");
    assert_eq!(issue.entry_ids, vec![7, 7]);

    assert_eq!(apply_fix(&mut log, issue), 1, "只删重复的那一条");
    assert_eq!(log.len(), 1, "保留项必须还在");
    assert_eq!(log[0].id, 7);
    assert!(log[0].lotw_rcvd, "确认不能随重复项一起被删掉");
  }

  #[test]
  fn merging_duplicates_carries_every_field_over() {
    // 每个可合并字段都要补：漏掉的字段在合并里等于被丢掉（重复项随后就被删）。
    let mut a = entry(1, "JA1AAA");
    a.freq.clear();
    a.band = "20m".into(); // 只剩波段字段，靠它保持与重复项同一个 key
    let mut b = entry(3, "JA1AAA");
    b.operator = "WXP".into();
    b.sota_ref = "JA/TK-001".into();
    b.pota_ref = "JA-0123".into();
    b.contest_id = "CQ-WW-CW".into();
    b.stx = "001".into();
    b.srx = "599".into();
    b.prop_mode = "SAT".into();
    b.sat_name = "AO-91".into();
    b.time_off = "12:34".into();
    let mut log = vec![a, b];
    let report = check(&log, &ctx());
    let issue = report
      .issues
      .iter()
      .find(|i| i.kind == IssueKind::Duplicate)
      .expect("应报出重复项");

    apply_fix(&mut log, issue);
    assert_eq!(log.len(), 1);
    let keep = &log[0];
    // 频率丢了就再也反推不出波段与模式子段；操作员丢了会把这条算到别人头上。
    assert_eq!(keep.freq, "14.074");
    assert_eq!(keep.operator, "WXP");
    assert_eq!(keep.sota_ref, "JA/TK-001");
    assert_eq!(keep.pota_ref, "JA-0123");
    assert_eq!(keep.contest_id, "CQ-WW-CW");
    assert_eq!(keep.stx, "001");
    assert_eq!(keep.srx, "599");
    assert_eq!(keep.prop_mode, "SAT");
    assert_eq!(keep.sat_name, "AO-91");
    assert_eq!(keep.time_off, "12:34");
  }

  #[test]
  fn a_batch_of_fixes_lands_in_one_pass() {
    // 「一键修正」走批量入口：回填波段与合并重复项两件事都要落到日志上。
    let mut a = entry(1, "JA1AAA");
    a.band = "40M".into(); // 频率是 14.074 → 波段应为 20m
    let mut dup = entry(3, "JA1AAA");
    dup.band = "20m".into();
    let mut log = vec![a, dup];
    let report = check(&log, &ctx());
    assert_eq!(report.count(IssueKind::BandMismatch), 1);
    assert_eq!(report.count(IssueKind::Duplicate), 1);

    assert_eq!(
      apply_fixes(&mut log, &report.issues),
      2,
      "回填一条 + 合并删一条"
    );
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].band, "20m");
  }

  #[test]
  fn a_context_needs_a_valid_date_and_time() {
    // 构造失败要返回 `None` 而不是 panic：本 crate 编进 wasm，一次 `expect` 就是整页白屏。
    assert!(HealthContext::new("2026-10-08", "01:30", 480).is_some());
    assert!(HealthContext::new("2026-10-08", "25:00", 480).is_none());
    assert!(HealthContext::new("", "", 0).is_none());
  }

  #[test]
  fn suspicious_prefixes_are_reported() {
    let mut a = entry(1, "JA1AAA");
    a.callsign = "Q1QQQ".into(); // 前缀查不出实体
    let mut b = entry(2, "BG1AA");
    b.callsign = "  ".into(); // 空呼号
    let report = check(&[a, b], &ctx());
    assert_eq!(report.count(IssueKind::SuspiciousPrefix), 2);
    assert_eq!(report.issues[0].extra.as_deref(), Some("Q1QQQ"));
    assert_eq!(report.issues[1].extra.as_deref(), Some(""));
  }

  #[test]
  fn a_frequency_without_units_is_ignored_instead_of_guessed() {
    // 带单位的频率解析不出 → 不报越界（宁可漏报也不误报）。
    let mut e = entry(1, "JA1AAA");
    e.freq = "14.074MHz".into();
    e.band.clear();
    assert!(check(&[e], &ctx()).is_clean());
  }

  #[test]
  fn fixes_on_records_that_vanished_are_noops() {
    let log = vec![entry(1, "JA1AAA")];
    let mut e = entry(1, "JA1AAA");
    e.band = "40M".into();
    let report = check(&[e], &ctx());
    let mut empty: Vec<LogEntry> = Vec::new();
    assert_eq!(apply_fix(&mut empty, &report.issues[0]), 0);
    // 日志里只剩别的记录（id 对不上）时同样什么都不做。
    let mut others = log;
    others[0].id = 99;
    assert_eq!(apply_fix(&mut others, &report.issues[0]), 0);
    assert_eq!(others[0].band, "", "不该顺手改到别的记录");
  }

  #[test]
  fn report_counts_group_by_kind() {
    let mut a = entry(1, "JA1AAA");
    a.gridsquare.clear();
    let mut b = entry(2, "BG1AA");
    b.freq = "27.555".into();
    b.band.clear();
    let c = entry(3, "W1AW");
    let report = check(&[a, b, c], &ctx());
    assert_eq!(report.count(IssueKind::MissingGrid), 1);
    assert_eq!(report.count(IssueKind::FrequencyOutOfBand), 1);
    assert_eq!(report.count(IssueKind::Duplicate), 0);
    assert_eq!(
      report.fixable_count(),
      report.fixable_of(IssueKind::FrequencyOutOfBand)
    );
    assert!(!report.is_clean());
  }

  #[test]
  fn stamp_and_minutes_round_trip_across_midnight() {
    let (d, t) = stamp(minutes_of("2026-10-08", "00:10").expect("分钟数"));
    assert_eq!((d.as_str(), t.as_str()), ("2026-10-08", "00:10"));
    // 减去 8 小时会退到前一天。
    let (d, t) = stamp(minutes_of("2026-10-08", "03:00").expect("分钟数") - 480);
    assert_eq!((d.as_str(), t.as_str()), ("2026-10-07", "19:00"));
  }
}
