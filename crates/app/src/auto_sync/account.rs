//! 自动同步里「在线账号」这条通道：启动与写入都是**一轮按 key 的增量同步**。
//!
//! 与自定义远端那条（只能拿整份内容的指纹当基线，不一样就整份覆盖）不同，账号是我们自己的服务，
//! 每个 key 有摘要、写入带条件，所以由 `ham_web_core::delta_sync` 逐个 key 三方对比（基线 / 本机 / 云端）：
//! 各改各的 key 互不打扰，只传改动的 key，真正两边都改了同一项才停下来问用户。算法与它的穷举测试都在那里；
//! 这里只负责把它接到浏览器上——云端是 `crate::account_cloud`，本机是下面的 [`BrowserDevice`]——
//! 以及把结果翻成状态、提示与重试。
//!
//! 页面刚打开时（启动）那一轮可以把云端的变化落到本机；之后的自动写入只上传：页面里的信号已经读过存储，
//! 中途改写存储，用户下一次操作就会拿旧值盖掉它。需要写本机的变化留到下次打开。
//!
//! 不再需要「取回期间用户的操作也算本机改动」的簿记：落盘前引擎核对每个要写的 key 本机是不是还在读快照时的样子，
//! 不是就整轮重来，用户的改动自然成了「本机改了」的一边。

use std::cell::Cell;
use std::collections::BTreeMap;

use ham_web_core::delta::{Base, Manifest};
use ham_web_core::delta_sync::{self, Applied, Device, Edits, Mode, Options, Outcome, Problem};
use ham_web_core::sync_channel::AccountSync;

use crate::account_cloud::{AccountCloud, Op};
use crate::account_sync::{self, AccountState};
use crate::i18n::{t, tf};
use crate::pages::log::LogStore;
use crate::remote_sync::{AUTO_FETCH_TIMEOUT_MS, PushError, SyncError};
use crate::util::BackupModules;

use super::{
  AutoStatus, KEYS, KEYS_EPOCH, LOG, PUSHING, USER_WROTE, acquire_lock, arm, checkpoint_keys,
  is_stale, pause, pause_conflict, pause_pull_failed, pause_push_failed, pause_signed_out,
  push_failed, push_succeeded, seq, set_keys, set_status, state, update_state,
};

/// 一轮没跑成的原因。
enum Failure {
  Cloud { op: Op, error: SyncError },
  Device(String),
}

/// 浏览器里的本机：读 `localStorage` / IndexedDB，写也是。
struct BrowserDevice {
  log: LogStore,
  include_images: bool,
  /// 这一轮属于哪一「代」：换了通道就不许再落盘（见 `super::GEN`）。
  at: u64,
  /// 这一轮开始时基线的纪元：中途存基线前要核对它没变（见 [`checkpoint_keys`]）。
  epoch: u64,
}

impl Device for BrowserDevice {
  type Error = String;

  async fn snapshot(&mut self) -> Result<BTreeMap<String, String>, String> {
    crate::util::build_sync_backup(BackupModules::for_sync(self.include_images)).await
  }

  async fn apply(&mut self, edits: &Edits, expect: &Manifest) -> Result<Applied, String> {
    // 备份锁：落盘期间页面是 `inert` 的，用户动不了，核对与写入之间不会再有人改数据。
    let Some(task) = acquire_lock(self.log).await else {
      return Err(t("tools.backup-in-progress"));
    };
    // 等锁期间换了通道（或退出了登录）：现在落盘就是用旧通道的内容改新环境。
    // 这条错误不会显示给用户（调用方在 `is_stale` 时整轮丢弃），但必须是条真话——
    // 空字符串一旦漏到界面上就是一次「失败但什么都不说」。
    if is_stale(self.at) {
      drop(task);
      return Err(t("tools.account-channel-changed"));
    }
    let result = {
      // 这里的存储写入是在「照搬云端」，不是用户改了数据。
      let _quiet = super::Suppress::new();
      crate::backup_apply::apply(edits, expect).await
    };
    // 写了一半才失败时也要把日志 Store 对齐到存储（与手动导入一致）。
    if !matches!(result, Ok(Applied::LocalChanged)) {
      self.log.refresh_from_storage().await;
    }
    drop(task);
    result
  }

  fn checkpoint(&mut self, base: &Base) {
    // 换了通道（或退出了登录）之后这一轮已经作废，不许再碰新环境的状态。
    if !is_stale(self.at) {
      checkpoint_keys(self.epoch, base);
    }
  }
}

/// 跑一轮。基线从 [`KEYS`] 复制出来、跑完再存回去（无论成败：中途失败时它记着已经完成的那部分）。
///
/// 存回之前核对 [`KEYS_EPOCH`]：这一轮在路上时，手动上传 / 恢复、登出、换账号都会替换基线，
/// 它们的结果更新，这一轮的就丢掉。
async fn round(
  mode: Mode,
  at: u64,
  log: LogStore,
  manifest_timeout_ms: i32,
  fresh: bool,
) -> Result<Outcome, Failure> {
  let include_images = account_sync::include_images();
  let mut cloud = AccountCloud::new(manifest_timeout_ms);
  let epoch = KEYS_EPOCH.with(Cell::get);
  let mut device = BrowserDevice {
    log,
    include_images,
    at,
    epoch,
  };
  let mut base: Base = KEYS.with(|k| k.borrow().clone());
  let opts = Options {
    mode,
    include_images,
    // 全新浏览器（没有基线、本机也没有用户数据）：两边都有而不同的 key 是应用自己的默认值，以云端为准。
    prefer_remote: fresh && base.is_empty(),
  };
  let result = delta_sync::run(&mut cloud, &mut device, &mut base, opts).await;
  if KEYS_EPOCH.with(Cell::get) == epoch {
    set_keys(base);
  }
  result.map_err(|e| match e {
    delta_sync::Error::Cloud(error) => Failure::Cloud {
      op: cloud.last_op,
      error,
    },
    delta_sync::Error::Device(message) => Failure::Device(message),
  })
}

