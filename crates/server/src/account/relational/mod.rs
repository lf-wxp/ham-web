//! 账号数据的多表存储：把同步用的 `{ key: value }` 快照拆进关系表，再无损地拼回来。
//!
//! 设计见 `docs/domain/account-sync.md`「数据结构」。要点：
//!
//! - **`user_kv` 是目录**：账号的每个 key 在里面恰好一行。值要么直接放在这一行（`inline`），
//!   要么在专表里（`table`，这一行只留摘要）。没有它，「`[]`（有这个 key、内容为空）」与
//!   「根本没这个 key」在专表里无法区分。
//! - **导入即规范化**：核心数据（通联、台站档案、错题…）按 `ham_web_core` 里的类型**宽松解析**
//!   （缺字段补默认值、旧字段名按别名认、多出来的未知字段丢弃），存进专表；取回时再由类型序列化成当前形态。
//!   所以任何一份合法的本地 JSON 备份——不论哪个版本的应用导出的——都能正确进表，不需要「字节与原文一致」。
//!   不进专表的只有两类：这个 key 没有专表（设置、前端私有结构……），或者值根本解析不了 / 违反表的行级约束
//!   （重复的日志编号、与 key 对不上的进度……）——它们原样放在 `user_kv`，一个字节都不丢。
//! - **摘要、指纹都对「规范化后的内容」算**：服务端取回的内容就是规范形态，所以每个 key 的摘要
//!   （[`ham_web_core::delta::Digest`]，SHA-256）是规范文本的摘要，前端拿本机内容算出的摘要才能直接与云端清单比较。
//!   整个备份的指纹（[`ham_web_core::delta::manifest_fingerprint`]）对 `(key, 摘要)` 的清单求，**不用读出任何内容**，
//!   所以只改一个 key 的增量写入也能在同一个事务里算出新指纹。规范化是幂等的（规范形态再规范化不变），不会越同步越漂。
//! - **纯逻辑与 SQL 分开**：[`Split::from_map`] / [`Split::into_map`] 不碰数据库，
//!   可以放在锁外做（解析与序列化是 CPU 重活）；[`apply`] / [`load`] 才读写表。
//! - **摘要跳过**：没变的 key 不重写（图片这类大值尤其重要）。
//! - **整份替换与按 key 增量两种写法共用同一套单 key 写入**（[`apply`] / [`apply_delta`]）：
//!   后者逐个核对「你看到的这个 key 还是不是现在这一版」，有一个对不上整批都不生效。
//! - **一张专表认领一个 key（[`Key::Exact`]）或一族 key（[`Key::Prefix`]，如 `practice:*`）**。
//!   后者每个 key 在表里是自己的一行，认领只是候选：`decode` 认不出的（如 `practice:lastMode`）照样留在目录里。
//!
//! 新增一张专表：在这个目录下写一个 [`Table`] 实现、在 [`codecs`] 登记、在 `schema/content.sql` 建表。
//! 不用改接口，也不用改前端；每张表在 `tests.rs` 里要有一条「规范值进表、原样取回」的用例。

mod cards;
mod day_records;
#[cfg(test)]
mod delta_tests;
mod exam_results;
mod exam_review;
mod groups;
mod images;
mod lists;
#[cfg(test)]
mod local_tests;
mod mistakes;
mod morse;
mod morse_pages;
mod nec;
mod notes;
mod page_state;
mod progress;
mod qsos;
mod question_stats;
mod rpg;
mod settings;
mod stations;
mod study_stats;
#[cfg(test)]
mod tables_tests;
#[cfg(test)]
mod tests;
mod workshop;
mod wrong;

use std::collections::BTreeMap;

use ham_web_core::account::Change;
#[cfg(test)]
use ham_web_core::delta::manifest_fingerprint;
use ham_web_core::delta::{Digest, Manifest, in_scope};
use ham_web_core::question::Pages;
#[cfg(test)]
use ham_web_core::remote_sync::Fingerprint;
use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// 一张专表认领哪些 key。
#[derive(Clone, Copy)]
pub(super) enum Key {
  /// 恰好这一个 key（如 `logbook`）。
  Exact(&'static str),
  /// 这个前缀下的每个 key（如 `practice:2025-10:A`）：每个 key 是表里自己的一行。
  Prefix(&'static str),
}

impl Key {
  fn matches(self, key: &str) -> bool {
    match self {
      Self::Exact(k) => k == key,
      Self::Prefix(p) => key.starts_with(p),
    }
  }
}

/// 一个落在专表里的 key。
pub(super) trait Table: Sized + Send + 'static {
  /// 认领哪些 key。
  const KEY: Key;

  /// 这张表（连同它的子表）占用的 SQL 表名，用来清掉一个账号的全部行。
  const TABLES: &'static [&'static str];

