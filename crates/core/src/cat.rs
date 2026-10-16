//! 电台 CAT 协议：生成查询命令并从串口字节流中解析频率与模式。
//!
//! 支持两类常见协议：
//! - ASCII 分号协议（Kenwood / Elecraft / 新款 Yaesu）：`FA;` 查频率、`MD;` 查模式；
//! - Icom CI-V 二进制帧：`FE FE <rig> E0 <cmd> … FD`，频率为小端 BCD。
//!
//! 除读取外还能生成**设频**命令（[`set_frequency_command`]）。ASCII 协议里 `FA` 后面的位数因机型而异
//! （Kenwood / Elecraft 11 位，FT-991 / FTDX10 / FT-891 等 9 位，FT-450 等 8 位），写错位数电台会回
//! `?;` 并忽略，所以**不靠机型表猜**，而是照电台自己读频率时回的 `FA` 位数写（[`Parser::freq_digits`]），
//! 没读到过就拒绝写。命令格式对照 Hamlib 的实现核对过（Kenwood `FA%011d;`、Yaesu `FA%09d;`、
//! Icom 命令 `05` + 5 字节小端 BCD）。

use serde::{Deserialize, Serialize};

/// CAT 协议类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
  /// Kenwood、Elecraft 与 FT-991/FTDX10 等新款 Yaesu 的 ASCII 命令。
  #[default]
  Kenwood,
  /// Icom CI-V。
  Icom,
}

impl Protocol {
  pub const ALL: [Self; 2] = [Self::Kenwood, Self::Icom];

  #[must_use]
  pub const fn label(self) -> &'static str {
    match self {
      Self::Kenwood => "Kenwood / Elecraft / 新款 Yaesu",
      Self::Icom => "Icom CI-V",
    }
  }
}

/// 常见串口波特率。
pub const BAUD_RATES: [u32; 5] = [4800, 9600, 19200, 38400, 115_200];

/// 默认 CI-V 地址（IC-7300）。
pub const DEFAULT_CIV_ADDR: u8 = 0x94;

/// CI-V 控制器（电脑端）地址。
const CIV_CONTROLLER: u8 = 0xE0;

/// 从电台读到的一次状态更新。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Update {
  /// 频率（Hz）。
  Freq(u64),
  /// 日志模式名，取值属于 [`crate::logbook::MODES`]。
  Mode(&'static str),
}

/// 一轮轮询需要发送的命令。
#[must_use]
pub fn poll_commands(protocol: Protocol, civ_addr: u8) -> Vec<Vec<u8>> {
  match protocol {
    Protocol::Kenwood => vec![b"FA;".to_vec(), b"MD;".to_vec()],
    Protocol::Icom => [0x03, 0x04]
      .into_iter()
      .map(|cmd| vec![0xFE, 0xFE, civ_addr, CIV_CONTROLLER, cmd, 0xFD])
      .collect(),
  }
}

/// 流式解析器：串口数据可能被任意切分，未完整的帧留在缓冲区等待后续字节。
#[derive(Debug, Clone, Default)]
pub struct Parser {
  protocol: Protocol,
  buf: Vec<u8>,
  /// 电台最近一次 `FA` 应答里频率的位数（仅 ASCII 协议；见 [`Parser::freq_digits`]）。
  freq_digits: Option<usize>,
}

/// 缓冲区上限，防止持续收到垃圾数据时无限增长。
const MAX_BUF: usize = 256;

impl Parser {
  #[must_use]
  pub const fn new(protocol: Protocol) -> Self {
    Self {
      protocol,
      buf: Vec::new(),
      freq_digits: None,
    }
  }

  /// 电台自己回的 `FA` 频率有几位数字（Kenwood 系 11，新款 Yaesu 9 或 8）；还没收到过则为 `None`。
  ///
  /// Icom 的 CI-V 频率恒为 5 字节 BCD，没有这个问题，始终为 `None`。
  #[must_use]
  pub const fn freq_digits(&self) -> Option<usize> {
    self.freq_digits
  }

  /// 喂入新收到的字节，返回其中解析出的全部更新。
  pub fn feed(&mut self, bytes: &[u8]) -> Vec<Update> {
    self.buf.extend_from_slice(bytes);
    let out = match self.protocol {
      Protocol::Kenwood => self.drain_ascii(),
      Protocol::Icom => self.drain_civ(),
    };
    if self.buf.len() > MAX_BUF {
      let cut = self.buf.len() - MAX_BUF;
      self.buf.drain(..cut);
    }
    out
  }

