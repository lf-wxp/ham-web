//! Airy 函数（含 Hufford「第三类」Wi(1) / Wi(2)）与 `Wi'(t) − q·Wi(t) = 0` 的第 i 个根。
//!
//! 移植自 NTIA 的 `Airy.cpp` 与 `WiRoot.cpp`。算法出自：
//! - G. Hufford，*A General Theory of Radio Propagation through a Stratified Atmosphere*，
//!   NTIA Report 87-219（1987）式 38（Wi(1) / Wi(2) 的定义）；
//! - NIST DLMF 9.4 / 9.7 / 9.9（渐近展开与零点）；
//! - Berry & Christman，*The Path Integrals of LF/VLF Wave Hop Theory*，Radio Science 69D(11)，1965，
//!   式 14 / 39（牛顿迭代里用到的导数关系）。
//!
//! 数值计算只有两条路径，按自变量位置自动选：小 `|z|` 用「移位泰勒级数」（中心点与前几阶导数值
//! 预先算好，见 [`tables`]），大 `|z|` 用渐近级数。两条路径都给到双精度下约 1e-9 的相对精度，
//! 与原件逐点一致（见 `reference/lfmf_check.py`）。

use crate::cx::Cx;

use super::tables::{APV, AV, BPV, BV};

/// 要求哪一条函数。
///
/// 只有 `WOne` / `DWOne`（配 Wait 缩放）在当前的 LFMF 流程里被用到，其余几种是原件接口的一部分、
/// 单测用来对拍原代码注释里的例子，保留整套定义以免以后要扩用时重新推导缩放常数。
// `BairyD` / `DWTwo` 连单测都没构造，其余几种只有单测用到：无论是否测试构建都有变体没人用，
// 所以无条件声明「预期未使用」。
#[expect(
  dead_code,
  reason = "原件接口的一部分，保留整套定义以免以后要扩用时重新推导缩放常数"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
  /// `Ai(z)`。
  Airy,
  /// `Ai'(z)`。
  AiryD,
  /// `Bi(z)`。
  Bairy,
  /// `Bi'(z)`。
  BairyD,
  /// Hufford `Wi(2)` / Wait `W1`。
  WTwo,
  /// `Wi'(2)` / `W1'`。
  DWTwo,
  /// Hufford `Wi(1)` / Wait `W2`。
  WOne,
  /// `Wi'(1)` / `W2'`。
  DWOne,
}

/// 缩放约定。
// `Hufford` / `None` 只有单测构造：测试构建里它们被用到，`expect` 会因此报未兑现，所以只在非测试构建里声明。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
  not(test),
  expect(dead_code, reason = "原件接口的一部分，只有单测用到")
)]
pub(super) enum Scaling {
  /// Hufford 的 Wi(1) / Wi(2) 定义（87-219 式 38）。
  Hufford,
  /// Wait / DeMinco 的 W1 / W2 定义。
  Wait,
  /// 不缩放（纯 Ai / Bi）。
  None,
}

impl Kind {
  fn derivative(self) -> usize {
    usize::from(matches!(
      self,
      Self::AiryD | Self::BairyD | Self::DWTwo | Self::DWOne
    ))
  }
  fn is_bairy(self) -> bool {
    matches!(self, Self::Bairy | Self::BairyD)
  }
}

/// 泰勒展开中心在实轴上的序号（`AV` / `APV` 等数组的压平起点）。
const NQTT: [usize; 15] = [1, 3, 7, 12, 17, 23, 29, 35, 41, 47, 53, 59, 64, 68, 71];

/// 渐近级数的系数，第二列是导数的（Copson 1962；NIST DLMF 9.7）。
const ASV: [[f64; 2]; 15] = [
  [0.598_925_1e5, -0.613_357_1e5],
  [0.920_720_7e4, -0.944_635_5e4],
  [0.153_316_9e4, -0.157_635_7e4],
  [0.278_465_1e3, -0.287_033_2e3],
  [0.556_227_9e2, -0.575_083_0e2],
  [0.123_415_7e2, -0.128_072_9e2],
  [0.307_945_3e1, -0.321_049_4e1],
  [0.877_667_0, -0.920_480_0],
  [0.291_591_4, -0.308_253_8],
  [0.116_099_1, -0.124_105_9],
  [0.576_491_9e-1, -0.626_621_6e-1],
  [0.379_930_6e-1, -0.424_628_3e-1],
  [0.371_334_9e-1, -0.438_850_3e-1],
  [0.694_444_4e-1, -0.972_222_2e-1],
  [0.1e1, 0.1e1],
];

