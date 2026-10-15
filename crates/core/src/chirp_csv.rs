//! 中继台 → CHIRP 通用 CSV：手台 / 车台写频软件 CHIRP 可直接导入的频道表。
//!
//! 列顺序与取值沿用 CHIRP 通用 CSV 的约定（`Location,Name,Frequency,Duplex,Offset,Tone,…`）：
//! `Duplex` 取 `""`（同频）/ `+` / `-`，`Offset` 是频差的**绝对值**（MHz，6 位小数），
//! 中继台用 `Tone`（只发射亚音，由 `rToneFreq` 给出）。没用到的列按 CHIRP 自己导出时的默认值填写。
//!
//! 边界：
//! - 这里只生成文件；没有真机、也没有 CHIRP 可对拍，列定义来自 CHIRP 文档约定，单测只钉住表头与
//!   一行样例，**未在真实 CHIRP 里实测导入**；
//! - 亚音只认 [`crate::tone_squelch::CTCSS_TONES`] 里的 CTCSS；DCS、`CSQ`、带说明文字的写法不猜，
//!   整条跳过并计数，而不是悄悄导出成「无亚音」的中继（那样的频道发出去进不了中继）；
//! - 不写功率、步进、跳扫：交给 CHIRP 与具体机型的默认值。

use crate::bands::AMATEUR_BAND_EDGES;
use crate::tone_squelch::CTCSS_TONES;

/// CHIRP 通用 CSV 的表头（单一事实来源，页面与测试共用）。
pub const HEADER: &str = "Location,Name,Frequency,Duplex,Offset,Tone,rToneFreq,cToneFreq,DtcsCode,DtcsPolarity,RxDtcsCode,CrossMode,Mode,TStep,Skip,Power,Comment,URCALL,RPT1CALL,RPT2CALL,DVCODE";

/// CHIRP 自己导出时，没用到的亚音列的默认值。
const DEFAULT_TONE_HZ: f64 = 88.5;

/// 频道名最多保留的字符数：多数手台名称只有 6–8 个字符，超出由 CHIRP 按机型再截。
const MAX_NAME_CHARS: usize = 8;

/// 一条待导出的中继台（字段是 `/api/repeaters` 返回的原样字符串）。
#[derive(Debug, Clone, Copy)]
pub struct RepeaterRow<'a> {
  pub callsign: &'a str,
  /// 接收（下行）频率，MHz。
  pub frequency_mhz: &'a str,
  /// 带符号的频差，MHz（发射 = 接收 + 频差），如 `-0.6000`。
  pub offset_mhz: &'a str,
  /// 亚音，如 `88.5`；空表示无亚音。
  pub pl: &'a str,
  pub city: &'a str,
  pub status: &'a str,
}

/// 导出选项。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportOptions {
  /// 只导出接收与发射频率都落在业余波段表内的频点（按全站 [`AMATEUR_BAND_EDGES`] 的口径）。
  pub only_amateur_bands: bool,
  /// 跳过状态里写着 off-air（已停用）的中继。
  pub skip_off_air: bool,
}

impl Default for ExportOptions {
  fn default() -> Self {
    Self {
      only_amateur_bands: true,
      skip_off_air: true,
    }
  }
}

/// 被跳过的原因（按判定顺序，一条只记第一个命中的原因）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Skip {
  /// 接收频率缺失或不合法。
  BadFrequency,
  OffAir,
  /// 接收或发射频率在业余波段表之外。
  OutOfBand,
  /// 亚音不是标准 CTCSS。
  UnsupportedTone,
}

/// 各原因的跳过条数。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SkipCounts {
  pub bad_frequency: usize,
  pub off_air: usize,
  pub out_of_band: usize,
  pub unsupported_tone: usize,
}

impl SkipCounts {
  fn add(&mut self, s: Skip) {
    match s {
      Skip::BadFrequency => self.bad_frequency += 1,
      Skip::OffAir => self.off_air += 1,
      Skip::OutOfBand => self.out_of_band += 1,
      Skip::UnsupportedTone => self.unsupported_tone += 1,
    }
  }

  #[must_use]
  pub fn total(&self) -> usize {
    self.bad_frequency + self.off_air + self.out_of_band + self.unsupported_tone
  }
}

/// 导出结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Export {
  /// 含表头的完整 CSV 文本（`\r\n` 换行，与 CHIRP 所用的 Python `csv` 模块一致）。
  pub csv: String,
  pub exported: usize,
  pub skipped: SkipCounts,
}

