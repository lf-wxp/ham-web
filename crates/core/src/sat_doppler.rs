//! 卫星多普勒修正：由卫星相对地面站的径向速度算出该调的收发频率，并在服务端给出的采样轨迹上逐秒插值；
//! 另含「自动调谐写电台」的纯逻辑（写入节流、手动改频检测）。
//!
//! 公式（`ṙ` = 径向速度，远离为正，km/s；`c` = 299 792.458 km/s；经典非相对论多普勒）：
//!
//! ```text
//! 下行  地面收到  f_rx = f_dl / (1 + ṙ/c)        卫星是运动的声源：靠近（ṙ < 0）时频率升高
//! 上行  要让卫星收到 f_up，地面应发  f_tx = f_up / (1 − ṙ/c)   卫星是运动的观察者
//! ```
//!
//! 一阶近似是 `Δf ≈ −f·ṙ/c`；这里用上面的闭式，两者在 ISS 的 7.8 km/s、437.8 MHz 下只差约 0.3 Hz。
//!
//! 轨迹本身（方位、仰角、距离、径向速度）由服务端用 SGP4 按固定步长算好，本模块只做**插值**：
//! 5 s 步长、线性插值在最坏的「低轨直飞头顶」情形下径向速度误差约 7 m/s，折合 437.8 MHz 上 11 Hz，
//! 远小于 FM 的容忍度（千赫兹量级），所以不用更高阶的插值。
//!
//! 边界（先读这个）：
//! - **只修正固定频率**。线性转发器（AO-7、FO-29 等）的上下行互相耦合（反相、随调谐联动），
//!   本模块不处理这种耦合；但对「某个固定的下行频点」，修正本身是对的；
//! - 精度取决于 TLE 的新旧与**电脑时钟**：时钟差 1 s，最坏约 175 Hz（437.8 MHz，过顶时）；
//! - 径向速度来自简化的 TEME→ECEF 旋转（只转 GMST，不含岁差、章动与极移），对多普勒与指向都远小于上述误差；
//! - 写电台只写**一个频率**（下行修正频率），不碰模式、不开分集、不写发射频率 —— 见 [`should_write`]。

use serde::{Deserialize, Serialize};

use crate::sat_watch::Pass;

/// 光速（km/s）。
pub const C_KM_S: f64 = 299_792.458;
/// 服务端采样步长默认值（秒）。
pub const DEFAULT_STEP_SECS: u32 = 5;
/// 采样步长上限（秒）：再稀疏，线性插值的误差就不再可忽略（10 s 步长约 43 Hz，30 s 约 380 Hz）。
pub const MAX_STEP_SECS: u32 = 30;
/// 一次请求最多覆盖的秒数（足以容纳一次低轨过境加前后余量）。
pub const MAX_TRACK_SECS: u32 = 1800;
/// 过境前后各多取的秒数：过境列表按「最小仰角」算 AOS，卫星其实更早就过了地平线。
pub const PASS_PAD_SECS: i64 = 300;

/// 两次写电台之间，目标频率至少要变化这么多 Hz 才再写一次（FM 信道约 15 kHz 宽，100 Hz 的滞后可以忽略；
/// 写得太勤会让不少电台的静噪反复闪一下）。
pub const MIN_WRITE_DELTA_HZ: u64 = 100;
/// 写入前把频率凑到的整数 Hz 间隔：多数电台的最小步进是 10 Hz，给它整 10 Hz 的值就不会被它自己再四舍五入。
pub const WRITE_RESOLUTION_HZ: u64 = 10;
/// 电台回读频率与我们上次写入的偏差超过这么多 Hz，就视为「有人动了电台」。
pub const OVERRIDE_TOLERANCE_HZ: u64 = 1000;
/// 连续这么多次回读都偏离，才判定为手动改频（写入后电台回读有一两拍滞后，单次偏离不算）。
pub const OVERRIDE_CONFIRM_POLLS: u32 = 3;

fn positive(v: f64) -> bool {
  v.is_finite() && v > 0.0
}

fn rate_ok(rr: f64) -> bool {
  rr.is_finite() && rr.abs() < C_KM_S
}