  fn drain_ascii(&mut self) -> Vec<Update> {
    let mut out = Vec::new();
    while let Some(end) = self.buf.iter().position(|&b| b == b';') {
      let frame: Vec<u8> = self.buf.drain(..=end).collect();
      let body = &frame[..frame.len() - 1];
      if let Some(u) = parse_ascii(body) {
        // 只在确实解析出了频率时才记位数：垃圾帧不能把「电台用几位」改掉。
        if matches!(u, Update::Freq(_)) {
          self.freq_digits = fa_digits(body).or(self.freq_digits);
        }
        out.push(u);
      }
    }
    out
  }

  fn drain_civ(&mut self) -> Vec<Update> {
    let mut out = Vec::new();
    while let Some(end) = self.buf.iter().position(|&b| b == 0xFD) {
      let frame: Vec<u8> = self.buf.drain(..=end).collect();
      if let Some(u) = parse_civ(&frame) {
        out.push(u);
      }
    }
    out
  }
}

/// `FA` 应答里频率的位数；格式不对返回 `None`。
fn fa_digits(frame: &[u8]) -> Option<usize> {
  let s = std::str::from_utf8(frame).ok()?.trim();
  let s = s.get(s.find(['F', 'M'])?..)?;
  let digits = s.strip_prefix("FA")?;
  (digits.len() >= 8 && digits.bytes().all(|b| b.is_ascii_digit())).then_some(digits.len())
}

/// ASCII 协议允许的频率位数范围：8（FT-450 等）到 11（Kenwood / Elecraft）。
const ASCII_DIGITS: std::ops::RangeInclusive<usize> = 8..=11;

/// CI-V 频率字段的字节数：5 字节 BCD 最大 9 999 999 999 Hz，覆盖到微波 10 GHz 以下。
const CIV_FREQ_BYTES: usize = 5;

/// 生成「把当前 VFO 设到 `hz`」的命令；输入或参数不合法时返回 `None`（宁可不写，也不写一个错的频率）。
///
/// - ASCII 协议（Kenwood / Elecraft / 新款 Yaesu）：`FA` + 补零到 `ascii_digits` 位 + `;`。
///   `ascii_digits` 必须是电台自己的位数（[`Parser::freq_digits`]），没读到过传 `None` 会拒绝；
///   频率放不进这么多位（如 8 位机型写 437 MHz）也拒绝，**不截断**；
/// - Icom：`FE FE <rig> E0 05 <5 字节小端 BCD> FD`，`ascii_digits` 被忽略，频率须小于 10 GHz。
///
/// 只设频率：不碰模式、不切 VFO、不开分集，也没有任何发射相关命令。
#[must_use]
pub fn set_frequency_command(
  protocol: Protocol,
  civ_addr: u8,
  hz: u64,
  ascii_digits: Option<usize>,
) -> Option<Vec<u8>> {
  if hz == 0 {
    return None;
  }
  match protocol {
    Protocol::Kenwood => {
      let n = ascii_digits.filter(|n| ASCII_DIGITS.contains(n))?;
      // 放得下才写：`10^n` 对 n ≤ 11 不会溢出 u64。
      if hz >= 10u64.pow(u32::try_from(n).ok()?) {
        return None;
      }
      Some(format!("FA{hz:0n$};").into_bytes())
    }
    Protocol::Icom => {
      let bcd = to_bcd_le(hz, CIV_FREQ_BYTES)?;
      let mut cmd = vec![0xFE, 0xFE, civ_addr, CIV_CONTROLLER, 0x05];
      cmd.extend_from_slice(&bcd);
      cmd.push(0xFD);
      Some(cmd)
    }
  }
}

/// 小端 BCD 编码（[`bcd_le`] 的逆运算）：放不进 `len` 字节返回 `None`。
fn to_bcd_le(mut value: u64, len: usize) -> Option<Vec<u8>> {
  let mut out = Vec::with_capacity(len);
  for _ in 0..len {
    let pair = u8::try_from(value % 100).ok()?;
    value /= 100;
    out.push(((pair / 10) << 4) | (pair % 10));
  }
  (value == 0).then_some(out)
}

