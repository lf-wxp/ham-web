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
}
