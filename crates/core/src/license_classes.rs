//! 中国业余无线电 A/B/C 类操作证权限对比。

/// 各类别的典型设备与适用场景。
pub const CLASS_USAGE: &[(&str, &str)] = &[
  ("A 类", "手台、车台、中继通联，覆盖 VHF/UHF 本地通信。"),
  ("B 类", "在 A 类基础上增加短波（HF），可全球远距离通联。"),
  ("C 类", "短波可用大功率（≤1000W），适合竞赛与远距离 DX。"),
];

/// 常见波段与可使用类别。
pub const BAND_PERMISSIONS: &[(&str, &str)] = &[
  ("160m / 80m / 40m / 20m 等短波", "B、C 类（30MHz 以下）"),
  ("10m / 6m", "A、B、C 类（30MHz 以上）"),
  ("2m / 70cm", "A、B、C 类"),
  ("23cm 及以上微波", "A、B、C 类"),
];

/// 备考要点。
pub const CLASS_TIPS: &[&str] = &[
  "A 类为入门类别，通过后方可申请 B 类，依次递进。",
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
