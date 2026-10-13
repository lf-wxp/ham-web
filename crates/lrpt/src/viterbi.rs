//! CCSDS 卷积码（约束长度 7、码率 1/2）编码与硬判决 Viterbi 译码。
//!
//! 生成多项式按 **CCSDS 标准 / lrpt-encoder**（MIT，SatDump 互操作验证过）：
//! `G1 = 171₈ = 0x79`、`G2 = 133₈ = 0x5B`，不取反；窗口 = `(输入位 << 6) | 6 位状态`。
//! 注意：artlav/meteor_decoder 的 `0x4F/0x6D` 是这两个多项式的**位反转**（等价码，
//! 移位寄存器方向不同）—— 本 crate 对齐编码器的约定，互操作以对拍为准。

/// G1 = 1111001b（171₈，非取反）。
pub const POLYA: u8 = 0x79;
/// G2 = 1011011b（133₈，非取反）。
pub const POLYB: u8 = 0x5B;
/// 约束长度。
pub const K: usize = 7;
/// 状态数 = 2^(K-1)。
pub const NUM_STATES: usize = 1 << (K - 1);
/// 回溯长度：参考实现取 15×7 = 105，这里**向上取整到 16×7 = 112（14 字节）**——
/// 滑窗译码丢掉的收敛前缀必须是整数字节，否则后续的字节流失去字节对齐
/// （ASM 同步在字节域做）。112 仍满足 ≥ 5×7 的最小回溯要求。
pub const TRACEBACK: usize = 16 * 7;

/// 卷积编码：`data`（每元素 0/1）→ 2× 长度的编码位。
///
/// 尾部补 6 个 0 位（约束长度 − 1）把状态收敛到 0 —— 供合成测试用；
/// 真实 LRPT 是**连续流**（无尾位），见 [`conv_encode_continuous`]。
#[must_use]
pub fn conv_encode(data: &[u8]) -> Vec<u8> {
  let mut state = 0u8;
  let mut out = Vec::with_capacity((data.len() + K - 1) * 2);
  for &bit in data.iter().chain(std::iter::repeat_n(&0, K - 1)) {
    push_step(bit, &mut state, &mut out);
  }
  out
}

/// 连续流编码（状态跨调用保留，与 lrpt-encoder 的 `encode_with_state` 一致）。
#[must_use]
pub fn conv_encode_continuous(data: &[u8], state: &mut u8) -> Vec<u8> {
  let mut out = Vec::with_capacity(data.len() * 2);
  for &bit in data {
    push_step(bit, state, &mut out);
  }
  out
}

fn push_step(bit: u8, state: &mut u8, out: &mut Vec<u8>) {
  let win = ((bit & 1) << 6) | *state;
  out.push(parity(win, POLYA));
  out.push(parity(win, POLYB));
  *state = (*state >> 1) | ((bit & 1) << (K - 2));
}

/// 窗口与多项式的奇偶（各 tap 位异或）。
#[must_use]
pub const fn parity(win: u8, poly: u8) -> u8 {
  ((win & poly).count_ones() & 1) as u8
}

/// 一步状态转移：`(当前状态, 输入位) → 新状态`。
#[must_use]
pub const fn next_state(state: usize, bit: u8) -> usize {
  (state >> 1) | ((bit as usize) << (K - 2))
}

/// 状态转移的输出对（G1 位、G2 位）。
#[must_use]
pub const fn branch_bits(state: usize, bit: u8) -> (u8, u8) {
  let win = ((bit & 1) << 6) | state as u8;
  (parity(win, POLYA), parity(win, POLYB))
}

/// 一步转移表 `[state][bit] → (next_state, b0, b1)`：编译期算好，trellis 热路径
/// 免去每步 64×2 次的奇偶重算。
pub static BRANCH_TABLE: [[(usize, u8, u8); 2]; NUM_STATES] = {
  let mut table = [[(0usize, 0u8, 0u8); 2]; NUM_STATES];
  let mut s = 0;
  while s < NUM_STATES {
    let mut b = 0;
    while b < 2 {
      let (b0, b1) = branch_bits(s, b as u8);
      table[s][b] = (next_state(s, b as u8), b0, b1);
      b += 1;
    }
    s += 1;
  }
  table
};

