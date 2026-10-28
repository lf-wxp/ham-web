//! 自定义远端的自动同步：打开页面时自动获取，数据变化后自动写入。
//!
//! 「该不该覆盖、该不该停下来问用户」是纯逻辑，在 `ham_web_core::remote_sync::plan_startup`
//! （带单测）；这里只负责把它接到浏览器上：写入钩子、定时器、网络请求与状态展示。
//!
//! # 几条不能破的约束
//!
//! - **自动获取是覆盖，不是合并。** 合并对答题统计是累加的，同一份远端数据合并两次计数就翻倍，
//!   而自动获取每次打开页面都会发生。覆盖幂等，所以只在「覆盖不丢本机数据」时才做。
//! - **拿不准就停下来。** 启动时获取失败（无法确认远端现状）、两边都有改动（选哪边都会丢）、
//!   或这组接口从没同步成功过而远端看起来是空的（可能只是地址写错）时，暂停自动写入并告诉用户，
//!   绝不拿本机去盖一个没看过的远端。
//! - **取回远端期间用户的操作也算本机改动。** 启动时取远端要等网络，这段时间用户可能已经在答题；
//!   覆盖前的判定把「第一次点击 / 按键之后发生的写入」并进「本机有未同步改动」。
//!   应用自己在启动时的写入（迁移标记、每日打卡）发生在用户碰页面之前，不算——否则正常的跨设备
//!   使用会天天报冲突。
//! - **自动写入必须先「上岗」。** 本次会话里启动流程正常走完、或用户手动同步成功过，才会自动写；
//!   改接口地址时立刻撤岗，免得把数据 PUT 到敲了一半的地址上。
//! - **写入失败要分「暂时的」和「永久的」。** 网络 / 超时 / 5xx 指数退避重试；令牌、地址、体积不对
//!   这类重试无益的错误直接暂停并说明原因，等用户处理后手动同步一次再恢复。
//! - **自动同步总是同步完整数据**（日志、其他数据；QSL 影像可按配置整体去掉），与「导出模块」的勾选
//!   无关：远端是一份整体替换的快照，漏掉某个模块就等于把它从远端抹掉。
//! - **不占用备份锁去写。** 备份锁会让整个页面 `inert`，每次改动都冻结界面不可接受。
//!   自动写入只读本地数据，不需要它；只有会改写本地数据的自动获取才拿锁。

use std::cell::Cell;
use std::time::Duration;

use ham_web_core::remote_sync::{
  Fingerprint, Remote, STATE_KEY, StartupPlan, SyncState, has_user_data, is_device_local_key,
  plan_startup, retry_delay_ms,
};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::i18n::{t, tf};
use crate::pages::log::LogStore;
use crate::remote_sync::{self, AUTO_FETCH_TIMEOUT_MS, Fetched};
use crate::util::storage;

/// 数据变化后等多久再写：攒一批改动，也给用户留出改主意的余地。
const PUSH_DELAY_MS: u64 = 5_000;
/// 两次自动写入之间的最短间隔：整份快照可能很大（含 QSL 影像），不能每次作答都上传一遍。
const MIN_INTERVAL_MS: f64 = 30_000.0;
/// 备份 / 导入占着锁时，稍后再看。
const BUSY_RETRY_MS: u64 = 3_000;

/// 面向界面的自动同步状态。
#[derive(Clone, Debug, PartialEq)]
pub enum AutoStatus {
  /// 没在工作（没配接口，或刚改了地址还没同步过）。
  Off,
  /// 正在获取或写入。
  Syncing,
  /// 最近一次成功同步的本地时间（`HH:MM`）。
  Synced(String),
  /// 两边都有改动，已暂停。
  Conflict,
  /// 这组接口从没同步成功过而远端是空的：等用户手动写入一次来确认地址。
  NeedsFirstPush,
  /// 启动时获取失败，已暂停自动写入；附原因。
  PullFailed(String),
  /// 自动写入暂时失败（网络 / 超时 / 5xx），稍后重试；附原因。
  PushFailed(String),
  /// 自动写入遇到重试无益的错误（令牌 / 地址 / 体积），已暂停；附原因。
  PushPaused(String),
}