/// 地面站该调到的**接收**频率（Hz）：卫星在 `downlink_hz` 上发射、径向速度为 `range_rate_km_s` 时。
#[must_use]
pub fn rx_frequency_hz(downlink_hz: f64, range_rate_km_s: f64) -> Option<f64> {
  (positive(downlink_hz) && rate_ok(range_rate_km_s))
    .then(|| downlink_hz / (1.0 + range_rate_km_s / C_KM_S))
}

/// 地面站该**发射**的频率（Hz）：为了让卫星恰好收到 `uplink_hz`。
#[must_use]
pub fn tx_frequency_hz(uplink_hz: f64, range_rate_km_s: f64) -> Option<f64> {
  (positive(uplink_hz) && rate_ok(range_rate_km_s))
    .then(|| uplink_hz / (1.0 - range_rate_km_s / C_KM_S))
}

/// 轨迹上的一个采样（与 `/api/sat-track` 返回的字段一致）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Sample {
  /// Unix 秒。
  pub t: i64,
  /// 方位角（度，从北顺时针）。
  pub az: f64,
  /// 仰角（度，负数在地平线下）。
  pub el: f64,
  pub range_km: f64,
  /// 径向速度（km/s）：远离为正。
  pub range_rate_km_s: f64,
}

/// 某一时刻的插值结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Observation {
  pub az: f64,
  pub el: f64,
  pub range_km: f64,
  pub range_rate_km_s: f64,
}

/// 轨迹不合法的原因。序号从 1 起算。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackError {
  /// 少于 2 个采样，没法插值。
  TooFew,
  /// 第 `n` 个采样的时间没有比前一个更大。
  NotIncreasing(usize),
  /// 第 `n` 个采样与前一个相隔超过 [`MAX_STEP_SECS`]。
  GapTooLarge(usize),
  /// 第 `n` 个采样含非有限数。
  NotFinite(usize),
}

/// 按时间排序的采样轨迹。
#[derive(Debug, Clone, PartialEq)]
pub struct Track {
  samples: Vec<Sample>,
}

impl Track {
  /// 校验并构造：至少 2 个采样、时间严格递增、相邻间隔不超过 [`MAX_STEP_SECS`]、数值有限。
  pub fn new(samples: Vec<Sample>) -> Result<Self, TrackError> {
    if samples.len() < 2 {
      return Err(TrackError::TooFew);
    }
    for (i, s) in samples.iter().enumerate() {
      if ![s.az, s.el, s.range_km, s.range_rate_km_s]
        .iter()
        .all(|v| v.is_finite())
      {
        return Err(TrackError::NotFinite(i + 1));
      }
    }
    for (i, w) in samples.windows(2).enumerate() {
      if w[1].t <= w[0].t {
        return Err(TrackError::NotIncreasing(i + 2));
      }
      if w[1]
        .t
        .checked_sub(w[0].t)
        .is_none_or(|gap| gap > i64::from(MAX_STEP_SECS))
      {
        return Err(TrackError::GapTooLarge(i + 2));
      }
    }
    Ok(Self { samples })
  }

  /// 第一个采样的时间（Unix 秒）。
  #[must_use]
  pub fn start(&self) -> i64 {
    self.samples[0].t
  }

  /// 最后一个采样的时间（Unix 秒）。
  #[must_use]
  pub fn end(&self) -> i64 {
    self.samples[self.samples.len() - 1].t
  }

  /// 在 `t`（Unix 秒，可带小数）处线性插值；超出 `[start, end]` 或 `t` 非有限数时为 `None`。
  ///
  /// 方位角沿**短弧**插值（359° → 1° 经过 0°，而不是绕一大圈）。
  #[must_use]
  pub fn at(&self, t: f64) -> Option<Observation> {
    if !t.is_finite() || t < self.start() as f64 || t > self.end() as f64 {
      return None;
    }
    // 第一个时间 > t 的采样位置；t 恰好等于末尾时没有这样的采样，取最后一段的右端。
    let hi = self
      .samples
      .partition_point(|s| (s.t as f64) <= t)
      .clamp(1, self.samples.len() - 1);
    let (a, b) = (&self.samples[hi - 1], &self.samples[hi]);
    let w = (t - a.t as f64) / (b.t - a.t) as f64;
    let lerp = |x: f64, y: f64| x + (y - x) * w;
    let da = (b.az - a.az + 540.0).rem_euclid(360.0) - 180.0;
    Some(Observation {
      az: (a.az + da * w).rem_euclid(360.0),
      el: lerp(a.el, b.el),
      range_km: lerp(a.range_km, b.range_km),
      range_rate_km_s: lerp(a.range_rate_km_s, b.range_rate_km_s),
    })
  }
}

