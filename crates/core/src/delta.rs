//! 按 key 的增量同步（在线账号通道）：摘要、清单、基线，以及「谁该取回、谁该上传、谁需要合并」的三方对比。
//!
//! 纯逻辑，不碰网络和存储，所以前端（wasm）与服务端用的是同一份定义，也能在原生环境里穷举测试。
//! 把这些步骤串成一次完整同步的是 [`crate::delta_sync`]。
//!
//! # 一条记录就是一个 key
//!
//! 同步内容是 `{ key: 文本 }`，一个 key 是一条有稳定身份的记录：`bookmarks`、`mistake-book`、
//! 每一张 `qsl-image:<编号>`、每个 `practice:{版本}:{题库}` 进度……各自独立。旧协议每次传整份快照，
//! 一个 key 变了就要把几十 MB 的 QSL 影像重传一遍，两台设备各改各的也会被判成「冲突」。
//! 现在每个 key 有一个**摘要**（内容的 SHA-256），云端给出全部 key 的清单，双方只交换有变化的那几条。
//!
//! # 基线是一对摘要
//!
//! 服务端存的是**规范化**之后的文本（补默认值、认旧字段名、丢掉不认识的字段），所以老版本写出的内容，
//! 本机算的摘要与云端清单里的摘要永远对不上。如果把「本机与云端摘要不同」当成「有改动」，
//! 这类 key 会每次同步都被重传；把规范文本写回本机又会让比服务端新的客户端丢字段。
//!
//! 所以每个 key 的基线记**两个**摘要（[`Synced`]）：上次同步成功时云端那份、和当时本机那份。
//! 「本机有没有改」只拿本机现在的摘要与**本机当时的摘要**比；「云端有没有改」只拿云端现在的摘要
//! 与**云端当时的摘要**比。两边内容在语义上等价而字节不同，不影响这两个判断。
//!
//! # 三方对比
//!
//! | 本机 vs 云端 | 本机改了 | 云端改了 | 结果 |
//! | --- | --- | --- | --- |
//! | 摘要一样 | | | 不用动 |
//! | | 没改 | 没改 | 不用动（等价但字节不同） |
//! | | 没改 | 改了 | 取回云端的（含云端删除） |
//! | | 改了 | 没改 | 上传本机的（含本机删除） |
//! | | 改了 | 改了 | 两边都改了：按 [`Resolution`] 处理 |
//!
//! 没有基线（从没同步过、换了账号）等于基线是空的：本机独有的 key 上传，云端独有的取回，
//! 两边都有而内容不同的才算「都改了」。不需要墓碑——删除是「基线里有、现在没有」。
//!
//! # 删除不一定传播
//!
//! 「本机没有了」可能是用户删的，也可能是浏览器清了站点数据、IndexedDB 被回收。对设置、进度、影像这类
//! 重来成本低或用户明确会删的内容，删除照传；对日志、错题、统计这类**积累出来的数据**，一边没有、另一边没动，
//! 一律当作「丢了，补回来」而不是「该删」（[`deletion_propagates`]）。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest as _, Sha256};

use crate::account::SYNC_MAX_KEY_LEN;
use crate::remote_sync::{Fingerprint, fingerprint_pairs, is_device_local_key};

/// 一个 key 的内容摘要（SHA-256）。线上写成 64 位小写十六进制。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest([u8; 32]);

impl Digest {
  /// 一段内容的摘要。
  #[must_use]
  pub fn of(text: &str) -> Self {
    Self(Sha256::digest(text.as_bytes()).into())
  }

  /// 从 32 个字节还原（服务端存库用）。
  #[must_use]
  pub const fn from_bytes(bytes: [u8; 32]) -> Self {
    Self(bytes)
  }

  /// 32 个字节。
  #[must_use]
  pub const fn as_bytes(&self) -> &[u8; 32] {
    &self.0
  }

  /// 小写十六进制。
  #[must_use]
  pub fn to_hex(&self) -> String {
    use fmt::Write as _;
    self.0.iter().fold(String::with_capacity(64), |mut out, b| {
      let _ = write!(out, "{b:02x}");
      out
    })
  }

  /// 从十六进制还原；长度或字符不对返回 `None`。
  #[must_use]
  pub fn from_hex(text: &str) -> Option<Self> {
    if text.len() != 64 || !text.is_ascii() {
      return None;
    }
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
      *byte = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(Self(out))
  }
}

impl fmt::Debug for Digest {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "Digest({})", &self.to_hex()[..12])
  }
}

impl Serialize for Digest {
  fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&self.to_hex())
  }
}

impl<'de> Deserialize<'de> for Digest {
  fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
    let text = String::deserialize(d)?;
    Self::from_hex(&text).ok_or_else(|| D::Error::custom("摘要必须是 64 位十六进制"))
  }
}

/// 全部同步 key 的清单：`key → 摘要`。
pub type Manifest = BTreeMap<String, Digest>;

/// 上次同步成功时，一个 key 两侧各是什么样（见模块文档「基线是一对摘要」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Synced {
  /// 云端那份的摘要（服务端规范化之后的）。
  #[serde(rename = "r")]
  pub remote: Digest,
  /// 本机那份的摘要（本机自己写的文本，没有被规范化）。
  #[serde(rename = "l")]
  pub local: Digest,
}