  /// 按核心类型解析；形态不对返回 `None`（这个 key 就原样留在目录里）。
  fn decode(key: &str, raw: &str) -> Option<Self>;

  /// 序列化回前端写出的文本。
  fn encode(&self) -> Option<String>;

  /// 行级约束（如自然键唯一）是否满足；不满足整个 key 退回 `user_kv` 原样存放，而不是丢数据或报错。
  fn storable(&self) -> bool {
    true
  }

  /// 写入这个 key 的全部行（调用前已 [`Table::clear`]）。父表先写、子表后写。
  fn insert(&self, tx: &Transaction<'_>, user: i64, key: &str) -> rusqlite::Result<()>;

  /// 读出这个 key 的全部行。
  fn load(conn: &Connection, user: i64, key: &str) -> rusqlite::Result<Self>;

  /// 这个 key 在库里已经是专表存放时，把它的行改成与 `self` 一致。
  ///
  /// 默认是清空再全部写入。行很多、通常只改其中几行的表（通联日志）覆盖它，只动有变化的行，
  /// 免得加一条记录就把几万行删光重写、全程占着全局写锁。
  fn replace(&self, tx: &Transaction<'_>, user: i64, key: &str) -> rusqlite::Result<()> {
    Self::clear(tx, user, key)?;
    self.insert(tx, user, key)
  }

  /// 删掉这个 key 的行。认领整张表的 key 就是清空该账号在这张表里的全部行；
  /// 认领一族 key 的表要覆盖它，只删这一个 key 的行。
  fn clear(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<()> {
    Self::clear_user(conn, user)
  }

  /// 删掉这个账号在这张表（含子表）里的全部行。
  fn clear_user(conn: &Connection, user: i64) -> rusqlite::Result<()> {
    for table in Self::TABLES {
      conn.execute(&format!("DELETE FROM {table} WHERE user_id = ?1"), [user])?;
    }
    Ok(())
  }
}

/// [`Table`] 去掉关联常量与 `Self: Sized` 之后能放进 `Box<dyn …>` 的那一半。
pub(super) trait Doc: Send {
  fn text(&self) -> Option<String>;
  fn insert(&self, tx: &Transaction<'_>, user: i64, key: &str) -> rusqlite::Result<()>;
  fn replace(&self, tx: &Transaction<'_>, user: i64, key: &str) -> rusqlite::Result<()>;
}

impl<T: Table> Doc for T {
  fn text(&self) -> Option<String> {
    self.encode()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, key: &str) -> rusqlite::Result<()> {
    <T as Table>::insert(self, tx, user, key)
  }

  fn replace(&self, tx: &Transaction<'_>, user: i64, key: &str) -> rusqlite::Result<()> {
    <T as Table>::replace(self, tx, user, key)
  }
}

/// 一种专表的「按 key 分派」入口。
struct Codec {
  key: Key,
  parse: fn(&str, &str) -> Option<Box<dyn Doc>>,
  load: fn(&Connection, i64, &str) -> rusqlite::Result<Box<dyn Doc>>,
  clear: fn(&Connection, i64, &str) -> rusqlite::Result<()>,
  clear_user: fn(&Connection, i64) -> rusqlite::Result<()>,
  /// 这张表（含子表）的 SQL 表名，用来数行数（见 [`row_count`]）。
  tables: &'static [&'static str],
}

impl Codec {
  fn of<T: Table>() -> Self {
    Self {
      key: T::KEY,
      parse: parse_as::<T>,
      load: load_as::<T>,
      clear: T::clear,
      clear_user: T::clear_user,
      tables: T::TABLES,
    }
  }
}

/// 一个 key 的原文超过这个字节数就**不展开**，原样留在目录里（仍计入账号的总量上限）。
///
/// 展开（解码成类型）会比原文大得多：实测最短合法通联 74 字节，解码后每条 744 字节（约 10 倍），
/// 规范文本还要再占一份。预算若只在「一个 key 处理完之后」累加，一份 64 MiB 的请求体就能让单个
/// 请求的峰值内存到 1 GB 以上，而上传名额有 4 个，注册又默认开放。
///
/// 取 32 MiB：真实日志每条约 600 字节，10 万条约 58 MiB——超过它的日志极罕见，且即便不展开，
/// 内容照样同步、照样不丢，只是在服务端当作一整段文本存放（取回时逐字节一致）。
/// 与 [`MAX_ROWS_PER_USER`](super::store) 的行数上限同一个出发点：把「单次请求最坏占多少」变成有界的。
const MAX_TABLE_RAW_BYTES: usize = 32 * 1024 * 1024;

fn parse_as<T: Table>(key: &str, raw: &str) -> Option<Box<dyn Doc>> {
  parse_within::<T>(key, raw, MAX_TABLE_RAW_BYTES)
}

/// [`parse_as`]，展开上限可调：测试用小值，免得为了触顶去造 32 MiB 的数据。
fn parse_within<T: Table>(key: &str, raw: &str, max_raw: usize) -> Option<Box<dyn Doc>> {
  if raw.len() > max_raw {
    return None;
  }
  // 解析得了、行级约束成立就入表；不要求重新序列化与原文逐字节相同（见模块文档）。
  let table = T::decode(key, raw)?;
  table.storable().then(|| Box::new(table) as Box<dyn Doc>)
}

fn load_as<T: Table>(conn: &Connection, user: i64, key: &str) -> rusqlite::Result<Box<dyn Doc>> {
  Ok(Box::new(T::load(conn, user, key)?))
}

/// 全部专表。新增一张表：实现 [`Table`]、在这里登记、在迁移里建表。
fn codecs() -> Vec<Codec> {
  vec![
    // 通联与台站
    Codec::of::<qsos::Qsos>(),
    Codec::of::<stations::Stations>(),
    Codec::of::<stations::StationInfoTable>(),
    Codec::of::<images::QslImages>(),
    Codec::of::<lists::QslImageIndex>(),
    // 题库学习
    Codec::of::<mistakes::Mistakes>(),
    Codec::of::<exam_results::ExamResults>(),
    Codec::of::<exam_review::ExamReviews>(),
    Codec::of::<progress::PracticeProgress>(),
    Codec::of::<progress::ExamProgress>(),
    Codec::of::<wrong::WrongQuestions>(),
    Codec::of::<study_stats::StudyStatsTable>(),
    Codec::of::<question_stats::QuestionStatsTable>(),
    Codec::of::<day_records::StudyDaily>(),
    Codec::of::<day_records::DailyChallenge>(),
    Codec::of::<lists::Bookmarks>(),
    Codec::of::<groups::BookmarkGroupsTable>(),
    Codec::of::<notes::QuestionNotes>(),
    Codec::of::<cards::CardReviews>(),
    Codec::of::<rpg::RpgStars>(),
    Codec::of::<rpg::RpgBestiary>(),
    Codec::of::<lists::AchievementsSeen>(),
    // 莫尔斯与电台
    Codec::of::<morse::KochProgressTable>(),
    Codec::of::<morse::RunnerStatsTable>(),
    Codec::of::<morse_pages::MorseStatsTable>(),
    Codec::of::<morse_pages::TrainerSettingsTable>(),
    Codec::of::<morse_pages::SendStatsTable>(),
    Codec::of::<morse_pages::MorseSettingsTable>(),
    Codec::of::<lists::DxccWantedDone>(),
    // 页面自己的小文档
    Codec::of::<page_state::Countdowns>(),
    Codec::of::<page_state::DailyCheckin>(),
    Codec::of::<page_state::ContestSessionTable>(),
    Codec::of::<page_state::GridFiltersTable>(),
    Codec::of::<page_state::ListenSettingsTable>(),
    // 工具与器材
    Codec::of::<workshop::GearMeasurementsTable>(),
    Codec::of::<workshop::DiyProgressTable>(),
    Codec::of::<workshop::NoiseSamples>(),
    Codec::of::<nec::NecDesigns>(),
    // 设置
    Codec::of::<settings::StudyPlanTable>(),
    Codec::of::<settings::DxAlerts>(),
    Codec::of::<settings::SatWatchTable>(),
  ]
}

fn find<'a>(codecs: &'a [Codec], key: &str) -> Option<&'a Codec> {
  codecs.iter().find(|c| c.key.matches(key))
}

fn corrupt(what: impl Into<String>) -> rusqlite::Error {
  rusqlite::Error::FromSqlConversionFailure(0, Type::Text, what.into().into())
}

// ───────────────────────── 各专表共用的小工具 ─────────────────────────

/// 小型嵌套文档（题面快照、位置数组……）存成 JSON 文本列：整体读写，不按字段查询。
pub(super) fn to_json<T: Serialize>(value: &T) -> rusqlite::Result<String> {
  serde_json::to_string(value).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
}

pub(super) fn from_json<T: DeserializeOwned>(raw: &str, col: usize) -> rusqlite::Result<T> {
  serde_json::from_str(raw)
    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(col, Type::Text, Box::new(e)))
}

