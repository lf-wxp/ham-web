//! 外部噪声（ITU-R P.372）：人为噪声与银河噪声的**中位**噪声系数 `Fa`。
//!
//! # 它回答什么
//!
//! 接收天线端子处，除热噪声 `kT0b` 之外还有天线从空间里收到的噪声：人为的（电机、开关电源、
//! 点火……）、银河的，以及雷电引起的大气噪声。P.372 把它们统一写成「相对 `kT0b` 的 dB 数」，
//! 即外部噪声系数 `Fa`；可用噪声功率为 `Pn = Fa + 10·lg(b) − 204`（dBW，`b` 为带宽 Hz）。
//! 这与 [`crate::station_link::LinkInput::ext_noise_db`] 是同一个量，该字段此前只能由用户自填。
//!
//! 在 80 m（3.65 MHz）上，曲线给出城市 61.2 dB、住宅 56.9 dB、农村 51.6 dB、安静乡村 37.5 dB，
//! 都远高于热噪声；而 2 m（145.5 MHz）上城市只有 16.9 dB。所以 HF 上「接收机灵敏度只由 NF 决定」
//! 不成立，VHF 以上才是接收机 NF 要紧；这张表就是为了让这个数不必凭印象。
//!
//! # 公式与常数
//!
//! 人为噪声与银河噪声都是频率的对数线性函数（`f` 以 MHz 计）：
//!
//! ```text
//! Fam = c − d · lg f        （P.372-14 式 15 与式 13）
//! ```
//!
//! | 曲线 | 环境 | c | d | 适用频段（MHz） |
//! | --- | --- | --- | --- | --- |
//! | A | 城市 | 76.8 | 27.7 | 0.3–250 |
//! | B | 住宅 | 72.5 | 27.7 | 0.3–250 |
//! | C | 农村 | 67.2 | 27.7 | 0.3–250 |
//! | D | 安静乡村 | 53.6 | 28.6 | 0.3–30 |
//! | E | 银河 | 52.0 | 23.0 | 10–100 |
//!
//! 参考天线是**无损、接地的短垂直单极子**（图 39 的标题）。原文脚注 1：给出的是外部噪声系数
//! 中服从高斯分布的那一部分；人为噪声常带脉冲成分，对数字系统可能更要紧，这里不建模。
//!
//! # 出处与核对
//!
//! - **常数**取自 ITU-R P.372-14（2019-08）表 1，并与 P.372-6（1994）、P.372-7（2001）的官方
//!   PDF 逐行比对，三个版本的数值完全相同（曲线 A 的名称由 Business 改成了 City，数值没变）。
//!   现行版 P.372-18（2026-09）**没有直接对照**：ITU 网站拒绝脚本下载其 PDF。
//! - **图 39**：把官方 PDF 渲染成 300 dpi 图，沿五条曲线在 0.3–100 MHz 内量出 29 个点的
//!   实际 Fam，与上表公式相差不超过 1 dB（量出的最大差 0.99 dB，是图的标定误差量级；
//!   250 MHz 处的点被曲线端部的标签遮住，没读到）。这独立于文本，能抓住 `c`、`d` 抄错一位数的
//!   错误；其中 15 个点写进了 `matches_readings_off_figure_39`。
//! - **适用频段**：A–C 的 0.3–250 MHz 是原文文字（式 15 的说明）。**D 与 E 原文只写「如图所示」**：
//!   从图上量得 D 在 30 MHz 处结束、E（虚线）从 10 MHz 起。E 的上端图上画到约 150 MHz，
//!   而原文文字写的是「up to about 100 MHz」（式 13），这里取更严格的 100 MHz。
//!   银河噪声在低于 foF2 的频率上被电离层屏蔽、观测不到（原文 4.1 节），所以不给 10 MHz 以下。
//! - **超出范围一律返回 `None`，不外推**。
//!
//! # 与近年实测的差距（必须知道）
//!
//! 这些曲线来自 1970 年代的测量，原文称 2006–2007 年欧洲与 2009–2011 年日本的测量
//! 「基本印证」（表 3–4）。逐点比较（`recent_measurements_mostly_sit_below_the_curves`）：
//! 落在 A–C 范围内的 23 个实测点里，**21 个低于曲线**，平均低 4.3 dB，范围 −12.7～+3.5 dB；
//! 差得最多的是 35–37 MHz 的住宅 / 农村。所以城市 / 住宅 / 农村曲线在 VHF 上偏保守
//! （算出的外部噪声偏高）。这不是本模块的误差，而是标准曲线本身与近年测量的差距。
//!
//! # 没有做
//!
//! - **大气噪声**（雷电；P.372 图 13–36，随季节、时段、地点而变）：原文只给图，数值要用 ITU-R
//!   第 3 研究组网站提供的程序与系数文件。HF 上它可与人为噪声相当（原文第 7 部分），所以本模块的
//!   值**不是 HF 的总外部噪声**，只是其中的人为 + 银河两项之一；
//! - **多种噪声的合成**（第 7 部分，式 16–24）：公式已对着渲染页核实，但要的是各分量的 `σ_i`，
//!   原文没有写明中位数 `F_amT` 用上侧还是下侧的 `σ_i`，且没有大气噪声输入就合成不了，所以不做。
//!   **不要把两条曲线的 `Fa` 直接按功率相加当总噪声**；
//! - **随时间与地点的起伏**（表 2 的十分位偏差：城市上十分位 11.0 dB、地点 8.4 dB 等）：只给中位数，
//!   即一半的时间 / 地点噪声更高；
//! - 室内与天线靠近障碍物的情形（表 5）。

