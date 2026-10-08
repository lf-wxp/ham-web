//! 电子 QSL 与日志确认：LoTW、eQSL、QRZ、Club Log 等现代确认方式。

use crate::dxcc;
use crate::logbook::StationInfo;

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

/// TQSL 的 Station Location 要填的字段值。
///
/// **这几项必须与导出 ADIF 里的 `MY_*` 一致**，否则 LoTW 会把 QSO 判成「不匹配」；
/// 而 TQSL 的对话框里没有任何交叉校验，填错了要等上传之后才发现。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LotwStationFields {
  /// 本台呼号（`STATION_CALLSIGN` / TQSL 的 Callsign）。
  pub callsign: String,
  /// 本台网格（`MY_GRIDSQUARE`）。
  pub gridsquare: String,
  /// DXCC 实体编号（`MY_DXCC` / TQSL 的 DXCC entity）。
  pub dxcc: Option<u16>,
  /// 实体中文名（界面展示用）。
  pub dxcc_name: Option<&'static str>,
  /// CQ 分区（`MY_CQ_ZONE`）。
  pub cq_zone: Option<u8>,
  /// ITU 分区（`MY_ITU_ZONE`）。
  pub itu_zone: Option<u8>,
}

impl LotwStationFields {
  /// 关键字段是否齐全：呼号（推 DXCC 与分区的前提）与网格（LoTW 判匹配时会用）都要有，
  /// 且呼号前缀能查出实体。
  #[must_use]
  pub fn is_complete(&self) -> bool {
    !self.callsign.trim().is_empty() && !self.gridsquare.trim().is_empty() && self.dxcc.is_some()
  }
}

/// 按本台信息算出 TQSL Station Location 的字段值。
///
/// **为什么要算**：这四个值就是 LoTW 匹配 QSO 时看的 `MY_*`，与 TQSL 里填的不一致时，
/// 上传不会报错、只会在 LoTW 端被标成「不匹配」。数据来源只有两处 —— 本台信息
/// （呼号 / 网格）与 [`crate::dxcc`]（实体编号与主分区，cty 数据的唯一事实来源），
/// 不另立一份分区表。
#[must_use]
pub fn lotw_station_fields(info: &StationInfo) -> LotwStationFields {
  let callsign = info.callsign.trim().to_ascii_uppercase();
  let entity = dxcc::lookup(&callsign);
  LotwStationFields {
    callsign,
    gridsquare: info.gridsquare.trim().to_ascii_uppercase(),
    dxcc: entity.map(|e| e.dxcc),
    dxcc_name: entity.map(|e| e.name),
    cq_zone: entity.map(|e| e.cq),
    itu_zone: entity.map(|e| e.itu),
  }
}

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

  fn station(callsign: &str, grid: &str) -> StationInfo {
    StationInfo {
      callsign: callsign.into(),
      gridsquare: grid.into(),
      ..Default::default()
    }
  }

  #[test]
  fn station_location_fields_come_from_the_dxcc_table() {
    let f = lotw_station_fields(&station(" bg4xxx ", " om89ew "));
    assert_eq!(f.callsign, "BG4XXX", "呼号规整成大写");
    assert_eq!(f.gridsquare, "OM89EW");
    assert_eq!(f.dxcc, Some(318), "中国 = 318");
    assert_eq!(f.dxcc_name, Some("中国"));
    assert_eq!(f.cq_zone, Some(24));
    assert_eq!(f.itu_zone, Some(44));
    assert!(f.is_complete());
  }

  #[test]
  fn an_unknown_prefix_leaves_the_entity_blank() {
    // 查不出实体就不猜：宁可让界面显示「填不出，先去补台站信息」，
    // 也不要给一个可能让对方判「不匹配」的分区。
    let f = lotw_station_fields(&station("QQ1QQQ", "OM89EW"));
    assert_eq!(f.callsign, "QQ1QQQ");
    assert_eq!(f.dxcc, None);
    assert_eq!(f.dxcc_name, None);
    assert_eq!(f.cq_zone, None);
    assert_eq!(f.itu_zone, None);
    assert!(!f.is_complete(), "查不出实体就不算齐全");
  }

  #[test]
  fn an_empty_station_yields_an_empty_table() {
    let f = lotw_station_fields(&StationInfo::default());
    assert!(f.callsign.is_empty() && f.gridsquare.is_empty());
    assert!(f.dxcc.is_none());
    assert!(!f.is_complete());
    // 有呼号但没网格：分区能算，网格这一栏空着 —— 仍然不算齐全（LoTW 判匹配要看网格）。
    let f = lotw_station_fields(&station("BG4XXX", ""));
    assert_eq!(f.dxcc, Some(318));
    assert!(!f.is_complete());
  }

  #[test]
  fn the_fields_match_what_the_exporter_writes() {
    // 这张表存在的唯一理由：与导出的 `MY_*` 一致。把这条契约钉成断言 ——
    // 哪天导出改了写法而这里没跟上，这条会红。
    let info = station("BG4XXX", "OM89EW");
    let fields = lotw_station_fields(&info);
    let book = crate::station::StationBook::from_info(&info);
    let entry = crate::logbook::LogEntry {
      callsign: "JA1AAA".into(),
      date: "2026-10-08".into(),
      time: "12:00".into(),
      freq: "14.074".into(),
      mode: "FT8".into(),
      ..Default::default()
    };
    let adif = crate::logbook::export_adif(&[entry], &book);
    assert!(adif.contains("<STATION_CALLSIGN:6>BG4XXX"), "{adif}");
    assert!(adif.contains("<MY_GRIDSQUARE:6>OM89EW"), "{adif}");
    assert_eq!(fields.callsign, "BG4XXX");
    assert_eq!(fields.gridsquare, "OM89EW");
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
