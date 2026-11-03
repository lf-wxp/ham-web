//! 首页「实时频谱」的动画模型：把时间推进成一组频点幅度（纯逻辑，可单测）。
//!
//! 为什么模型在 core 而不是组件里：柱高怎么随时间变（起振快、余辉慢、峰值保持、
//! 扫描位置、静默时段底噪仍在流）是**读数的行为**，与 Canvas 无关；放在这里才能在
//! 非浏览器环境里验证「静默时画面仍在动」「有信号时峰值抬得起来」这类性质，
//! 而不是靠盯着浏览器猜。
//!
//! 幅度映射的口径与音频驱动一致：
//! - 载波强度 → 柱高（音量）；
//! - 载波中心 → 柱位置（频率）；
//! - 底噪 → 基线，静默窗口里只剩它在流动（没有输入时的待机状态）。

/// 底噪基线（0–1 幅度）。
const FLOOR: f32 = 0.07;
/// 底噪流动的幅度：待机时看得见起伏，但不会顶到有信号的高度。
const FLOOR_FLOW: f32 = 0.06;
/// 每帧叠加的热噪声上限。
const FLOOR_JITTER: f32 = 0.02;
/// 底噪游走回归零的速度（每秒）。太小会一直贴着 ±1 变成方波。
const FLOOR_PULL: f32 = 0.9;
/// 起振时间常数（秒）：小 = 跟手。
const ATTACK_TAU: f32 = 0.05;
/// 余辉时间常数（秒）：大 = 拖尾，像荧光屏的磷光。
const RELEASE_TAU: f32 = 0.40;
/// 峰值保持每秒回落的幅度。
const HOLD_DECAY: f32 = 0.22;
/// 扫描线走完一遍的秒数。
const SCAN_PERIOD: f32 = 3.6;
/// 频点上同时驻留的载波数。
const CARRIERS: usize = 6;
/// 柱数下限：再少就画不出「频段」的形状。
pub const MIN_BINS: usize = 24;

/// 柱数上限（物理上限）：`vec![FLOOR; n]` 的 n 来自调用方，不设上限的话一个异常的画布宽度
/// （或 `usize::MAX`）就会触发 capacity overflow，整个 wasm 实例崩掉。前端的视觉上限
/// （`kit_modern/spectrum.rs` 的 `MAX_BINS`）比它小得多，这里只兜底。
pub const MAX_BINS: usize = 512;
/// 默认随机种子：固定值让首屏每次打开都是同一幅「台况」，不会刷新一次换一个世界。
const DEFAULT_SEED: u32 = 0x9e37_79b9;

/// xorshift32：够用的伪随机，且不引依赖。
struct Rng(u32);

impl Rng {
  const fn new(seed: u32) -> Self {
    // 状态不能为 0（xorshift 会锁死在 0），补一个奇数。
    Self(seed | 1)
  }

  fn next(&mut self) -> f32 {
    self.0 ^= self.0 << 13;
    self.0 ^= self.0 >> 17;
    self.0 ^= self.0 << 5;
    (self.0 % 100_000) as f32 / 100_000.0
  }

  fn range(&mut self, lo: f32, hi: f32) -> f32 {
    lo + self.next() * (hi - lo)
  }
}

/// 一个驻留信号：某个频点上的发射，强度会漂、可能被键控切成断续。
struct Carrier {
  /// 中心频点（0–1 的带宽比例）。
  center: f32,
  /// 峰宽（占带宽比例）。
  width: f32,
  /// 当前强度（已含波段增益）。
  strength: f32,
  /// 无增益、无键控时的满幅强度。
  target: f32,
  /// 中心漂移速度（每秒）。电离层与 VFO 微调都会让信号在频点上滑。
  drift: f32,
  phase: f32,
  /// 键控深度：0 = 常发载波，1 = 满幅发报。
  keying: f32,
  keyed: bool,
  /// 距下一次键控切换的秒数。
  key_left: f32,
  /// 已驻留时长与寿命：到点换一个频点，读起来像「扫到了别的台」。
  age: f32,
  life: f32,
}

