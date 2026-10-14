//! DominoEX FEC（MultiPSK 兼容）：卷积码 + 4×4×4 交织 + IZ8BLY MFSK Varicode。
//!
//! 转写自 fldigi（GPL-3.0）`src/dominoex/dominoex.cxx` 的 `sendMuPskEX` / `decodeMuPskEX`
//! 一组方法，以及它们用到的 `src/filters/viterbi.cxx`（编码器与 Viterbi 译码器）、
//! `src/mfsk/interleave.cxx`（交织器）。参数照抄 fldigi：
//!
//! - 卷积码 K = 7、多项式 0x6D / 0x4F（与 MFSK16 相同的「NASA」码），码率 1/2；
//! - 发送：每个 Varicode 位进编码器得到 2 个码位，**4 个码位凑成一个 nibble**，过前向交织器
//!   （size 4、depth 4）后作为 IFK+ 符号发出 —— 所以 FEC 不改变调制，只改变 nibble 的含义；
//! - 接收：nibble 拆成 4 个硬判决软值（1 → 255、0 → 1），过反向交织器（初值 128 = 删除位），
//!   每两个软值进一次 Viterbi（回溯 45、每次出 1 位）；
//! - 字符边界：Varicode 码字以 1 开头、以 `00` 结尾、内部不含 `001`，所以位流里出现 `001`
//!   就是「上一个字符结束、下一个字符开始」。
//!
//! 二次信道（ID 信标）不是独立的码表：MultiPSK 把一部分**主信道里用不到的码字**挪去表示
//! 二次信道的字符（`MuPskSec2Pri` / `MuPskPriSecChar`）。代价是主信道丢了这些码字原本的
//! 字符（多数控制符与 0x80 以上的字符），发送前统一换成 `_` —— 与 fldigi 一致。

use crate::mfsk_varicode::VARICODE;

/// 约束长度。
const K: u32 = 7;
/// 两个生成多项式（fldigi `dominoex.h` 的 `POLY1` / `POLY2`）。
const POLY1: u32 = 0x6D;
const POLY2: u32 = 0x4F;
/// 交织器：size × size × depth。
const INLV_SIZE: usize = 4;
const INLV_DEPTH: usize = 4;
/// 反向交织器的初值：删除位（软值居中，两种假设的度量相同）。
const PUNCTURE: u8 = 128;
/// Viterbi 的路径存储深度（fldigi `PATHMEM`）。
const PATHMEM: usize = 256;
/// 回溯深度（fldigi 在 DominoEX 里 `settraceback(45)`）。
const TRACEBACK: usize = 45;
const NSTATES: usize = 1 << (K - 1);

fn parity(x: u32) -> u32 {
  x.count_ones() & 1
}

/// `(POLY1 码位) | (POLY2 码位) << 1`，下标为 K 位移位寄存器。
fn conv_output(state: u32) -> u32 {
  parity(POLY1 & state) | (parity(POLY2 & state) << 1)
}

/// 卷积编码器。
#[derive(Debug, Default, Clone)]
struct ConvEncoder {
  shreg: u32,
}

impl ConvEncoder {
  fn encode(&mut self, bit: bool) -> u32 {
    self.shreg = (self.shreg << 1) | u32::from(bit);
    conv_output(self.shreg & ((1 << K) - 1))
  }
}

/// 卷积交织器（fldigi `interleave`）：`depth` 级，每级一个 size×size 的移位方阵，
/// 前向取反对角线、反向取主对角线，两者串起来恰好把每个位置的延迟补齐。
#[derive(Debug, Clone)]
struct Interleaver {
  forward: bool,
  table: [u8; INLV_SIZE * INLV_SIZE * INLV_DEPTH],
}

impl Interleaver {
  fn new(forward: bool) -> Self {
    Self {
      forward,
      table: [if forward { 0 } else { PUNCTURE }; INLV_SIZE * INLV_SIZE * INLV_DEPTH],
    }
  }

  fn idx(k: usize, i: usize, j: usize) -> usize {
    INLV_SIZE * INLV_SIZE * k + INLV_SIZE * i + j
  }

