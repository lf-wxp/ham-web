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
  /// `.nec` 文本（NEC 免费格式子集：`GW` / `GE` / `EX` / `FR`）。
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
GW 1 30 0.0 0.0 0.0 0.0 0.0 1.551 0.005
GW 2 10 0.03 0.0 0.0 0.03 0.0 0.517 0.005
GW 3 1 0.0 0.0 0.0 0.03 0.0 0.0 0.005
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
CM 单元间距 0.15λ / 0.10λ；有源振子在中间（第二根导线）
GW 1 15 -0.310 0.0 0.0 -0.310 0.0 1.075 0.004
GW 2 15 0.0 0.0 0.0 0.0 0.0 0.972 0.004
GW 3 15 0.207 0.0 0.0 0.207 0.0 0.910 0.004
GE 1
EX 0 2 8 0 1.0 0.0
FR 0 1 145.0 0.0
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
  use crate::nec::{parse_nec, solve, wavelength};

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

    assert!(
      file
        .input
        .wires
        .iter()
        .any(|w| w.a == [0.0, 0.0, 0.0] && w.b == [0.03, 0.0, 0.0]),
      "缺少底部短路棒（0,0,0）→（0.03,0,0），两臂并未相连"
    );

    let lam = wavelength(file.input.freq_hz);
    let feed = file.input.feeds[0];
    // 激励必须落在**长臂**（整根 0.75λ 那根）上，且抽头高度是「底部往上百分之几波长」。
    let wire = file.input.wires[feed.wire];
    let height = wire.a[2] + feed.at * (wire.b[2] - wire.a[2]);
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
      let r = solve(&file.input).unwrap_or_else(|| panic!("{} 无法求解", t.id));

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
}
