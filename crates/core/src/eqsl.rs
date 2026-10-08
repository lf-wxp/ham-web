//! 电子 QSL 与日志确认：LoTW、eQSL、QRZ、Club Log 等现代确认方式。

/// 主要服务。
pub const EQSL_SERVICES: &[(&str, &str, &str)] = &[
  (
    "LoTW",
    "ARRL 维护，数字签名日志",
    "DXCC 等奖状的主要确认方式，需 TQSL 证书签名上传。",
  ),
  ("eQSL", "电子 QSL 卡片", "电子卡片交换，与纸质 QSL 类似。"),
  (
    "QRZ Logbook",
    "QRZ.com 日志",
    "在线日志与 QSL 管理，社区广泛使用。",
  ),
  (
    "Club Log",
    "日志分析平台",
    "传播分析、稀有台需求匹配与 OQRS 卡片请求。",
  ),
];

/// 说明要点。
pub const EQSL_NOTES: &[&str] = &[
  "电子确认通常比纸质 QSL 更快捷，是现代奖状申请的主流方式。",
  "LoTW 需要 TQSL 证书并签名上传，可复用通联日志导出的 ADIF 文件。",
  "纸质 QSL 仍有纪念意义，可通过 OQRS 或直邮 + 回邮券交换。",
];

/// LoTW 签名上传的完整流程（`(步骤, 说明)`）。
///
/// 从「申请证书」一路写到「把确认回写进日志」，因为这套流程最容易断在中间：
/// 证书没到位、Station Location 填错、时间差几分钟导致对不上，都是常见卡点。
pub const LOTW_UPLOAD_STEPS: &[(&str, &str)] = &[
  (
    "申请呼号证书",
    "在 ARRL 的 LoTW 网站用呼号登录、申请 Callsign Certificate，收到 .tq6 邮件后导入 TQSL。证书有有效期，换呼号或到期都要重新申请。",
  ),
  (
    "在 TQSL 里建立 Station Location",
    "填写呼号、DXCC 实体、CQ / ITU 分区与网格。这里填错不会报错，但会让 QSO 在 LoTW 判为「不匹配」—— 报给 LoTW 的分区以它为准。",
  ),
  (
    "从本站导出 ADIF",
    "通联日志页点「导出 ADIF」，得到整库的 .adi 文件。LoTW 按「呼号 + 日期 + 时间 + 波段 + 模式」匹配，所以本地时间差几分钟也会对不上，导出前值得核对一遍。",
  ),
  (
    "用 TQSL 签名",
    "在 TQSL 里选 Sign and upload（或 Sign a log file）→ 选中刚导出的 .adi 与对应的 Station Location，生成签名后的 .tq8 文件。签名在你自己电脑上完成，私钥不出本机。",
  ),
  (
    "上传 .tq8",
    "TQSL 可以直接上传；也可以到 LoTW 网站的 Upload File 手动传 .tq8。上传成功后 LoTW 端记录的是「已上传」，真正的确认要等双方日志都在 LoTW 上。",
  ),
  (
    "把状态回写进日志",
    "上传成功后回到本站，点「标记已上传 LoTW」把这一批标成已上传。等 LoTW 处理完（通常几分钟到一天），在 LoTW 网站下载确认报告（ADIF），回到通联日志页点「同步 QSL」：先看差异、逐条裁决，再应用。",
  ),
];

/// LoTW 的边界与常见坑。
pub const LOTW_UPLOAD_NOTES: &[&str] = &[
  "本站不做代签：证书与私钥只存在你自己的电脑上，签名与上传都由 TQSL 完成。本站只负责导出待签名的 ADIF，以及把你手动回写的状态（已上传 / 已确认）合并进日志。",
  "「已上传」与「已确认」是两件事：上传只表示 LoTW 收到了你的日志，确认要等对方也上传、且两边关键字段一致。",
  "对不上的 QSO 会出现在「同步 QSL」的「本地缺失」里：多半是时间差了几分钟、波段或模式的写法不同，或者本地那条呼号抄错了。",
  "证书有有效期，到期后上传会被拒；换呼号（例如从 B 类升到 C 类）要为新呼号单独申请证书并新建 Station Location。",
  "QSL 卡片与 LoTW 是两条独立的确认渠道：同一批通联可以既寄纸卡又走 LoTW，日志里的两位标志位互不影响。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn eqsl_data_populated() {
    assert!(EQSL_SERVICES.len() >= 3);
    assert!(!EQSL_NOTES.is_empty());
  }

  #[test]
  fn lotw_guide_covers_the_whole_loop() {
    // 步骤是「从申请证书一路到回写确认」的闭环，缺一环用户就断在中间。
    assert!(LOTW_UPLOAD_STEPS.len() >= 5, "步骤太少，多半漏了环节");
    let all: String = LOTW_UPLOAD_STEPS
      .iter()
      .map(|(step, desc)| format!("{step}{desc}"))
      .collect();
    for keyword in [
      "证书",             // 第一环：证书
      "Station Location", // 第二环：站址（分区填错是对不上的主因）
      "导出 ADIF",        // 第三环：从本站拿待签名的日志
      "TQSL",             // 第四环：签名工具
      ".tq8",             // 第五环：签名产物
      "同步 QSL",         // 第六环：把确认回写进来
    ] {
      assert!(all.contains(keyword), "流程里少了「{keyword}」这一环");
    }
  }

  #[test]
  fn lotw_steps_are_well_formed() {
    for (step, desc) in LOTW_UPLOAD_STEPS {
      assert!(!step.trim().is_empty(), "步骤名不能为空");
      assert!(
        desc.trim().len() >= 10,
        "「{step}」的说明太短，起不到指引作用"
      );
    }
    // 步骤名唯一（中文反向索引按原文找 key，重名会撞车）。
    let mut names: Vec<&str> = LOTW_UPLOAD_STEPS.iter().map(|(s, _)| *s).collect();
    names.sort_unstable();
    let before = names.len();
    names.dedup();
    assert_eq!(names.len(), before, "步骤名重复");
  }

  #[test]
  fn lotw_notes_state_the_no_proxy_signing_boundary() {
    // 这条边界是本模块存在的理由之一：不代签、不碰私钥、只合并状态。
    assert!(
      LOTW_UPLOAD_NOTES
        .iter()
        .any(|n| n.contains("私钥") && n.contains("代签")),
      "必须明说「不代签、不碰私钥」"
    );
    assert!(
      LOTW_UPLOAD_NOTES
        .iter()
        .any(|n| n.contains("已上传") && n.contains("已确认")),
      "必须区分「已上传」与「已确认」"
    );
    for note in LOTW_UPLOAD_NOTES {
      assert!(!note.trim().is_empty());
    }
  }
}