/// 枚举存成它在 JSON 里的那个字符串（`Bank::A` → `A`，`Deck::QCode` → `qcode`）：
/// 与前端写出的文本同一个写法，不另造一套取值。
pub(super) fn enum_text<T: Serialize>(value: &T) -> rusqlite::Result<String> {
  match serde_json::to_value(value) {
    Ok(serde_json::Value::String(text)) => Ok(text),
    _ => Err(corrupt("枚举没有字符串形态")),
  }
}

pub(super) fn enum_from_text<T: DeserializeOwned>(text: &str) -> rusqlite::Result<T> {
  serde_json::from_value(serde_json::Value::String(text.to_owned()))
    .map_err(|_| corrupt(format!("认不出的取值 {text}")))
}

/// 读单行时没有这一行：目录说这个 key 在专表里，表里却没有——库被改坏了。
pub(super) fn missing(what: &str) -> impl FnOnce(rusqlite::Error) -> rusqlite::Error {
  let what = what.to_owned();
  move |e| match e {
    rusqlite::Error::QueryReturnedNoRows => {
      corrupt(format!("{what} 标为专表存放，但表里没有这一行"))
    }
    other => other,
  }
}

/// 题目快照里的 `pages`（历史字段）：有没有、起止页。`NULL` 与「有但起止都为空」是两回事。
pub(super) fn pages_cols(pages: Option<Pages>) -> (bool, Option<u32>, Option<u32>) {
  (
    pages.is_some(),
    pages.and_then(|p| p.start),
    pages.and_then(|p| p.end),
  )
}