/// 每个 key 的基线。没有记录的 key 等于「上次同步时两边都没有它」。
pub type Base = BTreeMap<String, Synced>;

/// 这个 key 参与增量同步吗。
///
/// 不参与的：
/// - 设备专属的 key（主题、语言、接口缓存……见 [`is_device_local_key`]）；
/// - `qsl-image-index`：它只是「本机有哪些影像」的索引，由影像本身推出来。各设备带的影像集合可以不同
///   （账号通道可以选择不同步影像），把它当成可同步的内容，不同的设备会互相覆盖对方的索引，永远停不下来；
/// - 名字超过 [`SYNC_MAX_KEY_LEN`] 字节的：服务端不收，留着只会让每次上传都被拒绝；
/// - 没勾选同步 QSL 影像时的 `qsl-image:*`。
#[must_use]
pub fn in_scope(key: &str, include_images: bool) -> bool {
  !is_device_local_key(key)
    && key != crate::qsl_image::INDEX_KEY
    && key.len() <= SYNC_MAX_KEY_LEN
    && (include_images || !is_image(key))
}

/// 是不是一张 QSL 影像。
#[must_use]
pub fn is_image(key: &str) -> bool {
  key.starts_with(crate::qsl_image::KEY_PREFIX)
}

/// 一份同步内容的清单。只算参与增量同步的 key（[`in_scope`]）。
#[must_use]
pub fn manifest_of(map: &BTreeMap<String, String>, include_images: bool) -> Manifest {
  map
    .iter()
    .filter(|(key, _)| in_scope(key, include_images))
    .map(|(key, value)| (key.clone(), Digest::of(value)))
    .collect()
}

/// 清单的指纹：整个云端备份的「版本号」，`ETag` / `If-Match` 比的就是它。
///
/// 对 `(key, 摘要)` 求指纹而不是对内容：服务端不必为了回答「现在是哪一版」读出全部内容，
/// 目录里的摘要就够。键按字节序，与出现顺序无关。
///
/// 长度前缀只消除「`("a", "bc")` 与 `("ab", "c")` 拼成同一串」这种**边界歧义**，并不解决哈希碰撞：
/// 64 位非密码学哈希可以被构造出碰撞。这里够用，因为它只当版本号（防「两台设备同时写」，
/// 不防伪造），而真正的数据完整性由每个 key 的 SHA-256 摘要保证——碰撞一个指纹只能换来一次
/// 被 `412` 挡下的写入，改不动内容。
#[must_use]
pub fn manifest_fingerprint(manifest: &Manifest) -> Fingerprint {
  fingerprint_pairs(manifest.iter().map(|(k, d)| (k.as_str(), d.to_hex())))
}

// ───────────────────────── 两边都改了怎么办 ─────────────────────────

/// 一个 key 两边都改了时的处理方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
  /// 有安全的合并规则（并集 / 去重 / 取最优，且重复合并结果不变）：取回云端的做合并，再把结果传上去。
  Merge,
  /// 设置、续答位置这类**只有最新一份有意义**的内容：以本机为准（谁后同步谁生效），不打扰用户。
  LastWriterWins,
  /// 累计计数、日志、列表等没有安全规则的内容：停下来问用户。合并会重复累加，任选一边会丢另一边的积累。
  Block,
}

/// 「前缀名下全是断点 / 偏好」的命名空间（形态见 `crate::saved_state`）。
///
/// 单独列出来是为了让「新前缀」和「前缀下新增累计型 key」都要改到这里、被复审看见，
/// 而不是靠 `starts_with` 散在匹配分支里。
const LWW_PREFIXES: &[&str] = &["practice:", "exam:"];

/// 明确归过类的 key 的处理方式；没归过类返回 `None`。
///
/// 新增本地存储 key 时必须在这里归类（有测试逼着）：漏掉的 key 会落到 [`Resolution::Block`]，
/// 宁可多问用户一次，也不静默地覆盖或重复累加别人的积累。
///
/// 通联日志与 QSL 影像是 [`Resolution::Block`]：影像靠通联编号挂在日志上，而编号是各设备本地自增的，
/// 两边各记了一条编号相同的不同通联，合并时要重新编号并把影像跟着挪——这件事有现成的手动「合并」，
/// 自动同步遇到就停下来请用户点它，不在后台悄悄做。
#[must_use]
pub fn classify(key: &str) -> Option<Resolution> {
  use Resolution::{Block, LastWriterWins, Merge};
  if key == "logbook" || is_image(key) {
    return Some(Block);
  }
  match key {
    // 与 `backup_merge::merge_value` 的合并规则一一对应，且重复合并结果不变。
    "station-book" | "bookmarks" | "dxcc_wanted_done" | "exam-history" | "mistake-book"
    | "daily-challenge" | "rpg-stars" | "rpg-bestiary" => Some(Merge),
    // 设置、偏好、最近一次的状态：只有最新的有意义。
    "station-info"
    | "contest-session"
    | "grid-map-filters"
    | "listen-settings"
    | "morse-settings"
    | "morse-trainer"
    | "learning-plan"
    | "dx-alerts"
    | "sat-watch"
    | "qsl-label-layout"
    | "qsl-card-template"
    | "qsl-card-imposition"
    | "log-stats-scope"
    | "study-reminder-last"
    | "mistake-book:seeded"
    | "study-stats:seen-seeded" => Some(LastWriterWins),
    // 练习 / 考试的断点、考试复盘、本次错题、各类显示偏好都带 `practice:` / `exam:` 前缀
    // （形态见 `crate::saved_state`）。这些前缀下**只放「最新的一份有意义」的数据**：
    // 往里面加累计型的东西（统计、列表）必须先在本函数加一条规则，否则会被当成本机覆盖云端。
    _ if LWW_PREFIXES.iter().any(|p| key.starts_with(p)) => Some(LastWriterWins),
    // 累计计数与列表：没有安全的合并规则。
    "study-stats"
    | "question-stats"
    | "study-daily"
    | "card-review"
    | "question-notes"
    | "bookmark-groups"
    | "countdowns"
    | "daily-checkin"
    | "achievements-seen"
    | "morse-stats"
    | "morse-koch"
    | "morse-runner"
    | "morse-send"
    | "gear-measurements"
    | "diy-progress"
    | "noise-floor-samples"
    | "nec-designs" => Some(Block),
    _ => None,
  }
}