/// 数据对官方文本的核对日期（`YYYY-MM-DD`）。登记在 [`crate::staleness`]：转录的外部数据过了
/// 半年没人回去看就会在 `cargo make check` 里报过期。
pub const CHECKED_ON: &str = "2026-11-11";

/// P.372 的环境类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
  /// 城市（曲线 A；P.372-6 / -7 里叫 Business）。
  City,
  /// 住宅（曲线 B）。
  Residential,
  /// 农村（曲线 C）。
  Rural,
  /// 安静乡村（曲线 D）。
  QuietRural,
  /// 银河噪声（曲线 E）：理论下限，不是一种「环境」。
  Galactic,
}

impl Environment {
  /// 全部类别，顺序即图 39 的曲线 A–E。
  pub const ALL: [Self; 5] = [
    Self::City,
    Self::Residential,
    Self::Rural,
    Self::QuietRural,
    Self::Galactic,
  ];

  /// 该类别的曲线。
  #[must_use]
  pub const fn curve(self) -> Curve {
    match self {
      Self::City => Curve::new('A', 76.8, 27.7, 0.3, 250.0),
      Self::Residential => Curve::new('B', 72.5, 27.7, 0.3, 250.0),
      Self::Rural => Curve::new('C', 67.2, 27.7, 0.3, 250.0),
      Self::QuietRural => Curve::new('D', 53.6, 28.6, 0.3, 30.0),
      Self::Galactic => Curve::new('E', 52.0, 23.0, 10.0, 100.0),
    }
  }
}

/// 一条 `Fam = c − d·lg f` 曲线及其适用频段。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Curve {
  /// P.372 图 39 的曲线字母（`'A'`–`'E'`）。
  pub letter: char,
  /// 常数 `c`（dB）。
  pub c: f64,
  /// 常数 `d`（dB/十倍频程）。
  pub d: f64,
  /// 适用频段下端（MHz，含）。
  pub lo_mhz: f64,
  /// 适用频段上端（MHz，含）。
  pub hi_mhz: f64,
}

impl Curve {
  const fn new(letter: char, c: f64, d: f64, lo_mhz: f64, hi_mhz: f64) -> Self {
    Self {
      letter,
      c,
      d,
      lo_mhz,
      hi_mhz,
    }
  }

