//! 自动同步：打开页面时自动获取，数据变化后自动写入。
//!
//! 有两条同步通道（自定义远端、在线账号，见 `ham_web_core::sync_channel`），**自动同步一次只跑一条**，
//! 由「通道偏好 + 有没有配自定义接口」决定（`resolve`）；另一条仍可手动「写入 / 恢复 / 合并」。
//! 两条通道各存各的同步状态（基线与脏位），互不覆盖。
//!
//! 两条通道判定「谁覆盖谁」的办法不同，所以各走各的分支：
//!
//! - **自定义远端**（本文件）：对端只是用户配的两个接口，能拿到的只有整份内容，基线是它的指纹，
//!   「该不该覆盖、该不该停下来问用户」是纯逻辑，在 `ham_web_core::remote_sync::plan_startup`（带单测）。
//! - **在线账号**（`account` 子模块）：对端是我们自己的服务，按 key 对比、按 key 传，
//!   算法在 `ham_web_core::delta_sync`。
//!
//! 本文件负责两条通道共用的部分：写入钩子、定时器、上岗 / 暂停、状态展示，以及通道切换。
//!
//! # 几条不能破的约束
//!
//! - **自定义远端的自动获取是覆盖，不是合并。** 合并对答题统计是累加的，同一份远端数据合并两次计数就翻倍，
//!   而自动获取每次打开页面都会发生。覆盖幂等，所以只在「覆盖不丢本机数据」时才做。
//!   （账号通道逐 key 处理：能安全合并的才合并，累计类的 key 两边都改了就停下来问。）
//! - **拿不准就停下来。** 启动时获取失败（无法确认远端现状）、两边都有改动（选哪边都会丢）、
//!   或这组接口从没同步成功过而远端看起来是空的（可能只是地址写错）时，暂停自动写入并告诉用户，
//!   绝不拿本机去盖一个没看过的远端。
//! - **取回远端期间用户的操作也算本机改动**（自定义远端）。启动时取远端要等网络，这段时间用户可能已经在答题；
//!   覆盖前的判定把「第一次点击 / 按键之后发生的写入」并进「本机有未同步改动」。
//!   应用自己在启动时的写入（迁移标记、每日打卡）发生在用户碰页面之前，不算——否则正常的跨设备
//!   使用会天天报冲突。账号通道不需要这套簿记：落盘前逐 key 核对本机还是不是读快照时的样子。
//! - **自动写入必须先「上岗」。** 本次会话里启动流程正常走完、或用户手动同步成功过，才会自动写；
//!   改接口地址时立刻撤岗，免得把数据 PUT 到敲了一半的地址上。
//! - **写入失败要分「暂时的」和「永久的」。** 网络 / 超时 / 5xx 指数退避重试；令牌、地址、体积不对
//!   这类重试无益的错误直接暂停并说明原因，等用户处理后手动同步一次再恢复。
//! - **自动同步总是同步完整数据**（日志、其他数据；QSL 影像可按配置整体去掉），与「导出模块」的勾选
//!   无关：漏掉某个模块就等于把它从对端抹掉。
//! - **不占用备份锁去写。** 备份锁会让整个页面 `inert`，每次改动都冻结界面不可接受。
//!   自动写入只读本地数据，不需要它；只有会改写本地数据的自动获取才拿锁。
//! - **换通道、换账号都不沿用旧基线。** 通道不在跑时本机的改动没人追踪，旧基线会把「期间改过」
//!   当成「没改过」；离开通道时作废它（`ham_web_core::sync_channel::leave` / `AccountSync::leave`），
//!   账号基线还记着主人（`AccountSync::adopt_owner`）。异步任务带「代」号（`GEN`）：换通道后，
//!   旧通道还在路上的请求回来也不许改新通道的状态。

mod account;

use std::cell::{Cell, RefCell};
use std::time::Duration;