/// 登录的是 `user_id`：基线只对它的主人有效（见 [`AccountSync::adopt_owner`]）。
fn adopt_owner(user_id: u64) {
  let current = state();
  let before = AccountSync {
    owner: current.owner,
    dirty: current.dirty,
    keys: KEYS.with(|k| k.borrow().clone()),
  };
  let after = before.clone().adopt_owner(user_id);
  if after.keys != before.keys {
    set_keys(after.keys);
  }
  update_state(|s| s.owner = after.owner);
}

/// 一轮跑完但没完成：两边都改了同一项、本机少了很多数据、云端一直在变。
fn paused_by(problem: Problem) {
  match problem {
    Problem::Conflict(_) => pause_conflict(),
    Problem::SuspiciousDeletions(_) => pause_mass_deletion(),
    Problem::Busy => {
      // 云端在这几次尝试之间一直被别处改：稍后再来一次就好，不必让用户做什么。
      update_state(|s| s.dirty = true);
      push_failed(PushError {
        message: t("tools.account-auto-busy"),
        retryable: true,
      });
    }
  }
}

/// 本机比上次同步时少了很多 key：多半是浏览器清了站点数据，不是用户真删了这么多。
/// 照传就会把云端也清空，所以停下来，让用户决定是找回还是确认删除。
fn pause_mass_deletion() {
  pause();
  set_status(AutoStatus::MassDeletion);
  crate::ui::push_error(t("tools.account-auto-mass-deletion"));
}

// ───────────────────────── 启动 ─────────────────────────

/// 页面打开（或刚登录）时的一轮：可以把云端的变化落到本机。
pub(super) async fn startup(at: u64, local_has_data: bool) {
  let Some(log) = LOG.with(Cell::get) else {
    return;
  };
  super::set_status(AutoStatus::Syncing);
  // 先问一次登录态：没登录就停下来说明，而不是发一个必然 401 的请求；
  // 登录的是谁也要在这里认定，基线只对它的主人有效。
  match account_sync::refresh().await {
    _ if is_stale(at) => return,
    AccountState::SignedIn(user) => adopt_owner(user.id),
    AccountState::SignedOut { .. } => return pause_signed_out(),
    AccountState::Unknown | AccountState::Unavailable => {
      return pause_pull_failed(t("tools.account-err-unavailable"));
    }
  }

  let started_at_seq = seq();
  let fresh = !local_has_data && !USER_WROTE.with(Cell::get);
  let result = round(Mode::Startup, at, log, AUTO_FETCH_TIMEOUT_MS, fresh).await;
  if is_stale(at) {
    return;
  }
  match result {
    Ok(Outcome {
      problem: Some(problem),
      ..
    }) => {
      // 云端一直在变（Busy）要上岗，否则稍后的重试没人接；冲突与大量删除则是暂停。
      if matches!(problem, Problem::Busy) {
        super::engage();
      }
      paused_by(problem);
    }
    Ok(out) => {
      if out.changed_local() {
        crate::ui::push_info(t("tools.account-auto-updated"));
      }
      // 云端有几项没能写进本机（内容不合格、没有合并规则）：它们不记基线、下次启动还会再试，
      // 说一声，免得用户以为这几项早就同步下来了。只在启动那一轮说：自动写入不碰本机，不会产生这个数。
      if out.skipped > 0 {
        crate::ui::push_info(tf(
          "tools.account-auto-skipped",
          &[&out.skipped.to_string()],
        ));
      }
      push_succeeded(started_at_seq, None);
      arm();
    }
    Err(Failure::Device(message)) => pause_pull_failed(message),
    Err(Failure::Cloud {
      op: Op::Push,
      error,
    }) => {
      // 取回已经完成、只有上传失败：本机里有没传出去的东西，上岗后按退避重试。
      update_state(|s| s.dirty = true);
      super::engage();
      push_failed(PushError::from_account(&error));
    }
    Err(Failure::Cloud { error, .. }) => {
      pause_pull_failed(tf(
        "tools.account-pull-failed",
        &[&account_sync::describe(&error)],
      ));
    }
  }
}

// ───────────────────────── 写入 ─────────────────────────

/// 数据变了之后的自动写入：只上传（见模块文档）。调用方已经置好 `PUSHING` 与「同步中」状态。
pub(super) async fn push(at: u64, sent_seq: u64, log: LogStore) {
  let result = round(
    Mode::PushOnly,
    at,
    log,
    crate::account_sync::TIMEOUT_MS,
    false,
  )
  .await;
  // 等网络期间换了通道：这次的结果属于旧通道，不能碰新通道的状态。
  if is_stale(at) {
    return;
  }
  PUSHING.with(|p| p.set(false));
  match result {
    Ok(Outcome {
      problem: Some(problem),
      ..
    }) => paused_by(problem),
    Ok(_) => push_succeeded(sent_seq, None),
    // 自动写入里不区分是哪一步失败：对用户来说都是「这次没能同步到云端」，按退避重试。
    Err(Failure::Cloud { error, .. }) => push_failed(PushError::from_account(&error)),
    Err(Failure::Device(message)) => pause_push_failed(message),
  }
}
