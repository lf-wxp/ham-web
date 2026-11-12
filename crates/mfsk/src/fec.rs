//! 卷积码 / Viterbi / 交织器：MFSK 与 THOR 共用的 FEC 原语。
//!
//! 转写自 fldigi（GPL-3.0）`src/filters/viterbi.cxx`（编码器与 Viterbi 译码器）和
//! `src/mfsk/interleave.cxx`（交织器，fldigi 注明改编自 gMFSK，© Tomi Manninen OH2BNS）。
//! 参数含义与 fldigi 一致：码率 1/2，两个生成多项式各产生一个码位（`poly1` → 码字位 0，
//! `poly2` → 码字位 1）。
//!
//! 与 fldigi 的刻意差异只有一处，且不影响任何判决：度量用 `i32` 存整段路径记忆，每 65 536 步
//! 把全部度量减去同一个常数（[`REBASE_EVERY`]）。fldigi 在 `chunksize = 1` 时永远走不到它自己
//! 的归一化分支，`int` 要连续跑近千万步才会溢出；这里的平移保持所有差值不变，比较结果逐位相同。

/// 路径存储深度（fldigi `PATHMEM`）。
pub const PATHMEM: usize = 256;

/// 删除位：软值居中，两种假设的度量相同（fldigi `PUNCTURE`）。
pub const PUNCTURE: u8 = 128;

/// 每隔多少步把度量整体平移一次，防止 `i32` 溢出。
const REBASE_EVERY: u32 = 65_536;

fn parity(x: u32) -> u32 {
  x.count_ones() & 1
}

/// 约束长度 `k`、两个生成多项式的卷积码参数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Code {
  pub k: u32,
  pub poly1: u32,
  pub poly2: u32,
}

impl Code {
  /// NASA 标准码（K = 7，0x6D / 0x4F）：MFSK 全部模式与 THOR 慢速模式。
  pub const NASA_K7: Self = Self {
    k: 7,
    poly1: 0x6D,
    poly2: 0x4F,
  };

  /// IEEE 码（K = 15，八进制 044735 / 063057）：THOR 的高速模式（25x4 / 50x1 / 50x2 / 100）。
  pub const IEEE_K15: Self = Self {
    k: 15,
    poly1: 0o44735,
    poly2: 0o63057,
  };

  /// 移位寄存器内容 → 码字（`poly1` 码位 | `poly2` 码位 << 1）。
  fn output(self, state: u32) -> u8 {
    (parity(self.poly1 & state) | (parity(self.poly2 & state) << 1)) as u8
  }
}

/// 卷积编码器（fldigi `encoder`）。
#[derive(Debug, Clone)]
pub struct Encoder {
  code: Code,
  shreg: u32,
}

impl Encoder {
  #[must_use]
  pub fn new(code: Code) -> Self {
    Self { code, shreg: 0 }
  }

  /// 编码一个位，返回 2 位码字（位 0 先发）。
  pub fn encode(&mut self, bit: bool) -> u8 {
    self.shreg = (self.shreg << 1) | u32::from(bit);
    self.code.output(self.shreg & ((1 << self.code.k) - 1))
  }
}

/// 软判决 Viterbi 译码器（fldigi `viterbi`，`chunksize = 1`：每喂一对软值回溯一次、出 1 位）。
#[derive(Debug, Clone)]
pub struct Viterbi {
  nstates: usize,
  traceback: usize,
  /// 码字表：下标是 K 位移位寄存器内容。
  output: Vec<u8>,
  /// `metrics[step * nstates + state]`。
  metrics: Vec<i32>,
  /// `history[step * nstates + state]` = 该状态的幸存前驱。
  history: Vec<u16>,
  sequence: [usize; PATHMEM],
  ptr: usize,
  steps: u32,
}

