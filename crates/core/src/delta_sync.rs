//! 一次按 key 的增量同步：把 [`crate::delta`] 的三方对比串成「取回 → 落到本机 → 上传」。
//!
//! 引擎对云端（[`Cloud`]）与本机（[`Device`]）都是泛型的：浏览器里它们是真实的 HTTP 请求和 `localStorage` /
//! IndexedDB，测试里是内存里的假云端和假设备——**同一份算法**既被前端使用，也能在原生环境里用多台设备
//! 反复同步来穷举验证（见本文件的测试），而不是在测试里另写一份「差不多」的。
//!
//! # 一轮的步骤
//!
//! 1. 问云端要清单，读本机现在的内容，三方对比（基线、本机、云端）；
//! 2. 有要停下来问用户的 key（[`Problem::Conflict`]）就**什么都不动**；
//! 3. （仅 [`Mode::Startup`]）把云端的变化落到本机：覆盖、合并、删除。落之前本机必须还是读快照时的样子，
//!    否则用户这期间动过数据，放弃这一轮重来；
//! 4. 重新读本机再对比一次，把本机的改动（含刚合并出的结果）按 key 上传，每个 key 带着「我看到的云端那一版」，
//!    云端对不上整批作废（[`Cloud::is_stale`]），整轮重来；
//! 5. 把每个 key 两侧的摘要记进基线，交给调用方保存。
//!
//! # 两种模式
//!
//! 页面刚打开时（[`Mode::Startup`]）用户还没动页面，可以把云端的变化落到本机。之后的自动写入
//! （[`Mode::PushOnly`]）只上传：页面里的信号已经读过存储，中途改写存储，用户下一次操作就会拿旧值盖掉它。
//! 这时遇到需要写本机的变化就留到下次打开（计入 [`Outcome::deferred`]），需要合并的 key 报
//! [`Problem::Conflict`] 请用户处理。
//!
//! # 几道保险
//!
//! - 云端是空的：基线作废，把本机全部传上去，**不**当成「云端把所有东西都删了」；
//! - 一轮要删的 key 太多（[`suspicious`]）：多半是浏览器清了站点数据而不是用户真的删了这么多，停下来问；
//! - 一轮最多重来 [`MAX_ATTEMPTS`] 次，不会在忙碌的云端上空转。

use std::collections::{BTreeMap, BTreeSet};

use crate::account::{Change, PushResponse, SyncManifest};
use crate::delta::{Action, Base, Digest, Manifest, Synced, in_scope, is_image, manifest_of, plan};
use crate::remote_sync::Fingerprint;

/// 一轮最多重来几次（本机中途被改、云端被别处抢先写）。
pub const MAX_ATTEMPTS: usize = 3;

/// 云端。
///
/// 方法不要求 `Send`：浏览器里是单线程，原生测试也不跨线程。
#[allow(async_fn_in_trait)]
pub trait Cloud {
  type Error;

  /// 云端的清单；还没有备份为 `None`。
  async fn manifest(&mut self) -> Result<Option<SyncManifest>, Self::Error>;

  /// 取指定 key 的值（云端已经没有的不在结果里）。
  async fn pull(&mut self, keys: &[String]) -> Result<BTreeMap<String, String>, Self::Error>;

  /// 按 key 上传改动。云端上有 key 已经不是改动里的 `base`，整批不生效。
  async fn push(&mut self, changes: Vec<Change>) -> Result<PushResponse, Self::Error>;

  /// 这个错误是「云端已经不是你看到的那一版」吗（`412`）。是的话整轮重来，而不是当成失败。
  fn is_stale(error: &Self::Error) -> bool;
}

/// 本机。
#[allow(async_fn_in_trait)]
pub trait Device {
  type Error;

  /// 本机现在要同步的内容。设备专属的 key 可以不去掉（引擎自己按 [`in_scope`] 过滤）；
  /// 是否带 QSL 影像由设备按自己的选项决定，并要与 [`Options::include_images`] 一致。
  async fn snapshot(&mut self) -> Result<BTreeMap<String, String>, Self::Error>;