/// 解析一条去掉结尾分号的 ASCII 应答，如 `FA00014074000`、`MD2`、`MD02`。
fn parse_ascii(frame: &[u8]) -> Option<Update> {
  let s = std::str::from_utf8(frame).ok()?.trim();
  let s = s.get(s.find(['F', 'M'])?..)?;
  if let Some(digits) = s.strip_prefix("FA") {
    if digits.len() < 8 || !digits.bytes().all(|b| b.is_ascii_digit()) {
      return None;
    }
    return digits.parse().ok().filter(|&hz| hz > 0).map(Update::Freq);
  }
  let code = s.strip_prefix("MD")?;
  let mode = match code.len() {
    // Kenwood / Elecraft：单个数字
    1 => match code {
      "1" | "2" => "SSB",
      "3" | "7" => "CW",
      "4" => "FM",
      "5" => "AM",
      "6" | "9" => "RTTY",
      _ => return None,
    },
    // 新款 Yaesu：`MD0` + 十六进制码。
    // 按**字节**取，而不是 `&code[1..]`：串口噪声里 `0xC3 0xA9`（`é`）是合法的
    // UTF-8 两字节序列，能过 `from_utf8`，但 `len() == 2` 时按字符切会 panic 在边界上。
    2 => match code.as_bytes()[1] {
      b'1' | b'2' => "SSB",
      b'3' | b'7' => "CW",
      b'4' | b'B' => "FM",
      b'5' | b'D' => "AM",
      b'6' | b'9' => "RTTY",
      b'8' | b'A' | b'C' => "DATA",
      b'E' => "C4FM",
      _ => return None,
    },
    _ => return None,
  };
  Some(Update::Mode(mode))
}

/// 解析一条完整的 CI-V 帧（含 `FE FE` 前导与 `FD` 结尾）。
fn parse_civ(frame: &[u8]) -> Option<Update> {
  let start = frame.windows(2).position(|w| w == [0xFE, 0xFE])?;
  let body = &frame[start + 2..frame.len() - 1];
  // 跳过多余的 FE 前导
  let body = &body[body.iter().position(|&b| b != 0xFE)?..];
  let [to, _from, cmd, data @ ..] = body else {
    return None;
  };
  // 只关心发给电脑（E0）或广播（00）的帧，忽略回显的查询命令
  if *to != CIV_CONTROLLER && *to != 0x00 {
    return None;
  }
  match cmd {
    0x00 | 0x03 if data.len() >= 4 => bcd_le(&data[..data.len().min(5)])
      .filter(|&hz| hz > 0)
      .map(Update::Freq),
    0x01 | 0x04 if !data.is_empty() => {
      let mode = match data[0] {
        0x00 | 0x01 => "SSB",
        0x02 => "AM",
        0x03 | 0x07 => "CW",
        0x04 | 0x08 => "RTTY",
        0x05 | 0x06 => "FM",
        0x17 => "DSTAR",
        _ => return None,
      };
      Some(Update::Mode(mode))
    }
    _ => None,
  }
}

/// 小端 BCD：每字节两位十进制，低位在前。
fn bcd_le(bytes: &[u8]) -> Option<u64> {
  let mut value = 0u64;
  for &b in bytes.iter().rev() {
    let (hi, lo) = (u64::from(b >> 4), u64::from(b & 0x0F));
    if hi > 9 || lo > 9 {
      return None;
    }
    value = value * 100 + hi * 10 + lo;
  }
  Some(value)
}

