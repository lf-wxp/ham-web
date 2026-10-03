//! 学习资源索引：视频课程、技术文档与自学建议。

/// 视频资源。
pub const VIDEO_RESOURCES: &[(&str, &str, &str)] = &[
  (
    "B 站 / YouTube",
    "呼号抄收、天线架设、通联实录",
    "搜索「业余无线电 入门」等关键词",
  ),
  ("ARRL 官方", "操作、执照、技术", "英文权威入门资料"),
  ("火腿个人频道", "设备评测、实战经验", "结合评论区交叉验证"),
];

/// 技术文档资源。
pub const DOC_RESOURCES: &[(&str, &str, &str)] = &[
  ("操作证考试大纲", "各级别知识点", "对照复习、查漏补缺"),
  ("《业余无线电台管理办法》", "设台、呼号、功率", "法规依据"),
  ("ITU《无线电规则》", "频段划分", "国际法规参考"),
  ("设备手册 PDF", "各厂商说明书", "操作与维护"),
];

/// 自学建议。
pub const LEARNING_RESOURCES_TIPS: &[&str] = &[
  "本站题库 + 解析是最直接的备考资源，配合术语表与考点手册使用。",
  "视频与文档互补：先看视频建立直觉，再看文档与法规夯实细节。",
  "用「备考计划」安排每日任务，用「学习周报」复盘进度。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn learning_resources_populated() {
    assert!(!VIDEO_RESOURCES.is_empty());
    assert!(!DOC_RESOURCES.is_empty());
    assert!(!LEARNING_RESOURCES_TIPS.is_empty());
  }
}