  /// 把云端的变化落到本机。
  ///
  /// 落之前先核对：`edits` 涉及的每个 key，本机现在的摘要必须等于 `expect` 里的（`expect` 里没有 = 本机没有）。
  /// 对不上说明用户在读快照之后动了数据，**一个 key 都不要写**，返回 [`Applied::LocalChanged`]。
  async fn apply(&mut self, edits: &Edits, expect: &Manifest) -> Result<Applied, Self::Error>;

  /// 基线推进到了这一步，调用方可以先存下来。
  ///
  /// 引擎在每次落盘本机（[`Device::apply`]）之后、每一批上传成功之后调用。不这样做的话，基线要等整轮
  /// 结束才交给调用方：上传分批时，第一批已经写进云端、第二批还没发，这时关掉页面，已写进云端的 key
  /// 在本机没有基线——本机文本与云端规范文本的摘要往往不同，下次启动会被当成「两边都改了」，
  /// 累计类的 key 因此误报冲突。默认什么都不做（测试里的假设备不需要）。
  fn checkpoint(&mut self, _base: &Base) {}
}

/// 要落到本机的变化。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Edits {
  /// 直接用云端的值覆盖。
  pub set: BTreeMap<String, String>,
  /// 云端的值，与本机现有的合并（合并规则见 [`crate::backup_merge::merge_value`]）。
  pub merge: BTreeMap<String, String>,
  /// 本机也删掉。
  pub remove: Vec<String>,
}

impl Edits {
  /// 涉及的全部 key。
  pub fn keys(&self) -> impl Iterator<Item = &String> {
    self
      .set
      .keys()
      .chain(self.merge.keys())
      .chain(self.remove.iter())
  }

  /// 有没有要写的。
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.set.is_empty() && self.merge.is_empty() && self.remove.is_empty()
  }
}

/// [`Device::apply`] 的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Applied {
  /// 已写入。`skipped` 是**没有**写进去的 key（内容不合格、没有合并规则能处理……）：
  /// 引擎不给它们记基线，否则下一轮会把「本机没有」当成「用户删了」传给云端。
  Done { skipped: Vec<String> },
  /// 本机在读快照之后又变了：什么都没写，这一轮要重来。
  LocalChanged,
}

impl Applied {
  /// 全部写进去了。
  #[must_use]
  pub const fn all() -> Self {
    Self::Done {
      skipped: Vec::new(),
    }
  }
}

/// 一轮的模式（见模块文档「两种模式」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
  /// 页面刚打开：可以把云端的变化落到本机。
  Startup,
  /// 之后的自动写入：只上传。
  PushOnly,
}

/// 一轮的选项。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
  pub mode: Mode,
  /// 同步 QSL 影像吗（账号通道可以选择不同步）。
  pub include_images: bool,
  /// 本机是全新的（没有用户数据，有的只是应用自己写的默认值）：两边都有而不同的 key 以云端为准。
  pub prefer_remote: bool,
}

/// 一轮同步失败。
#[derive(Debug, PartialEq, Eq)]
pub enum Error<C, D> {
  Cloud(C),
  Device(D),
}

/// 一轮没能完成的原因。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
  /// 这些 key 两边都改了，又没有安全的合并规则（或在自动写入中需要合并）：请用户处理。
  Conflict(Vec<String>),
  /// 一次要删很多 key，多半不是用户真想删（见 [`suspicious`]）：请用户确认。
  SuspiciousDeletions(Vec<String>),
  /// 重来了 [`MAX_ATTEMPTS`] 次，本机或云端一直在变：稍后再试。
  Busy,
}