  fn symbols(&mut self, syms: &mut [u8; INLV_SIZE]) {
    for k in 0..INLV_DEPTH {
      for i in 0..INLV_SIZE {
        for j in 0..INLV_SIZE - 1 {
          self.table[Self::idx(k, i, j)] = self.table[Self::idx(k, i, j + 1)];
        }
      }
      for (i, s) in syms.iter().enumerate() {
        self.table[Self::idx(k, i, INLV_SIZE - 1)] = *s;
      }
      for (i, s) in syms.iter_mut().enumerate() {
        let j = if self.forward { INLV_SIZE - i - 1 } else { i };
        *s = self.table[Self::idx(k, i, j)];
      }
    }
  }

  /// 4 位整数（最高位先出）过交织器。
  fn bits(&mut self, value: u32) -> u32 {
    let mut syms = [0u8; INLV_SIZE];
    for (i, s) in syms.iter_mut().enumerate() {
      *s = ((value >> (INLV_SIZE - i - 1)) & 1) as u8;
    }
    self.symbols(&mut syms);
    syms.iter().fold(0, |acc, &s| (acc << 1) | u32::from(s))
  }
}

/// 硬判决 Viterbi 译码器（fldigi `viterbi`，回溯 45、每次输出 1 位）。
///
/// 路径度量用 `i64`：fldigi 在 chunksize = 1 时永远走不到它的「度量归一化」分支
/// （`decode` 每次都在归一化之前就 `return traceback()`），`int` 要连续跑几百万步才会溢出；
/// 这里换成 64 位，比较结果与 fldigi 逐位相同，又不会在长录音里溢出。
#[derive(Debug, Clone)]
struct Viterbi {
  metrics: Vec<[i64; NSTATES]>,
  history: Vec<[usize; NSTATES]>,
  sequence: [usize; PATHMEM],
  ptr: usize,
}

impl Viterbi {
  fn new() -> Self {
    Self {
      metrics: vec![[0; NSTATES]; PATHMEM],
      history: vec![[0; NSTATES]; PATHMEM],
      sequence: [0; PATHMEM],
      ptr: 0,
    }
  }

  /// 软值 → 「这一位是 0 / 是 1」的度量（fldigi `mettab`）。
  fn met(bit: u32, sym: u8) -> i64 {
    if bit == 0 {
      128 - i64::from(sym)
    } else {
      i64::from(sym) - 128
    }
  }

  fn decode(&mut self, sym: [u8; 2]) -> u32 {
    let cur = self.ptr;
    let prev = (cur + PATHMEM - 1) % PATHMEM;
    let met: [i64; 4] = std::array::from_fn(|o| {
      let o = o as u32;
      Self::met((o >> 1) & 1, sym[1]) + Self::met(o & 1, sym[0])
    });
    for n in 0..NSTATES {
      let (s0, s1) = (n, n + NSTATES);
      let (p0, p1) = (s0 >> 1, s1 >> 1);
      let m0 = self.metrics[prev][p0] + met[conv_output(s0 as u32) as usize];
      let m1 = self.metrics[prev][p1] + met[conv_output(s1 as u32) as usize];
      // 平手取 p1：与 fldigi 的 `if (m0 > m1)` 同向。
      if m0 > m1 {
        self.metrics[cur][n] = m0;
        self.history[cur][n] = p0;
      } else {
        self.metrics[cur][n] = m1;
        self.history[cur][n] = p1;
      }
    }
    self.ptr = (self.ptr + 1) % PATHMEM;
    self.traceback()
  }

  fn traceback(&mut self) -> u32 {
    let mut p = (self.ptr + PATHMEM - 1) % PATHMEM;
    let mut best = (i64::MIN, 0usize);
    for (i, &m) in self.metrics[p].iter().enumerate() {
      // 严格大于：同分取下标最小的状态（与 fldigi 一致）。
      if m > best.0 {
        best = (m, i);
      }
    }
    self.sequence[p] = best.1;
    for _ in 0..TRACEBACK {
      let prev = (p + PATHMEM - 1) % PATHMEM;
      self.sequence[prev] = self.history[p][self.sequence[p]];
      p = prev;
    }
    // 状态的最低位就是那一步的输入位。
    (self.sequence[p] & 1) as u32
  }
}

