//! QSL 生命周期状态机：把 6 个 ADIF 标志位收敛成一个**单一档位**的状态与四色档位。
//!
//! # 为什么需要它
//!
//! 数据模型（[`LogEntry`]）忠实保留了 ADIF 的 6 个独立标志位：纸卡寄出 / 收到、LoTW
//! 上传 / 确认、eQSL 寄出 / 确认。这 6 位可以组合出 64 种取值，但用户看列表时只想看到一个
//! 「到哪一步了」。页面上原先用 `if qsl_rcvd { 已确认 } else if qsl_sent { 已寄出 }` 就地推导，
//! 结果是**走 LoTW / eQSL 确认的通联全部显示成「—」**（看起来根本没寄过卡）。这里把推导
//! 收敛到唯一事实来源，并让 UI 只负责把状态映射成颜色与文案。
//!
//! # 状态与优先级
//!
//! ```text
//! 未寄出 ──▶ 已寄出（卡片局 / 直寄 / 方式未知）──▶ 电子已确认（LoTW / eQSL / QRZ）──▶ 纸质已收
//! ```
//!
//! 确认优先于「已寄出」：对方可能先寄卡，所以「确认」并不要求本地先寄。多个确认同时存在时
//! 取**最硬**的那个，顺序是 纸质 > LoTW > eQSL > QRZ：纸质卡是实物凭据，LoTW 有 ARRL 背书
//! （也是 DXCC 唯一认可的电子来源），eQSL 是纯电子确认，QRZ Logbook 的站内确认最弱
//! （既不在 DXCC 的认可来源里，也没有标准 ADIF 字段）。这四档正好映射到徽章的四个颜色
//! （[`QslTone`]）。
//!
//! 「已寄出」按**任一途径**判定：`qsl_sent`（纸卡）、`lotw_sent`（上传）、`eqsl_sent`（寄出）
//! 任一为真即算已寄出；只有电子渠道已寄出时没有纸卡方式，落到 `QslVia::None`（方式未知）。
//! 这样才与 [`crate::logbook::pending_qsl`]「已寄出但未确认」的口径一致 —— 否则一条
//! LoTW 已上传未确认的通联会在徽章上显示「未寄出」，同时出现在待追卡清单里。
//!
//! # 与现有 API 的关系
//!
//! - [`LogEntry::confirmed`]：只要任一途径确认即为真（用于筛选 / 统计），本模块不替代它；
//! - [`crate::logbook::pending_qsl`]：已寄出但未确认的追卡清单，同样按「任一途径」判定；
//! - 本模块只负责「单一档位」的推导与四色，供列表徽章与筛选使用。

use serde::{Deserialize, Serialize};

use crate::logbook::LogEntry;

/// 纸质卡片的寄出方式（ADIF `QSL_SENT_VIA`）。
///
/// 反序列化走 [`QslVia::from_key`]（见下面的 `QslViaWire`）：这个字段落在整份日志的
/// JSON 里，而 `#[derive(Deserialize)]` 遇到枚举里没有的取值会**整份解析失败** ——
/// 一条手改过的记录就足以让日志读成空。宽容处理与 `from_key` / `from_adif_code`
/// 的既有口径一致：认不出就按「未寄出 / 未知」。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "QslViaWire", rename_all = "lowercase")]
pub enum QslVia {
  /// 未寄出，或已寄出但方式未知（旧数据、ADIF 里没写 `QSL_SENT_VIA`）。
  #[default]
  None,
  /// 经卡片局。
  Bureau,
  /// 直寄。
  Direct,
}

impl From<String> for QslVia {
  fn from(s: String) -> Self {
    Self::from_key(&s)
  }
}

/// [`QslVia`] 的反序列化中间体：只认字符串，其余（`null` / 数字 / 布尔）一律按「未知」。
///
/// 用 [`serde::de::IgnoredAny`] 兜底而不是只接 `String`：字段是 `null` 或数字时
/// `String` 同样会解析失败，而它落在整份日志的 JSON 里 —— 一条脏记录就够让整库读成空。
#[derive(Deserialize)]
#[serde(untagged)]
enum QslViaWire {
  Text(String),
  Other(serde::de::IgnoredAny),
}

impl From<QslViaWire> for QslVia {
  fn from(wire: QslViaWire) -> Self {
    match wire {
      QslViaWire::Text(s) => Self::from_key(&s),
      QslViaWire::Other(_) => Self::None,
    }
  }
}

impl QslVia {
  /// 全部取值，表单 / 下拉按此顺序展示。
  pub const ALL: [Self; 3] = [Self::None, Self::Bureau, Self::Direct];

