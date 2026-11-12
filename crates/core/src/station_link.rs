//! 台站链路预算：把「发射机 → 馈线 → 天线 → 路径 → 天线 → 馈线 → 接收机 → 噪声 → 所需 SNR」
//! 串成一条链，输出余量、可达距离，以及**哪个环节最值得先动**。
//!
//! 站内已有的 `link_budget`（只算 FSPL 与到达功率）与 `/tools` 里各自孤立的馈线损耗、
//! 接收机灵敏度计算器，都只回答链上的一小段；这里回答整条链的问题：**这条链路够不够、差多少、
//! 先改哪里**。
//!
//! # 参考面与公式
//!
//! 信号与噪声都**折算到接收天线端子**（在接收馈线之前）：
//!
//! ```text
//! P_ant = P_tx − L_tx线 + G_tx − (FSPL + L_路径附加) − L_附加 + G_rx            [dBm]
//! f     = fa − 1 + L · Fr                                                     [线性]
//! N     = −174 + 10·lg(B) + 10·lg(f)                                          [dBm]
//! SNR   = P_ant − N,    余量 = SNR − SNR_所需
//! ```
//!
//! 其中 `fa = 10^(Fa/10)` 是外部噪声（天线端子处，相对 `kT0`），`L` 是接收馈线损耗、`Fr`
//! 是接收机噪声系数（线性）。`f = fa − 1 + L·Fr` 是 ITU-R P.372 的运行噪声系数公式在
//! 「天线无损、馈线在 `T0`」下的形式；本模块**自己推导并用逐级功率累加独立验证**（见测试），
//! 不依赖该文件里的任何数值表。`Fa` 的取值由调用方给：页面可以让用户自填，也可以按 P.372 的
//! 环境曲线随频率取（[`crate::noise_model::fam_db`]，只含人为 + 银河噪声，范围见该模块）。
//!
//! 两个直接的推论，也是这个模块存在的理由：
//!
//! - `f − 1 = (fa − 1) + (L·Fr − 1)`：超出热噪声的部分**按外部与接收链严格相加**，所以
//!   「噪声主要来自外部还是接收链」有精确的占比（[`LinkResult::ext_share`]），不需要设阈值；
//! - 外部噪声很大时（`fa ≫ L·Fr`），接收馈线损耗与接收机噪声系数几乎不影响 SNR —— 信号与噪声
//!   一起被衰减。这正是 HF 上「馈线损耗无所谓」的物理原因，而在只有热噪声的 VHF / UHF 上恰好相反。
//!
//! # 「最先动哪里」的定义
//!
//! 对五个有损或有噪声的环节，各自**单独换成理想**（损耗 0、NF 0、`Fa` 0）后重算余量，
//! 多得的 dB 就是该环节的「回收潜力」，降序排列。它是**上限**而不是建议：各环节不可加（噪声级联
//! 非线性），而且「换成理想」做不到。排第一的就是单点改善收益最大的那一个。
//! 功率、天线增益、带宽、距离不在其中 —— 它们没有「理想值」，对余量就是一比一，
//! 见 [`LinkResult::max_dist_km`] 与 [`LinkResult::required_tx_w`]。
//!
//! # 口径与不建模
//!
//! - **路径只有自由空间**（`link_budget::free_space_path_loss_db`）加用户给的附加损耗。HF 天波、
//!   绕射、多径、地面反射都不在内；`max_dist_km` 也只是「FSPL 这一项用完余量」的距离，
//!   不管地平线与遮挡；
//! - **外部噪声 `Fa` 由调用方给**。[`crate::noise_model`] 有 P.372 的人为 / 银河噪声中位数，
//!   但**没有大气噪声（雷电）**，HF 上那一项可与人为噪声相当，所以它不是 HF 的总外部噪声；
//!   `Fa = 0` 表示只有热噪声，对 HF / 嘈杂环境**偏乐观**；
//! - `Fa ≥ 0`：冷天空（卫星 / EME，天线噪声温度低于 290 K）不在内；
//! - 噪声热底取 `−174 dBm/Hz`（290 K 取整，与站内接收机灵敏度计算器同一常数）；
//! - 所需 SNR 必须是**在输入的带宽内**量得的；本模块不内置各模式的门限；
//! - 发射机馈线只影响信号，不影响噪声；失配（驻波）损耗请自行并入「附加损耗」；
//! - **近场**：自由空间公式要求收发两端在彼此的远场。距离不到 [`FAR_FIELD_WAVELENGTHS`] 个波长时
//!   [`LinkResult::near_field`] 为真，页面提示结果仅供参考 —— 再近下去 FSPL 会变成负数（信号比 EIRP
//!   还大），那是公式失效，不是链路真有增益。10 个波长是常用的经验口径；天线本身很大（抛物面、
//!   长 Yagi）时远场更远，这里不按天线尺寸算。
//!
//! # 验证
//!
//! Python 独立原型（不共享任何公式；噪声用逐级功率累加，与闭式相对误差 4e-16）给出全部锚点；
//! 另有往返（最大距离 / 所需功率代回应得余量 0）、回收潜力与闭式逐位一致、与 `link_budget`
//! 的到达功率交叉核对、一比一响应与非法输入。