  /// `freq_mhz` 是否在适用频段内（含两端）。NaN 与 ±∞ 都不在。
  #[must_use]
  pub fn contains(&self, freq_mhz: f64) -> bool {
    (self.lo_mhz..=self.hi_mhz).contains(&freq_mhz)
  }
}

/// 外部噪声系数的**中位数** `Fam`（dB，相对 `kT0b`）。
///
/// 频率不在该曲线的适用频段内（含 NaN / ±∞）时返回 `None`，**不外推**。在各曲线的适用频段内
/// 结果恒不小于 0.77 dB（农村曲线在 250 MHz），满足 [`crate::station_link`] 对 `Fa ≥ 0` 的前提。
#[must_use]
pub fn fam_db(env: Environment, freq_mhz: f64) -> Option<f64> {
  let curve = env.curve();
  curve
    .contains(freq_mhz)
    .then(|| curve.c - curve.d * freq_mhz.log10())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn constants_match_table_1_of_the_official_text() {
    // 逐行抄自 P.372-14 表 1（与 -6 / -7 相同），连同适用频段：A–C 的 0.3–250 MHz 是原文文字，
    // D 的 30 MHz 与 E 的 10 / 100 MHz 见模块文档。这条只防「有人顺手改了一个数」；
    // 抄错的检验在 `matches_readings_off_figure_39`。
    let table = [
      (Environment::City, 'A', 76.8, 27.7, 0.3, 250.0),
      (Environment::Residential, 'B', 72.5, 27.7, 0.3, 250.0),
      (Environment::Rural, 'C', 67.2, 27.7, 0.3, 250.0),
      (Environment::QuietRural, 'D', 53.6, 28.6, 0.3, 30.0),
      (Environment::Galactic, 'E', 52.0, 23.0, 10.0, 100.0),
    ];
    for (env, letter, c, d, lo, hi) in table {
      let curve = env.curve();
      assert_eq!(
        (curve.letter, curve.c, curve.d, curve.lo_mhz, curve.hi_mhz),
        (letter, c, d, lo, hi),
        "{env:?}"
      );
    }
    // ALL 的顺序就是曲线字母的顺序，页面与文档都靠它。
    let letters: Vec<char> = Environment::ALL.iter().map(|e| e.curve().letter).collect();
    assert_eq!(letters, ['A', 'B', 'C', 'D', 'E']);
  }

  #[test]
  fn formula_values_from_an_independent_script() {
    // Python 独立算的（不共用这里的代码路径）：`c − d·log10(f)`。
    let cases = [
      (Environment::City, 0.3, 91.283_741_244),
      (Environment::City, 14.2, 44.881_612_861),
      (Environment::City, 145.5, 16.888_695_085),
      (Environment::City, 250.0, 10.377_061_760),
      (Environment::Residential, 14.2, 40.581_612_861),
      (Environment::Residential, 145.5, 12.588_695_085),
      (Environment::Rural, 14.2, 35.281_612_861),
      (Environment::Rural, 250.0, 0.777_061_760),
      (Environment::QuietRural, 0.3, 68.554_332_115),
      (Environment::QuietRural, 14.2, 20.644_553_351),
      (Environment::QuietRural, 30.0, 11.354_332_115),
      (Environment::Galactic, 10.0, 29.0),
      (Environment::Galactic, 14.2, 25.497_368_079),
      (Environment::Galactic, 100.0, 6.0),
    ];
    for (env, f, want) in cases {
      let got = fam_db(env, f).expect("在适用频段内");
      assert!(
        (got - want).abs() < 1e-8,
        "{env:?} @ {f} MHz：{got} ≠ {want}"
      );
    }
  }

  #[test]
  fn matches_readings_off_figure_39() {
    // 从官方 PDF 的图 39 渲染图上量出的实际 Fam（300 dpi，沿曲线取粗黑线中心，
    // 坐标按图框与网格线标定）：`(环境, MHz, 图上读数 dB)`。与文本里的 c、d 相互独立，
    // 所以能抓住抄错一位数；容限 1.2 dB 对应读图本身的标定误差（实测最大差 0.99 dB）。
    let readings = [
      (Environment::City, 1.0, 76.48),
      (Environment::City, 10.0, 48.82),
      (Environment::City, 100.0, 21.19),
      (Environment::Residential, 1.0, 71.87),
      (Environment::Residential, 10.0, 44.16),
      (Environment::Residential, 100.0, 16.58),
      (Environment::Rural, 1.0, 66.92),
      (Environment::Rural, 10.0, 38.92),
      (Environment::Rural, 100.0, 10.91),
      (Environment::QuietRural, 1.0, 53.38),
      (Environment::QuietRural, 10.0, 25.25),
      (Environment::QuietRural, 28.0, 12.94),
      (Environment::Galactic, 11.0, 28.05),
      (Environment::Galactic, 30.0, 17.94),
      (Environment::Galactic, 90.0, 6.13),
    ];
    for (env, f, read) in readings {
      let formula = fam_db(env, f).expect("在适用频段内");
      assert!(
        (formula - read).abs() < 1.2,
        "{env:?} @ {f} MHz：公式 {formula:.2} dB，图上读数 {read} dB"
      );
    }
  }

  #[test]
  fn outside_the_validity_range_there_is_no_value() {
    // 每条曲线：两端含在内，外侧一点点就没有；不外推。
    for env in Environment::ALL {
      let c = env.curve();
      assert!(fam_db(env, c.lo_mhz).is_some(), "{env:?} 下端含");
      assert!(fam_db(env, c.hi_mhz).is_some(), "{env:?} 上端含");
      assert_eq!(fam_db(env, c.lo_mhz * 0.999), None, "{env:?} 下端以外");
      assert_eq!(fam_db(env, c.hi_mhz * 1.001), None, "{env:?} 上端以外");
    }
    // 具体到会被问到的场景：2 m（145.5 MHz）上安静乡村与银河噪声都没有定义；
    // 160 m（1.9 MHz）上银河噪声被电离层屏蔽、也不给。
    assert_eq!(fam_db(Environment::QuietRural, 145.5), None);
    assert_eq!(fam_db(Environment::Galactic, 145.5), None);
    assert_eq!(fam_db(Environment::Galactic, 1.9), None);
    assert!(fam_db(Environment::City, 145.5).is_some());
    // 非有限频率不出值（NaN 与任何数比较都是 false，必须显式挡住）。
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -14.2] {
      for env in Environment::ALL {
        assert_eq!(fam_db(env, bad), None, "{env:?} @ {bad}");
      }
    }
  }

  #[test]
  fn never_negative_inside_the_ranges() {
    // `station_link` 要求 Fa ≥ 0（冷天空不在内）。曲线随频率单调下降，最小值在上端。
    for env in Environment::ALL {
      let curve = env.curve();
      let mut f = curve.lo_mhz;
      while f <= curve.hi_mhz {
        let fa = fam_db(env, f).expect("在适用频段内");
        assert!(fa >= 0.77, "{env:?} @ {f} MHz：Fa = {fa}");
        f *= 1.05;
      }
      assert!(fam_db(env, curve.hi_mhz).expect("上端") >= 0.77);
    }
  }

  #[test]
  fn each_curve_falls_with_frequency_and_environments_are_ordered() {
    for env in Environment::ALL {
      let curve = env.curve();
      let (lo, hi) = (curve.lo_mhz, curve.hi_mhz);
      assert!(
        fam_db(env, lo).unwrap() > fam_db(env, hi).unwrap(),
        "{env:?}"
      );
    }
    // 同一频率上，越嘈杂的环境 Fa 越大：城市 > 住宅 > 农村 > 安静乡村（在 D 的范围内比较）。
    let at = |e| fam_db(e, 14.2).expect("14.2 MHz 四条曲线都有定义");
    assert!(at(Environment::City) > at(Environment::Residential));
    assert!(at(Environment::Residential) > at(Environment::Rural));
    assert!(at(Environment::Rural) > at(Environment::QuietRural));
    // 城市比住宅高 4.3 dB、住宅比农村高 5.3 dB（c 之差，d 相同所以与频率无关）。
    assert!((at(Environment::City) - at(Environment::Residential) - 4.3).abs() < 1e-9);
    assert!((at(Environment::Residential) - at(Environment::Rural) - 5.3).abs() < 1e-9);
  }

  #[test]
  fn hf_noise_is_far_above_thermal_and_vhf_is_not() {
    // 模块文档开头那段话的出处：80 m（3.65 MHz）上各环境的 Fa（Python 独立算的）。
    let at = |e| fam_db(e, 3.65).expect("3.65 MHz 四条曲线都有定义");
    assert!((at(Environment::City) - 61.22).abs() < 0.01);
    assert!((at(Environment::Residential) - 56.92).abs() < 0.01);
    assert!((at(Environment::Rural) - 51.62).abs() < 0.01);
    assert!((at(Environment::QuietRural) - 37.52).abs() < 0.01);
    // 反过来 2 m 上城市只有 16.9 dB：这就是 VHF 以上「接收机 NF 才要紧」的原因。
    assert!((fam_db(Environment::City, 145.5).unwrap() - 16.89).abs() < 0.01);
  }

  /// P.372-14 表 3（欧洲，2006–2007）与表 4（日本，2009–2011）里落在 A–C 范围内的实测中位数：
  /// `(环境, MHz, 实测 Fa dB)`。表 3 另有 270 / 425 MHz、表 5 是室内，都在范围之外，不收。
  const MEASURED: [(Environment, f64, f64); 23] = [
    // 表 3：欧洲
    (Environment::City, 35.0, 23.0),
    (Environment::Residential, 35.0, 17.0),
    (Environment::Rural, 35.0, 16.0),
    (Environment::City, 140.0, 12.0),
    (Environment::Residential, 140.0, 8.0),
    (Environment::Rural, 140.0, 6.0),
    (Environment::City, 210.0, 16.0),
    (Environment::Residential, 210.0, 8.0),
    (Environment::Rural, 210.0, 5.0),
    // 表 4：日本（只有城市与住宅）
    (Environment::City, 37.0, 27.1),
    (Environment::Residential, 37.0, 20.2),
    (Environment::City, 67.0, 21.4),
    (Environment::Residential, 67.0, 17.1),
    (Environment::City, 75.0, 21.1),
    (Environment::Residential, 75.0, 15.2),
    (Environment::City, 99.0, 18.6),
    (Environment::Residential, 99.0, 11.1),
    (Environment::City, 121.0, 15.5),
    (Environment::Residential, 121.0, 10.3),
    (Environment::City, 163.0, 13.0),
    (Environment::Residential, 163.0, 9.1),
    (Environment::City, 222.0, 9.0),
    (Environment::Residential, 222.0, 6.8),
  ];

  #[test]
  fn recent_measurements_mostly_sit_below_the_curves() {
    // 把模块文档里「23 个点 21 个低于曲线、平均低 4.3 dB、−12.7～+3.5 dB」钉成断言：
    // 既是文档数字的出处，也在有人改动 c、d 时让这个差距显形。
    let diffs: Vec<f64> = MEASURED
      .iter()
      .map(|&(env, f, measured)| measured - fam_db(env, f).expect("在 A–C 范围内"))
      .collect();
    assert_eq!(diffs.len(), 23);
    let below = diffs.iter().filter(|&&d| d < 0.0).count();
    let mean = diffs.iter().sum::<f64>() / diffs.len() as f64;
    let (min, max) = diffs
      .iter()
      .fold((f64::MAX, f64::MIN), |(lo, hi), &d| (lo.min(d), hi.max(d)));
    assert_eq!(below, 21, "低于曲线的点数");
    assert!((mean + 4.3).abs() < 0.05, "平均差 {mean:.2}");
    assert!((min + 12.7).abs() < 0.05, "最低 {min:.2}");
    assert!((max - 3.5).abs() < 0.05, "最高 {max:.2}");
  }
}