thread_local! {
  static STATUS: ArcRwSignal<AutoStatus> = ArcRwSignal::new(AutoStatus::Off);
  static STATE: Cell<SyncState> = const { Cell::new(SyncState { base: None, dirty: false }) };
  static LOG: Cell<Option<LogStore>> = const { Cell::new(None) };
  /// 允许自动写入（见模块文档「先上岗」）。
  static ARMED: Cell<bool> = const { Cell::new(false) };
  /// 因冲突 / 启动获取失败 / 写入遇到永久性错误而暂停。
  static PAUSED: Cell<bool> = const { Cell::new(false) };
  /// 已经排了一次写入。
  static TIMER: Cell<bool> = const { Cell::new(false) };
  static PUSHING: Cell<bool> = const { Cell::new(false) };
  /// 本机改动计数：写入期间又有改动时，靠它判断「这次写出去的是不是最新的」。
  static SEQ: Cell<u64> = const { Cell::new(0) };
  static LAST_PUSH_MS: Cell<f64> = const { Cell::new(0.0) };
  /// 连续失败次数：只在第一次失败时弹提示，并决定下一次重试的退避时长。
  static FAIL_STREAK: Cell<u32> = const { Cell::new(0) };
  /// 应用自己在导入时写存储，不能算成「用户改了数据」。
  static SUPPRESS: Cell<u32> = const { Cell::new(0) };
  /// 用户碰过页面（第一次指针按下 / 按键）。此前的存储写入都是应用自己的启动逻辑。
  static INTERACTED: Cell<bool> = const { Cell::new(false) };
  /// 用户碰过页面之后发生过存储写入：启动判定里要当成「本机有未同步的改动」。
  static USER_WROTE: Cell<bool> = const { Cell::new(false) };
}

/// 自动同步状态（给「数据备份」页展示）。
pub fn status() -> ArcRwSignal<AutoStatus> {
  STATUS.with(Clone::clone)
}

fn set_status(next: AutoStatus) {
  STATUS.with(|s| s.set(next));
}

fn state() -> SyncState {
  STATE.with(Cell::get)
}

/// 改状态并落盘；没变化就不写。
fn update_state(f: impl FnOnce(&mut SyncState)) {
  let before = state();
  let mut next = before;
  f(&mut next);
  if next != before {
    STATE.with(|s| s.set(next));
    storage::set_json(STATE_KEY, &next);
  }
}

fn now_ms() -> f64 {
  js_sys::Date::now()
}

fn now_hhmm() -> String {
  let d = js_sys::Date::new_0();
  format!("{:02}:{:02}", d.get_hours(), d.get_minutes())
}

fn seq() -> u64 {
  SEQ.with(Cell::get)
}

/// 导入期间屏蔽写入钩子，离开作用域自动恢复。
struct Suppress;

impl Suppress {
  fn new() -> Self {
    SUPPRESS.with(|s| s.set(s.get() + 1));
    Self
  }
}

impl Drop for Suppress {
  fn drop(&mut self) {
    SUPPRESS.with(|s| s.set(s.get().saturating_sub(1)));
  }
}

/// 启动前的本机快照（见 `plan_startup` 为什么要「启动前」）。
pub struct Snapshot {
  state: SyncState,
  local_has_data: bool,
}

/// 取启动前的本机快照。**必须在 `App` 的第一行调用**：应用自己的启动逻辑也会写存储
/// （日志 store 写默认台站档案，`ensure_seeded` 写迁移标记和默认统计），晚一步取，
/// 一个从没用过的浏览器也会因为这些默认值被当成「本机有数据」。
pub fn snapshot() -> Snapshot {
  Snapshot {
    state: storage::get_json(STATE_KEY).unwrap_or_default(),
    local_has_data: has_user_data(storage::keys().iter().map(String::as_str)),
  }
}

/// 在 `App` 根部（日志 store 就绪后）调用一次：装上写入钩子，并按配置做启动时的自动获取。
pub fn start(log: LogStore, snapshot: Snapshot) {
  LOG.with(|l| l.set(Some(log)));
  STATE.with(|s| s.set(snapshot.state));

  storage::set_write_hook(on_write);
  // 「用户碰过页面」的标记：必须用比 `click` 更早的事件——点击处理函数里的写入发生在 `click`，
  // 而 `pointerdown` / `keydown` 总在它之前，这一次写入才不会被漏掉。
  window_event_listener(leptos::ev::pointerdown, |_| {
    INTERACTED.with(|i| i.set(true));
  });
  window_event_listener(leptos::ev::keydown, |_| {
    INTERACTED.with(|i| i.set(true));
  });
  // 切走 / 关闭标签页前尽力把没写出去的改动写掉，不必等满延时。
  window_event_listener(leptos::ev::visibilitychange, move |_| {
    if crate::util::document().visibility_state() == web_sys::VisibilityState::Hidden {
      spawn_local(push_now());
    }
  });
  spawn_local(startup(snapshot.state.dirty, snapshot.local_has_data));
}

fn on_write(key: &str) {
  if SUPPRESS.with(Cell::get) > 0 || is_device_local_key(key) {
    return;
  }
  mark_dirty();
}