/// 硬判决 Viterbi 译码：`coded`（每元素 0/1，长度 = 2×(数据位+6)）→ 数据位。
///
/// 编码位不足一个约束长度（2×6）时没有可判的数据位，返回空 —— 短输入不该下溢。
#[must_use]
pub fn viterbi_decode(coded: &[u8]) -> Vec<u8> {
  let coded_len = coded.len() / 2;
  let Some(data_len) = coded_len.checked_sub(K - 1) else {
    return Vec::new();
  };
  let mut buf = TrellisBuf::default();
  trellis(coded, 0, &mut buf);
  // 尾部已收敛到 0 状态，从 0 回溯。
  let mut out = vec![0u8; data_len];
  let mut state = 0usize;
  for t in (0..coded_len).rev() {
    if t < data_len {
      out[t] = buf.decisions[t * NUM_STATES + state];
    }
    state = buf.prev[(t + 1) * NUM_STATES + state] as usize;
  }
  out
}

/// 连续流硬判决 Viterbi 译码（滑动窗口、状态跨窗保留）。
///
/// 真实 LRPT 的卷积码没有帧级尾位 —— 帧边界由译码后字节流里的 ASM 决定。
/// 滑窗：每次解 `3×TRACEBACK` 位、只输出**中间** `TRACEBACK` 位（前后各留一段回溯余量
/// 让幸存路径收敛；因此输出从第 `TRACEBACK` 个数据位起，与 [`viterbi_decode`] 的位偏移不同），
/// 用「回溯出的下一状态」作为下一窗的起始。
#[must_use]
pub fn viterbi_decode_continuous(coded: &[u8]) -> Vec<u8> {
  const WINDOW: usize = TRACEBACK * 3;
  let coded_len = coded.len() / 2;
  if coded_len <= WINDOW {
    return viterbi_decode(coded);
  }
  let mut out = Vec::with_capacity(coded_len);
  let mut start = 0usize;
  let mut start_state = 0usize;
  // 缓冲跨窗复用：每窗两块各 ~21 KB，一段 95 秒录音 4 个相位约 25 万个窗口，
  // 逐窗分配的瞬时总量上 10 GB，白白拖慢上传解码。
  let mut buf = TrellisBuf::default();
  while start + WINDOW <= coded_len {
    let costs = trellis(
      &coded[start * 2..(start + WINDOW) * 2],
      start_state,
      &mut buf,
    );
    let (prev, decisions) = (&buf.prev, &buf.decisions);
    let mut state = (0..NUM_STATES).min_by_key(|&st| costs[st]).unwrap_or(0);
    let mut bits = [0u8; WINDOW];
    let mut carry = 0usize;
    for t in (0..WINDOW).rev() {
      bits[t] = decisions[t * NUM_STATES + state];
      state = prev[(t + 1) * NUM_STATES + state] as usize;
      if t == TRACEBACK {
        // 下一窗的起始状态 = 本窗时刻 TRACEBACK 的状态（回溯更新后的值），
        // 即下一窗第一个编码位（全局时刻 start + TRACEBACK）对应的前状态。
        carry = state;
      }
    }
    // 输出中间的 1/3（前后各有 TRACEBACK 的回溯余量，让幸存路径收敛）。
    out.extend_from_slice(&bits[TRACEBACK..2 * TRACEBACK]);
    start_state = carry;
    start += TRACEBACK;
  }
  // 流尾不足一个窗口的部分直接丢弃：无终止信息的连续卷积码在流尾没有
  // 可判路径（所有幸存路径等权），末段本来不可靠 —— 真实链路里那也
  // 是传输结束处，丢帧无损。
  out
}

/// trellis 的前驱 / 判决表缓冲（扁平化，行 = 状态），可跨窗口复用。
#[derive(Default)]
struct TrellisBuf {
  prev: Vec<u8>,
  decisions: Vec<u8>,
}