/// 该卫星在 `now` 之后（含正在进行）的下一次过境；`passes` 不要求有序。
#[must_use]
pub fn next_pass(passes: &[Pass], norad: u64, now: i64) -> Option<&Pass> {
  passes
    .iter()
    .filter(|p| p.norad == norad && p.los > now)
    .min_by_key(|p| p.aos)
}

/// 取一次过境的轨迹要请求的窗口：`(起始 Unix 秒, 总秒数)`，前后各多 [`PASS_PAD_SECS`]，总长不超过 [`MAX_TRACK_SECS`]。
#[must_use]
pub fn track_window(pass: &Pass) -> (i64, u32) {
  let secs = (pass.los - pass.aos + 2 * PASS_PAD_SECS).clamp(1, i64::from(MAX_TRACK_SECS));
  (pass.aos - PASS_PAD_SECS, secs as u32)
}

/// 一颗预置的 FM 卫星：NORAD 编号与上下行频率（Hz）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Preset {
  pub name: &'static str,
  pub norad: u64,
  pub uplink_hz: u64,
  pub downlink_hz: u64,
}

/// 预置的 FM 卫星。频率取自 [`crate::satellites::SATELLITES`]（单测逐条对照），NORAD 编号对照 Celestrak 业余卫星组。
///
/// 只收了在 Celestrak 业余卫星组里能取到 TLE 的卫星；表里的 AO-92 当前不在该组，所以没放进来。
pub const PRESETS: &[Preset] = &[
  Preset {
    name: "ISS",
    norad: 25_544,
    uplink_hz: 145_990_000,
    downlink_hz: 437_800_000,
  },
  Preset {
    name: "SO-50",
    norad: 27_607,
    uplink_hz: 145_850_000,
    downlink_hz: 436_795_000,
  },
  Preset {
    name: "AO-91",
    norad: 43_017,
    uplink_hz: 435_250_000,
    downlink_hz: 145_960_000,
  },
];

/// 把浮点目标频率凑成电台能收的整数 Hz（四舍五入到 [`WRITE_RESOLUTION_HZ`]）；非正或非有限数为 `None`。
#[must_use]
pub fn round_for_rig(hz: f64) -> Option<u64> {
  if !positive(hz) || hz > 1.0e11 {
    return None;
  }
  let step = WRITE_RESOLUTION_HZ as f64;
  Some(((hz / step).round() * step) as u64)
}

/// 这次要不要再写一次电台：没写过就写；否则目标与上次写入值至少相差 [`MIN_WRITE_DELTA_HZ`]。
#[must_use]
pub fn should_write(last_written: Option<u64>, target: u64) -> bool {
  last_written.is_none_or(|last| last.abs_diff(target) >= MIN_WRITE_DELTA_HZ)
}

/// 手动改频检测：电台回读的频率连续多次偏离我们上次写入的值，就说明有人转了旋钮或换了波段。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OverrideDetector {
  deviating: u32,
}

impl OverrideDetector {
  /// 喂入一次回读；返回「现在可以判定为手动改频」。
  ///
  /// 还没写过（`last_written` 为 `None`）时没有可比较的对象，返回 `false`。偏差在 [`OVERRIDE_TOLERANCE_HZ`] 内
  /// 视为一致并清零计数；连续 [`OVERRIDE_CONFIRM_POLLS`] 次偏离才返回 `true`。
  pub fn observe(&mut self, last_written: Option<u64>, rig_hz: u64) -> bool {
    match last_written {
      Some(last) if last.abs_diff(rig_hz) > OVERRIDE_TOLERANCE_HZ => {
        self.deviating = self.deviating.saturating_add(1);
        self.deviating >= OVERRIDE_CONFIRM_POLLS
      }
      _ => {
        self.deviating = 0;
        false
      }
    }
  }