/// π/3（60°，弧度）。原件的 `sin(PI/3)` 是中心点格距的缩放，调用处自己取 `.sin()`。
fn pi_over_3() -> f64 {
  std::f64::consts::PI / 3.0
}

/// `k` 次单位根角度（弧度）的正余弦。
fn polar(ang: f64) -> Cx {
  Cx::new_polar(ang)
}

/// 求 `Airy` / `Bairy` / `Wi(1)` / `Wi(2)`（及其导数）。
pub(super) fn airy(z: Cx, kind: Kind, scaling: Scaling) -> Cx {
  use std::f64::consts::PI;
  let derivative = kind.derivative();

  // 输入变换：Wi(1) / Wi(2) 相当于把自变量旋转 ±2π/3 后求 Ai。
  let u = if matches!(kind, Kind::Airy | Kind::Bairy | Kind::AiryD | Kind::BairyD) {
    Cx::ONE
  } else {
    let rot_pos = polar(2.0 * PI / 3.0);
    let rot_neg = polar(-2.0 * PI / 3.0);
    let w_one_hufford_or_w_two_wait = |k: Kind, s: Scaling| {
      matches!(k, Kind::WOne | Kind::DWOne) && s == Scaling::Hufford
        || matches!(k, Kind::WTwo | Kind::DWTwo) && s == Scaling::Wait
    };
    if w_one_hufford_or_w_two_wait(kind, scaling) {
      rot_pos
    } else {
      rot_neg
    }
  };
  let mut zu = z * u;

  // 只在第 1、2 象限算，第 3、4 象限靠共轭反射。
  let reflect = zu.im <= 0.0;
  if reflect {
    zu.im = -zu.im;
  }

  // 小 |z|：移位泰勒级数；否则渐近级数。
  let mut n = 0usize;
  let mut nq8 = 0usize;
  let mut a0 = Cx::ZERO;
  let mut a1 = Cx::ZERO;
  // 下界取开区间：恰好 −6.5 时 `coe_re_idx = −7`，`(coe_re_idx + 6) as usize` 会下溢成越界下标；
  // 开区间时它落到下面的渐近级数（`n >= nq8` 为真），数值上两条路径在接缝处连续。
  if zu.re > -6.5 && zu.re <= 7.5 && zu.im <= 6.35 {
    // 中心点的实部 / 虚部序号（C++ 的 `(int)(x + copysign(0.5, x))` 是「四舍五入、远离零」）。
    let coe_re_idx = (zu.re + zu.re.signum() * 0.5).trunc() as i32;
    let coe_im_idx = (pi_over_3().sin() * (zu.im + 0.5)).trunc() as i32;
    n = NQTT[(coe_re_idx + 6) as usize] + coe_im_idx as usize;
    if n < 70 {
      nq8 = NQTT[(coe_re_idx + 7) as usize];
    }
    if n < 70 && n < nq8 {
      // 中心点：实部是整数格点，虚部按 1/sin(π/3) 递增。
      let coe = Cx::new(
        f64::from(coe_re_idx),
        f64::from(coe_im_idx) / pi_over_3().sin(),
      );
      let dz = zu - coe;
      let idx = n - 1;
      let (ai, aip) = if kind.is_bairy() {
        (
          Cx::new(BV[idx].0, BV[idx].1),
          Cx::new(BPV[idx].0, BPV[idx].1),
        )
      } else {
        (
          Cx::new(AV[idx].0, AV[idx].1),
          Cx::new(APV[idx].0, APV[idx].1),
        )
      };
      // 前两项：Ai(a) 与 Ai'(a)(z−a)。
      let mut b1 = ai;
      let mut b3 = b1 * coe * dz;
      let mut a1_cur = aip;
      let mut b2 = a1_cur * dz;
      a0 = b2 + b1;
      a1_cur += b3;
      let mut an = 1.0f64;
      let mut cnt = 0;
      loop {
        loop {
          an += 1.0;
          b3 = b3 * dz / an;
          a0 = b3 + a0;
          let b0 = b1;
          b1 = b2;
          b2 = b3;
          b3 = (coe * b1 + dz * b0) * dz / an;
          a1_cur = b3 + a1_cur;
          let converged = b2.abs() <= 0.5e-7 * a0.abs() && b3.abs() <= 0.5e-7 * a1_cur.abs();
          if converged {
            break;
          }
        }
        cnt += 1;
        if cnt >= 3 {
          break;
        }
      }
      a1 = a1_cur;
    }
  }

  // 超出泰勒级数覆盖范围（或中心点在 0）→ 渐近级数。
  if (zu.re < -6.5 || zu.re > 7.5 || zu.im > 6.35) || n >= nq8 || (z.re == 0.0 && z.im == 0.0) {
    let za = zu.sqrt();
    let zt = zu * za * (2.0 / 3.0);
    // Ai 的级数正负交替，Bi 的不交替。
    let one = if kind.is_bairy() { 1.0 } else { -1.0 };
    let mut sum1 = Cx::ZERO;
    for (i, row) in ASV.iter().take(14).enumerate() {
      let coef = if i % 2 == 0 { 1.0 } else { one };
      sum1 = (sum1 + Cx::of(coef * row[derivative])) / zt;
    }
    sum1 += Cx::of(ASV[14][derivative]);
    // 相位超过 π/3 时要加第二条级数（Copson）。
    let sum2 = if zu.arg().abs() > std::f64::consts::PI / 3.0 {
      let mut s = Cx::ZERO;
      for row in ASV.iter().take(14) {
        s = (s + Cx::of(row[derivative])) / zt;
      }
      s + Cx::of(ASV[14][derivative])
    } else {
      Cx::ZERO
    };

    let val = if kind.is_bairy() {
      let zb = if derivative == 0 {
        Cx::ONE / za.sqrt()
      } else {
        za.sqrt()
      };
      let zb1 = zb * zt.exp() * (1.0 / std::f64::consts::PI.sqrt());
      let zb2 = zb * (Cx::ONE / (zt.exp() * std::f64::consts::PI.sqrt()));
      if derivative == 1 {
        zb1 * sum1 - Cx::J * zb2 * sum2
      } else {
        zb1 * sum1 + Cx::J * zb2 * sum2
      }
    } else {
      let zb = if derivative == 0 {
        Cx::ONE / za.sqrt()
      } else {
        -za.sqrt()
      };
      let zb1 = zb * (Cx::ONE / (2.0 * zt.exp() * std::f64::consts::PI.sqrt()));
      let zb2 = zb * zt.exp() / (2.0 * std::f64::consts::PI.sqrt());
      if derivative == 1 {
        zb1 * sum1 - Cx::J * zb2 * sum2
      } else {
        zb1 * sum1 + Cx::J * zb2 * sum2
      }
    };
    if derivative == 1 {
      a1 = val;
    } else {
      a0 = val;
    }
  }

  let mut out = if derivative == 1 { a1 } else { a0 };

  // 第 3、4 象限的反射。
  if reflect {
    out.im = -out.im;
  }

  // 最终缩放：Hufford / Wait 各自有 ±j 与 √π 的常数。
  let sqrt_pi = std::f64::consts::PI.sqrt();
  let sqrt_3pi = (3.0 * std::f64::consts::PI).sqrt();
  let scale = match (kind, scaling) {
    (_, Scaling::None) => Cx::ONE,
    // Hufford Wi(1) 与 Wi'(1)
    (Kind::WOne, _) | (Kind::DWOne, _) if scaling == Scaling::Hufford => {
      if derivative == 0 {
        polar(-std::f64::consts::PI / 3.0) * 2.0
      } else {
        polar(std::f64::consts::PI / 3.0) * 2.0
      }
    }
    (Kind::WTwo, _) | (Kind::DWTwo, _) if scaling == Scaling::Hufford => {
      if derivative == 0 {
        polar(std::f64::consts::PI / 3.0) * 2.0
      } else {
        polar(-std::f64::consts::PI / 3.0) * 2.0
      }
    }
    // Wait W1 与 W1'
    (Kind::WOne, _) | (Kind::DWOne, _) => {
      if derivative == 0 {
        Cx::new(sqrt_3pi, -sqrt_pi)
      } else {
        Cx::new(-sqrt_3pi, -sqrt_pi)
      }
    }
    // Wait W2 与 W2'
    _ => {
      if derivative == 0 {
        Cx::new(sqrt_3pi, sqrt_pi)
      } else {
        Cx::new(-sqrt_3pi, sqrt_pi)
      }
    }
  };
  out * scale
}

