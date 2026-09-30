//! QSL 卡片设计：必备信息与设计建议。

/// 必备信息。
pub const QSL_REQUIRED: &[(&str, &str)] = &[
  ("双方呼号", "本台呼号与对方呼号（正反面均可）。"),
  ("通联确认", "日期、时间、频率、模式、信号报告。"),
  ("操作员信息", "操作员姓名、QTH、设备。"),
  ("签名", "操作员签名确认。"),
];

/// 设计建议。
pub const QSL_DESIGN_TIPS: &[&str] = &[
  "突出呼号，正反面信息清晰、易读。",
  "正面可用本地风光、设备照片或个性化设计。",
  "留出填写区域与贴邮票/回邮位置。",
  "批量打印或在线定制（如 Gennady/UX5UO 等专业打印服务）。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn qsl_card_data_populated() {
    assert!(QSL_REQUIRED.len() >= 3);
    assert!(!QSL_DESIGN_TIPS.is_empty());
  }
}