impl Viterbi {
  /// `traceback` 为回溯深度（fldigi：K = 7 取 45，K = 15 取 180），须 `< PATHMEM`。
  ///
  /// 内存随约束长度指数增长：路径度量 + 幸存前驱共 `PATHMEM × 2^(K−1) × 6` 字节，K = 7 约 96 KB，
  /// K = 15 约 24 MB（登记在 `docs/domain/decode-worker-protocol.md`「质量门限与能力边界」）。
  ///
  /// # Panics
  /// `k` 不在 2..=15、或 `traceback` 不小于 [`PATHMEM`] 时。两个参数都只来自编译期常量
  /// （[`Code`] 的两个预设与各模式表里的回溯深度），不是用户输入，所以用 `assert!` 而不是返回 `Result`。
  #[must_use]
  pub fn new(code: Code, traceback: usize) -> Self {
    assert!((2..=15).contains(&code.k) && traceback < PATHMEM);
    let nstates = 1usize << (code.k - 1);
    Self {
      nstates,
      traceback,
      output: (0..1u32 << code.k).map(|s| code.output(s)).collect(),
      metrics: vec![0; PATHMEM * nstates],
      history: vec![0; PATHMEM * nstates],
      sequence: [0; PATHMEM],
      ptr: 0,
      steps: 0,
    }
  }

  /// 软值 → 「这一位是 0 / 是 1」的度量（fldigi `mettab`）。
  fn met(bit: u8, sym: u8) -> i32 {
    if bit == 0 {
      128 - i32::from(sym)
    } else {
      i32::from(sym) - 128
    }
  }

  /// 喂入一对软值（`sym[0]` 对应 `poly1`，`sym[1]` 对应 `poly2`），返回回溯出的位与该步的
  /// 路径度量增量（fldigi 的 `*metric`，MFSK 奇数位模式用它在两个译码器之间投票）。
  pub fn decode(&mut self, sym: [u8; 2]) -> (bool, i32) {
    let n = self.nstates;
    let cur = self.ptr;
    let prev = (cur + PATHMEM - 1) % PATHMEM;
    let met: [i32; 4] = std::array::from_fn(|o| {
      let o = o as u8;
      Self::met((o >> 1) & 1, sym[1]) + Self::met(o & 1, sym[0])
    });
    for state in 0..n {
      let (s0, s1) = (state, state + n);
      let (p0, p1) = (s0 >> 1, s1 >> 1);
      let m0 = self.metrics[prev * n + p0] + met[usize::from(self.output[s0])];
      let m1 = self.metrics[prev * n + p1] + met[usize::from(self.output[s1])];
      // 平手取 p1：与 fldigi 的 `if (m0 > m1)` 同向。
      if m0 > m1 {
        self.metrics[cur * n + state] = m0;
        self.history[cur * n + state] = p0 as u16;
      } else {
        self.metrics[cur * n + state] = m1;
        self.history[cur * n + state] = p1 as u16;
      }
    }
    self.ptr = (self.ptr + 1) % PATHMEM;
    self.steps = self.steps.wrapping_add(1);
    let out = self.traceback();
    if self.steps.is_multiple_of(REBASE_EVERY) {
      self.rebase(cur);
    }
    out
  }

  fn traceback(&mut self) -> (bool, i32) {
    let n = self.nstates;
    let mut p = (self.ptr + PATHMEM - 1) % PATHMEM;
    // 严格大于：同分取下标最小的状态（与 fldigi 一致）。
    let mut best = (i32::MIN, 0usize);
    for (i, &m) in self.metrics[p * n..(p + 1) * n].iter().enumerate() {
      if m > best.0 {
        best = (m, i);
      }
    }
    self.sequence[p] = best.1;
    for _ in 0..self.traceback {
      let prev = (p + PATHMEM - 1) % PATHMEM;
      self.sequence[prev] = usize::from(self.history[p * n + self.sequence[p]]);
      p = prev;
    }
    let start = self.metrics[p * n + self.sequence[p]];
    // 状态的最低位就是那一步的输入位。
    let bit = self.sequence[p] & 1 == 1;
    p = (p + 1) % PATHMEM;
    let end = self.metrics[p * n + self.sequence[p]];
    (bit, end - start)
  }

