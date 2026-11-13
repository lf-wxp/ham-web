//! 大气噪声（雷电）与多噪声合成：ITU-R P.372 第 5、8 部分的实现。
//!
//! # 它回答什么
//!
//! HF 上真正决定信噪比的往往不是接收机 NF，而是天线从环境里收到的噪声。其中**大气噪声**
//! （远处的雷电，经电离层与地波传到天线）在 1–30 MHz 常年与人为噪声同量级。P.372 把它写成
//! 「相对 `kT0b` 的 dB 数」`Fa`，随**季节（月）、当地时间（4 小时时段）、经纬度、频率**变化，
//! 由一套球谐级数 + 频率多项式给出。
//!
//! 与本模块配套的 [`crate::noise_model`] 给的是人为噪声与银河噪声的中位数；三者按 P.372 第 8 部分
//! 合成才是总外部噪声。**不要**把几个 `Fa` 直接按 dB 相加 —— 合成是在功率域做的，而且要用各分量
//! 的上 / 下十分位偏差。
//!
//! # 数据与许可
//!
//! 系数表来自 ITU-R SG3 的公开仓库 `ITU-R-Study-Group-3/ITU-R-HF`（`P372/Data/COEFFxxW.txt`，
//! 12 个月各一份；生成 Rust 常量的脚本是 `crates/core/reference/p372_coeffs.py`）。仓库 README
//! 写明「The ITURHFProp, P533 and P372 software … may be used by implementers in their
//! implementation of the Recommendation … free from any copyright assertions」，并附免责声明。
//! 这些系数文件是同一份包的一部分、是该算法不可拆分的输入（原文自己也写「available from …
//! Radiocommunication Study Group 3」）。
//!
//! **没有**使用推荐书 PDF 里的图（图 13–36）：那份文档是「All rights reserved」，图不能复制。
//! 本模块是**按官方程序的口径自己算**，不是抄图上的曲线。
//!
//! # 算法与验证
//!
//! 逐行对照 `P372/Src/P372/Noise.c` 的 `AtmosphericNoise()` / `GetFamParameters()` 与 `Noise()`
//! （版本号 14.3，对应 P.372-14）。验证方式是把官方源码编成命令行程序，与本模块对同一批输入
//! 逐点比较：见 `crates/core/reference/p372_check.py`（12 个月 × 7 个小时 × 13 个纬度 × 18 个经度 ×
//! 12 个频率 = 235 872 组大气噪声，再加合成）。官方源码固定在脚本里的 `OFFICIAL_SHA` 那一版。
//!
//! **经度网格必须含非 15° 整倍数**：本地时由 UTC 与经度折算，官方只截断「经度 / 15」那一项；
//! 早先的网格只取 30° 的整倍数，截断顺序写反也测不出来（西经 73.9° 的某些小时会差整个时段，
//! 最大差 32 dB），所以网格里专门加了 `LNGS_OFF_GRID`。
//!
//! # 边界
//!
//! - 频率只到 0.1–30 MHz（原文的气象噪声曲线覆盖 0.1–20 MHz，20 MHz 以上按 20 MHz 的值处理，
//!   与官方源码一致）；位置用**地理坐标**，极区（|φ| > 80°）的外推不可靠；
//! - 只给**中位**与上下十分位偏差，不给完整的幅度概率分布（APD 曲线）；
//! - 不含室内、城市峡谷等局部效应，也不含雷暴的短时爆发（P.372 给的是长期中位统计）；
//! - 合成只做 P.372 第 8 部分的三分量（大气 + 银河 + 人为）高斯近似，**不建模脉冲成分**
//!   （原文脚注：人为噪声常带脉冲性，对数字系统可能更要紧）。

#[path = "atmospheric_coeffs.rs"]
mod coeffs;

use crate::noise_model::Environment;

/// P.372 里 4 小时时段的个数。
const TIME_BLOCKS: usize = 6;

/// 系数表从官方仓库取回并核对算法的日期（[`crate::staleness`] 用它判过期）。
pub const CHECKED_ON: &str = "2026-11-12";

/// 大气噪声在某个「月 × 4 小时时段」上的原始输出（dB above kT0b，1 MHz）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AtmNoise {
  /// 中位噪声系数 `Fam`（dB）。
  pub fa_db: f64,
  /// 上十分位偏差 `Du`（dB）。
  pub du_db: f64,
  /// 下十分位偏差 `Dl`（dB）。
  pub dl_db: f64,
  /// 中位的标准差（dB）。
  pub sigma_db: f64,
}