fn in_amateur_band(mhz: f64) -> bool {
  AMATEUR_BAND_EDGES
    .iter()
    .any(|&(_, lo, hi)| (lo..hi).contains(&mhz))
}

fn parse_mhz(s: &str) -> Option<f64> {
  s.trim().parse::<f64>().ok().filter(|v| v.is_finite())
}

/// 标准 CTCSS 频率（容差 0.05 Hz，吸收 `88.50` / `88.5` 这类写法差异）。
fn ctcss(s: &str) -> Option<f64> {
  let hz = parse_mhz(s)?;
  CTCSS_TONES.iter().copied().find(|t| (t - hz).abs() < 0.05)
}

/// 频道名：只留 ASCII 字母、数字与少量标点，截到 [`MAX_NAME_CHARS`]。
fn channel_name(callsign: &str) -> String {
  callsign
    .trim()
    .chars()
    .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '/' | '.' | ' '))
    .take(MAX_NAME_CHARS)
    .collect::<String>()
    .trim()
    .to_string()
}

/// CSV 字段转义（RFC 4180）：含逗号、引号或换行时整体加引号并把引号双写。
/// 备注来自第三方数据：以 `= + - @` 开头的在 Excel 里会被当公式，前面补一个空格。
fn field(raw: &str, comment: bool) -> String {
  let mut s: String = raw.chars().filter(|c| !c.is_control()).collect();
  if comment && s.starts_with(['=', '+', '-', '@']) {
    s.insert(0, ' ');
  }
  if s.contains([',', '"']) {
    format!("\"{}\"", s.replace('"', "\"\""))
  } else {
    s
  }
}