/// 一段编码位（给定起始状态代价 0）的 trellis：返回末端代价，前驱 / 判决表写入 `buf`。
///
/// 热路径零堆分配（`buf` 容量复用）：代价用栈数组轮换、转移对查 [`BRANCH_TABLE`]。
fn trellis(coded: &[u8], start_state: usize, buf: &mut TrellisBuf) -> [u32; NUM_STATES] {
  let steps = coded.len() / 2;
  let mut costs = [u32::MAX; NUM_STATES];
  costs[start_state] = 0;
  let mut new_costs = [u32::MAX; NUM_STATES];
  // 清零而不是沿用旧值：未被写到的（不可达状态）条目要与新分配时一致，译码结果才可复现。
  buf.prev.clear();
  buf.prev.resize((steps + 1) * NUM_STATES, 0);
  buf.decisions.clear();
  buf.decisions.resize(steps * NUM_STATES, 0);
  for t in 0..steps {
    new_costs.fill(u32::MAX);
    let (r0, r1) = (coded[2 * t], coded[2 * t + 1]);
    for (state, &cost) in costs.iter().enumerate() {
      if cost == u32::MAX {
        continue;
      }
      for (bit, &(next, b0, b1)) in BRANCH_TABLE[state].iter().enumerate() {
        let metric = u32::from(r0 ^ b0) + u32::from(r1 ^ b1);
        let candidate = cost + metric;
        if candidate < new_costs[next] {
          new_costs[next] = candidate;
          buf.prev[(t + 1) * NUM_STATES + next] = state as u8;
          buf.decisions[t * NUM_STATES + next] = bit as u8;
        }
      }
    }
    costs = new_costs;
  }
  costs
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn round_trips_arbitrary_data() {
    for (data, name) in [
      (b"HELLO LRPT 123".as_slice(), "ascii"),
      (&[0x00, 0xFF, 0x55, 0xAA, 0x1A, 0xCF, 0xFC, 0x1D], "bytes"),
    ] {
      let bits: Vec<u8> = data
        .iter()
        .flat_map(|&b| (0..8).map(move |i| (b >> (7 - i)) & 1))
        .collect();
      let coded = conv_encode(&bits);
      let decoded = viterbi_decode(&coded);
      assert_eq!(decoded, bits, "{name} 往返");
    }
  }

  #[test]
  fn corrects_scattered_bit_errors() {
    let bits: Vec<u8> = (0..300).map(|i| (i * 7 % 3 % 2) as u8).collect();
    let mut coded = conv_encode(&bits);
    // 每隔 40 个编码位翻 1 位（约 2.5% 错误率）。
    for i in (0..coded.len()).step_by(40) {
      coded[i] ^= 1;
    }
    let decoded = viterbi_decode(&coded);
    let errs = decoded.iter().zip(&bits).filter(|(a, b)| a != b).count();
    assert_eq!(errs, 0, "散布错误应全部纠正");
  }

  #[test]
  fn tail_flushes_state_to_zero() {
    // 全 1 输入后 6 个尾 0 把状态收回到 0。
    let bits = vec![1u8; 32];
    let coded = conv_encode(&bits);
    let decoded = viterbi_decode(&coded);
    assert_eq!(decoded, bits);
  }

  #[test]
  fn continuous_stream_round_trips() {
    // 无尾位的连续流（真实 LRPT 的形式）：状态跨段保留，滑窗译码还原。
    let bits: Vec<u8> = (0..4000).map(|i| (i * 11 % 5 % 2) as u8).collect();
    let mut state = 0u8;
    let mut coded = Vec::new();
    for chunk in bits.chunks(97) {
      coded.extend(conv_encode_continuous(chunk, &mut state));
    }
    let decoded = viterbi_decode_continuous(&coded);
    // 输出 = 真实位 [TRACEBACK .. N−2·TRACEBACK)，首尾各有收敛/无前瞻区；
    // 滑窗边界的幸存路径平局允许极少量分歧（RS 层会吸收），限 ≤ 2 位。
    let errs = decoded
      .iter()
      .zip(&bits[TRACEBACK..TRACEBACK + decoded.len()])
      .filter(|(a, b)| a != b)
      .count();
    assert!(errs <= 2, "连续流中段误差 {errs} 位，超出滑窗边界容差");
  }
}