/// 一轮同步的结果。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outcome {
  /// 同步之后云端的版本号（空为 `None`）。
  pub fingerprint: Option<Fingerprint>,
  /// 用云端的值覆盖了本机的 key 数。
  pub pulled: usize,
  /// 与云端合并了的 key 数。
  pub merged: usize,
  /// 本机删掉的 key 数。
  pub removed_local: usize,
  /// 上传了的 key 数。
  pub pushed: usize,
  /// 在云端删掉的 key 数。
  pub removed_remote: usize,
  /// 需要写本机、但自动写入中不做、留到下次打开的 key 数。
  pub deferred: usize,
  /// 设备没能写进本机的 key 数（内容不合格、没有合并规则能处理……，见 [`Applied::Done`]）。
  ///
  /// 它们不记基线，下一轮还会再试一遍；这里给调用方一个数，好让界面说一声，
  /// 而不是每次启动都悄悄重试、用户永远不知道这几条没同步下来。
  pub skipped: usize,
  /// 没能完成的原因；`None` = 这一轮完成了。
  pub problem: Option<Problem>,
}

impl Outcome {
  /// 本机有没有被改写。
  #[must_use]
  pub const fn changed_local(&self) -> bool {
    self.pulled + self.merged + self.removed_local > 0
  }
}

/// 一次要删的 key 多到值得停下来问。
///
/// 浏览器清了站点数据、IndexedDB 被回收时，本机会一下子「少了很多 key」；用户自己一次删这么多几乎不会发生。
/// **不少于** [`SUSPICIOUS_MIN`] 个且超过全部 key 的一半。
#[must_use]
pub fn suspicious(deletions: usize, total: usize) -> bool {
  deletions >= SUSPICIOUS_MIN && deletions * 2 > total
}

/// 见 [`suspicious`]。
pub const SUSPICIOUS_MIN: usize = 5;

/// 一次尝试的结果。
enum Step {
  Done(Outcome),
  /// 这一轮作废，重来。`local_changed`：是因为用户在读快照之后动了数据。
  Retry {
    local_changed: bool,
  },
}

/// 跑一轮增量同步。`base` 是每个 key 的基线，会就地更新；**无论成败调用方都应把它存下来**
/// （中途失败时它记着已经完成的那部分，重复做是幂等的）。
///
/// 一轮可能重来（本机中途被改、云端被别处抢先写）。本机已经被改写的计数（取回 / 合并 / 本机删除）
/// 跨尝试累加：重来的那一轮里这些 key 两边已经一样、不会再被取回，若只数最后一次，
/// 本机明明被改写过，结果却是「没改」，界面就不会提示用户。
///
/// # Errors
///
/// 云端或本机的 I/O 失败。
pub async fn run<C: Cloud, D: Device>(
  cloud: &mut C,
  device: &mut D,
  base: &mut Base,
  opts: Options,
) -> Result<Outcome, Error<C::Error, D::Error>> {
  let mut opts = opts;
  let mut writes = LocalWrites::default();
  for _ in 0..MAX_ATTEMPTS {
    match attempt(cloud, device, base, opts, &mut writes).await {
      Ok(Step::Done(outcome)) => return Ok(writes.add_to(outcome)),
      // 用户动过数据，这台设备就不再是「只有应用默认值的全新设备」：
      // 之后两边都有而不同的 key 不能再无条件以云端为准，否则会盖掉用户刚输入的内容。
      Ok(Step::Retry { local_changed }) => opts.prefer_remote &= !local_changed,
      Err(Error::Cloud(e)) if C::is_stale(&e) => {}
      Err(e) => return Err(e),
    }
  }
  Ok(writes.add_to(Outcome {
    problem: Some(Problem::Busy),
    ..Outcome::default()
  }))
}

/// 已经落到本机的改写数，跨尝试累计（见 [`run`]）。
#[derive(Debug, Default)]
struct LocalWrites {
  pulled: usize,
  merged: usize,
  removed_local: usize,
  /// 没写成的 key（去重：同一个 key 重来几次也只算一个）。
  skipped: BTreeSet<String>,
}

impl LocalWrites {
  fn add_to(self, mut outcome: Outcome) -> Outcome {
    outcome.pulled += self.pulled;
    outcome.merged += self.merged;
    outcome.removed_local += self.removed_local;
    outcome.skipped += self.skipped.len();
    outcome
  }
}

/// 参与同步的清单。
fn scoped(manifest: &Manifest, include_images: bool) -> Manifest {
  manifest
    .iter()
    .filter(|(key, _)| in_scope(key, include_images))
    .map(|(k, d)| (k.clone(), *d))
    .collect()
}

