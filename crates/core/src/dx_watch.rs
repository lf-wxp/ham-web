//! DX 热点与本地日志对照：是否需要（新 DXCC / 新波段）、关注呼号匹配与提醒判定。

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::dxcc::lookup;
use crate::logbook::LogEntry;

/// 波段 / 模式无法归类时的取值（中文原文，界面经 `t()` 的原文反查翻成当前语言）。
///
/// 常量而不是散落的字面量：页面要拿它做「是不是未知」的判断，也要把它显示出来 ——
/// 判断用这个常量、显示走 `t()`，才不会在英文 / 西语界面里露出中文。
pub const OTHER: &str = "其他";

/// 频率 kHz → 业余波段（DX Cluster 常见波段），无法归类时为 [`OTHER`]。
#[must_use]
pub const fn band_of_khz(khz: u32) -> &'static str {
  match khz {
    1800..=2000 => "160m",
    3500..=4000 => "80m",
    5351..=5367 => "60m",
    7000..=7300 => "40m",
    10100..=10150 => "30m",
    14000..=14350 => "20m",
    18068..=18168 => "17m",
    21000..=21450 => "15m",
    24890..=24990 => "12m",
    28000..=29700 => "10m",
    50000..=54000 => "6m",
    144000..=148000 => "2m",
    _ => OTHER,
  }
}

/// 频率 kHz → MHz 数字串（至多 3 位小数、去尾随 0）；`0` 表示「上游没给出可用频率」，返回 `None`。
///
/// 为什么不直接把 0 格式化成 `"0"`：页面会把它显示成「0 MHz」，点呼号记日志时还会把 `0`
/// 当频率写进用户的日志。「没有频率」必须是一个能被调用方显式处理的状态，而不是一个看着合法的数。
#[must_use]
pub fn format_mhz(khz: u32) -> Option<String> {
  if khz == 0 {
    return None;
  }
  let mut s = format!("{:.3}", f64::from(khz) / 1000.0);
  while s.ends_with('0') {
    s.pop();
  }
  if s.ends_with('.') {
    s.pop();
  }
  Some(s)
}

/// 频率展示串：`x MHz`；上游没给出可用频率（0）时是占位破折号「—」，不是「0 MHz」。
///
/// `/dx-spots` 与仪表盘的热点摘要卡共用这一份，两处的频率口径不能各写一遍。
#[must_use]
pub fn freq_label(khz: u32) -> String {
  format_mhz(khz).map_or_else(|| "—".to_owned(), |mhz| format!("{mhz} MHz"))
}

/// 从 spot 备注推断模式，无法判断时为「其他」。
#[must_use]
pub fn mode_of_comment(comment: &str) -> &'static str {
  let c = comment.to_uppercase();
  ["FT8", "FT4", "RTTY", "CW", "SSB", "PSK"]
    .into_iter()
    .find(|m| c.contains(m))
    .unwrap_or(OTHER)
}

/// 某条 spot 相对本地日志的需要程度。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Need {
  /// 日志中从未通联过该 DXCC 实体。
  NewDxcc,
  /// 通联过该实体，但不是这个波段。
  NewBand,
  /// 这个呼号在这个波段已经联过。
  Worked,
  None,
}

impl Need {
  /// 是否值得去联（新 DXCC 或新波段）。
  #[must_use]
  pub const fn is_needed(self) -> bool {
    matches!(self, Self::NewDxcc | Self::NewBand)
  }
}

/// 本地日志的已通联索引。
#[derive(Debug, Default)]
pub struct Worked {
  dxcc: HashSet<u16>,
  dxcc_band: HashSet<(u16, String)>,
  call_band: HashSet<(String, String)>,
}

impl Worked {
  #[must_use]
  pub fn from_entries(entries: &[LogEntry]) -> Self {
    let mut w = Self::default();
    for e in entries {
      let band = e.band_label();
      if let Some(en) = e.entity() {
        w.dxcc.insert(en.dxcc);
        w.dxcc_band.insert((en.dxcc, band.clone()));
      }
      w.call_band
        .insert((e.callsign.trim().to_ascii_uppercase(), band));
    }
    w
  }

  /// 判断 `dx` 在 `band` 上是否需要。
  #[must_use]
  pub fn need(&self, dx: &str, band: &str) -> Need {
    if self
      .call_band
      .contains(&(dx.trim().to_ascii_uppercase(), band.to_owned()))
    {
      return Need::Worked;
    }
    match lookup(dx) {
      Some(en) if !self.dxcc.contains(&en.dxcc) => Need::NewDxcc,
      Some(en) if band != "其他" && !self.dxcc_band.contains(&(en.dxcc, band.to_owned())) => {
        Need::NewBand
      }
      _ => Need::None,
    }
  }
}

/// 呼号是否匹配关注模式：不区分大小写，`*` 匹配任意长度（如 `VP8*`、`*/P`），无 `*` 时需完全相同。
#[must_use]
pub fn call_matches(pattern: &str, call: &str) -> bool {
  let p = pattern.trim().to_ascii_uppercase();
  let c = call.trim().to_ascii_uppercase();
  if p.is_empty() {
    return false;
  }
  let parts: Vec<&str> = p.split('*').collect();
  if parts.len() == 1 {
    return p == c;
  }
  let (first, last) = (parts[0], parts[parts.len() - 1]);
  if !c.starts_with(first) || c.len() < first.len() + last.len() || !c.ends_with(last) {
    return false;
  }
  let mut rest = &c[first.len()..c.len() - last.len()];
  for mid in &parts[1..parts.len() - 1] {
    match rest.find(mid) {
      Some(i) => rest = &rest[i + mid.len()..],
      None => return false,
    }
  }
  true
}