/// 一个 key 两边都改了时的处理方式（没归过类的按 [`Resolution::Block`]）。
#[must_use]
pub fn resolution(key: &str) -> Resolution {
  classify(key).unwrap_or(Resolution::Block)
}

/// 一边删了这个 key、另一边没动时，删除要不要传给另一边（见模块文档「删除不一定传播」）。
///
/// 传播的：设置与进度（用完就删是它们的正常生命周期）、QSL 影像（用户会删掉一张扫描件）。
/// 不传播的：日志、错题、统计等积累出来的数据，以及没归过类的 key——没有就补回来。
#[must_use]
pub fn deletion_propagates(key: &str) -> bool {
  is_image(key) || classify(key) == Some(Resolution::LastWriterWins)
}

// ───────────────────────── 三方对比 ─────────────────────────

/// 对一个 key 要做的事。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
  /// 不用动：两边一样，或等价（见模块文档）。
  Same,
  /// 云端有改动、本机没动：取回。
  Pull,
  /// 云端删了、本机没动：本机也删。
  PullDelete,
  /// 本机有改动、云端没动：上传。
  Push,
  /// 本机删了、云端没动：把云端的也删掉。
  PushDelete,
  /// 两边都改了，有安全的合并规则：取回云端的做合并，再上传合并结果。
  Merge,
  /// 两边都改了，按「后同步的为准」：以本机为准上传。
  KeepLocal,
  /// 两边都改了，没有安全规则：停下来问用户。
  Conflict,
}

/// 一次对比的结果。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan(BTreeMap<String, Action>);

impl Plan {
  /// 某个 key 要做的事（没出现过的是 [`Action::Same`]）。
  #[must_use]
  pub fn action(&self, key: &str) -> Action {
    self.0.get(key).copied().unwrap_or(Action::Same)
  }

  /// 需要动手的 key（不含 [`Action::Same`]）。
  pub fn iter(&self) -> impl Iterator<Item = (&str, Action)> {
    self.0.iter().map(|(k, a)| (k.as_str(), *a))
  }

  /// 具有某种动作的 key。
  #[must_use]
  pub fn keys(&self, wanted: impl Fn(Action) -> bool) -> Vec<String> {
    self
      .0
      .iter()
      .filter(|(_, a)| wanted(**a))
      .map(|(k, _)| k.clone())
      .collect()
  }

  /// 要停下来问用户的 key。
  #[must_use]
  pub fn conflicts(&self) -> Vec<String> {
    self.keys(|a| a == Action::Conflict)
  }

  /// 把「本来要往外传」的 key 改回 [`Action::Conflict`]。
  ///
  /// 用于跨计划保留判断：落盘之后会重新对比一次，那时日志已经两边一样、联动规则不再触发，
  /// 但「这批影像先别传」的理由（编号可能撞）并没有消失。
  pub fn hold_as_conflict(&mut self, keys: impl IntoIterator<Item = String>) {
    for key in keys {
      if matches!(
        self.0.get(&key),
        Some(Action::Push | Action::PushDelete | Action::KeepLocal | Action::Merge)
      ) {
        self.0.insert(key, Action::Conflict);
      }
    }
  }

  /// 没有任何事要做。
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.0.is_empty()
  }

  /// 要删的 key（两个方向）。
  #[must_use]
  pub fn deletions(&self) -> Vec<String> {
    self.keys(|a| matches!(a, Action::PullDelete | Action::PushDelete))
  }

  /// 要改写本机数据的动作（取回、本机删除、合并）：这些动作要拿备份锁。
  #[must_use]
  pub fn writes_local(&self) -> bool {
    self
      .0
      .values()
      .any(|a| matches!(a, Action::Pull | Action::PullDelete | Action::Merge))
  }
}

