//! FT8 / FT4 的 CRC-14。
//!
//! 转写自 MIT 许可的 ft8_lib `crc.c`（多项式 0x2757，宽度 14）。
//! FT8 消息的 77 位有效载荷加 14 位 CRC 组成 91 位，CRC 在源编码消息零扩展 77→82 位后计算。

/// CRC-14 多项式（不含前导 MSB 1）。
const CRC_POLYNOMIAL: u16 = 0x2757;
/// CRC 宽度（位）。
const CRC_WIDTH: u16 = 14;
/// 最高位掩码。
const TOPBIT: u16 = 1 << (CRC_WIDTH - 1);

/// 计算一段（按 MSB first 打包的）比特序列的 14 位 CRC。
///
/// `message` 是字节序列，`num_bits` 是要参与计算的位数；`num_bits > message.len() * 8`
/// 时返回 `None`（调用者给错长度时报错，而不是越界 panic）。
#[must_use]
pub fn compute_crc(message: &[u8], num_bits: usize) -> Option<u16> {
  if num_bits > message.len() * 8 {
    return None;
  }
  let mut remainder: u16 = 0;
  let mut idx_byte = 0;
  for idx_bit in 0..num_bits {
    if idx_bit % 8 == 0 {
      remainder ^= u16::from(message[idx_byte]) << (CRC_WIDTH - 8);
      idx_byte += 1;
    }
    if remainder & TOPBIT != 0 {
      remainder = (remainder << 1) ^ CRC_POLYNOMIAL;
    } else {
      remainder <<= 1;
    }
  }
  Some(remainder & ((TOPBIT << 1) - 1))
}

/// 把 77 位 payload（10 字节，MSB first）加 14 位 CRC，得到 91 位（12 字节）。
///
/// 对应 ft8_lib `ftx_add_crc`：先零扩展 77→82 位再算 CRC，CRC 的 14 位落在 `a91[9..12]`。
pub fn add_crc(payload: &[u8; 10], a91: &mut [u8; 12]) {
  a91[..10].copy_from_slice(payload);
  a91[9] &= 0xF8;
  a91[10] = 0;
  // CRC 在 82 位（96 - 14）上计算：a91 固定 12 字节，长度恒合法。
  let checksum = compute_crc(a91, 96 - 14).expect("a91 有 12 字节，82 位恒在范围内");
  // checksum 是 14 位：bit 11–13 → a91[9] 低 3 位，bit 3–10 → a91[10]，bit 0–2 → a91[11] 高 3 位。
  a91[9] |= (checksum >> 11) as u8;
  a91[10] = (checksum >> 3) as u8;
  a91[11] = (u32::from(checksum) << 5) as u8;
}

/// 从 91 位（12 字节）里取出 CRC（14 位）。
pub fn extract_crc(a91: &[u8; 12]) -> u16 {
  (u16::from(a91[9] & 0x07) << 11) | (u16::from(a91[10]) << 3) | u16::from(a91[11] >> 5)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn crc_matches_ft8_lib() {
    // ft8_lib test/test.c 的 test3：0x11 00 00 00 00 0e 10 04 01 00 的 76 位 CRC = 0x2e63。
    // （test.c 注释里写的 0x0708 是过时值；以实际编译运行的 ft8_lib 输出 0x2e63 为准。）
    let in2: [u8; 10] = [0x11, 0x00, 0x00, 0x00, 0x00, 0x0e, 0x10, 0x04, 0x01, 0x00];
    assert_eq!(compute_crc(&in2, 76), Some(0x2e63));
    assert_eq!(compute_crc(&in2, 81), None); // 超长：报错而不是越界 panic
  }

  #[test]
  fn add_and_extract_crc_roundtrip() {
    let payload: [u8; 10] = [0x11, 0x00, 0x00, 0x00, 0x00, 0x0e, 0x10, 0x04, 0x01, 0x00];
    let mut a91 = [0u8; 12];
    add_crc(&payload, &mut a91);
    // 前 9 字节原样；第 10 字节（索引 9）高 5 位保留 payload、低 3 位被 CRC 高 3 位占据。
    assert_eq!(&a91[..9], &payload[..9]);
    assert_eq!(a91[9] & 0xF8, payload[9] & 0xF8);
    // 取出的 CRC = add_crc 写入的 CRC = 在「写入前」的 82 位（77 payload + 5 零）上算出的值。
    let mut pre = [0u8; 12];
    pre[..10].copy_from_slice(&payload);
    pre[9] &= 0xF8;
    let checksum = compute_crc(&pre, 82).unwrap();
    assert_eq!(extract_crc(&a91), checksum);
  }
}
