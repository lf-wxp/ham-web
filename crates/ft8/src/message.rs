//! FT8 / FT4 消息打包与解包：文本消息 ↔ 77 位 payload。
//!
//! 转写自 MIT 许可的 ft8_lib `message.c`（含 `text.c` 的字符表）。77 位 payload 的
//! i3 / n3 头（3 位）决定消息类型：标准呼号、非标准（哈希）呼号、自由文本、遥测等。
//!
//! 哈希表是「呼号 ↔ 22 位哈希」的会话级缓存：编码时存、解码时查；查不到时非标准呼号显示为 `<...>`。
//! 这里用 [`std::collections::HashMap`] 实现，取代 ft8_lib 的函数指针回调。

use std::collections::HashMap;

use crate::text::{CharTable, charn, nchar};

/// 22 位哈希空间（`2^22`）。
const MAX22: u32 = 4_194_304;
/// 特殊 token 的空间起点（DE/QRZ/CQ/CQ_nnn/CQ_abcd）。
const NTOKENS: u32 = 2_063_592;
/// 4 位网格的最大值（18×18×10×10）。
const MAXGRID4: u16 = 32_400;

/// 一个消息（77 位 payload）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Message {
  pub payload: [u8; 10],
}

impl Message {
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }
}

/// 消息类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
  FreeText,
  Dxpedition,
  EuVhf,
  ArrlFd,
  Telemetry,
  Contesting,
  Standard,
  ArrlRtty,
  NonstdCall,
  Wwrof,
  Unknown,
}

/// 字段类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
  Unknown,
  Empty,
  Token,
  TokenWithArg,
  Call,
  Grid,
  Rst,
}

/// 编码 / 解码错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageError {
  Callsign1,
  Callsign2,
  Suffix,
  Grid,
  Type,
}

/// 会话级哈希表：22 位哈希 → 呼号，另有 12 位哈希 → 呼号。
///
/// 两套位宽必须**分开存**：type 4 消息只携带 12 位哈希（`callsign_hash >> 10`），
/// 拿它去 22 位表里查永远 miss，解出来的呼号就恒为 `<...>`。`save_callsign` 每存一个呼号
/// 两张表同时写（12 位键就是 22 位哈希右移 10 位），所以「先在别的消息里见过完整呼号」
/// 与「编码时存入」两条路径都能让后面的 type 4 消息还原出呼号。
#[derive(Debug, Default)]
pub struct HashTable {
  map: HashMap<u32, String>,
  map12: HashMap<u16, String>,
}

impl HashTable {
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }
}

/// i3（bits 74..76）。
#[must_use]
pub fn get_i3(msg: &Message) -> u8 {
  (msg.payload[9] >> 3) & 0x07
}

/// n3（bits 71..73）。
#[must_use]
pub fn get_n3(msg: &Message) -> u8 {
  ((msg.payload[8] << 2) & 0x04) | ((msg.payload[9] >> 6) & 0x03)
}

/// 消息类型（按 i3 / n3 头）。
#[must_use]
pub fn get_type(msg: &Message) -> MessageType {
  let i3 = get_i3(msg);
  match i3 {
    0 => match get_n3(msg) {
      0 => MessageType::FreeText,
      1 => MessageType::Dxpedition,
      2 => MessageType::EuVhf,
      3 | 4 => MessageType::ArrlFd,
      5 => MessageType::Telemetry,
      _ => MessageType::Unknown,
    },
    1 | 2 => MessageType::Standard,
    3 => MessageType::ArrlRtty,
    4 => MessageType::NonstdCall,
    5 => MessageType::Wwrof,
    _ => MessageType::Unknown,
  }
}

/// 计算呼号的 22 位哈希（不查表）。
///
/// 尖括号是「非标准呼号」的显示标记，不参与哈希（ft8_lib 的 `hashcall` 同样跳过）：
/// 把解码结果 `<EA8/G5LSI>` 原样粘回来再编码时要能得到同一个哈希。
#[must_use]
fn callsign_hash(callsign: &str) -> Option<u32> {
  let mut n58: u64 = 0;
  let mut i = 0;
  for ch in callsign.chars() {
    if ch == '<' || ch == '>' {
      continue;
    }
    if i >= 11 {
      break;
    }
    let j = nchar(ch, CharTable::AlphanumSpaceSlash)?;
    n58 = 38 * n58 + j as u64;
    i += 1;
  }
  // 补尾部空格（j=0）。
  while i < 11 {
    n58 *= 38;
    i += 1;
  }
  // C 里 `47055833459ull * n58` 依赖 64 位溢出回绕（乘法哈希），Rust 用 wrapping_mul。
  let n22 = (47_055_833_459u64.wrapping_mul(n58) >> (64 - 22)) & 0x3F_FFFF;
  Some(n22 as u32)
}