  /// 稳定的短键（下拉框的值、持久化用）。
  #[must_use]
  pub const fn key(self) -> &'static str {
    match self {
      Self::None => "none",
      Self::Bureau => "bureau",
      Self::Direct => "direct",
    }
  }

  /// 由短键还原；无法识别时按[`QslVia::None`]（未寄出 / 未知）处理。
  #[must_use]
  pub fn from_key(key: &str) -> Self {
    match key.trim().to_ascii_lowercase().as_str() {
      "bureau" => Self::Bureau,
      "direct" => Self::Direct,
      _ => Self::None,
    }
  }

  /// ADIF 单字母代码（`B` 卡片局 / `D` 直寄）；「未寄出 / 未知」没有对应代码。
  ///
  /// ADIF 还定义了 `E`（电子投递）与 `M`（经 QSL 管理员），本工具不区分这两者，导入时按
  /// 「未知」处理 —— 与其猜一个语义，不如不写（导出时也就不会凭空生成这两个代码）。
  #[must_use]
  pub const fn adif_code(self) -> Option<&'static str> {
    match self {
      Self::Bureau => Some("B"),
      Self::Direct => Some("D"),
      Self::None => None,
    }
  }

  /// 由 ADIF 代码还原（大小写不敏感）。
  #[must_use]
  pub fn from_adif_code(code: &str) -> Self {
    match code.trim().to_ascii_uppercase().as_str() {
      "B" => Self::Bureau,
      "D" => Self::Direct,
      _ => Self::None,
    }
  }
}

/// 确认途径。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QslConfirm {
  /// 纸质卡片已收到。
  Paper,
  /// LoTW 已确认。
  Lotw,
  /// eQSL 已确认。
  Eqsl,
  /// QRZ Logbook 已确认（本工具的扩展字段，见 [`crate::adif::APP_QRZ_RCVD`]）。
  Qrz,
}

/// 徽章的四色档位（颜色本身由前端映射，这里只给出语义档位，保持浏览器无关、可单测）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QslTone {
  /// 未寄出（灰）。
  Muted,
  /// 已寄出待确认（琥珀）。
  Amber,
  /// 电子已确认（蓝）。
  Sky,
  /// 纸质已收（绿）。
  Emerald,
}

impl QslTone {
  /// 全部档位，筛选下拉按此顺序展示。
  pub const ALL: [Self; 4] = [Self::Muted, Self::Amber, Self::Sky, Self::Emerald];

  /// 稳定的短键（筛选下拉的值）。
  #[must_use]
  pub const fn key(self) -> &'static str {
    match self {
      Self::Muted => "muted",
      Self::Amber => "amber",
      Self::Sky => "sky",
      Self::Emerald => "emerald",
    }
  }

  /// 由短键还原；无法识别时返回 `None`（筛选时按「全部」处理）。
  #[must_use]
  pub fn from_key(key: &str) -> Option<Self> {
    Self::ALL.into_iter().find(|t| t.key() == key)
  }
}

/// QSL 生命周期状态（单一档位）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QslStatus {
  /// 未寄出。
  NotSent,
  /// 已寄出，尚未确认；括号内是寄出方式（可能未知）。
  Sent(QslVia),
  /// 已确认；括号内是确认途径。
  Confirmed(QslConfirm),
}

impl QslStatus {
  /// 从记录派生状态。优先级见模块文档：纸质 > LoTW > eQSL > 已寄出 > 未寄出。
  #[must_use]
  pub fn of(e: &LogEntry) -> Self {
    if e.qsl_rcvd {
      return Self::Confirmed(QslConfirm::Paper);
    }
    if e.lotw_rcvd {
      return Self::Confirmed(QslConfirm::Lotw);
    }
    if e.eqsl_rcvd {
      return Self::Confirmed(QslConfirm::Eqsl);
    }
    if e.qrz_rcvd {
      return Self::Confirmed(QslConfirm::Qrz);
    }
    if e.qsl_sent {
      return Self::Sent(e.qsl_sent_via);
    }
    // 电子渠道的「已寄出」也算已寄出：LoTW 上传 / eQSL 寄出而未确认时，若只认 `qsl_sent`，
    // 徽章会显示「未寄出」，而同一条通联又出现在「待追卡」清单里（`pending_qsl` 按
    // **任一途径**判定），两处结论相反。这个档位没有纸卡寄出方式，取「方式未知」。
    if e.lotw_sent || e.eqsl_sent {
      return Self::Sent(QslVia::None);
    }
    Self::NotSent
  }