/// 把 Hz 格式化为日志用的 MHz 字符串，去掉多余的尾零但至少保留三位小数，如 `14.074`、`7.0255`。
#[must_use]
pub fn format_mhz(hz: u64) -> String {
  let s = format!("{}.{:06}", hz / 1_000_000, hz % 1_000_000);
  let keep = s.len() - s.bytes().rev().take(3).take_while(|&b| b == b'0').count();
  s[..keep].to_owned()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn kenwood_split_frames() {
    let mut p = Parser::new(Protocol::Kenwood);
    assert!(p.feed(b"FA000140").is_empty());
    assert_eq!(
      p.feed(b"74000;MD3;"),
      vec![Update::Freq(14_074_000), Update::Mode("CW")]
    );
    // 新款 Yaesu 9 位频率与十六进制模式码
    assert_eq!(
      p.feed(b"FA007025500;MD0C;"),
      vec![Update::Freq(7_025_500), Update::Mode("DATA")]
    );
    // 回显的查询命令、错误应答都被忽略
    assert!(p.feed(b"FA;MD;?;").is_empty());
  }

  #[test]
  fn non_ascii_frames_are_ignored_instead_of_panicking() {
    // 串口噪声里 `0xC3 0xA9`（`é`）是合法的 UTF-8 两字节序列，`MDé` 能过 `from_utf8`
    // 且 `code.len() == 2`；模式码按字节取，按字符切会 panic 在字符边界上。
    let mut p = Parser::new(Protocol::Kenwood);
    assert!(p.feed("MDé;".as_bytes()).is_empty());
    // 模式码里混进非 ASCII：整帧丢弃，后面的正常帧照常解析。
    assert_eq!(
      p.feed("MDé;FA00014074000;".as_bytes()),
      vec![Update::Freq(14_074_000)]
    );
  }

  #[test]
  fn civ_frames() {
    let mut p = Parser::new(Protocol::Icom);
    // 回显：发往电台的查询帧
    let echo = [0xFE, 0xFE, 0x94, 0xE0, 0x03, 0xFD];
    // 14.074.000 Hz → 00 40 07 14 00
    let freq = [
      0xFE, 0xFE, 0xE0, 0x94, 0x03, 0x00, 0x40, 0x07, 0x14, 0x00, 0xFD,
    ];
    let mode = [0xFE, 0xFE, 0xE0, 0x94, 0x04, 0x03, 0x01, 0xFD];
    let mut bytes = echo.to_vec();
    bytes.extend_from_slice(&freq);
    bytes.extend_from_slice(&mode[..4]);
    assert_eq!(p.feed(&bytes), vec![Update::Freq(14_074_000)]);
    assert_eq!(p.feed(&mode[4..]), vec![Update::Mode("CW")]);
    // 转发模式下电台主动广播（目标地址 00）
    let broadcast = [
      0xFE, 0xFE, 0x00, 0x94, 0x00, 0x00, 0x25, 0x02, 0x07, 0x00, 0xFD,
    ];
    assert_eq!(p.feed(&broadcast), vec![Update::Freq(7_022_500)]);
  }

  #[test]
  fn kenwood_set_frequency_uses_eleven_digits() {
    // Hamlib kenwood.c：`F%c%011` —— 11 位补零。
    let set = |hz| set_frequency_command(Protocol::Kenwood, 0, hz, Some(11));
    assert_eq!(set(437_810_220), Some(b"FA00437810220;".to_vec()));
    assert_eq!(set(14_074_000), Some(b"FA00014074000;".to_vec()));
    assert_eq!(set(99_999_999_999), Some(b"FA99999999999;".to_vec()));
    assert_eq!(set(100_000_000_000), None); // 11 位放不下
  }

  #[test]
  fn yaesu_set_frequency_follows_the_digits_the_rig_reports() {
    // 新款 Yaesu（Hamlib newcat.c：`FA%09.0f`）9 位；FT-450 一类 8 位。
    let set = |hz, n| set_frequency_command(Protocol::Kenwood, 0, hz, Some(n));
    assert_eq!(set(437_810_220, 9), Some(b"FA437810220;".to_vec()));
    assert_eq!(set(14_074_000, 9), Some(b"FA014074000;".to_vec()));
    assert_eq!(set(14_074_000, 8), Some(b"FA14074000;".to_vec()));
    // 8 位机型写不下 437 MHz：拒绝，而不是截成 37 MHz 写进去。
    assert_eq!(set(437_810_220, 8), None);
    assert_eq!(set(1_000_000_000, 9), None);
  }

  #[test]
  fn ascii_set_frequency_refuses_to_guess() {
    // 没读到过电台的位数：不写。
    assert_eq!(
      set_frequency_command(Protocol::Kenwood, 0, 14_074_000, None),
      None
    );
    // 位数不在已知范围：不写。
    for n in [0, 7, 12, 20] {
      assert_eq!(
        set_frequency_command(Protocol::Kenwood, 0, 14_074_000, Some(n)),
        None,
        "{n}"
      );
    }
    assert_eq!(
      set_frequency_command(Protocol::Kenwood, 0, 0, Some(11)),
      None
    );
    assert_eq!(set_frequency_command(Protocol::Icom, 0x94, 0, None), None);
  }

  #[test]
  fn icom_set_frequency_is_command_05_with_five_bcd_bytes() {
    let set = |addr, hz| set_frequency_command(Protocol::Icom, addr, hz, None);
    // 与既有解析测试同一组数：14.074 MHz → 00 40 07 14 00。
    assert_eq!(
      set(0x94, 14_074_000),
      Some(vec![
        0xFE, 0xFE, 0x94, 0xE0, 0x05, 0x00, 0x40, 0x07, 0x14, 0x00, 0xFD
      ])
    );
    // 437 810 220 Hz → 20 02 81 37 04（独立脚本算出）。
    assert_eq!(
      set(0xA2, 437_810_220),
      Some(vec![
        0xFE, 0xFE, 0xA2, 0xE0, 0x05, 0x20, 0x02, 0x81, 0x37, 0x04, 0xFD
      ])
    );
    assert_eq!(
      set(0x94, 1_296_100_000),
      Some(vec![
        0xFE, 0xFE, 0x94, 0xE0, 0x05, 0x00, 0x00, 0x10, 0x96, 0x12, 0xFD
      ])
    );
    // 5 字节 BCD 的上限：9 999 999 999 Hz 能写，10 GHz 写不下。
    assert!(set(0x94, 9_999_999_999).is_some());
    assert_eq!(set(0x94, 10_000_000_000), None);
    // ASCII 位数对 Icom 无意义，传不传都一样。
    assert_eq!(
      set_frequency_command(Protocol::Icom, 0x94, 14_074_000, Some(11)),
      set(0x94, 14_074_000)
    );
  }

  #[test]
  fn bcd_encoding_is_the_inverse_of_decoding() {
    for hz in [
      1,
      99,
      100,
      14_074_000,
      437_810_220,
      1_296_100_000,
      9_999_999_999,
    ] {
      let b = to_bcd_le(hz, 5).unwrap();
      assert_eq!(b.len(), 5);
      assert_eq!(bcd_le(&b), Some(hz), "{hz}");
    }
    assert_eq!(to_bcd_le(100, 1), None);
    assert_eq!(to_bcd_le(99, 1), Some(vec![0x99]));
  }

  #[test]
  fn what_we_write_is_what_the_parser_reads_back() {
    // ASCII 的设频命令与电台的 FA 应答语法一致：生成的命令喂回解析器必须得到同一个频率。
    for (digits, hz) in [
      (11, 437_810_220),
      (11, 14_074_000),
      (9, 437_810_220),
      (9, 7_025_500),
    ] {
      let cmd = set_frequency_command(Protocol::Kenwood, 0, hz, Some(digits)).unwrap();
      let mut p = Parser::new(Protocol::Kenwood);
      assert_eq!(p.feed(&cmd), vec![Update::Freq(hz)], "{digits} 位 {hz}");
      assert_eq!(p.freq_digits(), Some(digits));
    }
    // Icom：帧里的 BCD 字节解出来就是写入的频率。
    let cmd = set_frequency_command(Protocol::Icom, 0x94, 437_810_220, None).unwrap();
    assert_eq!(bcd_le(&cmd[5..10]), Some(437_810_220));
  }

  #[test]
  fn the_parser_learns_the_rigs_digit_width_only_from_real_frequency_replies() {
    let mut p = Parser::new(Protocol::Kenwood);
    assert_eq!(p.freq_digits(), None);
    // 没收完整的帧不算。
    p.feed(b"FA000140");
    assert_eq!(p.freq_digits(), None);
    p.feed(b"74000;");
    assert_eq!(p.freq_digits(), Some(11));
    // 模式、查询回显、错误应答、垃圾都不改它。
    p.feed(b"MD3;FA;?;garbage;FA12;");
    assert_eq!(p.freq_digits(), Some(11));
    // 换了台电台（9 位）就跟着变。
    p.feed(b"FA007025500;");
    assert_eq!(p.freq_digits(), Some(9));
    // Icom 恒为 None。
    let mut i = Parser::new(Protocol::Icom);
    i.feed(&[
      0xFE, 0xFE, 0xE0, 0x94, 0x03, 0x00, 0x40, 0x07, 0x14, 0x00, 0xFD,
    ]);
    assert_eq!(i.freq_digits(), None);
  }

  #[test]
  fn poll_and_format() {
    assert_eq!(
      poll_commands(Protocol::Icom, 0x94)[0],
      vec![0xFE, 0xFE, 0x94, 0xE0, 0x03, 0xFD]
    );
    assert_eq!(poll_commands(Protocol::Kenwood, 0)[1], b"MD;".to_vec());
    assert_eq!(format_mhz(14_074_000), "14.074");
    assert_eq!(format_mhz(7_025_500), "7.0255");
    assert_eq!(format_mhz(144_390_125), "144.390125");
    for m in ["SSB", "CW", "AM", "FM", "RTTY", "DATA", "C4FM", "DSTAR"] {
      assert!(crate::logbook::MODES.contains(&m));
    }
  }
}