  /// 偏离确认期间暂停写入，不能用自己的写回来清掉手动改频证据。
  #[must_use]
  pub const fn write_allowed(&self) -> bool {
    self.deviating == 0
  }

  /// 清零（重新连接、重新开启自动调谐时用）。
  pub fn reset(&mut self) {
    self.deviating = 0;
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::satellites::SATELLITES;

  #[test]
  fn pending_manual_override_blocks_new_targets() {
    let mut detector = OverrideDetector::default();
    assert!(!detector.observe(Some(437_800_000), 437_800_000 + OVERRIDE_TOLERANCE_HZ + 1));
    assert!(!detector.write_allowed());
    assert!(should_write(Some(437_800_000), 437_799_790));
    for _ in 0..OVERRIDE_CONFIRM_POLLS {
      detector.observe(Some(437_800_000), 437_800_000 + OVERRIDE_TOLERANCE_HZ + 1);
    }
    assert!(!detector.write_allowed());
    detector.reset();
    assert!(detector.write_allowed());
  }

  fn near(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
  }

  fn sample(t: i64, az: f64, el: f64, range: f64, rr: f64) -> Sample {
    Sample {
      t,
      az,
      el,
      range_km: range,
      range_rate_km_s: rr,
    }
  }

  // 以下对拍值由与实现无关的 Python 脚本独立算出（c = 299 792.458 km/s，经典多普勒闭式）。
  #[test]
  fn doppler_matches_independently_computed_values() {
    for (f, rr, rx, tx) in [
      (437.8e6, -7.0, 437_810_222.644, 437_789_777.833),
      (437.8e6, 7.0, 437_789_777.833, 437_810_222.644),
      (437.8e6, 0.0, 437.8e6, 437.8e6),
      (145.99e6, -7.0, 145_993_408.871, 145_986_591.288),
      (436.795e6, -3.5, 436_800_099.529, 436_789_900.590),
    ] {
      assert!(
        near(rx_frequency_hz(f, rr).unwrap(), rx, 0.01),
        "rx {f} {rr}"
      );
      assert!(
        near(tx_frequency_hz(f, rr).unwrap(), tx, 0.01),
        "tx {f} {rr}"
      );
    }
  }

  #[test]
  fn approaching_raises_the_downlink_and_lowers_the_uplink() {
    // 靠近（ṙ < 0）：听到的下行偏高；要让卫星收到标称上行，地面得发低一点。
    assert!(rx_frequency_hz(437.8e6, -5.0).unwrap() > 437.8e6);
    assert!(tx_frequency_hz(145.99e6, -5.0).unwrap() < 145.99e6);
    // 远离反过来。
    assert!(rx_frequency_hz(437.8e6, 5.0).unwrap() < 437.8e6);
    assert!(tx_frequency_hz(145.99e6, 5.0).unwrap() > 145.99e6);
  }

  #[test]
  fn rx_and_tx_are_mirror_images() {
    for rr in [-7.6, -3.0, 0.4, 6.1] {
      let f = 436.795e6;
      assert!(near(
        rx_frequency_hz(f, rr).unwrap(),
        tx_frequency_hz(f, -rr).unwrap(),
        1e-6
      ));
    }
  }

  #[test]
  fn transmitting_the_corrected_uplink_lands_on_the_nominal_uplink() {
    // 地面发 f_tx，卫星（以 ṙ 远离）收到 f_tx·(1 − ṙ/c)，必须回到标称上行。
    for rr in [-7.6, -2.0, 0.0, 3.3, 7.6] {
      let up = 145.85e6;
      let tx = tx_frequency_hz(up, rr).unwrap();
      assert!(near(tx * (1.0 - rr / C_KM_S), up, 1e-6), "rr {rr}");
    }
  }

  #[test]
  fn the_closed_form_is_within_a_fraction_of_a_hertz_of_the_first_order_shift() {
    // 文档里的断言：ISS 极端情形（7.8 km/s、437.8 MHz）下两者只差约 0.3 Hz。
    let (f, rr) = (437.8e6, -7.8);
    let first_order = f - f * rr / C_KM_S;
    let gap = (rx_frequency_hz(f, rr).unwrap() - first_order).abs();
    assert!(gap < 0.4, "{gap}");
  }

  #[test]
  fn doppler_rejects_nonsense() {
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
      assert_eq!(rx_frequency_hz(bad, 1.0), None, "{bad}");
      assert_eq!(tx_frequency_hz(bad, 1.0), None, "{bad}");
    }
    for rr in [f64::NAN, f64::INFINITY, C_KM_S, -C_KM_S] {
      assert_eq!(rx_frequency_hz(437.8e6, rr), None, "{rr}");
      assert_eq!(tx_frequency_hz(437.8e6, rr), None, "{rr}");
    }
  }