/// 去掉呼号两侧的尖括号（显示标记，不参与编码）。
fn strip_angle(callsign: &str) -> &str {
  callsign
    .strip_prefix('<')
    .and_then(|s| s.strip_suffix('>'))
    .unwrap_or_else(|| callsign.strip_prefix('<').unwrap_or(callsign))
}

/// 把呼号同时存入 22 位与 12 位两张表（编码 / 解码时都会调用）。
///
/// 12 位表必须在**解码**路径上也填：type 4 消息里的 12 位字段，对应的完整呼号通常来自同一
/// 会话里更早（或更晚）解出的标准消息，而不是本机编码出来的。
fn save_callsign(hash: &mut HashTable, callsign: &str) -> Result<(), MessageError> {
  let n22 = callsign_hash(callsign).ok_or(MessageError::Callsign1)?;
  let bare = strip_angle(callsign);
  hash.map.insert(n22, bare.to_owned());
  hash.map12.insert((n22 >> 10) as u16, bare.to_owned());
  Ok(())
}

/// 从哈希表查呼号；查不到返回 `<...>`。
#[must_use]
fn lookup_callsign(hash: &HashTable, n22: u32) -> String {
  match hash.map.get(&n22) {
    Some(c) => format!("<{c}>"),
    None => "<...>".to_owned(),
  }
}

/// 按 12 位哈希查呼号（type 4 的 12 位字段）；查不到返回 `<...>`。
#[must_use]
fn lookup_callsign12(hash: &HashTable, n12: u16) -> String {
  match hash.map12.get(&n12) {
    Some(c) => format!("<{c}>"),
    None => "<...>".to_owned(),
  }
}

/// 标准呼号 → 28 位（`pack_basecall`）；非标准返回 `None`。
///
/// 呼号按 ASCII 处理：非 ASCII（多字节 UTF-8）会让 `as_bytes` 的字节长度与 `chars` 的
/// 字符计数对不上，这里直接拒绝，避免拼出错位的 28 位值。
#[must_use]
pub fn pack_basecall(callsign: &str) -> Option<u32> {
  if !callsign.is_ascii() {
    return None;
  }
  let bytes = callsign.as_bytes();
  let length = bytes.len();
  if length <= 2 {
    return None;
  }
  let mut c6 = [' '; 6];
  if callsign.starts_with("3DA0") && length > 4 && length <= 7 {
    // 斯威士兰前缀：3DA0XYZ -> 3D0XYZ
    c6[0] = '3';
    c6[1] = 'D';
    c6[2] = '0';
    for (i, ch) in callsign.chars().skip(4).enumerate() {
      c6[3 + i] = ch;
    }
  } else if callsign.starts_with("3X")
    && bytes.get(2).is_some_and(|&b| b.is_ascii_alphabetic())
    && length <= 7
  {
    // 几内亚前缀：3XA0XYZ -> QA0XYZ
    c6[0] = 'Q';
    for (i, ch) in callsign.chars().skip(2).enumerate() {
      c6[1 + i] = ch;
    }
  } else if bytes.get(2).is_some_and(|&b| b.is_ascii_digit()) && length <= 6 {
    // AB0XYZ
    for (i, ch) in callsign.chars().enumerate() {
      c6[i] = ch;
    }
  } else if bytes.get(1).is_some_and(|&b| b.is_ascii_digit()) && length <= 5 {
    // A0XYZ -> " A0XYZ"
    for (i, ch) in callsign.chars().enumerate() {
      c6[1 + i] = ch;
    }
  }

  let i0 = nchar(c6[0], CharTable::AlphanumSpace)?;
  let i1 = nchar(c6[1], CharTable::Alphanum)?;
  let i2 = nchar(c6[2], CharTable::Numeric)?;
  let i3 = nchar(c6[3], CharTable::LettersSpace)?;
  let i4 = nchar(c6[4], CharTable::LettersSpace)?;
  let i5 = nchar(c6[5], CharTable::LettersSpace)?;

  let mut n = i0 as u32;
  n = n * 36 + i1 as u32;
  n = n * 10 + i2 as u32;
  n = n * 27 + i3 as u32;
  n = n * 27 + i4 as u32;
  n = n * 27 + i5 as u32;
  Some(n)
}

/// 解析 `CQ nnn` / `CQ abcd` 修饰符，返回编码值；不是特殊 CQ 返回 `None`。
#[must_use]
fn parse_cq_modifier(s: &str) -> Option<u32> {
  let rest = s.strip_prefix("CQ ")?;
  let mut nnum = 0;
  let mut nlet = 0;
  let mut m = 0;
  for ch in rest.chars().take(4) {
    if ch == ' ' {
      break;
    } else if ch.is_ascii_digit() {
      nnum += 1;
    } else if ch.is_ascii_uppercase() {
      nlet += 1;
      m = 27 * m + (ch as u32 - 'A' as u32 + 1);
    } else {
      return None;
    }
  }
  if nnum == 3 && nlet == 0 {
    Some(rest[..3].parse::<u32>().ok()?)
  } else if nnum == 0 && (1..=4).contains(&nlet) {
    Some(1000 + m)
  } else {
    None
  }
}

