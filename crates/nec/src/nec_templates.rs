//! 方案库挂载的 `.nec` 模型模板。
//!
//! 「实用天线」「天线 DIY」里的每条典型方案，如果能用线天线模型表达，就在这里挂一份
//! `.nec` 文本；方案库的表格上会出现「在求解器中打开」，点了直接把这份模型加载进
//! `/nec`。
//!
//! **每一份模板都必须能解析并解出合理结果** —— 这是由本模块的单元测试强制保证的
//! （模板是手写的字符串，写错一个字段就必须当场发现，而不是等用户点开才看到一片空白）。
//!
//! 尺寸取自各方案自身的「尺寸估算」公式（`143/f`、`71.5/f` 等），所以文章里的数字与
//! 加载出来的模型是一致的，不存在「文章说 10.1 m、模型里是 9.5 m」。

/// 一份可加载到求解器的 `.nec` 模型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NecTemplate {
  /// 稳定 id（方案库的数据行用它引用）。
  pub id: &'static str,
  /// 展示名（加载后作为提示文案的一部分）。
  pub name: &'static str,
  /// `.nec` 文本（NEC 免费格式子集：`GW` / `GE` / `EX` / `FR`，对数周期模板另有 `TL`）。
  pub text: &'static str,
}

/// 全部模板。id 必须唯一（单测把关）。
pub const NEC_TEMPLATES: &[NecTemplate] = &[
  NecTemplate {
    id: "dipole-14mhz",
    name: "半波偶极（14 MHz，架高 10 m）",
    text: "\
CM 半波偶极 14.1 MHz：总长 143/f = 10.14 m，架高 10 m，理想导体地面
CM 中心馈电，阻抗约 70 Ω
GW 1 21 -5.07 0.0 10.0 5.07 0.0 10.0 0.002
GE 1
EX 0 1 11 0 1.0 0.0
FR 0 1 14.1 0.0
EN
",
  },
  NecTemplate {
    id: "inverted-v-14mhz",
    name: "倒 V（14 MHz，120° 夹角）",
    text: "\
CM 倒 V 14.1 MHz：同样的 10.14 m 导线折成 120° 夹角，顶点架高 10 m
CM 两臂各 5.07 m、下垂 30°，末端离地 7.46 m
GW 1 21 0.0 0.0 10.0 -4.39 0.0 7.46 0.002
GW 2 21 0.0 0.0 10.0 4.39 0.0 7.46 0.002
GE 1
EX 0 1 1 0 1.0 0.0
FR 0 1 14.1 0.0
EN
",
  },
  NecTemplate {
    id: "vertical-gp-2m",
    name: "λ/4 垂直地网天线（2 m）",
    text: "\
CM 2 m 波段 λ/4 垂直天线：振子长 71.5/f = 0.493 m
CM 3–4 根地网用理想导体地面（镜像法）等效
GW 1 21 0.0 0.0 0.0 0.0 0.0 0.493 0.004
GE 1
EX 0 1 1 0 1.0 0.0
FR 0 1 145.0 0.0
EN
",
  },
  NecTemplate {
    id: "efhw-40m",
    name: "EFHW 端馈半波（40 m 段）",
    text: "\
CM EFHW 7.1 MHz：半波长导线 143/f = 20.14 m，一端馈电
CM 端馈阻抗在 kΩ 量级 —— 这正是必须用 49:1 / 64:1 unun 的原因
GW 1 41 0.0 0.0 9.0 20.14 0.0 9.0 0.002
GE 1
EX 0 1 1 0 1.0 0.0
FR 0 1 7.1 0.0
EN
",
  },
  NecTemplate {
    id: "j-pole-2m",
    name: "J 型天线（2 m）",
    text: "\
CM J 型天线 145 MHz：长臂 0.75λ = 1.551 m、短臂 0.25λ = 0.517 m，底部由短路棒相连
CM 两臂间距 0.03 m；匹配支节就是这两根平行导线 + 底部短路棒
CM 抽头取短臂底部往上约 0.04λ（长臂第 2 段、段心 0.078 m）。**不要**取在短臂顶端：
CM 那是 0.25λ 短路支节的开路端，阻抗在 kΩ 量级，实测解出 113 + j1090 Ω，没法用
CM 激励写成「串在长臂抽头所在的那一段上」：另拉一根跨接导线会在长臂接点处形成
CM 度为 3 的节点，而本求解器对度 ≥ 3 的接点只作近似（实测解出 418 − j9019 Ω 的垃圾值）
CM 底部架在 2 m 桅杆上：短路棒若躺在 z = 0 就贴着地面，NEC-2 把它判为非法几何
GW 1 30 0.0 0.0 2.0 0.0 0.0 3.551 0.005
GW 2 10 0.03 0.0 2.0 0.03 0.0 2.517 0.005
GW 3 1 0.0 0.0 2.0 0.03 0.0 2.0 0.005
GE 1
EX 0 1 2 0 1.0 0.0
FR 0 1 145.0 0.0
EN
",
  },
  NecTemplate {
    id: "yagi3-2m",
    name: "三单元八木（2 m）",
    text: "\
CM 2 m 三单元八木 145 MHz：反射器 0.52λ、有源振子 0.47λ、引向器 0.44λ
CM 单元间距 0.15λ / 0.10λ；有源振子在中间（第二根导线）；自由空间、单元以 z = 0 为中心
CM 曾经写成「单元从 z = 0 往上 + 理想地面」：那是三根接地的单极、中间馈电，两套求解器都
CM 解出约 4 kΩ —— 根本不是八木。与 necpp 交叉验证时暴露出来
GW 1 15 -0.310 0.0 -0.5375 -0.310 0.0 0.5375 0.004
GW 2 15 0.0 0.0 -0.486 0.0 0.0 0.486 0.004
GW 3 15 0.207 0.0 -0.455 0.207 0.0 0.455 0.004
GE 0
EX 0 2 8 0 1.0 0.0
FR 0 1 145.0 0.0
EN
",
  },
  NecTemplate {
    id: "lpda-14-28mhz",
    name: "对数周期偶极阵（14–28 MHz，12 根）",
    text: "\
CM 对数周期偶极阵（LPDA）14–28 MHz：τ = 0.90、σ = 0.16、12 根振子（与「天线补充专题」的算例同一组参数）
CM 最长振子 10.7 m（14 MHz 的半波长）、最短 3.36 m，臂长 23.5 m；振子沿 y 轴、排列沿 x 轴，自由空间
CM 相邻振子的中心之间各接一条**交叉线**（TL 的 Z0 为负 = 180° 反相，NEC-2 的约定），线长取两振子的间距；
CM Z0 取 150 Ω：只是让 14–28.5 MHz 的馈电阻抗落在 110–130 Ω 的一个取值，不是最优设计，需要匹配到 50 Ω 时另加变换
CM 激励在最短振子（顶点），波束指向顶点一侧（+x）；后方没有终端支节
GW 1 15 0.000 -5.350 0.0 0.000 5.350 0.0 0.01
GW 2 15 3.424 -4.815 0.0 3.424 4.815 0.0 0.01
GW 3 15 6.506 -4.333 0.0 6.506 4.333 0.0 0.01
GW 4 15 9.279 -3.900 0.0 9.279 3.900 0.0 0.01
GW 5 15 11.775 -3.510 0.0 11.775 3.510 0.0 0.01
GW 6 15 14.022 -3.159 0.0 14.022 3.159 0.0 0.01
GW 7 15 16.043 -2.843 0.0 16.043 2.843 0.0 0.01
GW 8 15 17.863 -2.559 0.0 17.863 2.559 0.0 0.01
GW 9 15 19.501 -2.303 0.0 19.501 2.303 0.0 0.01
GW 10 15 20.975 -2.073 0.0 20.975 2.073 0.0 0.01
GW 11 15 22.301 -1.865 0.0 22.301 1.865 0.0 0.01
GW 12 15 23.495 -1.679 0.0 23.495 1.679 0.0 0.01
GE 0
TL 1 8 2 8 -150 3.424
TL 2 8 3 8 -150 3.082
TL 3 8 4 8 -150 2.773
TL 4 8 5 8 -150 2.496
TL 5 8 6 8 -150 2.246
TL 6 8 7 8 -150 2.022
TL 7 8 8 8 -150 1.820
TL 8 8 9 8 -150 1.638
TL 9 8 10 8 -150 1.474
TL 10 8 11 8 -150 1.327
TL 11 8 12 8 -150 1.194
EX 0 12 8 0 1.0 0.0
FR 0 1 14.2 0.0
EN
",
  },
];

