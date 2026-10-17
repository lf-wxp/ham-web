//! FT8 / FT4 编码与解码内核。
//!
//! 转写自 MIT 许可的 [ft8_lib](https://github.com/kgoba/ft8_lib)（Copyright (c) 2018 Kārlis Goba）。
//! 数值逐例对 ft8_lib 编译运行的结果（见单测）。
//!
//! 组成：
//! - [`message`]：文本消息 ↔ 77 位 payload（标准 / 非标准呼号 / 自由文本 / 遥测）；
//! - [`crc`]、[`ldpc`]：CRC-14 与 LDPC(174,91) 编码 / 软译码；
//! - [`monitor`]：音频 → waterfall（滑窗 + Hann + 实数 FFT）；
//! - [`decode`]：Costas 同步打分 → 候选 → 似然提取 → 软译码 → CRC 校验；
//! - [`recording`]：整段录音 → 按时隙切片逐个解码 → 汇总（页面 / Worker 的入口）；
//! - [`audio`]：信道符号 → GFSK 音频（测试与「发」方向用）。
//!
//! 已知范围限制：消息层只解析标准呼号、非标准呼号（type 4）、自由文本与遥测（十六进制）四类。
//! DXpedition（Fox / Hound）、EU VHF、ARRL Field Day、RTTY Roundup、WWROF 等能通过 CRC，
//! 但没有字段解析，[`decode_recording`] 只计数、不列文本；
//! FT4 只有编码方向有真实信号对拍，解调没有。
//!
//! # FT8 消息结构
//!
//! ```text
//! 77 位 payload → +14 位 CRC = 91 位 → LDPC 编码 = 174 位 → 8-FSK（3 比特/符号 × 58）交织进
//! S7 D29 S7 D29 S7（79 个信道符号，S 是 7 符号 Costas 同步）
//! ```
//!
//! # 符号层
//!
//! 编码函数输出的 `tones` 是信道符号（0–7，对应 8 个音），还不是音频采样；
//! 需要 WAV 时走 [`synthesize`] / [`synth_gfsk`]（GFSK 成形 + 首尾 ramp）。

mod audio;
mod constants;
mod crc;
mod decode;
mod fft;
mod ldpc;
mod message;
mod monitor;
mod recording;
mod text;

pub use audio::{synth_gfsk, synthesize, synthesize_payload};
pub use crc::{add_crc, compute_crc, extract_crc};
pub use decode::{Candidate, DecodeStatus, decode_candidate, decode_waterfall, find_candidates};
pub use fft::{Cpx, Fft, RealFft};
pub use ldpc::{bp_decode, pack_bits};
pub use message::{
  Field, HashTable, Message, MessageError, MessageType, decode, encode, get_i3, get_n3, get_type,
};
pub use monitor::{Monitor, MonitorConfig, Protocol, Waterfall};
pub use recording::{DecodedMessage, Recording, RecordingError, SAMPLE_RATE, decode_recording};

use constants::{
  FT4_COSTAS_PATTERN, FT4_GRAY_MAP, FT4_XOR_SEQUENCE, FT8_COSTAS_PATTERN, FT8_GRAY_MAP,
  LDPC_GENERATOR, LDPC_K,
};

/// FT8 信道符号总数（79）。
pub const FT8_NN: usize = 79;
/// FT4 信道符号总数（105）。
pub const FT4_NN: usize = 105;
/// 91 位（payload + CRC）的字节数。
pub const LDPC_K_BYTES: usize = 12;
/// 174 位码字的字节数。
pub const LDPC_N_BYTES: usize = 22;

/// 返回 `x` 里置 1 的位数是否为奇数（奇偶校验，模 2 和）。
fn parity8(x: u8) -> u8 {
  let mut x = x;
  x ^= x >> 4;
  x ^= x >> 2;
  x ^= x >> 1;
  x & 1
}