  /// 全部度量减去同一个常数（当前步的最优度量），差值不变。
  fn rebase(&mut self, cur: usize) {
    let n = self.nstates;
    let best = self.metrics[cur * n..(cur + 1) * n]
      .iter()
      .copied()
      .max()
      .unwrap_or(0);
    for m in &mut self.metrics {
      *m -= best;
    }
  }
}

/// 卷积交织器（fldigi `interleave`）：`depth` 级，每级一个 `size × size` 的移位方阵。
/// 前向取反对角线、反向取主对角线，两者串起来恰好把每个位置的延迟补齐。
#[derive(Debug, Clone)]
pub struct Interleaver {
  size: usize,
  depth: usize,
  forward: bool,
  table: Vec<u8>,
}

impl Interleaver {
  /// 反向（接收端）交织器的初值是删除位，前向的是 0（fldigi `interleave::flush`）。
  #[must_use]
  pub fn new(size: usize, depth: usize, forward: bool) -> Self {
    let fill = if forward { 0 } else { PUNCTURE };
    Self {
      size,
      depth,
      forward,
      table: vec![fill; size * size * depth],
    }
  }

  fn idx(&self, k: usize, i: usize, j: usize) -> usize {
    self.size * self.size * k + self.size * i + j
  }

  /// 过交织器（就地）。`syms.len()` 必须等于 `size`。
  pub fn symbols(&mut self, syms: &mut [u8]) {
    let size = self.size;
    debug_assert_eq!(syms.len(), size);
    for k in 0..self.depth {
      for i in 0..size {
        for j in 0..size - 1 {
          let (dst, src) = (self.idx(k, i, j), self.idx(k, i, j + 1));
          self.table[dst] = self.table[src];
        }
      }
      for (i, s) in syms.iter().enumerate() {
        let at = self.idx(k, i, size - 1);
        self.table[at] = *s;
      }
      for (i, s) in syms.iter_mut().enumerate() {
        let j = if self.forward { size - i - 1 } else { i };
        *s = self.table[self.idx(k, i, j)];
      }
    }
  }

