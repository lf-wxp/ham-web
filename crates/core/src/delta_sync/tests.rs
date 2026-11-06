//! 增量同步引擎的测试：假云端 + 假设备，跑**同一份**生产算法。
//!
//! 重点不是单步行为（`delta` 的三方对比已经逐行测过），而是**多台设备反复同步**之后的性质：
//! 收敛、幂等、不丢数据、各改各的互不打扰。

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use super::*;
use crate::account::{BackupInfo, Change, PushResponse, SyncManifest};
use crate::delta::{Resolution, manifest_fingerprint, resolution};

/// 假云端和假设备从不真正挂起，所以轮询到底即可，不需要执行器。
fn block_on<F: Future>(future: F) -> F::Output {
  let mut future = pin!(future);
  let mut cx = Context::from_waker(Waker::noop());
  loop {
    if let Poll::Ready(v) = future.as_mut().poll(&mut cx) {
      return v;
    }
  }
}

// ───────────────────────── 假云端 ─────────────────────────

#[derive(Default)]
struct CloudState {
  data: BTreeMap<String, String>,
  exists: bool,
  clock: i64,
  /// 记下每一次上传，测试用来数「传了几个 key」。
  pushes: Vec<Vec<String>>,
  pulls: usize,
  /// 下一次 `push` 之前，先让「另一台设备」抢先改这些 key（模拟并发）。
  race: Option<BTreeMap<String, String>>,
  /// 下一次取值之前，先让另一台设备把这些 key 删掉。
  vanish: Vec<String>,
}

#[derive(Clone, Default)]
struct FakeCloud(Rc<RefCell<CloudState>>);

#[derive(Debug, PartialEq, Eq)]
enum CloudError {
  Stale,
  Down,
}

impl FakeCloud {
  fn manifest_now(state: &CloudState) -> Manifest {
    state
      .data
      .iter()
      .filter(|(k, _)| in_scope(k, true))
      .map(|(k, v)| (k.clone(), Digest::of(v)))
      .collect()
  }

  fn put(&self, pairs: &[(&str, &str)]) {
    let mut s = self.0.borrow_mut();
    for (k, v) in pairs {
      s.data.insert((*k).to_owned(), (*v).to_owned());
    }
    s.exists = true;
  }

  fn data(&self) -> BTreeMap<String, String> {
    self.0.borrow().data.clone()
  }

  fn pushes(&self) -> Vec<Vec<String>> {
    self.0.borrow().pushes.clone()
  }

  fn pulls(&self) -> usize {
    self.0.borrow().pulls
  }
}

impl Cloud for FakeCloud {
  type Error = CloudError;

  async fn manifest(&mut self) -> Result<Option<SyncManifest>, CloudError> {
    let s = self.0.borrow();
    if !s.exists {
      return Ok(None);
    }
    let keys = Self::manifest_now(&s);
    Ok(Some(SyncManifest {
      fingerprint: manifest_fingerprint(&keys),
      updated_at: s.clock,
      size: 0,
      keys,
    }))
  }

  async fn pull(&mut self, keys: &[String]) -> Result<BTreeMap<String, String>, CloudError> {
    let mut s = self.0.borrow_mut();
    let gone = std::mem::take(&mut s.vanish);
    for k in gone {
      s.data.remove(&k);
    }
    s.pulls += 1;
    Ok(
      keys
        .iter()
        .filter_map(|k| s.data.get(k).map(|v| (k.clone(), v.clone())))
        .collect(),
    )
  }

  async fn push(&mut self, changes: Vec<Change>) -> Result<PushResponse, CloudError> {
    let mut s = self.0.borrow_mut();
    if let Some(race) = s.race.take() {
      s.data.extend(race);
    }
    let current = Self::manifest_now(&s);
    if changes
      .iter()
      .any(|c| current.get(&c.key).copied() != c.base)
    {
      return Err(CloudError::Stale);
    }
    s.pushes
      .push(changes.iter().map(|c| c.key.clone()).collect());
    for c in changes {
      match c.value {
        Some(v) => {
          s.data.insert(c.key, v);
        }
        None => {
          s.data.remove(&c.key);
        }
      }
    }
    s.exists = true;
    s.clock += 1;
    let keys = Self::manifest_now(&s);
    Ok(PushResponse {
      info: BackupInfo {
        size: 0,
        updated_at: s.clock,
        fingerprint: manifest_fingerprint(&keys),
      },
      keys,
    })
  }

  fn is_stale(error: &CloudError) -> bool {
    *error == CloudError::Stale
  }
}

// ───────────────────────── 假设备 ─────────────────────────

#[derive(Default)]
struct DeviceState {
  data: BTreeMap<String, String>,
  /// 下一次 `apply` 之前，先让用户改这些 key（模拟读快照之后用户动了数据）。
  user_edit: Option<BTreeMap<String, String>>,
}

#[derive(Clone, Default)]
struct FakeDevice(Rc<RefCell<DeviceState>>);

impl FakeDevice {
  fn with(pairs: &[(&str, &str)]) -> Self {
    let d = Self::default();
    d.set(pairs);
    d
  }

  fn set(&self, pairs: &[(&str, &str)]) {
    let mut s = self.0.borrow_mut();
    for (k, v) in pairs {
      s.data.insert((*k).to_owned(), (*v).to_owned());
    }
  }

  fn delete(&self, key: &str) {
    self.0.borrow_mut().data.remove(key);
  }

  fn data(&self) -> BTreeMap<String, String> {
    self.0.borrow().data.clone()
  }

  fn get(&self, key: &str) -> Option<String> {
    self.0.borrow().data.get(key).cloned()
  }
}

impl Device for FakeDevice {
  type Error = ();

  async fn snapshot(&mut self) -> Result<BTreeMap<String, String>, ()> {
    Ok(self.0.borrow().data.clone())
  }

  async fn apply(&mut self, edits: &Edits, expect: &Manifest) -> Result<Applied, ()> {
    let mut s = self.0.borrow_mut();
    if let Some(edit) = s.user_edit.take() {
      s.data.extend(edit);
    }
    for key in edits.keys() {
      if s.data.get(key).map(|v| Digest::of(v)) != expect.get(key).copied() {
        return Ok(Applied::LocalChanged);
      }
    }
    for (k, v) in &edits.set {
      s.data.insert(k.clone(), v.clone());
    }
    for (k, remote) in &edits.merge {
      // 复用生产里的合并规则；本机没有就采用云端的。
      let merged = crate::backup_merge::merge_value(k, s.data.get(k).map(String::as_str), remote);
      if let Some(m) = merged {
        s.data.insert(k.clone(), m);
      }
    }
    for k in &edits.remove {
      s.data.remove(k);
    }
    Ok(Applied::all())
  }
}

// ───────────────────────── 辅助 ─────────────────────────

const STARTUP: Options = Options {
  mode: Mode::Startup,
  include_images: true,
  prefer_remote: false,
};
const PUSH_ONLY: Options = Options {
  mode: Mode::PushOnly,
  include_images: true,
  prefer_remote: false,
};