/// 本机数据变了：记下来，稍后自动写入。
///
/// 存储写入的钩子会自动调它；直接写 `localStorage` 的路径（备份导入）要手动调。
pub fn mark_dirty() {
  // 没配接口就不跟踪：用户没开这个功能，不该给每一次存储写入平白加开销。
  if !remote_sync::with_config(|c| c.can_pull() || c.can_push()) {
    return;
  }
  if INTERACTED.with(Cell::get) {
    USER_WROTE.with(|w| w.set(true));
  }
  SEQ.with(|s| s.set(s.get() + 1));
  update_state(|s| s.dirty = true);
  schedule_push(PUSH_DELAY_MS);
}

/// 排一次自动写入；已经排了就不重复排。
///
/// 计时从**第一次**改动算起而不是每次改动都重置：持续作答时，重置式的防抖永远等不到安静期。
fn schedule_push(delay_ms: u64) {
  if TIMER.with(|t| t.replace(true)) {
    return;
  }
  let since_last = now_ms() - LAST_PUSH_MS.with(Cell::get);
  let cooldown = (MIN_INTERVAL_MS - since_last).max(0.0) as u64;
  set_timeout(
    move || {
      TIMER.with(|t| t.set(false));
      spawn_local(push_now());
    },
    Duration::from_millis(delay_ms.max(cooldown)),
  );
}

/// 允许自动写入；有没写出去的改动就排一次。
fn arm() {
  ARMED.with(|a| a.set(true));
  PAUSED.with(|p| p.set(false));
  FAIL_STREAK.with(|f| f.set(0));
  if state().dirty {
    schedule_push(PUSH_DELAY_MS);
  }
}

/// 暂停自动写入（撤岗）。要恢复得经过一次成功的手动同步，或下次打开页面时重新判定。
fn pause() {
  PAUSED.with(|p| p.set(true));
  ARMED.with(|a| a.set(false));
}

async fn push_now() {
  if PUSHING.with(Cell::get) || !ARMED.with(Cell::get) || PAUSED.with(Cell::get) {
    return;
  }
  let cfg = remote_sync::config();
  if !cfg.can_push() || !state().dirty {
    return;
  }
  let Some(log) = LOG.with(Cell::get) else {
    return;
  };
  // 手动备份 / 导入进行中：本地数据正在被改写，等它结束再写。
  if log.backup_busy.get_untracked() {
    schedule_push(BUSY_RETRY_MS);
    return;
  }

  PUSHING.with(|p| p.set(true));
  let sent_seq = seq();
  set_status(AutoStatus::Syncing);
  // 本机内容与基线一致就不发请求：很多写入只是写了同样的值。
  let result = remote_sync::push_if_changed(&cfg, state().base).await;
  PUSHING.with(|p| p.set(false));

  match result {
    Ok(pushed) => {
      FAIL_STREAK.with(|f| f.set(0));
      if pushed.is_some() {
        LAST_PUSH_MS.with(|l| l.set(now_ms()));
      }
      // 写入期间又有改动，这次写出去的就不是最新的：保持脏位，再排一次。
      let newer = seq() != sent_seq;
      update_state(|s| {
        if let Some(p) = &pushed {
          s.base = Some(p.fp);
        }
        s.dirty = newer;
      });
      set_status(AutoStatus::Synced(now_hhmm()));
      if newer {
        schedule_push(PUSH_DELAY_MS);
      }
    }
    Err(e) if !e.retryable => pause_push_failed(e.message),
    Err(e) => {
      let streak = FAIL_STREAK.with(|f| {
        f.set(f.get() + 1);
        f.get()
      });
      set_status(AutoStatus::PushFailed(e.message.clone()));
      if streak == 1 {
        crate::ui::push_error(e.message);
      }
      schedule_push(retry_delay_ms(streak));
    }
  }
}

fn pause_conflict() {
  pause();
  set_status(AutoStatus::Conflict);
  crate::ui::push_error(t("tools.remote-auto-conflict"));
}

fn pause_pull_failed(reason: String) {
  pause();
  set_status(AutoStatus::PullFailed(reason.clone()));
  crate::ui::push_error(reason);
}

fn pause_push_failed(reason: String) {
  pause();
  FAIL_STREAK.with(|f| f.set(0));
  set_status(AutoStatus::PushPaused(reason.clone()));
  crate::ui::push_error(reason);
}

fn pause_first_push() {
  pause();
  set_status(AutoStatus::NeedsFirstPush);
  crate::ui::push_info(t("tools.remote-auto-first-push"));
}

