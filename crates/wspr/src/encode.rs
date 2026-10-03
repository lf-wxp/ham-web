//! WSPR Type 1 消息编码：呼号 / 网格 / 功率 → 50 bit → 卷积码 → 交织 → 162 符号。

/// 同步向量（162 bit，21 字节 MSB-first 打包，末 6 bit 为填充）。
const SYNC_PACKED: [u8; 21] = [
  0xc0, 0x8e, 0x25, 0xe0, 0x25, 0x02, 0xcd, 0x1a, 0x1a, 0xa9, 0x2c, 0x6a, 0x20, 0x93, 0xb3, 0x47,
  0x05, 0x30, 0x1a, 0xc6, 0x00,
];

/// 卷积码生成多项式（K=32，r=1/2，非系统）。
pub(crate) const POLY_0: u32 = 0xf2d0_5351;
pub(crate) const POLY_1: u32 = 0xe461_3c47;

pub(crate) const ALPHABET_37: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ ";
pub(crate) const ALPHABET_36: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
pub(crate) const ALPHABET_27: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ ";

/// 合法功率等级（dBm）。
pub const POWER_LEVELS: [u8; 19] = [
  0, 3, 7, 10, 13, 17, 20, 23, 27, 30, 33, 37, 40, 43, 47, 50, 53, 57, 60,
];

/// 同步向量的第 `i` 位（0..162）。
pub fn sync_bit(i: usize) -> u8 {
  (SYNC_PACKED[i >> 3] >> (7 - (i & 7))) & 1
}

fn index_of(alphabet: &[u8], c: u8) -> Option<usize> {
  alphabet.iter().position(|&a| a == c)
}

/// 32 位奇偶校验（折叠到 4 bit 后查表）。
pub(crate) fn parity32(mut value: u32) -> u8 {
  value ^= value >> 16;
  value ^= value >> 8;
  value ^= value >> 4;
  value &= 0x0f;
  ((0x6996u32 >> value) & 1) as u8
}

/// 规范化呼号为 6 字符字段（第 3 个字符必须是数字，否则前面补空格）。
fn normalize_callsign(call: &str) -> [u8; 6] {
  let bytes = call.trim().to_ascii_uppercase().into_bytes();
  let padded: Vec<u8> = if bytes.get(2).is_some_and(|c| c.is_ascii_digit()) {
    bytes
  } else {
    let mut v = vec![b' '];
    v.extend(bytes);
    v
  };
  let mut field = [b' '; 6];
  for (i, &c) in padded.iter().take(6).enumerate() {
    field[i] = c;
  }
  field
}

/// 呼号 6 字符字段 → 28 bit。
fn pack_callsign(field: &[u8; 6]) -> u32 {
  let mut n = index_of(ALPHABET_37, field[0]).unwrap_or(0) as u32;
  n = n * 36 + index_of(ALPHABET_36, field[1]).unwrap_or(0) as u32;
  n = n * 10 + index_of(ALPHABET_36, field[2]).unwrap_or(0) as u32;
  n = n * 27 + index_of(ALPHABET_27, field[3]).unwrap_or(0) as u32;
  n = n * 27 + index_of(ALPHABET_27, field[4]).unwrap_or(0) as u32;
  n = n * 27 + index_of(ALPHABET_27, field[5]).unwrap_or(0) as u32;
  n
}

/// 网格（4 位）+ 功率 → 22 bit；网格非法（长度 / 字符范围）时返回 `None`。
fn pack_locator_power(locator: &str, power: u8) -> Option<u32> {
  let b = locator.as_bytes();
  // Maidenhead 4 位：前两位 A–R，后两位 0–9。先校验再索引，避免越界与下溢。
  if b.len() != 4
    || !(b'A'..=b'R').contains(&b[0])
    || !(b'A'..=b'R').contains(&b[1])
    || !b[2].is_ascii_digit()
    || !b[3].is_ascii_digit()
  {
    return None;
  }
  let lon_field = (b[0] - b'A') as i32;
  let lat_field = (b[1] - b'A') as i32;
  let lon_sq = (b[2] - b'0') as i32;
  let lat_sq = (b[3] - b'0') as i32;
  let grid = 180 * (179 - 10 * lon_field - lon_sq) + 10 * lat_field + lat_sq;
  Some(grid as u32 * 128 + power as u32 + 64)
}