fn sync(cloud: &FakeCloud, device: &FakeDevice, base: &mut Base, opts: Options) -> Outcome {
  block_on(run(&mut cloud.clone(), &mut device.clone(), base, opts)).expect("同步不应失败")
}

fn startup(cloud: &FakeCloud, device: &FakeDevice, base: &mut Base) -> Outcome {
  sync(cloud, device, base, STARTUP)
}

/// 只看同步内容（去掉设备专属的 key 与索引）。
fn synced(map: &BTreeMap<String, String>) -> BTreeMap<String, String> {
  map
    .iter()
    .filter(|(k, _)| in_scope(k, true))
    .map(|(k, v)| (k.clone(), v.clone()))
    .collect()
}

// ───────────────────────── 单台设备 ─────────────────────────

#[test]
fn the_first_sync_uploads_everything_and_an_idle_second_sync_does_nothing() {
  let cloud = FakeCloud::default();
  let dev = FakeDevice::with(&[
    ("bookmarks", r#"["A-1"]"#),
    ("practice:x", "p"),
    ("theme", "dark"),
  ]);
  let mut base = Base::new();
  let out = startup(&cloud, &dev, &mut base);
  assert_eq!((out.pushed, out.pulled, out.problem), (2, 0, None));
  assert_eq!(
    cloud.data().keys().collect::<Vec<_>>(),
    ["bookmarks", "practice:x"],
    "设备专属的 key 不上云"
  );
  assert!(out.fingerprint.is_some());

  // 什么都没变：不再有任何请求写云端。
  let before = cloud.pushes().len();
  let out = startup(&cloud, &dev, &mut base);
  assert_eq!(
    (out.pushed, out.pulled, out.merged, out.removed_local),
    (0, 0, 0, 0)
  );
  assert_eq!(cloud.pushes().len(), before);
}

#[test]
fn only_the_keys_that_changed_are_uploaded() {
  let cloud = FakeCloud::default();
  let dev = FakeDevice::with(&[
    ("bookmarks", "[]"),
    ("qsl-image:1", &"x".repeat(1000)),
    ("learning-plan", "p0"),
  ]);
  let mut base = Base::new();
  startup(&cloud, &dev, &mut base);
  dev.set(&[("learning-plan", "p1")]);
  let out = sync(&cloud, &dev, &mut base, PUSH_ONLY);
  assert_eq!(out.pushed, 1);
  // 最后一次上传只含改过的那个 key：旧协议会把整份（含大图）再传一遍。
  assert_eq!(cloud.pushes().last().unwrap(), &["learning-plan"]);
  assert_eq!(cloud.data()["learning-plan"], "p1");
}

#[test]
fn a_fresh_device_pulls_everything_from_the_cloud() {
  let cloud = FakeCloud::default();
  cloud.put(&[
    ("bookmarks", r#"["A-1"]"#),
    ("study-stats", "s"),
    ("qsl-image:2", "img"),
  ]);
  let dev = FakeDevice::default();
  let mut base = Base::new();
  let out = startup(&cloud, &dev, &mut base);
  assert_eq!((out.pulled, out.pushed, out.problem), (3, 0, None));
  assert_eq!(synced(&dev.data()), cloud.data());
}

#[test]
fn a_fresh_device_with_only_app_defaults_takes_the_cloud_side() {
  let cloud = FakeCloud::default();
  cloud.put(&[("study-stats", "real"), ("bookmarks", r#"["A-1"]"#)]);
  // 应用自己写的默认值：不是用户的改动，不该因此去问用户。
  let dev = FakeDevice::with(&[("study-stats", "default"), ("only-local", "x")]);
  let mut base = Base::new();
  let opts = Options {
    prefer_remote: true,
    ..STARTUP
  };
  let out = sync(&cloud, &dev, &mut base, opts);
  assert_eq!(out.problem, None);
  assert_eq!(dev.get("study-stats").as_deref(), Some("real"));
  assert_eq!(dev.get("only-local").as_deref(), Some("x"));
  assert_eq!(cloud.data()["only-local"], "x", "本机独有的上传，不丢");
}

#[test]
fn the_cloud_being_emptied_does_not_wipe_the_device_and_the_device_refills_it() {
  let cloud = FakeCloud::default();
  // 进度类的 key：删除本来是会传播的。如果引擎把「云端空了」当成「云端把它们都删了」，
  // 本机的这些 key 就会被删光——这条测试专门挑传播删除的 key，才分得出两种理解。
  let dev = FakeDevice::with(&[
    ("practice:a", "1"),
    ("practice:b", "2"),
    ("learning-plan", "p"),
  ]);
  let mut base = Base::new();
  startup(&cloud, &dev, &mut base);
  assert_eq!(base.len(), 3, "同步成功后三个 key 都有基线");

  // 云端被清空了（账号重建、运维误操作），本机没动。
  *cloud.0.borrow_mut() = CloudState::default();
  let before = dev.data();
  let out = startup(&cloud, &dev, &mut base);
  assert_eq!(out.problem, None);
  assert_eq!(dev.data(), before, "本机的东西一个都没丢");
  assert_eq!(out.removed_local, 0);
  assert_eq!(cloud.data().len(), 3, "本机的东西原样传回了云端");
}

// ───────────────────────── 两台设备 ─────────────────────────

#[test]
fn two_devices_editing_different_keys_converge_without_a_conflict() {
  let cloud = FakeCloud::default();
  let a = FakeDevice::with(&[
    ("bookmarks", "[]"),
    ("practice:x", "p0"),
    ("learning-plan", "l0"),
  ]);
  let b = FakeDevice::default();
  let (mut base_a, mut base_b) = (Base::new(), Base::new());
  startup(&cloud, &a, &mut base_a);
  startup(&cloud, &b, &mut base_b);
  assert_eq!(synced(&b.data()), synced(&a.data()));

  // 各改各的，都还没同步。
  a.set(&[("bookmarks", r#"["A-1"]"#)]);
  b.set(&[("practice:x", "p1")]);
  assert_eq!(sync(&cloud, &a, &mut base_a, PUSH_ONLY).problem, None);
  let out = startup(&cloud, &b, &mut base_b);
  assert_eq!(out.problem, None, "改的是不同的 key：不是冲突");
  assert_eq!((out.pulled, out.pushed), (1, 1));
  let out = startup(&cloud, &a, &mut base_a);
  assert_eq!((out.pulled, out.problem), (1, None));
  assert_eq!(synced(&a.data()), synced(&b.data()));
  assert_eq!(a.get("bookmarks").as_deref(), Some(r#"["A-1"]"#));
  assert_eq!(a.get("practice:x").as_deref(), Some("p1"));
}

#[test]
fn both_editing_a_counter_key_is_a_conflict_that_changes_nothing() {
  let cloud = FakeCloud::default();
  let a = FakeDevice::with(&[("study-stats", "s0"), ("bookmarks", "[]")]);
  let b = FakeDevice::default();
  let (mut base_a, mut base_b) = (Base::new(), Base::new());
  startup(&cloud, &a, &mut base_a);
  startup(&cloud, &b, &mut base_b);
  a.set(&[("study-stats", "from-a"), ("bookmarks", r#"["A-1"]"#)]);
  b.set(&[("study-stats", "from-b")]);
  startup(&cloud, &a, &mut base_a);

  let (cloud_before, b_before) = (cloud.data(), b.data());
  let out = startup(&cloud, &b, &mut base_b);
  assert_eq!(
    out.problem,
    Some(Problem::Conflict(vec!["study-stats".to_owned()]))
  );
  // 冲突时一个字节都不动：不拉、不推、不留半截状态。
  assert_eq!((cloud.data(), b.data()), (cloud_before, b_before));
  assert_eq!((out.pulled, out.pushed), (0, 0));
}

#[test]
fn both_editing_a_mergeable_key_merges_and_both_sides_end_up_with_the_union() {
  let cloud = FakeCloud::default();
  let a = FakeDevice::with(&[("bookmarks", r#"["A-1"]"#)]);
  let b = FakeDevice::default();
  let (mut base_a, mut base_b) = (Base::new(), Base::new());
  startup(&cloud, &a, &mut base_a);
  startup(&cloud, &b, &mut base_b);
  a.set(&[("bookmarks", r#"["A-1","A-2"]"#)]);
  b.set(&[("bookmarks", r#"["A-1","B-3"]"#)]);
  startup(&cloud, &a, &mut base_a);

  let out = startup(&cloud, &b, &mut base_b);
  assert_eq!((out.merged, out.problem), (1, None));
  let merged = b.get("bookmarks").unwrap();
  for id in ["A-1", "A-2", "B-3"] {
    assert!(merged.contains(id), "{id} 丢了：{merged}");
  }
  assert_eq!(cloud.data()["bookmarks"], merged, "合并结果传上了云端");
  startup(&cloud, &a, &mut base_a);
  assert_eq!(a.get("bookmarks"), b.get("bookmarks"));
}

#[test]
fn both_editing_a_setting_keeps_the_device_that_syncs_last() {
  let cloud = FakeCloud::default();
  let a = FakeDevice::with(&[("learning-plan", "l0")]);
  let b = FakeDevice::default();
  let (mut base_a, mut base_b) = (Base::new(), Base::new());
  startup(&cloud, &a, &mut base_a);
  startup(&cloud, &b, &mut base_b);
  a.set(&[("learning-plan", "from-a")]);
  b.set(&[("learning-plan", "from-b")]);
  startup(&cloud, &a, &mut base_a);
  let out = startup(&cloud, &b, &mut base_b);
  assert_eq!(out.problem, None, "设置不打扰用户");
  assert_eq!(cloud.data()["learning-plan"], "from-b");
  startup(&cloud, &a, &mut base_a);
  assert_eq!(a.get("learning-plan").as_deref(), Some("from-b"));
}

#[test]
fn a_deleted_progress_key_propagates_but_a_lost_logbook_is_restored() {
  let cloud = FakeCloud::default();
  let a = FakeDevice::with(&[("practice:x", "p"), ("logbook", "L"), ("mistake-book", "M")]);
  let b = FakeDevice::default();
  let (mut base_a, mut base_b) = (Base::new(), Base::new());
  startup(&cloud, &a, &mut base_a);
  startup(&cloud, &b, &mut base_b);

  // A 用完了练习进度（删），B 的浏览器清了站点数据（logbook 与 mistake-book 丢了）。
  a.delete("practice:x");
  b.delete("logbook");
  b.delete("mistake-book");
  startup(&cloud, &a, &mut base_a);
  assert!(
    !cloud.data().contains_key("practice:x"),
    "进度的删除传给了云端"
  );

  let out = startup(&cloud, &b, &mut base_b);
  assert_eq!(out.problem, None);
  assert!(b.get("practice:x").is_none(), "进度的删除传到了 B");
  assert_eq!(
    b.get("logbook").as_deref(),
    Some("L"),
    "丢了的日志从云端补回来"
  );
  assert_eq!(b.get("mistake-book").as_deref(), Some("M"));
  assert_eq!(
    cloud.data()["logbook"],
    "L",
    "没有因为 B 丢了就把云端的也删了"
  );
}

#[test]
fn an_unexpected_mass_deletion_stops_and_asks_instead_of_wiping_the_cloud() {
  let cloud = FakeCloud::default();
  let pairs: Vec<(String, String)> = (0..10)
    .map(|i| (format!("practice:k{i}"), "p".into()))
    .collect();
  let refs: Vec<(&str, &str)> = pairs
    .iter()
    .map(|(k, v)| (k.as_str(), v.as_str()))
    .collect();
  let a = FakeDevice::with(&refs);
  let mut base = Base::new();
  startup(&cloud, &a, &mut base);
  // 浏览器清了站点数据：本机一下子少了十个 key 中的八个。
  for (k, _) in pairs.iter().take(8) {
    a.delete(k);
  }
  let out = startup(&cloud, &a, &mut base);
  assert!(
    matches!(out.problem, Some(Problem::SuspiciousDeletions(ref v)) if v.len() == 8),
    "{:?}",
    out.problem
  );
  assert_eq!(cloud.data().len(), 10, "云端一个都没删");
  // 少量删除（用户真的删了一两个）照常传播。
  let a = FakeDevice::with(&refs);
  let cloud = FakeCloud::default();
  let mut base = Base::new();
  startup(&cloud, &a, &mut base);
  a.delete("practice:k0");
  assert_eq!(startup(&cloud, &a, &mut base).problem, None);
  assert_eq!(cloud.data().len(), 9);
}

#[test]
fn the_suspicion_threshold_needs_both_a_minimum_count_and_a_majority() {
  assert!(!suspicious(4, 4), "太少不算");
  assert!(!suspicious(5, 20), "没过半不算");
  assert!(suspicious(5, 9));
  assert!(suspicious(8, 10));
  assert!(!suspicious(0, 0));
}

// ───────────────────────── 并发与中途被改 ─────────────────────────

#[test]
fn losing_a_race_on_push_retries_the_whole_round_and_still_converges() {
  let cloud = FakeCloud::default();
  let a = FakeDevice::with(&[("practice:x", "p0"), ("learning-plan", "l0")]);
  let mut base = Base::new();
  startup(&cloud, &a, &mut base);
  a.set(&[("practice:x", "mine")]);
  // 在 A 上传之前，另一台设备抢先把同一个 key 改了。
  cloud.0.borrow_mut().race = Some([("practice:x".to_owned(), "theirs".to_owned())].into());
  let out = startup(&cloud, &a, &mut base);
  assert_eq!(out.problem, None);
  // 进度是「后写为准」：重来之后 A 的覆盖了对方的，而且云端与本机一致。
  assert_eq!(cloud.data()["practice:x"], "mine");
  assert_eq!(a.get("practice:x").as_deref(), Some("mine"));
}

#[test]
fn local_writes_made_before_a_lost_race_are_still_reported() {
  let cloud = FakeCloud::default();
  let a = FakeDevice::with(&[("practice:x", "theirs")]);
  let b = FakeDevice::default();
  let (mut base_a, mut base_b) = (Base::new(), Base::new());
  startup(&cloud, &a, &mut base_a);

  // B 要取回 practice:x，同时本机有 learning-plan 要上传。
  b.set(&[("learning-plan", "mine")]);
  // 取回落盘之后、上传之前，另一台设备抢先改了 learning-plan：上传被 412，整轮重来。
  cloud.0.borrow_mut().race = Some([("learning-plan".to_owned(), "other".to_owned())].into());
  let out = startup(&cloud, &b, &mut base_b);

  assert_eq!(out.problem, None);
  assert_eq!(b.get("practice:x").as_deref(), Some("theirs"));
  // 重来的那一轮里 practice:x 两边已经一样、没有再取回；但本机确实被第一轮改写过，
  // 界面据此提示「已更新」，不能因为重来就把它丢了。
  assert_eq!(out.pulled, 1);
  assert!(out.changed_local());
  // 进度类是后写为准：B 的改动最终生效。
  assert_eq!(cloud.data()["learning-plan"], "mine");
}

#[test]
fn a_cloud_that_keeps_changing_still_reports_what_was_written_locally() {
  // 每一轮都在上传前被抢先改掉：最终放弃（`Busy`），但第一轮取回的内容已经落在本机上，计数要如实给出。
  struct Racing {
    inner: FakeCloud,
    race: usize,
  }
  impl Cloud for Racing {
    type Error = CloudError;
    async fn manifest(&mut self) -> Result<Option<SyncManifest>, CloudError> {
      self.inner.manifest().await
    }
    async fn pull(&mut self, keys: &[String]) -> Result<BTreeMap<String, String>, CloudError> {
      self.inner.pull(keys).await
    }
    async fn push(&mut self, _: Vec<Change>) -> Result<PushResponse, CloudError> {
      self.race += 1;
      Err(CloudError::Stale)
    }
    fn is_stale(e: &CloudError) -> bool {
      FakeCloud::is_stale(e)
    }
  }
  let cloud = FakeCloud::default();
  cloud.put(&[("practice:x", "theirs")]);
  let b = FakeDevice::with(&[("learning-plan", "mine")]);
  let mut racing = Racing {
    inner: cloud,
    race: 0,
  };
  let out = block_on(run(&mut racing, &mut b.clone(), &mut Base::new(), STARTUP)).unwrap();
  assert_eq!(out.problem, Some(Problem::Busy));
  assert_eq!(racing.race, MAX_ATTEMPTS, "每一轮都撞上了");
  assert_eq!(b.get("practice:x").as_deref(), Some("theirs"));
  assert_eq!(out.pulled, 1, "只取回了一次，不重复计数");
}

#[test]
fn the_user_editing_during_a_pull_makes_the_round_start_over_without_losing_the_edit() {
  let cloud = FakeCloud::default();
  let a = FakeDevice::with(&[("practice:x", "p0"), ("bookmarks", "[]")]);
  let b = FakeDevice::default();
  let (mut base_a, mut base_b) = (Base::new(), Base::new());
  startup(&cloud, &a, &mut base_a);
  startup(&cloud, &b, &mut base_b);
  a.set(&[("practice:x", "p1")]);
  startup(&cloud, &a, &mut base_a);

  // B 要取回 practice:x，但读完快照之后用户又改了它。
  b.0.borrow_mut().user_edit =
    Some([("practice:x".to_owned(), "typed-meanwhile".to_owned())].into());
  let out = startup(&cloud, &b, &mut base_b);
  assert_eq!(out.problem, None);
  // 用户的编辑没有被悄悄吞掉：它和云端的 p1 是两边都改了，进度是后写为准，用户的赢。
  assert_eq!(b.get("practice:x").as_deref(), Some("typed-meanwhile"));
  assert_eq!(cloud.data()["practice:x"], "typed-meanwhile");
}

/// 设备拒绝落盘（返回 `LocalChanged`）时，引擎必须当作「这一轮作废」而不是当作成功：
/// 不能记基线、不能报告「已取回」，也不能带着没写进去的东西继续往下传。
/// 这条测试守的是引擎对 [`Device::apply`] 返回值的响应——设备那一半由 `FakeDevice` 和真实设备各自保证。
#[test]
fn a_device_refusing_to_apply_makes_the_engine_start_over_and_never_claim_success() {
  struct Refuses {
    inner: FakeDevice,
    refusals_left: usize,
    applies: usize,
  }
  impl Device for Refuses {
    type Error = ();
    async fn snapshot(&mut self) -> Result<BTreeMap<String, String>, ()> {
      self.inner.snapshot().await
    }
    async fn apply(&mut self, edits: &Edits, expect: &Manifest) -> Result<Applied, ()> {
      self.applies += 1;
      if self.refusals_left > 0 {
        self.refusals_left -= 1;
        return Ok(Applied::LocalChanged);
      }
      self.inner.apply(edits, expect).await
    }
  }
  let cloud = FakeCloud::default();
  cloud.put(&[("practice:x", "p1"), ("bookmarks", r#"["A-1"]"#)]);

  // 拒绝一次：重来之后落盘成功，结果正确，且只数一次「取回」。
  let mut dev = Refuses {
    inner: FakeDevice::default(),
    refusals_left: 1,
    applies: 0,
  };
  let mut base = Base::new();
  let out = block_on(run(&mut cloud.clone(), &mut dev, &mut base, STARTUP)).unwrap();
  assert_eq!((dev.applies, out.pulled, out.problem), (2, 2, None));
  assert_eq!(synced(&dev.inner.data()), cloud.data());

  // 一直拒绝：重试几次就放弃（Busy），而且没有记下任何「已经取回」的基线——
  // 否则下一轮会把「本机没有」当成「用户删了」传播出去。
  let mut dev = Refuses {
    inner: FakeDevice::default(),
    refusals_left: usize::MAX,
    applies: 0,
  };
  let mut base = Base::new();
  let out = block_on(run(&mut cloud.clone(), &mut dev, &mut base, STARTUP)).unwrap();
  assert_eq!(out.problem, Some(Problem::Busy));
  assert_eq!(dev.applies, MAX_ATTEMPTS);
  assert!(base.is_empty(), "设备没写进去，就不能记基线");
  assert_eq!(cloud.data().len(), 2, "也没有往云端传任何东西");
  assert_eq!((out.pulled, out.merged, out.pushed), (0, 0, 0));
}

/// 全新设备上只有应用自己写的默认值，两边都有而不同的 key 以云端为准。
/// 但用户在取回期间动了数据之后，这台设备就不再是「全新」的：这时不能再无条件以云端为准。
#[test]
fn a_device_the_user_touched_mid_round_stops_being_treated_as_fresh() {
  let cloud = FakeCloud::default();
  cloud.put(&[("study-stats", "real"), ("bookmarks", r#"["A-1"]"#)]);
  let dev = FakeDevice::with(&[("study-stats", "default")]);
  // 读完快照之后、落盘之前，用户改了 study-stats。
  dev.0.borrow_mut().user_edit =
    Some([("study-stats".to_owned(), "typed-by-user".to_owned())].into());
  let mut base = Base::new();
  let opts = Options {
    prefer_remote: true,
    ..STARTUP
  };
  let out = sync(&cloud, &dev, &mut base, opts);
  // 重来之后 study-stats 是「两边都有且不同」，没有安全规则：停下来问，而不是用云端的盖掉用户刚输入的。
  assert_eq!(
    out.problem,
    Some(Problem::Conflict(vec!["study-stats".to_owned()]))
  );
  assert_eq!(dev.get("study-stats").as_deref(), Some("typed-by-user"));
  assert_eq!(cloud.data()["study-stats"], "real", "云端也没被动");
}

/// 设备说某个 key 没写成：引擎不能给它记基线，否则下一轮会把「本机没有」当成「用户删了它」。
#[test]
fn a_key_the_device_could_not_write_gets_no_baseline_and_is_never_uploaded_as_a_deletion() {
  struct SkipsOne(FakeDevice);
  impl Device for SkipsOne {
    type Error = ();
    async fn snapshot(&mut self) -> Result<BTreeMap<String, String>, ()> {
      self.0.snapshot().await
    }
    async fn apply(&mut self, edits: &Edits, expect: &Manifest) -> Result<Applied, ()> {
      // 把 qsl-image:2 当成不合格的内容跳过：其余照常写。
      let mut rest = edits.clone();
      rest.set.remove("qsl-image:2");
      self.0.apply(&rest, expect).await?;
      Ok(Applied::Done {
        skipped: vec!["qsl-image:2".to_owned()],
      })
    }
  }
  let cloud = FakeCloud::default();
  cloud.put(&[("qsl-image:1", "ok"), ("qsl-image:2", "weird")]);
  let mut dev = SkipsOne(FakeDevice::default());
  let mut base = Base::new();
  let out = block_on(run(&mut cloud.clone(), &mut dev, &mut base, STARTUP)).unwrap();
  assert_eq!((out.pulled, out.problem), (1, None), "只数写成了的");
  assert!(base.contains_key("qsl-image:1"));
  assert!(!base.contains_key("qsl-image:2"), "没写成的不记基线");
  // 再跑几轮：不会把「本机没有 qsl-image:2」当成删除传给云端。
  for _ in 0..3 {
    block_on(run(&mut cloud.clone(), &mut dev, &mut base, STARTUP)).unwrap();
    assert!(
      cloud.data().contains_key("qsl-image:2"),
      "云端的影像被当成「用户删了」删掉了"
    );
  }
}

/// 记下每一次 `checkpoint` 时基线的样子，其余委托给 `FakeDevice`。
struct Recording {
  inner: FakeDevice,
  checkpoints: Rc<RefCell<Vec<Base>>>,
}

impl Device for Recording {
  type Error = ();
  async fn snapshot(&mut self) -> Result<BTreeMap<String, String>, ()> {
    self.inner.snapshot().await
  }
  async fn apply(&mut self, edits: &Edits, expect: &Manifest) -> Result<Applied, ()> {
    self.inner.apply(edits, expect).await
  }
  fn checkpoint(&mut self, base: &Base) {
    self.checkpoints.borrow_mut().push(base.clone());
  }
}

#[test]
fn the_baseline_is_saved_after_every_upload_batch_not_only_when_the_round_ends() {
  // 上传分成好几批时，每批生效之后基线就该存下来：第二批之前被打断（关页、断网），
  // 第一批已经写进云端的 key 不能因为没有基线，下次启动被当成「两边都改了」。
  let cloud = FakeCloud::default();
  let n = PUSH_BATCH_KEYS * 2 + 5;
  let owned: Vec<(String, String)> = (0..n)
    .map(|i| (format!("practice:k{i:04}"), format!("v{i}")))
    .collect();
  let pairs: Vec<(&str, &str)> = owned
    .iter()
    .map(|(k, v)| (k.as_str(), v.as_str()))
    .collect();
  let checkpoints = Rc::new(RefCell::new(Vec::new()));
  let mut dev = Recording {
    inner: FakeDevice::with(&pairs),
    checkpoints: checkpoints.clone(),
  };
  let mut base = Base::new();
  let out = block_on(run(&mut cloud.clone(), &mut dev, &mut base, STARTUP)).unwrap();
  assert_eq!((out.pushed, out.problem), (n, None));

  let saved = checkpoints.borrow();
  assert_eq!(saved.len(), 3, "三批各存一次");
  // 基线随批次单调增长：第一批之后只有第一批的 key，最后一次是全部。
  assert_eq!(saved[0].len(), PUSH_BATCH_KEYS);
  assert_eq!(saved[1].len(), PUSH_BATCH_KEYS * 2);
  assert_eq!(saved[2].len(), n);
}

#[test]
fn the_baseline_is_saved_right_after_the_device_is_written() {
  // 本机已经被改写、基线也推进了，但接下来的上传失败（断网）：落盘那一刻的基线已经交出去了。
  struct Offline(FakeCloud);
  impl Cloud for Offline {
    type Error = CloudError;
    async fn manifest(&mut self) -> Result<Option<SyncManifest>, CloudError> {
      self.0.manifest().await
    }
    async fn pull(&mut self, keys: &[String]) -> Result<BTreeMap<String, String>, CloudError> {
      self.0.pull(keys).await
    }
    async fn push(&mut self, _: Vec<Change>) -> Result<PushResponse, CloudError> {
      Err(CloudError::Down)
    }
    fn is_stale(e: &CloudError) -> bool {
      FakeCloud::is_stale(e)
    }
  }
  let cloud = FakeCloud::default();
  cloud.put(&[("practice:a", "from-cloud")]);
  let checkpoints = Rc::new(RefCell::new(Vec::new()));
  let mut dev = Recording {
    inner: FakeDevice::with(&[("practice:b", "mine")]),
    checkpoints: checkpoints.clone(),
  };
  let mut base = Base::new();
  let r = block_on(run(&mut Offline(cloud), &mut dev, &mut base, STARTUP));
  assert!(matches!(r, Err(Error::Cloud(CloudError::Down))));
  let saved = checkpoints.borrow();
  assert_eq!(
    saved.len(),
    1,
    "取回落盘之后存了一次，上传没成功所以没有第二次"
  );
  assert!(
    saved[0].contains_key("practice:a"),
    "取回来的 key 的基线已经存下"
  );
  assert!(!saved[0].contains_key("practice:b"), "没传出去的不记基线");
}

#[test]
fn skipped_keys_are_counted_once_across_retries_and_deferred_adds_up() {
  // 设备没写成的 key 要让调用方知道（界面据此提示一次），而不是每次启动悄悄重试。
  struct SkipsOne(FakeDevice);
  impl Device for SkipsOne {
    type Error = ();
    async fn snapshot(&mut self) -> Result<BTreeMap<String, String>, ()> {
      self.0.snapshot().await
    }
    async fn apply(&mut self, edits: &Edits, expect: &Manifest) -> Result<Applied, ()> {
      let mut rest = edits.clone();
      rest.set.remove("qsl-image:2");
      self.0.apply(&rest, expect).await?;
      Ok(Applied::Done {
        skipped: vec!["qsl-image:2".to_owned()],
      })
    }
  }
  let cloud = FakeCloud::default();
  cloud.put(&[("qsl-image:1", "ok"), ("qsl-image:2", "weird")]);
  let mut dev = SkipsOne(FakeDevice::default());
  let out = block_on(run(&mut cloud.clone(), &mut dev, &mut Base::new(), STARTUP)).unwrap();
  assert_eq!((out.pulled, out.skipped), (1, 1));
}

#[test]
fn push_only_adds_held_images_to_the_deferred_pulls_instead_of_overwriting_them() {
  // 日志要取回（自动写入里不做、算 deferred），同时本机有一张影像因此被 hold：两者都该计入。
  let cloud = FakeCloud::default();
  let a = FakeDevice::with(&[("logbook", "L1")]);
  let b = FakeDevice::default();
  let (mut base_a, mut base_b) = (Base::new(), Base::new());
  startup(&cloud, &a, &mut base_a);
  startup(&cloud, &b, &mut base_b);
  a.set(&[("logbook", "L2"), ("practice:x", "p")]);
  startup(&cloud, &a, &mut base_a);
  // B 新增了一张影像（日志那边云端有更新，所以这张影像先别传）。
  b.set(&[("qsl-image:9", "mine")]);
  let out = sync(&cloud, &b, &mut base_b, PUSH_ONLY);
  // 要取回的：logbook、practice:x（2 个）；被 hold 的影像：1 个。
  assert_eq!(out.deferred, 3, "{out:?}");
}

#[test]
fn a_key_that_vanishes_between_the_manifest_and_the_pull_restarts_the_round() {
  let cloud = FakeCloud::default();
  let a = FakeDevice::with(&[("bookmarks", r#"["A-1"]"#), ("practice:x", "p")]);
  let b = FakeDevice::default();
  let (mut base_a, mut base_b) = (Base::new(), Base::new());
  startup(&cloud, &a, &mut base_a);
  cloud.0.borrow_mut().vanish = vec!["practice:x".to_owned()];
  let out = startup(&cloud, &b, &mut base_b);
  assert_eq!(out.problem, None);
  assert!(
    b.get("practice:x").is_none(),
    "清单里有、取值时没了：重来之后按「没有」处理"
  );
  assert_eq!(b.get("bookmarks").as_deref(), Some(r#"["A-1"]"#));
}

#[test]
fn a_cloud_that_keeps_changing_gives_up_after_a_few_rounds_instead_of_spinning() {
  struct Hostile(FakeCloud);
  impl Cloud for Hostile {
    type Error = CloudError;
    async fn manifest(&mut self) -> Result<Option<SyncManifest>, CloudError> {
      self.0.manifest().await
    }
    async fn pull(&mut self, keys: &[String]) -> Result<BTreeMap<String, String>, CloudError> {
      self.0.pull(keys).await
    }
    async fn push(&mut self, _: Vec<Change>) -> Result<PushResponse, CloudError> {
      Err(CloudError::Stale)
    }
    fn is_stale(e: &CloudError) -> bool {
      FakeCloud::is_stale(e)
    }
  }
  let dev = FakeDevice::with(&[("bookmarks", "[]")]);
  let out = block_on(run(
    &mut Hostile(FakeCloud::default()),
    &mut dev.clone(),
    &mut Base::new(),
    PUSH_ONLY,
  ))
  .unwrap();
  assert_eq!(out.problem, Some(Problem::Busy));
}

#[test]
fn a_cloud_outage_is_an_error_not_a_silent_success() {
  struct Down;
  impl Cloud for Down {
    type Error = CloudError;
    async fn manifest(&mut self) -> Result<Option<SyncManifest>, CloudError> {
      Err(CloudError::Down)
    }
    async fn pull(&mut self, _: &[String]) -> Result<BTreeMap<String, String>, CloudError> {
      Err(CloudError::Down)
    }
    async fn push(&mut self, _: Vec<Change>) -> Result<PushResponse, CloudError> {
      Err(CloudError::Down)
    }
    fn is_stale(e: &CloudError) -> bool {
      *e == CloudError::Stale
    }
  }
  let dev = FakeDevice::with(&[("bookmarks", "[]")]);
  let mut base = Base::new();
  let r = block_on(run(&mut Down, &mut dev.clone(), &mut base, STARTUP));
  assert!(matches!(r, Err(Error::Cloud(CloudError::Down))));
  assert!(base.is_empty(), "失败时不记基线");
}

// ───────────────────────── 只上传模式 ─────────────────────────

#[test]
fn push_only_never_writes_the_device_and_defers_what_it_would_have_pulled() {
  let cloud = FakeCloud::default();
  let a = FakeDevice::with(&[("practice:x", "p0"), ("bookmarks", "[]")]);
  let b = FakeDevice::default();
  let (mut base_a, mut base_b) = (Base::new(), Base::new());
  startup(&cloud, &a, &mut base_a);
  startup(&cloud, &b, &mut base_b);
  a.set(&[("practice:x", "p1")]);
  startup(&cloud, &a, &mut base_a);

  // B 自己改了另一个 key，然后自动写入跑起来：只上传自己的，云端的变化留到下次打开。
  b.set(&[("bookmarks", r#"["B-1"]"#)]);
  let b_before = b.data();
  let out = sync(&cloud, &b, &mut base_b, PUSH_ONLY);
  assert_eq!(
    (out.pushed, out.pulled, out.deferred, out.problem),
    (1, 0, 1, None)
  );
  assert_eq!(b.data(), b_before, "自动写入不碰本机数据");
  assert_eq!(cloud.data()["bookmarks"], r#"["B-1"]"#);
  // 下次打开：补上推迟的那一个。
  let out = startup(&cloud, &b, &mut base_b);
  assert_eq!((out.pulled, out.problem), (1, None));
  assert_eq!(b.get("practice:x").as_deref(), Some("p1"));
}

#[test]
fn push_only_reports_a_mergeable_clash_instead_of_merging_behind_the_pages_back() {
  let cloud = FakeCloud::default();
  let a = FakeDevice::with(&[("bookmarks", r#"["A-1"]"#)]);
  let b = FakeDevice::default();
  let (mut base_a, mut base_b) = (Base::new(), Base::new());
  startup(&cloud, &a, &mut base_a);
  startup(&cloud, &b, &mut base_b);
  a.set(&[("bookmarks", r#"["A-1","A-2"]"#)]);
  b.set(&[("bookmarks", r#"["A-1","B-3"]"#)]);
  startup(&cloud, &a, &mut base_a);
  let (b_before, cloud_before) = (b.data(), cloud.data());
  let out = sync(&cloud, &b, &mut base_b, PUSH_ONLY);
  assert_eq!(
    out.problem,
    Some(Problem::Conflict(vec!["bookmarks".to_owned()]))
  );
  assert_eq!((b.data(), cloud.data()), (b_before, cloud_before));
}

// ───────────────────────── 服务端规范化带来的字节差异 ─────────────────────────

/// 一个会把 `study-stats` 的值改写成规范形态的云端（模拟服务端补默认值）。
struct Canonicalizing(FakeCloud);

impl Canonicalizing {
  fn canon(key: &str, value: &str) -> String {
    if key == "study-stats" {
      format!("{{canon:{}}}", value.trim_matches(|c| c == '{' || c == '}'))
    } else {
      value.to_owned()
    }
  }
}

impl Cloud for Canonicalizing {
  type Error = CloudError;
  async fn manifest(&mut self) -> Result<Option<SyncManifest>, CloudError> {
    self.0.manifest().await
  }
  async fn pull(&mut self, keys: &[String]) -> Result<BTreeMap<String, String>, CloudError> {
    self.0.pull(keys).await
  }
  async fn push(&mut self, changes: Vec<Change>) -> Result<PushResponse, CloudError> {
    let changes = changes
      .into_iter()
      .map(|mut c| {
        c.value = c.value.map(|v| Self::canon(&c.key, &v));
        c
      })
      .collect();
    self.0.push(changes).await
  }
  fn is_stale(e: &CloudError) -> bool {
    FakeCloud::is_stale(e)
  }
}

#[test]
fn content_the_server_rewrites_is_uploaded_once_and_then_left_alone() {
  // 老版本写出的内容，服务端补成规范形态：云端摘要 ≠ 本机摘要，但谁也没改。
  // 如果把「摘要不同」当成「有改动」，这个 key 会每一轮都被重传，永远稳定不下来。
  let cloud = FakeCloud::default();
  let dev = FakeDevice::with(&[("study-stats", "{answered:3}")]);
  let mut base = Base::new();
  let mut wrapped = Canonicalizing(cloud.clone());
  let out = block_on(run(&mut wrapped, &mut dev.clone(), &mut base, STARTUP)).unwrap();
  assert_eq!(out.pushed, 1);
  assert_ne!(cloud.data()["study-stats"], dev.get("study-stats").unwrap());
  assert_eq!(
    dev.get("study-stats").as_deref(),
    Some("{answered:3}"),
    "本机的文本不被改写（可能是比服务端新的客户端写的，服务端不认识的字段不能丢）"
  );

  for _ in 0..3 {
    let before = cloud.pushes().len();
    let out = block_on(run(&mut wrapped, &mut dev.clone(), &mut base, STARTUP)).unwrap();
    assert_eq!((out.pushed, out.pulled, out.problem), (0, 0, None));
    assert_eq!(cloud.pushes().len(), before, "稳定之后不再上传");
  }
  // 本机真的改了：传上去，而且不是冲突。
  dev.set(&[("study-stats", "{answered:4}")]);
  let out = block_on(run(&mut wrapped, &mut dev.clone(), &mut base, PUSH_ONLY)).unwrap();
  assert_eq!((out.pushed, out.problem), (1, None));
}

// ───────────────────────── QSL 影像 ─────────────────────────

#[test]
fn images_are_left_out_when_the_account_does_not_sync_them() {
  let cloud = FakeCloud::default();
  let dev = FakeDevice::with(&[
    ("bookmarks", "[]"),
    ("qsl-image:1", "img"),
    ("qsl-image-index", "[1]"),
  ]);
  let mut base = Base::new();
  let opts = Options {
    include_images: false,
    ..STARTUP
  };
  sync(&cloud, &dev, &mut base, opts);
  assert_eq!(cloud.data().keys().collect::<Vec<_>>(), ["bookmarks"]);
  // 云端有影像而本机选择不同步：既不取回，也不当成「云端被别处改了」。
  cloud.put(&[("qsl-image:9", "other")]);
  let out = sync(&cloud, &dev, &mut base, opts);
  assert_eq!((out.pulled, out.pushed, out.problem), (0, 0, None));
  assert!(dev.get("qsl-image:9").is_none());
  // 云端的影像也没有因为本机没有而被删。
  assert!(cloud.data().contains_key("qsl-image:9"));
}

#[test]
fn a_cloud_holding_only_out_of_scope_keys_is_not_empty() {
  // 关掉影像同步时「收窄后的清单」会是空的，但云端并不空：基线不能因此作废。
  let cloud = FakeCloud::default();
  cloud.put(&[("qsl-image:1", "img")]);
  let dev = FakeDevice::default();
  let mut base: Base = [(
    "qsl-image:1".to_owned(),
    Synced {
      remote: Digest::of("img"),
      local: Digest::of("img"),
    },
  )]
  .into();
  let opts = Options {
    include_images: false,
    ..STARTUP
  };
  let out = sync(&cloud, &dev, &mut base, opts);
  assert_eq!(out.problem, None);
  assert!(
    base.contains_key("qsl-image:1"),
    "云端只有范围外的 key，不等于云端是空的"
  );
}

#[test]
fn switching_images_off_keeps_their_baseline() {
  // 关一次影像同步、期间两端各自改了影像、再打开：基线还在，所以「只有云端改了」的影像能取回，
  // 而不是被判成「两边都改了」去问用户。
  let cloud = FakeCloud::default();
  let dev = FakeDevice::with(&[("bookmarks", "[]"), ("qsl-image:1", "v1")]);
  let mut base = Base::new();
  startup(&cloud, &dev, &mut base);
  assert!(base.contains_key("qsl-image:1"));

  let off = Options {
    include_images: false,
    ..STARTUP
  };
  sync(&cloud, &dev, &mut base, off);
  assert!(
    base.contains_key("qsl-image:1"),
    "关掉影像同步不该把影像的基线清掉"
  );

  // 另一台设备换了影像，本机没动：打开开关应该取回，而不是报冲突。
  cloud.put(&[("qsl-image:1", "v2")]);
  let out = sync(&cloud, &dev, &mut base, STARTUP);
  assert_eq!(out.problem, None, "只有云端改了，不该是冲突");
  assert_eq!(dev.get("qsl-image:1").as_deref(), Some("v2"));
}

#[test]
fn an_image_held_by_a_logbook_pull_does_not_block_other_keys() {
  // 日志要取回时本机影像先不传（编号会撞），但其它 key 的同步不能跟着停。
  let cloud = FakeCloud::default();
  cloud.put(&[("logbook", "L2"), ("bookmarks", "[9]")]);
  let dev = FakeDevice::with(&[("logbook", "L1"), ("qsl-image:5", "mine")]);
  let mut base: Base = [(
    "logbook".to_owned(),
    Synced {
      remote: Digest::of("L1"),
      local: Digest::of("L1"),
    },
  )]
  .into();
  let out = startup(&cloud, &dev, &mut base);

  // 其它 key 照常同步。
  assert_eq!(dev.get("bookmarks").as_deref(), Some("[9]"));
  assert_eq!(dev.get("logbook").as_deref(), Some("L2"));
  // 影像没被传上去（编号可能撞），而且被报告出来了。
  assert!(!cloud.data().contains_key("qsl-image:5"));
  assert_eq!(
    out.problem,
    Some(Problem::Conflict(vec!["qsl-image:5".to_owned()]))
  );
  assert_eq!(out.deferred, 1);
}

#[test]
fn a_large_pull_is_split_into_batches() {
  let cloud = FakeCloud::default();
  let pairs: Vec<(String, String)> = (0..(PULL_BATCH_KEYS + 44))
    .map(|i| (format!("note:{i}"), i.to_string()))
    .collect();
  let borrowed: Vec<(&str, &str)> = pairs
    .iter()
    .map(|(k, v)| (k.as_str(), v.as_str()))
    .collect();
  cloud.put(&borrowed);

  let dev = FakeDevice::default();
  let mut base = Base::new();
  let out = startup(&cloud, &dev, &mut base);

  assert_eq!(out.problem, None);
  assert_eq!(cloud.pulls(), 2, "超过一批的 key 要分两次取");
  for (key, value) in &pairs {
    assert_eq!(dev.get(key).as_deref(), Some(value.as_str()));
  }
}

#[test]
fn a_large_push_is_split_into_batches() {
  let dev = FakeDevice::default();
  for i in 0..(PUSH_BATCH_KEYS + 20) {
    let key = format!("note:{i}");
    dev.set(&[(&key, "x")]);
  }
  let cloud = FakeCloud::default();
  let mut base = Base::new();
  let out = startup(&cloud, &dev, &mut base);

  assert_eq!(out.problem, None);
  let pushes = cloud.pushes();
  assert_eq!(pushes.len(), 2, "超过一批的改动要分两次传");
  assert!(pushes.iter().all(|batch| batch.len() <= PUSH_BATCH_KEYS));
  assert_eq!(
    pushes.iter().map(Vec::len).sum::<usize>(),
    PUSH_BATCH_KEYS + 20
  );
  assert_eq!(out.pushed, PUSH_BATCH_KEYS + 20);
}

#[test]
fn the_derived_image_index_is_never_synced() {
  let cloud = FakeCloud::default();
  let a = FakeDevice::with(&[("qsl-image:1", "i"), ("qsl-image-index", "[1]")]);
  let b = FakeDevice::default();
  let (mut base_a, mut base_b) = (Base::new(), Base::new());
  startup(&cloud, &a, &mut base_a);
  assert!(!cloud.data().contains_key("qsl-image-index"));
  startup(&cloud, &b, &mut base_b);
  assert_eq!(b.get("qsl-image:1").as_deref(), Some("i"));
}

// ───────────────────────── 性质：任意操作序列后收敛 ─────────────────────────

/// 线性同余伪随机，测试可复现。
struct Lcg(u64);
impl Lcg {
  fn next(&mut self, n: usize) -> usize {
    self.0 = self
      .0
      .wrapping_mul(6_364_136_223_846_793_005)
      .wrapping_add(1_442_695_040_888_963_407);
    ((self.0 >> 33) as usize) % n
  }
}

#[test]
fn many_devices_editing_at_random_always_converge_and_never_lose_an_accumulated_key() {
  // 三台设备，随机地改 / 删 / 同步，最后每台都同步到稳定：三方一致，且没有凭空消失的积累数据。
  const KEYS: [&str; 6] = [
    "bookmarks",
    "learning-plan",
    "practice:a",
    "practice:b",
    "study-stats",
    "mistake-book",
  ];
  for seed in 0..40u64 {
    let mut rng = Lcg(seed + 1);
    let cloud = FakeCloud::default();
    let devs = [
      FakeDevice::default(),
      FakeDevice::default(),
      FakeDevice::default(),
    ];
    let mut bases = [Base::new(), Base::new(), Base::new()];
    let mut ever_written: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    let mut ever_deleted: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for step in 0..30 {
      let d = rng.next(3);
      let key = KEYS[rng.next(KEYS.len())];
      match rng.next(5) {
        0 | 1 => {
          let value = match key {
            "bookmarks" => format!("[\"{}\"]", rng.next(4)),
            "mistake-book" => r#"{"records":[]}"#.to_owned(),
            _ => format!("v{step}-{d}"),
          };
          devs[d].set(&[(key, &value)]);
          ever_written.insert(key);
        }
        2 => {
          devs[d].delete(key);
          ever_deleted.insert(key);
        }
        _ => {
          let opts = if rng.next(2) == 0 { STARTUP } else { PUSH_ONLY };
          let _ = block_on(run(
            &mut cloud.clone(),
            &mut devs[d].clone(),
            &mut bases[d],
            opts,
          ));
        }
      }
    }
    // 收尾：用户把冲突都解决了（这里直接让第一台的值为准），然后每台设备都同步到稳定。
    for round in 0..4 {
      for d in 0..3 {
        let out = block_on(run(
          &mut cloud.clone(),
          &mut devs[d].clone(),
          &mut bases[d],
          STARTUP,
        ))
        .unwrap();
        if let Some(Problem::Conflict(keys) | Problem::SuspiciousDeletions(keys)) = out.problem {
          for k in keys {
            // 模拟用户选了「以本机为准」：把云端对齐成本机，基线对齐。
            match devs[d].get(&k) {
              Some(v) => {
                cloud.put(&[(k.as_str(), v.as_str())]);
                let digest = Digest::of(&v);
                bases[d].insert(
                  k.clone(),
                  Synced {
                    remote: digest,
                    local: digest,
                  },
                );
              }
              None => {
                cloud.0.borrow_mut().data.remove(&k);
                bases[d].remove(&k);
              }
            }
          }
        }
        let _ = round;
      }
    }
    let reference = synced(&devs[0].data());
    for (i, dev) in devs.iter().enumerate() {
      assert_eq!(
        synced(&dev.data()),
        reference,
        "seed {seed}: 设备 {i} 没有收敛"
      );
    }
    assert_eq!(
      synced(&cloud.data()),
      reference,
      "seed {seed}: 云端与设备不一致"
    );
    // 稳定之后再同步一轮：没有任何变化。
    for d in 0..3 {
      let out = block_on(run(
        &mut cloud.clone(),
        &mut devs[d].clone(),
        &mut bases[d],
        STARTUP,
      ))
      .unwrap();
      assert_eq!(
        (
          out.pulled,
          out.merged,
          out.removed_local,
          out.pushed,
          out.removed_remote,
          out.problem
        ),
        (0, 0, 0, 0, 0, None),
        "seed {seed}: 设备 {d} 在已收敛的状态下又动了数据"
      );
    }
    // 名字里的承诺：写过的 key（没被显式删过的）收敛之后必须还在。
    for key in ever_written.difference(&ever_deleted) {
      assert!(
        reference.contains_key(*key),
        "seed {seed}: 写过的 {key} 丢了"
      );
    }
  }
}

#[test]
fn resolutions_used_by_the_engine_are_the_ones_the_docs_promise() {
  assert_eq!(resolution("bookmarks"), Resolution::Merge);
  assert_eq!(resolution("learning-plan"), Resolution::LastWriterWins);
  assert_eq!(resolution("study-stats"), Resolution::Block);
}