/// 一次查询。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AtmNoiseInput {
  /// 月（1–12）。
  pub month: u32,
  /// UTC 小时（0 ≤ h < 24）。官方程序的小时是整数，**小数部分舍去**（12.9 按 12 算），
  /// 本站同口径，不在小时内再做插值。
  pub hour_utc: f64,
  /// 纬度（度，北正）。
  pub lat_deg: f64,
  /// 经度（度，东正）。
  pub lon_deg: f64,
  /// 频率（MHz）。
  pub freq_mhz: f64,
}

/// 把用户输入的月份（数值）收成 1–12 的整数。
///
/// 带小数（7.5）、越界或非有限数都返回 `None`：`as u32` 会把 7.9 悄悄截成 7、把 −3 钳成 0，
/// 等于算了另一个月。两个用到大气噪声的页面共用这一条口径。
#[must_use]
pub fn month_from_f64(v: f64) -> Option<u32> {
  (v.is_finite() && v.fract() == 0.0 && (1.0..=12.0).contains(&v)).then_some(v as u32)
}

impl AtmNoiseInput {
  /// 是否在可用范围内。
  #[must_use]
  pub fn validate(&self) -> bool {
    (1..=12).contains(&self.month)
      && self.hour_utc.is_finite()
      && (0.0..24.0).contains(&self.hour_utc)
      && self.lat_deg.is_finite()
      && (-90.0..=90.0).contains(&self.lat_deg)
      && self.lon_deg.is_finite()
      && (-180.0..=180.0).contains(&self.lon_deg)
      && self.freq_mhz.is_finite()
      && (0.1..=30.0).contains(&self.freq_mhz)
  }
}

/// 某个「月 + 时段」在给定位置 / 频率上的噪声参数（对应官方 `GetFamParameters`）。
fn fam_parameters(month: u32, tmblk: usize, lon_rad: f64, lat_rad: f64, freq_mhz: f64) -> AtmNoise {
  let fakp = &coeffs::FAKP[(month - 1) as usize][tmblk];
  let fakabp = &coeffs::FAKABP[(month - 1) as usize][tmblk];
  let fam = &coeffs::FAM[(month - 1) as usize];

  // 经度的傅里叶正弦级数：q = 东经 / 2（负经度折成 0–2π）。
  let q0 = if lon_rad < 0.0 {
    (lon_rad + 2.0 * std::f64::consts::PI) / 2.0
  } else {
    lon_rad / 2.0
  };
  let mut zz = [0.0f64; 29];
  for (j, z) in zz.iter_mut().enumerate() {
    // 官方：`R = Σ_{k=0..14} sin((k+1)·q)·fakp[k][j]`，常数项是 `fakp[15][j]`。
    let mut r = 0.0;
    for (k, row) in fakp.iter().take(15).enumerate() {
      r += ((k + 1) as f64 * q0).sin() * row[j];
    }
    *z = r + fakp[15][j];
  }
  // 纬度：再来一层正弦级数。
  let q = lat_rad + std::f64::consts::FRAC_PI_2;
  let mut r = 0.0;
  for (j, &z) in zz.iter().enumerate() {
    r += ((j + 1) as f64 * q).sin() * z;
  }
  let fam_1mhz = r + fakabp[0] + fakabp[1] * q;

  // 频率多项式的两个自变量（NBS Tech Note 318，Lucas & Harper）。
  let u = [-0.75, (8.0 * 2f64.powf(freq_mhz.log10()) - 11.0) / 4.0];
  // 南半球的时段索引要 +6（官方源码的约定）。
  let row = if lat_rad < 0.0 { tmblk + 6 } else { tmblk };
  let (mut pz, mut px) = (0.0, 0.0);
  let mut cz = 0.0;
  for (k, &uk) in u.iter().enumerate() {
    pz = uk * fam[row][0] + fam[row][1];
    px = uk * fam[row][7] + fam[row][8];
    for j in 2..7 {
      pz = uk * pz + fam[row][j];
      px = uk * px + fam[row][j + 7];
    }
    if k == 0 {
      cz = fam_1mhz * (2.0 - pz) - px;
    }
  }
  let fa_db = cz * pz + px;

  // 偏差：频率上限分别是 20 MHz（Du / Dl / 其标准差）与 10 MHz（SigmaFam）。
  let mut x = freq_mhz.log10();
  if freq_mhz > 20.0 {
    x = 20f64.log10();
  }
  let dud = &coeffs::DUD[(month - 1) as usize];
  let mut v = [0.0f64; 5];
  for (j, slot) in v.iter_mut().enumerate() {
    let xj = if j == 4 && freq_mhz > 10.0 { 1.0 } else { x };
    let coef = &dud[j][row];
    let mut y = coef[0];
    for c in &coef[1..] {
      y = y * xj + *c;
    }
    *slot = y;
  }
  AtmNoise {
    fa_db,
    du_db: v[0],
    dl_db: v[1],
    sigma_db: v[4],
  }
}