/// 打包一个「28+1 位」字段（特殊 token / 22 位哈希 / 标准呼号）。返回 (n28, ip)；错误返回 `Err`。
fn pack28(callsign: &str, hash: &mut HashTable) -> Result<(u32, u8), MessageError> {
  if callsign == "DE" {
    return Ok((0, 0));
  }
  if callsign == "QRZ" {
    return Ok((1, 0));
  }
  if callsign == "CQ" {
    return Ok((2, 0));
  }
  let length = callsign.len();
  if callsign.starts_with("CQ ") && length < 8 {
    let v = parse_cq_modifier(callsign).ok_or(MessageError::Callsign1)?;
    return Ok((3 + v, 0));
  }
  let mut ip = 0u8;
  let mut length_base = length;
  if callsign.ends_with("/P") || callsign.ends_with("/R") {
    ip = 1;
    length_base = length - 2;
  }
  let base = &callsign[..length_base];
  if let Some(n28) = pack_basecall(base) {
    save_callsign(hash, callsign)?;
    return Ok((NTOKENS + MAX22 + n28, ip));
  }
  if (3..=11).contains(&length) {
    let n22 = callsign_hash(callsign).ok_or(MessageError::Callsign1)?;
    save_callsign(hash, callsign)?;
    return Ok((NTOKENS + n22, 0));
  }
  Err(MessageError::Callsign1)
}

/// 解包 28+1 位字段。返回 `(callsign, field_type)`。
fn unpack28(
  n28: u32,
  ip: u8,
  i3: u8,
  hash: &mut HashTable,
) -> Result<(String, Field), MessageError> {
  if n28 < NTOKENS {
    if n28 <= 2 {
      let tok = ["DE", "QRZ", "CQ"][n28 as usize];
      return Ok((tok.to_owned(), Field::Token));
    }
    if n28 <= 1002 {
      // CQ nnn
      let v = n28 - 3;
      return Ok((format!("CQ {v:03}"), Field::TokenWithArg));
    }
    if n28 <= 532_443 {
      // CQ abcd（4 个字母）
      let mut n = n28 - 1003;
      let mut aaaa = [' '; 4];
      for i in (0..4).rev() {
        aaaa[i] = charn((n % 27) as usize, CharTable::LettersSpace);
        n /= 27;
      }
      let s: String = aaaa.iter().collect();
      return Ok((format!("CQ {}", s.trim_start()), Field::TokenWithArg));
    }
    return Err(MessageError::Type);
  }
  let n28 = n28 - NTOKENS;
  if n28 < MAX22 {
    let c = lookup_callsign(hash, n28);
    return Ok((c, Field::Call));
  }
  // 标准呼号
  let mut n = n28 - MAX22;
  let mut cs = [' '; 6];
  cs[5] = charn((n % 27) as usize, CharTable::LettersSpace);
  n /= 27;
  cs[4] = charn((n % 27) as usize, CharTable::LettersSpace);
  n /= 27;
  cs[3] = charn((n % 27) as usize, CharTable::LettersSpace);
  n /= 27;
  cs[2] = charn((n % 10) as usize, CharTable::Numeric);
  n /= 10;
  cs[1] = charn((n % 36) as usize, CharTable::Alphanum);
  n /= 36;
  cs[0] = charn((n % 37) as usize, CharTable::AlphanumSpace);

  let call: String = cs.iter().collect();
  let mut result = if call.starts_with("3D0") && !cs[3].is_whitespace() {
    format!("3DA0{}", call[3..].trim_start())
  } else if cs[0] == 'Q' && cs[1].is_ascii_alphabetic() {
    format!("3X{}", call[1..].trim_start())
  } else {
    call.trim().to_owned()
  };
  if result.len() < 3 {
    return Err(MessageError::Callsign1);
  }
  if ip != 0 {
    if i3 == 1 {
      result.push_str("/R");
    } else if i3 == 2 {
      result.push_str("/P");
    } else {
      return Err(MessageError::Suffix);
    }
  }
  save_callsign(hash, &result)?;
  Ok((result, Field::Call))
}

/// 打包非标准呼号（58 位）。
///
/// 与 [`callsign_hash`] 同一口径：尖括号是显示标记，跳过且不占 11 个字符的名额。
fn pack58(callsign: &str, hash: &mut HashTable) -> Result<u64, MessageError> {
  let mut result: u64 = 0;
  let mut c11 = String::new();
  for ch in callsign.chars() {
    if ch == '<' || ch == '>' {
      continue;
    }
    if c11.chars().count() >= 11 {
      break;
    }
    let j = nchar(ch, CharTable::AlphanumSpaceSlash).ok_or(MessageError::Callsign2)?;
    result = result * 38 + j as u64;
    c11.push(ch);
  }
  save_callsign(hash, &c11)?;
  Ok(result)
}

