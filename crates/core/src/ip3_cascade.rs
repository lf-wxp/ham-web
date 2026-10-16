//! 三阶截点（IP3）级联：多级放大 / 衰减 / 混频链的总增益与总 IIP3 / OIP3，
//! 以及每一级对总互调的贡献（谁是瓶颈）。
//!
//! 公式（线性毫瓦域，各级相位对齐的**最坏情况**相干叠加）：
//!
//! ```text
//! 1 / IIP3_总 = Σᵢ  G(前 i−1 级累计) / IIP3ᵢ
//! OIP3_总     = IIP3_总 × G_总
//! ```
//!
//! 也就是说：越靠后的级，它的 IIP3 被前面所有级的增益「放大」后才折算到输入端，
//! 所以前级增益越大，后级越容易成为瓶颈。
//!
//! 局限（界面也要写明）：
//! - 相干叠加是最坏情况；实际各级互调产物相位不一定对齐，总 IIP3 往往略好于这个值；
//! - 只对三阶（两音）互调；没有 P1dB、二阶截点、噪声系数（噪声级联见噪声系数级联计算器）；
//! - 假定各级工作在线性区、输入功率远低于压缩点。

/// 一级的 IP3 是按输入还是输出给的。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Referred {
  /// IIP3（折算到本级输入）。
  Input,
  /// OIP3（折算到本级输出）：IIP3 = OIP3 − 本级增益。
  Output,
}

/// 一级。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stage {
  /// 增益（dB；衰减器、滤波器、线缆填负值）。
  pub gain_db: f64,
  /// 三阶截点（dBm）。`None` = 理想线性（无源器件），对总 IP3 没有贡献。
  pub ip3_dbm: Option<f64>,
  pub referred: Referred,
}

/// 一级的结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StageResult {
  /// 本级输入处的累计增益（dB，即前面所有级之和）。
  pub gain_before_db: f64,
  /// 折算到本级输入的 IIP3（dBm）；理想线性为 `None`。
  pub iip3_dbm: Option<f64>,
  /// 本级对 1/IIP3_总 的贡献占比（0–1，各级之和为 1）；理想线性为 0。
  pub share: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cascade {
  pub gain_db: f64,
  /// 总 IIP3（dBm）；所有级都是理想线性时为 `None`。
  pub iip3_dbm: Option<f64>,
  pub oip3_dbm: Option<f64>,
  pub stages: Vec<StageResult>,
  /// 贡献最大的一级（从 0 起的序号）；并列取最靠前的；没有非线性级时为 `None`。
  pub limiting: Option<usize>,
}

/// 输入不合法的原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ip3Error {
  /// 一级也没有。
  Empty,
  /// 某级的增益或 IP3 不是有限数，或各级增益累积到中间量溢出（序号从 0 起）。
  NotFinite(usize),
}

/// 线性（毫瓦）。
fn mw(dbm: f64) -> f64 {
  10f64.powf(dbm / 10.0)
}

fn dbm(mw: f64) -> f64 {
  10.0 * mw.log10()
}