/// 一次 `pull` 最多带多少个 key。
///
/// 服务端的上限是 [`crate::account::SYNC_MAX_KEYS`]（20000）。分批不是为了绕开它，而是让单次请求与
/// 响应都可控：影像多的账号首次同步会一次要几千个 key，一条请求带着全部 key 既慢又可能撞上代理的
/// 请求上限（撞上就是每轮都一样的失败，自己好不了）。
pub const PULL_BATCH_KEYS: usize = 256;

/// 一次 `push` 最多带多少字节的改动（key + 值）。服务端整体上限 64 MiB，这里按批留足余量，
/// 让「一个账号总量超限」不再等于「永远同步不了」。
pub const PUSH_BATCH_BYTES: usize = 8 * 1024 * 1024;

/// 一次 `push` 最多带多少条改动。
pub const PUSH_BATCH_KEYS: usize = 512;

/// 按 key 数把要取的 key 分批。
fn pull_batches(keys: &[String]) -> impl Iterator<Item = &[String]> {
  keys.chunks(PULL_BATCH_KEYS.max(1))
}

/// 把要上传的改动按字节与条数分批。单条超过字节预算的独占一批：它自己已经是能发出去的最小单位，
/// 丢掉它才是真的不同步。
fn push_batches(changes: &[Change]) -> Vec<&[Change]> {
  let mut out = Vec::new();
  let mut start = 0;
  let mut bytes = 0;
  for (i, change) in changes.iter().enumerate() {
    let size = change.key.len() + change.value.as_ref().map_or(0, String::len);
    let full = i > start && (bytes + size > PUSH_BATCH_BYTES || i - start >= PUSH_BATCH_KEYS);
    if full {
      out.push(&changes[start..i]);
      start = i;
      bytes = 0;
    }
    bytes += size;
  }
  if start < changes.len() {
    out.push(&changes[start..]);
  }
  out
}

/// 把影像冲突从「整轮都别动」里摘出来：返回 (停下来的影像, 真正要让整轮停下的冲突)。
///
/// 影像的冲突来自 [`crate::delta::plan`] 的联动规则（日志要动时影像先不传），它是**局部**的：
/// 一张本地影像不该让其它 key 的取回与上传全部停摆。日志、统计这类 key 的冲突仍然是整轮的。
fn split_image_conflicts(conflicts: Vec<String>) -> (Vec<String>, Vec<String>) {
  conflicts.into_iter().partition(|key| is_image(key))
}

/// 报告冲突时把影像的一起带上：影像不阻塞整轮，但仍要让用户知道该点「合并」。
fn join_conflicts(blocking: Vec<String>, held_images: &[String]) -> Vec<String> {
  let mut out = blocking;
  out.extend(held_images.iter().cloned());
  dedup(out)
}

/// 按首次出现的顺序去重。
fn dedup(keys: Vec<String>) -> Vec<String> {
  let mut seen = BTreeSet::new();
  keys
    .into_iter()
    .filter(|k| seen.insert(k.clone()))
    .collect()
}