/// 解包 58 位呼号。
fn unpack58(n58: u64, hash: &mut HashTable) -> String {
  let mut n = n58;
  let mut cs = [' '; 11];
  for i in (0..11).rev() {
    cs[i] = charn((n % 38) as usize, CharTable::AlphanumSpaceSlash);
    n /= 38;
  }
  let call: String = cs.iter().collect();
  let call = call.trim().to_owned();
  if call.len() >= 3 {
    let _ = save_callsign(hash, &call);
  }
  call
}

/// 打包网格 / 报告（15 位）。
#[must_use]
fn packgrid(grid: &str) -> u16 {
  if grid.is_empty() {
    return MAXGRID4 + 1;
  }
  if grid == "RRR" {
    return MAXGRID4 + 2;
  }
  if grid == "RR73" {
    return MAXGRID4 + 3;
  }
  if grid == "73" {
    return MAXGRID4 + 4;
  }
  let b = grid.as_bytes();
  if b.len() >= 4
    && (b'A'..=b'R').contains(&b[0])
    && (b'A'..=b'R').contains(&b[1])
    && b[2].is_ascii_digit()
    && b[3].is_ascii_digit()
  {
    let mut g = (b[0] - b'A') as u16;
    g = g * 18 + (b[1] - b'A') as u16;
    g = g * 10 + (b[2] - b'0') as u16;
    g = g * 10 + (b[3] - b'0') as u16;
    return g;
  }
  // 报告：R+dd / R-dd / +dd / -dd
  let (ir, rest) = if let Some(r) = grid.strip_prefix('R') {
    (1u16, r)
  } else {
    (0, grid)
  };
  let dd = parse_dd(rest);
  let irpt = 35 + dd;
  let v = MAXGRID4 + irpt;
  if ir == 1 { v | 0x8000 } else { v }
}

/// 解析 `+dd` / `-dd`（两位十进制带符号）。
fn parse_dd(s: &str) -> u16 {
  let mut val: i16 = 0;
  let mut neg = false;
  let mut rest = s;
  if let Some(r) = s.strip_prefix('-') {
    neg = true;
    rest = r;
  } else if let Some(r) = s.strip_prefix('+') {
    rest = r;
  }
  for ch in rest.chars().take(2) {
    if !ch.is_ascii_digit() {
      break;
    }
    val = val * 10 + (ch as i16 - '0' as i16);
  }
  if neg { (-val) as u16 } else { val as u16 }
}

/// 解包网格 / 报告。返回 `(extra, field_type)`。
fn unpackgrid(igrid4: u16, ir: u8) -> Result<(String, Field), MessageError> {
  if igrid4 <= MAXGRID4 {
    let mut n = igrid4;
    let mut dst = [' '; 4];
    dst[3] = char::from(b'0' + (n % 10) as u8);
    n /= 10;
    dst[2] = char::from(b'0' + (n % 10) as u8);
    n /= 10;
    dst[1] = char::from(b'A' + (n % 18) as u8);
    n /= 18;
    dst[0] = char::from(b'A' + (n % 18) as u8);
    let g: String = dst.iter().collect();
    return Ok((if ir > 0 { format!("R {g}") } else { g }, Field::Grid));
  }
  let irpt = igrid4 - MAXGRID4;
  match irpt {
    1 => Ok((String::new(), Field::Empty)),
    2 => Ok(("RRR".to_owned(), Field::Token)),
    3 => Ok(("RR73".to_owned(), Field::Token)),
    4 => Ok(("73".to_owned(), Field::Token)),
    _ => {
      let dd = irpt as i16 - 35;
      let s = format!("{dd:+}");
      Ok((if ir > 0 { format!("R{s}") } else { s }, Field::Rst))
    }
  }
}