/// 三方对比。
///
/// `prefer_remote`：本机是全新的（没有用户数据，有的只是应用自己写的默认值）。两边都有而不同的
/// key 一律以云端为准——默认值不是用户的改动，不该因此去问用户或盖掉云端。
#[must_use]
pub fn plan(base: &Base, local: &Manifest, remote: &Manifest, prefer_remote: bool) -> Plan {
  let keys: BTreeSet<&String> = base
    .keys()
    .chain(local.keys())
    .chain(remote.keys())
    .collect();
  let mut out = BTreeMap::new();
  for key in keys {
    let action = decide(
      key,
      base.get(key),
      local.get(key),
      remote.get(key),
      prefer_remote,
    );
    if action != Action::Same {
      out.insert(key.clone(), action);
    }
  }
  hold_images_with_the_logbook(&mut out);
  Plan(out)
}

/// 影像靠通联编号挂在日志上，而编号是各设备本地自增的：两边各记了一条编号相同的不同通联，
/// 本机那张编号相同的影像传上去，就会挂到云端的另一条通联上。
///
/// 所以日志这一边**不是**「只有本机改了」（取回云端的、冲突、要合并）时，本机要往外传的影像一律先停下来，
/// 交给用户处理（手动「合并」会按通联身份重新编号再挪影像）。往里取的影像不受影响——它们跟着云端的日志走。
///
/// 停下来用的是 [`Action::Conflict`]（影像本来就是 [`Resolution::Block`]），但引擎会把**影像的**
/// 冲突从「整轮什么都别动」里摘出来：一张本地影像不该让其它 key 的取回与上传全部停摆。
fn hold_images_with_the_logbook(out: &mut BTreeMap<String, Action>) {
  let moving = matches!(
    out.get("logbook"),
    Some(Action::Pull | Action::PullDelete | Action::Conflict | Action::Merge)
  );
  if !moving {
    return;
  }
  for (key, action) in out.iter_mut() {
    // 影像恒为 `Block`，这里只可能出现「往外传」的两种动作。
    if is_image(key) && matches!(action, Action::Push | Action::PushDelete) {
      *action = Action::Conflict;
    }
  }
}

fn decide(
  key: &str,
  base: Option<&Synced>,
  local: Option<&Digest>,
  remote: Option<&Digest>,
  prefer_remote: bool,
) -> Action {
  let local_changed = local != base.map(|s| &s.local);
  let remote_changed = remote != base.map(|s| &s.remote);
  let pull = if remote.is_some() {
    Action::Pull
  } else {
    Action::PullDelete
  };
  let push = if local.is_some() {
    Action::Push
  } else {
    Action::PushDelete
  };
  let action = if local == remote || (!local_changed && !remote_changed) {
    Action::Same
  } else if !local_changed {
    pull
  } else if !remote_changed {
    push
  } else if prefer_remote {
    pull
  } else {
    match (resolution(key), local, remote) {
      (Resolution::Merge, Some(_), Some(_)) => Action::Merge,
      (Resolution::LastWriterWins, _, _) => Action::KeepLocal,
      _ => Action::Conflict,
    }
  };
  // 积累出来的数据：一边没有、另一边没动，补回来而不是删。
  match action {
    Action::PullDelete if !deletion_propagates(key) => Action::Push,
    Action::PushDelete if !deletion_propagates(key) => Action::Pull,
    other => other,
  }
}

/// 手动「上传」成功之后的基线：上传是整份替换，云端现在恰好等于本机，
/// 所以每个两边都有的 key，云端那份是回执里的摘要、本机那份是上传的文本的摘要。
#[must_use]
pub fn base_after_push(local: &Manifest, remote_after: &Manifest) -> Base {
  local
    .iter()
    .filter_map(|(key, l)| {
      remote_after.get(key).map(|r| {
        (
          key.clone(),
          Synced {
            remote: *r,
            local: *l,
          },
        )
      })
    })
    .collect()
}