impl Carrier {
  fn spawn(rng: &mut Rng) -> Self {
    Self {
      center: rng.range(0.06, 0.94),
      width: rng.range(0.012, 0.045),
      strength: 0.0,
      target: rng.range(0.35, 0.95),
      drift: rng.range(-0.02, 0.02),
      phase: rng.range(0.0, std::f32::consts::TAU),
      // 一半是发报台，一半是常发载波：全是断续会像故障，全常发又像静态图。
      keying: if rng.next() > 0.5 {
        rng.range(0.6, 1.0)
      } else {
        0.0
      },
      keyed: true,
      key_left: rng.range(0.1, 0.5),
      age: 0.0,
      life: rng.range(9.0, 26.0),
    }
  }

  fn step(&mut self, dt: f32, gain: f32, rng: &mut Rng) {
    self.age += dt;
    self.phase += dt;
    self.center += self.drift * dt;
    if self.center < 0.05 {
      self.center = 0.05;
      self.drift = self.drift.abs();
    } else if self.center > 0.95 {
      self.center = 0.95;
      self.drift = -self.drift.abs();
    }

    if self.keying > 0.0 {
      self.key_left -= dt;
      if self.key_left <= 0.0 {
        self.keyed = !self.keyed;
        // 点划长度不一：固定节拍会读成「机器在闪」，不像手键。
        self.key_left = if self.keyed {
          rng.range(0.10, 0.55)
        } else {
          rng.range(0.05, 0.26)
        };
      }
    }
    let keyed = if self.keyed {
      1.0
    } else {
      1.0 - self.keying * 0.85
    };
    // 缓慢起伏：信号本来的衰落（QSB）。
    let fade = 0.85 + 0.15 * self.phase.sin();
    let target = self.target * keyed * fade * gain;
    self.strength += (target - self.strength) * (1.0 - (-dt / 0.07).exp());
  }
}

/// 画布宽度（CSS px）→ 柱数：一根柱子约占 `pitch` px，结果夹在 `[min, max]`。
///
/// 宽度是 NaN / 负数 / 无穷（布局还没算出来、被 `display:none` 的容器）时取 `min`，
/// 不让一个异常宽度把柱数顶到上限或归零。
#[must_use]
pub fn bins_for_width(width: f64, pitch: f64, min: usize, max: usize) -> usize {
  if !width.is_finite() || !pitch.is_finite() || pitch <= 0.0 || width <= 0.0 {
    return min;
  }
  ((width / pitch).round() as usize).clamp(min, max)
}

/// 一屏频谱的全部状态。
pub struct ScopeModel {
  /// 平滑后的柱高（0–1），快起慢落。
  bins: Vec<f32>,
  /// 峰值保持刻度：只回落、不瞬降。
  hold: Vec<f32>,
  /// 每根柱子自己的底噪游走值（-1–1）。
  floor_walk: Vec<f32>,
  carriers: Vec<Carrier>,
  rng: Rng,
  /// 扫描线位置（0–1）。
  scan: f32,
  /// 波段增益：静默窗口里把载波整体压低，只剩底噪。
  gain: f32,
  quiet_left: f32,
  quiet_gap: f32,
  /// 当前总幅度（0–1），供外部判断「现在有没有信号」。
  level: f32,
}

impl ScopeModel {
  /// 按柱数建模型；同样柱数每次打开都是同一幅初始台况。
  pub fn new(bins: usize) -> Self {
    Self::with_seed(bins, DEFAULT_SEED)
  }

  /// 指定随机种子（测试与「换一幅台况」用）。
  pub fn with_seed(bins: usize, seed: u32) -> Self {
    let bins = bins.clamp(MIN_BINS, MAX_BINS);
    let mut rng = Rng::new(seed);
    let carriers: Vec<Carrier> = (0..CARRIERS).map(|_| Carrier::spawn(&mut rng)).collect();
    let floor_walk: Vec<f32> = (0..bins).map(|_| rng.range(-0.5, 0.5)).collect();
    let quiet_gap = rng.range(5.0, 10.0);
    Self {
      bins: vec![FLOOR; bins],
      hold: vec![FLOOR; bins],
      floor_walk,
      carriers,
      rng,
      scan: 0.0,
      gain: 1.0,
      quiet_left: 0.0,
      quiet_gap,
      level: FLOOR,
    }
  }

  /// 柱高（0–1）。
  pub fn bins(&self) -> &[f32] {
    &self.bins
  }

