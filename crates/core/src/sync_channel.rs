//! 同步通道：「自定义远端」与「在线账号」两条路并存时，谁在自动同步、切换时怎么不丢数据。
//!
//! 两条通道交换的是**同一份内容**（`remote_sync` 里那份不含设备专属 key 的备份 JSON），
//! 区别只在去处：自定义远端是用户自己的两个接口，在线账号是本站服务器里 SQLite 的一行。
//! 设计取舍与数据结构见 `docs/domain/account-sync.md`。
//!
//! # 为什么自动同步一次只跑一条
//!
//! 自动同步靠一条**基线**（上次同步成功时远端内容的指纹）加一个**脏位**判断「覆盖安不安全」。
//! 两条通道同时自动跑，从 A 拉下来的内容对 B 来说就是「本机变了」，B 又要写、写完 A 又觉得变了……
//! 基线互相作废、谁也说不清两边是否分叉。所以**自动**只跑一条（[`resolve`]），另一条保留
//! 手动的「写入 / 恢复 / 合并」；两条通道各存各的状态（键名见 [`SyncChannel::state_key`]），互不覆盖。
//!
//! 两条通道判定「谁覆盖谁」的办法也不同：自定义远端只有一个接口，只能拿整份内容的指纹做基线
//! （[`SyncState`] + `remote_sync::plan_startup`）；在线账号是我们自己的服务，按 key 交换，
//! 基线记每个 key 两侧的摘要（[`AccountSync`] + [`crate::delta_sync`]）。

use serde::{Deserialize, Serialize};

use crate::delta::Base;
use crate::remote_sync::SyncState;

// 三个 key 的定义在 `remote_sync`（它决定哪些 key 不进备份），这里重新导出，
// 让「通道相关的东西从 `sync_channel` 取」这条路径不变。
pub use crate::remote_sync::{ACCOUNT_OPTIONS_KEY, ACCOUNT_STATE_KEY, CHANNEL_KEY};

/// 在线账号通道的同步选项。
///
/// 与自定义远端的 [`RemoteSyncConfig::exclude_qsl_images`](crate::remote_sync::RemoteSyncConfig)
/// **各管各的**：云端空间是服务器的共享资源，用户可能想让自己配的远端带影像、站点的云端不带。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AccountOptions {
  /// 同步内容里不带 QSL 影像（同自定义远端的同名开关）。
  pub exclude_qsl_images: bool,
}

/// 一条同步通道。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncChannel {
  /// 用户自己配的获取 / 写入接口。
  Custom,
  /// 本站服务器上的在线账号（SQLite）。
  Account,
}

impl SyncChannel {
  /// 全部通道，顺序即界面上的顺序。
  pub const ALL: [Self; 2] = [Self::Custom, Self::Account];

  /// 存进 `localStorage` / 下拉框的稳定标识。
  #[must_use]
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Custom => "custom",
      Self::Account => "account",
    }
  }

  /// 从标识解析；不认识的返回 `None`（旧值 / 被手改过的值按「没选」处理）。
  #[must_use]
  pub fn parse(s: &str) -> Option<Self> {
    Self::ALL.into_iter().find(|c| c.as_str() == s.trim())
  }

  /// 这条通道的同步状态存在哪个 `localStorage` key。
  #[must_use]
  pub const fn state_key(self) -> &'static str {
    match self {
      Self::Custom => crate::remote_sync::STATE_KEY,
      Self::Account => ACCOUNT_STATE_KEY,
    }
  }
}

/// 用户对「谁来自动同步」的明确选择。
///
/// 比 [`SyncChannel`] 多一个 `Off`：两个面板里各有一个「自动同步」开关，关掉正在跑的那个，
/// 意思就是都不自动同步，而不是悄悄交给另一条。没有这个 key = 从没选过（见 [`resolve`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelPref {
  /// 自定义远端。
  Custom,
  /// 在线账号。
  Account,
  /// 都不自动同步（手动按钮仍可用）。
  Off,
}

impl ChannelPref {
  /// 存进 `localStorage` 的稳定标识。
  #[must_use]
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Custom => "custom",
      Self::Account => "account",
      Self::Off => "off",
    }
  }

  /// 从标识解析；不认识的返回 `None`（旧值 / 被手改过的值按「没选过」处理）。
  #[must_use]
  pub fn parse(s: &str) -> Option<Self> {
    [Self::Custom, Self::Account, Self::Off]
      .into_iter()
      .find(|p| p.as_str() == s.trim())
  }

  /// 选中某条通道。
  #[must_use]
  pub const fn of(channel: SyncChannel) -> Self {
    match channel {
      SyncChannel::Custom => Self::Custom,
      SyncChannel::Account => Self::Account,
    }
  }
}