fn to_power(db: f64) -> f64 {
  10f64.powf(db / 10.0)
}

fn to_db(p: f64) -> f64 {
  10.0 * p.log10()
}

/// 大气噪声（中位 + 上下十分位），按小时在相邻两个 4 小时时段之间在**功率域**插值。
///
/// 本地时间由 UTC 与经度换算（每 15° 一小时），与官方程序一致；时段内按整点小时线性插值
/// （小数部分舍去，见 `AtmNoiseInput::hour_utc`）。
#[must_use]
pub fn atmospheric(input: &AtmNoiseInput) -> Option<AtmNoise> {
  if !input.validate() {
    return None;
  }
  let lon_rad = input.lon_deg.to_radians();
  let lat_rad = input.lat_deg.to_radians();

  // 本地时：UTC + 经度 / 15。官方 `AtmosphericNoise()` 只把**经度那一项**截断成整点
  // （`lrxmt = hour + (int)(rlng / (15*D2R))`，`hour` 本来就是 `int`），再按 `lrxmt/4` 取时段、
  // `fmod(lrxmt,4)/4` 求插值权重。必须**分别**截断：先相加再截断，西经（经度 / 15 为负小数）
  // 的本地时会差 1 小时，跨过 4 小时时段边界时整个时段都不对（西经 73.9°、6 月 12 时差 32 dB）。
  let local = (input.hour_utc.trunc() + (input.lon_deg / 15.0).trunc()).rem_euclid(24.0);
  let blk = ((local / 4.0) as usize).min(TIME_BLOCKS - 1);
  let next = (blk + 1) % TIME_BLOCKS;
  let t = (local % 4.0) / 4.0;

  let now = fam_parameters(input.month, blk, lon_rad, lat_rad, input.freq_mhz);
  let adj = fam_parameters(input.month, next, lon_rad, lat_rad, input.freq_mhz);

  let mix = |a: f64, b: f64| to_db(to_power(a) * (1.0 - t) + to_power(b) * t);
  Some(AtmNoise {
    fa_db: mix(now.fa_db, adj.fa_db),
    du_db: mix(now.du_db, adj.du_db),
    dl_db: mix(now.dl_db, adj.dl_db),
    sigma_db: mix(now.sigma_db, adj.sigma_db),
  })
}

/// 合成结果（P.372 第 8 部分的高斯近似）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CombinedNoise {
  /// 合成后的中位（dB，相对 kT0b）。
  pub fa_db: f64,
  /// 上十分位偏差（dB）。
  pub du_db: f64,
  /// 下十分位偏差（dB）。
  pub dl_db: f64,
}