  /// 峰值保持刻度（0–1）。
  pub fn hold(&self) -> &[f32] {
    &self.hold
  }

  /// 扫描线位置（0–1，从左到右，走完一圈回到 0）。
  pub fn scan(&self) -> f32 {
    self.scan
  }

  /// 当前总幅度（0–1）。
  pub fn level(&self) -> f32 {
    self.level
  }

  /// 是否处在「没有信号进来」的待机窗口：此时画面只有底噪在流。
  pub fn is_idle(&self) -> bool {
    self.gain < 0.45
  }

  /// 换柱数（窗口缩放）。按线性插值重采样，形状与峰值都保留，不会闪一下重置。
  pub fn resize(&mut self, bins: usize) {
    let bins = bins.clamp(MIN_BINS, MAX_BINS);
    if bins == self.bins.len() {
      return;
    }
    self.bins = resample(&self.bins, bins);
    self.hold = resample(&self.hold, bins);
    self.floor_walk = resample(&self.floor_walk, bins);
  }

  /// 推进 `dt` 秒。`dt` 上限 0.2 s：切回标签页时时间戳会跳很远，不夹住会让一帧跳过整段动画。
  pub fn step(&mut self, dt: f32) {
    // NaN 必须先挡掉：`f32::clamp` 对 NaN 原样返回，`NaN <= 0.0` 又是 false，于是后面
    // 每一帧都在 NaN 上运算，`bins` / `hold` / `level` 被永久污染 —— 画布静默变空白且
    // 再也不会恢复。上游时间戳出一次 NaN 就够。
    if !dt.is_finite() {
      return;
    }
    let dt = dt.clamp(0.0, 0.2);
    if dt <= 0.0 {
      return;
    }
    self.scan = (self.scan + dt / SCAN_PERIOD).fract();

    // 静默窗口：首页刚打开时不该是一屏满格信号，中间也要有空档才像「真的在听」。
    if self.quiet_left > 0.0 {
      self.quiet_left -= dt;
    } else {
      self.quiet_gap -= dt;
      if self.quiet_gap <= 0.0 {
        self.quiet_left = self.rng.range(2.4, 4.6);
        self.quiet_gap = self.rng.range(9.0, 16.0);
      }
    }
    let target_gain = if self.quiet_left > 0.0 { 0.12 } else { 1.0 };
    self.gain += (target_gain - self.gain) * (1.0 - (-dt / 0.35).exp());

    let attack = 1.0 - (-dt / ATTACK_TAU).exp();
    let release = 1.0 - (-dt / RELEASE_TAU).exp();
    let hold_decay = HOLD_DECAY * dt;
    let gain = self.gain;

    // 一次解构拿到互不重叠的字段：`carriers` 的遍历里还要用 `rng`。
    let Self {
      bins,
      hold,
      floor_walk,
      carriers,
      rng,
      ..
    } = self;

    // 定长数组而不是 `Vec`：这一步每帧都跑，别给渲染循环添一次堆分配。
    let mut alive = [(0.0f32, 1.0f32, 0.0f32); CARRIERS];
    for (slot, c) in alive.iter_mut().zip(carriers.iter_mut()) {
      c.step(dt, gain, rng);
      if c.age > c.life {
        *c = Carrier::spawn(rng);
      }
      *slot = (c.center, c.width, c.strength);
    }

    let n = bins.len();
    let inv_n = 1.0 / n as f32;
    let mut total = 0.0f32;
    for (i, (value, keep)) in bins.iter_mut().zip(hold.iter_mut()).enumerate() {
      let x = (i as f32 + 0.5) * inv_n;

      // 底噪：每根柱子有自己的随机游走 + 一点热噪声 → 整条基线看起来在流。
      let walk = &mut floor_walk[i];
      *walk = (*walk * (1.0 - dt * FLOOR_PULL) + (rng.next() - 0.5) * dt * 2.0).clamp(-1.0, 1.0);
      let mut target = FLOOR + (*walk + 1.0) * 0.5 * FLOOR_FLOW + rng.next() * FLOOR_JITTER;

      for &(center, width, strength) in &alive {
        let d = (x - center) / width;
        // 3σ 之外的高斯已经衰减到 1e-5 以下，`exp()` 是这一帧里最贵的一步，先跳过。
        if d.abs() > 3.0 {
          continue;
        }
        // 取 `max` 而不是相加：两个台挨得近时不该叠出一根假的高柱。
        let bell = (-d * d * 1.7).exp() * strength;
        if bell > target {
          target = bell;
        }
      }

      let k = if target > *value { attack } else { release };
      let v = (*value + (target - *value) * k).clamp(0.0, 1.0);
      *value = v;
      total += v;
      *keep = (*keep - hold_decay).max(v).max(FLOOR);
    }

    self.level = total * inv_n;
  }
}