use crate::decibel::format_mw;
use crate::feedline::{FEEDLINE_SPECS, feedline_loss_db};
use crate::fresnel::wavelength_m;
use crate::link_budget::free_space_path_loss_db;
use crate::unit_math::format_distance_m;

/// 噪声热底（dBm/Hz）：`10·lg(k·290 K)` 取整，与 `/tools` 接收机灵敏度计算器一致。
pub const THERMAL_DBM_PER_HZ: f64 = -174.0;

/// 自由空间公式适用的最小距离，以波长计。
pub const FAR_FIELD_WAVELENGTHS: f64 = 10.0;

/// 链路输入。除增益与所需 SNR 外，损耗与噪声项都不能为负。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinkInput {
  /// 频率（MHz，> 0）。
  pub freq_mhz: f64,
  /// 距离（km，> 0）。
  pub dist_km: f64,
  /// 路径附加损耗（dB，≥ 0）：大气 / 雨衰 / 绕射 / 植被等，由用户另行估算后填入。
  pub path_extra_db: f64,
  /// 发射功率（W，> 0）。
  pub tx_power_w: f64,
  /// 发射馈线损耗（dB，≥ 0）。
  pub tx_line_db: f64,
  /// 发射天线增益（dBi）。
  pub tx_gain_dbi: f64,
  /// 接收天线增益（dBi）。
  pub rx_gain_dbi: f64,
  /// 接收馈线损耗（dB，≥ 0）。
  pub rx_line_db: f64,
  /// 附加损耗（dB，≥ 0）：失配、极化、接头、天调等。
  pub misc_loss_db: f64,
  /// 接收机噪声系数 NF（dB，≥ 0）。
  pub rx_nf_db: f64,
  /// 外部噪声 `Fa`（dB，≥ 0，相对 `kT0`；0 = 只有热噪声）。
  pub ext_noise_db: f64,
  /// 噪声带宽（Hz，> 0）。
  pub bandwidth_hz: f64,
  /// 所需 SNR（dB，在上面的带宽内）。
  pub required_snr_db: f64,
}

/// 可被「换成理想」的环节。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
  /// 发射馈线损耗。
  TxLine,
  /// 接收馈线损耗。
  RxLine,
  /// 附加损耗。
  Misc,
  /// 接收机噪声系数。
  RxNoiseFigure,
  /// 外部噪声。
  ExternalNoise,
}

/// 某环节的回收潜力。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reclaim {
  /// 环节。
  pub stage: Stage,
  /// 该环节单独换成理想后，余量多出的 dB（≥ 0）。
  pub gain_db: f64,
}

/// 链路结果。
#[derive(Debug, Clone, PartialEq)]
pub struct LinkResult {
  /// 路径总损耗（dB）：自由空间 + 附加。
  pub path_loss_db: f64,
  /// 等效全向辐射功率（dBm）：发射功率 − 发射馈线 + 发射天线增益。
  pub eirp_dbm: f64,
  /// 接收天线端子处的信号功率（dBm）。
  pub signal_dbm: f64,
  /// 折算到接收天线端子的总噪声功率（dBm）。
  pub noise_dbm: f64,
  /// 运行噪声系数 `10·lg(f)`（dB）。
  pub noise_factor_db: f64,
  /// 外部噪声占「超出热噪声部分」的比例 `(fa−1)/(f−1)`，0..=1。
  /// 没有任何超出热噪声的部分（`f = 1`）时为 `None`。
  pub ext_share: Option<f64>,
  /// 接收天线端子处的 SNR（dB）。
  pub snr_db: f64,
  /// 余量（dB）= SNR − 所需 SNR。负值表示不够。
  pub margin_db: f64,
  /// 只动距离、其余不变时，余量恰好为 0 的距离（km）。自由空间口径，不管地平线。
  pub max_dist_km: f64,
  /// 只动发射功率、其余不变时，余量恰好为 0 的发射功率（W）。
  pub required_tx_w: f64,
  /// 自由空间公式可信的最小距离（km）：[`FAR_FIELD_WAVELENGTHS`] 个波长。
  pub far_field_km: f64,
  /// 输入距离是否小于 [`far_field_km`](Self::far_field_km)（落入近场，结果仅供参考）。
  pub near_field: bool,
  /// 五个环节的回收潜力，按 `gain_db` 降序（并列保持声明顺序）。
  pub reclaim: [Reclaim; 5],
}

/// 一次前向计算的中间量。
struct Budget {
  path_loss_db: f64,
  eirp_dbm: f64,
  signal_dbm: f64,
  noise_dbm: f64,
  noise_factor_db: f64,
  ext_share: Option<f64>,
  snr_db: f64,
}

fn db_to_lin(db: f64) -> f64 {
  10f64.powf(db / 10.0)
}

fn lin_to_db(x: f64) -> f64 {
  10.0 * x.log10()
}