pub(super) fn pages_of(has: bool, start: Option<u32>, end: Option<u32>) -> Option<Pages> {
  has.then_some(Pages { start, end })
}

/// `usize` / `u64` 进 SQLite 的 `INTEGER`（i64）：按位原样搬，超过 `i64::MAX` 的会变成负数但能无损还原。
pub(super) fn db_usize(n: usize) -> i64 {
  (n as u64).cast_signed()
}

pub(super) fn rust_usize(n: i64) -> usize {
  usize::try_from(n.cast_unsigned()).unwrap_or(usize::MAX)
}

/// 单个字符的键（`BTreeMap<char, _>` 的键）：库里存成长度为 1 的文本。
pub(super) fn parse_char(text: &str) -> rusqlite::Result<char> {
  let mut it = text.chars();
  match (it.next(), it.next()) {
    (Some(c), None) => Ok(c),
    _ => Err(corrupt(format!("{text:?} 不是单个字符"))),
  }
}

/// 一个 key 的值放在哪儿。
enum Body {
  /// 原文就在 `user_kv.value` 里。
  Inline(String),
  /// 在专表里；这是已解析好的行集。
  Table(Box<dyn Doc>),
}

/// 一个 key 连同它的规范文本的摘要与字节数（写入时一并记在目录里）。
struct Entry {
  body: Body,
  digest: Digest,
  /// 规范文本的字节数。
  bytes: u64,
}

impl Entry {
  /// 按 key 分派：能进专表的解析好，其余原样留着。摘要、字节数都对**规范文本**算。
  fn new(codecs: &[Codec], key: &str, raw: String) -> Self {
    let table = find(codecs, key).and_then(|c| (c.parse)(key, &raw));
    if let Some(doc) = table
      && let Some(text) = doc.text()
    {
      return Self {
        digest: Digest::of(&text),
        bytes: text.len() as u64,
        body: Body::Table(doc),
      };
    }
    Self {
      digest: Digest::of(&raw),
      bytes: raw.len() as u64,
      body: Body::Inline(raw),
    }
  }

  fn storage(&self) -> Storage {
    match self.body {
      Body::Inline(_) => Storage::Inline,
      Body::Table(_) => Storage::Table,
    }
  }
}

/// 一份快照拆开后的样子：每个 key 一个 [`Entry`]。
pub struct Split {
  entries: BTreeMap<String, Entry>,
}

impl Split {
  /// 把快照拆开：能进专表的解析好，其余原样留着。纯 CPU，不碰数据库。
  ///
  /// 不设预算，只给测试用；生产代码一律走 [`Split::from_map_within`]。
  #[cfg(test)]
  #[must_use]
  pub fn from_map(map: BTreeMap<String, String>) -> Self {
    let codecs = codecs();
    let entries = map
      .into_iter()
      .map(|(key, raw)| {
        let entry = Entry::new(&codecs, &key, raw);
        (key, entry)
      })
      .collect();
    Self { entries }
  }