  /// `size` 位整数（最高位先出）过交织器。
  pub fn bits(&mut self, value: u32) -> u32 {
    let mut syms = [0u8; 8];
    let syms = &mut syms[..self.size];
    for (i, s) in syms.iter_mut().enumerate() {
      *s = ((value >> (self.size - i - 1)) & 1) as u8;
    }
    self.symbols(syms);
    syms.iter().fold(0, |acc, &s| (acc << 1) | u32::from(s))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn encoder_matches_hand_computed_codewords() {
    // K=7、0x6D / 0x4F：单个 1 之后寄存器依次是 0000001、0000010 …，
    // 码字取两个多项式与寄存器的奇偶。
    let mut enc = Encoder::new(Code::NASA_K7);
    let out: Vec<u8> = [true, false, false, false, false, false, false, false]
      .into_iter()
      .map(|b| enc.encode(b))
      .collect();
    // 多项式 0x6D = 1101101、0x4F = 1001111：第 n 步的码位就是多项式的第 n 位（自低位起）。
    let want: Vec<u8> = (0..8)
      .map(|n| (((0x6D >> n) & 1) | (((0x4F >> n) & 1) << 1)) as u8)
      .collect();
    assert_eq!(out, want);
  }

  #[test]
  fn viterbi_recovers_bits_through_both_codes() {
    for (code, tb) in [(Code::NASA_K7, 45), (Code::IEEE_K15, 180)] {
      let mut seed = 0xC0FF_EE11u32;
      let data: Vec<bool> = (0..400)
        .map(|_| {
          seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
          (seed >> 16) & 1 == 1
        })
        .collect();
      let mut enc = Encoder::new(code);
      let mut dec = Viterbi::new(code, tb);
      let hard = |b: u8| if b == 1 { 255u8 } else { 1u8 };
      let mut out = Vec::new();
      // 末尾补零把回溯窗口里压着的位推出来。
      for &b in data.iter().chain(std::iter::repeat_n(&false, tb + 8)) {
        let w = enc.encode(b);
        out.push(dec.decode([hard(w & 1), hard((w >> 1) & 1)]).0);
      }
      // 译码输出比输入晚 `tb` 步：out[i + tb] 对应 data[i]（`+1`：状态最低位是上一步的输入）。
      let delay = (0..=tb + 2)
        .find(|&d| (0..data.len() - tb).all(|i| out[i + d] == data[i]))
        .unwrap_or_else(|| panic!("K={} 译码结果与输入不符", code.k));
      assert!(delay >= tb - 1, "K={} 延迟 {delay}", code.k);
    }
  }

  #[test]
  fn viterbi_corrects_scattered_errors() {
    let code = Code::NASA_K7;
    let data: Vec<bool> = (0..300).map(|i| (i * 7 + i / 5) % 3 == 0).collect();
    let mut enc = Encoder::new(code);
    let mut dec = Viterbi::new(code, 45);
    let mut out = Vec::new();
    for (n, &b) in data
      .iter()
      .chain(std::iter::repeat_n(&false, 60))
      .enumerate()
    {
      let mut w = enc.encode(b);
      if n % 13 == 5 {
        w ^= 1; // 每 13 个码字打坏一位
      }
      let s = |x: u8| if x == 1 { 255u8 } else { 1u8 };
      out.push(dec.decode([s(w & 1), s((w >> 1) & 1)]).0);
    }
    let delay = (40..=50)
      .find(|&d| (0..250).all(|i| out[i + d] == data[i]))
      .expect("卷积码应纠回稀疏错误");
    assert!(delay >= 44);
  }

  #[test]
  fn interleavers_are_inverse_after_their_delay() {
    // 前向 + 反向串起来是一个纯延迟：size=4、depth=10 → 每级 3 个符号 × 10 级 = 30。
    for (size, depth) in [(4usize, 4usize), (4, 10), (5, 5), (3, 10)] {
      let mut tx = Interleaver::new(size, depth, true);
      let mut rx = Interleaver::new(size, depth, false);
      let mut seed = 0x1234_5678u32;
      let input: Vec<u32> = (0..400)
        .map(|_| {
          seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
          (seed >> 16) & ((1 << size) - 1)
        })
        .collect();
      let mut output = Vec::new();
      for &v in &input {
        let t = tx.bits(v);
        let mut syms: Vec<u8> = (0..size)
          .map(|i| ((t >> (size - 1 - i)) & 1) as u8)
          .collect();
        rx.symbols(&mut syms);
        output.push(syms);
      }
      let delay = (0..120)
        .find(|&d| {
          (0..200).all(|n| {
            (0..size).all(|i| u32::from(output[n + d][i]) == (input[n] >> (size - 1 - i)) & 1)
          })
        })
        .unwrap_or_else(|| panic!("size={size} depth={depth} 应存在固定延迟"));
      assert_eq!(delay, (size - 1) * depth, "size={size} depth={depth}");
    }
  }

  #[test]
  fn rebase_does_not_change_decisions() {
    // 跑过一次平移（65 536 步）后，译码结果与不平移的参照逐位相同：
    // 同一段输入各喂一遍，第二个译码器在第 65 536 步前后不受影响。
    let code = Code::NASA_K7;
    let mut a = Viterbi::new(code, 45);
    let mut b = Viterbi::new(code, 45);
    let mut seed = 99u32;
    for n in 0..(REBASE_EVERY + 500) {
      seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
      let sym = [(seed >> 8) as u8, (seed >> 16) as u8];
      let ra = a.decode(sym);
      // b 用「刻意不平移」的路径：把 steps 钉在 1，永远走不到平移分支。
      b.steps = 1;
      let rb = b.decode(sym);
      assert_eq!(ra.0, rb.0, "第 {n} 步");
    }
  }
}