  /// 是否已被任一途径确认（与 [`LogEntry::confirmed`] 同口径）。
  #[must_use]
  pub const fn is_confirmed(self) -> bool {
    matches!(self, Self::Confirmed(_))
  }

  /// 徽章颜色档位。
  #[must_use]
  pub const fn tone(self) -> QslTone {
    match self {
      Self::NotSent => QslTone::Muted,
      Self::Sent(_) => QslTone::Amber,
      // 三种电子确认共用蓝色：四色档位要的是「走到哪一步」，
      // 具体是 LoTW / eQSL / QRZ 由徽章文案区分。
      Self::Confirmed(QslConfirm::Paper) => QslTone::Emerald,
      Self::Confirmed(QslConfirm::Lotw | QslConfirm::Eqsl | QslConfirm::Qrz) => QslTone::Sky,
    }
  }
}

/// 标记为已寄出（记录寄出方式）。
///
/// **不碰确认标志**：对方可能先寄卡过来，确认与本地寄出是两件独立的事；把已确认的记录
/// 「回退成已寄出」是很危险的静默数据丢失。返回是否真的改动了记录 —— 已寄出且方式相同时
/// 返回 `false`，调用方据此避免无意义的落库与提示。
///
/// `via` 传[`QslVia::None`]表示「已寄出但方式未知」（例如从 ADIF 导入的、没带
/// `QSL_SENT_VIA` 的老记录）。
#[must_use]
pub fn mark_sent(e: &mut LogEntry, via: QslVia) -> bool {
  let changed = !e.qsl_sent || e.qsl_sent_via != via;
  e.qsl_sent = true;
  e.qsl_sent_via = via;
  changed
}

#[cfg(test)]
mod tests {
  use super::*;

  fn entry() -> LogEntry {
    LogEntry {
      callsign: "JA1X".into(),
      ..Default::default()
    }
  }

  #[test]
  fn single_flags_map_to_the_documented_states() {
    // 未寄出 → 已寄出（方式未知 / 卡片局）。
    let mut e = entry();
    assert_eq!(QslStatus::of(&e), QslStatus::NotSent);
    e.qsl_sent = true;
    assert_eq!(QslStatus::of(&e), QslStatus::Sent(QslVia::None));
    e.qsl_sent_via = QslVia::Bureau;
    assert_eq!(QslStatus::of(&e), QslStatus::Sent(QslVia::Bureau));

    // 三种确认途径分别单独成立。
    for flag in [
      QslConfirm::Qrz,
      QslConfirm::Eqsl,
      QslConfirm::Lotw,
      QslConfirm::Paper,
    ] {
      let mut e = entry();
      match flag {
        QslConfirm::Qrz => e.qrz_rcvd = true,
        QslConfirm::Eqsl => e.eqsl_rcvd = true,
        QslConfirm::Lotw => e.lotw_rcvd = true,
        QslConfirm::Paper => e.qsl_rcvd = true,
      }
      assert_eq!(
        QslStatus::of(&e),
        QslStatus::Confirmed(flag),
        "标志位 → 状态"
      );
      // 电子确认都是蓝色档位，纸质是绿色。
      let tone = QslStatus::of(&e).tone();
      assert_eq!(
        tone,
        if flag == QslConfirm::Paper {
          QslTone::Emerald
        } else {
          QslTone::Sky
        }
      );
    }
  }

  #[test]
  fn the_hardest_confirmation_wins() {
    // 四个确认同时存在时取最硬的：纸质 > LoTW > eQSL > QRZ。逐个摘掉验证链条。
    let mut e = entry();
    e.qrz_rcvd = true;
    assert_eq!(QslStatus::of(&e), QslStatus::Confirmed(QslConfirm::Qrz));
    e.eqsl_rcvd = true;
    assert_eq!(QslStatus::of(&e), QslStatus::Confirmed(QslConfirm::Eqsl));
    e.lotw_rcvd = true;
    assert_eq!(QslStatus::of(&e), QslStatus::Confirmed(QslConfirm::Lotw));
    e.qsl_rcvd = true;
    assert_eq!(QslStatus::of(&e), QslStatus::Confirmed(QslConfirm::Paper));
  }