  /// 同 [`Split::from_map`]，但规范化之后的总字节数一超过 `max_bytes` 就停下返回 `None`。
  ///
  /// 规范化会补齐缺省字段，拆开后的内存与存库体积都可能比原文大得多；不设预算，
  /// 一份没超过请求体上限的备份也能把内存撑到好几倍，所以边拆边数、超了立刻丢弃已拆好的部分。
  #[must_use]
  pub fn from_map_within(map: BTreeMap<String, String>, max_bytes: u64) -> Option<Self> {
    let codecs = codecs();
    let mut entries = BTreeMap::new();
    let mut total = 0u64;
    for (key, raw) in map {
      let entry = Entry::new(&codecs, &key, raw);
      total = total.saturating_add(entry.bytes);
      if total > max_bytes {
        return None;
      }
      entries.insert(key, entry);
    }
    Some(Self { entries })
  }

  /// 全部 key 的规范文本字节数之和：写入之后账号的体积。
  #[must_use]
  pub fn total_bytes(&self) -> u64 {
    self
      .entries
      .values()
      .fold(0u64, |sum, e| sum.saturating_add(e.bytes))
  }

  /// 拼回 `{ key: value }`（规范形态）。专表里的值序列化不出来时返回 `None`（不会发生，但不能悄悄丢 key）。
  #[must_use]
  pub fn into_map(self) -> Option<BTreeMap<String, String>> {
    self
      .entries
      .into_iter()
      .map(|(key, entry)| {
        let value = match entry.body {
          Body::Inline(text) => text,
          Body::Table(doc) => doc.text()?,
        };
        Some((key, value))
      })
      .collect()
  }
}

/// 一批按 key 的增量改动，拆好了、核对条件之前的样子（见 [`apply_delta`]）。
pub struct Delta {
  items: Vec<DeltaItem>,
}

struct DeltaItem {
  key: String,
  /// 调用方看到的这个 key 在云端的摘要。
  base: Option<Digest>,
  /// 新内容；`None` = 删除。
  new: Option<Entry>,
}

/// [`Delta::from_changes_within`] 为什么没拆成。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaRefused {
  /// 同一个 key 出现了两次。
  DuplicateKey,
  /// 规范化之后的新内容合计超过预算。
  TooLarge,
}

impl Delta {
  /// 把上传的改动拆开（解析 + 算规范文本的摘要，纯 CPU，放在锁外做），不设预算，只给测试用。
  ///
  /// 同一个 key 出现两次返回 `None`：基线核对与写入用的都是事务开头的快照，
  /// 第二条看不到第一条写了什么，没有一个说得清的结果。
  #[cfg(test)]
  #[must_use]
  pub fn from_changes(changes: Vec<Change>) -> Option<Self> {
    Self::from_changes_within(changes, u64::MAX).ok()
  }

  /// 同 [`Delta::from_changes`]，但规范化之后的新内容合计一超过 `max_bytes` 就停下。
  ///
  /// 与 [`Split::from_map_within`] 同一个理由：规范化会让内存放大好几倍，推送批和整份上传一样
  /// 必须边拆边数；否则一份没超过请求体上限的推送就能撑爆内存。
  ///
  /// # Errors
  ///
  /// [`DeltaRefused`]：重复的 key，或合计超过预算。
  pub fn from_changes_within(changes: Vec<Change>, max_bytes: u64) -> Result<Self, DeltaRefused> {
    let mut seen = std::collections::HashSet::new();
    if !changes.iter().all(|c| seen.insert(c.key.as_str())) {
      return Err(DeltaRefused::DuplicateKey);
    }
    drop(seen);
    let codecs = codecs();
    let mut total = 0u64;
    let mut items = Vec::with_capacity(changes.len());
    for Change { key, base, value } in changes {
      let new = value.map(|raw| Entry::new(&codecs, &key, raw));
      total = total.saturating_add(new.as_ref().map_or(0, |e| e.bytes));
      if total > max_bytes {
        return Err(DeltaRefused::TooLarge);
      }
      items.push(DeltaItem { key, base, new });
    }
    Ok(Self { items })
  }
}