use ham_web_core::delta::Base;
use ham_web_core::remote_sync::{
  Fingerprint, Remote, StartupPlan, SyncState, has_user_data, is_device_local_key, plan_startup,
  retry_delay_ms,
};
use ham_web_core::sync_channel::{
  AccountSync, CHANNEL_KEY, ChannelPref, SyncChannel, leave, pref_after_login, pref_after_logout,
  resolve,
};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::account_sync;
use crate::i18n::{t, tf};
use crate::pages::log::LogStore;
use crate::remote_sync::{self, AUTO_FETCH_TIMEOUT_MS, Fetched, PushError};
use crate::sync_io;
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
  /// 在线账号通道：没登录（或登录已过期），已暂停；登录后自动恢复。
  SignedOut,
  /// 在线账号通道：本机比上次同步时少了很多数据（多半是浏览器清了站点数据），已暂停，等用户确认。
  MassDeletion,
}

thread_local! {
  static STATUS: ArcRwSignal<AutoStatus> = ArcRwSignal::new(AutoStatus::Off);
  /// 正在自动同步的通道（给界面显示「谁在自动同步」）。
  static ACTIVE_SIGNAL: ArcRwSignal<Option<SyncChannel>> = ArcRwSignal::new(None);
  static ACTIVE: Cell<Option<SyncChannel>> = const { Cell::new(None) };
  /// 正在跑的通道的状态。账号通道只用其中的 `owner` 与 `dirty`，基线在 [`KEYS`]。
  static STATE: Cell<SyncState> = const { Cell::new(SyncState { base: None, owner: None, dirty: false }) };
  /// 在线账号通道每个 key 上次同步成功时两侧的摘要（只在账号通道跑时有内容）。
  /// **所有修改都走 [`set_keys`]**：它负责落盘并推进 [`KEYS_EPOCH`]。
  static KEYS: RefCell<Base> = const { RefCell::new(Base::new()) };
  /// [`KEYS`] 被整体替换的次数。一轮同步在路上时手动上传 / 恢复、登出、换账号都会替换基线，
  /// 这一轮跑完发现代号变了，就不能再用自己手里那份旧的把它们的结果盖掉。
  static KEYS_EPOCH: Cell<u64> = const { Cell::new(0) };
  static LOG: Cell<Option<LogStore>> = const { Cell::new(None) };
  /// 通道「代」号：换通道 / 重新启动时加一。在路上的异步任务回来先比一下，不一致就丢弃结果。
  static GEN: Cell<u64> = const { Cell::new(0) };
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
  ///
  /// **不论有没有通道在自动同步都记**：登录是运行中才发生的事，那一刻要知道「用户在本次打开之后
  /// 动过数据没有」，而不是只能拿一份此刻的存储快照——此刻的存储里已经有应用自己启动时写的默认值，
  /// 拿它判断，一个全新浏览器也会被当成「用过的」，永远走不到「直接取回云端」。
  static USER_WROTE: Cell<bool> = const { Cell::new(false) };
  /// 本页打开前，本机有没有用户数据（`snapshot` 在应用写任何默认值之前取的）。
  static BOOT_HAS_DATA: Cell<bool> = const { Cell::new(false) };
}

/// 自动同步状态（给「数据备份」页展示）。
pub fn status() -> ArcRwSignal<AutoStatus> {
  STATUS.with(Clone::clone)
}

/// 正在自动同步的通道；都没开为 `None`。
pub fn active_channel() -> ArcRwSignal<Option<SyncChannel>> {
  ACTIVE_SIGNAL.with(Clone::clone)
}

fn set_status(next: AutoStatus) {
  STATUS.with(|s| s.set(next));
}

fn active() -> Option<SyncChannel> {
  ACTIVE.with(Cell::get)
}

fn set_active(next: Option<SyncChannel>) {
  ACTIVE.with(|a| a.set(next));
  ACTIVE_SIGNAL.with(|s| s.set(next));
}

fn state() -> SyncState {
  STATE.with(Cell::get)
}

/// 把状态写到某条通道的存储 key 下。
///
/// 账号通道存的是另一种结构（[`AccountSync`]，基线是每个 key 一对摘要），所以脏位与主人取自 `state`，
/// 基线取自 [`KEYS`]。
fn persist(channel: SyncChannel, state: &SyncState) {
  match channel {
    SyncChannel::Custom => storage::set_json(channel.state_key(), state),
    SyncChannel::Account => storage::set_json(
      channel.state_key(),
      &AccountSync {
        owner: state.owner,
        dirty: state.dirty,
        keys: KEYS.with(|k| k.borrow().clone()),
      },
    ),
  }
}