/// 把大气、银河、人为三种噪声按 P.372 第 8 部分合成（官方 `Noise()` 的合成段）。
///
/// 口径逐行对着官方程序：上、下十分位**各合成一次**，`FamT` 取两者中**较小**的那个
/// （官方注释写「Worst-case noise」，代码是 `min(FamTu, FamTl)`；这里照代码）。
/// 银河噪声按官方的 `52.0 − 23.0·lg f`、两个十分位偏差都取 2 dB、`σG = 1.56` —— **不分频段**，
/// 官方程序在 10 MHz 以下同样把它算进去（P.372 原文说那里被电离层屏蔽、观测不到；
/// 这是官方程序与正文之间的既有差别，页面上要说明）。
/// 人为噪声取 [`crate::noise_model::fam_db`] 的曲线，**超出该曲线的适用频段返回 `None`**，不外推。
#[must_use]
pub fn combined(
  env: Environment,
  month: u32,
  hour_utc: f64,
  lat_deg: f64,
  lon_deg: f64,
  freq_mhz: f64,
) -> Option<CombinedNoise> {
  if matches!(env, Environment::Galactic) {
    return None; // 银河噪声是合成里固定存在的分量，不是「环境」。
  }
  let atm = atmospheric(&AtmNoiseInput {
    month,
    hour_utc,
    lat_deg,
    lon_deg,
    freq_mhz,
  })?;
  let fa_m = crate::noise_model::fam_db(env, freq_mhz)?;
  let (du_m, dl_m) = man_made_deciles(env);
  let fa_g = galactic_db(freq_mhz);
  let sigma_g = GALACTIC_SIGMA_DB;
  let c = 10.0 / std::f64::consts::LN_10;

  // 一个方向（上 / 下十分位）的合成：返回（中位，该方向的十分位偏差）。
  let one = |d_a: f64, d_m: f64| {
    let (sa, sm) = (d_a / 1.282, d_m / 1.282);
    let term = |fa: f64, s: f64| (fa / c + s * s / (2.0 * c * c)).exp();
    let (ta, tg, tm) = (term(atm.fa_db, sa), term(fa_g, sigma_g), term(fa_m, sm));
    let alpha = ta + tg + tm;
    let beta = ta * ta * (sa * sa / (c * c)).exp_m1()
      + tg * tg * (sigma_g * sigma_g / (c * c)).exp_m1()
      + tm * tm * (sm * sm / (c * c)).exp_m1();
    let gamma = (atm.fa_db / c).exp() + (fa_g / c).exp() + (fa_m / c).exp();
    // 官方的分支条件里 DuG = 2 dB 恒不超过 12，所以只看大气与人为两项。
    let sigma = if d_a > 12.0 || d_m > 12.0 {
      c * (2.0 * (alpha / gamma).ln()).sqrt()
    } else {
      c * (1.0 + beta / (alpha * alpha)).ln().sqrt()
    };
    (
      c * (alpha.ln() - sigma * sigma / (2.0 * c * c)),
      1.282 * sigma,
    )
  };

  let (fa_u, du_t) = one(atm.du_db, du_m);
  let (fa_l, dl_t) = one(atm.dl_db, dl_m);
  let fa_db = fa_u.min(fa_l);
  // 大气项十分位偏差 > 12 dB 的分支里对数的真数可能 < 1，官方程序此时给 NaN；这里不把 NaN 交出去。
  (fa_db.is_finite() && du_t.is_finite() && dl_t.is_finite()).then_some(CombinedNoise {
    fa_db,
    du_db: du_t,
    dl_db: dl_t,
  })
}

/// 「按业余波段列外部噪声」表用的频率（MHz）：630 m、160 m 到 10 m 各取常用的一点。
///
/// 全部落在人为噪声曲线（0.3–250 MHz，安静乡村 0.3–30 MHz）与大气噪声（0.1–30 MHz）的共同适用范围内，
/// 所以任何环境类别下每一行都有结果。
pub const BAND_FREQS_MHZ: [f64; 11] = [
  0.475, 1.85, 3.65, 5.36, 7.1, 10.12, 14.2, 18.1, 21.2, 24.9, 28.5,
];

/// 三种来源里哪一种主导（中位最高）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseSource {
  /// 大气噪声（雷电）。
  Atmospheric,
  /// 人为噪声。
  ManMade,
  /// 银河噪声。
  Galactic,
}

/// 一个频率上的外部噪声分解。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoiseRow {
  /// 频率（MHz）。
  pub freq_mhz: f64,
  /// 大气噪声中位（dB，相对 kT0b）。
  pub atmospheric_db: f64,
  /// 人为噪声中位（dB）。
  pub man_made_db: f64,
  /// 银河噪声中位（dB）：官方程序的 `52 − 23·lg f`，不分频段。
  pub galactic_db: f64,
  /// 三者按 P.372 第 8 部分合成的结果。
  pub total: CombinedNoise,
}

impl NoiseRow {
  /// 中位最高的来源（并列时按大气 > 人为 > 银河，只是为了结果确定）。
  #[must_use]
  pub fn dominant(&self) -> NoiseSource {
    if self.atmospheric_db >= self.man_made_db && self.atmospheric_db >= self.galactic_db {
      NoiseSource::Atmospheric
    } else if self.man_made_db >= self.galactic_db {
      NoiseSource::ManMade
    } else {
      NoiseSource::Galactic
    }
  }
}