/// 写一个 key：值进专表或放在目录里，目录那一行记下存放方式、摘要、字节数。
/// 摘要与存放方式都没变就跳过。
fn write_key(
  tx: &Transaction<'_>,
  codecs: &[Codec],
  user: i64,
  key: &str,
  entry: &Entry,
  prev: Option<&(Storage, Vec<u8>)>,
) -> rusqlite::Result<()> {
  let digest = entry.digest.as_bytes().as_slice();
  if prev.is_some_and(|(s, d)| *s == entry.storage() && d.as_slice() == digest) {
    return Ok(());
  }
  let bytes = entry.bytes.cast_signed();
  match &entry.body {
    Body::Inline(value) => {
      if prev.is_some_and(|(s, _)| *s == Storage::Table) {
        clear_typed(codecs, tx, user, key)?;
      }
      tx.prepare_cached(
        "INSERT INTO user_kv (user_id, key, value, storage, digest, bytes)
         VALUES (?1, ?2, ?3, 'inline', ?4, ?5)
         ON CONFLICT(user_id, key) DO UPDATE SET
           value = excluded.value, storage = 'inline', digest = excluded.digest,
           bytes = excluded.bytes",
      )?
      .execute(params![user, key, value, digest, bytes])?;
    }
    Body::Table(doc) => {
      if prev.is_some_and(|(s, _)| *s == Storage::Table) {
        // 原来就在专表里：交给表自己决定怎么改（行很多的表只动有变化的行）。
        doc.replace(tx, user, key)?;
      } else {
        // 目录里没有、或原来是原样存放：专表里本不该有这个 key 的行，清一遍只是保险。
        clear_typed(codecs, tx, user, key)?;
        doc.insert(tx, user, key)?;
      }
      tx.prepare_cached(
        "INSERT INTO user_kv (user_id, key, value, storage, digest, bytes)
         VALUES (?1, ?2, NULL, 'table', ?3, ?4)
         ON CONFLICT(user_id, key) DO UPDATE SET
           value = NULL, storage = 'table', digest = excluded.digest, bytes = excluded.bytes",
      )?
      .execute(params![user, key, digest, bytes])?;
    }
  }
  Ok(())
}

/// 删一个 key（目录那一行连同专表里的行）。
fn remove_key(
  tx: &Transaction<'_>,
  codecs: &[Codec],
  user: i64,
  key: &str,
  prev: Option<&(Storage, Vec<u8>)>,
) -> rusqlite::Result<()> {
  if prev.is_some_and(|(s, _)| *s == Storage::Table) {
    clear_typed(codecs, tx, user, key)?;
  }
  tx.execute(
    "DELETE FROM user_kv WHERE user_id = ?1 AND key = ?2",
    params![user, key],
  )?;
  Ok(())
}

/// 把拆好的快照写进这个账号名下，让库里的内容**恰好**等于它：缺的 key 删掉，没变的跳过。
///
/// # Errors
///
/// 数据库错误；调用方的事务会整体回滚。
pub(super) fn apply(tx: &Transaction<'_>, user: i64, split: &Split) -> rusqlite::Result<()> {
  let codecs = codecs();
  let old = catalog(tx, user)?;
  for (key, prev) in &old {
    if !split.entries.contains_key(key) {
      remove_key(tx, &codecs, user, key, Some(prev))?;
    }
  }
  for (key, entry) in &split.entries {
    write_key(tx, &codecs, user, key, entry, old.get(key))?;
  }
  Ok(())
}

/// [`apply_delta`] 的结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DeltaOutcome {
  /// 全部生效。
  Applied,
  /// 至少一个 key 在云端已经不是调用方看到的那一版：整批都没生效。
  Conflict,
  /// 写完之后账号的内容会超过体积上限：整批都没写。
  TooLarge,
}

/// 按 key 增量写入：每个 key 先核对「调用方看到的摘要」等于云端现在的摘要，全部对得上才写。
///
/// 核对与写入在调用方的同一个事务里，所以两台设备同时改同一个 key，只有一个能成功。
/// **返回非 `Applied` 时没有写过任何东西**（核对与体积预估都在动手之前），调用方不提交即可。
/// 写入之后账号的体积按「现有总量 − 涉及 key 的旧字节 + 新字节」预估；[`Delta::from_changes`]
/// 已保证同一个 key 不会出现两次，所以每个 key 的旧字节只减一次。
///
/// # Errors
///
/// 数据库错误。
pub(super) fn apply_delta(
  tx: &Transaction<'_>,
  user: i64,
  delta: &Delta,
  max_bytes: u64,
  max_rows: i64,
) -> rusqlite::Result<DeltaOutcome> {
  let codecs = codecs();
  let old = catalog(tx, user)?;
  let digest_of = |key: &str| {
    old
      .get(key)
      .and_then(|(_, d)| <[u8; 32]>::try_from(d.as_slice()).ok())
      .map(Digest::from_bytes)
  };
  if delta.items.iter().any(|i| digest_of(&i.key) != i.base) {
    return Ok(DeltaOutcome::Conflict);
  }
  let mut projected = total_bytes(tx, user)?;
  for item in &delta.items {
    projected = projected
      .saturating_sub(stored_bytes(tx, user, &item.key)?)
      .saturating_add(item.new.as_ref().map_or(0, |e| e.bytes));
  }
  if projected > max_bytes {
    return Ok(DeltaOutcome::TooLarge);
  }
  for item in &delta.items {
    match &item.new {
      Some(entry) => write_key(tx, &codecs, user, &item.key, entry, old.get(&item.key))?,
      None => remove_key(tx, &codecs, user, &item.key, old.get(&item.key))?,
    }
  }
  // 行数在事务里核对：超了就让调用方不提交（等于一行没写）。
  if row_count(tx, user)? > max_rows {
    return Ok(DeltaOutcome::TooLarge);
  }
  Ok(DeltaOutcome::Applied)
}