/// LDPC 编码：91 位 message（12 字节，MSB first）→ 174 位 codeword（22 字节）。
///
/// 对应 ft8_lib `encode174`：(174,91) 规则 LDPC 码，列重 3，生成矩阵 83×12。
pub fn encode174(message: &[u8; LDPC_K_BYTES], codeword: &mut [u8; LDPC_N_BYTES]) {
  // 先填 message + 零，后面只更新为 1 的校验位。
  for (j, cw) in codeword.iter_mut().enumerate() {
    *cw = if j < LDPC_K_BYTES { message[j] } else { 0 };
  }
  // 第一个校验位的字节下标与位掩码。
  let mut col_mask: u8 = 0x80u8 >> (LDPC_K % 8);
  let mut col_idx = LDPC_K_BYTES - 1;

  for row in &LDPC_GENERATOR {
    // 只算模 2 和：message 与生成矩阵第 i 行的按位点积的奇偶。
    let mut nsum: u8 = 0;
    for j in 0..LDPC_K_BYTES {
      nsum ^= parity8(message[j] & row[j]);
    }
    if nsum & 1 != 0 {
      codeword[col_idx] |= col_mask;
    }
    col_mask >>= 1;
    if col_mask == 0 {
      col_mask = 0x80;
      col_idx += 1;
    }
  }
  debug_assert_eq!(col_idx, LDPC_N_BYTES - 1);
}

/// FT8 编码：77 位 payload（10 字节，MSB first）→ 79 个信道符号。
///
/// 对应 ft8_lib `ft8_encode`：加 CRC、LDPC 编码、3 比特/符号经 Gray 映射，插入 3 段 Costas 同步。
pub fn ft8_encode(payload: &[u8; 10], tones: &mut [u8; FT8_NN]) {
  let mut a91 = [0u8; LDPC_K_BYTES];
  add_crc(payload, &mut a91);
  let mut codeword = [0u8; LDPC_N_BYTES];
  encode174(&a91, &mut codeword);

  let mut mask = 0x80u8;
  let mut i_byte = 0;
  for (i_tone, tone) in tones.iter_mut().enumerate() {
    if i_tone < 7 {
      *tone = FT8_COSTAS_PATTERN[i_tone];
    } else if (36..43).contains(&i_tone) {
      *tone = FT8_COSTAS_PATTERN[i_tone - 36];
    } else if (72..79).contains(&i_tone) {
      *tone = FT8_COSTAS_PATTERN[i_tone - 72];
    } else {
      let mut bits3 = 0u8;
      if codeword[i_byte] & mask != 0 {
        bits3 |= 4;
      }
      mask >>= 1;
      if mask == 0 {
        mask = 0x80;
        i_byte += 1;
      }
      if codeword[i_byte] & mask != 0 {
        bits3 |= 2;
      }
      mask >>= 1;
      if mask == 0 {
        mask = 0x80;
        i_byte += 1;
      }
      if codeword[i_byte] & mask != 0 {
        bits3 |= 1;
      }
      mask >>= 1;
      if mask == 0 {
        mask = 0x80;
        i_byte += 1;
      }
      *tone = FT8_GRAY_MAP[bits3 as usize];
    }
  }
}