  /// 直线飞过头顶的解析模型：高度 d、速度 v、t = 0 时过顶，径向速度 `v²t / √(d² + v²t²)`。
  fn straight_line(t: f64, d: f64, v: f64) -> f64 {
    v * v * t / (d * d + v * v * t * t).sqrt()
  }

  fn model_track(step: i64) -> Track {
    let base = 1_760_000_000;
    let samples = (-400..=400)
      .step_by(step as usize)
      .map(|k| {
        let rr = straight_line(f64::from(k), 400.0, 7.6);
        sample(base + i64::from(k), 90.0, 30.0, 1000.0, rr)
      })
      .collect();
    Track::new(samples).unwrap()
  }

  #[test]
  fn interpolation_error_on_the_analytic_model_matches_the_documented_bounds() {
    // 文档里的数字：5 s 步长最坏 ≈ 7.3 m/s（≈ 11 Hz @ 437.8 MHz），2 s 步长 ≈ 1.2 m/s。
    for (step, bound_m_s) in [(5, 8.0), (2, 1.3)] {
      let track = model_track(step);
      let mut worst = 0.0f64;
      let mut t = -399.9;
      while t < 399.9 {
        let got = track.at(1_760_000_000.0 + t).unwrap().range_rate_km_s;
        worst = worst.max((got - straight_line(t, 400.0, 7.6)).abs() * 1000.0);
        t += 0.05;
      }
      assert!(worst <= bound_m_s, "step {step}: {worst} m/s");
      assert!(worst > 0.1, "误差不该是 0，说明测试没测到东西：{worst}");
    }
    // 折合频率：5 s 步长 ≤ 11 Hz。
    let hz = 8.0 / 1000.0 / C_KM_S * 437.8e6;
    assert!(hz < 12.0, "{hz}");
  }

  #[test]
  fn interpolation_is_exact_at_the_samples_and_linear_between() {
    let t = Track::new(vec![
      sample(100, 10.0, 0.0, 2000.0, -6.0),
      sample(105, 20.0, 10.0, 1800.0, -4.0),
      sample(110, 30.0, 20.0, 1700.0, 0.0),
    ])
    .unwrap();
    let at = |x: f64| t.at(x).unwrap();
    assert_eq!(at(100.0).el, 0.0);
    assert_eq!(at(105.0).range_km, 1800.0);
    assert_eq!(at(110.0).range_rate_km_s, 0.0);
    // 中点：各量取两端平均。
    let m = at(102.5);
    assert!(near(m.az, 15.0, 1e-12) && near(m.el, 5.0, 1e-12));
    assert!(near(m.range_km, 1900.0, 1e-9) && near(m.range_rate_km_s, -5.0, 1e-12));
    // 带小数的时刻落在第二段。
    assert!(near(at(107.0).range_rate_km_s, -4.0 + 4.0 * 0.4, 1e-12));
  }

