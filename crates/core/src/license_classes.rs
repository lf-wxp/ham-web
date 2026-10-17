//! 中国业余无线电 A/B/C 类操作证权限对比。

/// 各类别的典型设备与适用场景。
pub const CLASS_USAGE: &[(&str, &str)] = &[
  (
    "A 类",
    "手台、车台、中继通联，覆盖 30MHz 以上 VHF/UHF 本地通信。",
  ),
  (
    "B 类",
    "在 A 类基础上增加短波（HF），可全球远距离通联；30MHz 以下 ≤15W（不大于 15W）。",
  ),
  (
    "C 类",
    "频段与 B 类相同，短波可用大功率（≤1000W），适合竞赛与远距离 DX。",
  ),
];

/// 常见波段与可使用类别。
pub const BAND_PERMISSIONS: &[(&str, &str)] = &[
  ("160m / 80m / 40m / 20m 等短波", "B、C 类（30MHz 以下）"),
  ("10m（28–29.7MHz）", "B、C 类（仍属短波，低于 30MHz）"),
  ("6m 及以上（50MHz 以上）", "A、B、C 类"),
  ("2m / 70cm", "A、B、C 类"),
  ("23cm / 13cm 等 3GHz 以下微波", "A、B、C 类"),
  ("3GHz 以上微波（5cm、3cm 等）", "B、C 类"),
];

/// 30MHz：**功率**口径的分界（30MHz 以上 A / B / C 一致为 [`CLASS_A_MAX_W`]），也是频率权限的
/// 判定线（A 类只能从 30MHz 起，上限另有 [`CLASS_A_UPPER_MHZ`]；B / C 类不受频率限制）。
///
/// 与下面的功率常量一样是**唯一事实来源**：[`crate::license_compliance`] 的频率权限判定与
/// [`BAND_PERMISSIONS`] 的文字口径都指向它，别在别处再写一个 `30.0`。
pub const VHF_THRESHOLD_MHZ: f64 = 30.0;

/// A 类频率上限（MHz）：办法第三十条的「30-3000MHz」。
///
/// 3GHz 以上只有 B / C 类可用（见 [`BAND_PERMISSIONS`] 最后一行）。
pub const CLASS_A_UPPER_MHZ: f64 = 3000.0;

/// A / B / C 类功率上限（W）：全站功率口径的**唯一事实来源**。
///
/// 30MHz 以上 A/B/C 一致为 [`CLASS_A_MAX_W`]；30MHz 以下 B 类为
/// [`CLASS_B_HF_MAX_W`]、C 类为 [`CLASS_C_HF_MAX_W`]。各处的文字说明与回归测试都引用
/// 这里，避免「A 类 25W / B 类 15W」在不同页面漂移。
pub const CLASS_A_MAX_W: f64 = 25.0;
/// B 类 30MHz 以下功率上限（W）——"不大于 15W"。
pub const CLASS_B_HF_MAX_W: f64 = 15.0;
/// C 类 30MHz 以下功率上限（W）。
pub const CLASS_C_HF_MAX_W: f64 = 1000.0;

/// 备考要点。
pub const CLASS_TIPS: &[&str] = &[
  "A 类为入门类别，通过后方可申请 B 类，依次递进。",
  concat!(
    "各等级可用频段与功率上限不同：A 类限 30MHz 以上且 ≤25W；",
    "B 类在 30MHz 以下 ≤15W，是各等级中最严格的功率限值（比 A 类的 25W 更低）；",
    "C 类频段与 B 类相同，30MHz 以下可到 ≤1000W。",
    "以现行《业余无线电台管理办法》为准。",
  ),
  "功率指发射机射频输出（峰包功率 PEP）上限。",
  "实际可用频率以国家无线电管理机构公布的划分表为准。",
  "操作证与电台执照是两回事，设台还需单独申领电台执照。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn license_classes_populated() {
    assert!(CLASS_USAGE.len() >= 3);
    assert!(BAND_PERMISSIONS.len() >= 4);
    assert!(!CLASS_TIPS.is_empty());
  }

  /// 单一事实来源的**执行器**：全站各处「A 类 25W / B 类 15W / C 类 1000W」的写法
  /// （reference / regulations / categories 的散落文案）必须与本模块常量一致 ——
  /// 谁把 25W 写成 30W，这条测试当场红。
  ///
  /// 匹配片段由常量格式化出来（带 `W` 单位），不手写数字，也不用裸数字做子串匹配
  /// （`"15"` 会误中 `1500`、`"25"` 会误中 `125`）。
  #[test]
  fn the_scattered_power_claims_match_the_canonical_constants() {
    // 常量本身先钉死（精确比较，不做整数截断 —— 谁把 25W 改成非整数，这里当场红）。
    assert_eq!(CLASS_A_MAX_W, 25.0, "A 类功率上限漂移");
    assert_eq!(CLASS_B_HF_MAX_W, 15.0, "B 类功率上限漂移");
    assert_eq!(CLASS_C_HF_MAX_W, 1000.0, "C 类功率上限漂移");
    let (a, b, c) = (
      format!("{CLASS_A_MAX_W}W"),
      format!("{CLASS_B_HF_MAX_W}W"),
      format!("{CLASS_C_HF_MAX_W}W"),
    );

    // reference::LICENSE_CLASSES 的 power 栏。
    let rows = crate::reference::LICENSE_CLASSES;
    assert!(rows[0].power.contains(&a), "{}", rows[0].power);
    assert!(
      rows[1].power.contains(&b) && rows[1].power.contains(&a),
      "{}",
      rows[1].power
    );
    assert!(
      rows[2].power.contains(&c) && rows[2].power.contains(&a),
      "{}",
      rows[2].power
    );

    // reference 的 REF 列表（「B 类 30MHz 以下不大于 15W」那条）。
    let not_more_than_b = format!("不大于 {b}");
    assert!(
      crate::reference::KNOWLEDGE_SOURCES
        .iter()
        .any(|(_, _, t)| t.contains(&not_more_than_b)),
      "reference 的「功率限值」来源行要与常量一致"
    );

    // regulations 的类别说明（LICENSE_LEVELS 里带功率口径）。
    let regs = crate::regulations::LICENSE_LEVELS;
    for (class, w) in [("A", &a), ("B", &b), ("C", &c)] {
      let needle = format!("≤{w}");
      assert!(
        regs.iter().any(|(_, t)| t.contains(&needle)),
        "regulations 里缺 {class} 类 {needle}"
      );
    }

    // categories 的知识点注（第三十条权限）。
    assert!(
      crate::categories::SUB_NOTES.iter().any(|(_, t)| {
        t.contains(&format!("≤{a}")) && t.contains(&format!("≤{b}")) && t.contains(&c)
      }),
      "categories 里的权限注要与常量一致"
    );

    // 本模块自己的文案。
    let all_text: String = CLASS_USAGE
      .iter()
      .map(|(_, t)| *t)
      .chain(CLASS_TIPS.iter().copied())
      .collect::<Vec<_>>()
      .join("\n");
    for needle in [&a, &b, &c] {
      assert!(all_text.contains(needle.as_str()), "本模块文案缺 {needle}");
    }
  }
}