/// DX 热点提醒设置（`localStorage` 的 `dx-alerts`）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlertSettings {
  /// 出现新 DXCC 时提醒。
  #[serde(default)]
  pub new_dxcc: bool,
  /// 出现已通联实体的新波段时提醒。
  #[serde(default)]
  pub new_band: bool,
  /// 关注的呼号模式（见 [`call_matches`]）。
  #[serde(default)]
  pub calls: Vec<String>,
}

/// 提醒原因。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlertReason {
  /// 命中关注呼号（模式）。
  Watched(String),
  NewDxcc,
  NewBand,
}

impl AlertSettings {
  /// 是否开启了任意提醒。
  #[must_use]
  pub fn enabled(&self) -> bool {
    self.new_dxcc || self.new_band || !self.calls.is_empty()
  }

  /// 一条 spot 是否需要提醒（关注呼号优先）。
  #[must_use]
  pub fn reason(&self, dx: &str, need: Need) -> Option<AlertReason> {
    if let Some(p) = self.calls.iter().find(|p| call_matches(p, dx)) {
      return Some(AlertReason::Watched(p.clone()));
    }
    match need {
      Need::NewDxcc if self.new_dxcc => Some(AlertReason::NewDxcc),
      Need::NewBand if self.new_band => Some(AlertReason::NewBand),
      _ => None,
    }
  }
}

/// 同一呼号同一波段只提醒一次的去重 key。
#[must_use]
pub fn alert_key(dx: &str, band: &str) -> String {
  format!("{}|{band}", dx.trim().to_ascii_uppercase())
}

#[cfg(test)]
mod tests {
  use super::*;

  fn qso(call: &str, freq: &str) -> LogEntry {
    LogEntry {
      callsign: call.into(),
      freq: freq.into(),
      ..Default::default()
    }
  }

  #[test]
  fn formats_mhz_and_treats_zero_as_unknown() {
    assert_eq!(format_mhz(14_074).as_deref(), Some("14.074"));
    assert_eq!(format_mhz(7_000).as_deref(), Some("7"));
    assert_eq!(format_mhz(1_842).as_deref(), Some("1.842"));
    assert_eq!(format_mhz(144_390).as_deref(), Some("144.39"));
    assert_eq!(format_mhz(1).as_deref(), Some("0.001"));
    // 0 = 上游没给出可用频率：不是「0 MHz」。
    assert_eq!(format_mhz(0), None);
  }

  #[test]
  fn freq_label_shows_a_dash_instead_of_zero_mhz() {
    assert_eq!(freq_label(14_074), "14.074 MHz");
    assert_eq!(freq_label(7_000), "7 MHz");
    assert_eq!(freq_label(0), "—");
  }

  #[test]
  fn infers_band_and_mode() {
    assert_eq!(band_of_khz(14_074), "20m");
    assert_eq!(band_of_khz(5_357), "60m");
    assert_eq!(band_of_khz(100), "其他");
    assert_eq!(mode_of_comment("ft8 -12dB"), "FT8");
    assert_eq!(mode_of_comment("up 2"), "其他");
  }

  #[test]
  fn need_against_log() {
    let w = Worked::from_entries(&[qso("JA1X", "14.074")]);
    assert_eq!(w.need("ja1x", "20m"), Need::Worked);
    assert_eq!(w.need("JA2Y", "20m"), Need::None);
    assert_eq!(w.need("JA2Y", "40m"), Need::NewBand);
    assert_eq!(w.need("JA2Y", "其他"), Need::None);
    assert_eq!(w.need("VK2ABC", "20m"), Need::NewDxcc);
    assert!(Need::NewBand.is_needed() && !Need::Worked.is_needed());
  }

  #[test]
  fn wildcard_matching() {
    assert!(call_matches("vp8*", "VP8LP"));
    assert!(call_matches("*/P", "BG4XX/P"));
    assert!(call_matches("3Y*B", "3Y0B"));
    assert!(call_matches("T*3*Z", "T33ZZ"));
    assert!(call_matches("K1AA", "k1aa"));
    assert!(!call_matches("K1AA", "K1AAB"));
    assert!(!call_matches("AB*BA", "ABA"));
    assert!(!call_matches("", "K1AA"));
  }

  #[test]
  fn alert_reasons() {
    let s = AlertSettings {
      new_dxcc: true,
      new_band: false,
      calls: vec!["VP8*".into()],
    };
    assert!(s.enabled());
    assert_eq!(
      s.reason("VP8LP", Need::Worked),
      Some(AlertReason::Watched("VP8*".into()))
    );
    assert_eq!(
      s.reason("VK2ABC", Need::NewDxcc),
      Some(AlertReason::NewDxcc)
    );
    assert_eq!(s.reason("JA2Y", Need::NewBand), None);
    assert!(!AlertSettings::default().enabled());
    assert_eq!(alert_key(" vp8lp ", "20m"), "VP8LP|20m");
  }
}