/// 目录里这个 key 现在记的字节数；没有为 0。
fn stored_bytes(conn: &Connection, user: i64, key: &str) -> rusqlite::Result<u64> {
  Ok(
    conn
      .prepare_cached("SELECT bytes FROM user_kv WHERE user_id = ?1 AND key = ?2")?
      .query_row(params![user, key], |r| r.get::<_, i64>(0))
      .optional()?
      .map_or(0, i64::cast_unsigned),
  )
}

/// 读出这个账号的全部数据。
///
/// # Errors
///
/// 数据库错误；目录与专表对不上（库被改坏）。
pub(super) fn load(conn: &Connection, user: i64) -> rusqlite::Result<Split> {
  load_where(conn, user, None)
}

/// 只读出指定的 key（没有的跳过）。
///
/// # Errors
///
/// 数据库错误；目录与专表对不上（库被改坏）。
pub(super) fn load_some(conn: &Connection, user: i64, keys: &[String]) -> rusqlite::Result<Split> {
  load_where(conn, user, Some(keys))
}

/// `user_kv` 的一行。
struct CatalogRow {
  key: String,
  value: Option<String>,
  storage: String,
  digest: Vec<u8>,
  bytes: i64,
}

const CATALOG_COLUMNS: &str = "key, value, storage, digest, bytes";

fn catalog_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<CatalogRow> {
  Ok(CatalogRow {
    key: r.get(0)?,
    value: r.get(1)?,
    storage: r.get(2)?,
    digest: r.get(3)?,
    bytes: r.get(4)?,
  })
}

/// 目录的一行 → 带内容的 [`Entry`]（专表里的值在这里读出来）。
fn entry_of(
  conn: &Connection,
  codecs: &[Codec],
  user: i64,
  row: CatalogRow,
) -> rusqlite::Result<(String, Entry)> {
  let CatalogRow {
    key,
    value,
    storage,
    digest,
    bytes,
  } = row;
  let body = match (value, storage.as_str()) {
    (Some(text), "inline") => Body::Inline(text),
    (None, "table") => {
      let codec = find(codecs, &key)
        .ok_or_else(|| corrupt(format!("key {key} 标为专表存放，但本程序没有对应的表")))?;
      Body::Table((codec.load)(conn, user, &key)?)
    }
    _ => return Err(corrupt(format!("key {key} 的存放方式与取值对不上"))),
  };
  let digest = <[u8; 32]>::try_from(digest.as_slice())
    .map(Digest::from_bytes)
    .map_err(|_| corrupt(format!("key {key} 的摘要不是 32 字节")))?;
  let entry = Entry {
    body,
    digest,
    bytes: bytes.cast_unsigned(),
  };
  Ok((key, entry))
}

/// 读出账号的 key：`only` 为 `None` 是全部（一次查询），否则只读指定的（逐个按主键取）。
///
/// 指定 key 时**不扫全表**：这条读取在 `Store::with` 的全局锁里，账号里有几十 MB 的原样值时，
/// 为了取一个 key 把它们全部读进内存，会拖慢所有人的会话校验。
fn load_where(conn: &Connection, user: i64, only: Option<&[String]>) -> rusqlite::Result<Split> {
  let codecs = codecs();
  let mut entries = BTreeMap::new();
  match only {
    None => {
      let rows: Vec<CatalogRow> = conn
        .prepare_cached(&format!(
          "SELECT {CATALOG_COLUMNS} FROM user_kv WHERE user_id = ?1"
        ))?
        .query_map([user], catalog_row)?
        .collect::<rusqlite::Result<_>>()?;
      for row in rows {
        let (key, entry) = entry_of(conn, &codecs, user, row)?;
        entries.insert(key, entry);
      }
    }
    Some(keys) => {
      for key in keys {
        let row = conn
          .prepare_cached(&format!(
            "SELECT {CATALOG_COLUMNS} FROM user_kv WHERE user_id = ?1 AND key = ?2"
          ))?
          .query_row(params![user, key], catalog_row)
          .optional()?;
        if let Some(row) = row {
          let (key, entry) = entry_of(conn, &codecs, user, row)?;
          entries.insert(key, entry);
        }
      }
    }
  }
  Ok(Split { entries })
}

