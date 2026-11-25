//! 从 APRS 信息字段里取出台站位置，并按站名汇总成可上地图的站点簿。
//!
//! 只认**未压缩**位置（数据类型 `!` `=` `/` `@`）：纬度 `ddmm.mmN` + 符号表 + 经度
//! `dddmm.mmE` + 符号码，共 19 字节。压缩位置（base-91）与 Mic-E（位置藏在目的地址里）
//! **没有解码器**，只计数并如实归类，**不猜坐标**。物件 / 条目 / 消息 / 状态等没有本台位置的
//! 类型归入 [`Report::Other`]，不算失败。
//!
//! 与 [`crate::aprs_codec::decode_position`] 的区别：后者要求符号表标识固定为 `/`，
//! 而真实报文里 `\`（备用符号表）与叠加字符很常见，所以这里按固定偏移取位。

use crate::aprs_codec::{decode_lat, decode_lon};

/// 一个已解出的位置。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fix {
  /// 纬度（十进制度，北正）。
  pub lat: f64,
  /// 经度（十进制度，东正）。
  pub lon: f64,
  /// 符号表标识：`/` 主表、`\` 备用表，或叠加字符。
  pub table: char,
  /// 符号码（决定图标）。
  pub symbol: char,
}

impl Fix {
  /// 与语言无关的坐标文字：`31.2417°N 121.4750°E`（南纬 / 西经用字母而不是负号）。
  #[must_use]
  pub fn label(&self) -> String {
    let ns = if self.lat < 0.0 { 'S' } else { 'N' };
    let ew = if self.lon < 0.0 { 'W' } else { 'E' };
    format!("{:.4}°{ns} {:.4}°{ew}", self.lat.abs(), self.lon.abs())
  }
}

/// 一条信息字段的分类结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Report {
  /// 未压缩位置，已解出。
  Position(Fix),
  /// 压缩位置：没有解码器。
  Compressed,
  /// Mic-E：位置在目的地址里，没有解码器。
  MicE,
  /// 是位置类型，但字段对不上（被截断、含模糊空格、数值越界）。
  Malformed,
  /// 不带本台位置的类型（消息、状态、物件、遥测……）。
  Other,
}

/// 未压缩位置体：纬度 8 + 符号表 1 + 经度 9 + 符号码 1。
const UNCOMPRESSED_LEN: usize = 19;
/// 压缩位置体：符号表 1 + 纬度 4 + 经度 4 + 符号码 1 + cs 2 + type 1。
const COMPRESSED_LEN: usize = 13;
/// 带时间戳的类型（`/` `@`）在位置前多 7 个字符（`DDHHMMz` / `HHMMSSh` / `MMDDHHMM`）。
const TIMESTAMP_LEN: usize = 7;

/// 分类一条 APRS 信息字段。
#[must_use]
pub fn classify(info: &[u8]) -> Report {
  let Some((&kind, rest)) = info.split_first() else {
    return Report::Other;
  };
  let body = match kind {
    b'!' | b'=' => rest,
    b'/' | b'@' => match rest.get(TIMESTAMP_LEN..) {
      Some(body) => body,
      None => return Report::Malformed,
    },
    // `` ` `` 与 `'` 是 Mic-E；0x1C / 0x1D 是旧版 Mic-E 数据类型。
    b'`' | b'\'' | 0x1c | 0x1d => return Report::MicE,
    _ => return Report::Other,
  };
  match body.first() {
    None => Report::Malformed,
    // 数字（或位置模糊用的空格）开头 = 纬度，未压缩。
    Some(b'0'..=b'9' | b' ') => uncompressed(body),
    // 符号表标识开头 = 压缩：`/` `\`、A–Z、a–j。
    Some(b'/' | b'\\' | b'A'..=b'Z' | b'a'..=b'j') => {
      if body.len() >= COMPRESSED_LEN {
        Report::Compressed
      } else {
        Report::Malformed
      }
    }
    Some(_) => Report::Malformed,
  }
}