async fn attempt<C: Cloud, D: Device>(
  cloud: &mut C,
  device: &mut D,
  base: &mut Base,
  opts: Options,
  writes: &mut LocalWrites,
) -> Result<Step, Error<C::Error, D::Error>> {
  // 「云端是空的」要按**未收窄**的清单判断：这次不参与同步的 key（关掉影像同步时的 `qsl-image:*`）
  // 不代表云端什么都没有。用收窄后的清单判断，会让云端只剩影像时基线被清空。
  let (remote_fp, remote, cloud_empty) = match cloud.manifest().await.map_err(Error::Cloud)? {
    Some(m) => {
      let empty = m.keys.is_empty();
      (
        Some(m.fingerprint),
        scoped(&m.keys, opts.include_images),
        empty,
      )
    }
    None => (None, Manifest::new(), true),
  };
  // 云端是空的：不把它当成「云端把所有东西都删了」。基线作废，本机的一律传上去。
  if cloud_empty {
    base.clear();
  }
  let mut out = Outcome {
    fingerprint: remote_fp,
    ..Outcome::default()
  };

  let mut local_map = device.snapshot().await.map_err(Error::Device)?;
  let mut local = manifest_of(&local_map, opts.include_images);
  let mut current = plan(base, &local, &remote, opts.prefer_remote);

  let deletions = current.deletions();
  if suspicious(deletions.len(), local.len().max(remote.len())) {
    out.problem = Some(Problem::SuspiciousDeletions(deletions));
    return Ok(Step::Done(out));
  }
  // 有真正的冲突：什么都不动，请用户处理。影像的冲突只是「先别传」，
  // 让整轮停下会把其它 key 的同步一起拖住（见 [`split_image_conflicts`]）。
  let (mut held_images, blocking) = split_image_conflicts(current.conflicts());
  if !blocking.is_empty() && opts.mode == Mode::Startup {
    out.problem = Some(Problem::Conflict(join_conflicts(blocking, &held_images)));
    return Ok(Step::Done(out));
  }
  // 被 hold 的影像也算「留到之后」：它们不阻塞这一轮，但这一轮没传。下面 `PushOnly` 还会再加上
  // 「想取回而不能写本机」的那部分——两者**累加**，不能互相覆盖（曾经后者直接赋值，前者的计数丢了）。
  out.deferred += held_images.len();

  // ── 把云端的变化落到本机（仅页面刚打开时） ──
  let writes_local = current.writes_local();
  let mut merge_left = Vec::new();
  if opts.mode == Mode::PushOnly {
    out.deferred += current
      .keys(|a| matches!(a, Action::Pull | Action::PullDelete))
      .len();
    merge_left = current.keys(|a| a == Action::Merge);
  } else if writes_local {
    let pull_keys = current.keys(|a| a == Action::Pull);
    let merge_keys = current.keys(|a| a == Action::Merge);
    let remove = current.keys(|a| a == Action::PullDelete);
    let wanted: Vec<String> = pull_keys.iter().chain(&merge_keys).cloned().collect();
    let mut values = BTreeMap::new();
    for batch in pull_batches(&wanted) {
      values.extend(cloud.pull(batch).await.map_err(Error::Cloud)?);
    }
    // 清单里有、取值时没有：中间被别处删了，重来。
    if wanted.iter().any(|k| !values.contains_key(k)) {
      return Ok(Step::Retry {
        local_changed: false,
      });
    }
    let pick = |keys: &[String]| -> BTreeMap<String, String> {
      keys
        .iter()
        .map(|k| (k.clone(), values[k].clone()))
        .collect()
    };
    let edits = Edits {
      set: pick(&pull_keys),
      merge: pick(&merge_keys),
      remove,
    };
    let expect: Manifest = edits
      .keys()
      .filter_map(|k| local.get(k).map(|d| (k.clone(), *d)))
      .collect();
    let skipped: BTreeSet<String> =
      match device.apply(&edits, &expect).await.map_err(Error::Device)? {
        Applied::Done { skipped } => skipped.into_iter().collect(),
        Applied::LocalChanged => {
          return Ok(Step::Retry {
            local_changed: true,
          });
        }
      };
    // 基线：取回来的 key，云端那份是清单里的，本机那份就是刚取回来的文本。合并的 key 也先这么记，
    // 这样合并结果（与取回来的不同）在下面重新对比时就是「本机改了」，会被传上去。
    // 没写成的 key 不记（见 [`Applied::Done`]）。
    for (key, text) in edits.set.iter().chain(&edits.merge) {
      if skipped.contains(key) {
        continue;
      }
      if let Some(&digest) = remote.get(key) {
        base.insert(
          key.clone(),
          Synced {
            remote: digest,
            local: Digest::of(text),
          },
        );
      }
    }
    for key in &edits.remove {
      base.remove(key);
    }
    let written =
      |keys: &BTreeMap<String, String>| keys.keys().filter(|k| !skipped.contains(*k)).count();
    // 记在跨尝试的计数里，不记在本次的 `out`：这一轮之后还可能因为上传被抢先而整轮重来。
    writes.pulled += written(&edits.set);
    writes.merged += written(&edits.merge);
    writes.removed_local += edits.remove.len();
    writes.skipped.extend(skipped.iter().cloned());
    // 本机已经改了、基线也跟着推进了：先存下来，别等整轮（含后面的分批上传）结束。
    device.checkpoint(base);

    // 本机变了，重新读一次再对比：合并的结果要传上去。
    local_map = device.snapshot().await.map_err(Error::Device)?;
    local = manifest_of(&local_map, opts.include_images);
    current = plan(base, &local, &remote, false);
    // 日志刚落成本机，联动规则在新计划里不会再触发；但「这些影像的编号可能撞」的理由还在。
    current.hold_as_conflict(held_images.iter().cloned());
    let (after_images, after_blocking) = split_image_conflicts(current.conflicts());
    if !after_blocking.is_empty() {
      out.problem = Some(Problem::Conflict(join_conflicts(
        after_blocking,
        &held_images,
      )));
      return Ok(Step::Done(out));
    }
    held_images.extend(after_images);
  }

  // ── 把本机的改动传上去 ──
  let changes: Vec<Change> = current
    .iter()
    .filter_map(|(key, action)| match action {
      Action::Push | Action::KeepLocal | Action::PushDelete => Some(Change {
        key: key.to_owned(),
        base: remote.get(key).copied(),
        value: local_map
          .get(key)
          .filter(|_| action != Action::PushDelete)
          .cloned(),
      }),
      _ => None,
    })
    // 本机明明没有这个 key 却要「保留本机」：就是删除。
    .collect();
  let mut remote_now = remote.clone();
  // 按批上传：一个账号的改动总量可能超过单次请求的上限，一条请求带全部改动就是「每轮都失败、
  // 自己好不了」。每批各自带着自己那几个 key 的 `base`，服务端仍然逐 key 检查，语义不变。
  for batch in push_batches(&changes) {
    let response = cloud.push(batch.to_vec()).await.map_err(Error::Cloud)?;
    // 回执里的清单是**全量**的（含前面几批），所以后一批覆盖前一批是对的。
    remote_now = scoped(&response.keys, opts.include_images);
    out.fingerprint = Some(response.info.fingerprint);
    for change in batch {
      match &change.value {
        Some(text) => {
          out.pushed += 1;
          if let Some(&digest) = remote_now.get(&change.key) {
            base.insert(
              change.key.clone(),
              Synced {
                remote: digest,
                local: Digest::of(text),
              },
            );
          }
        }
        None => {
          out.removed_remote += 1;
          base.remove(&change.key);
        }
      }
    }
    // 这一批已经在云端生效、基线也记上了：马上存下来。后面的批要是在路上被打断（关页、断网），
    // 前面这些 key 不会在下次启动时因为没有基线而被当成「两边都改了」。
    device.checkpoint(base);
  }

  // ── 两边已经一样的 key：基线对齐到现在；两边都没有的：基线清掉 ──
  for (key, digest) in &local {
    if remote_now.get(key) == Some(digest) {
      base.insert(
        key.clone(),
        Synced {
          remote: *digest,
          local: *digest,
        },
      );
    }
  }
  let present: BTreeSet<&String> = local.keys().chain(remote_now.keys()).collect();
  // 只清「本轮范围内、两边都已经没有」的 key。范围外的 key（这次没参与同步，例如关掉了影像同步）
  // 这一轮根本没看：删掉它的基线，下次打开时会把「只有云端改了」判成「两边都改了」。
  base.retain(|key, _| !in_scope(key, opts.include_images) || present.contains(key));

  // 影像的 hold 也在这时报告：它不阻塞这一轮，但用户得知道要手点「合并」。
  let mut conflict_keys = merge_left;
  conflict_keys.extend(held_images);
  conflict_keys.extend(current.conflicts());
  let conflict_keys = dedup(conflict_keys);
  if !conflict_keys.is_empty() {
    out.problem = Some(Problem::Conflict(conflict_keys));
  }
  Ok(Step::Done(out))
}

#[cfg(test)]
mod tests;