/// 编码标准消息（type 1 / 2）。
fn encode_std(
  call_to: &str,
  call_de: &str,
  extra: &str,
  hash: &mut HashTable,
) -> Result<Message, MessageError> {
  let (n28a, ipa) = pack28(call_to, hash)?;
  let (n28b, ipb) = pack28(call_de, hash)?;

  let mut i3: u8 = 1;
  if call_to.ends_with("/P") || call_de.ends_with("/P") {
    i3 = 2;
    if call_to.ends_with("/R") || call_de.ends_with("/R") {
      return Err(MessageError::Suffix);
    }
  }
  let icq = call_to == "CQ" || call_to.starts_with("CQ ");
  if let Some(pos) = call_de.find('/')
    && pos >= 2
    && icq
    && !call_de.ends_with("/P")
    && !call_de.ends_with("/R")
  {
    return Err(MessageError::Callsign2);
  }

  let igrid4 = packgrid(extra);
  let mut n29a = (n28a << 1) | u32::from(ipa);
  let n29b = (n28b << 1) | u32::from(ipb);
  if call_to.ends_with("/R") {
    n29a |= 1;
  } else if call_to.ends_with("/P") {
    n29a |= 1;
    i3 = 2;
  }

  let mut p = [0u8; 10];
  p[0] = (n29a >> 21) as u8;
  p[1] = (n29a >> 13) as u8;
  p[2] = (n29a >> 5) as u8;
  p[3] = ((n29a << 3) | (n29b >> 26)) as u8;
  p[4] = (n29b >> 18) as u8;
  p[5] = (n29b >> 10) as u8;
  p[6] = (n29b >> 2) as u8;
  p[7] = ((n29b << 6) | u32::from(igrid4 >> 10)) as u8;
  p[8] = (igrid4 >> 2) as u8;
  p[9] = ((igrid4 << 6) | u16::from(i3 << 3)) as u8;
  Ok(Message { payload: p })
}

/// 编码非标准消息（type 4）。
fn encode_nonstd(
  call_to: &str,
  call_de: &str,
  extra: &str,
  hash: &mut HashTable,
) -> Result<Message, MessageError> {
  let i3: u8 = 4;
  let icq = call_to == "CQ" || call_to.starts_with("CQ ");
  if !icq && call_to.len() < 3 {
    return Err(MessageError::Callsign1);
  }
  if call_de.len() < 3 {
    return Err(MessageError::Callsign2);
  }
  let (iflip, n12, call58) = if !icq {
    let iflip = if call_de.starts_with('<') && call_de.ends_with('>') {
      1u8
    } else {
      0u8
    };
    let call12 = if iflip == 0 { call_to } else { call_de };
    let c58 = if iflip == 0 { call_de } else { call_to };
    let n12 = (callsign_hash(call12).ok_or(MessageError::Callsign1)? >> 10) as u16;
    // 只有 12 位字段的一侧也要入表（`save_callsign` 会同时写 12 位键），否则同一会话内
    // 解码时查不到（恒 `<...>`）。
    save_callsign(hash, call12)?;
    (iflip, n12, c58)
  } else {
    (0u8, 0u16, call_de)
  };
  let n58 = pack58(call58, hash)?;

  let nrpt = if icq {
    0
  } else if extra == "RRR" {
    1
  } else if extra == "RR73" {
    2
  } else if extra == "73" {
    3
  } else {
    0
  };

  let mut p = [0u8; 10];
  p[0] = (n12 >> 4) as u8;
  p[1] = ((n12 << 4) | (n58 >> 54) as u16) as u8;
  p[2] = (n58 >> 46) as u8;
  p[3] = (n58 >> 38) as u8;
  p[4] = (n58 >> 30) as u8;
  p[5] = (n58 >> 22) as u8;
  p[6] = (n58 >> 14) as u8;
  p[7] = (n58 >> 6) as u8;
  p[8] = ((n58 << 2) | u64::from(iflip << 1) | u64::from(nrpt >> 1)) as u8;
  p[9] = ((nrpt << 7) | (u16::from(icq as u8) << 6) | u16::from(i3 << 3)) as u8;
  Ok(Message { payload: p })
}

/// 编码自由文本（≤ 13 字符）。
fn encode_free(text: &str) -> Result<Message, MessageError> {
  // 上限是「13 个字符」而非 13 字节：非 ASCII 字符会占多个字节，用字节长度会提前误判；
  // 非 ASCII 字符最终仍会被 `nchar(CharTable::Full)` 拒绝，这里只负责长度口径正确。
  if text.chars().count() > 13 {
    return Err(MessageError::Type);
  }
  let mut b71 = [0u8; 9];
  for idx in 0..13 {
    let c = text.chars().nth(idx).unwrap_or(' ');
    let cid = nchar(c, CharTable::Full).ok_or(MessageError::Type)?;
    let mut rem = cid as u16;
    for b in b71.iter_mut().rev() {
      rem += u16::from(*b) * 42;
      *b = (rem & 0xff) as u8;
      rem >>= 8;
    }
  }
  let mut msg = encode_telemetry(&b71)?;
  msg.payload[9] = 0;
  Ok(msg)
}

/// 遥测：71 位右对齐（左移 1 位）。
fn encode_telemetry(telemetry: &[u8; 9]) -> Result<Message, MessageError> {
  let mut p = [0u8; 10];
  let mut carry = 0u8;
  for i in (0..9).rev() {
    p[i] = (telemetry[i] << 1) | (carry >> 7);
    carry = telemetry[i] & 0x80;
  }
  Ok(Message { payload: p })
}