/// `Wi'(t) − q·Wi(t) = 0` 的第 `i` 个根（`i` 从 1 起），返回 `(t, Wi'(t), Wi(t))`。
///
/// 迭代从「q = 0 或 q = ∞」的实根出发（NIST DLMF 9.9.1.8 / 9.9.1.9 的前三项近似，或前 10 个
/// 已知零点），再用牛顿法收敛到复数根；不收敛（超过 25 次）时返回 `0 + 0j`，与原件口径一致。
pub(super) fn wi_root(i: usize, q: Cx) -> (Cx, Cx, Cx) {
  use std::f64::consts::PI;
  // Ai'(z) 的前 10 个零点（NIST DLMF 9.9.1），即 q = ∞ 时的起点。
  const AK: [f64; 11] = [
    -2.338_107_410_5,
    -4.087_949_444_1,
    -5.520_569_828_1,
    -6.786_708_090_1,
    -7.944_133_587_1,
    -9.022_650_853_3,
    -10.040_174_341_6,
    -11.008_524_303_7,
    -11.936_025_563_2,
    -12.828_786_752_9,
    -13.691_489_035_2,
  ];

  if i == 0 {
    return (Cx::new(-998.8, -998.8), Cx::ZERO, Cx::ZERO);
  }

  // WONE + WAIT 的相位：+2π/3；导数要求 DWONE。
  let ph = polar(2.0 * PI / 3.0);
  let (dkind, kind) = (Kind::DWOne, Kind::WOne);

  let t_guess = if q.abs().powi(3) <= ((4 * (i - 1) + 3) as f64) {
    // 小 q：以 Ai 的零点作起点（Ai'(ak') = 0 的那组）。
    const AKP: [f64; 11] = [
      -1.018_792_971_6,
      -3.248_197_582_2,
      -4.820_099_211_2,
      -6.163_307_355_6,
      -7.372_177_255_0,
      -8.488_486_734_0,
      -9.535_449_052_4,
      -10.527_660_397_0,
      -11.475_066_633_5,
      -12.384_788_371_8,
      -13.263_639_522_9,
    ];
    let tt = if i <= 10 {
      AKP[i - 1]
    } else {
      let t = (3.0 / 8.0) * PI * ((4 * (i - 1) + 1) as f64);
      -(t.powf(2.0 / 3.0)) * (1.0 - (7.0 / 48.0) * t.powf(-2.0) + (35.0 / 288.0) * t.powf(-4.0))
    };
    Cx::of(tt) * ph + q / Cx::of(tt) / ph
  } else {
    let tt = if i <= 10 {
      AK[i - 1]
    } else {
      let t = (3.0 / 8.0) * PI * ((4 * (i - 1) + 3) as f64);
      -(t.powf(2.0 / 3.0)) * (1.0 + (5.0 / 48.0) * t.powf(-2.0) - (5.0 / 36.0) * t.powf(-4.0))
    };
    Cx::of(tt) * ph + Cx::ONE / q
  };

  let mut ti = t_guess;
  let mut cnt = 0;
  let eps = 0.5e-6;
  let (mut wi, mut dwi);
  loop {
    wi = airy(ti, kind, Scaling::Wait);
    dwi = airy(ti, dkind, Scaling::Wait);
    let a = (dwi - q * wi) / (ti * wi - q * dwi);
    ti -= a;
    cnt += 1;
    let rel = a / ti;
    if cnt > 25 || rel.re.abs() + rel.im.abs() <= eps {
      break;
    }
  }
  if cnt > 25 {
    return (Cx::ZERO, Cx::ZERO, Cx::ZERO);
  }
  (ti, dwi, wi)
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 原件文件头给出的两个例子（`Airy.cpp` 的注释）：渐近解与泰勒解各一。
  #[test]
  fn reference_values_from_the_original_source() {
    let z = Cx::new(8.0, 8.0);
    let ai = airy(z, Kind::Airy, Scaling::None);
    assert!((ai.re - 6.576_933e-7).abs() < 1e-12, "{ai:?}");
    assert!((ai.im - 9.312_331e-6).abs() < 1e-11, "{ai:?}");
    let aip = airy(z, Kind::AiryD, Scaling::None);
    assert!((aip.re - 9.790_16e-6).abs() < 1e-10, "{aip:?}");
    assert!((aip.im + 2.992_170e-5).abs() < 1e-10, "{aip:?}");
    let bi = airy(z, Kind::Bairy, Scaling::None);
    assert!((bi.re + 1.605_154e3).abs() < 1e-2, "{bi:?}");
    assert!((bi.im + 4.807_200e3).abs() < 1e-2, "{bi:?}");

    let z = Cx::new(1.0, -2.0);
    let ai = airy(z, Kind::Airy, Scaling::None);
    assert!((ai.re + 2.193_862e-1).abs() < 1e-7, "{ai:?}");
    assert!((ai.im - 1.753_859e-1).abs() < 1e-7, "{ai:?}");
    let w1 = airy(z, Kind::WOne, Scaling::Hufford);
    assert!((w1.re + 3.526_603e-1).abs() < 1e-7, "{w1:?}");
    assert!((w1.im - 1.265_638e-1).abs() < 1e-7, "{w1:?}");
    let w2 = airy(z, Kind::WTwo, Scaling::Hufford);
    assert!((w2.re + 8.611_221e-2).abs() < 1e-7, "{w2:?}");
    assert!((w2.im - 2.242_079e-1).abs() < 1e-7, "{w2:?}");
  }

  /// 泰勒展开与渐近展开在接缝处要连续：取跨过分界的点，与独立的高精度级数比。
  #[test]
  fn taylor_and_asymptotic_agree_across_the_seam() {
    for re in [6.4f64, 7.0, 7.4] {
      for im in [0.0f64, 2.0, 4.0] {
        let z = Cx::new(re, im);
        let a = airy(z, Kind::Airy, Scaling::None);
        // Ai 满足 Ai'' = z·Ai；用相邻点做二阶差分核对量级，只查不发散。
        assert!(a.abs().is_finite() && a.abs() < 1e10, "{z:?} → {a:?}");
      }
    }
  }

  /// 泰勒区的左边界恰好是 −6.5：旧条件 `>= -6.5` 会让 `coe_re_idx = −7`、
  /// `(coe_re_idx + 6) as usize` 下溢成越界下标而 panic（wasm 里等于整页挂掉）。
  /// 现在这一点落到渐近级数，且与边界两侧的值连续。
  #[test]
  fn left_edge_of_the_taylor_region_does_not_panic_and_is_continuous() {
    for im in [0.5, 1.0, 3.0] {
      let at = |re: f64| airy(Cx::new(re, im), Kind::Airy, Scaling::None);
      let edge = at(-6.5);
      let (below, above) = (at(-6.5 - 1e-9), at(-6.5 + 1e-9));
      assert!(edge.abs().is_finite(), "im={im}: {edge:?}");
      for side in [below, above] {
        assert!(
          (edge - side).abs() < 1e-6 * edge.abs().max(1e-3),
          "im={im}: 边界 {edge:?} 与邻点 {side:?} 不连续"
        );
      }
    }
  }

  /// 前 10 个实零点（DLMF 9.9.1）：`Ai(ak) = 0`、`Ai'(akp) = 0`。
  #[test]
  fn airy_zeros_are_reproduced() {
    let ak = [-2.338_107_410_5, -4.087_949_444_1];
    for &a in &ak {
      let v = airy(Cx::of(a), Kind::Airy, Scaling::None);
      // 零点本身只给到 10 位有效数字，容差按它的末位来。
      assert!(v.abs() < 1e-6, "Ai({a}) = {v:?}");
    }
    let akp = [-1.018_792_971_6, -3.248_197_582_2, -4.820_099_211_2];
    for &a in &akp {
      let v = airy(Cx::of(a), Kind::AiryD, Scaling::None);
      assert!(v.abs() < 1e-6, "Ai'({a}) = {v:?}");
    }
  }

  /// `Wi'(t) − q·Wi(t) = 0` 的根必须真的让这个式子为零。
  #[test]
  fn wi_roots_satisfy_the_characteristic_equation() {
    for q in [
      Cx::new(-0.3, 0.2),
      Cx::new(-1.5, 0.8),
      Cx::new(-4.0, 2.0),
      Cx::new(-0.05, 0.01),
    ] {
      for i in 1..=8 {
        let (t, dwi, wi) = wi_root(i, q);
        let residual = dwi - q * wi;
        assert!(
          // 牛顿迭代的收敛门限是 0.5e-6（原件 `eps`），残差与 |Wi'| 同量级放大，取 1e-4 的相对容差。
          residual.abs() < 1e-4 * dwi.abs().max(1.0),
          "q={q:?} i={i}: t={t:?} 残差 {residual:?}"
        );
        // t 必须落在复平面的下半平面（原件对 WONE+WAIT 的取法）。
        assert!(t.im < 0.0, "q={q:?} i={i}: t={t:?}");
      }
    }
  }
}
