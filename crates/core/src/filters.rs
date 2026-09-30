//! 滤波器与双工器：低通、带通、陷波器与双工器。

/// 器件类型。
pub const FILTER_TYPES: &[(&str, &str, &str)] = &[
  (
    "低通滤波器",
    "LPF（Low Pass Filter）",
    "只通过低频，抑制谐波，接在发射机输出端。",
  ),
  (
    "高通滤波器",
    "HPF（High Pass Filter）",
    "只通过高频，用于抑制低频干扰。",
  ),
  (
    "带通滤波器",
    "BPF（Band Pass Filter）",
    "只通过某频段，竞赛多机工作时隔离收发。",
  ),
  (
    "陷波器",
    "Notch Filter",
    "滤除特定频率的强干扰，如广播干扰。",
  ),
  ("双工器", "Duplexer", "让收发共用一根天线，中继台核心器件。"),
  (
    "晶体滤波器",
    "Crystal Filter",
    "窄带高选择性，用于收信机中频。",
  ),
];

/// 选用要点。
pub const FILTER_TIPS: &[&str] = &[
  "发射低通滤波器按最高工作频段选择，抑制谐波干扰电视 / 电话。",
  "多机竞赛台用带通滤波器隔离，避免互相压制。",
  "中继台双工器需承受大功率且收发隔离度足够。",
  "接收端陷波器可压制本地强广播干扰，改善信噪比。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn filters_data_populated() {
    assert!(FILTER_TYPES.len() >= 5);
    assert!(!FILTER_TIPS.is_empty());
  }
}