/// 二次信道字符 → 借用的主信道码（fldigi `MuPskSec2Pri`，含它的重音字母折叠表）。
#[must_use]
pub fn sec_to_pri(c: u8) -> u8 {
  let mut c = c;
  if c.is_ascii_lowercase() {
    c -= 32;
  }
  c = fold_accent(c).unwrap_or(c);
  match c {
    b'A'..=b'Z' => c - b'A' + 127,
    b'0'..=b'9' => c - b'0' + 14,
    b' '..=b'"' => c - b' ' + 1,
    b'_' => 4,
    b'$'..=b'&' => c - b'$' + 5,
    b'\''..=b'*' => c - b'\'' + 9,
    b'+'..=b'/' => c - b'+' + 24,
    b':'..=b'<' => c - b':' + 29,
    b'='..=b'@' => c - b'=' + 153,
    b'['..=b']' => c - b'[' + 157,
    _ => b'_',
  }
}

/// fldigi `MuPsk_sec2pri_init` 的折叠表（Latin-1 → 基本字母 / 符号）。
///
/// 0xA1（¡）在原表里出现两次（`I` 组与 `!` 组），按原实现的插入顺序后者生效 → `!`。
fn fold_accent(c: u8) -> Option<u8> {
  Some(match c {
    0xC0..=0xC5 | 0xE0..=0xE5 => b'A',
    0xDF => b'B',
    0xC7 | 0xE7 | 0xA9 => b'C',
    0xD0 | 0xB0 => b'D',
    0xC6 | 0xE6 | 0xC8..=0xCB | 0xE8..=0xEB => b'E',
    0xCC..=0xCF | 0xEC..=0xEF => b'I',
    0xA3 => b'L',
    0xD1 | 0xF1 => b'N',
    0xF2..=0xF6 | 0xD2..=0xD6 => b'O',
    0xAE => b'R',
    0xD9..=0xDC | 0xF9..=0xFC => b'U',
    0xD7 => b'X',
    0xFF | 0xFD | 0xDD => b'Y',
    0xD8 => b'0',
    0xB9 => b'1',
    0xB2 => b'2',
    0xB3 => b'3',
    0xBF => b'?',
    0xA1 => b'!',
    0xAB => b'<',
    0xBB => b'>',
    b'(' => b'{',
    b')' => b'}',
    b'\\' => b'|',
    _ => return None,
  })
}

/// 收到的主信道码 → 字符；借给二次信道的码返回 `0x100 | 字符`（fldigi `MuPskPriSecChar`）。
#[must_use]
pub fn pri_to_char(c: u16) -> u16 {
  let base = |from: u16, to: u8| 0x100 | (c - from + u16::from(to));
  match c {
    127..=152 => base(127, b'A'),
    14..=23 => base(14, b'0'),
    1..=3 => base(1, b' '),
    4 => 0x100 | u16::from(b'_'),
    5..=7 => base(5, b'$'),
    9..=12 => base(9, b'\''),
    24..=28 => base(24, b'+'),
    29..=31 => base(29, b':'),
    153..=156 => base(153, b'='),
    157..=159 => base(157, b'['),
    _ => c,
  }
}

/// 码字（'1' / '0' 串）→ 整数，用于接收端反查。
///
/// 反查表只建一次（`OnceLock`）：接收热路径上每个候选都要反查，原来每次都线性扫
/// 256 个码字并重新 fold，解码器实例之间还各自重复。
fn varidec(code: u32) -> Option<u16> {
  static MAP: std::sync::OnceLock<std::collections::HashMap<u32, u16>> = std::sync::OnceLock::new();
  let map = MAP.get_or_init(|| {
    let mut m = std::collections::HashMap::with_capacity(VARICODE.len());
    for (i, s) in VARICODE.iter().enumerate() {
      let key = s
        .bytes()
        .fold(0u32, |acc, b| (acc << 1) | u32::from(b == b'1'));
      // 保留第一个出现的位置（与原来 `position` 的语义一致，尽管码字互不重复）。
      m.entry(key).or_insert(i as u16);
    }
    m
  });
  map.get(&code).copied()
}