/// 现在该由哪条通道自动同步。
///
/// - 用户明确选过（`pref`）：照选；选的通道暂时不可用（账号未登录 / 接口没填）由通道自己暂停并说明，
///   **不悄悄换成另一条**——换了就是用另一份基线去判断，行为用户看不懂；
/// - 没选过：保持升级前的行为——配了自定义接口就用它，没配就不自动同步。
#[must_use]
pub const fn resolve(pref: Option<ChannelPref>, custom_configured: bool) -> Option<SyncChannel> {
  match pref {
    Some(ChannelPref::Custom) => Some(SyncChannel::Custom),
    Some(ChannelPref::Account) => Some(SyncChannel::Account),
    Some(ChannelPref::Off) => None,
    None if custom_configured => Some(SyncChannel::Custom),
    None => None,
  }
}

/// 登录成功后的偏好：什么都没选、也没有自定义接口在自动同步时，登录就是想用账号同步，直接选上；
/// 已经有通道在跑、或用户明确关掉过自动同步时不抢，让用户在界面上自己开。
#[must_use]
pub const fn pref_after_login(
  pref: Option<ChannelPref>,
  custom_configured: bool,
) -> Option<ChannelPref> {
  match pref {
    None if !custom_configured => Some(ChannelPref::Account),
    other => other,
  }
}

/// 退出登录后的偏好：账号通道没了去处，回到「没选过」；选的是别的（或明确关掉）则不动。
#[must_use]
pub const fn pref_after_logout(pref: Option<ChannelPref>) -> Option<ChannelPref> {
  match pref {
    Some(ChannelPref::Account) => None,
    other => other,
  }
}

/// 离开一条通道（切走、退出登录）时它的状态该变成什么。
///
/// 通道不在跑的时候，本机数据的改动**不被追踪**（写入钩子只盯着正在跑的那条）。
/// 留着旧基线再回来，就会把「期间改过」当成「没改过」而放心覆盖——所以离开即作废基线，
/// 回来时按「从没同步过」处理，宁可多问一次也不静默覆盖。脏位保留：它的含义是「可能有没传出去的改动」。
#[must_use]
pub const fn leave(state: SyncState) -> SyncState {
  SyncState {
    base: None,
    owner: None,
    dirty: state.dirty,
  }
}

/// 在线账号通道落盘的同步状态。
///
/// 与自定义远端的 [`SyncState`] 同样存在 `localStorage` 里、同样不进备份（见 `remote_sync::is_excluded_from_backup`），
/// 区别是基线不再是一个指纹，而是**每个 key 两侧各一个摘要**（见 [`crate::delta`] 模块文档「基线是一对摘要」）。
/// 旧版本存下来的状态里那个整份指纹的 `base` 字段读不出来也无妨：没有基线就是「从没同步过」，
/// 引擎会逐个 key 对比，相同的不动、不同的按规则处理，不会误覆盖。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AccountSync {
  /// 这份基线属于哪个账号（账号 id）。
  ///
  /// 同一台浏览器可能换账号登录：A 的基线拿来判断 B 的云端，会把「B 的云端更新过」误当成
  /// 「可以放心取回覆盖本机」，本机里没传给 B 的数据就被吞了。基线不属于当前账号时作废。
  #[serde(skip_serializing_if = "Option::is_none")]
  pub owner: Option<u64>,
  /// 本机自上次同步以来可能有没传出去的改动（只用来决定要不要排一次自动写入；
  /// 引擎每次都会重新对比，所以它「多亮一次」只是多一次无谓的检查，不会丢数据）。
  pub dirty: bool,
  /// 每个 key 上次同步成功时两侧的摘要。
  #[serde(
    skip_serializing_if = "Base::is_empty",
    default,
    deserialize_with = "deserialize_base"
  )]
  pub keys: Base,
}

/// 逐条容错地读基线。
///
/// 一条坏摘要只丢那一条，不能让整份状态一起读不出来：丢了 `dirty` 就等于「可能有没传出去的改动」
/// 这条信息消失，界面不会再提示，而引擎只能等用户下一次操作才发现差异。
fn deserialize_base<'de, D>(deserializer: D) -> Result<Base, D::Error>
where
  D: serde::Deserializer<'de>,
{
  let raw = std::collections::BTreeMap::<String, serde_json::Value>::deserialize(deserializer)?;
  Ok(
    raw
      .into_iter()
      .filter_map(|(key, value)| serde_json::from_value(value).ok().map(|s| (key, s)))
      .collect(),
  )
}