/// 某月、某 UTC 小时、某地点、某种环境下，[`BAND_FREQS_MHZ`] 各频率的外部噪声分解。
///
/// 月 / 小时 / 坐标越界或 `env` 是 `Galactic` 返回 `None`。
#[must_use]
pub fn band_table(
  env: Environment,
  month: u32,
  hour_utc: f64,
  lat_deg: f64,
  lon_deg: f64,
) -> Option<Vec<NoiseRow>> {
  BAND_FREQS_MHZ
    .iter()
    .map(|&freq_mhz| {
      let atm = atmospheric(&AtmNoiseInput {
        month,
        hour_utc,
        lat_deg,
        lon_deg,
        freq_mhz,
      })?;
      Some(NoiseRow {
        freq_mhz,
        atmospheric_db: atm.fa_db,
        man_made_db: crate::noise_model::fam_db(env, freq_mhz)?,
        galactic_db: galactic_db(freq_mhz),
        total: combined(env, month, hour_utc, lat_deg, lon_deg, freq_mhz)?,
      })
    })
    .collect()
}

/// 银河噪声中位（dB）：官方程序的 `52 − 23·lg f`，**不分频段**。
///
/// `c` / `d` 取自 `noise_model` 的曲线 E，与那边共用一份常数；曲线 E 的适用频段（10–100 MHz）
/// 在这里不生效 —— 官方程序在合成里一直算到 0.1 MHz（见 `combined` 的说明）。
fn galactic_db(freq_mhz: f64) -> f64 {
  Environment::Galactic.curve().at(freq_mhz)
}

/// 官方程序里银河噪声中位的标准差 `σG`（dB）。
const GALACTIC_SIGMA_DB: f64 = 1.56;