/// FEC 发送端：字符 → IFK+ nibble。
#[derive(Debug, Clone)]
pub struct FecEncoder {
  enc: ConvEncoder,
  inlv: Interleaver,
  bitshreg: u32,
  bitstate: u32,
}

impl Default for FecEncoder {
  fn default() -> Self {
    Self::new()
  }
}

impl FecEncoder {
  #[must_use]
  pub fn new() -> Self {
    Self {
      enc: ConvEncoder::default(),
      inlv: Interleaver::new(true),
      bitshreg: 0,
      bitstate: 0,
    }
  }

  /// 发送前清交织器（fldigi `MuPskClearbits`）：把 100 对码位推进交织器但**不发符号**。
  pub fn clear_bits(&mut self) {
    let data = self.enc.encode(false);
    for _ in 0..100 {
      for i in 0..2 {
        self.bitshreg = (self.bitshreg << 1) | ((data >> i) & 1);
        self.bitstate += 1;
        if self.bitstate == 4 {
          self.inlv.bits(self.bitshreg);
          self.bitstate = 0;
          self.bitshreg = 0;
        }
      }
    }
  }

  /// 编码一个字符，把产生的 nibble 追加到 `out`（fldigi `sendMuPskEX`）。
  ///
  /// 主信道：LF 不发；控制符与 0x7F–0x9F 换成 `_`（这些码字借给了二次信道）。
  pub fn encode_char(&mut self, c: u8, secondary: bool, out: &mut Vec<u8>) {
    let c = if secondary {
      sec_to_pri(c)
    } else {
      if c == b'\n' {
        return;
      }
      if matches!(c, 1..=7 | 9..=12 | 14..=31 | 127..=159) {
        b'_'
      } else {
        c
      }
    };
    for b in VARICODE[c as usize].bytes() {
      let data = self.enc.encode(b == b'1');
      for i in 0..2 {
        self.bitshreg = (self.bitshreg << 1) | ((data >> i) & 1);
        self.bitstate += 1;
        if self.bitstate == 4 {
          out.push(self.inlv.bits(self.bitshreg) as u8);
          self.bitstate = 0;
          self.bitshreg = 0;
        }
      }
    }
  }
}

/// FEC 接收端：IFK+ nibble → 字符（`0x100 | c` 为二次信道）。
#[derive(Debug, Clone)]
pub struct FecDecoder {
  inlv: Interleaver,
  vit: Viterbi,
  pair: [u8; 2],
  odd: bool,
  datashreg: u32,
}

impl Default for FecDecoder {
  fn default() -> Self {
    Self::new()
  }
}

impl FecDecoder {
  #[must_use]
  pub fn new() -> Self {
    Self {
      inlv: Interleaver::new(false),
      vit: Viterbi::new(),
      pair: [0; 2],
      odd: false,
      datashreg: 1,
    }
  }

  /// 喂入一个 IFK+ nibble（0–17；与 fldigi 一样只看低 4 位），返回这一步译出的字符。
  pub fn feed(&mut self, nibble: u8, out: &mut Vec<u16>) {
    let mut syms = [0u8; INLV_SIZE];
    for (i, s) in syms.iter_mut().enumerate() {
      *s = if (nibble >> (INLV_SIZE - 1 - i)) & 1 == 1 {
        255
      } else {
        1
      };
    }
    self.inlv.symbols(&mut syms);
    for s in syms {
      self.symbol(s, out);
    }
  }