  #[test]
  fn electronic_sending_counts_as_sent() {
    // 只有电子渠道已寄出（未确认）时也算「已寄出·方式未知」：与 `pending_qsl`
    // 的「任一途径」口径一致，不能显示成「未寄出」。
    let mut e = entry();
    e.lotw_sent = true;
    assert_eq!(QslStatus::of(&e), QslStatus::Sent(QslVia::None));
    assert_eq!(QslStatus::of(&e).tone(), QslTone::Amber);

    let mut e = entry();
    e.eqsl_sent = true;
    assert_eq!(QslStatus::of(&e), QslStatus::Sent(QslVia::None));

    // 电子已确认仍然盖过「已寄出」。
    let mut e = entry();
    e.lotw_sent = true;
    e.lotw_rcvd = true;
    assert_eq!(QslStatus::of(&e), QslStatus::Confirmed(QslConfirm::Lotw));

    // 与 `pending_qsl` 对齐：凡是进入待追卡清单、又尚未确认的，都不该是「未寄出」。
    let mut e = entry();
    for setup in 0..3 {
      e.qsl_sent = setup == 0;
      e.lotw_sent = setup == 1;
      e.eqsl_sent = setup == 2;
      assert_ne!(
        QslStatus::of(&e),
        QslStatus::NotSent,
        "已寄出档位 {setup} 不应显示未寄出"
      );
    }
  }

  #[test]
  fn confirmations_outrank_sending_and_each_other() {
    let mut e = entry();
    // 已寄出（直寄）+ 只收到 eQSL 确认：状态仍是「电子已确认」，方式被确认盖过。
    e.qsl_sent = true;
    e.qsl_sent_via = QslVia::Direct;
    e.eqsl_rcvd = true;
    assert_eq!(QslStatus::of(&e), QslStatus::Confirmed(QslConfirm::Eqsl));

    // 电子确认 + LoTW 确认：取更硬的 LoTW。
    e.lotw_rcvd = true;
    assert_eq!(QslStatus::of(&e), QslStatus::Confirmed(QslConfirm::Lotw));

    // 再加上纸质卡：取最硬的纸质。
    e.qsl_rcvd = true;
    assert_eq!(QslStatus::of(&e), QslStatus::Confirmed(QslConfirm::Paper));
  }

  #[test]
  fn every_tone_is_reachable_and_matches_the_ladder() {
    let mut seen = Vec::new();
    for (setup, tone) in [
      (0u8, QslTone::Muted),
      (1, QslTone::Amber),
      (2, QslTone::Sky),
      (3, QslTone::Emerald),
    ] {
      let mut e = entry();
      if setup >= 1 {
        e.qsl_sent = true;
      }
      if setup == 2 {
        e.lotw_rcvd = true;
      }
      if setup == 3 {
        e.qsl_rcvd = true;
      }
      let got = QslStatus::of(&e).tone();
      assert_eq!(got, tone, "档位 {setup}");
      seen.push(got);
    }
    seen.sort_by_key(|t| t.key());
    seen.dedup();
    assert_eq!(seen.len(), QslTone::ALL.len(), "四色档位都应可达");

    // 三种电子确认共用蓝色档位。
    for set in [
      (|e: &mut LogEntry| e.eqsl_rcvd = true) as fn(&mut LogEntry),
      |e: &mut LogEntry| e.qrz_rcvd = true,
    ] {
      let mut e = entry();
      set(&mut e);
      assert_eq!(QslStatus::of(&e).tone(), QslTone::Sky);
    }
  }

  #[test]
  fn is_confirmed_matches_the_existing_helper() {
    // 16 种组合：四位标志位都要覆盖到，否则新增渠道会从这条等价性检查里漏过去。
    for flags in 0..16u8 {
      let mut e = entry();
      e.qsl_rcvd = flags & 1 != 0;
      e.lotw_rcvd = flags & 2 != 0;
      e.eqsl_rcvd = flags & 4 != 0;
      e.qrz_rcvd = flags & 8 != 0;
      assert_eq!(
        QslStatus::of(&e).is_confirmed(),
        e.confirmed(),
        "组合 {flags}"
      );
    }
  }

  #[test]
  fn mark_sent_records_the_route_without_touching_confirmations() {
    let mut e = entry();
    e.eqsl_rcvd = true;
    assert!(mark_sent(&mut e, QslVia::Bureau));
    assert!(e.qsl_sent);
    assert_eq!(e.qsl_sent_via, QslVia::Bureau);
    // 已经确认过的记录不会被「标为已寄出」拉回未确认。
    assert!(e.eqsl_rcvd);
    assert_eq!(QslStatus::of(&e), QslStatus::Confirmed(QslConfirm::Eqsl));
  }

  #[test]
  fn mark_sent_is_idempotent_but_reports_route_changes() {
    let mut e = entry();
    assert!(mark_sent(&mut e, QslVia::Bureau), "首次应报告改动");
    assert!(!mark_sent(&mut e, QslVia::Bureau), "重复标记不应报告改动");
    assert!(mark_sent(&mut e, QslVia::Direct), "换方式应报告改动");
    assert_eq!(e.qsl_sent_via, QslVia::Direct);
  }