/// 计算级联。
///
/// # Errors
///
/// 没有任何一级，或某个数值不是有限数。
pub fn cascade(stages: &[Stage]) -> Result<Cascade, Ip3Error> {
  if stages.is_empty() {
    return Err(Ip3Error::Empty);
  }
  for (i, s) in stages.iter().enumerate() {
    if !s.gain_db.is_finite() || s.ip3_dbm.is_some_and(|v| !v.is_finite()) {
      return Err(Ip3Error::NotFinite(i));
    }
  }

  let mut gain_before_db = 0.0;
  let mut inv_total = 0.0; // 1 / IIP3_总（1/mW）
  let mut terms = Vec::with_capacity(stages.len());
  let mut per = Vec::with_capacity(stages.len());
  for (i, s) in stages.iter().enumerate() {
    // 统一折算到本级输入。
    let iip3_stage = s.ip3_dbm.map(|v| match s.referred {
      Referred::Input => v,
      Referred::Output => v - s.gain_db,
    });
    // 前面累计增益（线性）/ 本级 IIP3（mW）。用 dB 域再转线性，避免连乘溢出。
    let term = iip3_stage.map_or(0.0, |v| mw(gain_before_db - v));
    // 累计增益大到 `mw()` 溢出（总和约 3082 dB 以上）时，继续算下去会得到
    // `Some(-inf)` 与全 NaN 的 `share`，却仍返回 `Ok` —— 宁可报错，也不要静默给出
    // 一个看着像结果的数字。累计增益本身溢出也一并拦住。
    if !term.is_finite()
      || (iip3_stage.is_some() && term == 0.0)
      || !(inv_total + term).is_finite()
      || !(gain_before_db + s.gain_db).is_finite()
    {
      return Err(Ip3Error::NotFinite(i));
    }
    inv_total += term;
    terms.push(term);
    per.push((gain_before_db, iip3_stage));
    gain_before_db += s.gain_db;
  }

  let iip3_dbm = (inv_total > 0.0).then(|| -dbm(inv_total));
  let limiting = if inv_total > 0.0 {
    // `max_by` 对并列取最后一个，要取最靠前的，所以手写。
    let mut best = 0;
    for (i, t) in terms.iter().enumerate() {
      if *t > terms[best] {
        best = i;
      }
    }
    Some(best)
  } else {
    None
  };

  let results = per
    .into_iter()
    .zip(&terms)
    .map(|((gain_before_db, iip3_dbm), t)| StageResult {
      gain_before_db,
      iip3_dbm,
      share: if inv_total > 0.0 { t / inv_total } else { 0.0 },
    })
    .collect();

  Ok(Cascade {
    gain_db: gain_before_db,
    iip3_dbm,
    oip3_dbm: iip3_dbm.map(|v| v + gain_before_db),
    stages: results,
    limiting,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn st(gain_db: f64, ip3: Option<f64>) -> Stage {
    Stage {
      gain_db,
      ip3_dbm: ip3,
      referred: Referred::Input,
    }
  }

  fn near(a: f64, b: f64, tol: f64) {
    assert!((a - b).abs() <= tol, "{a} vs {b}");
  }

  #[test]
  fn two_stages_match_the_hand_calculation() {
    // 第 1 级 G = 10 dB、IIP3 = 10 dBm（10 mW）；第 2 级 IIP3 = 20 dBm（100 mW）。
    // 1/IIP3 = 1/10 + 10/100 = 0.2 /mW → 5 mW = 6.9897 dBm；两级贡献各 50%。
    let c = cascade(&[st(10.0, Some(10.0)), st(0.0, Some(20.0))]).unwrap();
    near(c.iip3_dbm.unwrap(), 6.989_700_043, 1e-6);
    near(c.gain_db, 10.0, 1e-12);
    near(c.oip3_dbm.unwrap(), 16.989_700_043, 1e-6);
    near(c.stages[0].share, 0.5, 1e-12);
    near(c.stages[1].share, 0.5, 1e-12);
    // 并列时取最靠前的一级。
    assert_eq!(c.limiting, Some(0));
  }

  /// 数值来自独立脚本（毫瓦域，不复用本模块任何代码）。
  #[test]
  fn a_receive_chain_with_passive_stages_matches_the_independent_script() {
    // 衰减器 −3 dB（无源）→ LNA 20 dB / IIP3 +5 → 滤波器 −6 dB（无源）→ 混频 +15 / +15 → IF 放大 +10 / +25
    let c = cascade(&[
      st(-3.0, None),
      st(20.0, Some(5.0)),
      st(-6.0, None),
      st(15.0, Some(15.0)),
      st(10.0, Some(25.0)),
    ])
    .unwrap();
    near(c.gain_db, 36.0, 1e-12);
    near(c.iip3_dbm.unwrap(), -2.590_014_924, 1e-6);
    near(c.oip3_dbm.unwrap(), 33.409_985_076, 1e-6);
    let share: Vec<f64> = c.stages.iter().map(|s| s.share).collect();
    for (got, want) in share
      .iter()
      .zip([0.0, 0.087_296_837, 0.0, 0.219_279_740, 0.693_423_423])
    {
      near(*got, want, 1e-8);
    }
    // 最后一级的 IIP3 看着最高（25 dBm），但前面累计增益 +26 dB，它反而是瓶颈。
    assert_eq!(c.limiting, Some(4));
    near(share.iter().sum::<f64>(), 1.0, 1e-12);
    // 无源级折算后没有 IIP3，累计增益照常。
    assert_eq!(c.stages[0].iip3_dbm, None);
    near(c.stages[1].gain_before_db, -3.0, 1e-12);
    near(c.stages[4].gain_before_db, 26.0, 1e-12);
  }

  #[test]
  fn a_single_stage_is_its_own_ip3() {
    let c = cascade(&[st(20.0, Some(0.0))]).unwrap();
    near(c.iip3_dbm.unwrap(), 0.0, 1e-12);
    near(c.oip3_dbm.unwrap(), 20.0, 1e-12);
    assert_eq!(c.limiting, Some(0));
  }

  #[test]
  fn oip3_given_per_stage_is_converted_to_the_input() {
    // 同一条链，一次给 IIP3、一次给 OIP3（= IIP3 + 增益），结果必须一样。
    let by_in = cascade(&[
      st(15.0, Some(-5.0)),
      st(10.0, Some(10.0)),
      st(-10.0, None),
      st(25.0, Some(30.0)),
    ])
    .unwrap();
    let by_out = cascade(&[
      Stage {
        gain_db: 15.0,
        ip3_dbm: Some(10.0),
        referred: Referred::Output,
      },
      Stage {
        gain_db: 10.0,
        ip3_dbm: Some(20.0),
        referred: Referred::Output,
      },
      st(-10.0, None),
      Stage {
        gain_db: 25.0,
        ip3_dbm: Some(55.0),
        referred: Referred::Output,
      },
    ])
    .unwrap();
    near(by_in.iip3_dbm.unwrap(), by_out.iip3_dbm.unwrap(), 1e-9);
    near(by_in.iip3_dbm.unwrap(), -8.031_960_574, 1e-6);
    near(by_in.oip3_dbm.unwrap(), 31.968_039_425, 1e-6);
    // 衰减器级给 OIP3 同样要折算：−10 dB 的级，OIP3 = +20 dBm ↔ IIP3 = +30 dBm。
    let att = cascade(&[Stage {
      gain_db: -10.0,
      ip3_dbm: Some(20.0),
      referred: Referred::Output,
    }])
    .unwrap();
    near(att.iip3_dbm.unwrap(), 30.0, 1e-12);
  }

  #[test]
  fn ideal_linear_chains_have_no_ip3_and_the_gain_still_adds_up() {
    let c = cascade(&[st(-3.0, None), st(-6.0, None)]).unwrap();
    assert_eq!((c.iip3_dbm, c.oip3_dbm, c.limiting), (None, None, None));
    near(c.gain_db, -9.0, 1e-12);
    assert!(c.stages.iter().all(|s| s.share == 0.0));
  }

  #[test]
  fn a_lossy_front_end_protects_later_stages() {
    // 先衰减 10 dB，再放大：后级 IIP3 折算到输入端抬高 10 dB。
    let bare = cascade(&[st(20.0, Some(0.0))]).unwrap();
    let padded = cascade(&[st(-10.0, None), st(20.0, Some(0.0))]).unwrap();
    near(
      padded.iip3_dbm.unwrap() - bare.iip3_dbm.unwrap(),
      10.0,
      1e-9,
    );
  }

  #[test]
  fn an_enormous_gain_chain_does_not_overflow() {
    // 十级各 +60 dB：累计增益 540 dB，线性值 1e54 —— 在 dB 域累加就不会溢出。
    let stages: Vec<Stage> = (0..10).map(|_| st(60.0, Some(30.0))).collect();
    let c = cascade(&stages).unwrap();
    assert!(c.iip3_dbm.unwrap().is_finite());
    assert_eq!(c.limiting, Some(9));
    near(c.gain_db, 600.0, 1e-9);
  }

  #[test]
  fn rejects_empty_and_non_finite_input_with_the_stage_number() {
    assert_eq!(cascade(&[]), Err(Ip3Error::Empty));
    assert_eq!(
      cascade(&[st(10.0, Some(0.0)), st(f64::NAN, Some(0.0))]),
      Err(Ip3Error::NotFinite(1))
    );
    assert_eq!(
      cascade(&[st(10.0, Some(f64::INFINITY))]),
      Err(Ip3Error::NotFinite(0))
    );
    assert_eq!(
      cascade(&[st(f64::NEG_INFINITY, None)]),
      Err(Ip3Error::NotFinite(0))
    );
  }

  #[test]
  fn gains_beyond_the_exponent_range_are_rejected_not_returned_as_nan() {
    // 六级各 +700 dB：到第 6 级时累计增益 3500 dB，`mw()` 直接溢出成 `inf`。
    // 溢出若不管，会得到 `Some(-inf)` 与全 NaN 的 `share`，还自称成功。
    let stages: Vec<Stage> = (0..6).map(|_| st(700.0, Some(10.0))).collect();
    assert_eq!(cascade(&stages), Err(Ip3Error::NotFinite(5)));
    // 累计增益本身溢出也拦：两级各 `f64::MAX` dB，第二级一加就是 `inf`。
    let huge: Vec<Stage> = vec![st(f64::MAX, None), st(f64::MAX, None)];
    assert_eq!(cascade(&huge), Err(Ip3Error::NotFinite(1)));
  }
}