fn valid(i: &LinkInput) -> bool {
  let all = [
    i.freq_mhz,
    i.dist_km,
    i.path_extra_db,
    i.tx_power_w,
    i.tx_line_db,
    i.tx_gain_dbi,
    i.rx_gain_dbi,
    i.rx_line_db,
    i.misc_loss_db,
    i.rx_nf_db,
    i.ext_noise_db,
    i.bandwidth_hz,
    i.required_snr_db,
  ];
  all.iter().all(|v| v.is_finite())
    && i.freq_mhz > 0.0
    && i.dist_km > 0.0
    && i.tx_power_w > 0.0
    && i.bandwidth_hz > 0.0
    && [
      i.path_extra_db,
      i.tx_line_db,
      i.rx_line_db,
      i.misc_loss_db,
      i.rx_nf_db,
      i.ext_noise_db,
    ]
    .iter()
    .all(|v| *v >= 0.0)
}

/// 前向计算。输入非法或任一结果非有限时返回 `None`。
fn budget(i: &LinkInput) -> Option<Budget> {
  if !valid(i) {
    return None;
  }
  let fspl = free_space_path_loss_db(i.freq_mhz, i.dist_km)?;
  let path_loss_db = fspl + i.path_extra_db;
  let eirp_dbm = lin_to_db(i.tx_power_w * 1000.0) - i.tx_line_db + i.tx_gain_dbi;
  let signal_dbm = eirp_dbm - path_loss_db - i.misc_loss_db + i.rx_gain_dbi;

  let fa = db_to_lin(i.ext_noise_db);
  let chain = db_to_lin(i.rx_line_db) * db_to_lin(i.rx_nf_db);
  let f = fa - 1.0 + chain;
  let noise_factor_db = lin_to_db(f);
  let noise_dbm = THERMAL_DBM_PER_HZ + lin_to_db(i.bandwidth_hz) + noise_factor_db;
  // `f − 1` 是两项非负量之和：为 0 当且仅当 Fa = 0、L = 1、Fr = 1，此时谈占比没有意义。
  let excess = f - 1.0;
  let ext_share = (excess > 1e-12).then(|| ((fa - 1.0) / excess).clamp(0.0, 1.0));

  let b = Budget {
    path_loss_db,
    eirp_dbm,
    signal_dbm,
    noise_dbm,
    noise_factor_db,
    ext_share,
    snr_db: signal_dbm - noise_dbm,
  };
  [
    b.path_loss_db,
    b.eirp_dbm,
    b.signal_dbm,
    b.noise_dbm,
    b.noise_factor_db,
    b.snr_db,
  ]
  .iter()
  .all(|v| v.is_finite())
  .then_some(b)
}

/// 评估整条链路。输入非法（含任何非有限值、负损耗、非正频率 / 距离 / 功率 / 带宽）返回 `None`。
#[must_use]
pub fn evaluate(i: &LinkInput) -> Option<LinkResult> {
  let b = budget(i)?;
  let margin_db = b.snr_db - i.required_snr_db;

  // 每个环节单独换成理想后重算：与主路径共用同一个 `budget`，口径不会漂。
  let gain = |stage: Stage, ideal: LinkInput| -> Option<Reclaim> {
    let m = budget(&ideal)?.snr_db - i.required_snr_db;
    Some(Reclaim {
      stage,
      // 浮点舍入可能给出 −1e-15：理想只会更好，夹到 0。
      gain_db: (m - margin_db).max(0.0),
    })
  };
  let mut reclaim = [
    gain(
      Stage::TxLine,
      LinkInput {
        tx_line_db: 0.0,
        ..*i
      },
    )?,
    gain(
      Stage::RxLine,
      LinkInput {
        rx_line_db: 0.0,
        ..*i
      },
    )?,
    gain(
      Stage::Misc,
      LinkInput {
        misc_loss_db: 0.0,
        ..*i
      },
    )?,
    gain(
      Stage::RxNoiseFigure,
      LinkInput {
        rx_nf_db: 0.0,
        ..*i
      },
    )?,
    gain(
      Stage::ExternalNoise,
      LinkInput {
        ext_noise_db: 0.0,
        ..*i
      },
    )?,
  ];
  // 稳定排序：并列时保持声明顺序，输出才可复现。
  reclaim.sort_by(|a, c| c.gain_db.total_cmp(&a.gain_db));

  let max_dist_km = i.dist_km * 10f64.powf(margin_db / 20.0);
  let required_tx_w = i.tx_power_w * 10f64.powf(-margin_db / 10.0);
  // `budget` 已确认频率为正，波长必有值；取不到就当输入非法，不给半截结果。
  let far_field_km = wavelength_m(i.freq_mhz)? * FAR_FIELD_WAVELENGTHS / 1000.0;
  (max_dist_km.is_finite() && required_tx_w.is_finite() && far_field_km.is_finite()).then_some(
    LinkResult {
      path_loss_db: b.path_loss_db,
      eirp_dbm: b.eirp_dbm,
      signal_dbm: b.signal_dbm,
      noise_dbm: b.noise_dbm,
      noise_factor_db: b.noise_factor_db,
      ext_share: b.ext_share,
      snr_db: b.snr_db,
      margin_db,
      max_dist_km,
      required_tx_w,
      far_field_km,
      near_field: i.dist_km < far_field_km,
      reclaim,
    },
  )
}