/// 手动「恢复 / 合并」之后的基线：每个云端有、本机现在也有的 key，都记成「两边当时都停在云端那一版」。
///
/// 这样恢复之后本机里和云端不一样的部分（合并出的结果、本机独有的改动）下一轮都是「本机改了、云端没动」，
/// 会被传上去；而不是被当成「两边都改了」（累计类的 key 会因此卡在冲突上）。
/// 本机没有的（导入时跳过了）不记，下一轮仍然当成云端的新内容。
#[must_use]
pub fn base_after_pull(local_now: &Manifest, remote: &Manifest) -> Base {
  remote
    .iter()
    .filter(|(key, _)| local_now.contains_key(*key))
    .map(|(key, r)| {
      (
        key.clone(),
        Synced {
          remote: *r,
          local: *r,
        },
      )
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn m(pairs: &[(&str, &str)]) -> Manifest {
    pairs
      .iter()
      .map(|(k, v)| ((*k).to_owned(), Digest::of(v)))
      .collect()
  }

  /// 基线里一个 key：云端那份与本机那份。
  fn synced(remote: &str, local: &str) -> Synced {
    Synced {
      remote: Digest::of(remote),
      local: Digest::of(local),
    }
  }

  /// 单个 key 的三种状态：基线（云端那份，本机那份）、本机现在、云端现在。
  fn one(
    base: Option<(&str, &str)>,
    local: Option<&str>,
    remote: Option<&str>,
    key: &str,
  ) -> Action {
    let base: Base = base
      .map(|(r, l)| [(key.to_owned(), synced(r, l))].into())
      .unwrap_or_default();
    let side = |v: Option<&str>| -> Manifest { v.map(|v| m(&[(key, v)])).unwrap_or_default() };
    plan(&base, &side(local), &side(remote), false).action(key)
  }

  #[test]
  fn a_digest_is_sha256_in_lowercase_hex_and_round_trips() {
    // 已知向量：空串与 "abc" 的 SHA-256。
    assert_eq!(
      Digest::of("").to_hex(),
      "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
      Digest::of("abc").to_hex(),
      "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let d = Digest::of("你好");
    assert_eq!(Digest::from_hex(&d.to_hex()), Some(d));
    assert_eq!(Digest::from_bytes(*d.as_bytes()), d);
    let json = serde_json::to_string(&d).unwrap();
    assert_eq!(json, format!("\"{}\"", d.to_hex()));
    assert_eq!(serde_json::from_str::<Digest>(&json).unwrap(), d);
  }

  #[test]
  fn malformed_hex_is_rejected() {
    for bad in [
      "",
      "abc",
      &"g".repeat(64),
      &"a".repeat(63),
      &"a".repeat(65),
      &"é".repeat(32),
    ] {
      assert_eq!(Digest::from_hex(bad), None, "{bad:?}");
      assert!(serde_json::from_str::<Digest>(&format!("\"{bad}\"")).is_err());
    }
    // 大写也认（只是线上永远写小写）。
    assert!(Digest::from_hex(&"A".repeat(64)).is_some());
  }

  #[test]
  fn the_base_is_stored_compactly_and_reads_back() {
    let base: Base = [("a".to_owned(), synced("r", "l"))].into();
    let json = serde_json::to_string(&base).unwrap();
    assert!(
      json.contains("\"r\":\"") && json.contains("\"l\":\""),
      "{json}"
    );
    assert_eq!(serde_json::from_str::<Base>(&json).unwrap(), base);
  }

  #[test]
  fn scope_excludes_device_keys_the_derived_index_long_keys_and_optionally_images() {
    for key in ["bookmarks", "logbook", "qsl-image:7", "practice:2025-10:A"] {
      assert!(in_scope(key, true), "{key}");
    }
    for key in [
      "theme",
      "ui:x",
      "api-cache:/api/solar",
      "qsl-image-index",
      &"k".repeat(513),
    ] {
      assert!(!in_scope(key, true), "{key}");
    }
    assert!(!in_scope("qsl-image:7", false), "没勾选影像就不同步影像");
    assert!(in_scope("logbook", false));
  }

  #[test]
  fn the_manifest_and_its_fingerprint_follow_the_scope_not_the_order() {
    let map: BTreeMap<String, String> = [
      ("bookmarks", "[]"),
      ("theme", "dark"),
      ("qsl-image-index", "[1]"),
      ("qsl-image:1", "x"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v.to_owned()))
    .collect();
    assert_eq!(
      manifest_of(&map, true).keys().collect::<Vec<_>>(),
      ["bookmarks", "qsl-image:1"]
    );
    assert_eq!(
      manifest_of(&map, false).keys().collect::<Vec<_>>(),
      ["bookmarks"]
    );
    let a = m(&[("a", "1"), ("b", "2")]);
    let reordered: Manifest = [("b", "2"), ("a", "1")]
      .into_iter()
      .map(|(k, v)| (k.to_owned(), Digest::of(v)))
      .collect();
    assert_eq!(manifest_fingerprint(&a), manifest_fingerprint(&reordered));
    assert_ne!(
      manifest_fingerprint(&a),
      manifest_fingerprint(&m(&[("a", "1")]))
    );
    assert_ne!(
      manifest_fingerprint(&a),
      manifest_fingerprint(&m(&[("a", "1"), ("b", "3")]))
    );
  }

  // ─── 三方对比：逐行对照模块文档里的表 ───

  #[test]
  fn identical_sides_need_nothing_whatever_the_baseline_says() {
    for base in [None, Some(("x", "x")), Some(("y", "z"))] {
      assert_eq!(one(base, Some("y"), Some("y"), "k"), Action::Same);
      assert_eq!(one(base, None, None, "k"), Action::Same);
    }
  }

  #[test]
  fn equivalent_but_byte_different_content_is_not_a_change() {
    // 老版本写出的内容：本机是 "old"，云端规范化成 "canon"。上次同步时就是这样，两边之后都没动。
    // 这是基线用一对摘要的原因：本机摘要 ≠ 云端摘要，但两边都没改。
    let key = "study-stats";
    assert_eq!(
      one(Some(("canon", "old")), Some("old"), Some("canon"), key),
      Action::Same
    );
    // 之后本机改了：上传（不是冲突）。
    assert_eq!(
      one(Some(("canon", "old")), Some("old2"), Some("canon"), key),
      Action::Push
    );
    // 之后云端改了：取回。
    assert_eq!(
      one(Some(("canon", "old")), Some("old"), Some("canon2"), key),
      Action::Pull
    );
    // 两边都改了：study-stats 没有安全规则，问用户。
    assert_eq!(
      one(Some(("canon", "old")), Some("old2"), Some("canon2"), key),
      Action::Conflict
    );
  }

  #[test]
  fn only_the_remote_changed_means_pull() {
    assert_eq!(
      one(Some(("a", "a")), Some("a"), Some("b"), "bookmarks"),
      Action::Pull
    );
    // 本机没有、云端新增（基线也没有）。
    assert_eq!(one(None, None, Some("b"), "bookmarks"), Action::Pull);
  }

  #[test]
  fn only_the_local_side_changed_means_push() {
    assert_eq!(
      one(Some(("a", "a")), Some("b"), Some("a"), "bookmarks"),
      Action::Push
    );
    // 云端没有、本机新增（基线也没有）：包括从没同步过的设备上的全部数据。
    assert_eq!(one(None, Some("b"), None, "bookmarks"), Action::Push);
  }

  #[test]
  fn deletions_propagate_only_for_settings_progress_and_images() {
    // 进度、设置、影像：删除照传。
    for key in [
      "practice:2025-10:A",
      "exam:savedState:2025-10:A",
      "learning-plan",
      "qsl-image:7",
    ] {
      assert_eq!(
        one(Some(("a", "a")), None, Some("a"), key),
        Action::PushDelete,
        "{key}"
      );
      assert_eq!(
        one(Some(("a", "a")), Some("a"), None, key),
        Action::PullDelete,
        "{key}"
      );
    }
    // 积累出来的数据：一边没有另一边没动，补回来。浏览器清了站点数据、IndexedDB 被回收，
    // 不该让云端跟着一起清空。
    for key in [
      "logbook",
      "mistake-book",
      "study-stats",
      "bookmarks",
      "countdowns",
      "unknown-key",
    ] {
      assert_eq!(
        one(Some(("a", "a")), None, Some("a"), key),
        Action::Pull,
        "{key} 本机丢了 → 补回来"
      );
      assert_eq!(
        one(Some(("a", "a")), Some("a"), None, key),
        Action::Push,
        "{key} 云端丢了 → 补回去"
      );
    }
  }

  #[test]
  fn both_changed_follows_the_resolution_of_the_key() {
    let both = |key: &str| one(Some(("a", "a")), Some("b"), Some("c"), key);
    // 有合并规则：两边都有值才能合并。没有基线也一样。
    assert_eq!(both("bookmarks"), Action::Merge);
    assert_eq!(one(None, Some("b"), Some("c"), "bookmarks"), Action::Merge);
    // 后写的为准。
    assert_eq!(both("learning-plan"), Action::KeepLocal);
    assert_eq!(
      one(Some(("a", "a")), None, Some("c"), "learning-plan"),
      Action::KeepLocal
    );
    assert_eq!(
      one(Some(("a", "a")), Some("b"), None, "learning-plan"),
      Action::KeepLocal
    );
    // 没有安全规则：问用户。日志与影像也在其中。
    for key in ["study-stats", "no-such-key", "logbook", "qsl-image:3"] {
      assert_eq!(both(key), Action::Conflict, "{key}");
    }
    // 有合并规则的 key，一边改一边删：合并无从谈起，也问用户。
    assert_eq!(
      one(Some(("a", "a")), None, Some("c"), "bookmarks"),
      Action::Conflict
    );
    assert_eq!(
      one(Some(("a", "a")), Some("b"), None, "bookmarks"),
      Action::Conflict
    );
  }

  #[test]
  fn a_fresh_device_takes_the_remote_side_of_every_disagreement() {
    let (base, local, remote) = (
      Base::new(),
      m(&[
        ("study-stats", "default"),
        ("bookmarks", "[]"),
        ("only-local", "x"),
      ]),
      m(&[
        ("study-stats", "real"),
        ("bookmarks", "[\"A-1\"]"),
        ("only-remote", "y"),
      ]),
    );
    let p = plan(&base, &local, &remote, true);
    assert_eq!(
      p.action("study-stats"),
      Action::Pull,
      "默认值不是用户的改动"
    );
    assert_eq!(p.action("bookmarks"), Action::Pull);
    assert_eq!(p.action("only-remote"), Action::Pull);
    // 本机独有的仍然上传，不丢。
    assert_eq!(p.action("only-local"), Action::Push);
    assert!(p.conflicts().is_empty());
  }

  #[test]
  fn two_devices_changing_different_keys_never_conflict() {
    // 这是整套设计要解决的场景：A 改了收藏，B 改了练习进度，旧协议会判成冲突。
    let base: Base = [
      ("bookmarks".to_owned(), synced("[]", "[]")),
      ("practice:x".to_owned(), synced("p0", "p0")),
      ("study-stats".to_owned(), synced("s0", "s0")),
    ]
    .into();
    let local = m(&[
      ("bookmarks", "[\"A-1\"]"),
      ("practice:x", "p0"),
      ("study-stats", "s0"),
    ]);
    let remote = m(&[
      ("bookmarks", "[]"),
      ("practice:x", "p1"),
      ("study-stats", "s0"),
    ]);
    let p = plan(&base, &local, &remote, false);
    assert_eq!(p.action("bookmarks"), Action::Push);
    assert_eq!(p.action("practice:x"), Action::Pull);
    assert_eq!(p.action("study-stats"), Action::Same);
    assert!(p.conflicts().is_empty());
    assert!(p.writes_local());
  }

  #[test]
  fn the_plan_is_symmetric_between_the_two_devices() {
    // 同样的状态，从 A 看是 Pull 的，从 B 看就是 Push，反之亦然；都改了的两边都是都改了。
    let flip = |a: Action| match a {
      Action::Pull => Action::Push,
      Action::Push => Action::Pull,
      Action::PullDelete => Action::PushDelete,
      Action::PushDelete => Action::PullDelete,
      other => other,
    };
    let states = [None, Some("a"), Some("b"), Some("c")];
    // 基线用「两边相同」：对称性只在两边基线一致时成立。
    for key in ["bookmarks", "learning-plan", "study-stats", "qsl-image:1"] {
      for b in states {
        for l in states {
          for r in states {
            let base = |v: Option<&str>| -> Base {
              v.map(|v| [(key.to_owned(), synced(v, v))].into())
                .unwrap_or_default()
            };
            let side = |v: Option<&str>| v.map(|v| m(&[(key, v)])).unwrap_or_default();
            let a = plan(&base(b), &side(l), &side(r), false).action(key);
            let z = plan(&base(b), &side(r), &side(l), false).action(key);
            if a == Action::KeepLocal {
              assert_eq!(z, Action::KeepLocal, "{key} {b:?} {l:?} {r:?}");
            } else {
              assert_eq!(flip(a), z, "{key} {b:?} {l:?} {r:?}");
            }
          }
        }
      }
    }
  }

  #[test]
  fn a_local_image_is_never_sent_while_the_logbook_is_being_resolved() {
    let img = "qsl-image:5";
    let scenario = |l: &str, r: &str| {
      let base: Base = [
        ("logbook".to_owned(), synced("L0", "L0")),
        (img.to_owned(), synced("i0", "i0")),
      ]
      .into();
      let local = m(&[("logbook", l), (img, "mine")]);
      let remote = m(&[("logbook", r), (img, "i0")]);
      plan(&base, &local, &remote, false)
    };
    // 日志两边都改了（冲突）：本机的影像编号可能对着云端的另一条通联，不能传。
    let p = scenario("L1", "L2");
    assert_eq!(p.action("logbook"), Action::Conflict);
    assert_eq!(p.action(img), Action::Conflict);
    // 云端日志变了、本机没动：先取云端的，本机的影像也等到用户确认。
    let p = scenario("L0", "L2");
    assert_eq!(p.action("logbook"), Action::Pull);
    assert_eq!(p.action(img), Action::Conflict);
    // 只有本机改了日志：编号对得上，一起传。
    let p = scenario("L1", "L0");
    assert_eq!(p.action("logbook"), Action::Push);
    assert_eq!(p.action(img), Action::Push);
    // 日志没变：影像照传。
    let p = scenario("L0", "L0");
    assert_eq!(p.action("logbook"), Action::Same);
    assert_eq!(p.action(img), Action::Push);
  }

  #[test]
  fn images_coming_in_follow_the_remote_logbook_and_are_not_held() {
    let base: Base = [
      ("logbook".to_owned(), synced("L0", "L0")),
      ("qsl-image:7".to_owned(), synced("i", "i")),
    ]
    .into();
    let local = m(&[("logbook", "L0"), ("qsl-image:7", "i")]);
    let remote = m(&[
      ("logbook", "L1"),
      ("qsl-image:7", "i2"),
      ("qsl-image:8", "new"),
    ]);
    let p = plan(&base, &local, &remote, false);
    assert_eq!(p.action("logbook"), Action::Pull);
    assert_eq!(p.action("qsl-image:7"), Action::Pull);
    assert_eq!(p.action("qsl-image:8"), Action::Pull);
    assert!(p.conflicts().is_empty());
  }

  #[test]
  fn the_baseline_after_a_manual_push_pairs_each_side_with_its_own_digest() {
    let local = m(&[("a", "mine-a"), ("b", "mine-b")]);
    // 整份上传之后云端恰好是这两个 key；服务端把 a 规范化了，所以云端的摘要与本机的不同。
    let remote_after = m(&[("a", "canon-a"), ("b", "mine-b")]);
    let base = base_after_push(&local, &remote_after);
    assert_eq!(base["a"], synced("canon-a", "mine-a"));
    assert_eq!(base["b"], synced("mine-b", "mine-b"));
    // 紧接着再对比一次：规范化造成的字节差异不算任何一边的改动。
    assert!(plan(&base, &local, &remote_after, false).is_empty());
    // 回执里没有的 key（服务端没收）不记基线，下一轮仍当成本机新增。
    let partial = base_after_push(&local, &m(&[("b", "mine-b")]));
    assert!(!partial.contains_key("a"));
  }

  #[test]
  fn after_a_manual_pull_what_the_restore_changed_locally_is_uploaded_next_round() {
    // 云端：stats 是 s0，bookmarks 是 b0。恢复（合并）之后本机的 stats 是累加出的 s1，书签没变，还多了本机独有的 note。
    let remote = m(&[("study-stats", "s0"), ("bookmarks", "b0")]);
    let local_now = m(&[("study-stats", "s1"), ("bookmarks", "b0"), ("note", "n")]);
    let base = base_after_pull(&local_now, &remote);
    assert_eq!(base["study-stats"], synced("s0", "s0"));
    assert!(!base.contains_key("note"), "云端没有的 key 没有基线");
    let p = plan(&base, &local_now, &remote, false);
    // 累计类的 key 本来会在两边都改了时卡成冲突；这里本机改了、云端没动，所以是直接上传。
    assert_eq!(p.action("study-stats"), Action::Push);
    assert_eq!(p.action("bookmarks"), Action::Same);
    assert_eq!(p.action("note"), Action::Push);
    // 导入时被跳过（本机没有）的 key 不记基线，下一轮仍当成云端的新内容。
    let skipped = base_after_pull(&m(&[("bookmarks", "b0")]), &remote);
    assert!(!skipped.contains_key("study-stats"));
  }

  #[test]
  fn after_a_successful_sync_everything_is_the_same_again() {
    let remote = m(&[("a", "1"), ("b", "2")]);
    let base: Base = [
      ("a".to_owned(), synced("1", "1")),
      ("b".to_owned(), synced("2", "2")),
    ]
    .into();
    let p = plan(&base, &remote, &remote, false);
    assert!(p.is_empty());
    assert!(!p.writes_local());
  }

  #[test]
  fn deletions_are_listed_and_a_plan_can_say_which_keys_to_write_locally() {
    let base: Base = [
      ("exam:savedState:2025:A".to_owned(), synced("s", "s")),
      ("bookmarks".to_owned(), synced("[]", "[]")),
    ]
    .into();
    let local = m(&[("exam:savedState:2025:A", "s"), ("bookmarks", "[]")]);
    let remote = m(&[("bookmarks", "[]")]);
    let p = plan(&base, &local, &remote, false);
    assert_eq!(p.deletions(), ["exam:savedState:2025:A"]);
    assert!(p.writes_local());
  }

  // ─── 每个 key 的处理方式 ───

  #[test]
  fn every_documented_synced_key_has_an_explicit_resolution() {
    // 新增本地存储 key 时要在 `classify` 里归类；漏了这条测试会红，而不是悄悄落到「问用户」。
    for name in crate::remote_sync::DOCUMENTED_SYNCED_KEYS {
      let concrete = [
        (*name).to_owned(),
        format!("{name}:x"),
        format!("{name}:2025-10:A"),
      ];
      assert!(
        concrete.iter().any(|k| classify(k).is_some()),
        "{name} 没有在 delta::classify 里归类"
      );
    }
  }

  #[test]
  fn resolutions_follow_the_data_not_the_name() {
    use Resolution::{Block, LastWriterWins, Merge};
    for key in [
      "station-book",
      "bookmarks",
      "dxcc_wanted_done",
      "exam-history",
      "mistake-book",
      "daily-challenge",
      "rpg-stars",
      "rpg-bestiary",
    ] {
      assert_eq!(resolution(key), Merge, "{key}");
    }
    for key in [
      "practice:2025-10:A",
      "practice:lastMode",
      "exam:savedState:2025-10:A",
      "exam:wrong:B",
      "exam:last-review",
      "exam:weighted",
      "learning-plan",
      "listen-settings",
      "contest-session",
    ] {
      assert_eq!(resolution(key), LastWriterWins, "{key}");
    }
    for key in [
      "logbook",
      "qsl-image:7",
      "study-stats",
      "question-stats",
      "study-daily",
      "card-review",
      "question-notes",
      "countdowns",
      "morse-stats",
      "nec-designs",
      "brand-new-key-from-a-newer-client",
    ] {
      assert_eq!(resolution(key), Block, "{key}");
    }
  }

  #[test]
  fn merge_keys_are_exactly_the_ones_backup_merge_knows_how_to_merge() {
    // 自动合并复用手动「合并」的实现；归为 Merge 的 key 若 `merge_value` 不认，合并就会静默保留本机、
    // 云端那一份被丢掉。反过来，`merge_value` 认的累加类（study-stats）不能归 Merge，会重复累加。
    // 日志也是 `merge_value` 认的，但它连着影像的编号，自动同步不碰（见 `classify`）。
    for key in [
      "station-book",
      "bookmarks",
      "dxcc_wanted_done",
      "exam-history",
      "mistake-book",
      "daily-challenge",
      "rpg-stars",
      "rpg-bestiary",
    ] {
      assert_eq!(resolution(key), Resolution::Merge, "{key}");
      // 本机没有 → 采用云端的（校验过形态的）；本机有 → 合并出一个结果。两条路都得有输出。
      let sample = match key {
        "station-book" => {
          r#"{"next_id":2,"profiles":[{"id":1,"label":"","callsign":"BG1","operator":"","gridsquare":"","rig":"","antenna":""}],"active_id":1}"#
        }
        "bookmarks" | "dxcc_wanted_done" => r#"["A-1"]"#,
        "exam-history" => "[]",
        "mistake-book" => r#"{"records":[]}"#,
        "daily-challenge" => r#"{"days":{}}"#,
        "rpg-stars" => r#"{"best":{}}"#,
        _ => r#"{"defeated":[]}"#,
      };
      assert!(
        crate::backup_merge::merge_value(key, Some(sample), sample).is_some(),
        "{key} 的合并规则不认这个形态"
      );
    }
    assert_ne!(
      resolution("study-stats"),
      Resolution::Merge,
      "累加合并不是幂等的"
    );
  }
}
