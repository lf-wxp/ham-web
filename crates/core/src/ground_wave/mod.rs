//! 地波场强与基本传输损耗（ITU-R P.368 的 LFMF 平滑地球法）。
//!
//! # 来源与许可
//!
//! 算法移植自 ITU-R P.368-10 的**随机附带的源码**（`R-REC-P.368-10-202208-I!!ZIP-E.zip` 里的
//! `Integral Source Code/LFMF Source`，作者 William Kozma Jr，US Dept of Commerce, NTIA/ITS，
//! 2021 年 6 月）。NTIA 的 `Readme (Integral)` 写明：NTIA 雇员的作品在美国不受版权保护，并额外授予
//! 「non-exclusive irrevocable and unconditional right to print, publish, prepare derivative works
//! and distribute … royalty-free」，只要求改动处注明、并致谢 NTIA。
//!
//! **注意区分**：P.368 的**推荐书正文**（PDF）仍是「All rights reserved」，不能照抄；
//! 这里用的是那份**源码**，它的条件如上。
//!
//! 本移植的改动：改用本站的复数类型、把泰勒展开中心的四张系数表按程序导出（不手抄）、
//! 去掉原代码里 C 与 C++ 混用的类型转换与多余的中间变量、把「方法选择」与「输入校验」写成
//! 显式的枚举与 `Option`。数值口径没有改动 —— 与原件逐点对拍，见 `reference/lfmf_check.py`。
//!
//! # 方法与适用条件
//!
//! 两条解法自动选择（原代码的判据，`d < 80·f^(−1/3)` km 用第一条）：
//! - **平地 + 曲率修正**：近距离、地面波还没被地球曲率挡住；
//! - **残留级数**（Fock / Wait）：远距离、绕射区。
//!
//! 频率 0.01–30 MHz、两端天线高度 0–50 m、距离 0.001–10 000 km、地面相对介电常数 ≥ 1、
//! 电导率 > 0 S/m。地面按**平滑球面**处理（不做地形起伏、不做海陆混合路径）。
//! 参考天线是架在理想导体地上的**短垂直单极子**，增益取 4.77 dBi（原代码的常数）。
//!
//! # 本模块不收的
//!
//! - 地波之外的传播方式（天波、Es、对流层散射）：那是别处的模型；
//! - ITU 的区域地面电导率图：没有可程序化引用的数据文件，页面让用户自己填电导率；
//! - 混合路径（海陆分界）：原代码本身也不做。

mod airy;
mod cxp;
mod tables;
mod wofz;

use crate::cx::Cx;
use cxp::powi;

/// 真空介电常数（F/m），与原件同值。
const EPSILON_0: f64 = 8.854_187_817e-12;
/// 地球平均半径（km），与原件同值。
const EARTH_RADIUS_KM: f64 = 6370.0;
/// 光速（m/s）。
const C: f64 = 299_792_458.0;
/// 自由空间波阻抗（Ω）。
const ETA: f64 = 119.916_983_2 * std::f64::consts::PI;
/// 参考天线的增益（dBi）：架在理想导体地上的短垂直单极子。
const ANTENNA_GAIN_DBI: f64 = 4.77;

/// 极化。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarization {
  /// 水平极化。
  Horizontal,
  /// 垂直极化。
  Vertical,
}

/// 求解方法（原代码的 `method` 输出，页面据此说明「这个数是怎么来的」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
  /// 平地近似 + 曲率修正。
  FlatEarthCurve,
  /// 残留级数（绕射区）。
  ResidueSeries,
}

/// 地面常数预设与适用范围最近一次对着 ITU 资料核对的日期（`staleness` 汇总用）。
pub const CHECKED_ON: &str = "2026-11-12";

/// P.368 的曲线所用的典型地面常数（页面的预设）。
///
/// 数值取自 P.368 各图的图题，列表见 ITU-R《Handbook on Ground Wave Propagation》（R-HDB-59, 2014）
/// 表 2「Graphs of ground wave propagation for various values of ground constants」，
/// 湿地 / 中等干燥 / 极干燥三项与 P.527 的典型值一致。`id` 是页面词条的后缀，不是展示文字。
///
/// **平均盐度海水取 εr = 70**（P.368-10 图 2 的图题与 P.527 都是 70；上述手册表 2 写 80，
/// 与推荐书正文不一致，这里跟推荐书）。海水的 εr 对地波几乎没有影响（σ 主导），差别在 0.01 dB 量级。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroundType {
  /// 词条后缀（`sea-average` 等）。
  pub id: &'static str,
  /// 相对介电常数。
  pub epsilon_r: f64,
  /// 电导率（S/m）。
  pub sigma_s_m: f64,
}