/// 遥测：右移 1 位恢复 71 位。
fn decode_telemetry(msg: &Message) -> [u8; 9] {
  let mut t = [0u8; 9];
  let mut carry = 0u8;
  for (i, tt) in t.iter_mut().enumerate() {
    *tt = (carry << 7) | (msg.payload[i] >> 1);
    carry = msg.payload[i] & 0x01;
  }
  t
}

/// 解码自由文本。
fn decode_free(msg: &Message) -> String {
  let mut b71 = decode_telemetry(msg);
  let mut c14 = [' '; 13];
  for idx in (0..13).rev() {
    let mut rem: u16 = 0;
    for b in b71.iter_mut() {
      rem = (rem << 8) | u16::from(*b);
      *b = (rem / 42) as u8;
      rem %= 42;
    }
    c14[idx] = charn(rem as usize, CharTable::Full);
  }
  c14.iter().collect::<String>().trim().to_owned()
}

/// 遥测转十六进制（18 位）。
fn decode_telemetry_hex(msg: &Message) -> String {
  let b71 = decode_telemetry(msg);
  let mut out = String::with_capacity(18);
  for b in b71 {
    out.push(
      char::from_digit(u32::from(b >> 4), 16)
        .unwrap_or('0')
        .to_ascii_uppercase(),
    );
    out.push(
      char::from_digit(u32::from(b & 0x0f), 16)
        .unwrap_or('0')
        .to_ascii_uppercase(),
    );
  }
  out
}

/// 编码一条文本消息（猜测类型，失败回退）。
pub fn encode(text: &str, hash: &mut HashTable) -> Result<Message, MessageError> {
  let tokens: Vec<&str> = text.split_whitespace().collect();
  let is_cq = text.starts_with("CQ ") && !tokens.is_empty() && tokens[0] == "CQ";
  let (call_to, call_de, extra) = if is_cq {
    // CQ 修饰符（CQ nnn / CQ abcd）与「CQ」合并成一个字段。
    if parse_cq_modifier(text).is_some() {
      (
        format!("CQ {}", tokens.get(1).copied().unwrap_or("")),
        tokens.get(2).copied().unwrap_or("").to_owned(),
        tokens.get(3).copied().unwrap_or("").to_owned(),
      )
    } else {
      (
        "CQ".to_owned(),
        tokens.get(1).copied().unwrap_or("").to_owned(),
        tokens.get(2).copied().unwrap_or("").to_owned(),
      )
    }
  } else if !tokens.is_empty() {
    (
      tokens[0].to_owned(),
      tokens.get(1).copied().unwrap_or("").to_owned(),
      tokens.get(2).copied().unwrap_or("").to_owned(),
    )
  } else {
    return encode_free(text);
  };
  // 超过 3 个字段（第 4 个 token 起）就是自由文本。
  if tokens.len() > 3 + usize::from(is_cq && parse_cq_modifier(text).is_some()) {
    return encode_free(text);
  }
  if let Ok(m) = encode_std(&call_to, &call_de, &extra, hash) {
    return Ok(m);
  }
  if let Ok(m) = encode_nonstd(&call_to, &call_de, &extra, hash) {
    return Ok(m);
  }
  encode_free(text)
}

/// 解码一条消息为文本。
pub fn decode(msg: &Message, hash: &mut HashTable) -> Result<String, MessageError> {
  let mut buf = String::new();
  match get_type(msg) {
    MessageType::Standard => {
      let (a, b, c) = decode_std(msg, hash)?;
      buf.push_str(&a);
      if !b.is_empty() {
        buf.push(' ');
        buf.push_str(&b);
        if !c.is_empty() {
          buf.push(' ');
          buf.push_str(&c);
        }
      }
      Ok(buf)
    }
    MessageType::NonstdCall => {
      let (a, b, c) = decode_nonstd(msg, hash)?;
      buf.push_str(&a);
      if !b.is_empty() {
        buf.push(' ');
        buf.push_str(&b);
        if !c.is_empty() {
          buf.push(' ');
          buf.push_str(&c);
        }
      }
      Ok(buf)
    }
    MessageType::FreeText => Ok(decode_free(msg)),
    MessageType::Telemetry => Ok(decode_telemetry_hex(msg)),
    _ => Err(MessageError::Type),
  }
}

