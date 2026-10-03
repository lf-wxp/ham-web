//! WSPR 解码：去交织 + 卷积解码（堆栈算法）+ 消息解包。

use std::collections::BinaryHeap;

use super::encode::{ALPHABET_27, ALPHABET_36, ALPHABET_37, POLY_0, POLY_1, parity32, reverse8};

/// 解码出的 WSPR 消息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
  pub callsign: String,
  pub locator: String,
  pub power: u8,
}

impl std::fmt::Display for Message {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{} {} {}", self.callsign, self.locator, self.power)
  }
}

/// 去交织（交织的逆映射）。
fn deinterleave(bits: &[u8; 162]) -> [u8; 162] {
  let mut out = [0u8; 162];
  let mut source = 0;
  for i in 0..256u16 {
    let dest = reverse8(i as u8) as usize;
    if dest < 162 {
      out[source] = bits[dest];
      source += 1;
    }
  }
  out
}

/// 堆栈解码节点。
///
/// 派生排序按字段声明顺序比较（`metric` 在前），`BinaryHeap` 为大顶堆，因此
/// metric 最大者优先弹出；其余字段一并参与比较，保证 `Ord` 与 `Eq` 一致。
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
struct Node {
  metric: i32,
  depth: u8,
  register: u32,
  path: u128,
}

/// 卷积解码（堆栈算法）：162 bit → 81 bit（前 50 bit 为消息）。
fn convolutional_decode(received: &[u8; 162]) -> Option<u128> {
  let mut heap = BinaryHeap::new();
  heap.push(Node {
    metric: 0,
    depth: 0,
    register: 0,
    path: 0,
  });
  while let Some(node) = heap.pop() {
    if node.depth == 81 {
      return Some(node.path);
    }
    let d = node.depth as usize;
    for bit in [0u32, 1u32] {
      let reg = (node.register << 1) | bit;
      let p0 = parity32(reg & POLY_0);
      let p1 = parity32(reg & POLY_1);
      let m = (p0 == received[2 * d]) as i32 * 2 - 1 + (p1 == received[2 * d + 1]) as i32 * 2 - 1;
      heap.push(Node {
        metric: node.metric + m,
        depth: node.depth + 1,
        register: reg,
        path: (node.path << 1) | bit as u128,
      });
    }
  }
  None
}

/// 解包 11 字节为消息（呼号 / 网格 / 功率）。
fn unpack(bytes: &[u8; 11]) -> Message {
  let c = ((bytes[0] as u32) << 20)
    | ((bytes[1] as u32) << 12)
    | ((bytes[2] as u32) << 4)
    | ((bytes[3] as u32) >> 4);
  let g = (((bytes[3] as u32) & 0x0f) << 18)
    | ((bytes[4] as u32) << 10)
    | ((bytes[5] as u32) << 2)
    | ((bytes[6] as u32) >> 6);

  // 反解呼号（28 bit）。
  let mut n = c;
  let c5 = n % 27;
  n /= 27;
  let c4 = n % 27;
  n /= 27;
  let c3 = n % 27;
  n /= 27;
  let c2 = n % 10;
  n /= 10;
  let c1 = n % 36;
  n /= 36;
  let c0 = n % 37;
  let field = [
    ALPHABET_37[c0 as usize],
    ALPHABET_36[c1 as usize],
    ALPHABET_36[c2 as usize],
    ALPHABET_27[c3 as usize],
    ALPHABET_27[c4 as usize],
    ALPHABET_27[c5 as usize],
  ];
  let callsign = String::from_utf8_lossy(&field).trim().to_owned();

  // 反解网格 + 功率（22 bit）。
  let power = (g % 128).saturating_sub(64) as u8;
  let grid = g / 128;
  let m = grid / 180;
  let n = grid % 180;
  let lat_field = n / 10;
  let lat_sq = n % 10;
  // 损坏解码可能使 m > 179，直接用 179 - m 会下溢；saturating_sub 兜底为 0，
  // 避免 debug panic / release 回绕。
  let lon = 179u32.saturating_sub(m);
  let lon_field = lon / 10;
  let lon_sq = lon % 10;
  let locator = format!(
    "{}{}{}{}",
    (lon_field + 65) as u8 as char,
    (lat_field + 65) as u8 as char,
    (lon_sq + 48) as u8 as char,
    (lat_sq + 48) as u8 as char,
  );

  Message {
    callsign,
    locator,
    power,
  }
}

/// 由 162 个符号解码消息（符号取值 0–3，`symbol = sync + 2·data`）。
pub fn decode_symbols(symbols: &[u8; 162]) -> Option<Message> {
  // 提取数据 bit（bit = symbol >> 1）。
  let mut received = [0u8; 162];
  for (i, &s) in symbols.iter().enumerate() {
    received[i] = s >> 1;
  }
  let deinterleaved = deinterleave(&received);
  let path = convolutional_decode(&deinterleaved)?;

  // 前 50 bit → 11 字节（MSB-first）。
  let mut bytes = [0u8; 11];
  for (i, byte) in bytes.iter_mut().enumerate().take(7) {
    for b in 0..8 {
      let idx = i * 8 + b;
      if idx < 50 {
        let bit = ((path >> (80 - idx)) & 1) as u8;
        *byte |= bit << (7 - b);
      }
    }
  }
  Some(unpack(&bytes))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::encode::encode;

  #[test]
  fn roundtrip_type1() {
    for (call, loc, power) in [
      ("AA0NT", "EM18", 20u8),
      ("AA0NT", "EM18", 37),
      ("K1ABC", "FN20", 30),
      ("G4JNT", "IO91", 40),
    ] {
      let symbols = encode(call, loc, power).expect("应编码成功");
      let msg = decode_symbols(&symbols).expect("应解码成功");
      assert_eq!(msg.callsign, call, "呼号 {call}");
      assert_eq!(msg.locator, loc, "网格 {loc}");
      assert_eq!(msg.power, power, "功率 {power}");
    }
  }
}