  #[test]
  fn azimuth_takes_the_short_way_round() {
    let t = Track::new(vec![
      sample(0, 359.0, 0.0, 1.0, 0.0),
      sample(2, 1.0, 0.0, 1.0, 0.0),
      sample(4, 350.0, 0.0, 1.0, 0.0),
      sample(6, 10.0, 0.0, 1.0, 0.0),
    ])
    .unwrap();
    assert!(near(t.at(1.0).unwrap().az, 0.0, 1e-9)); // 359 → 1：经过 0
    assert!(near(t.at(3.0).unwrap().az, 355.5, 1e-9)); // 1 → 350：往回走 11°，中点 1 − 5.5
    assert!(near(t.at(5.0).unwrap().az, 0.0, 1e-9)); // 350 → 10：经过 0
    assert!(near(t.at(1.5).unwrap().az, 0.5, 1e-9)); // 359 → 1 的 3/4 处：359 + 2·0.75 − 360
    for k in 0..=120 {
      let az = t.at(f64::from(k) * 0.05).unwrap().az;
      assert!((0.0..360.0).contains(&az), "{az}");
    }
  }

  #[test]
  fn outside_the_track_there_is_no_answer() {
    let t = Track::new(vec![
      sample(100, 0.0, 0.0, 1.0, 0.0),
      sample(105, 0.0, 0.0, 1.0, 0.0),
    ])
    .unwrap();
    assert_eq!((t.start(), t.end()), (100, 105));
    assert!(t.at(99.999).is_none() && t.at(105.001).is_none());
    assert!(t.at(100.0).is_some() && t.at(105.0).is_some());
    assert!(t.at(f64::NAN).is_none() && t.at(f64::INFINITY).is_none());
  }

  #[test]
  fn invalid_tracks_are_rejected_with_the_position() {
    let ok = |t, rr| sample(t, 0.0, 0.0, 1.0, rr);
    assert_eq!(Track::new(vec![]).unwrap_err(), TrackError::TooFew);
    assert_eq!(
      Track::new(vec![ok(0, 0.0)]).unwrap_err(),
      TrackError::TooFew
    );
    assert_eq!(
      Track::new(vec![ok(0, 0.0), ok(5, 0.0), ok(5, 0.0)]).unwrap_err(),
      TrackError::NotIncreasing(3)
    );
    assert_eq!(
      Track::new(vec![ok(0, 0.0), ok(5, 0.0), ok(3, 0.0)]).unwrap_err(),
      TrackError::NotIncreasing(3)
    );
    assert_eq!(
      Track::new(vec![ok(0, 0.0), ok(31, 0.0)]).unwrap_err(),
      TrackError::GapTooLarge(2)
    );
    assert!(Track::new(vec![ok(0, 0.0), ok(30, 0.0)]).is_ok());
    assert_eq!(
      Track::new(vec![ok(0, 0.0), ok(5, f64::NAN)]).unwrap_err(),
      TrackError::NotFinite(2)
    );
  }

  fn pass(norad: u64, aos: i64, los: i64) -> Pass {
    Pass {
      name: String::new(),
      norad,
      aos,
      los,
      max_elev: 40.0,
      max_elev_time: (aos + los) / 2,
      azimuth: 180.0,
    }
  }

  #[test]
  fn next_pass_picks_the_current_or_earliest_future_pass_of_that_satellite() {
    let list = [
      pass(25_544, 5000, 5600),
      pass(27_607, 1500, 2100),
      pass(25_544, 1000, 1600), // 无序给出
      pass(25_544, 100, 700),
    ];
    // 此刻 1200：1000–1600 这次正在进行，应选它而不是已经结束的、也不是更晚的。
    assert_eq!(next_pass(&list, 25_544, 1200).unwrap().aos, 1000);
    // 恰好等于 LOS：这次已经结束。
    assert_eq!(next_pass(&list, 25_544, 1600).unwrap().aos, 5000);
    assert_eq!(next_pass(&list, 27_607, 0).unwrap().aos, 1500);
    assert!(next_pass(&list, 25_544, 5600).is_none());
    assert!(next_pass(&list, 99_999, 0).is_none());
    assert!(next_pass(&[], 25_544, 0).is_none());
  }