/// 打包消息为 11 字节（50 bit 消息 + 零尾）；网格非法时返回 `None`。
fn pack_message(call: &str, locator: &str, power: u8) -> Option<[u8; 11]> {
  let field = normalize_callsign(call);
  let c = pack_callsign(&field);
  let g = pack_locator_power(locator, power)?;
  let mut bytes = [0u8; 11];
  bytes[0] = (c >> 20) as u8;
  bytes[1] = (c >> 12) as u8;
  bytes[2] = (c >> 4) as u8;
  bytes[3] = ((c << 4) & 0xf0) as u8 | ((g >> 18) & 0x0f) as u8;
  bytes[4] = (g >> 10) as u8;
  bytes[5] = (g >> 2) as u8;
  bytes[6] = (g << 6) as u8;
  Some(bytes)
}

/// 卷积编码：81 bit（50 消息 + 31 零尾）→ 162 bit。
fn convolutional_encode(bytes: &[u8; 11]) -> [u8; 162] {
  let mut out = [0u8; 162];
  let mut register: u32 = 0;
  let mut at = 0;
  for i in 0..81 {
    let bit = (bytes[i >> 3] >> (7 - (i & 7))) & 1;
    register = (register << 1) | bit as u32;
    out[at] = parity32(register & POLY_0);
    out[at + 1] = parity32(register & POLY_1);
    at += 2;
  }
  out
}

pub(crate) fn reverse8(mut value: u8) -> u8 {
  let mut out = 0;
  for _ in 0..8 {
    out = (out << 1) | (value & 1);
    value >>= 1;
  }
  out
}

/// 交织：8 bit 反转，保留目标 < 162 的位置。
pub(crate) fn interleave(bits: &[u8; 162]) -> [u8; 162] {
  let mut out = [0u8; 162];
  let mut source = 0;
  for i in 0..256u16 {
    let dest = reverse8(i as u8) as usize;
    if dest < 162 {
      out[dest] = bits[source];
      source += 1;
    }
    if source >= 162 {
      break;
    }
  }
  out
}

/// 编码一条 Type 1 消息为 162 个符号（每个取值 0–3）。
///
/// 呼号与网格非法（网格须为 4 位 Maidenhead）时返回 `None`。
pub fn encode(call: &str, locator: &str, power: u8) -> Option<[u8; 162]> {
  let bytes = pack_message(call, locator, power)?;
  let data = interleave(&convolutional_encode(&bytes));
  let mut symbols = [0u8; 162];
  for (i, s) in symbols.iter_mut().enumerate() {
    *s = sync_bit(i) + 2 * data[i];
  }
  Some(symbols)
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 黄金向量：AA0NT EM18 20（WSPR-Reference test_vectors）。
  #[test]
  fn golden_vector_aa0nt_em18_20() {
    let expected = "132000023020111000302321113000022230012300222012112033210003103022213010303230030032332021321030223220203223221310310213010221110002210302110200222310303320011002";
    let symbols = encode("AA0NT", "EM18", 20).expect("应编码成功");
    let got: String = symbols.iter().map(|&s| (b'0' + s) as char).collect();
    assert_eq!(got, expected);
  }

  /// 黄金向量：AA0NT EM18 37。
  #[test]
  fn golden_vector_aa0nt_em18_37() {
    let expected = "132202003022131200302323113002002232012302202012132231210201103222213210303230010232312023321232223022203021221110310011010221130002210302110202202112323320031202";
    let symbols = encode("AA0NT", "EM18", 37).expect("应编码成功");
    let got: String = symbols.iter().map(|&s| (b'0' + s) as char).collect();
    assert_eq!(got, expected);
  }

  /// 非法网格（长度 / 字符范围不符）应返回 `None` 而不是越界或下溢。
  #[test]
  fn encode_rejects_invalid_locator() {
    assert!(encode("AA0NT", "EM", 20).is_none());
    assert!(encode("AA0NT", "EM1", 20).is_none());
    assert!(encode("AA0NT", "EM18X", 20).is_none());
    assert!(encode("AA0NT", "em18", 20).is_none()); // 小写不符合规范
    assert!(encode("AA0NT", "S9XX", 20).is_none()); // 前两位超出 A–R
    assert!(encode("AA0NT", "EM18", 20).is_some());
  }
}