/// 人为噪声的上 / 下十分位偏差 `(Du, Dl)`（dB），官方 `Noise()` 里按环境类别取值。
///
/// 中位（`c`、`d`）不在这里：它们只有 `noise_model` 一份。
fn man_made_deciles(env: Environment) -> (f64, f64) {
  match env {
    Environment::City => (11.0, 6.7),
    Environment::Residential => (10.6, 5.3),
    Environment::Rural | Environment::QuietRural => (9.2, 4.6),
    // 银河不是一种「环境」：它的中位与偏差由 `combined()` 单独处理（两个十分位偏差都取 2 dB），
    // 这里只为了让 `Environment` 是穷尽的（调用方不该把 `Galactic` 传进来）。
    Environment::Galactic => (2.0, 2.0),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 官方程序（`ITU-R-HF` 的 `P372`，版本 14.3）对同一批输入的输出。
  ///
  /// 由 `crates/core/reference/p372_check.py` 生成：那个脚本把官方源码编成命令行程序，
  /// 按 `月（脚本内部转成官方 0 起的 ReadFamDud 参数）小时 纬度 经度 频率` 喂进去，逐点比较。
  /// 完整网格是 12 个月 × 7 个小时 × 13 个纬度 × 18 个经度（含非 15° 整倍数）× 12 个频率 = 235 872 组，
  /// 实测最大差 **0.0 dB**（逐位相同）。这里只钉几个代表点。
  #[test]
  fn agrees_with_the_official_p372_program() {
    // (月, UTC 小时, 纬度, 经度, 频率 MHz, 官方 FaA, 官方 DuA, 官方 DlA)
    let cases = [
      (1, 0, 40.0, 116.0, 7.0, 33.408_968, 8.435_214, 7.124_371),
      (1, 4, 40.0, 116.0, 7.0, 26.239_268, 8.508_046, 7.162_275),
      (1, 8, 40.0, 116.0, 7.0, 38.030_635, 9.083_345, 8.103_559),
      (1, 12, 40.0, 116.0, 7.0, 40.313_947, 7.928_185, 6.936_833),
      (1, 2, 0.0, 150.0, 0.5, 72.735_424, 10.475_533, 5.907_442),
      (1, 18, -30.0, 20.0, 14.0, 34.413_367, 5.514_460, 4.690_757),
    ];
    for (month, hour, lat, lon, f, fa, du, dl) in cases {
      let got = atmospheric(&AtmNoiseInput {
        month,
        hour_utc: f64::from(hour),
        lat_deg: lat,
        lon_deg: lon,
        freq_mhz: f,
      })
      .unwrap();
      assert!(
        (got.fa_db - fa).abs() < 1e-4,
        "{month}/{hour}/{lat}/{lon}/{f}: {} vs {fa}",
        got.fa_db
      );
      assert!(
        (got.du_db - du).abs() < 1e-4,
        "{month}/{hour}: Du {} vs {du}",
        got.du_db
      );
      assert!(
        (got.dl_db - dl).abs() < 1e-4,
        "{month}/{hour}: Dl {} vs {dl}",
        got.dl_db
      );
    }
  }

  /// 本地时由 UTC 与经度折算时，官方只截断「经度 / 15」那一项；先相加再截断会让西经的本地时差 1 小时。
  ///
  /// 西经 73.9°：`−73.9 / 15 = −4.93`，官方截成 −4，UTC 12 时 → 本地 8 时（时段 2 的起点）；
  /// 先相加则 `12 − 4.93 = 7.07` → 7 时（时段 1 的 3/4 处），6 月、南纬 15°、2 MHz 差 32 dB。
  /// 下面两组参照值取自 `p372_check.py` 对官方程序的输出（它们是旧网格看不到的点：30° 步长的经度都是 15° 的整倍数）。
  #[test]
  fn local_time_truncates_longitude_alone_as_the_official_program_does() {
    let atm = atmospheric(&AtmNoiseInput {
      month: 6,
      hour_utc: 12.0,
      lat_deg: -15.0,
      lon_deg: -73.9,
      freq_mhz: 2.0,
    })
    .unwrap();
    assert!((atm.fa_db - 28.407_712).abs() < 1e-5, "Fa {}", atm.fa_db);
    assert!((atm.du_db - 8.726_274).abs() < 1e-5, "Du {}", atm.du_db);
    assert!((atm.dl_db - 6.623_028).abs() < 1e-5, "Dl {}", atm.dl_db);

    // 合成（安静乡村）走同一条本地时折算。
    let c = combined(Environment::QuietRural, 6, 12.0, 0.0, -73.9, 3.5).unwrap();
    assert!((c.fa_db - 42.563_513).abs() < 1e-5, "FamT {}", c.fa_db);
    assert!((c.du_db - 8.869_420).abs() < 1e-5, "DuT {}", c.du_db);
    assert!((c.dl_db - 4.465_333).abs() < 1e-5, "DlT {}", c.dl_db);
  }

  /// 小时的小数部分舍去（官方的小时是 `int`）：12.9 与 12 给出同一个结果。
  #[test]
  fn fractional_hours_are_truncated_like_the_official_integer_hour() {
    let at = |hour_utc: f64| {
      atmospheric(&AtmNoiseInput {
        month: 6,
        hour_utc,
        lat_deg: 40.0,
        lon_deg: 116.0,
        freq_mhz: 7.0,
      })
      .unwrap()
    };
    assert_eq!(at(12.0), at(12.9));
    assert_ne!(at(12.0), at(13.0), "整点之间应有变化，否则上一条断言是空的");
  }

  /// 系数表是**生成物**：这个单测钉死几处「抄错就会崩」的位置，防止重新生成时静默换了一份源。
  ///
  /// - `FAKP[0][0][0]`（1 月、时段 0、正弦 1 次项、0 阶）与 `FAKP[0][15][0]`（常数项）必须不同 ——
  ///   这两层的顺序写反过，正弦项会全部落到常数项上（表现是 `Fa` 差十几 dB，但曲线形状还对）；
  /// - `FAM` 的行数 12 = 6 个时段 × 南北半球，南半球用 `tmblk + 6`；
  /// - `DUD` 的第一层是「5 类偏差」而不是「12 行」：`DUD[0]` 有 12 行，`DUD[0][0]` 有 5 项。
  #[test]
  fn generated_coefficients_have_the_expected_shape() {
    assert_eq!(coeffs::FAKP.len(), 12);
    assert_eq!(coeffs::FAKP[0].len(), 6);
    assert_eq!(coeffs::FAKP[0][0].len(), 16);
    assert_eq!(coeffs::FAKP[0][0][0].len(), 29);
    assert!(
      (coeffs::FAKP[0][0][0][0] - coeffs::FAKP[0][0][15][0]).abs() > 1e-9,
      "正弦项与常数项的第一位相同，说明 FAKP 的 reshape 写反了"
    );
    assert_eq!(coeffs::FAKABP[0][0].len(), 2);
    assert_eq!(coeffs::DUD[0].len(), 5, "DUD 第一层是 5 类偏差");
    assert_eq!(coeffs::DUD[0][0].len(), 12, "第二层是 12 行（含南北半球）");
    assert_eq!(coeffs::DUD[0][0][0].len(), 5);
    assert_eq!(coeffs::FAM[0].len(), 12);
    assert_eq!(coeffs::FAM[0][0].len(), 14);
    // 所有系数都必须是有限数（读出 NaN 说明解析文件时切错了段）。
    assert!(
      coeffs::FAKP
        .iter()
        .flatten()
        .flatten()
        .flatten()
        .all(|v| v.is_finite()),
      "FAKP 里有 NaN / inf"
    );
  }

  #[test]
  fn month_input_must_be_a_whole_number_from_1_to_12() {
    assert_eq!(month_from_f64(1.0), Some(1));
    assert_eq!(month_from_f64(7.0), Some(7));
    assert_eq!(month_from_f64(12.0), Some(12));
    for bad in [0.0, 13.0, -3.0, 7.5, 0.9, f64::NAN, f64::INFINITY, 1e20] {
      assert_eq!(month_from_f64(bad), None, "{bad}");
    }
  }

  #[test]
  fn rejects_inputs_outside_the_standard() {
    let base = AtmNoiseInput {
      month: 6,
      hour_utc: 12.0,
      lat_deg: 40.0,
      lon_deg: 116.0,
      freq_mhz: 7.0,
    };
    assert!(atmospheric(&base).is_some());
    for bad in [
      AtmNoiseInput { month: 0, ..base },
      AtmNoiseInput { month: 13, ..base },
      AtmNoiseInput {
        hour_utc: 24.0,
        ..base
      },
      AtmNoiseInput {
        lat_deg: 91.0,
        ..base
      },
      AtmNoiseInput {
        lon_deg: 181.0,
        ..base
      },
      AtmNoiseInput {
        freq_mhz: 0.05,
        ..base
      },
      AtmNoiseInput {
        freq_mhz: 31.0,
        ..base
      },
    ] {
      assert!(atmospheric(&bad).is_none(), "{bad:?}");
    }
  }

  /// 合成（大气 + 银河 + 人为）对官方 `Noise()` 的 `FamT / DuT / DlT`。
  ///
  /// 同一个脚本比了 334 152 组（环境 0–3 各一遍），最大差 0.0 dB；这里钉五个代表点。
  /// **变异验证**：把 `fa_u.min(fa_l)` 改成 `max`，脚本最坏差 3.6 dB，这里第 1 点先红（城市 7 MHz 差 0.07 dB）——
  /// 合成取较小者是官方代码，不是直觉上的「最坏情形取较大」。
  #[test]
  fn combined_agrees_with_the_official_p372_program() {
    // (环境, 月, UTC 小时, 纬度, 经度, 频率, 官方 FamT, DuT, DlT)
    let cases = [
      (
        Environment::City,
        1,
        0,
        40.0,
        116.0,
        7.0,
        53.440_038,
        10.984_175,
        6.646_469,
      ),
      (
        Environment::Residential,
        1,
        12,
        40.0,
        116.0,
        7.0,
        49.616_723,
        10.424_477,
        4.817_564,
      ),
      (
        Environment::Rural,
        7,
        18,
        40.0,
        116.0,
        14.0,
        35.997_350,
        8.993_940,
        4.082_968,
      ),
      (
        Environment::QuietRural,
        1,
        2,
        0.0,
        150.0,
        0.5,
        73.284_101,
        10.291_002,
        5.586_905,
      ),
      (
        Environment::City,
        12,
        4,
        -30.0,
        20.0,
        3.5,
        61.781_292,
        10.994_527,
        6.630_534,
      ),
    ];
    for (env, month, hour, lat, lon, f, fam, du, dl) in cases {
      let got = combined(env, month, f64::from(hour), lat, lon, f).unwrap();
      assert!(
        (got.fa_db - fam).abs() < 1e-5,
        "{env:?}/{month}: {} vs {fam}",
        got.fa_db
      );
      assert!(
        (got.du_db - du).abs() < 1e-5,
        "{env:?}/{month}: Du {} vs {du}",
        got.du_db
      );
      assert!(
        (got.dl_db - dl).abs() < 1e-5,
        "{env:?}/{month}: Dl {} vs {dl}",
        got.dl_db
      );
    }
  }

  /// 超出人为噪声曲线适用频段、或把「银河」当环境传入时不给结果（官方程序不限频段，本站不外推）。
  #[test]
  fn combined_refuses_what_the_man_made_curves_do_not_cover() {
    assert!(combined(Environment::City, 1, 0.0, 40.0, 116.0, 0.2).is_none());
    assert!(combined(Environment::QuietRural, 1, 0.0, 40.0, 116.0, 31.0).is_none());
    assert!(combined(Environment::Galactic, 1, 0.0, 40.0, 116.0, 7.0).is_none());
  }

  /// 银河噪声的 `c` / `d` 取自 `noise_model` 的曲线 E，必须与官方程序的 `52 − 23·lg f` 逐位相同，
  /// 且不受曲线 E 适用频段（10–100 MHz）的限制：官方合成在 0.1 MHz 也把它算进去。
  #[test]
  fn galactic_median_is_the_official_formula_at_every_frequency() {
    for f in [0.1, 0.475, 3.65, 10.0, 28.5, 30.0] {
      assert_eq!(galactic_db(f), 52.0 - 23.0 * f64::log10(f), "{f} MHz");
    }
  }

  /// 合成值不能比任何一个分量单独的中位低（合成是功率相加，只会更高）。
  #[test]
  fn combined_is_never_below_its_components() {
    let atm = atmospheric(&AtmNoiseInput {
      month: 7,
      hour_utc: 18.0,
      lat_deg: 40.0,
      lon_deg: 116.0,
      freq_mhz: 7.0,
    })
    .unwrap();
    let fa_m = crate::noise_model::fam_db(Environment::Residential, 7.0).unwrap();
    let fa_g = 52.0 - 23.0 * 7f64.log10(); // 这里故意手写一遍公式：对着 `galactic_db` 独立核对
    let c = combined(Environment::Residential, 7, 18.0, 40.0, 116.0, 7.0).unwrap();
    assert!(
      c.fa_db >= atm.fa_db - 0.5,
      "{} vs 大气 {}",
      c.fa_db,
      atm.fa_db
    );
    assert!(c.fa_db >= fa_m - 0.5, "{} vs 人为 {}", c.fa_db, fa_m);
    assert!(c.fa_db >= fa_g - 0.5, "{} vs 银河 {}", c.fa_db, fa_g);
  }

  /// 表里每一行与单点接口一致，且频率表全部落在三种曲线的共同适用范围内（任何环境都不出现空行）。
  #[test]
  fn band_table_matches_the_point_api_for_every_environment() {
    for env in [
      Environment::City,
      Environment::Residential,
      Environment::Rural,
      Environment::QuietRural,
    ] {
      let rows = band_table(env, 7, 12.0, 40.0, 116.0).expect("四种环境都应有完整一张表");
      assert_eq!(rows.len(), BAND_FREQS_MHZ.len());
      for r in &rows {
        let one = combined(env, 7, 12.0, 40.0, 116.0, r.freq_mhz).unwrap();
        assert_eq!(r.total, one);
        assert_eq!(
          r.man_made_db,
          crate::noise_model::fam_db(env, r.freq_mhz).unwrap()
        );
      }
    }
    assert!(band_table(Environment::City, 13, 12.0, 40.0, 116.0).is_none());
    assert!(band_table(Environment::Galactic, 7, 12.0, 40.0, 116.0).is_none());
  }

  /// 主导来源跟着中位最高的那一项（数值取自 `band_table` 的实际输出，核对过与单点接口一致）。
  /// 城市环境整张表都是人为噪声；安静乡村夏季傍晚低频是大气噪声、高频掉到银河噪声。
  #[test]
  fn dominant_source_follows_the_largest_median() {
    let city = band_table(Environment::City, 1, 0.0, 40.0, 116.0).unwrap();
    assert!(city.iter().all(|r| r.dominant() == NoiseSource::ManMade));
    let q = band_table(Environment::QuietRural, 7, 18.0, 40.0, 116.0).unwrap();
    assert_eq!(q.first().unwrap().dominant(), NoiseSource::Atmospheric);
    assert_eq!(q.last().unwrap().dominant(), NoiseSource::Galactic);
    // 冬季午夜的安静乡村：3.65 MHz 上银河只比大气高 1 dB，5.36 MHz 就翻回大气 —— 中间有交叉。
    let w = band_table(Environment::QuietRural, 1, 0.0, 40.0, 116.0).unwrap();
    let at = |f: f64| w.iter().find(|r| r.freq_mhz == f).unwrap().dominant();
    assert_eq!(at(3.65), NoiseSource::Galactic);
    assert_eq!(at(5.36), NoiseSource::Atmospheric);
  }
}