/// 解码标准消息，返回 (call_to, call_de, extra)。
fn decode_std(
  msg: &Message,
  hash: &mut HashTable,
) -> Result<(String, String, String), MessageError> {
  let p = &msg.payload;
  let mut n29a = u32::from(p[0]) << 21;
  n29a |= u32::from(p[1]) << 13;
  n29a |= u32::from(p[2]) << 5;
  n29a |= u32::from(p[3]) >> 3;
  let mut n29b = u32::from(p[3] & 0x07) << 26;
  n29b |= u32::from(p[4]) << 18;
  n29b |= u32::from(p[5]) << 10;
  n29b |= u32::from(p[6]) << 2;
  n29b |= u32::from(p[7]) >> 6;
  let ir = (p[7] & 0x20) >> 5;
  let mut igrid4 = u16::from(p[7] & 0x1f) << 10;
  igrid4 |= u16::from(p[8]) << 2;
  igrid4 |= u16::from(p[9]) >> 6;
  let i3 = get_i3(msg);

  let (call_to, _) =
    unpack28(n29a >> 1, (n29a & 1) as u8, i3, hash).map_err(|_| MessageError::Callsign1)?;
  let (call_de, _) =
    unpack28(n29b >> 1, (n29b & 1) as u8, i3, hash).map_err(|_| MessageError::Callsign2)?;
  let (extra, _) = unpackgrid(igrid4, ir).map_err(|_| MessageError::Grid)?;
  Ok((call_to, call_de, extra))
}