/// 改状态并落盘（落在正在跑的通道名下）；没变化就不写。
fn update_state(f: impl FnOnce(&mut SyncState)) {
  let before = state();
  let mut next = before;
  f(&mut next);
  if next != before {
    STATE.with(|s| s.set(next));
    if let Some(channel) = active() {
      persist(channel, &next);
    }
  }
}

fn load_state(channel: SyncChannel) -> SyncState {
  match channel {
    SyncChannel::Custom => storage::get_json(channel.state_key()).unwrap_or_default(),
    SyncChannel::Account => {
      let stored: AccountSync = storage::get_json(channel.state_key()).unwrap_or_default();
      SyncState {
        base: None,
        owner: stored.owner,
        dirty: stored.dirty,
      }
    }
  }
}

/// 账号通道存下来的基线；没存过 / 读不出来（含升级前存的旧格式）就是空的，即「从没同步过」。
fn load_keys() -> Base {
  storage::get_json::<AccountSync>(SyncChannel::Account.state_key())
    .unwrap_or_default()
    .keys
}

/// 整体替换账号通道的基线并落盘。
fn set_keys(keys: Base) {
  KEYS.with(|k| *k.borrow_mut() = keys);
  KEYS_EPOCH.with(|e| e.set(e.get() + 1));
  if active() == Some(SyncChannel::Account) {
    persist(SyncChannel::Account, &state());
  }
}