/// 所选馈线型号（[`FEEDLINE_SPECS`] 的下标）与长度在该频率下的损耗（dB）。
///
/// 下标越界、长度为负 / 非有限、频率非正都是输入错误，返回 `None`。必须在这里拦下：
/// `feedline_loss_db` 对 `length_m <= 0` 或 `freq_mhz <= 0` 直接返回 0，一个手滑的 `-5 m`
/// 会被悄悄当成「没有馈线」，算出一个偏乐观的余量。长度恰为 0 是合法的（没有这段馈线）。
#[must_use]
pub fn cable_loss_db(spec_index: usize, length_m: f64, freq_mhz: f64) -> Option<f64> {
  if !(length_m.is_finite() && length_m >= 0.0 && freq_mhz.is_finite() && freq_mhz > 0.0) {
    return None;
  }
  let spec = FEEDLINE_SPECS.get(spec_index)?;
  Some(feedline_loss_db(spec, freq_mhz, length_m))
}

/// 距离文案（入参为 km）。
///
/// 量级在 1 mm ~ 1 万 km 之间复用 [`format_distance_m`]（取整顶到上一档时整体升档）；
/// 再往外只给量级：自由空间外推出来的最大距离早已超出任何真实链路，保留小数是假精度 ——
/// ≥ 1 万 km 只留整数，≥ 1000 万 km（比地月距离还大一个量级）与 < 1 mm 用科学计数。
/// 非法输入给 `—`。
#[must_use]
pub fn format_reach_km(km: f64) -> String {
  if !km.is_finite() || km < 0.0 {
    "—".to_owned()
  } else if km >= 1e7 || (km > 0.0 && km < 1e-6) {
    format!("{km:.2e} km")
  } else if km >= 1e4 {
    format!("{km:.0} km")
  } else {
    format_distance_m(km * 1000.0)
  }
}