  fn symbol(&mut self, s: u8, out: &mut Vec<u16>) {
    self.pair = [self.pair[1], s];
    self.odd = !self.odd;
    if self.odd {
      return;
    }
    let bit = self.vit.decode(self.pair);
    self.datashreg = (self.datashreg << 1) | bit;
    if self.datashreg & 7 == 1 {
      if let Some(c) = varidec(self.datashreg >> 1) {
        out.push(pri_to_char(c));
      }
      self.datashreg = 1;
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn interleavers_are_inverse_after_their_delay() {
    // 前向 + 反向串起来是一个纯延迟：送入一串随机 nibble，输出是同一串（错后若干个）。
    let mut tx = Interleaver::new(true);
    let mut rx = Interleaver::new(false);
    let mut seed = 0x1234_5678u32;
    let input: Vec<u32> = (0..200)
      .map(|_| {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
        (seed >> 16) & 0xF
      })
      .collect();
    let mut output = Vec::new();
    for &v in &input {
      let t = tx.bits(v);
      let mut syms = [0u8; 4];
      for (i, s) in syms.iter_mut().enumerate() {
        *s = ((t >> (3 - i)) & 1) as u8;
      }
      rx.symbols(&mut syms);
      output.push(syms);
    }
    let delay = (0..40)
      .find(|&d| {
        (0..100).all(|n| {
          let want = input[n];
          let got = output[n + d];
          (0..4).all(|i| u32::from(got[i]) == (want >> (3 - i)) & 1)
        })
      })
      .expect("应存在固定延迟");
    assert_eq!(delay, 12, "4 级 × 每级 3 个 nibble");
  }

  #[test]
  fn varicode_codes_are_self_delimiting() {
    // 字符边界靠 `001`：每个码字 1 开头、00 结尾、内部不含 `001`
    // （结尾可以是一串 0，如 `11101100000`，但 00 之后不能再出现 1）。
    for (i, c) in VARICODE.iter().enumerate() {
      assert!(c.starts_with('1') && c.ends_with("00"), "{i}: {c}");
      assert!(!c.contains("001"), "{i}: {c}");
      assert_eq!(varidec(u32::from_str_radix(c, 2).unwrap()), Some(i as u16));
    }
  }

  #[test]
  fn secondary_mapping_round_trips_through_the_borrowed_codes() {
    // 二次信道能表达的字符：发出去是借来的主码，收回来是 0x100 | 原字符（大写化后）。
    for c in (b' '..=b']').chain(b'a'..=b'z') {
      let pri = sec_to_pri(c);
      let back = pri_to_char(u16::from(pri));
      let want = if c.is_ascii_lowercase() { c - 32 } else { c };
      if pri == b'_' && want != b'_' {
        continue; // 这类字符二次信道表达不了（如 `#`），fldigi 也是换成 `_` 发
      }
      assert_eq!(back, 0x100 | u16::from(want), "{:?}", char::from(c));
    }
    // 重音字母折叠；0xA1 在原表里出现两次，后写的 `!` 生效。
    assert_eq!(
      pri_to_char(u16::from(sec_to_pri(0xE9))),
      0x100 | u16::from(b'E')
    );
    assert_eq!(
      pri_to_char(u16::from(sec_to_pri(0xA1))),
      0x100 | u16::from(b'!')
    );
  }

  /// 编码一段文本（前后补二次信道空格让 Viterbi 收敛 / 排空）。
  fn nibbles(text: &str) -> Vec<u8> {
    let mut enc = FecEncoder::new();
    enc.clear_bits();
    let mut out = Vec::new();
    for _ in 0..4 {
      enc.encode_char(b' ', true, &mut out);
    }
    for b in text.bytes() {
      enc.encode_char(b, false, &mut out);
    }
    for _ in 0..10 {
      enc.encode_char(b' ', true, &mut out);
    }
    out
  }

  fn primary(ns: &[u8]) -> String {
    let mut dec = FecDecoder::new();
    let mut chars = Vec::new();
    for &n in ns {
      dec.feed(n, &mut chars);
    }
    chars
      .iter()
      .filter(|c| **c < 0x100)
      .map(|&c| char::from(c as u8))
      .collect()
  }

  #[test]
  fn nibble_round_trip_and_error_correction() {
    let text = "CQ CQ DE BG1AAA 599 K";
    let clean = nibbles(text);
    assert!(clean.iter().all(|&n| n < 16));
    assert_eq!(primary(&clean), text);
    // 码率 1/2 的意义：每 12 个 nibble 打坏一个（翻掉一位），文本仍然逐字还原。
    let mut hurt = clean.clone();
    for i in (5..hurt.len()).step_by(12) {
      hurt[i] ^= 0b0100;
    }
    assert_eq!(primary(&hurt), text);
  }
}