  #[test]
  fn the_track_window_pads_both_sides_and_is_capped() {
    // 10 分钟的过境：前后各 300 s → 起点提前 300 s，总长 1200 s。
    assert_eq!(track_window(&pass(1, 10_000, 10_600)), (9_700, 1_200));
    // 超长的（不应出现）被截到上限，起点仍然只提前 PASS_PAD_SECS。
    assert_eq!(track_window(&pass(1, 0, 5_000)), (-300, MAX_TRACK_SECS));
    // 退化的零长过境至少有 2·PAD。
    assert_eq!(track_window(&pass(1, 100, 100)).1, 600);
    // 窗口覆盖整个过境。
    let p = pass(1, 20_000, 20_840);
    let (start, secs) = track_window(&p);
    assert!(start < p.aos && start + i64::from(secs) > p.los);
  }

  #[test]
  fn presets_agree_with_the_reviewed_satellite_table() {
    // 预置频率不是新数据：必须能在 SATELLITES 对应条目的上 / 下行文字里找到同样的数。
    for (name, table_name) in [
      ("ISS", "国际空间站"),
      ("SO-50", "SO-50"),
      ("AO-91", "AO-91"),
    ] {
      let preset = PRESETS.iter().find(|p| p.name == name).unwrap();
      let entry = SATELLITES.iter().find(|s| s.name == table_name).unwrap();
      let mhz = |hz: u64| format!("{:.3} MHz", hz as f64 / 1e6);
      assert!(
        entry.uplink.contains(&mhz(preset.uplink_hz)),
        "{name} 上行 {}",
        entry.uplink
      );
      assert!(
        entry.downlink.contains(&mhz(preset.downlink_hz)),
        "{name} 下行 {}",
        entry.downlink
      );
    }
    let mut ids: Vec<u64> = PRESETS.iter().map(|p| p.norad).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), PRESETS.len(), "NORAD 编号重复");
  }

  #[test]
  fn rig_frequencies_are_rounded_to_ten_hertz() {
    assert_eq!(round_for_rig(437_810_222.644), Some(437_810_220));
    assert_eq!(round_for_rig(437_810_225.0), Some(437_810_230)); // 5 向上
    assert_eq!(round_for_rig(145_000_004.9), Some(145_000_000));
    for bad in [0.0, -5.0, f64::NAN, f64::INFINITY, 2.0e11] {
      assert_eq!(round_for_rig(bad), None, "{bad}");
    }
  }

  #[test]
  fn writes_are_throttled_but_the_first_one_always_goes_out() {
    assert!(should_write(None, 437_800_000));
    assert!(!should_write(Some(437_800_000), 437_800_000));
    assert!(!should_write(Some(437_800_000), 437_800_090));
    assert!(should_write(Some(437_800_000), 437_800_100));
    assert!(should_write(Some(437_800_100), 437_800_000)); // 向下同理
  }

  #[test]
  fn a_manual_retune_needs_several_consecutive_deviating_readings() {
    let mut d = OverrideDetector::default();
    let last = Some(437_810_220);
    // 还没写过：没有可比较的对象。
    assert!(!d.observe(None, 14_074_000));
    // 刚写完的头两拍电台可能还在回读旧频率：不算。
    assert!(!d.observe(last, 14_074_000));
    assert!(!d.observe(last, 14_074_000));
    // 第三次仍然偏离：确认是手动改频。
    assert!(d.observe(last, 14_074_000));
    // 一旦对上就清零，之后又得重新攒够 3 次。
    let mut d = OverrideDetector::default();
    assert!(!d.observe(last, 14_074_000));
    assert!(!d.observe(last, 14_074_000));
    assert!(!d.observe(last, 437_810_300)); // 回到容差内
    assert!(!d.observe(last, 14_074_000));
    assert!(!d.observe(last, 14_074_000));
    assert!(d.observe(last, 14_074_000));
    // 容差边界：恰好 1 kHz 不算偏离，多 1 Hz 算。
    let mut d = OverrideDetector::default();
    for _ in 0..5 {
      assert!(!d.observe(last, 437_811_220));
    }
    let mut d = OverrideDetector::default();
    assert!(!d.observe(last, 437_811_221));
    assert!(!d.observe(last, 437_811_221));
    assert!(d.observe(last, 437_811_221));
    // reset 清零。
    d.reset();
    assert!(!d.observe(last, 14_074_000));
  }
}
