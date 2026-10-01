//! 电台 CAT 协议：生成查询命令并从串口字节流中解析频率与模式。
//!
//! 支持两类常见协议：
//! - ASCII 分号协议（Kenwood / Elecraft / 新款 Yaesu）：`FA;` 查频率、`MD;` 查模式；
//! - Icom CI-V 二进制帧：`FE FE <rig> E0 <cmd> … FD`，频率为小端 BCD。

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
}

/// 缓冲区上限，防止持续收到垃圾数据时无限增长。
const MAX_BUF: usize = 256;

impl Parser {
  #[must_use]
  pub const fn new(protocol: Protocol) -> Self {
    Self {
      protocol,
      buf: Vec::new(),
    }
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
      if let Some(u) = parse_ascii(&frame[..frame.len() - 1]) {
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
    // 新款 Yaesu：`MD0` + 十六进制码
    2 => match &code[1..] {
      "1" | "2" => "SSB",
      "3" | "7" => "CW",
      "4" | "B" => "FM",
      "5" | "D" => "AM",
      "6" | "9" => "RTTY",
      "8" | "A" | "C" => "DATA",
      "E" => "C4FM",
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