/// 云端的清单：参与同步的 key 的摘要（范围见 [`ham_web_core::delta::in_scope`]）。读目录，不读任何内容。
///
/// # Errors
///
/// 数据库错误；摘要不是 32 字节（库被改坏）。
pub(super) fn manifest(conn: &Connection, user: i64) -> rusqlite::Result<Manifest> {
  let mut out = Manifest::new();
  for (key, (_, digest)) in catalog(conn, user)? {
    // 范围只有一处定义（`delta::in_scope`），服务端与客户端对「哪些 key 参与同步」不可能各说各话。
    // 服务端存影像，所以总是按「带影像」的范围算；不同步影像的客户端自己再收窄。
    if !in_scope(&key, true) {
      continue;
    }
    let digest = <[u8; 32]>::try_from(digest.as_slice())
      .map(Digest::from_bytes)
      .map_err(|_| corrupt(format!("key {key} 的摘要不是 32 字节")))?;
    out.insert(key, digest);
  }
  Ok(out)
}

/// 云端内容的指纹：对清单求，见 [`manifest_fingerprint`]。
///
/// 生产代码已经拿着清单（回执要带），直接对它求指纹，不再读第二遍目录；这个函数只给测试核对用。
///
/// # Errors
///
/// 同 [`manifest`]。
#[cfg(test)]
pub(super) fn fingerprint(conn: &Connection, user: i64) -> rusqlite::Result<Fingerprint> {
  Ok(manifest_fingerprint(&manifest(conn, user)?))
}

/// 这个账号全部 key 的规范文本字节数之和。
///
/// # Errors
///
/// 数据库错误。
pub(super) fn total_bytes(conn: &Connection, user: i64) -> rusqlite::Result<u64> {
  conn
    .query_row(
      "SELECT COALESCE(SUM(bytes), 0) FROM user_kv WHERE user_id = ?1",
      [user],
      |r| r.get::<_, i64>(0),
    )
    .map(i64::cast_unsigned)
}

/// 这个账号在全部专表里有多少行。
///
/// 体积上限挡不住行数：`question-stats` 这种每行几十字节的 key，64 MiB 能展开成上百万行，
/// 逐行 INSERT 会把全局写锁占上十几秒。调用方在**事务里**用它核对总量，超了就回滚。
pub(super) fn row_count(conn: &Connection, user: i64) -> rusqlite::Result<i64> {
  let mut total: i64 = 0;
  for codec in codecs() {
    for table in codec.tables {
      // 表名来自各表的 `TABLES` 常量，不是外部输入。
      total += conn.query_row(
        &format!("SELECT COUNT(*) FROM {table} WHERE user_id = ?1"),
        [user],
        |r| r.get::<_, i64>(0),
      )?;
    }
  }
  Ok(total)
}

/// 清空这个账号的全部数据（目录与所有专表）。
///
/// # Errors
///
/// 数据库错误。
pub(super) fn clear_all(conn: &Connection, user: i64) -> rusqlite::Result<()> {
  for codec in codecs() {
    (codec.clear_user)(conn, user)?;
  }
  conn.execute("DELETE FROM user_kv WHERE user_id = ?1", [user])?;
  Ok(())
}

fn clear_typed(codecs: &[Codec], conn: &Connection, user: i64, key: &str) -> rusqlite::Result<()> {
  match find(codecs, key) {
    Some(codec) => (codec.clear)(conn, user, key),
    None => Ok(()),
  }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Storage {
  Inline,
  Table,
}

/// 这个账号现有的目录：key → （存放方式，摘要）。
fn catalog(conn: &Connection, user: i64) -> rusqlite::Result<BTreeMap<String, (Storage, Vec<u8>)>> {
  conn
    .prepare_cached("SELECT key, storage, digest FROM user_kv WHERE user_id = ?1")?
    .query_map([user], |r| {
      let storage: String = r.get(1)?;
      let storage = match storage.as_str() {
        "inline" => Storage::Inline,
        "table" => Storage::Table,
        other => return Err(corrupt(format!("未知的存放方式 {other}"))),
      };
      Ok((r.get(0)?, (storage, r.get(2)?)))
    })?
    .collect()
}