fn uncompressed(body: &[u8]) -> Report {
  let Some(head) = body.get(..UNCOMPRESSED_LEN) else {
    return Report::Malformed;
  };
  // 前 19 字节必须是 ASCII：注释部分可以是任何字节，但位置本身不行。
  let Ok(text) = std::str::from_utf8(head) else {
    return Report::Malformed;
  };
  if !text.is_ascii() {
    return Report::Malformed;
  }
  let table = char::from(head[8]);
  if !table.is_ascii_graphic() {
    return Report::Malformed;
  }
  let symbol = char::from(head[18]);
  if !symbol.is_ascii_graphic() {
    return Report::Malformed;
  }
  match (decode_lat(&text[0..8]), decode_lon(&text[9..18])) {
    (Some(lat), Some(lon)) => Report::Position(Fix {
      lat,
      lon,
      table,
      symbol,
    }),
    _ => Report::Malformed,
  }
}

/// 各类结果的累计。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
  /// 解出位置的帧数。
  pub positioned: usize,
  /// 压缩位置（未解）。
  pub compressed: usize,
  /// Mic-E（未解）。
  pub mic_e: usize,
  /// 位置类型但字段对不上。
  pub malformed: usize,
}

/// 簿里最多留多少个站：满了就挤掉最久没更新的那个。
pub const MAX_STATIONS: usize = 200;

/// 簿里的一个站：最近一次解出的位置。
#[derive(Debug, Clone, PartialEq)]
pub struct Station {
  /// 站名，含 SSID（`BG1AAA-7`）。
  pub name: String,
  /// 最近一次位置。
  pub fix: Fix,
  /// 该站累计解出位置的帧数。
  pub reports: usize,
}

/// 收到的位置报文汇总：每个站只留最近一次位置，另有各类未解 / 无效的计数。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StationBook {
  stations: Vec<Station>,
  tally: Tally,
}

impl StationBook {
  /// 空簿。
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// 记一帧：`name` 是源站名（含 SSID），`info` 是信息字段。
  pub fn record(&mut self, name: &str, info: &[u8]) {
    match classify(info) {
      Report::Position(fix) => {
        self.tally.positioned += 1;
        self.upsert(name, fix);
      }
      Report::Compressed => self.tally.compressed += 1,
      Report::MicE => self.tally.mic_e += 1,
      Report::Malformed => self.tally.malformed += 1,
      Report::Other => {}
    }
  }

  fn upsert(&mut self, name: &str, fix: Fix) {
    // 站名不区分大小写（呼号本来就是大写，防的是别的来源）。
    if let Some(i) = self
      .stations
      .iter()
      .position(|s| s.name.eq_ignore_ascii_case(name))
    {
      let mut s = self.stations.remove(i);
      s.fix = fix;
      s.reports += 1;
      self.stations.push(s);
      return;
    }
    if self.stations.len() >= MAX_STATIONS {
      self.stations.remove(0);
    }
    self.stations.push(Station {
      name: name.to_owned(),
      fix,
      reports: 1,
    });
  }

  /// 按「最近更新在后」排列的站点。
  #[must_use]
  pub fn stations(&self) -> &[Station] {
    &self.stations
  }

  /// 各类计数。
  #[must_use]
  pub const fn tally(&self) -> Tally {
    self.tally
  }

  /// 是否什么都没有记过（没有站、也没有任何未解 / 无效计数）。
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.stations.is_empty() && self.tally == Tally::default()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // 坐标取自 APRS 协议规范 1.0.1 的示例报文 `!4903.50N/07201.75W-Test 001234`：
  // 49°03.50′ N = 49 + 3.5/60，72°01.75′ W = −(72 + 1.75/60)。数值是手算的，不经过被测代码。
  const LAT: f64 = 49.0 + 3.5 / 60.0;
  const LON: f64 = -(72.0 + 1.75 / 60.0);

  fn fix_of(info: &[u8]) -> Fix {
    match classify(info) {
      Report::Position(f) => f,
      other => panic!("应解出位置，实际是 {other:?}"),
    }
  }

  fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
  }

  #[test]
  fn plain_position_lands_on_the_spec_example() {
    let f = fix_of(b"!4903.50N/07201.75W-Test 001234");
    assert!(near(f.lat, LAT) && near(f.lon, LON), "{f:?}");
    assert_eq!((f.table, f.symbol), ('/', '-'));
  }

  #[test]
  fn messaging_flag_and_timestamped_variants_agree() {
    for info in [
      &b"=4903.50N/07201.75W-"[..],
      b"/092345z4903.50N/07201.75W>",
      b"@092345z4903.50N/07201.75W>Test",
      // 时间戳的另一种写法 HHMMSSh，同样是 7 个字符。
      b"/234517h4903.50N/07201.75W>",
    ] {
      let f = fix_of(info);
      assert!(
        near(f.lat, LAT) && near(f.lon, LON),
        "{}",
        String::from_utf8_lossy(info)
      );
    }
  }

  #[test]
  fn a_timestamp_of_the_wrong_length_shifts_the_fields_and_is_rejected() {
    // 8 个字符的时间戳不用于位置报；按 7 个字符跳过会让位置错一位 ——
    // 必须判为无效，不能读出一个看似合理的坐标。
    assert_eq!(
      classify(b"@10092345 4903.50N/07201.75W>"),
      Report::Malformed
    );
  }

  #[test]
  fn alternate_table_and_overlay_are_not_dropped() {
    // 规范里的备用符号表是 `\`；`decode_position` 只认 `/`，这里必须能读。
    let f = fix_of(b"!4903.50N\\07201.75W#");
    assert!(near(f.lat, LAT) && near(f.lon, LON));
    assert_eq!((f.table, f.symbol), ('\\', '#'));
    // 叠加字符（数字或大写字母）占符号表位置。
    assert_eq!(fix_of(b"!4903.50N107201.75W#").table, '1');
    assert_eq!(fix_of(b"!4903.50NS07201.75W#").table, 'S');
  }

  #[test]
  fn southern_and_eastern_hemispheres_get_the_right_signs() {
    let f = fix_of(b"!3114.50S/12128.50E-");
    assert!(near(f.lat, -(31.0 + 14.5 / 60.0)));
    assert!(near(f.lon, 121.0 + 28.5 / 60.0));
  }

  #[test]
  fn label_uses_hemisphere_letters() {
    assert_eq!(
      fix_of(b"!4903.50N/07201.75W-").label(),
      "49.0583°N 72.0292°W"
    );
    assert_eq!(
      fix_of(b"!3114.50S/12128.50E-").label(),
      "31.2417°S 121.4750°E"
    );
  }

  #[test]
  fn compressed_and_mic_e_are_classified_not_guessed() {
    // 规范里的压缩位置示例（`/` 表、base-91 纬经度）。
    assert_eq!(classify(b"=/5L!!<*e7>7P["), Report::Compressed);
    assert_eq!(classify(b"!\\5L!!<*e7>7P[ extra"), Report::Compressed);
    assert_eq!(classify(b"@092345z/5L!!<*e7>7P["), Report::Compressed);
    // 规范里的 Mic-E 示例报文 `` `(_fn"Oj/ ``；旧版数据类型 0x1C / 0x1D 同样归 Mic-E。
    assert_eq!(classify(b"`(_fn\"Oj/"), Report::MicE);
    assert_eq!(classify(b"'(_fn\"Oj/"), Report::MicE);
    assert_eq!(classify(&[0x1c, b'a']), Report::MicE);
    assert_eq!(classify(&[0x1d, b'a']), Report::MicE);
  }

  #[test]
  fn types_without_a_position_are_other_not_errors() {
    for info in [
      &b":BLN3     :Snow expected"[..],
      b">status text",
      b"T#005,199,000,255,073,123,01101001",
      b";LEADER   *092345z4903.50N/07201.75W>",
      b")!4903.50N/07201.75W-",
      b"$GPRMC,,A",
      b"",
    ] {
      assert_eq!(
        classify(info),
        Report::Other,
        "{}",
        String::from_utf8_lossy(info)
      );
    }
  }

  #[test]
  fn damaged_position_fields_are_malformed_never_a_wrong_point() {
    for info in [
      &b"!"[..],                // 只有类型
      b"!4903.50N/0720",        // 被截断
      b"!4903.50N/07201.75",    // 缺符号码
      b"!49  .  N/072  .  W-",  // 位置模糊（空格）：不猜中点
      b"!9959.99N/07201.75W-",  // 纬度越界
      b"!4903.50N/19959.99W-",  // 经度越界
      b"!4960.00N/07201.75W-",  // 分钟 ≥ 60
      b"!4903.50X/07201.75W-",  // 半球字符非法
      b"/092345z",              // 时间戳后没有位置
      b"/0923",                 // 时间戳都不全
      b"=/5L!!<*e7",            // 压缩位置被截断
      b"!~4903.50N/07201.75W-", // 位置体开头不是数字也不是符号表
    ] {
      assert_eq!(
        classify(info),
        Report::Malformed,
        "{}",
        String::from_utf8_lossy(info)
      );
    }
    // 位置本身必须是 ASCII；多字节字符不能在切片处 panic。
    assert_eq!(
      classify("!4903.5中N/07201.75W-".as_bytes()),
      Report::Malformed
    );
    assert_eq!(classify(&[b'!', 0xff, 0xfe, b'0']), Report::Malformed);
  }

  #[test]
  fn comment_after_the_position_may_be_any_bytes() {
    let mut info = b"!4903.50N/07201.75W-".to_vec();
    info.extend_from_slice(&[0xff, 0x00, 0x80]);
    let f = fix_of(&info);
    assert!(near(f.lat, LAT));
  }

  #[test]
  fn book_keeps_the_latest_fix_per_station_and_counts_the_rest() {
    let mut book = StationBook::new();
    assert!(book.is_empty());
    book.record("BG1AAA-7", b"!4903.50N/07201.75W-");
    book.record("BG1BBB", b"!3114.50N/12128.50E>");
    // 同呼号不同 SSID 是两个站。
    book.record("BG1AAA-9", b"!3000.00N/12000.00E>");
    // 同一个站再报一次：只留最新、报数 +1、排到最后。
    book.record("BG1AAA-7", b"!3114.50N/12128.50E-");
    book.record("BG1CCC", b"=/5L!!<*e7>7P[");
    book.record("BG1CCC", b"`(_fn\"Oj/");
    book.record("BG1DDD", b"!99");
    book.record("BG1EEE", b":BLN3     :hello");

    let names: Vec<_> = book.stations().iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["BG1BBB", "BG1AAA-9", "BG1AAA-7"]);
    let a7 = book.stations().last().unwrap();
    assert_eq!(a7.reports, 2);
    assert!(near(a7.fix.lat, 31.0 + 14.5 / 60.0), "应是第二次的位置");
    assert_eq!(
      book.tally(),
      Tally {
        positioned: 4,
        compressed: 1,
        mic_e: 1,
        malformed: 1,
      }
    );
    assert!(!book.is_empty());
  }

  #[test]
  fn only_undecodable_frames_still_make_the_book_non_empty() {
    let mut book = StationBook::new();
    book.record("BG1CCC", b"=/5L!!<*e7>7P[");
    assert!(book.stations().is_empty());
    assert!(
      !book.is_empty(),
      "有计数就该让页面显示汇总，而不是「还没收到」"
    );
  }

  #[test]
  fn book_evicts_the_stalest_station_when_full() {
    let mut book = StationBook::new();
    for i in 0..=MAX_STATIONS {
      book.record(&format!("N{i}"), b"!4903.50N/07201.75W-");
    }
    assert_eq!(book.stations().len(), MAX_STATIONS);
    assert_eq!(book.stations()[0].name, "N1", "最早的 N0 被挤掉");
    // 刷新最老的一个，下一次满员时被挤掉的是下一个。
    book.record("N1", b"!4903.50N/07201.75W-");
    book.record("NEW", b"!4903.50N/07201.75W-");
    assert!(book.stations().iter().any(|s| s.name == "N1"));
    assert!(!book.stations().iter().any(|s| s.name == "N2"));
  }
}