async fn startup(stale_dirty: bool, local_has_data: bool) {
  let cfg = remote_sync::config();
  if !cfg.can_pull() && !cfg.can_push() {
    return;
  }
  if !cfg.can_pull() {
    // 只配了写入：没有可对照的远端，按配置直接开启自动写入。
    arm();
    return;
  }

  set_status(AutoStatus::Syncing);
  let (remote, body) = match remote_sync::fetch(&cfg, AUTO_FETCH_TIMEOUT_MS).await {
    Err(e) => return pause_pull_failed(e),
    Ok(Fetched::Empty) => (Remote::Empty, None),
    Ok(Fetched::Data(body)) => match remote_sync::fingerprint_of(&body) {
      Ok(fp) => (Remote::Data(fp), Some((body, fp))),
      Err(e) => return pause_pull_failed(e),
    },
  };

  // 取远端要等网络：这段时间用户可能已经在答题 / 记通联，这些改动不在启动前的快照里。
  let stale_dirty = stale_dirty || USER_WROTE.with(Cell::get);
  match plan_startup(&state(), stale_dirty, remote, local_has_data) {
    StartupPlan::InSync => finish_in_sync(),
    // 只配了获取：没有可上传的去处，不置脏位、不等写入。
    StartupPlan::Upload | StartupPlan::NeedsFirstPush if !cfg.can_push() => finish_in_sync(),
    StartupPlan::Upload => {
      update_state(|s| s.dirty = true);
      arm();
      set_status(AutoStatus::Syncing);
    }
    StartupPlan::NeedsFirstPush => pause_first_push(),
    StartupPlan::FastForward => {
      if let Some((body, fp)) = body {
        fast_forward(body, fp).await;
      }
    }
    StartupPlan::Conflict => pause_conflict(),
  }
}

fn finish_in_sync() {
  arm();
  set_status(AutoStatus::Synced(now_hhmm()));
}

/// 取一把备份锁；被手动备份 / 导入占着时等一会儿。
async fn acquire_lock(log: LogStore) -> Option<crate::util::BackupTaskGuard> {
  for _ in 0..20 {
    if let Some(task) = crate::util::begin_backup_task(log.backup_busy) {
      return Some(task);
    }
    crate::util::sleep(500).await;
  }
  None
}

/// 用远端覆盖本机（调用方已确认本机没有未同步的改动）。
async fn fast_forward(body: String, fp: Fingerprint) {
  let Some(log) = LOG.with(Cell::get) else {
    return;
  };
  let Some(task) = acquire_lock(log).await else {
    return pause_pull_failed(t("tools.backup-in-progress"));
  };
  // 等锁期间用户又改了数据：现在覆盖就会把这些改动静默吞掉。拿到锁之后页面是 `inert` 的，
  // 用户动不了了，所以这里是最后一次、也是足够的检查点。
  if USER_WROTE.with(Cell::get) {
    drop(task);
    return pause_conflict();
  }
  let result = {
    // 导入里的存储写入是在「照搬远端」，不是用户改了数据。
    let _quiet = Suppress::new();
    crate::util::import_backup(&body).await
  };
  // 无论成败都把日志 Store 对齐到存储（与手动导入一致）：失败时可能已经写了一半。
  log.refresh_from_storage().await;
  drop(task);

  match result {
    Ok(_) => {
      note_pulled(fp, false);
      crate::ui::push_info(t("tools.remote-auto-updated"));
    }
    Err(e) => pause_pull_failed(tf("tools.remote-pull-failed", &[&e])),
  }
}

/// 手动「写入远端」成功：远端现在就等于本机，可以当基线。
pub fn note_pushed(fp: Fingerprint) {
  update_state(|s| {
    s.base = Some(fp);
    s.dirty = false;
  });
  arm();
  set_status(AutoStatus::Synced(now_hhmm()));
}

/// 手动「从远端恢复 / 合并」成功，或启动时自动快进成功。
///
/// 覆盖导入是按 key 叠加的：远端没有、本机独有的 key 会留下，所以拉取之后本机并不一定等于远端；
/// 合并之后本机更是两边的并集。能写出去（配了写入接口）时一律置脏位，让下一次自动写入
/// 把差异补回远端——写出前会再比一次指纹，没有差异就不发请求。
/// 只配了获取接口时无处可写：覆盖后清掉脏位，免得它永远清不掉、下次远端一更新就被当成冲突。
pub fn note_pulled(fp: Fingerprint, merged: bool) {
  let can_push = remote_sync::with_config(|c| c.can_push());
  update_state(|s| {
    s.base = Some(fp);
    s.dirty = merged || can_push;
  });
  arm();
  set_status(AutoStatus::Synced(now_hhmm()));
}

/// 用户改了接口地址。
///
/// 旧基线属于另一组接口，不能再拿来判断；改的过程中地址往往是半截的，也不能自动往上写
/// （见模块文档「先上岗」）。重新手动同步一次、或下次打开页面，才会再次上岗。
pub fn config_changed() {
  ARMED.with(|a| a.set(false));
  PAUSED.with(|p| p.set(false));
  update_state(|s| s.base = None);
  set_status(AutoStatus::Off);
}
