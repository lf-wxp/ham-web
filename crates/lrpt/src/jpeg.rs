//! LRPT MSU-MR 的 JPEG 子集解码（14 个 MCU 一段的「段」为单位）。
//!
//! 编码侧的转写源：lrpt-encoder（MIT）`compression.rs` —— 8×8 DCT（Type-II，
//! 0.25·cu·cv 归一化）、亮度量化表按质量因子缩放、**LRPT 专用 ZIGZAG 序**（不是
//! 标准 JPEG 之字形！）、DC 差分 + AC 游程的 Huffman 编码（亮度表）。
//!
//! 本模块是纯解码侧：BitReader（MSB 先）→ Huffman 表查码 → 反量化 → IDCT →
//! 电平还原（+128，裁剪 0..255）。

use crate::huffman_tables::{AC_HUFFMAN, DC_HUFFMAN};

/// 段内 MCU 数（每段 14 个 8×8 块 = 112 像素宽）。
pub const MCUS_PER_SEGMENT: usize = 14;
pub const MCU: usize = 8;

/// 标准 JPEG 亮度量化表（转写自 lrpt-encoder `constants.rs`）。
pub const LUMA_QT: [u8; 64] = [
  16, 11, 10, 16, 24, 40, 51, 61, 12, 12, 14, 19, 26, 58, 60, 55, 14, 13, 16, 24, 40, 57, 69, 56,
  14, 17, 22, 29, 51, 87, 80, 62, 18, 22, 37, 56, 68, 109, 103, 77, 24, 35, 55, 64, 81, 104, 113,
  92, 49, 64, 78, 87, 103, 121, 120, 101, 72, 92, 95, 98, 112, 100, 103, 99,
];

/// LRPT 熵编码序（自然索引 → 熵序索引），转写自 lrpt-encoder `constants.rs`。
pub const ZIGZAG: [usize; 64] = [
  0, 1, 5, 6, 14, 15, 27, 28, 2, 4, 7, 13, 16, 26, 29, 42, 3, 8, 12, 17, 25, 30, 41, 43, 9, 11, 18,
  24, 31, 40, 44, 53, 10, 19, 23, 32, 39, 45, 52, 54, 20, 22, 33, 38, 46, 51, 55, 60, 21, 34, 37,
  47, 50, 56, 59, 61, 35, 36, 48, 49, 57, 58, 62, 63,
];

/// 质量因子 → 量化表（与参照实现 `build_quant_table` 一致）。
#[must_use]
pub fn quant_table(quality: u8) -> [u16; 64] {
  let quality = quality.clamp(1, 100) as u32;
  let scale = if quality < 50 {
    5000 / quality
  } else {
    200 - 2 * quality
  };
  let mut qt = [0u16; 64];
  for (i, q) in qt.iter_mut().enumerate() {
    *q = ((u32::from(LUMA_QT[i]) * scale + 50) / 100).clamp(1, 255) as u16;
  }
  qt
}

struct BitReader<'a> {
  data: &'a [u8],
  pos: usize, // 已消费的位
}

impl<'a> BitReader<'a> {
  fn new(data: &'a [u8]) -> Self {
    BitReader { data, pos: 0 }
  }

  /// 读 `count` 位（MSB 先），`None` = 越界。
  fn read(&mut self, count: u8) -> Option<u32> {
    if self.pos + usize::from(count) > self.data.len() * 8 {
      return None;
    }
    let mut v = 0u32;
    for _ in 0..count {
      let byte = self.data[self.pos / 8];
      let bit = (byte >> (7 - self.pos % 8)) & 1;
      v = (v << 1) | u32::from(bit);
      self.pos += 1;
    }
    Some(v)
  }

  /// 前缀码查表：逐位累积码字，在 `(码长, 码字) → 符号` 索引里 O(1) 命中。
  fn read_huffman(&mut self, index: &HuffIndex) -> Option<u8> {
    let mut acc = 0u32;
    for len in 1..=16u32 {
      let bit = self.read(1)?;
      acc = (acc << 1) | bit;
      if let Some(&sym) = index.get(&(len << 16 | acc)) {
        return Some(sym);
      }
    }
    None
  }
}