/// 把中继台列表导出成 CHIRP 通用 CSV。`Location` 从 0 起连续编号（与 CHIRP 自己的导出一致）。
#[must_use]
pub fn export(rows: &[RepeaterRow<'_>], opts: ExportOptions) -> Export {
  let mut skipped = SkipCounts::default();
  let mut out = String::from(HEADER);
  out.push_str("\r\n");
  let mut exported = 0usize;

  for row in rows {
    match build(row, opts) {
      Err(s) => skipped.add(s),
      Ok(line) => {
        out.push_str(&format!("{exported},{line}\r\n"));
        exported += 1;
      }
    }
  }

  Export {
    csv: out,
    exported,
    skipped,
  }
}

/// 一行 CSV（不含 `Location` 列与换行），或跳过原因。
fn build(row: &RepeaterRow<'_>, opts: ExportOptions) -> Result<String, Skip> {
  let rx = parse_mhz(row.frequency_mhz)
    .filter(|v| *v > 0.0)
    .ok_or(Skip::BadFrequency)?;
  if opts.skip_off_air && row.status.to_ascii_lowercase().contains("off") {
    return Err(Skip::OffAir);
  }
  // 频差缺失按 0（同频）处理；写不出来的（非数字）才算坏数据。
  let offset = if row.offset_mhz.trim().is_empty() {
    0.0
  } else {
    parse_mhz(row.offset_mhz).ok_or(Skip::BadFrequency)?
  };
  let tx = rx + offset;
  if opts.only_amateur_bands && !(in_amateur_band(rx) && in_amateur_band(tx)) {
    return Err(Skip::OutOfBand);
  }
  let tone = if row.pl.trim().is_empty() {
    None
  } else {
    Some(ctcss(row.pl).ok_or(Skip::UnsupportedTone)?)
  };

  // 频差为 0 就是同频；极小的浮点残差不应产生 `+0.000000` 之类的假频差。
  let duplex = if offset.abs() < 1e-9 {
    ""
  } else if offset > 0.0 {
    "+"
  } else {
    "-"
  };
  let (tone_mode, r_tone) = match tone {
    Some(hz) => ("Tone", hz),
    None => ("", DEFAULT_TONE_HZ),
  };

  Ok(format!(
    "{name},{rx:.6},{duplex},{off:.6},{tone_mode},{r_tone:.1},{c_tone:.1},023,NN,023,Tone->Tone,FM,5.00,,,{comment},,,,",
    name = field(&channel_name(row.callsign), false),
    off = offset.abs(),
    c_tone = DEFAULT_TONE_HZ,
    comment = field(row.city.trim(), true),
  ))
}

#[cfg(test)]
mod tests {
  use super::*;

  fn row<'a>(callsign: &'a str, freq: &'a str, offset: &'a str, pl: &'a str) -> RepeaterRow<'a> {
    RepeaterRow {
      callsign,
      frequency_mhz: freq,
      offset_mhz: offset,
      pl,
      city: "Beijing",
      status: "On-air",
    }
  }

  fn lines(e: &Export) -> Vec<&str> {
    e.csv.split("\r\n").filter(|l| !l.is_empty()).collect()
  }

  #[test]
  fn header_has_the_21_chirp_columns() {
    let cols: Vec<&str> = HEADER.split(',').collect();
    assert_eq!(cols.len(), 21);
    assert_eq!(
      &cols[..6],
      ["Location", "Name", "Frequency", "Duplex", "Offset", "Tone"]
    );
    assert_eq!(cols[20], "DVCODE");
  }

  #[test]
  fn every_data_row_has_as_many_columns_as_the_header() {
    let rows = [
      row("BR1BJ", "145.6500", "-0.6000", "88.5"),
      row("BG1ABC", "439.4000", "-5.0000", ""),
      row("BY1XX", "145.5000", "+0.0000", ""),
    ];
    let e = export(&rows, ExportOptions::default());
    for l in lines(&e) {
      assert_eq!(l.split(',').count(), 21, "{l}");
    }
  }

  #[test]
  fn a_negative_offset_repeater_with_ctcss() {
    let e = export(
      &[row("BR1BJ", "145.6500", "-0.6000", "88.5")],
      ExportOptions::default(),
    );
    assert_eq!(lines(&e)[0], HEADER);
    assert_eq!(
      lines(&e)[1],
      "0,BR1BJ,145.650000,-,0.600000,Tone,88.5,88.5,023,NN,023,Tone->Tone,FM,5.00,,,Beijing,,,,"
    );
    assert_eq!(e.exported, 1);
    assert_eq!(e.skipped, SkipCounts::default());
  }

  #[test]
  fn duplex_follows_the_sign_and_offset_is_absolute() {
    let e = export(
      &[
        row("A", "145.6500", "-0.6000", ""),
        row("B", "145.0000", "+0.6000", ""),
        row("C", "145.5000", "+0.0000", ""),
        row("D", "145.5000", "", ""),
      ],
      ExportOptions::default(),
    );
    let l = lines(&e);
    assert!(l[1].starts_with("0,A,145.650000,-,0.600000,,"));
    assert!(l[2].starts_with("1,B,145.000000,+,0.600000,,"));
    assert!(l[3].starts_with("2,C,145.500000,,0.000000,,"));
    assert!(l[4].starts_with("3,D,145.500000,,0.000000,,"));
  }

  #[test]
  fn no_tone_leaves_mode_empty_and_chirp_defaults() {
    let e = export(
      &[row("A", "145.6500", "-0.6000", "")],
      ExportOptions::default(),
    );
    assert!(lines(&e)[1].contains(",,88.5,88.5,023,NN,023,Tone->Tone,FM,"));
  }

  #[test]
  fn tone_is_snapped_to_the_standard_ctcss_table() {
    for (pl, expect) in [("88.50", "88.5"), (" 100.0 ", "100.0"), ("123", "123.0")] {
      let e = export(
        &[row("A", "145.6500", "-0.6000", pl)],
        ExportOptions::default(),
      );
      assert!(
        lines(&e)[1].contains(&format!(",Tone,{expect},88.5,")),
        "{pl} -> {}",
        lines(&e)[1]
      );
    }
  }

  #[test]
  fn unsupported_tones_skip_the_whole_row_instead_of_dropping_the_tone() {
    for pl in ["DCS 023", "CSQ", "88.0", "0", "abc", "88.5 Hz"] {
      let e = export(
        &[row("A", "145.6500", "-0.6000", pl)],
        ExportOptions::default(),
      );
      assert_eq!(e.exported, 0, "{pl}");
      assert_eq!(e.skipped.unsupported_tone, 1, "{pl}");
    }
  }

  #[test]
  fn skip_reasons_are_counted_by_the_first_match() {
    let mut off = row("OFF", "145.6500", "-0.6000", "");
    off.status = "Off-air";
    let rows = [
      row("BAD", "", "-0.6000", ""),
      row("BAD2", "0", "-0.6000", ""),
      row("BAD3", "145.65", "x", ""),
      off,
      // 美国 70 cm（442 MHz）不在我国 430–440 MHz 内。
      row("US", "442.0000", "+5.0000", ""),
      // 接收在波段内、发射落到波段外。
      row("EDGE", "147.9000", "+0.6000", ""),
      row("TONE", "145.6500", "-0.6000", "DCS 023"),
      row("OK", "145.6500", "-0.6000", "88.5"),
    ];
    let e = export(&rows, ExportOptions::default());
    assert_eq!(
      e.skipped,
      SkipCounts {
        bad_frequency: 3,
        off_air: 1,
        out_of_band: 2,
        unsupported_tone: 1,
      }
    );
    assert_eq!(e.skipped.total(), 7);
    assert_eq!(e.exported, 1);
    // Location 只给真正导出的行编号。
    assert!(lines(&e)[1].starts_with("0,OK,"));
  }

  #[test]
  fn options_turn_the_filters_off() {
    let mut off = row("OFF", "145.6500", "-0.6000", "");
    off.status = "Off-air";
    let us = row("US", "442.0000", "+5.0000", "");
    let rows = [off, us];
    let strict = export(&rows, ExportOptions::default());
    assert_eq!(strict.exported, 0);
    let loose = export(
      &rows,
      ExportOptions {
        only_amateur_bands: false,
        skip_off_air: false,
      },
    );
    assert_eq!(loose.exported, 2);
    assert!(lines(&loose)[2].starts_with("1,US,442.000000,+,5.000000,"));
    // 关掉波段过滤时，坏频率仍然要跳过。
    let bad = export(
      &[row("BAD", "", "", "")],
      ExportOptions {
        only_amateur_bands: false,
        skip_off_air: false,
      },
    );
    assert_eq!(bad.skipped.bad_frequency, 1);
  }

  #[test]
  fn band_edges_are_half_open() {
    // 144.0 在 2 m 内；148.0 是上沿，不在内。
    assert_eq!(
      export(&[row("A", "144.0000", "", "")], ExportOptions::default()).exported,
      1
    );
    assert_eq!(
      export(&[row("A", "148.0000", "", "")], ExportOptions::default()).exported,
      0
    );
    assert_eq!(
      export(&[row("A", "143.9999", "", "")], ExportOptions::default()).exported,
      0
    );
  }

  #[test]
  fn names_are_ascii_and_short_and_comments_are_quoted() {
    let mut r = row("BR1/BJ-LONGNAME", "145.6500", "-0.6000", "");
    r.city = "Haidian, \"North\"";
    let e = export(&[r], ExportOptions::default());
    assert!(lines(&e)[1].starts_with("0,BR1/BJ-L,"));
    assert!(lines(&e)[1].contains(",\"Haidian, \"\"North\"\"\","));
    assert_eq!(channel_name("北京BR1"), "BR1");
    assert_eq!(channel_name("  AB  "), "AB");
  }

  #[test]
  fn comments_cannot_start_a_spreadsheet_formula() {
    for (city, want) in [
      ("=1+1", " =1+1"),
      ("@x", " @x"),
      ("-5", " -5"),
      ("+a", " +a"),
    ] {
      let mut r = row("A", "145.6500", "-0.6000", "");
      r.city = city;
      let e = export(&[r], ExportOptions::default());
      assert!(
        lines(&e)[1].contains(&format!(",{want},,,,")),
        "{city}: {}",
        lines(&e)[1]
      );
    }
    // 名称不会因此被改动，control 字符被剔除。
    let mut r = row("A", "145.6500", "-0.6000", "");
    r.city = "Bei\njing\r";
    assert!(lines(&export(&[r], ExportOptions::default()))[1].contains(",Beijing,"));
  }

  #[test]
  fn empty_input_is_just_the_header() {
    let e = export(&[], ExportOptions::default());
    assert_eq!(e.csv, format!("{HEADER}\r\n"));
    assert_eq!((e.exported, e.skipped.total()), (0, 0));
  }

  #[test]
  fn line_endings_are_crlf() {
    let e = export(
      &[row("A", "145.6500", "-0.6000", "")],
      ExportOptions::default(),
    );
    assert!(e.csv.ends_with("\r\n"));
    assert_eq!(e.csv.matches("\r\n").count(), 2);
    assert!(!e.csv.replace("\r\n", "").contains('\n'));
  }
}