/// 线性插值重采样。
fn resample(src: &[f32], len: usize) -> Vec<f32> {
  if src.is_empty() {
    return vec![0.0; len];
  }
  (0..len)
    .map(|i| {
      let x = (i as f32 + 0.5) / len as f32 * src.len() as f32 - 0.5;
      let lo = x.floor().max(0.0);
      #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
      let i0 = lo as usize;
      let i1 = (i0 + 1).min(src.len() - 1);
      let k = (x - lo).clamp(0.0, 1.0);
      src[i0] + (src[i1] - src[i0]) * k
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  const STEP: f32 = 1.0 / 30.0;

  fn peak(model: &ScopeModel) -> f32 {
    model.bins().iter().copied().fold(0.0, f32::max)
  }

  #[test]
  fn values_stay_normalized() {
    let mut model = ScopeModel::with_seed(64, 0x1234_5678);
    for _ in 0..900 {
      model.step(STEP);
      assert!(model.bins().iter().all(|v| (0.0..=1.0).contains(v)));
      assert!(model.hold().iter().all(|v| (0.0..=1.0).contains(v)));
      assert!((0.0..1.0).contains(&model.scan()));
      assert!((0.0..=1.0).contains(&model.level()));
    }
  }

  #[test]
  fn carriers_lift_the_trace() {
    // 30 秒里一定出现过明显高过底噪的峰，否则整幅图就是一条抖动的平线。
    let mut model = ScopeModel::with_seed(64, 0x1234_5678);
    let mut top = 0.0f32;
    // 只在静默窗口内取样：初值本来就是底噪、载波又从 0 起振，开头几帧「幅度很低」
    // 与有没有空档无关，混进来会让这条断言恒真。
    let mut idle_frames = 0u32;
    let mut idle_level = 1.0f32;
    for _ in 0..900 {
      model.step(STEP);
      top = top.max(peak(&model));
      if model.is_idle() {
        idle_frames += 1;
        idle_level = idle_level.min(model.level());
      }
    }
    assert!(top > 0.5, "频谱始终没抬起来：{top}");
    assert!(idle_frames > 0, "30 秒里没出现过静默窗口，没有空档");
    assert!(idle_level < 0.3, "静默窗口里幅度仍然很高：{idle_level}");
  }

  #[test]
  fn hold_trails_above_bars() {
    let mut model = ScopeModel::with_seed(48, 99);
    for _ in 0..300 {
      model.step(STEP);
      for (bar, held) in model.bins().iter().zip(model.hold()) {
        assert!(*held + 1e-4 >= *bar, "峰值刻度掉到柱高以下：{held} < {bar}");
      }
    }
  }

  #[test]
  fn idle_window_keeps_the_floor_flowing() {
    // 静默窗口里没有任何信号，但底噪必须还在动 —— 否则首屏会像一张截图。
    let mut model = ScopeModel::with_seed(64, 0x2468_ace0);
    for _ in 0..900 {
      model.step(STEP);
      if model.is_idle() {
        break;
      }
    }
    assert!(model.is_idle(), "30 秒内没进过静默窗口");
    // 先等柱子落到底噪上：刚进窗口时载波还在衰减（柱的释放时间常数约 0.4 s），
    // 那时柱高必然在变，与「底噪是不是活的」无关。静默窗口至少 2.4 s，等 1.5 s 仍在窗口内。
    for _ in 0..45 {
      model.step(STEP);
    }
    assert!(model.is_idle(), "等待期间窗口提前结束了，测试前提不成立");
    assert!(peak(&model) < 0.5, "静默窗口里仍有满格柱：{}", peak(&model));

    let before = model.bins().to_vec();
    for _ in 0..6 {
      model.step(STEP);
    }
    assert!(model.is_idle());
    let moved = before
      .iter()
      .zip(model.bins())
      .any(|(a, b)| (a - b).abs() > 0.002);
    assert!(moved, "静默窗口里的底噪是死的");
  }

  #[test]
  fn non_finite_dt_does_not_poison_the_model() {
    // 上游时间戳偶尔会给 NaN / inf：夹不住的话整幅状态会被污染成 NaN，画布静默变空白。
    let mut model = ScopeModel::with_seed(48, 5);
    for _ in 0..30 {
      model.step(STEP);
    }
    let before = model.bins().to_vec();
    model.step(f32::NAN);
    model.step(f32::INFINITY);
    model.step(f32::NEG_INFINITY);
    assert_eq!(model.bins(), before.as_slice());
    assert!(model.level().is_finite());
    // 之后仍然能正常推进。
    model.step(STEP);
    assert!(model.bins().iter().all(|v| v.is_finite()));
  }

  #[test]
  fn resize_keeps_the_shape() {
    let mut model = ScopeModel::with_seed(64, 7);
    for _ in 0..60 {
      model.step(STEP);
    }
    // 记下最高柱的位置与高度：重采样后「同一相对位置」的高度应当接近。
    // 底噪托底在 0.07，只断言「峰值 > 0.05」抹平了形状也照样通过。
    let (peak_at, peak_height) =
      model
        .bins()
        .iter()
        .copied()
        .enumerate()
        .fold(
          (0, 0.0f32),
          |best, (i, v)| if v > best.1 { (i, v) } else { best },
        );
    assert!(
      peak_height > 0.2,
      "测试前提不成立：重采样前没有明显的峰（{peak_height}）"
    );

    model.resize(128);
    assert_eq!(model.bins().len(), 128);
    assert_eq!(model.hold().len(), 128);
    assert!(model.bins().iter().all(|v| (0.0..=1.0).contains(v)));
    let mapped = (((peak_at as f32 + 0.5) / 64.0) * 128.0 - 0.5).round() as usize;
    let near = model.bins()[mapped.saturating_sub(1)..=(mapped + 1).min(127)]
      .iter()
      .copied()
      .fold(0.0f32, f32::max);
    assert!(
      (near - peak_height).abs() < 0.15,
      "重采样后峰高变了：{peak_height} → {near}（形状被抹平了）"
    );

    model.resize(4);
    assert_eq!(model.bins().len(), MIN_BINS);
    model.step(STEP);
    assert!(model.bins().iter().all(|v| (0.0..=1.0).contains(v)));
  }

  #[test]
  fn bins_follow_width_within_limits() {
    // 8px 一根：800px → 100 根；窄屏取下限，超宽取上限。
    assert_eq!(bins_for_width(800.0, 8.0, 28, 132), 100);
    assert_eq!(bins_for_width(100.0, 8.0, 28, 132), 28);
    assert_eq!(bins_for_width(5000.0, 8.0, 28, 132), 132);
    // 布局没算出来 / 容器被隐藏：取下限，而不是 NaN 转整型后的 0 或上限。
    assert_eq!(bins_for_width(f64::NAN, 8.0, 28, 132), 28);
    assert_eq!(bins_for_width(f64::INFINITY, 8.0, 28, 132), 28);
    assert_eq!(bins_for_width(-10.0, 8.0, 28, 132), 28);
    assert_eq!(bins_for_width(0.0, 8.0, 28, 132), 28);
    assert_eq!(bins_for_width(800.0, 0.0, 28, 132), 28);
  }

  #[test]
  fn bin_count_is_clamped_on_both_sides() {
    // 柱数来自调用方（画布宽度），异常值不能让 `vec![_; n]` 溢出。
    assert_eq!(ScopeModel::new(usize::MAX).bins().len(), MAX_BINS);
    assert_eq!(ScopeModel::new(0).bins().len(), MIN_BINS);
    let mut model = ScopeModel::new(64);
    model.resize(usize::MAX);
    assert_eq!(model.bins().len(), MAX_BINS);
  }

  #[test]
  fn scan_sweeps_the_whole_band() {
    let mut model = ScopeModel::with_seed(32, 3);
    let mut covered = 0.0f32;
    for _ in 0..200 {
      model.step(STEP);
      covered = covered.max(model.scan());
    }
    assert!(covered > 0.9, "扫描线没走到底：{covered}");
  }
}