/// `(码长 << 16 | 码字) → 符号` 索引。原先每读一位都线性扫全表（AC 表 162 项），
/// 3400 万次符号读取的热路径上这是主要开销。
type HuffIndex = std::collections::HashMap<u32, u8>;

fn build_index(table: &[(u8, u32, u8)]) -> HuffIndex {
  table
    .iter()
    .map(|&(sym, code, len)| (u32::from(len) << 16 | code, sym))
    .collect()
}

fn dc_index() -> &'static HuffIndex {
  static INDEX: std::sync::OnceLock<HuffIndex> = std::sync::OnceLock::new();
  INDEX.get_or_init(|| build_index(DC_HUFFMAN))
}

fn ac_index() -> &'static HuffIndex {
  static INDEX: std::sync::OnceLock<HuffIndex> = std::sync::OnceLock::new();
  INDEX.get_or_init(|| build_index(AC_HUFFMAN))
}

/// Huffman 码的幅值恢复：`(类别, 幅值位)` → 有符号值（一补码约定）。
#[must_use]
pub fn extend(bits: u32, cat: u8) -> i16 {
  if cat == 0 {
    return 0;
  }
  let max = (1i32 << cat) - 1;
  let v = bits as i32;
  if v >= (1 << (cat - 1)) {
    v as i16
  } else {
    (v - max) as i16
  }
}

/// 解码一个「段」（14 个 MCU 的独立 Huffman 位流，DC 预测在段内差分）。
///
/// 返回 14 个 8×8 像素块（已做电平还原、裁剪）。
#[must_use]
pub fn decode_segment(data: &[u8], quality: u8) -> Option<Vec<[[u8; MCU]; MCU]>> {
  let qt = quant_table(quality);
  let mut reader = BitReader::new(data);
  let mut prev_dc = 0i16;
  let mut out = Vec::with_capacity(MCUS_PER_SEGMENT);
  for _ in 0..MCUS_PER_SEGMENT {
    // DC
    let dc_sym = reader.read_huffman(dc_index())?;
    let dc_cat = dc_sym;
    if dc_cat > 11 {
      return None;
    }
    let dc_bits = if dc_cat > 0 { reader.read(dc_cat)? } else { 0 };
    let dc_val = extend(dc_bits, dc_cat);
    prev_dc = prev_dc.wrapping_add(dc_val);
    let mut coeffs = [0i16; 64];
    coeffs[0] = prev_dc;

    // AC
    let mut idx = 1usize;
    while idx < 64 {
      let sym = reader.read_huffman(ac_index())?;
      let run = (sym >> 4) & 0x0F;
      let cat = sym & 0x0F;
      if cat == 0 {
        if run == 0 {
          break; // EOB
        }
        // ZRL（LRPT 表 0xF0）：表示 16 个连续零系数 —— 与编码端
        // （本文件测试用编码器 `run -= 16`）一致，而不是 run 字段 × 16。
        idx += 16;
        if idx > 64 {
          return None; // ZRL 越过块边界：非法流
        }
        continue;
      }
      if cat > 10 {
        return None;
      }
      idx += usize::from(run);
      if idx >= 64 {
        return None;
      }
      let bits = reader.read(cat)?;
      coeffs[idx] = extend(bits, cat);
      idx += 1;
    }

    out.push(idct_dequantize(&coeffs, &qt));
  }
  Some(out)
}

/// `cos((2x+1)·u·π/16)` 的 8×8 查表：IDCT 热路径每块省约 8192 次 f64 `cos`
/// （wasm 无原生 cos 指令，全图约 3.8 万个块）。值与现算表达式完全一致；
/// `f64::cos` 尚非 const fn，用 OnceLock 惰性初始化一次。
static COS_TABLE: std::sync::OnceLock<[[f64; 8]; 8]> = std::sync::OnceLock::new();