/// P.368 图 1–9 的地面常数，按电导率从大到小排。
pub const GROUND_TYPES: [GroundType; 9] = [
  GroundType {
    id: "sea-average",
    epsilon_r: 70.0,
    sigma_s_m: 5.0,
  },
  GroundType {
    id: "sea-low-salinity",
    epsilon_r: 80.0,
    sigma_s_m: 1.0,
  },
  GroundType {
    id: "land-good",
    epsilon_r: 40.0,
    sigma_s_m: 3e-2,
  },
  GroundType {
    id: "wet",
    epsilon_r: 30.0,
    sigma_s_m: 1e-2,
  },
  GroundType {
    id: "fresh-water",
    epsilon_r: 80.0,
    sigma_s_m: 3e-3,
  },
  GroundType {
    id: "land-fair",
    epsilon_r: 22.0,
    sigma_s_m: 3e-3,
  },
  GroundType {
    id: "medium-dry",
    epsilon_r: 15.0,
    sigma_s_m: 1e-3,
  },
  GroundType {
    id: "dry",
    epsilon_r: 7.0,
    sigma_s_m: 3e-4,
  },
  GroundType {
    id: "very-dry",
    epsilon_r: 3.0,
    sigma_s_m: 1e-4,
  },
];

/// 页面「场强随距离」表的距离（km）。
pub const TABLE_DISTANCES_KM: [f64; 10] =
  [1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0];

/// 输入。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroundWaveInput {
  /// 发射天线高度（m，0–50）。
  pub h_tx_m: f64,
  /// 接收天线高度（m，0–50）。
  pub h_rx_m: f64,
  /// 频率（MHz，0.01–30）。
  pub freq_mhz: f64,
  /// 发射功率（W，> 0）：只影响场强与接收功率的绝对值，不影响路径损耗。
  pub power_w: f64,
  /// 地面折射率 Ns（N-units，250–400；温带海平面标准大气约 315）。
  pub surface_refractivity: f64,
  /// 路径长度（km，0.001–10 000）。
  pub distance_km: f64,
  /// 地面相对介电常数（≥ 1）。
  pub epsilon_r: f64,
  /// 地面电导率（S/m，> 0）。
  pub sigma_s_m: f64,
  /// 极化。
  pub polarization: Polarization,
}

/// 结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroundWaveResult {
  /// 基本传输损耗（dB）：`L = Pt / Pr`，与功率、增益无关。
  pub basic_loss_db: f64,
  /// 场强（dB(µV/m)）。
  pub field_dbu_vm: f64,
  /// 接收功率（dBm，按 4.77 dBi 的参考天线）。
  pub rx_power_dbm: f64,
  /// 用的是哪条解法。
  pub method: Method,
}

/// 场强低于它（dB(µV/m)）就已远在任何实际噪声底之下。
///
/// 噪声场强 `E_n = Fa + 20·lg f(MHz) + 10·lg B − 95.5`（P.372 的换算）：HF 上最安静的银河噪声
/// （10–30 MHz，Fa ≈ 18–29 dB）在 10 kHz 带宽里约 −8…−6 dB(µV/m)，把带宽收窄到 50 Hz 也只到约 −30。
/// 取 −40 是「再低就不是弱信号，而是数学上的结果」的分界；原件在 1000 km 以上、水平极化
/// 这类条件下会给出 −500 dB(µV/m) 这样的读数，页面据此提示，免得读成「微弱但可收」。
pub const BELOW_NOISE_FLOOR_DBU_VM: f64 = -40.0;

impl GroundWaveResult {
  /// 场强是否低到不可能被任何接收机收到（见 [`BELOW_NOISE_FLOOR_DBU_VM`]）。
  #[must_use]
  pub fn is_below_any_noise_floor(&self) -> bool {
    self.field_dbu_vm < BELOW_NOISE_FLOOR_DBU_VM
  }
}