impl AccountSync {
  /// 离开账号通道（切走、退出登录）：基线与主人作废，脏位保留。
  ///
  /// 理由同 [`leave`]：通道不在跑时本机的改动没人追踪，留着旧基线回来，会把「期间改过」当成「没改过」。
  #[must_use]
  pub fn leave(self) -> Self {
    Self {
      owner: None,
      dirty: self.dirty,
      keys: Base::new(),
    }
  }

  /// 登录的是 `user_id`：基线不属于它就作废，改记新主人，脏位不丢。
  #[must_use]
  pub fn adopt_owner(self, user_id: u64) -> Self {
    if self.owner == Some(user_id) {
      return self;
    }
    Self {
      owner: Some(user_id),
      dirty: self.dirty,
      keys: Base::new(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::remote_sync::fingerprint;

  fn fp(n: &str) -> crate::remote_sync::Fingerprint {
    let mut m = std::collections::BTreeMap::new();
    m.insert("k".to_owned(), n.to_owned());
    fingerprint(&m)
  }

  #[test]
  fn channels_round_trip_through_their_identifier() {
    for c in SyncChannel::ALL {
      assert_eq!(SyncChannel::parse(c.as_str()), Some(c));
    }
    assert_eq!(SyncChannel::parse(" account "), Some(SyncChannel::Account));
    assert_eq!(SyncChannel::parse(""), None);
    assert_eq!(SyncChannel::parse("Custom"), None);
    assert_eq!(SyncChannel::parse("cloud"), None);
  }

  #[test]
  fn a_broken_entry_does_not_discard_the_rest() {
    let good = crate::delta::Digest::of("v").to_hex();
    let raw = format!(
      r#"{{"owner":7,"dirty":true,"keys":{{
        "bookmarks":{{"r":"{good}","l":"{good}"}},
        "study-stats":{{"r":"not-a-digest","l":"{good}"}},
        "question-stats":{{"l":"{good}"}}
      }}}}"#
    );
    let state: AccountSync = serde_json::from_str(&raw).expect("坏条目不该让整份状态读不出来");
    assert!(state.dirty, "脏位必须保住");
    assert_eq!(state.owner, Some(7));
    assert_eq!(state.keys.len(), 1, "只有完整的那条留下");
    assert!(state.keys.contains_key("bookmarks"));
  }

  #[test]
  fn each_channel_keeps_its_own_state_key() {
    assert_ne!(
      SyncChannel::Custom.state_key(),
      SyncChannel::Account.state_key()
    );
    // 自定义远端沿用升级前的 key：已存下来的基线仍然读得出。
    assert_eq!(SyncChannel::Custom.state_key(), "remote-sync-state");
  }

  #[test]
  fn without_a_choice_the_pre_account_behaviour_is_kept() {
    assert_eq!(resolve(None, false), None);
    assert_eq!(resolve(None, true), Some(SyncChannel::Custom));
  }

  #[test]
  fn an_explicit_choice_is_never_overridden() {
    for configured in [false, true] {
      assert_eq!(
        resolve(Some(ChannelPref::Account), configured),
        Some(SyncChannel::Account)
      );
      assert_eq!(
        resolve(Some(ChannelPref::Custom), configured),
        Some(SyncChannel::Custom)
      );
      // 明确关掉就是都不自动同步：哪怕配了自定义接口，也不能「顺手」把它开起来。
      assert_eq!(resolve(Some(ChannelPref::Off), configured), None);
    }
  }

  #[test]
  fn prefs_round_trip_through_their_identifier() {
    for p in [ChannelPref::Custom, ChannelPref::Account, ChannelPref::Off] {
      assert_eq!(ChannelPref::parse(p.as_str()), Some(p));
    }
    assert_eq!(ChannelPref::parse(" off "), Some(ChannelPref::Off));
    assert_eq!(ChannelPref::parse(""), None);
    assert_eq!(ChannelPref::parse("cloud"), None);
    for c in SyncChannel::ALL {
      assert_eq!(ChannelPref::of(c).as_str(), c.as_str());
    }
  }

  #[test]
  fn logging_in_only_takes_over_when_nothing_else_is_syncing() {
    // 什么都没在自动同步：登录就是想用账号。
    assert_eq!(pref_after_login(None, false), Some(ChannelPref::Account));
    // 配了自定义接口：不抢，保持「没选过」（运行时仍解析成自定义）。
    assert_eq!(pref_after_login(None, true), None);
    // 明确选过的、明确关掉的都不改。
    for configured in [false, true] {
      for pref in [ChannelPref::Custom, ChannelPref::Account, ChannelPref::Off] {
        assert_eq!(pref_after_login(Some(pref), configured), Some(pref));
      }
    }
  }

  #[test]
  fn logging_out_only_clears_an_account_choice() {
    assert_eq!(pref_after_logout(Some(ChannelPref::Account)), None);
    assert_eq!(
      pref_after_logout(Some(ChannelPref::Custom)),
      Some(ChannelPref::Custom)
    );
    assert_eq!(
      pref_after_logout(Some(ChannelPref::Off)),
      Some(ChannelPref::Off)
    );
    assert_eq!(pref_after_logout(None), None);
  }

  #[test]
  fn account_options_tolerate_missing_fields_and_default_to_everything() {
    let o: AccountOptions = serde_json::from_str("{}").unwrap();
    assert!(!o.exclude_qsl_images);
    let back: AccountOptions = serde_json::from_str(
      &serde_json::to_string(&AccountOptions {
        exclude_qsl_images: true,
      })
      .unwrap(),
    )
    .unwrap();
    assert!(back.exclude_qsl_images);
  }

  fn synced(r: &str, l: &str) -> crate::delta::Synced {
    crate::delta::Synced {
      remote: crate::delta::Digest::of(r),
      local: crate::delta::Digest::of(l),
    }
  }

  fn account(owner: Option<u64>, dirty: bool) -> AccountSync {
    AccountSync {
      owner,
      dirty,
      keys: [("k".to_owned(), synced("r", "l"))].into(),
    }
  }

  #[test]
  fn leaving_a_channel_voids_the_baseline_but_keeps_the_dirty_bit() {
    let state = SyncState {
      base: Some(fp("a")),
      owner: Some(7),
      dirty: true,
    };
    let left = leave(state);
    assert_eq!(left.base, None);
    assert_eq!(left.owner, None);
    assert!(left.dirty);
  }

  #[test]
  fn leaving_the_account_channel_voids_every_key_baseline_but_keeps_the_dirty_bit() {
    let left = account(Some(7), true).leave();
    assert!(left.keys.is_empty());
    assert_eq!(left.owner, None);
    assert!(left.dirty);
    assert!(!account(Some(7), false).leave().dirty);
  }

  #[test]
  fn a_baseline_only_belongs_to_the_account_that_made_it() {
    let mine = account(Some(1), false);
    // 同一个账号：原样保留。
    assert_eq!(mine.clone().adopt_owner(1), mine);
    // 换了账号：基线作废、改记新主人，脏位不丢。
    let other = account(Some(1), true).adopt_owner(2);
    assert!(other.keys.is_empty());
    assert_eq!(other.owner, Some(2));
    assert!(other.dirty);
    // 没有主人的状态（从没登录过 / 升级前存的）：第一次登录也要作废，那份基线不知道是谁的。
    assert!(account(None, false).adopt_owner(1).keys.is_empty());
  }

  #[test]
  fn the_account_state_round_trips_and_stays_small_when_there_is_no_baseline() {
    let state = account(Some(3), true);
    let json = serde_json::to_string(&state).unwrap();
    assert_eq!(serde_json::from_str::<AccountSync>(&json).unwrap(), state);
    let empty = serde_json::to_string(&AccountSync::default()).unwrap();
    assert_eq!(empty, r#"{"dirty":false}"#);
  }

  #[test]
  fn a_state_saved_by_the_whole_file_era_is_read_as_never_synced_not_as_an_error() {
    // 旧版本存的是 `{"base":<整份指纹>,"owner":7,"dirty":false}`：整份指纹对按 key 的基线没有意义，
    // 读成「从没同步过」（主人与脏位照留）。读失败就会连脏位一起丢掉。
    let old: AccountSync =
      serde_json::from_str(r#"{"base":123456789,"owner":7,"dirty":true}"#).unwrap();
    assert_eq!(old.owner, Some(7));
    assert!(old.dirty);
    assert!(old.keys.is_empty());
    // 什么都没存、或存了别的东西：默认。
    assert_eq!(
      serde_json::from_str::<AccountSync>("{}").unwrap(),
      AccountSync::default()
    );
  }
}