/// 一轮同步进行到一半时把基线存下来（引擎每次落盘本机、每批上传成功之后调用）。
///
/// 与 [`set_keys`] 的区别：**不推进 [`KEYS_EPOCH`]**——这一轮自己的中间结果不该让这一轮觉得
/// 「基线被别人换过了」；同时要求 `epoch` 没变：手动上传 / 恢复、登出、换账号在路上替换了基线，
/// 它们的结果更新，这一轮手里的旧基线不能把它们盖掉（与轮末的 `set_keys` 同一个判据）。
fn checkpoint_keys(epoch: u64, keys: &Base) {
  if KEYS_EPOCH.with(Cell::get) != epoch || active() != Some(SyncChannel::Account) {
    return;
  }
  KEYS.with(|k| *k.borrow_mut() = keys.clone());
  persist(SyncChannel::Account, &state());
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

fn generation() -> u64 {
  GEN.with(Cell::get)
}

fn bump_generation() -> u64 {
  GEN.with(|g| {
    g.set(g.get() + 1);
    g.get()
  })
}

/// 异步任务发出时的那一代已经过去了（换了通道、或重新启动过）。
fn is_stale(generation_at_start: u64) -> bool {
  generation() != generation_at_start
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

// ───────────────────────── 通道偏好 ─────────────────────────

fn pref() -> Option<ChannelPref> {
  storage::get(CHANNEL_KEY).and_then(|v| ChannelPref::parse(&v))
}

fn save_pref(pref: Option<ChannelPref>) {
  match pref {
    Some(p) => storage::set(CHANNEL_KEY, p.as_str()),
    None => storage::remove(CHANNEL_KEY),
  }
}

/// 配了自定义接口（读或写任一个）：没选过通道时，这就是「自定义远端在自动同步」的依据。
fn custom_configured() -> bool {
  remote_sync::with_config(|c| c.can_pull() || c.can_push())
}

fn resolved_channel() -> Option<SyncChannel> {
  resolve(pref(), custom_configured())
}

// ───────────────────────── 启动 ─────────────────────────

/// 启动前的本机快照（见 `plan_startup` 为什么要「启动前」）。
pub struct Snapshot {
  channel: Option<SyncChannel>,
  state: SyncState,
  local_has_data: bool,
}

/// 取启动前的本机快照。**必须在 `App` 的第一行调用**：应用自己的启动逻辑也会写存储
/// （日志 store 写默认台站档案，`ensure_seeded` 写迁移标记和默认统计），晚一步取，
/// 一个从没用过的浏览器也会因为这些默认值被当成「本机有数据」。
pub fn snapshot() -> Snapshot {
  let channel = resolved_channel();
  Snapshot {
    channel,
    state: channel.map(load_state).unwrap_or_default(),
    local_has_data: has_user_data(storage::keys().iter().map(String::as_str)),
  }
}

/// 在 `App` 根部（日志 store 就绪后）调用一次：装上写入钩子，并按配置做启动时的自动获取。
pub fn start(log: LogStore, snapshot: Snapshot) {
  LOG.with(|l| l.set(Some(log)));
  BOOT_HAS_DATA.with(|b| b.set(snapshot.local_has_data));
  set_active(snapshot.channel);
  STATE.with(|s| s.set(snapshot.state));
  if snapshot.channel == Some(SyncChannel::Account) {
    set_keys(load_keys());
  }

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
  if let Some(channel) = snapshot.channel {
    let generation = bump_generation();
    spawn_local(startup(
      channel,
      generation,
      snapshot.state.dirty,
      snapshot.local_has_data,
    ));
  }
}

fn on_write(key: &str) {
  if SUPPRESS.with(Cell::get) > 0 || is_device_local_key(key) {
    return;
  }
  if INTERACTED.with(Cell::get) {
    USER_WROTE.with(|w| w.set(true));
  }
  mark_dirty();
}

/// 此刻「本机有没有用户数据」：打开页面前就有，或打开之后用户写过。
fn local_has_data_now() -> bool {
  BOOT_HAS_DATA.with(Cell::get) || USER_WROTE.with(Cell::get)
}

/// 本机数据变了：记下来，稍后自动写入。
///
/// 存储写入的钩子会自动调它；直接写 `localStorage` 的路径（备份导入）要手动调。
pub fn mark_dirty() {
  // 没有通道在自动同步就不跟踪：用户没开这个功能，不该给每一次存储写入平白加开销。
  if active().is_none() {
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

/// 允许自动写入，但不动失败计数、也不排写入：给「刚启动就上传失败、要接着重试」用。
fn engage() {
  ARMED.with(|a| a.set(true));
  PAUSED.with(|p| p.set(false));
}

/// 允许自动写入；有没写出去的改动就排一次。
fn arm() {
  engage();
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
  let Some(channel) = active() else {
    return;
  };
  if !sync_io::can_push(channel) || !state().dirty {
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
  let at = generation();
  let sent_seq = seq();
  set_status(AutoStatus::Syncing);
  match channel {
    SyncChannel::Account => account::push(at, sent_seq, log).await,
    SyncChannel::Custom => push_custom(at, sent_seq).await,
  }
}

/// 自定义远端的自动写入：本机内容的指纹与基线相同就不发请求（很多写入只是写了同样的值）。
async fn push_custom(at: u64, sent_seq: u64) {
  let result = remote_sync::push_if_changed(&remote_sync::config(), state().base).await;
  // 等网络期间换了通道：这次的结果属于旧通道，不能碰新通道的状态（`PUSHING` 也早已被换通道时重置）。
  if is_stale(at) {
    return;
  }
  PUSHING.with(|p| p.set(false));
  match result {
    Ok(pushed) => push_succeeded(sent_seq, pushed.map(|p| p.fp)),
    Err(e) => push_failed(e),
  }
}

/// 一次自动写入成功（含「没有差异所以没发请求」）。
///
/// `base` 是自定义远端写出内容的指纹；账号通道的基线已经由那一轮自己存好，传 `None`。
fn push_succeeded(sent_seq: u64, base: Option<Fingerprint>) {
  FAIL_STREAK.with(|f| f.set(0));
  // 不论有没有东西写出去都算一次：账号通道每轮至少发一次清单请求（`GET /sync/manifest`），
  // 只在「真的写了」时才刷新冷却，会让「没什么可推」的轮次把 30 s 下限白白跳过。
  LAST_PUSH_MS.with(|l| l.set(now_ms()));
  // 写入期间又有改动，这次写出去的就不是最新的：保持脏位，再排一次。
  let newer = seq() != sent_seq;
  update_state(|s| {
    if let Some(fp) = base {
      s.base = Some(fp);
    }
    s.dirty = newer;
  });
  set_status(AutoStatus::Synced(now_hhmm()));
  if newer {
    schedule_push(PUSH_DELAY_MS);
  }
}

/// 一次自动写入失败：重试无益的暂停并说明原因，其余按退避重试。
fn push_failed(e: PushError) {
  if !e.retryable {
    return pause_push_failed(e.message);
  }
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

/// 账号通道出问题时顺手问一次登录态：多半是会话过期，面板应当回到登录表单。
fn recheck_account_session() {
  if active() == Some(SyncChannel::Account) {
    spawn_local(async {
      account_sync::refresh().await;
    });
  }
}

fn pause_conflict() {
  pause();
  set_status(AutoStatus::Conflict);
  crate::ui::push_error(match active() {
    Some(SyncChannel::Account) => t("tools.account-auto-conflict"),
    _ => t("tools.remote-auto-conflict"),
  });
}

fn pause_pull_failed(reason: String) {
  pause();
  set_status(AutoStatus::PullFailed(reason.clone()));
  crate::ui::push_error(reason);
  recheck_account_session();
}

fn pause_push_failed(reason: String) {
  pause();
  FAIL_STREAK.with(|f| f.set(0));
  set_status(AutoStatus::PushPaused(reason.clone()));
  crate::ui::push_error(reason);
  recheck_account_session();
}

fn pause_first_push() {
  pause();
  set_status(AutoStatus::NeedsFirstPush);
  crate::ui::push_info(t("tools.remote-auto-first-push"));
}

fn pause_signed_out() {
  pause();
  set_status(AutoStatus::SignedOut);
  crate::ui::push_info(t("tools.account-auto-signed-out"));
}

async fn startup(channel: SyncChannel, at: u64, stale_dirty: bool, local_has_data: bool) {
  match channel {
    SyncChannel::Account => account::startup(at, local_has_data).await,
    SyncChannel::Custom => startup_custom(at, stale_dirty, local_has_data).await,
  }
}

/// 自定义远端的启动判定：取回整份内容，拿指纹与基线对比。
async fn startup_custom(at: u64, stale_dirty: bool, local_has_data: bool) {
  let channel = SyncChannel::Custom;
  let (can_pull, can_push) = (sync_io::can_pull(channel), sync_io::can_push(channel));
  if !can_pull && !can_push {
    return;
  }
  if !can_pull {
    // 只配了写入：没有可对照的远端，按配置直接开启自动写入。
    arm();
    return;
  }

  set_status(AutoStatus::Syncing);
  let config = remote_sync::config();
  let fetched = remote_sync::fetch(&config, AUTO_FETCH_TIMEOUT_MS).await;
  if is_stale(at) {
    return;
  }
  let (remote, body) = match fetched {
    Err(e) => return pause_pull_failed(e),
    Ok(Fetched::Empty) => (Remote::Empty, None),
    Ok(Fetched::Data(body)) => match remote_sync::fingerprint_of(&body) {
      Ok(fp) => (Remote::Data(fp), Some((body, fp))),
      Err(e) => return pause_pull_failed(e),
    },
  };

  // 取远端要等网络：这段时间用户可能已经在答题 / 记通联，这些改动不在启动前的快照里。
  let stale_dirty = stale_dirty || USER_WROTE.with(Cell::get);
  // 从没同步过（或基线被作废：换了通道），本机又有数据：先看看两边其实是不是一样的。
  // 内容相同就没有谁覆盖谁的问题，直接认作基线；否则每台装过同一份数据的设备
  // 都要被问一遍「谁覆盖谁」。只在这条少见的路径上才付出「生成一遍同步内容」的代价。
  if state().base.is_none()
    && local_has_data
    && let Remote::Data(fp) = remote
  {
    let same = remote_sync::local_fingerprint(&config).await == Some(fp);
    if is_stale(at) {
      return;
    }
    if same {
      update_state(|s| {
        s.base = Some(fp);
        s.dirty = false;
      });
      return finish_in_sync();
    }
  }
  match plan_startup(&state(), stale_dirty, remote, local_has_data) {
    StartupPlan::InSync => finish_in_sync(),
    // 只配了获取：没有可上传的去处，不置脏位、不等写入。
    StartupPlan::Upload | StartupPlan::NeedsFirstPush if !can_push => finish_in_sync(),
    StartupPlan::Upload => {
      update_state(|s| s.dirty = true);
      arm();
      set_status(AutoStatus::Syncing);
    }
    StartupPlan::NeedsFirstPush => pause_first_push(),
    StartupPlan::FastForward => {
      if let Some((body, fp)) = body {
        fast_forward(at, body, fp).await;
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

/// 用自定义远端覆盖本机（调用方已确认本机没有未同步的改动）。
async fn fast_forward(at: u64, body: String, fp: Fingerprint) {
  let Some(log) = LOG.with(Cell::get) else {
    return;
  };
  let Some(task) = acquire_lock(log).await else {
    return pause_pull_failed(t("tools.backup-in-progress"));
  };
  // 等锁期间换了通道，或用户又改了数据：现在覆盖就会用旧通道的内容盖掉别的东西，或把这些改动静默吞掉。
  // 拿到锁之后页面是 `inert` 的，用户动不了了，所以这里是最后一次、也是足够的检查点。
  if is_stale(at) {
    drop(task);
    return;
  }
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
      note_pulled(SyncChannel::Custom, fp, None, false);
      crate::ui::push_info(t("tools.remote-auto-updated"));
    }
    Err(e) => pause_pull_failed(tf("tools.remote-pull-failed", &[&e])),
  }
}

/// 账号通道的基线要记主人（见 `AccountSync::adopt_owner`）；自定义远端没有。
fn owner_of(channel: SyncChannel) -> Option<u64> {
  match channel {
    SyncChannel::Account => account_sync::current_user_id(),
    SyncChannel::Custom => None,
  }
}

/// 手动「写入」成功：对端现在就等于本机，可以当基线。
///
/// 自定义远端传整份内容的指纹 `fp`；账号通道传每个 key 的基线 `base`（读不出来为 `None`，
/// 下一轮自动同步会当成从没同步过逐个 key 对比）。
///
/// 只有正在自动同步的通道才记基线：另一条通道的手动操作不该改动它的状态——
/// 它不在跑，本机之后的改动没人追踪，留下的基线只会在将来误导判断（见 `leave`）。
pub fn note_pushed(channel: SyncChannel, fp: Fingerprint, base: Option<Base>) {
  if active() != Some(channel) {
    return;
  }
  match channel {
    SyncChannel::Custom => update_state(|s| {
      s.base = Some(fp);
      s.owner = None;
      s.dirty = false;
    }),
    SyncChannel::Account => {
      set_keys(base.unwrap_or_default());
      update_state(|s| {
        s.owner = owner_of(channel);
        s.dirty = false;
      });
    }
  }
  arm();
  set_status(AutoStatus::Synced(now_hhmm()));
}

/// 手动「从对端恢复 / 合并」成功，或启动时自动快进成功。
///
/// 覆盖导入是按 key 叠加的：对端没有、本机独有的 key 会留下，所以拉取之后本机并不一定等于对端；
/// 合并之后本机更是两边的并集。能写出去（配了写入接口）时一律置脏位，让下一次自动写入
/// 把差异补回对端——写出前会再比一次（自定义远端比指纹，账号逐个 key 比），没有差异就不发请求。
/// 只配了获取接口时无处可写：覆盖后清掉脏位，免得它永远清不掉、下次远端一更新就被当成冲突。
///
/// 通道不是正在自动同步的那条时不记（理由同 [`note_pushed`]）。
pub fn note_pulled(channel: SyncChannel, fp: Fingerprint, base: Option<Base>, merged: bool) {
  if active() != Some(channel) {
    return;
  }
  let can_push = sync_io::can_push(channel);
  match channel {
    SyncChannel::Custom => update_state(|s| {
      s.base = Some(fp);
      s.owner = None;
      s.dirty = merged || can_push;
    }),
    SyncChannel::Account => {
      set_keys(base.unwrap_or_default());
      update_state(|s| {
        s.owner = owner_of(channel);
        s.dirty = merged || can_push;
      });
    }
  }
  arm();
  set_status(AutoStatus::Synced(now_hhmm()));
}

// ───────────────────────── 通道切换 ─────────────────────────

/// 把自动同步切到 `next`。
///
/// 通道变了：作废旧通道的基线（`leave`）、换上新通道自己的状态、重置运行时标志并换「代」。
/// `restart`：新通道马上做一次启动判定（用户主动开启 / 刚登录时要；只是在后台改了配置时不要）。
fn activate(next: Option<SyncChannel>, restart: bool) {
  let prev = active();
  if prev != next {
    if let Some(prev) = prev {
      if prev == SyncChannel::Account {
        set_keys(Base::new());
      }
      persist(prev, &leave(state()));
    }
    set_active(next);
    STATE.with(|s| s.set(next.map(load_state).unwrap_or_default()));
    if next == Some(SyncChannel::Account) {
      set_keys(load_keys());
    }
    ARMED.with(|a| a.set(false));
    PAUSED.with(|p| p.set(false));
    PUSHING.with(|p| p.set(false));
    FAIL_STREAK.with(|f| f.set(0));
    set_status(AutoStatus::Off);
  }
  if !restart {
    if prev != next {
      bump_generation();
    }
    return;
  }
  let Some(channel) = next else {
    bump_generation();
    return;
  };
  // 重新启动（含「通道没变、但刚登录」）：旧的在途任务作废，从头判定。
  // 在途的写入回来发现自己过期会直接返回、不会替它复位 `PUSHING`：通道没变时上面那段重置被跳过了，
  // 这里不复位，`PUSHING` 会永远是真，之后所有自动写入都被它拦下，界面却停在"同步中"。
  ARMED.with(|a| a.set(false));
  PAUSED.with(|p| p.set(false));
  PUSHING.with(|p| p.set(false));
  FAIL_STREAK.with(|f| f.set(0));
  let at = bump_generation();
  spawn_local(startup(channel, at, state().dirty, local_has_data_now()));
}

/// 用户在面板里选了「谁来自动同步」（`None` = 清掉选择，回到默认规则）。
pub fn set_pref(pref: Option<ChannelPref>) {
  save_pref(pref);
  let next = resolved_channel();
  activate(next, next != active());
}

/// 登录 / 注册成功。
///
/// 还没有通道在自动同步时（见 `pref_after_login`）顺手选上账号；账号通道正在跑（会话过期后重新登录）
/// 则重新做一次启动判定。
pub fn account_signed_in() {
  save_pref(pref_after_login(pref(), custom_configured()));
  let next = resolved_channel();
  let restart = next == Some(SyncChannel::Account);
  activate(next, restart);
}

/// 退出登录 / 注销账号成功：账号通道没了去处，作废它的基线并回到默认规则。
///
/// 不论账号通道是不是正在跑都要清：它不在跑的时候本机的改动没人追踪，留着旧基线，
/// 下次登录（哪怕是同一个账号）就会把「期间改过」当成「没改过」（见 `AccountSync::leave`）。
pub fn account_signed_out() {
  save_pref(pref_after_logout(pref()));
  let account_state = if active() == Some(SyncChannel::Account) {
    state()
  } else {
    load_state(SyncChannel::Account)
  };
  // 先清内存里的基线，再按「离开」后的状态落盘：落盘的内容取自这两处。
  set_keys(Base::new());
  persist(SyncChannel::Account, &leave(account_state));
  let next = resolved_channel();
  activate(next, next != active());
}

/// 用户改了自定义接口地址。
///
/// 旧基线属于另一组接口，不能再拿来判断；改的过程中地址往往是半截的，也不能自动往上写
/// （见模块文档「先上岗」）。重新手动同步一次、或下次打开页面，才会再次上岗。
/// 地址从无到有 / 从有到无时，「自定义远端是不是在自动同步」也跟着变（没选过通道时由它是否配好决定）。
pub fn config_changed() {
  activate(resolved_channel(), false);
  if active() == Some(SyncChannel::Custom) {
    ARMED.with(|a| a.set(false));
    PAUSED.with(|p| p.set(false));
    update_state(|s| s.base = None);
    set_status(AutoStatus::Off);
  }
}