impl GroundWaveInput {
  /// 按原件的规则校验：任一项越界返回 `None`（原件返回 1000–1008 的错误码，这里不需要分那么细，
  /// 页面把每一项的合法区间写在输入框旁边）。
  #[must_use]
  pub fn validate(&self) -> bool {
    let ok = |v: f64, lo: f64, hi: f64| v.is_finite() && v >= lo && v <= hi;
    ok(self.h_tx_m, 0.0, 50.0)
      && ok(self.h_rx_m, 0.0, 50.0)
      && ok(self.freq_mhz, 0.01, 30.0)
      && self.power_w.is_finite()
      && self.power_w > 0.0
      && ok(self.surface_refractivity, 250.0, 400.0)
      && ok(self.distance_km, 0.001, 10_000.0)
      && self.epsilon_r.is_finite()
      && self.epsilon_r >= 1.0
      && self.sigma_s_m.is_finite()
      && self.sigma_s_m > 0.0
  }
}

/// 求解一次。输入越界返回 `None`。
#[must_use]
pub fn solve(input: &GroundWaveInput) -> Option<GroundWaveResult> {
  if !input.validate() {
    return None;
  }
  let j = Cx::J;
  let f_hz = input.freq_mhz * 1e6;
  let lambda_m = C / f_hz;

  let (h_low_km, h_high_km) = if input.h_tx_m <= input.h_rx_m {
    (input.h_tx_m / 1000.0, input.h_rx_m / 1000.0)
  } else {
    (input.h_rx_m / 1000.0, input.h_tx_m / 1000.0)
  };

  // 等效地球半径（4/3 地球的折射率修正，原件式）。
  let a_e_km = EARTH_RADIUS_KM / (1.0 - 0.046_65 * (0.005_577 * input.surface_refractivity).exp());
  let theta_rad = input.distance_km / a_e_km;
  let k = 2.0 * std::f64::consts::PI / (lambda_m / 1000.0); // rad/km
  let nu = (a_e_km * k / 2.0).powf(1.0 / 3.0);

  // 地表面阻抗（DeMinco 99-368 式 15/17）。
  let eta = Cx::new(
    input.epsilon_r,
    -input.sigma_s_m / (EPSILON_0 * 2.0 * std::f64::consts::PI * f_hz),
  );
  let mut delta = (eta - Cx::ONE).sqrt();
  if input.polarization == Polarization::Vertical {
    delta = delta / eta;
  }
  let q = -(j * nu * delta);

  let (e_gw, method) = if input.distance_km < 80.0 * input.freq_mhz.powf(-1.0 / 3.0) {
    (
      flat_earth_curve_correction(delta, q, h_low_km, h_high_km, input.distance_km, k, a_e_km),
      Method::FlatEarthCurve,
    )
  } else {
    (
      residue_series(k, h_low_km, h_high_km, nu, theta_rad, q),
      Method::ResidueSeries,
    )
  };
  if !e_gw.is_finite() {
    return None;
  }

  // 反归一化：参考天线的增益与发射功率。
  let g_tx = 10f64.powf(ANTENNA_GAIN_DBI / 10.0);
  let e_0 =
    (ETA * (input.power_w * g_tx) / (4.0 * std::f64::consts::PI)).sqrt() / input.distance_km;
  let e_gw = e_gw * e_0;

  let basic_loss_db = 10.0 * (input.power_w * g_tx).log10()
    + 10.0 * (ETA * 4.0 * std::f64::consts::PI).log10()
    + 20.0 * f_hz.log10()
    - 20.0 * (e_gw / 1000.0).log10()
    - 20.0 * C.log10();
  // 60 = 20·lg(10³)：mV/m → µV/m。
  let field_dbu_vm = 60.0 + 20.0 * e_gw.log10();
  // 功率与频率相关；42.8 是 MHz→Hz、dBm 与其余常数的合并项（原件的注释）。
  let rx_power_dbm = field_dbu_vm + ANTENNA_GAIN_DBI - 20.0 * f_hz.log10() + 42.8;

  (basic_loss_db.is_finite() && field_dbu_vm.is_finite() && rx_power_dbm.is_finite()).then_some(
    GroundWaveResult {
      basic_loss_db,
      field_dbu_vm,
      rx_power_dbm,
      method,
    },
  )
}