/// 功率文案（入参为 W）。
///
/// 量级在 1 nW ~ 100 万 W 之间复用 [`format_mw`]（单位按取整后的数值判，不会写出
/// `1000.00 mW`）；两端之外用科学计数，免得把 `1e-14 W` 排成 `0.000 μW`。非法输入给 `—`。
#[must_use]
pub fn format_power_w(w: f64) -> String {
  if !w.is_finite() || w < 0.0 {
    "—".to_owned()
  } else if w >= 1e6 || (w > 0.0 && w < 1e-9) {
    format!("{w:.2e} W")
  } else {
    format_mw(w * 1000.0)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::link_budget::received_dbm;

  /// 与 Python 原型 `BASE` 同一组：2 m、100 km、10 W。
  fn base() -> LinkInput {
    LinkInput {
      freq_mhz: 145.0,
      dist_km: 100.0,
      path_extra_db: 0.0,
      tx_power_w: 10.0,
      tx_line_db: 1.5,
      tx_gain_dbi: 3.0,
      rx_gain_dbi: 3.0,
      rx_line_db: 2.0,
      misc_loss_db: 0.5,
      rx_nf_db: 1.0,
      ext_noise_db: 0.0,
      bandwidth_hz: 2400.0,
      required_snr_db: 10.0,
    }
  }

  /// 与 Python 原型 `HF` 同一组：外部噪声 40 dB。路径仍按自由空间算 —— 这里只拿它验证噪声级联，
  /// 不是说 3.7 MHz 的天波是自由空间。
  fn noisy() -> LinkInput {
    LinkInput {
      freq_mhz: 3.7,
      dist_km: 300.0,
      tx_power_w: 100.0,
      tx_line_db: 1.0,
      tx_gain_dbi: 0.0,
      rx_gain_dbi: 0.0,
      rx_line_db: 3.0,
      misc_loss_db: 0.0,
      rx_nf_db: 8.0,
      ext_noise_db: 40.0,
      bandwidth_hz: 2500.0,
      required_snr_db: -21.0,
      ..base()
    }
  }

  fn near(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * b.abs().max(1.0)
  }

  fn gain_of(r: &LinkResult, stage: Stage) -> f64 {
    r.reclaim
      .iter()
      .find(|x| x.stage == stage)
      .expect("五个环节都在")
      .gain_db
  }

  #[test]
  fn matches_independent_python_anchors() {
    let r = evaluate(&base()).expect("合法");
    assert!(near(r.path_loss_db, 115.677_360_044_699_5, 1e-12));
    assert!(near(r.signal_dbm, -71.677_360_044_699_5, 1e-12));
    // 2 dB 馈线 + 1 dB NF，Fa = 0（fa = 1）：f = L·Fr，正好 3 dB。
    assert!(near(r.noise_factor_db, 3.0, 1e-12));
    assert!(near(r.noise_dbm, -137.197_887_582_883_94, 1e-12));
    assert!(near(r.snr_db, 65.520_527_538_184_44, 1e-12));
    assert!(near(r.margin_db, 55.520_527_538_184_44, 1e-12));
    assert!(near(r.max_dist_km, 59_707.154_866_969_97, 1e-10));
    assert!(near(r.required_tx_w, 2.805_092_882_186_458_6e-5, 1e-10));
    // 10 W = 40 dBm；EIRP = 40 − 1.5 + 3。
    assert!(near(r.eirp_dbm, 41.5, 1e-12));
  }

  #[test]
  fn thermal_only_ideal_chain_equals_the_receiver_sensitivity_formula() {
    // 站内接收机灵敏度计算器：S = −174 + 10·lg(B) + NF。Fa = 0、无接收馈线时必须一致。
    for nf in [0.0, 1.0, 8.0] {
      for bw in [500.0, 2700.0, 12_500.0] {
        let r = evaluate(&LinkInput {
          rx_line_db: 0.0,
          rx_nf_db: nf,
          ext_noise_db: 0.0,
          bandwidth_hz: bw,
          ..base()
        })
        .expect("合法");
        assert!(
          near(r.noise_dbm, -174.0 + 10.0 * bw.log10() + nf, 1e-12),
          "NF {nf} B {bw}"
        );
      }
    }
  }

  #[test]
  fn signal_agrees_with_the_existing_link_budget_kernel() {
    // 与 `link_budget::received_dbm` 交叉核对：发射馈线并入「−发射功率」，其余损耗并入附加损耗。
    let mut i = noisy();
    i.path_extra_db = 4.0;
    i.misc_loss_db = 2.5;
    let r = evaluate(&i).expect("合法");
    let tx_dbm = 10.0 * (i.tx_power_w * 1000.0).log10() - i.tx_line_db;
    let theirs = received_dbm(
      tx_dbm,
      i.tx_gain_dbi,
      i.rx_gain_dbi,
      i.freq_mhz,
      i.dist_km,
      i.path_extra_db + i.misc_loss_db,
    )
    .expect("合法");
    assert!(
      near(r.signal_dbm, theirs, 1e-12),
      "{} vs {theirs}",
      r.signal_dbm
    );
  }

  #[test]
  fn excess_noise_adds_between_external_and_receive_chain() {
    // 独立路径：不用闭式 `fa − 1 + L·Fr`，而是逐项累加功率（以 kT0B 为单位）：
    //   外部噪声经馈线衰减 + 馈线自身热噪声 + 接收机附加噪声，再除以馈线增益折回天线端子。
    for i in [
      base(),
      noisy(),
      LinkInput {
        ext_noise_db: 12.0,
        ..base()
      },
    ] {
      let (fa, l, fr) = (
        db_to_lin(i.ext_noise_db),
        db_to_lin(i.rx_line_db),
        db_to_lin(i.rx_nf_db),
      );
      let g = 1.0 / l;
      let f_stage = (fa * g + (1.0 - g) + (fr - 1.0)) / g;
      let r = evaluate(&i).expect("合法");
      assert!(near(db_to_lin(r.noise_factor_db), f_stage, 1e-12), "{i:?}");
      let share = r.ext_share.expect("有超出热噪声的部分");
      assert!(near(share, (fa - 1.0) / (f_stage - 1.0), 1e-9), "{share}");
      assert!((0.0..=1.0).contains(&share));
    }
  }

  #[test]
  fn ext_share_is_none_only_when_nothing_exceeds_thermal() {
    let ideal = LinkInput {
      ext_noise_db: 0.0,
      rx_line_db: 0.0,
      rx_nf_db: 0.0,
      ..base()
    };
    assert_eq!(evaluate(&ideal).expect("合法").ext_share, None);
    // 只有接收链超出：外部占比恰为 0。
    let r = evaluate(&base()).expect("合法");
    assert_eq!(r.ext_share, Some(0.0));
  }

  #[test]
  fn external_noise_makes_the_receive_chain_nearly_irrelevant() {
    // 外部噪声 40 dB：接收馈线 3 dB 与接收机 NF 8 dB 合起来也只回收不到 0.01 dB，
    // 而把外部噪声降到热噪声能多 29 dB。值来自 Python 原型。
    let r = evaluate(&noisy()).expect("合法");
    assert!(near(
      gain_of(&r, Stage::RxLine),
      0.002_724_928_349_223_887_4,
      1e-9
    ));
    assert!(near(
      gain_of(&r, Stage::RxNoiseFigure),
      0.004_598_019_418_580_179,
      1e-9
    ));
    assert!(near(
      gain_of(&r, Stage::ExternalNoise),
      29.005_030_234_841_996,
      1e-12
    ));
    assert!(near(gain_of(&r, Stage::TxLine), 1.0, 1e-12));
    assert!(gain_of(&r, Stage::Misc).abs() < 1e-12);
    assert!(near(
      r.ext_share.expect("有超出"),
      0.998_842_300_505_619_6,
      1e-12
    ));
    // 排第一的就是外部噪声。
    assert_eq!(r.reclaim[0].stage, Stage::ExternalNoise);
  }

  #[test]
  fn thermal_only_flips_the_ranking_to_the_lossy_stages() {
    // 同一套对象换成「只有热噪声」：外部噪声没得回收，接收馈线与 NF 的损耗是 1 比 1 的。
    let r = evaluate(&base()).expect("合法");
    assert!(near(gain_of(&r, Stage::RxLine), 2.0, 1e-12));
    assert!(near(gain_of(&r, Stage::TxLine), 1.5, 1e-12));
    assert!(near(gain_of(&r, Stage::RxNoiseFigure), 1.0, 1e-12));
    assert!(near(gain_of(&r, Stage::Misc), 0.5, 1e-12));
    assert!(gain_of(&r, Stage::ExternalNoise).abs() < 1e-12);
    let order: Vec<Stage> = r.reclaim.iter().map(|x| x.stage).collect();
    assert_eq!(
      order,
      [
        Stage::RxLine,
        Stage::TxLine,
        Stage::RxNoiseFigure,
        Stage::Misc,
        Stage::ExternalNoise
      ]
    );
  }

  #[test]
  fn reclaim_matches_the_closed_forms() {
    // 与流水线无关的闭式：三个噪声类环节各自等于 f 变化的 dB。
    for ext in [0.0, 3.0, 15.0, 40.0] {
      for (line, nf) in [(0.5, 1.0), (3.0, 8.0), (10.0, 2.0)] {
        let i = LinkInput {
          ext_noise_db: ext,
          rx_line_db: line,
          rx_nf_db: nf,
          ..base()
        };
        let r = evaluate(&i).expect("合法");
        let (fa, l, fr) = (db_to_lin(ext), db_to_lin(line), db_to_lin(nf));
        let f = fa - 1.0 + l * fr;
        let tag = format!("Fa {ext} L {line} NF {nf}");
        assert!(
          near(
            gain_of(&r, Stage::RxLine),
            lin_to_db(f / (fa - 1.0 + fr)),
            1e-9
          ),
          "{tag}"
        );
        assert!(
          near(
            gain_of(&r, Stage::RxNoiseFigure),
            lin_to_db(f / (fa - 1.0 + l)),
            1e-9
          ),
          "{tag}"
        );
        assert!(
          near(
            gain_of(&r, Stage::ExternalNoise),
            lin_to_db(f / (l * fr)),
            1e-9
          ),
          "{tag}"
        );
        // 两个纯损耗环节就是它们自己的 dB。
        assert!(near(gain_of(&r, Stage::TxLine), i.tx_line_db, 1e-12));
        assert!(near(gain_of(&r, Stage::Misc), i.misc_loss_db, 1e-12));
      }
    }
  }

  #[test]
  fn reclaim_is_sorted_and_covers_every_stage_once() {
    for i in [
      base(),
      noisy(),
      LinkInput {
        ext_noise_db: 6.0,
        ..base()
      },
    ] {
      let r = evaluate(&i).expect("合法");
      assert!(r.reclaim.windows(2).all(|w| w[0].gain_db >= w[1].gain_db));
      let mut seen = r.reclaim.map(|x| x.stage as u8).to_vec();
      seen.sort_unstable();
      assert_eq!(seen, [0, 1, 2, 3, 4], "每个环节恰好出现一次");
      assert!(
        r.reclaim
          .iter()
          .all(|x| x.gain_db >= 0.0 && x.gain_db.is_finite())
      );
    }
  }

  #[test]
  fn max_distance_and_required_power_round_trip() {
    // 把求出的距离 / 功率代回完整流水线，余量必须回到 0。
    for i in [
      base(),
      noisy(),
      LinkInput {
        required_snr_db: 70.0,
        ..base()
      },
    ] {
      let r = evaluate(&i).expect("合法");
      let at_d = evaluate(&LinkInput {
        dist_km: r.max_dist_km,
        ..i
      })
      .expect("合法");
      assert!(
        at_d.margin_db.abs() < 1e-9,
        "距离往返余量 {}",
        at_d.margin_db
      );
      let at_p = evaluate(&LinkInput {
        tx_power_w: r.required_tx_w,
        ..i
      })
      .expect("合法");
      assert!(
        at_p.margin_db.abs() < 1e-9,
        "功率往返余量 {}",
        at_p.margin_db
      );
    }
    // 余量为负时最大距离比当前近、所需功率比当前大。
    let short = evaluate(&LinkInput {
      required_snr_db: 70.0,
      ..base()
    })
    .expect("合法");
    assert!(short.margin_db < 0.0 && short.max_dist_km < 100.0 && short.required_tx_w > 10.0);
  }

  #[test]
  fn margin_responds_one_for_one_where_it_should() {
    let m0 = evaluate(&base()).expect("合法").margin_db;
    let m = |f: fn(&mut LinkInput)| {
      let mut i = base();
      f(&mut i);
      evaluate(&i).expect("合法").margin_db - m0
    };
    assert!(near(m(|i| i.tx_power_w *= 10.0), 10.0, 1e-9));
    assert!(near(m(|i| i.tx_gain_dbi += 1.0), 1.0, 1e-9));
    assert!(near(m(|i| i.rx_gain_dbi += 1.0), 1.0, 1e-9));
    assert!(near(m(|i| i.tx_line_db += 1.0), -1.0, 1e-9));
    assert!(near(m(|i| i.misc_loss_db += 1.0), -1.0, 1e-9));
    assert!(near(m(|i| i.path_extra_db += 1.0), -1.0, 1e-9));
    assert!(near(m(|i| i.required_snr_db += 1.0), -1.0, 1e-9));
    // 带宽 ×10 → 噪声 +10 dB；距离 ×10 → 路径损耗 +20 dB。
    assert!(near(m(|i| i.bandwidth_hz *= 10.0), -10.0, 1e-9));
    assert!(near(m(|i| i.dist_km *= 10.0), -20.0, 1e-9));
    // 只有热噪声时，接收馈线也是一比一。
    assert!(near(m(|i| i.rx_line_db += 1.0), -1.0, 1e-9));
  }

  #[test]
  fn rx_line_costs_less_than_its_loss_once_external_noise_dominates() {
    // 接收馈线多 1 dB，余量损失在 (0, 1] 之间，外部噪声越大越接近 0。
    let drop = |ext: f64| {
      let a = evaluate(&LinkInput {
        ext_noise_db: ext,
        ..base()
      })
      .expect("合法")
      .margin_db;
      let b = evaluate(&LinkInput {
        ext_noise_db: ext,
        rx_line_db: 3.0,
        ..base()
      })
      .expect("合法")
      .margin_db;
      a - b
    };
    let (d0, d10, d30, d50) = (drop(0.0), drop(10.0), drop(30.0), drop(50.0));
    assert!(near(d0, 1.0, 1e-9));
    assert!(
      d0 > d10 && d10 > d30 && d30 > d50 && d50 > 0.0,
      "{d0} {d10} {d30} {d50}"
    );
    assert!(d50 < 1e-3, "Fa = 50 dB 时 1 dB 馈线几乎不花余量：{d50}");
  }

  #[test]
  fn reference_plane_is_consistent() {
    for i in [base(), noisy()] {
      let r = evaluate(&i).expect("合法");
      assert!(near(r.signal_dbm - r.noise_dbm, r.snr_db, 1e-12));
      assert!(near(r.snr_db - i.required_snr_db, r.margin_db, 1e-12));
    }
  }

  #[test]
  fn invalid_inputs_are_rejected() {
    let ok = base();
    let bad: [LinkInput; 16] = [
      LinkInput {
        freq_mhz: 0.0,
        ..ok
      },
      LinkInput {
        freq_mhz: -1.0,
        ..ok
      },
      LinkInput { dist_km: 0.0, ..ok },
      LinkInput {
        tx_power_w: 0.0,
        ..ok
      },
      LinkInput {
        bandwidth_hz: 0.0,
        ..ok
      },
      LinkInput {
        tx_line_db: -0.1,
        ..ok
      },
      LinkInput {
        rx_line_db: -0.1,
        ..ok
      },
      LinkInput {
        misc_loss_db: -0.1,
        ..ok
      },
      LinkInput {
        rx_nf_db: -0.1,
        ..ok
      },
      LinkInput {
        ext_noise_db: -0.1,
        ..ok
      },
      LinkInput {
        path_extra_db: -0.1,
        ..ok
      },
      LinkInput {
        freq_mhz: f64::NAN,
        ..ok
      },
      LinkInput {
        tx_gain_dbi: f64::INFINITY,
        ..ok
      },
      LinkInput {
        required_snr_db: f64::NAN,
        ..ok
      },
      LinkInput {
        rx_gain_dbi: f64::NEG_INFINITY,
        ..ok
      },
      // 各项有限但求和溢出：不能把 `inf` 当结果交出去。
      LinkInput {
        tx_gain_dbi: 1e308,
        rx_gain_dbi: 1e308,
        ..ok
      },
    ];
    for (n, i) in bad.iter().enumerate() {
      assert_eq!(evaluate(i), None, "第 {n} 组应被拒绝：{i:?}");
    }
    assert!(evaluate(&ok).is_some());
  }

  #[test]
  fn near_field_is_flagged_below_ten_wavelengths() {
    // 145 MHz：λ = 2.0675 m，十个波长 = 20.675 m。
    let at = |dist_km: f64| evaluate(&LinkInput { dist_km, ..base() }).expect("合法");
    let r = at(0.010);
    assert!(near(
      r.far_field_km,
      10.0 * 299.792_458 / 145.0 / 1000.0,
      1e-12
    ));
    assert!(r.near_field, "10 m < 20.675 m");
    assert!(!at(0.030).near_field, "30 m > 20.675 m");
    // 恰在边界上算远场（严格小于才是近场），略近一点就是近场。
    assert!(!at(r.far_field_km).near_field);
    assert!(at(r.far_field_km * (1.0 - 1e-9)).near_field);
    // 真实场景都在远场；低频的远场更远（3.7 MHz 时 810 m）。
    assert!(!evaluate(&base()).expect("合法").near_field);
    let hf = evaluate(&noisy()).expect("合法");
    assert!(
      !hf.near_field && near(hf.far_field_km, 0.810_2, 1e-3),
      "{}",
      hf.far_field_km
    );
    assert!(
      evaluate(&LinkInput {
        dist_km: 0.5,
        ..noisy()
      })
      .expect("合法")
      .near_field,
      "3.7 MHz 下 500 m 还没到远场"
    );
  }

  #[test]
  fn negative_fspl_only_happens_inside_the_near_field() {
    // 近场标记的存在理由：FSPL 为负（信号比 EIRP 还大）的点必须都被标成近场。
    for f in [1.9, 7.1, 145.0, 1296.0] {
      for d_km in [1e-5, 1e-4, 1e-3, 1e-2, 0.1, 1.0, 100.0] {
        let r = evaluate(&LinkInput {
          freq_mhz: f,
          dist_km: d_km,
          ..base()
        })
        .expect("合法");
        if r.path_loss_db < 0.0 {
          assert!(
            r.near_field,
            "{f} MHz {d_km} km 的 FSPL = {} dB 却不是近场",
            r.path_loss_db
          );
        }
      }
    }
  }

  fn rg213() -> usize {
    FEEDLINE_SPECS
      .iter()
      .position(|s| s.name == "RG-213")
      .expect("表内有 RG-213")
  }

  #[test]
  fn cable_loss_matches_the_feedline_kernel_for_valid_input() {
    let i = rg213();
    let expect = feedline_loss_db(&FEEDLINE_SPECS[i], 145.5, 10.0);
    assert_eq!(cable_loss_db(i, 10.0, 145.5), Some(expect));
    // e2e 里的默认场景：RG-213 10 m @ 145.5 MHz ≈ 1.03 dB。
    assert!((expect - 1.03).abs() < 0.005, "{expect}");
    // 长度恰为 0：合法，没有这段馈线。
    assert_eq!(cable_loss_db(i, 0.0, 145.5), Some(0.0));
  }

  #[test]
  fn cable_loss_rejects_what_the_kernel_would_silently_zero() {
    let i = rg213();
    // 底层对非正长度 / 非正频率直接给 0 —— 这正是必须在入口拦下的原因。
    assert_eq!(feedline_loss_db(&FEEDLINE_SPECS[i], 145.5, -5.0), 0.0);
    assert_eq!(feedline_loss_db(&FEEDLINE_SPECS[i], 0.0, 10.0), 0.0);
    for (len, f) in [
      (-5.0, 145.5),
      (-0.001, 145.5),
      (f64::NAN, 145.5),
      (f64::INFINITY, 145.5),
      (10.0, 0.0),
      (10.0, -1.0),
      (10.0, f64::NAN),
      (10.0, f64::INFINITY),
    ] {
      assert_eq!(cable_loss_db(i, len, f), None, "长度 {len} 频率 {f}");
    }
    assert_eq!(
      cable_loss_db(FEEDLINE_SPECS.len(), 10.0, 145.5),
      None,
      "型号下标越界"
    );
  }

  #[test]
  fn power_text_never_writes_a_number_that_belongs_to_the_next_unit() {
    assert_eq!(format_power_w(5.0), "5.00 W");
    assert_eq!(format_power_w(0.5), "500 mW");
    // 取整后顶到 1000：整体升档，而不是写成 `1000 mW`。
    assert_eq!(format_power_w(0.999_996), "1.00 W");
    assert_eq!(format_power_w(0.000_999_996), "1.00 mW");
    // 页面上常见的小功率（`required_tx_w` 的锚点值 2.8e-5 W）落在 μW 档。
    let small = format_power_w(2.805_092_882e-5);
    assert!(
      small.ends_with(" μW") && small.starts_with("28."),
      "{small}"
    );
  }

  #[test]
  fn power_text_keeps_extremes_readable_and_rejects_garbage() {
    // 两端之外给量级，不排出 `0.000 μW` 或一长串零。
    assert_eq!(format_power_w(1e7), "1.00e7 W");
    assert_eq!(format_power_w(1e-12), "1.00e-12 W");
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0] {
      assert_eq!(format_power_w(bad), "—", "{bad}");
    }
  }

  #[test]
  fn reach_text_upgrades_the_unit_at_the_boundary_and_keeps_the_order_of_magnitude() {
    assert_eq!(format_reach_km(200.0), "200.00 km");
    assert_eq!(format_reach_km(0.020_6), "20.60 m");
    // 999.999 m 取整后是 1000：写成 `1.00 km`，不是 `1000.00 m`。
    assert_eq!(format_reach_km(0.999_999), "1.00 km");
    // 自由空间外推出来的大数只留整数，更大的给量级。
    assert_eq!(format_reach_km(59_707.154_9), "59707 km");
    assert_eq!(format_reach_km(2.5e7), "2.50e7 km");
    assert_eq!(format_reach_km(1e-9), "1.00e-9 km");
    for bad in [f64::NAN, f64::INFINITY, -0.1] {
      assert_eq!(format_reach_km(bad), "—", "{bad}");
    }
  }
}