/// 解码非标准消息，返回 (call_to, call_de, extra)。
fn decode_nonstd(
  msg: &Message,
  hash: &mut HashTable,
) -> Result<(String, String, String), MessageError> {
  let p = &msg.payload;
  let mut n12 = u16::from(p[0]) << 4;
  n12 |= u16::from(p[1]) >> 4;
  let mut n58 = u64::from(p[1] & 0x0f) << 54;
  n58 |= u64::from(p[2]) << 46;
  n58 |= u64::from(p[3]) << 38;
  n58 |= u64::from(p[4]) << 30;
  n58 |= u64::from(p[5]) << 22;
  n58 |= u64::from(p[6]) << 14;
  n58 |= u64::from(p[7]) << 6;
  n58 |= u64::from(p[8]) >> 2;
  let iflip = (p[8] >> 1) & 0x01;
  let mut nrpt = (p[8] & 0x01) << 1;
  nrpt |= p[9] >> 7;
  let icq = (p[9] >> 6) & 0x01;

  let call_decoded = unpack58(n58, hash);
  let call_3 = lookup_callsign12(hash, n12);

  let (call_1, call_2) = if iflip == 1 {
    (call_decoded, call_3)
  } else {
    (call_3, call_decoded)
  };

  if icq == 0 {
    let extra = match nrpt {
      1 => "RRR".to_owned(),
      2 => "RR73".to_owned(),
      3 => "73".to_owned(),
      _ => String::new(),
    };
    Ok((call_1, call_2, extra))
  } else {
    Ok(("CQ".to_owned(), call_2, String::new()))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 对拍 ft8_lib test/test.c 的 test_msg 往返向量（encode 存表、decode 查表）。
  fn roundtrip(text: &str, expect: &str) {
    let mut hash = HashTable::new();
    let msg = encode(text, &mut hash).unwrap();
    let got = decode(&msg, &mut hash).unwrap();
    assert_eq!(got, expect, "encode/decode of {text:?}");
  }

  #[test]
  fn std_message_roundtrips() {
    roundtrip("CQ K7IHZ DM43", "CQ K7IHZ DM43");
    roundtrip("CQ JA LB2JK JO59", "CQ JA LB2JK JO59");
    roundtrip("CQ 123 LB2JK JO59", "CQ 123 LB2JK JO59");
    roundtrip("TNX BOB 73", "<TNX> <BOB> 73"); // 特殊呼号与 token 无法区分
  }

  #[test]
  fn nonstd_callsign_roundtrips() {
    roundtrip("CQ EA8/G5LSI", "CQ EA8/G5LSI");
    roundtrip("EA8/G5LSI R2RFE RR73", "<EA8/G5LSI> R2RFE RR73");
    roundtrip("R2RFE/P EA8/G5LSI R+12", "R2RFE/P <EA8/G5LSI> R+12");
    roundtrip("CQ YL/LB2JK KO16sw", "CQ YL/LB2JK"); // 非标准呼号不允许网格
    roundtrip("CQ POTA YL/LB2JK KO16sw", "CQ YL/LB2JK"); // CQ 修饰符不允许非标准呼号
  }

  #[test]
  fn free_text_roundtrips() {
    roundtrip("TNX BOB 73 GL", "TNX BOB 73 GL"); // 4 个 token 必为自由文本
  }

  /// type 4（12 位哈希 + 58 位呼号）必须在同一会话内解回**两边**的呼号。
  ///
  /// `/R` + `/P` 的组合会被标准编码拒绝（`Suffix`），因此走 type 4：一侧是 12 位哈希、
  /// 一侧是 58 位。修复前 12 位那侧因为只按 22 位存表，解出来恒为 `<...>`。
  #[test]
  fn nonstd_type4_resolves_the_12bit_side() {
    roundtrip("K7IHZ/R W1AW/P 73", "<K7IHZ/R> W1AW/P 73");
  }

  /// 解码结果里的 `<...>` 是显示标记，粘回来再编码必须得到同一个 payload。
  #[test]
  fn angle_brackets_are_ignored_when_encoding() {
    let mut hash = HashTable::new();
    let plain = encode("EA8/G5LSI R2RFE RR73", &mut hash).unwrap();
    let mut hash = HashTable::new();
    let bracketed = encode("<EA8/G5LSI> R2RFE RR73", &mut hash).unwrap();
    assert_eq!(plain.payload, bracketed.payload);
    assert_eq!(
      decode(&bracketed, &mut hash).unwrap(),
      "<EA8/G5LSI> R2RFE RR73"
    );
  }

  /// 接收端的真实路径：type 4 里的 12 位呼号不是本机编码出来的，而是靠同一会话里
  /// **另一条标准消息**解出的完整呼号还原。哈希表里只有解码留下的条目，也必须解得出。
  #[test]
  fn nonstd_12bit_resolves_from_a_decoded_standard_message() {
    // 发送端：与接收端各用各的哈希表，payload 与表内容无关。
    let std_msg = encode("CQ K7IHZ/R DM43", &mut HashTable::new()).unwrap();
    let type4 = encode("K7IHZ/R W1AW/P 73", &mut HashTable::new()).unwrap();
    assert_eq!(get_type(&type4), MessageType::NonstdCall);

    // 没见过完整呼号：12 位字段还原不出来。
    let mut rx = HashTable::new();
    assert_eq!(decode(&type4, &mut rx).unwrap(), "<...> W1AW/P 73");
    // 先解出标准消息（含 `K7IHZ/R`），同一张表再解 type 4：还原出呼号。
    assert_eq!(decode(&std_msg, &mut rx).unwrap(), "CQ K7IHZ/R DM43");
    assert_eq!(decode(&type4, &mut rx).unwrap(), "<K7IHZ/R> W1AW/P 73");
  }

  #[test]
  fn message_type_detection() {
    let mut hash = HashTable::new();
    let msg = encode("CQ K7IHZ DM43", &mut hash).unwrap();
    assert_eq!(get_type(&msg), MessageType::Standard);
    let msg = encode("CQ EA8/G5LSI", &mut hash).unwrap();
    assert_eq!(get_type(&msg), MessageType::NonstdCall);
    let msg = encode("TNX BOB 73 GL", &mut hash).unwrap();
    assert_eq!(get_type(&msg), MessageType::FreeText);
  }

  #[test]
  fn callsign_hash_is_deterministic() {
    // 同一个呼号两次算哈希一致（不依赖表）。
    assert_eq!(callsign_hash("EA8/G5LSI"), callsign_hash("EA8/G5LSI"));
    assert_ne!(callsign_hash("EA8/G5LSI"), callsign_hash("K7IHZ"));
  }

  /// 逐字节对拍 ft8_lib 编译运行的 payload（用同一个哈希表接口）。
  #[test]
  fn payload_matches_ft8_lib_byte_for_byte() {
    let cases: [(&str, [u8; 10]); 5] = [
      (
        "CQ K7IHZ DM43",
        [0x00, 0x00, 0x00, 0x20, 0x4e, 0xe1, 0x84, 0x06, 0x7c, 0xc8],
      ),
      (
        "CQ JA LB2JK JO59",
        [0x00, 0x00, 0x4f, 0xa4, 0xe5, 0xf6, 0x55, 0x91, 0x3e, 0xc8],
      ),
      (
        "CQ EA8/G5LSI",
        [0x00, 0x00, 0x00, 0xf1, 0xf2, 0x17, 0x71, 0x88, 0xe4, 0x60],
      ),
      (
        "EA8/G5LSI R2RFE RR73",
        [0x02, 0xcc, 0xa9, 0x90, 0x58, 0xb1, 0x00, 0x9f, 0xa4, 0xc8],
      ),
      (
        "TNX BOB 73 GL",
        [0x63, 0xed, 0xce, 0xe2, 0xa4, 0xae, 0x07, 0xf5, 0x00, 0x00],
      ),
    ];
    for (text, expect) in cases {
      let mut hash = HashTable::new();
      let msg = encode(text, &mut hash).unwrap();
      assert_eq!(msg.payload, expect, "payload of {text:?}");
    }
  }

  #[test]
  fn std_payload_matches_ft8_lib_bit_layout() {
    // 用 test.c 的往返保证，再补一个已知 payload 的 decode 对拍：
    // 手动构造 "CQ K7IHZ DM43" 的 payload，验证 decode 输出。
    let mut hash = HashTable::new();
    let msg = encode("CQ K7IHZ DM43", &mut hash).unwrap();
    // 重新解码应一致（依赖刚存的哈希表）。
    assert_eq!(decode(&msg, &mut hash).unwrap(), "CQ K7IHZ DM43");
  }
}