/// 平地近似 + 曲率修正（NTIA 99-368 式 31/32、DeMinco 式 28/30）。
fn flat_earth_curve_correction(
  delta: Cx,
  q: Cx,
  h_low_km: f64,
  h_high_km: f64,
  d_km: f64,
  k: f64,
  a_e_km: f64,
) -> f64 {
  let j = Cx::J;
  let sqrt_pi = std::f64::consts::PI.sqrt();
  let qi = Cx::new(-0.5, 0.5) * (k * d_km).sqrt() * delta;
  let p = qi * qi;
  let p2 = p * p;
  let q3 = powi(q, 3);
  let q6 = powi(q, 6);
  let q9 = powi(q, 9);

  let fofx = if q.abs() > 0.1 {
    let f_of_p = Cx::ONE + (j * sqrt_pi * qi) * wofz::wofz(qi);
    let t1 = f_of_p
      + (Cx::ONE - j * (std::f64::consts::PI * p).sqrt() - (Cx::ONE + p * 2.0) * f_of_p)
        * (Cx::ONE / (4.0 * q3));
    t1 + (Cx::ONE - j * (std::f64::consts::PI * p).sqrt() * (Cx::ONE - p) - p * 2.0
      + p2 * (5.0 / 6.0)
      + (p2 * 0.5 - Cx::ONE) * f_of_p)
      * (Cx::ONE / (4.0 * q6))
  } else {
    // 小 q 的级数（DeMinco 式 30）：A[0..10]，每一项都带 q⁻³ 的修正。
    let a = [
      Cx::ONE,
      -(j * sqrt_pi),
      Cx::of(-2.0),
      j * sqrt_pi * (Cx::ONE + Cx::ONE / (4.0 * q3)),
      Cx::of(4.0 / 3.0) * (Cx::ONE + Cx::ONE / (2.0 * q3)),
      -(j * sqrt_pi) * 0.25 * (Cx::ONE + Cx::of(3.0) / (4.0 * q3)),
      Cx::of(-8.0 / 15.0) * (Cx::ONE + Cx::ONE / q3 + Cx::of(7.0) / (32.0 * q6)),
      j * (sqrt_pi / 6.0) * (Cx::ONE + Cx::of(5.0) / (4.0 * q3) + Cx::of(27.0) / (32.0 * q6)),
      Cx::of(16.0 / 105.0) * (Cx::ONE + Cx::of(3.0) / (2.0 * q3) + Cx::of(27.0) / (32.0 * q6)),
      -(j * (sqrt_pi / 24.0))
        * (Cx::ONE
          + Cx::of(7.0) / (4.0 * q3)
          + Cx::of(5.0) / (4.0 * q6)
          + Cx::of(21.0) / (64.0 * q9)),
    ];
    let x = d_km / a_e_km * (k * a_e_km / 2.0).powf(1.0 / 3.0);
    let base = Cx::new_polar(std::f64::consts::PI / 4.0) * q * Cx::of(x.sqrt());
    let mut sum = Cx::ZERO;
    let mut pow = Cx::ONE;
    for coef in a {
      sum += coef * pow;
      pow *= base;
    }
    sum
  };

  // 两端的高度增益各取泰勒级数的两项（DeMinco 式 36）。
  let hg = (Cx::ONE + j * (k * h_high_km) * delta) * (Cx::ONE + j * (k * h_low_km) * delta);
  (fofx * hg).abs()
}