/// FT4 编码：77 位 payload（10 字节）→ 105 个信道符号。
///
/// 对应 ft8_lib `ft4_encode`：先异或伪随机序列、加 CRC、LDPC 编码、2 比特/符号 Gray 映射，插入 4 段 Costas 同步与首尾 ramp。
pub fn ft4_encode(payload: &[u8; 10], tones: &mut [u8; FT4_NN]) {
  let mut payload_xor = [0u8; 10];
  for (i, b) in payload_xor.iter_mut().enumerate() {
    *b = payload[i] ^ FT4_XOR_SEQUENCE[i];
  }
  let mut a91 = [0u8; LDPC_K_BYTES];
  add_crc(&payload_xor, &mut a91);
  let mut codeword = [0u8; LDPC_N_BYTES];
  encode174(&a91, &mut codeword);

  let mut mask = 0x80u8;
  let mut i_byte = 0;
  for (i_tone, tone) in tones.iter_mut().enumerate() {
    if i_tone == 0 || i_tone == 104 {
      *tone = 0; // ramp 符号
    } else if (1..5).contains(&i_tone) {
      *tone = FT4_COSTAS_PATTERN[0][i_tone - 1];
    } else if (34..38).contains(&i_tone) {
      *tone = FT4_COSTAS_PATTERN[1][i_tone - 34];
    } else if (67..71).contains(&i_tone) {
      *tone = FT4_COSTAS_PATTERN[2][i_tone - 67];
    } else if (100..104).contains(&i_tone) {
      *tone = FT4_COSTAS_PATTERN[3][i_tone - 100];
    } else {
      let mut bits2 = 0u8;
      if codeword[i_byte] & mask != 0 {
        bits2 |= 2;
      }
      mask >>= 1;
      if mask == 0 {
        mask = 0x80;
        i_byte += 1;
      }
      if codeword[i_byte] & mask != 0 {
        bits2 |= 1;
      }
      mask >>= 1;
      if mask == 0 {
        mask = 0x80;
        i_byte += 1;
      }
      *tone = FT4_GRAY_MAP[bits2 as usize];
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::message::{HashTable, decode, encode};

  /// 完整数字层闭环：消息 → payload → CRC → LDPC 编码 → 软信息 → LDPC 译码 → 消息。
  /// 覆盖 message / crc / encode174 / bp_decode 四层，证明「编出去的东西译得回来」。
  #[test]
  fn full_digital_roundtrip_message_to_codeword_and_back() {
    let mut hash = HashTable::new();
    let text = "CQ K7IHZ DM43";
    let msg = encode(text, &mut hash).unwrap();

    // payload(77 位) → a91(91 位) → codeword(174 位)。
    let mut a91 = [0u8; LDPC_K_BYTES];
    add_crc(&msg.payload, &mut a91);
    let mut codeword = [0u8; LDPC_N_BYTES];
    encode174(&a91, &mut codeword);

    // 码字 → 软信息（约定 log(P(1)/P(0))，±5）→ 软译码。
    let mut bits = [0u8; 174];
    for i in 0..174 {
      bits[i] = (codeword[i / 8] >> (7 - (i % 8))) & 1;
    }
    let mut soft = [0.0f32; 174];
    for (i, s) in soft.iter_mut().enumerate() {
      *s = if bits[i] != 0 { 5.0 } else { -5.0 };
    }
    let mut plain = [0u8; 174];
    let errors = bp_decode(&soft, 25, &mut plain);
    assert_eq!(errors, 0);

    // 前 91 位应精确恢复 a91；再提取 77 位 payload 解回文本。
    let mut a91_decoded = [0u8; LDPC_K_BYTES];
    for i in 0..91 {
      if plain[i] != 0 {
        a91_decoded[i / 8] |= 0x80 >> (i % 8);
      }
    }
    assert_eq!(a91_decoded, a91);
    let mut payload = [0u8; 10];
    payload.copy_from_slice(&a91_decoded[..10]);
    payload[9] &= 0xF8; // 去掉 CRC 占据的低 3 位
    assert_eq!(decode(&Message { payload }, &mut hash).unwrap(), text);
  }

  /// 编码器的对拍目标来自 ft8_lib 编译运行的输出（本仓库转写后逐例一致）。
  #[test]
  fn ft8_encode_matches_ft8_lib() {
    // payload = 0x11 00 00 00 00 0e 10 04 01 00
    let payload: [u8; 10] = [0x11, 0x00, 0x00, 0x00, 0x00, 0x0e, 0x10, 0x04, 0x01, 0x00];
    let mut tones = [0u8; FT8_NN];
    ft8_encode(&payload, &mut tones);
    // ft8_lib 输出（79 个符号）：
    let expect: [u8; FT8_NN] = {
      let s = "3140652053000000000001405003001015773140652371736454671610306010374601573140652";
      let b = s.as_bytes();
      let mut a = [0u8; FT8_NN];
      for i in 0..FT8_NN {
        a[i] = b[i] - b'0';
      }
      a
    };
    assert_eq!(tones, expect);
  }

  #[test]
  fn ft4_encode_matches_ft8_lib() {
    let payload: [u8; 10] = [0x11, 0x00, 0x00, 0x00, 0x00, 0x0e, 0x10, 0x04, 0x01, 0x00];
    let mut tones = [0u8; FT4_NN];
    ft4_encode(&payload, &mut tones);
    let expect: [u8; FT4_NN] = {
      let s = "001321132112330313210320030101331110231013222033011122201032120133123103133221223312302130022331333132010";
      let b = s.as_bytes();
      let mut a = [0u8; FT4_NN];
      for i in 0..FT4_NN {
        a[i] = b[i] - b'0';
      }
      a
    };
    assert_eq!(tones, expect);
  }
}