/// 惰性初始化并返回 cos 查表（模块内首次使用时算一次）。
fn cos_table() -> &'static [[f64; 8]; 8] {
  COS_TABLE.get_or_init(|| {
    let mut t = [[0.0f64; 8]; 8];
    for (x, row) in t.iter_mut().enumerate() {
      for (u, slot) in row.iter_mut().enumerate() {
        *slot = ((2 * x + 1) as f64 * u as f64 * std::f64::consts::PI / 16.0).cos();
      }
    }
    t
  })
}

/// 反量化 + IDCT + 电平还原。量化/熵序与参照实现逐点互逆。
fn idct_dequantize(coeffs: &[i16; 64], qt: &[u16; 64]) -> [[u8; MCU]; MCU] {
  // 反量化：自然索引 n 的 DCT 系数 = coeff[ZIGZAG[n]] × qt[n]。
  let mut dct = [[0.0f64; MCU]; MCU];
  for n in 0..64 {
    let c = f64::from(coeffs[ZIGZAG[n]]) * f64::from(qt[n]);
    dct[n / MCU][n % MCU] = c;
  }
  // IDCT：与参照实现的 forward_dct 互为逆变换；cos 因子查表。
  let ct = cos_table();
  let mut block = [[0i16; MCU]; MCU];
  for (y, brow) in block.iter_mut().enumerate() {
    for (x, slot) in brow.iter_mut().enumerate() {
      let mut sum = 0.0f64;
      for (v, drow) in dct.iter().enumerate() {
        let cv = if v == 0 {
          std::f64::consts::FRAC_1_SQRT_2
        } else {
          1.0
        };
        for (u, &c) in drow.iter().enumerate() {
          let cu = if u == 0 {
            std::f64::consts::FRAC_1_SQRT_2
          } else {
            1.0
          };
          sum += cu * cv * c * ct[x][u] * ct[y][v];
        }
      }
      let p = (0.25 * sum).round() as i32 + 128;
      *slot = p.clamp(0, 255) as i16;
    }
  }
  let mut out = [[0u8; MCU]; MCU];
  for (y, row) in block.iter().enumerate() {
    for (x, &v) in row.iter().enumerate() {
      out[y][x] = v as u8;
    }
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Huffman 表（稀疏的 (符号, 码, 长) 列表）按**符号**查码 —— 表序不等同于
  /// 符号下标，不能直接 `TABLE[sym]` 索引。
  fn code_of_opt(table: &[(u8, u32, u8)], sym: u8) -> Option<(u32, u8)> {
    table
      .iter()
      .find(|(s, _, _)| *s == sym)
      .map(|(_, c, l)| (*c, *l))
  }

  /// 同上，符号必须在表内。
  fn code_of(table: &[(u8, u32, u8)], sym: u8) -> (u32, u8) {
    code_of_opt(table, sym).expect("符号应在 Huffman 表内")
  }

  /// 测试用：编码一个段（镜像参照实现的编码链，仅测试对拍用）。
  fn encode_segment(blocks: &[[[u8; MCU]; MCU]], quality: u8) -> Vec<u8> {
    let qt = quant_table(quality);
    let mut writer = BitWriter::new();
    let mut prev_dc = 0i16;
    for block in blocks {
      // 电平平移 + 前向 DCT（与参照实现 forward_dct 同式）。
      let mut shifted = [[0i16; MCU]; MCU];
      for (i, row) in shifted.iter_mut().enumerate() {
        for (j, s) in row.iter_mut().enumerate() {
          *s = i16::from(block[i][j]) - 128;
        }
      }
      let mut dct = [[0.0f64; MCU]; MCU];
      for (v, drow) in dct.iter_mut().enumerate() {
        for (u, slot) in drow.iter_mut().enumerate() {
          let cu = if u == 0 {
            std::f64::consts::FRAC_1_SQRT_2
          } else {
            1.0
          };
          let cv = if v == 0 {
            std::f64::consts::FRAC_1_SQRT_2
          } else {
            1.0
          };
          let mut sum = 0.0f64;
          for (y, srow) in shifted.iter().enumerate() {
            for (x, &s) in srow.iter().enumerate() {
              let cuv = (2 * x + 1) as f64 * u as f64 * std::f64::consts::PI / 16.0;
              let cvv = (2 * y + 1) as f64 * v as f64 * std::f64::consts::PI / 16.0;
              sum += f64::from(s) * cuv.cos() * cvv.cos();
            }
          }
          *slot = 0.25 * cu * cv * sum;
        }
      }
      // 量化 + LRPT 熵序
      let mut coeffs = [0i16; 64];
      for n in 0..64 {
        coeffs[ZIGZAG[n]] = (dct[n / MCU][n % MCU] / f64::from(qt[n])).round() as i16;
      }
      // DC 差分 + Huffman
      let dc_diff = coeffs[0] - prev_dc;
      prev_dc = coeffs[0];
      let (cat, bits) = category(dc_diff);
      let (_, code, len) = DC_HUFFMAN[usize::from(cat)];
      writer.write(code, len);
      if cat > 0 {
        writer.write(bits, cat);
      }
      // AC
      let mut run = 0u8;
      for &c in coeffs.iter().skip(1) {
        if c == 0 {
          run += 1;
        } else {
          while run >= 16 {
            let (zrl, l) = code_of(AC_HUFFMAN, 0xF0);
            writer.write(zrl, l);
            run -= 16;
          }
          let (cat2, bits2) = category(c);
          let sym = (run << 4) | cat2;
          let Some((code2, len2)) = code_of_opt(AC_HUFFMAN, sym) else {
            // 参照实现：表外 (run, cat) 组合直接输出 EOB 截断（_ 分支）。
            let (eob, l) = code_of(AC_HUFFMAN, 0x00);
            writer.write(eob, l);
            break;
          };
          writer.write(code2, len2);
          writer.write(bits2, cat2);
          run = 0;
        }
      }
      if run > 0 {
        let (eob, l) = code_of(AC_HUFFMAN, 0x00);
        writer.write(eob, l);
      }
    }
    writer.flush()
  }

  fn category(v: i16) -> (u8, u32) {
    if v == 0 {
      return (0, 0);
    }
    let abs = v.unsigned_abs();
    let cat = 16 - abs.leading_zeros() as u8;
    let bits = if v > 0 {
      v as u32
    } else {
      (v + (1 << cat) - 1) as u32
    };
    (cat, bits)
  }

  struct BitWriter {
    buf: Vec<u8>,
    cur: u8,
    pos: u8,
  }

  impl BitWriter {
    fn new() -> Self {
      BitWriter {
        buf: Vec::new(),
        cur: 0,
        pos: 0,
      }
    }
    fn write(&mut self, value: u32, count: u8) {
      for i in (0..count).rev() {
        let bit = ((value >> i) & 1) as u8;
        self.cur = (self.cur << 1) | bit;
        self.pos += 1;
        if self.pos == 8 {
          self.buf.push(self.cur);
          self.cur = 0;
          self.pos = 0;
        }
      }
    }
    fn flush(mut self) -> Vec<u8> {
      if self.pos > 0 {
        self.cur <<= 8 - self.pos;
        self.buf.push(self.cur);
      }
      self.buf
    }
  }

  #[test]
  fn flat_blocks_round_trip_exactly() {
    let blocks: Vec<[[u8; MCU]; MCU]> = (0..MCUS_PER_SEGMENT)
      .map(|i| [[(i * 17 + 40) as u8; MCU]; MCU])
      .collect();
    let data = encode_segment(&blocks, 80);
    let decoded = decode_segment(&data, 80).expect("解码");
    for (d, b) in decoded.iter().zip(&blocks) {
      assert_eq!(d, b, "纯色块应逐字节还原");
    }
  }

  #[test]
  fn gradient_blocks_round_trip() {
    // 平缓梯度（DCT 系数小、类别 ≤ 10）：参照编码器对类别超出 AC 表的系数
    // 直接输出 EOB（_ 分支）—— 陡峭梯度会被它截断，这是参照实现的既定行为，
    // 测试用编码器能力内的信号。
    let mut blocks = Vec::new();
    for i in 0..MCUS_PER_SEGMENT {
      let mut b = [[0u8; MCU]; MCU];
      for (y, row) in b.iter_mut().enumerate() {
        for (x, slot) in row.iter_mut().enumerate() {
          *slot = (64 + x + y + i) as u8;
        }
      }
      blocks.push(b);
    }
    let data = encode_segment(&blocks, 80);
    let decoded = decode_segment(&data, 80).expect("解码");
    for (d, b) in decoded.iter().zip(&blocks) {
      for (y, (drow, brow)) in d.iter().zip(b).enumerate() {
        for (x, (&dp, &bp)) in drow.iter().zip(brow).enumerate() {
          let diff = i16::from(dp) - i16::from(bp);
          assert!(diff.abs() <= 2, "({x},{y}) 差 {diff}（量化舍入）");
        }
      }
    }
  }

  #[test]
  fn quality_affects_quant_table() {
    let q80 = quant_table(80);
    let q50 = quant_table(50);
    assert!(q80[0] < q50[0], "质量越高量化步长越小");
    let q100 = quant_table(100);
    assert!(q100.iter().all(|&v| v == 1));
  }

  #[test]
  fn zrl_can_finish_a_block_without_consuming_the_next_block() {
    let mut writer = BitWriter::new();
    let (dc, dc_len) = code_of(DC_HUFFMAN, 0);
    let (run, run_len) = code_of(AC_HUFFMAN, 0xE1);
    let (zrl, zrl_len) = code_of(AC_HUFFMAN, 0xF0);
    let (eob, eob_len) = code_of(AC_HUFFMAN, 0);
    writer.write(dc, dc_len);
    writer.write(run, run_len);
    writer.write(1, 1);
    for _ in 0..3 {
      writer.write(zrl, zrl_len);
    }
    for _ in 1..MCUS_PER_SEGMENT {
      writer.write(dc, dc_len);
      writer.write(eob, eob_len);
    }
    let blocks = decode_segment(&writer.flush(), 80).expect("块尾 ZRL 合法");
    assert_eq!(blocks.len(), MCUS_PER_SEGMENT);
    assert!(blocks[1..].iter().flatten().flatten().all(|&p| p == 128));
    let mut invalid = BitWriter::new();
    invalid.write(dc, dc_len);
    for _ in 0..4 {
      invalid.write(zrl, zrl_len);
    }
    assert!(decode_segment(&invalid.flush(), 80).is_none());
  }

  #[test]
  fn zrl_runs_of_sixteen_zeros_decode_correctly() {
    // 手工构造位流：块 0 的 AC = 17 个零 + 1 个非零（cat 1、值 +1），触发
    // ZRL（16 连零）路径 —— 修复前 ZRL 按 run×16 跳 240 个系数，EOB 未被
    // 消费，后续块位流整体失同步。
    let mut writer = BitWriter::new();
    // 块 0：DC 差 0。
    let (dc0, dc0l) = code_of(DC_HUFFMAN, 0x00);
    writer.write(dc0, dc0l);
    let (zrl, zrll) = code_of(AC_HUFFMAN, 0xF0);
    writer.write(zrl, zrll); // ZRL：16 零
    let (r1c1, r1c1l) = code_of(AC_HUFFMAN, 0x11);
    writer.write(r1c1, r1c1l); // run=1、cat=1
    writer.write(0b1, 1); // 值 +1 → 第 18 个 AC 系数
    let (eob, eobl) = code_of(AC_HUFFMAN, 0x00);
    writer.write(eob, eobl); // EOB
    // 其余块：全零。
    for _ in 1..MCUS_PER_SEGMENT {
      writer.write(dc0, dc0l);
      writer.write(eob, eobl);
    }
    let decoded = decode_segment(&writer.flush(), 80).expect("解码");
    // 块 0 有非零 AC 系数 → 像素偏离电平；块 1 为纯电平 128。
    assert!(decoded[0].iter().flatten().any(|&p| p != 128));
    assert!(decoded[1].iter().flatten().all(|&p| p == 128));
    // 其余块与块 1 相同：证明块 0 的 ZRL + EOB 都被正确消费、位流未失同步。
    for b in &decoded[2..] {
      assert_eq!(b, &decoded[1]);
    }
  }
}