/// 残留级数（Wait / Fock）：第 i 项用 Airy 三类的根 `Wi'(t) − q·Wi(t) = 0`（NTIA 99-368 式 26）。
fn residue_series(k: f64, h_low_km: f64, h_high_km: f64, nu: f64, theta_rad: f64, q: Cx) -> f64 {
  let y_high = k * h_high_km / nu;
  let y_low = k * h_low_km / nu;
  let x = nu * theta_rad;
  let mut gw = Cx::ZERO;
  for i in 0..200 {
    let (t, _dwi, _wi) = airy::wi_root(i + 1, q);
    let w1 = airy::airy(t, airy::Kind::WOne, airy::Scaling::Wait);

    let mut w = if h_low_km > 0.0 {
      let mut v = airy::airy(t - Cx::of(y_low), airy::Kind::WOne, airy::Scaling::Wait) / w1;
      if h_high_km > 0.0 {
        v = v * airy::airy(t - Cx::of(y_high), airy::Kind::WOne, airy::Scaling::Wait) / w1;
      }
      v
    } else if h_high_km > 0.0 {
      airy::airy(t - Cx::of(y_high), airy::Kind::WOne, airy::Scaling::Wait) / w1
    } else {
      Cx::ONE
    };
    w = w / (t - q * q);
    let g = w * (-(Cx::J) * x * t).exp();
    gw += g;
    if i != 0 {
      let g2 = gw * gw;
      if g2.re.abs() + g2.im.abs() == 0.0 {
        return 0.0;
      }
      let ratio = g / gw;
      if ratio.re.abs() + ratio.im.abs() < 0.000_5 {
        break;
      }
    }
  }

  // √(π/2)·(1 − j)：复数常数提出来就是幅度。
  let e_w = x.sqrt()
    * Cx::new(
      (std::f64::consts::PI / 2.0).sqrt(),
      -(std::f64::consts::PI / 2.0).sqrt(),
    )
    * gw;
  e_w.abs()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn input(
    h_tx_m: f64,
    h_rx_m: f64,
    freq_mhz: f64,
    distance_km: f64,
    epsilon_r: f64,
    sigma_s_m: f64,
    polarization: Polarization,
  ) -> GroundWaveInput {
    GroundWaveInput {
      h_tx_m,
      h_rx_m,
      freq_mhz,
      power_w: 100.0,
      surface_refractivity: 301.0,
      distance_km,
      epsilon_r,
      sigma_s_m,
      polarization,
    }
  }

  /// 与 NTIA 原件（P.368-10 ZIP 里的 `LFMF Source`，v1.1）逐点一致的几个锚点。
  ///
  /// 参照值由 `crates/core/reference/lfmf_check.py` 生成：那个脚本下载原件、用 clang 编成命令行
  /// 程序，再把同一批输入同时喂给两边。全网格 4410 组加边界 48 组的最大差是 4.8e-8 dB（主网格）
  /// 与 3.9e-5 dB（0.01 MHz + εr = 1 + σ = 1e-6 的角点，复平方根的末位差异）。这里只钉 6 个
  /// 代表点，避免把 4410 个常数塞进源码；两条解法、两种极化、两端高度都为 0 与都不为 0 都覆盖。
  #[test]
  fn agrees_with_the_ntia_reference_implementation() {
    // (输入, 场强 dB(µV/m), 基本传输损耗 dB, 方法)
    let cases: [(GroundWaveInput, f64, f64, Method); 6] = [
      (
        input(10.0, 10.0, 1.0, 100.0, 15.0, 0.005, Polarization::Vertical),
        32.291_970_109_8,
        99.694_020_141_4,
        Method::ResidueSeries,
      ),
      (
        input(0.0, 0.0, 0.2, 50.0, 15.0, 0.005, Polarization::Vertical),
        64.422_168_135_8,
        53.584_422_028_6,
        Method::FlatEarthCurve,
      ),
      (
        input(10.0, 2.0, 7.0, 300.0, 15.0, 0.005, Polarization::Vertical),
        -43.742_534_373_4,
        192.630_485_424_8,
        Method::ResidueSeries,
      ),
      (
        input(
          0.0,
          0.0,
          14.0,
          2000.0,
          15.0,
          0.005,
          Polarization::Horizontal,
        ),
        -497.662_428_789_1,
        652.570_979_753_9,
        Method::ResidueSeries,
      ),
      (
        input(30.0, 50.0, 3.5, 800.0, 4.0, 0.001, Polarization::Horizontal),
        -111.243_067_598_6,
        254.110_418_736_7,
        Method::ResidueSeries,
      ),
      (
        input(50.0, 0.0, 21.0, 1000.0, 80.0, 4.0, Polarization::Vertical),
        -156.326_565_950_4,
        314.756_942_096_2,
        Method::ResidueSeries,
      ),
    ];
    for (i, (input, e_want, a_want, method)) in cases.into_iter().enumerate() {
      let got = solve(&input).unwrap_or_else(|| panic!("第 {i} 组应能求解"));
      assert_eq!(got.method, method, "第 {i} 组的解法");
      assert!(
        (got.field_dbu_vm - e_want).abs() < 1e-6,
        "第 {i} 组场强：{} vs {e_want}",
        got.field_dbu_vm
      );
      assert!(
        (got.basic_loss_db - a_want).abs() < 1e-6,
        "第 {i} 组损耗：{} vs {a_want}",
        got.basic_loss_db
      );
    }
  }

  /// 场强与损耗的关系：损耗不依赖功率，场强按 10·lg(P₂/P₁) 平移。
  #[test]
  fn power_shifts_the_field_but_not_the_loss() {
    let base = input(10.0, 10.0, 1.0, 100.0, 15.0, 0.005, Polarization::Vertical);
    let a = solve(&base).unwrap();
    let mut louder = base;
    louder.power_w = 1000.0;
    let b = solve(&louder).unwrap();
    assert!(
      (a.basic_loss_db - b.basic_loss_db).abs() < 1e-9,
      "损耗不该随功率变"
    );
    assert!(
      (b.field_dbu_vm - a.field_dbu_vm - 10.0).abs() < 1e-9,
      "功率 ×10 场强应 +10 dB：{} vs {}",
      a.field_dbu_vm,
      b.field_dbu_vm
    );
  }

  /// 场强随距离单调下降（同一套地面参数、同一天线高度），且两条解法的接缝处不跳变。
  #[test]
  fn field_falls_off_with_distance_without_a_step_at_the_seam() {
    let mut prev = f64::INFINITY;
    for d in [1.0, 10.0, 50.0, 79.0, 81.0, 120.0, 500.0, 2000.0] {
      let r = solve(&input(
        10.0,
        10.0,
        1.0,
        d,
        15.0,
        0.005,
        Polarization::Vertical,
      ))
      .unwrap();
      assert!(
        r.field_dbu_vm < prev,
        "{d} km: {} 没有比上一点更弱",
        r.field_dbu_vm
      );
      prev = r.field_dbu_vm;
    }
    // 接缝两侧（d = 80·f^(−1/3) ≈ 80 km）相差不到 1 dB。
    let lo = solve(&input(
      10.0,
      10.0,
      1.0,
      79.9,
      15.0,
      0.005,
      Polarization::Vertical,
    ))
    .unwrap();
    let hi = solve(&input(
      10.0,
      10.0,
      1.0,
      80.1,
      15.0,
      0.005,
      Polarization::Vertical,
    ))
    .unwrap();
    assert_eq!(lo.method, Method::FlatEarthCurve);
    assert_eq!(hi.method, Method::ResidueSeries);
    assert!(
      (lo.field_dbu_vm - hi.field_dbu_vm).abs() < 1.0,
      "接缝跳变 {:.3} dB",
      (lo.field_dbu_vm - hi.field_dbu_vm).abs()
    );
  }

  /// 地面越好（电导率越高），同一路径的场强越强。
  #[test]
  fn better_ground_gives_a_stronger_field() {
    let poor = solve(&input(
      10.0,
      2.0,
      1.0,
      200.0,
      4.0,
      0.001,
      Polarization::Vertical,
    ))
    .unwrap();
    let good = solve(&input(
      10.0,
      2.0,
      1.0,
      200.0,
      80.0,
      4.0,
      Polarization::Vertical,
    ))
    .unwrap();
    assert!(
      good.field_dbu_vm > poor.field_dbu_vm,
      "{} vs {}",
      good.field_dbu_vm,
      poor.field_dbu_vm
    );
  }

  /// 越界输入按原件的区间拒绝（频率、高度、距离、Ns、εr、σ）。
  #[test]
  fn out_of_range_inputs_are_rejected() {
    let ok = input(10.0, 10.0, 1.0, 100.0, 15.0, 0.005, Polarization::Vertical);
    assert!(solve(&ok).is_some());
    let mutations: [fn(&mut GroundWaveInput); 10] = [
      |i| i.h_tx_m = -0.1,
      |i| i.h_tx_m = 50.1,
      |i| i.h_rx_m = 60.0,
      |i| i.freq_mhz = 0.009,
      |i| i.freq_mhz = 30.1,
      |i| i.power_w = 0.0,
      |i| i.surface_refractivity = 249.0,
      |i| i.distance_km = 0.0009,
      |i| i.distance_km = 10_001.0,
      |i| i.epsilon_r = 0.9,
    ];
    for (k, mutate) in mutations.into_iter().enumerate() {
      let mut bad = ok;
      mutate(&mut bad);
      assert!(solve(&bad).is_none(), "第 {k} 个变异应被拒绝");
    }
    let mut bad = ok;
    bad.sigma_s_m = 0.0;
    assert!(solve(&bad).is_none());
    let mut bad = ok;
    bad.freq_mhz = f64::NAN;
    assert!(solve(&bad).is_none());
  }

  /// 极远 / 水平极化给出的离谱读数要被标出来；正常的弱信号不能误报。
  #[test]
  fn absurdly_weak_fields_are_flagged_as_below_any_noise_floor() {
    let flagged = |i: GroundWaveInput| solve(&i).unwrap().is_below_any_noise_floor();
    // 水平极化、14 MHz、2000 km：−497.66 dB(µV/m)（与 NTIA 原件对拍的那一组）。
    assert!(flagged(input(
      0.0,
      0.0,
      14.0,
      2000.0,
      15.0,
      0.005,
      Polarization::Horizontal
    )));
    // 1 MHz、1000 km、中等干燥地、垂直极化：−63.13 dB(µV/m)。
    assert!(flagged(input(
      0.0,
      0.0,
      1.0,
      1000.0,
      15.0,
      0.001,
      Polarization::Vertical
    )));
    // 1 MHz、100 km 的 29.39 dB(µV/m)、7 MHz 海水 500 km 的 25.15 dB(µV/m)：都是能收到的信号。
    assert!(!flagged(input(
      0.0,
      0.0,
      1.0,
      100.0,
      15.0,
      0.001,
      Polarization::Vertical
    )));
    assert!(!flagged(input(
      0.0,
      0.0,
      7.0,
      500.0,
      70.0,
      5.0,
      Polarization::Vertical
    )));
  }

  /// 预设表里每一项都在求解器的合法范围内，且电导率单调下降（页面按这个顺序列）。
  #[test]
  fn ground_presets_are_valid_and_ordered_by_conductivity() {
    for g in GROUND_TYPES {
      let i = input(
        10.0,
        10.0,
        1.0,
        100.0,
        g.epsilon_r,
        g.sigma_s_m,
        Polarization::Vertical,
      );
      assert!(solve(&i).is_some(), "{} 求解失败", g.id);
    }
    assert!(
      GROUND_TYPES
        .windows(2)
        .all(|w| w[0].sigma_s_m >= w[1].sigma_s_m)
    );
    let mut ids: Vec<_> = GROUND_TYPES.iter().map(|g| g.id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), GROUND_TYPES.len(), "id 不能重复");
  }

  /// 物理上：同一条路径，电导率越高地波损耗越小（海水 < 中等干燥 < 极干燥）。
  /// 防的是把预设的 εr / σ 抄反。
  #[test]
  fn better_ground_carries_the_ground_wave_further() {
    let loss = |id: &str| {
      let g = GROUND_TYPES.iter().find(|g| g.id == id).unwrap();
      let i = input(
        10.0,
        10.0,
        1.0,
        100.0,
        g.epsilon_r,
        g.sigma_s_m,
        Polarization::Vertical,
      );
      solve(&i).unwrap().basic_loss_db
    };
    assert!(loss("sea-average") < loss("medium-dry"));
    assert!(loss("medium-dry") < loss("very-dry"));
    // 同一类地面上，页面距离表里每个距离都得有解（表里不该出现「—」之外的意外空洞）。
    let g = GROUND_TYPES[6];
    for d in TABLE_DISTANCES_KM {
      let i = input(
        10.0,
        10.0,
        1.0,
        d,
        g.epsilon_r,
        g.sigma_s_m,
        Polarization::Vertical,
      );
      assert!(solve(&i).is_some(), "{d} km");
    }
  }
}