/// 按 id 取模板。
#[must_use]
pub fn nec_template(id: &str) -> Option<&'static NecTemplate> {
  NEC_TEMPLATES.iter().find(|t| t.id == id)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::nec::{parse_nec, solve, solve_with_lines, wavelength};

  #[test]
  fn template_ids_are_unique() {
    let mut ids: Vec<&str> = NEC_TEMPLATES.iter().map(|t| t.id).collect();
    ids.sort_unstable();
    let before = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), before, "模板 id 不能重复");
    for t in NEC_TEMPLATES {
      assert!(!t.id.is_empty() && !t.name.is_empty() && !t.text.is_empty());
    }
  }

  #[test]
  fn lookup_by_id_works() {
    assert_eq!(nec_template("dipole-14mhz").unwrap().id, "dipole-14mhz");
    assert!(nec_template("no-such-template").is_none());
  }

  /// J 型天线必须有**底部短路棒**，且抽头位置要落在「能用」的高度上。
  ///
  /// 曾经只有两根平行导线：注释写着「底部相连」，实际两臂底端相距 0.03 m —— 远超节点
  /// 合并阈值（坐标按 1e-7 m 量化，见 `nec::Model::from_wires`），也没有任何连接段，
  /// 于是模型是两根**互不相连**的导线，匹配支节根本不存在；而
  /// [`every_template_parses_and_solves_to_something_sane`] 只断言 `R > 1` / `gain > 0`，
  /// 抓不到这种拓扑错误。抽头取在短臂顶端（0.25λ）时阻抗还会跑到 kΩ 量级 —— 同样抓不到。
  #[test]
  fn j_pole_has_a_bottom_strap_and_a_usable_tap() {
    let t = nec_template("j-pole-2m").expect("J 型模板必须在");
    let file = parse_nec(t.text).expect("应能解析");

    // 底部短路棒：一根水平导线，两端分别落在长臂与短臂的底端上。
    let (long, short) = (file.input.wires[0], file.input.wires[1]);
    assert!(
      file
        .input
        .wires
        .iter()
        .any(|w| w.a == long.a && w.b == short.a),
      "缺少底部短路棒 {:?} → {:?}，两臂并未相连",
      long.a,
      short.a
    );
    // 短路棒不能贴在地面上（z = 0 的水平导线在 NEC-2 里是非法几何）。
    assert!(long.a[2] > 0.0, "底部高度 {} m：短路棒贴地", long.a[2]);

    let lam = wavelength(file.input.freq_hz);
    let feed = file.input.feeds[0];
    // 激励必须落在**长臂**（整根 0.75λ 那根）上，且抽头高度是「底部往上百分之几波长」。
    let wire = file.input.wires[feed.wire];
    let height = feed.at * (wire.b[2] - wire.a[2]);
    // 抽头高度从**底端**（短路棒）往上量，而不是从地面。
    assert!(
      (0.01..0.10).contains(&(height / lam)),
      "抽头高度 {height:.3} m = {:.3}λ 不在可用区间（0.01λ–0.10λ）：0.25λ 处是开路支节，阻抗 kΩ 量级",
      height / lam
    );

    // 解出来必须是一份「能拿去找匹配」的数值：电阻几十欧量级、电抗不跑到 kΩ。
    let r = solve(&file.input).expect("应能求解");
    assert!(
      (5.0..150.0).contains(&r.impedance.0),
      "J 型馈电阻抗 {:.2} + j{:.2} Ω 不像话",
      r.impedance.0,
      r.impedance.1
    );
    assert!(
      r.impedance.1.abs() < 300.0,
      "电抗 {:.2} Ω 太大（多半是抽头取在开路支节端）",
      r.impedance.1
    );
  }

  #[test]
  fn every_template_parses_and_solves_to_something_sane() {
    // 模板是手写字符串：这里强制它们**全部**能解析、能求解，且解出来的东西物理上
    // 说得过去（电阻为正、增益合理）。写错一个字段就会在这里红掉。
    for t in NEC_TEMPLATES {
      let file = parse_nec(t.text).unwrap_or_else(|| panic!("{} 解析失败", t.id));
      assert!(!file.input.wires.is_empty(), "{} 没有导线", t.id);
      assert!(!file.input.feeds.is_empty(), "{} 没有馈电点", t.id);
      assert!(file.input.freq_hz > 0.0, "{} 频率非法", t.id);
      let r =
        solve_with_lines(&file.input, &file.lines).unwrap_or_else(|| panic!("{} 无法求解", t.id));

      assert!(
        r.impedance.0 > 1.0,
        "{} 电阻 {} Ω 太小，多半是几何或馈电写错了",
        t.id,
        r.impedance.0
      );
      assert!(
        r.gain_max_dbi > 0.0,
        "{} 最大增益 {} dBi 不合理",
        t.id,
        r.gain_max_dbi
      );
      assert!(
        r.segments <= 200,
        "{} 用了 {} 段，超过前端上限",
        t.id,
        r.segments
      );
      // 电长度要落在「天线」而不是「一小段导线」的范畴：最长导线至少 0.1λ。
      let lam = wavelength(file.input.freq_hz);
      let longest = file
        .input
        .wires
        .iter()
        .map(|w| w.length())
        .fold(0.0f64, f64::max);
      assert!(
        longest > 0.1 * lam,
        "{} 最长导线 {longest} m 只有 {:.3}λ",
        t.id,
        longest / lam
      );
    }
  }

  #[test]
  fn template_dimensions_match_their_stated_formulas() {
    // 文章里的「尺寸估算」用的是 143/f 与 71.5/f 两个经典公式。模板必须真的按它们
    // 取值，否则会出现「文章说 10.14 m、加载出来是别的长度」。
    let dipole = parse_nec(nec_template("dipole-14mhz").unwrap().text).unwrap();
    let want = 143.0 / 14.1;
    let got = dipole.input.wires[0].length();
    assert!(
      (got - want).abs() < 0.02,
      "十四兆偶极应为 {want:.3} m（143/f），实际 {got:.3} m"
    );

    let gp = parse_nec(nec_template("vertical-gp-2m").unwrap().text).unwrap();
    let want = 71.5 / 145.0;
    let got = gp.input.wires[0].length();
    assert!(
      (got - want).abs() < 0.01,
      "2 m λ/4 振子应为 {want:.3} m（71.5/f），实际 {got:.3} m"
    );

    // 倒 V 与偶极用同一根导线：总长应当一致（两臂之和）。
    let inv = parse_nec(nec_template("inverted-v-14mhz").unwrap().text).unwrap();
    let total: f64 = inv.input.wires.iter().map(|w| w.length()).sum();
    assert!(
      (total - want_of_dipole()).abs() < 0.05,
      "倒 V 两臂之和 {total:.3} m 应等于偶极总长 {:.3} m",
      want_of_dipole()
    );

    fn want_of_dipole() -> f64 {
      143.0 / 14.1
    }
  }

  #[test]
  fn efhw_end_feed_is_the_high_impedance_case() {
    // 端馈半波的意义就在于「端馈阻抗极高」——模板必须真的把馈电放在端点，
    // 而且解出来的阻抗要明显高于同长度中心馈电的偶极。
    let efhw = parse_nec(nec_template("efhw-40m").unwrap().text).unwrap();
    let r = solve(&efhw.input).unwrap();
    assert!(
      r.impedance.0 > 500.0,
      "端馈半波电阻 {} Ω 应远高于中心馈电（这正是要 unun 的原因）",
      r.impedance.0
    );
    assert_eq!(efhw.input.feeds[0].at, 0.5 / 41.0, "馈电点应落在端点段上");
  }

  /// necpp（PyNEC 2.3.4 起、2.4.0 重跑逐位相同，NEC-2 的 C++ 移植）对同一份模板的参照解：`(id, R, X, 最大增益)`。
  ///
  /// 生成方式（`crates/nec/reference/necpp_ref.py`）：把模板逐卡转给 PyNEC
  /// （`GE 1` 补 `GN 1` = 理想导体地面），最大增益在上半空间 θ / φ 各 5° 网格上取。这是**另一套 NEC-2 实现**的结果，不是实测值 —— 它证明的是
  /// 「两套独立的矩量法对同一几何给出同一答案」，而不是「比真值更准」。
  const NECPP_REF: &[(&str, f64, f64, f64)] = &[
    ("dipole-14mhz", 70.267, -36.972, 7.947),
    ("inverted-v-14mhz", 63.188, -44.975, 7.124),
    ("vertical-gp-2m", 37.965, 6.608, 5.159),
    ("efhw-40m", 4155.489, -10253.688, 7.925),
    ("j-pole-2m", 10.966, 25.061, 8.554),
    ("yagi3-2m", 34.177, -23.083, 7.066),
    ("lpda-14-28mhz", 128.248, -7.102, 7.987),
  ];

  #[test]
  fn templates_agree_with_necpp() {
    assert_eq!(
      NECPP_REF.len(),
      NEC_TEMPLATES.len(),
      "新模板也要补 necpp 参照"
    );
    for &(id, r, x, g) in NECPP_REF {
      let file = parse_nec(nec_template(id).unwrap().text).unwrap();
      let got = solve_with_lines(&file.input, &file.lines).unwrap();
      let (gr, gx) = got.impedance;
      // 增益（方向图积分）两边一致到 0.5 dB；J 型的差最大（0.41 dB）。
      assert!(
        (got.gain_max_dbi - g).abs() < 0.5,
        "{id}: 增益 {:.3} vs necpp {g}",
        got.gain_max_dbi
      );
      if id == "efhw-40m" {
        // 端馈半波的馈电点落在电流波节上：阻抗是两个大数之差，对段长与间隙模型极端敏感，
        // 两套实现只在**量级**上一致（都是 kΩ、都是容性），不做数值对拍。
        assert!(gr > 1000.0 && gx < 0.0, "{id}: {gr} {gx}");
        continue;
      }
      // 复阻抗的相对差 < 12%：本实现（hat 基 Galerkin）对教科书偶极的电阻本就偏 +10%
      // （见 nec.rs 模块文档），necpp 是脉冲基 + 点匹配，两者离散方式不同；
      // 实测最大的是八木（10.3%）与 J 型（8.5%），其余都在 2% 以内。
      let d = (gr - r).hypot(gx - x) / r.hypot(x);
      assert!(
        d < 0.12,
        "{id}: {gr:.3}{gx:+.3}j vs necpp {r}{x:+}j（相对差 {:.1}%）",
        d * 100.0
      );
    }
  }

  /// LPDA 模板在 5 个频点上与 necpp 的逐点对拍：`(MHz, R, X, 最大增益 dBi, 前后比 dB)`。
  ///
  /// 生成：`python crates/nec/reference/tl_necpp.py template`（PyNEC 2.4.0）。
  /// 「天线补充专题」页面上写的「阻抗相差 2% 以内、增益不超过 0.1 dB、前后比不超过 2 dB」就是
  /// 下面三个容限 —— 页面文字改了数字，这里和脚本输出要一起改。
  const NECPP_LPDA_SWEEP: &[(f64, f64, f64, f64, f64)] = &[
    (14.2, 128.248, -7.102, 7.987, 19.73),
    (18.1, 121.574, -10.646, 8.569, 29.03),
    (21.2, 117.695, -20.574, 8.303, 27.10),
    (24.9, 111.875, -6.670, 8.024, 21.58),
    (28.5, 121.946, -19.235, 8.551, 29.94),
  ];

  #[test]
  fn lpda_sweep_agrees_with_necpp_as_the_page_claims() {
    let file = parse_nec(nec_template("lpda-14-28mhz").unwrap().text).unwrap();
    for &(mhz, r, x, g, fb) in NECPP_LPDA_SWEEP {
      let mut input = file.input.clone();
      input.freq_hz = mhz * 1e6;
      let got = solve_with_lines(&input, &file.lines).unwrap();
      let dz = (got.impedance.0 - r).hypot(got.impedance.1 - x) / r.hypot(x);
      assert!(
        dz < 0.02,
        "{mhz} MHz：阻抗相差 {:.2}%，页面写的是 2% 以内",
        dz * 100.0
      );
      let dg = (got.gain_max_dbi - g).abs();
      assert!(
        dg < 0.1,
        "{mhz} MHz：增益相差 {dg:.3} dB，页面写的是不超过 0.1 dB"
      );
      let dfb = (got.front_to_back_db - fb).abs();
      assert!(
        dfb < 2.0,
        "{mhz} MHz：前后比相差 {dfb:.2} dB，页面写的是不超过 2 dB"
      );
      assert!(
        got.gain_max_azimuth_deg.abs() < 1.0 || (got.gain_max_azimuth_deg - 360.0).abs() < 1.0,
        "{mhz} MHz：波束方位 {}° 不在顶点一侧",
        got.gain_max_azimuth_deg
      );
    }
  }

  /// LPDA 模板的意义在**交叉线**：相邻振子反相馈电，波束才指向顶点（最短振子）一侧、前后比才高。
  /// 把 `Z0` 的符号翻回去（普通线）整个方向图就垮掉 —— 这条用例把这个差别钉住，
  /// 也防止以后有人把「负 Z0」当成非法输入而悄悄丢掉。
  #[test]
  fn lpda_needs_its_crossed_lines() {
    let file = parse_nec(nec_template("lpda-14-28mhz").unwrap().text).unwrap();
    assert_eq!(file.lines.len(), 11, "12 根振子之间 11 条线");
    assert!(
      file.lines.iter().all(|l| l.z0 < 0.0),
      "模板里的线都应是交叉线（Z0 为负）：{:?}",
      file.lines
    );
    assert!(
      file.ignored.is_empty(),
      "不应有被忽略的卡片：{:?}",
      file.ignored
    );
    let crossed = solve_with_lines(&file.input, &file.lines).unwrap();
    // 波束指向顶点（+x，方位 0°）；necpp 同口径下 F/B = 19.7 dB（本实现 19.2 dB）。
    assert!(
      crossed.gain_max_azimuth_deg.abs() < 1.0
        || (crossed.gain_max_azimuth_deg - 360.0).abs() < 1.0,
      "方位 {}°",
      crossed.gain_max_azimuth_deg
    );
    assert!(
      crossed.front_to_back_db > 15.0,
      "F/B {} dB",
      crossed.front_to_back_db
    );
    let plain: Vec<_> = file
      .lines
      .iter()
      .map(|l| crate::nec::TransmissionLine { z0: -l.z0, ..*l })
      .collect();
    let plain = solve_with_lines(&file.input, &plain).unwrap();
    assert!(
      plain.front_to_back_db < crossed.front_to_back_db - 10.0,
      "不交叉时 F/B {} dB 应比交叉时 {} dB 差得多",
      plain.front_to_back_db,
      crossed.front_to_back_db
    );
  }
}
