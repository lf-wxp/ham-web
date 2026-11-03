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

use serde::{Deserialize, Serialize};

use crate::remote_sync::{Remote, StartupPlan, SyncState, plan_startup};

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

  /// 去处是不是「已经确认过是对的」。
  ///
  /// 自定义远端的地址是用户手敲的：获取返回 404 可能只是地址写错，所以从没成功过的接口
  /// 不自动上传（见 [`StartupPlan::NeedsFirstPush`]）。在线账号的去处是登录态本身，
  /// 登录成功就说明地址与身份都对，云端为空只是「还没传过」，可以直接传。
  #[must_use]
  pub const fn endpoint_is_verified(self) -> bool {
    matches!(self, Self::Account)
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

/// 启动时的决策：[`plan_startup`] 的通道版。
///
/// 只有一处不同：已确认去处的通道（[`SyncChannel::endpoint_is_verified`]）遇到
/// 「从没同步成功过而远端是空的」时直接上传，而不是等用户手动确认。
#[must_use]
pub fn plan_for(
  channel: SyncChannel,
  state: &SyncState,
  stale_dirty: bool,
  remote: Remote,
  local_has_data: bool,
) -> StartupPlan {
  match plan_startup(state, stale_dirty, remote, local_has_data) {
    StartupPlan::NeedsFirstPush if channel.endpoint_is_verified() => StartupPlan::Upload,
    plan => plan,
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

/// 状态里的基线是不是属于这个账号。
///
/// 同一台浏览器可能换账号登录：A 的基线拿来判断 B 的云端，会把「B 的云端更新过」误当成
/// 「可以放心快进覆盖本机」，本机里没传给 B 的数据就被吞了。基线不属于当前账号时作废。
#[must_use]
pub fn adopt_owner(state: SyncState, user_id: u64) -> SyncState {
  if state.owner == Some(user_id) {
    return state;
  }
  SyncState {
    base: None,
    owner: Some(user_id),
    dirty: state.dirty,
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

  #[test]
  fn an_empty_cloud_is_uploaded_to_without_asking_but_a_custom_endpoint_is_not() {
    let never = SyncState::default();
    assert_eq!(
      plan_for(SyncChannel::Account, &never, false, Remote::Empty, true),
      StartupPlan::Upload
    );
    assert_eq!(
      plan_for(SyncChannel::Custom, &never, false, Remote::Empty, true),
      StartupPlan::NeedsFirstPush
    );
  }

  #[test]
  fn every_other_decision_is_the_same_for_both_channels() {
    let clean = SyncState {
      base: Some(fp("a")),
      owner: None,
      dirty: false,
    };
    let cases = [
      (clean, false, Remote::Data(fp("a")), true),
      (clean, false, Remote::Data(fp("b")), true),
      (clean, true, Remote::Data(fp("b")), true),
      (SyncState::default(), false, Remote::Data(fp("b")), true),
      (SyncState::default(), false, Remote::Data(fp("b")), false),
      (clean, false, Remote::Empty, true),
      (SyncState::default(), false, Remote::Empty, false),
    ];
    for (state, stale, remote, has) in cases {
      assert_eq!(
        plan_for(SyncChannel::Account, &state, stale, remote, has),
        plan_for(SyncChannel::Custom, &state, stale, remote, has),
      );
    }
  }

  #[test]
  fn a_fresh_login_on_a_used_browser_with_cloud_data_is_a_conflict() {
    // 云端有数据、本机也有、从没同步过：不知道谁新，交给用户（与自定义远端一致）。
    assert_eq!(
      plan_for(
        SyncChannel::Account,
        &SyncState::default(),
        false,
        Remote::Data(fp("cloud")),
        true
      ),
      StartupPlan::Conflict
    );
    // 全新浏览器：直接取云端。
    assert_eq!(
      plan_for(
        SyncChannel::Account,
        &SyncState::default(),
        false,
        Remote::Data(fp("cloud")),
        false
      ),
      StartupPlan::FastForward
    );
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
    // 作废之后回来：远端变了也不会被当成「没动过」而快进覆盖。
    assert_eq!(
      plan_for(
        SyncChannel::Account,
        &leave(SyncState {
          base: Some(fp("a")),
          owner: Some(7),
          dirty: false
        }),
        false,
        Remote::Data(fp("b")),
        true
      ),
      StartupPlan::Conflict
    );
  }

  #[test]
  fn a_baseline_only_belongs_to_the_account_that_made_it() {
    let mine = SyncState {
      base: Some(fp("a")),
      owner: Some(1),
      dirty: false,
    };
    // 同一个账号：原样保留。
    assert_eq!(adopt_owner(mine, 1), mine);
    // 换了账号：基线作废、改记新主人，脏位不丢。
    let other = adopt_owner(
      SyncState {
        dirty: true,
        ..mine
      },
      2,
    );
    assert_eq!(other.base, None);
    assert_eq!(other.owner, Some(2));
    assert!(other.dirty);
    // 升级前的状态没有主人：第一次登录也要作废（那份基线来自别的通道）。
    let legacy = SyncState {
      base: Some(fp("a")),
      owner: None,
      dirty: false,
    };
    assert_eq!(adopt_owner(legacy, 1).base, None);
  }
}