  #[test]
  fn via_keys_and_adif_codes_round_trip() {
    for via in QslVia::ALL {
      assert_eq!(QslVia::from_key(via.key()), via, "短键往返");
      if let Some(code) = via.adif_code() {
        assert_eq!(QslVia::from_adif_code(code), via, "ADIF 代码往返");
        assert_eq!(QslVia::from_adif_code(&code.to_lowercase()), via);
      }
    }
    // 未建模的代码与垃圾输入一律落到「未知」，绝不 panic。
    assert_eq!(QslVia::from_adif_code("E"), QslVia::None);
    assert_eq!(QslVia::from_adif_code("M"), QslVia::None);
    assert_eq!(QslVia::from_adif_code("?"), QslVia::None);
    assert_eq!(QslVia::from_key("卡片局"), QslVia::None);
    assert_eq!(QslVia::None.adif_code(), None);
    assert_eq!(QslVia::from_key("none").adif_code(), None);
  }

  #[test]
  fn tone_keys_are_unique_and_parse_back() {
    for tone in QslTone::ALL {
      assert_eq!(QslTone::from_key(tone.key()), Some(tone));
    }
    assert_eq!(QslTone::from_key(""), None);
    assert_eq!(QslTone::from_key("purple"), None);
  }

  #[test]
  fn legacy_json_without_the_via_field_still_loads() {
    // 旧数据（以及不带 QSL_SENT_VIA 的 ADIF 导入）里没有这个字段：默认「未知」。
    let mut e: LogEntry = serde_json::from_str(
      r#"{"id":1,"date":"2026-01-01","time":"00:00","freq":"14.074","mode":"FT8",
          "callsign":"JA1X","remark":"","qsl_sent":true}"#,
    )
    .expect("旧 JSON 应能反序列化");
    assert_eq!(e.qsl_sent_via, QslVia::None);
    assert_eq!(QslStatus::of(&e), QslStatus::Sent(QslVia::None));
    // 序列化后再读回来，方式仍然保持。
    let _ = mark_sent(&mut e, QslVia::Direct);
    let json = serde_json::to_string(&e).expect("序列化");
    assert!(json.contains("\"qsl_sent_via\":\"direct\""), "{json}");
    let back: LogEntry = serde_json::from_str(&json).expect("往返");
    assert_eq!(back.qsl_sent_via, QslVia::Direct);
  }

  #[test]
  fn an_unrecognised_via_value_does_not_break_the_whole_logbook() {
    // 严格枚举在这里的代价特别大：`qsl_sent_via` 落在整份日志的 JSON 里，一条手改过的
    // 记录（或别的软件写进来的 `E` / `M`）会让**整份日志**解析失败，界面直接读成空日志。
    // 认不出就按「未寄出 / 未知」，与 `from_key` / `from_adif_code` 同一口径。
    for raw in [
      "\"e\"",
      "\"M\"",
      "\"Bureau\"",
      "\"\"",
      "\"direct\"",
      // 非字符串（`null` / 数字 / 布尔）同样不能拖垮整库：旧版本与手改数据都可能留下。
      "null",
      "3",
      "true",
    ] {
      let json = format!(
        r#"{{"id":1,"date":"2026-01-01","time":"00:00","freq":"14.074","mode":"FT8",
            "callsign":"JA1X","remark":"","qsl_sent":true,"qsl_sent_via":{raw}}}"#
      );
      serde_json::from_str::<LogEntry>(&json).unwrap_or_else(|e| panic!("{raw} 应能反序列化：{e}"));
    }
    let via = |raw: &str| {
      let json = format!(
        r#"{{"id":1,"date":"2026-01-01","time":"00:00","freq":"14.074","mode":"FT8",
            "callsign":"JA1X","remark":"","qsl_sent":true,"qsl_sent_via":{raw}}}"#
      );
      serde_json::from_str::<LogEntry>(&json)
        .expect("反序列化")
        .qsl_sent_via
    };
    assert_eq!(via("\"e\""), QslVia::None);
    assert_eq!(via("\"M\""), QslVia::None);
    assert_eq!(via("null"), QslVia::None);
    assert_eq!(via("3"), QslVia::None);
    assert_eq!(via("true"), QslVia::None);
    // 大小写不敏感：手写的 `Direct` 也算数。
    assert_eq!(via("\"Direct\""), QslVia::Direct);
  }
}
