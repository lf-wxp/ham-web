//! FT8 / FT4 的候选检测、似然提取与解码。
//!
//! 转写自 MIT 许可的 ft8_lib `ft8/decode.c`：从 waterfall 找 Costas 同步最强的候选，
//! 对每个候选提取 174 位对数似然比，归一化后跑 LDPC 软译码，再验 CRC。

use crate::constants::{
  FT4_COSTAS_PATTERN, FT4_GRAY_MAP, FT4_XOR_SEQUENCE, FT8_COSTAS_PATTERN, FT8_GRAY_MAP,
};
use crate::crc::{compute_crc, extract_crc};
use crate::ldpc::bp_decode;
use crate::message::Message;
use crate::monitor::{Protocol, Waterfall};

/// 一个候选（消息在时频图上的可能起点）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Candidate {
  pub score: i32,
  pub time_offset: i32,
  pub freq_offset: i32,
  pub time_sub: u8,
  pub freq_sub: u8,
}

/// 解码状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeStatus {
  pub ldpc_errors: usize,
  pub crc_extracted: u16,
  pub crc_calculated: u16,
}

fn max2(a: f32, b: f32) -> f32 {
  if a >= b { a } else { b }
}

fn max4(a: f32, b: f32, c: f32, d: f32) -> f32 {
  max2(max2(a, b), max2(c, d))
}

/// FT8 同步打分：3 组 Costas（各 7 符号），比较期望 bin 与频/时邻居的幅度差。
fn ft8_sync_score(wf: &Waterfall, cand: &Candidate) -> i32 {
  let mut score = 0i32;
  let mut num_average = 0i32;
  for m in 0..3usize {
    for (k, &costas) in FT8_COSTAS_PATTERN.iter().enumerate() {
      let block_abs = cand.time_offset + (36 * m + k) as i32;
      if block_abs < 0 {
        continue;
      }
      if block_abs >= wf.num_blocks as i32 {
        break;
      }
      let sm = costas as i32;
      let base = wf.int_at(
        block_abs as isize,
        cand.time_sub as usize,
        cand.freq_sub as usize,
        (cand.freq_offset + sm) as usize,
      );
      if sm > 0 {
        score += base
          - wf.int_at(
            block_abs as isize,
            cand.time_sub as usize,
            cand.freq_sub as usize,
            (cand.freq_offset + sm - 1) as usize,
          );
        num_average += 1;
      }
      if sm < 7 {
        score += base
          - wf.int_at(
            block_abs as isize,
            cand.time_sub as usize,
            cand.freq_sub as usize,
            (cand.freq_offset + sm + 1) as usize,
          );
        num_average += 1;
      }
      if k > 0 && block_abs > 0 {
        score += base
          - wf.int_at(
            block_abs as isize - 1,
            cand.time_sub as usize,
            cand.freq_sub as usize,
            (cand.freq_offset + sm) as usize,
          );
        num_average += 1;
      }
      if k + 1 < 7 && block_abs + 1 < wf.num_blocks as i32 {
        score += base
          - wf.int_at(
            block_abs as isize + 1,
            cand.time_sub as usize,
            cand.freq_sub as usize,
            (cand.freq_offset + sm) as usize,
          );
        num_average += 1;
      }
    }
  }
  if num_average > 0 {
    score /= num_average;
  }
  score
}

/// FT4 同步打分：4 组 Costas（各 4 符号）。
fn ft4_sync_score(wf: &Waterfall, cand: &Candidate) -> i32 {
  let mut score = 0i32;
  let mut num_average = 0i32;
  for (m, &costas_row) in FT4_COSTAS_PATTERN.iter().enumerate() {
    for (k, &costas) in costas_row.iter().enumerate() {
      let block_abs = cand.time_offset + (1 + 33 * m + k) as i32;
      if block_abs < 0 {
        continue;
      }
      if block_abs >= wf.num_blocks as i32 {
        break;
      }
      let sm = costas as i32;
      let base = wf.int_at(
        block_abs as isize,
        cand.time_sub as usize,
        cand.freq_sub as usize,
        (cand.freq_offset + sm) as usize,
      );
      if sm > 0 {
        score += base
          - wf.int_at(
            block_abs as isize,
            cand.time_sub as usize,
            cand.freq_sub as usize,
            (cand.freq_offset + sm - 1) as usize,
          );
        num_average += 1;
      }
      if sm < 3 {
        score += base
          - wf.int_at(
            block_abs as isize,
            cand.time_sub as usize,
            cand.freq_sub as usize,
            (cand.freq_offset + sm + 1) as usize,
          );
        num_average += 1;
      }
      if k > 0 && block_abs > 0 {
        score += base
          - wf.int_at(
            block_abs as isize - 1,
            cand.time_sub as usize,
            cand.freq_sub as usize,
            (cand.freq_offset + sm) as usize,
          );
        num_average += 1;
      }
      if k + 1 < 4 && block_abs + 1 < wf.num_blocks as i32 {
        score += base
          - wf.int_at(
            block_abs as isize + 1,
            cand.time_sub as usize,
            cand.freq_sub as usize,
            (cand.freq_offset + sm) as usize,
          );
        num_average += 1;
      }
    }
  }
  if num_average > 0 {
    score /= num_average;
  }
  score
}

/// 找出同步分数最高的若干候选（按分数降序）。
pub fn find_candidates(wf: &Waterfall, num_candidates: usize, min_score: i32) -> Vec<Candidate> {
  let num_tones = if wf.protocol == Protocol::Ft4 { 4 } else { 8 };
  let sync = if wf.protocol == Protocol::Ft4 {
    ft4_sync_score as fn(&Waterfall, &Candidate) -> i32
  } else {
    ft8_sync_score
  };

  let mut found: Vec<Candidate> = Vec::new();
  for time_sub in 0..wf.time_osr as u8 {
    for freq_sub in 0..wf.freq_osr as u8 {
      for time_offset in -10..20 {
        for freq_offset in 0..(wf.num_bins as i32 - num_tones + 1) {
          let cand = Candidate {
            score: 0,
            time_offset,
            freq_offset,
            time_sub,
            freq_sub,
          };
          let score = sync(wf, &cand);
          if score >= min_score {
            found.push(Candidate { score, ..cand });
          }
        }
      }
    }
  }
  found.sort_by_key(|c| std::cmp::Reverse(c.score));
  // 同一信号会被相邻的 `freq_sub` / `time_sub` 反复命中（差值都在 1 个格内），不去重的话
  // 高分信号会把候选名额吃满，弱信号一个名额都拿不到。这里按分数贪心取，相邻格视为重复。
  // 抑制半径是 1 个 bin（FT8 的 6.25 Hz / FT4 的 20.8 Hz），真实台站不会隔这么近。
  let mut selected: Vec<Candidate> = Vec::with_capacity(num_candidates);
  for cand in found {
    let duplicate = selected.iter().any(|s| {
      s.freq_sub == cand.freq_sub
        && s.time_sub == cand.time_sub
        && (s.freq_offset - cand.freq_offset).abs() <= 1
        && (s.time_offset - cand.time_offset).abs() <= 1
    });
    if duplicate {
      continue;
    }
    selected.push(cand);
    if selected.len() >= num_candidates {
      break;
    }
  }
  selected
}

/// 提取一个 FT8 符号（8 个 bin）的 3 比特对数似然比。
fn ft8_extract_symbol(bins: &[f32; 8]) -> [f32; 3] {
  let s = [
    FT8_GRAY_MAP[0],
    FT8_GRAY_MAP[1],
    FT8_GRAY_MAP[2],
    FT8_GRAY_MAP[3],
    FT8_GRAY_MAP[4],
    FT8_GRAY_MAP[5],
    FT8_GRAY_MAP[6],
    FT8_GRAY_MAP[7],
  ];
  let v = [
    bins[s[0] as usize],
    bins[s[1] as usize],
    bins[s[2] as usize],
    bins[s[3] as usize],
    bins[s[4] as usize],
    bins[s[5] as usize],
    bins[s[6] as usize],
    bins[s[7] as usize],
  ];
  [
    max4(v[4], v[5], v[6], v[7]) - max4(v[0], v[1], v[2], v[3]),
    max4(v[2], v[3], v[6], v[7]) - max4(v[0], v[1], v[4], v[5]),
    max4(v[1], v[3], v[5], v[7]) - max4(v[0], v[2], v[4], v[6]),
  ]
}

/// 提取一个 FT4 符号（4 个 bin）的 2 比特对数似然比。
fn ft4_extract_symbol(bins: &[f32; 4]) -> [f32; 2] {
  let s = [
    FT4_GRAY_MAP[0],
    FT4_GRAY_MAP[1],
    FT4_GRAY_MAP[2],
    FT4_GRAY_MAP[3],
  ];
  let v = [
    bins[s[0] as usize],
    bins[s[1] as usize],
    bins[s[2] as usize],
    bins[s[3] as usize],
  ];
  [
    max2(v[2], v[3]) - max2(v[0], v[1]),
    max2(v[1], v[3]) - max2(v[0], v[2]),
  ]
}

/// 提取 174 位对数似然比（跳过 Costas 同步符号）。
fn extract_likelihood(wf: &Waterfall, cand: &Candidate) -> [f32; 174] {
  let mut log = [0.0f32; 174];
  let (nd, n_bits) = if wf.protocol == Protocol::Ft4 {
    (87usize, 2usize)
  } else {
    (58usize, 3usize)
  };
  for k in 0..nd {
    // FT8: 数据符号 k 的绝对位置 = k + (k<29 ? 7 : 14)；FT4: = k + (k<29?5 : k<58?9 : 13)。
    let sym_idx = if wf.protocol == Protocol::Ft4 {
      k + if k < 29 {
        5
      } else if k < 58 {
        9
      } else {
        13
      }
    } else {
      k + if k < 29 { 7 } else { 14 }
    };
    let bit_idx = n_bits * k;
    let block = cand.time_offset + sym_idx as i32;
    if block < 0 || block >= wf.num_blocks as i32 {
      continue;
    }
    // 读出该符号的 num_tones 个 bin（从候选频率起）。
    let tones = if wf.protocol == Protocol::Ft4 {
      4usize
    } else {
      8usize
    };
    let mut bins = [0.0f32; 8];
    for (t, b) in bins.iter_mut().enumerate().take(tones) {
      *b = wf.db_at(
        block as isize,
        cand.time_sub as usize,
        cand.freq_sub as usize,
        (cand.freq_offset + t as i32) as usize,
      );
    }
    if wf.protocol == Protocol::Ft4 {
      let l = ft4_extract_symbol(&bins[..4].try_into().unwrap());
      log[bit_idx] = l[0];
      log[bit_idx + 1] = l[1];
    } else {
      let l = ft8_extract_symbol(&bins);
      log[bit_idx] = l[0];
      log[bit_idx + 1] = l[1];
      log[bit_idx + 2] = l[2];
    }
  }
  log
}

/// 归一化：把方差缩放到 24（ft8_lib 经验系数）。
///
/// 方差为 0（静默音频、整段似然全零）时 `24/0` 得 `inf`，再乘 0 就是 NaN —— NaN 灌进
/// BP 迭代只会白跑 25 轮。这里返回 `false` 让调用方直接跳过该候选。
fn normalize_logl(log: &mut [f32; 174]) -> bool {
  let sum: f32 = log.iter().sum();
  let sum2: f32 = log.iter().map(|x| x * x).sum();
  let inv_n = 1.0 / 174.0f32;
  let variance = (sum2 - sum * sum * inv_n) * inv_n;
  if !variance.is_finite() || variance <= 0.0 {
    return false;
  }
  let norm = (24.0 / variance).sqrt();
  if !norm.is_finite() {
    return false;
  }
  for x in log.iter_mut() {
    *x *= norm;
  }
  true
}

/// 候选是否落在 waterfall 的频/时网格内（防止 `pub` API 收到越界候选后索引 panic）。
fn candidate_in_bounds(wf: &Waterfall, cand: &Candidate) -> bool {
  let tones = if wf.protocol == Protocol::Ft4 { 4 } else { 8 };
  cand.freq_offset >= 0
    && cand.freq_offset as usize + tones <= wf.num_bins
    && (cand.freq_sub as usize) < wf.freq_osr
    && (cand.time_sub as usize) < wf.time_osr
}

/// 尝试解码一个候选：提取似然 → 归一化 → LDPC 软译码 → 验 CRC。
/// 成功返回 `(消息, 状态)`；失败返回 `None`（原因看 `status.ldpc_errors` / CRC 不匹配）。
pub fn decode_candidate(
  wf: &Waterfall,
  cand: &Candidate,
  max_iterations: usize,
) -> Option<(Message, DecodeStatus)> {
  if !candidate_in_bounds(wf, cand) {
    return None;
  }
  let mut log = extract_likelihood(wf, cand);
  if !normalize_logl(&mut log) {
    return None;
  }

  let mut plain = [0u8; 174];
  let ldpc_errors = bp_decode(&log, max_iterations, &mut plain);
  if ldpc_errors > 0 {
    return None;
  }

  // 前 91 位 = payload + CRC。
  let mut a91 = [0u8; 12];
  for (i, &b) in plain.iter().take(91).enumerate() {
    if b != 0 {
      a91[i / 8] |= 0x80 >> (i % 8);
    }
  }

  let crc_extracted = extract_crc(&a91);
  // CRC 在源编码消息零扩展 77→82 位上计算。
  a91[9] &= 0xF8;
  a91[10] = 0;
  // `a91` 固定 12 字节、82 位恒在范围内；若将来改动导致长度不符，宁可放弃该候选，
  // 也不能把错误吞成 0 —— 与「提取出的 CRC 恰为 0」撞上就会把垃圾当成有效消息。
  let crc_calculated = compute_crc(&a91, 96 - 14)?;
  if crc_extracted != crc_calculated {
    return None;
  }

  let mut payload = [0u8; 10];
  payload.copy_from_slice(&a91[..10]);
  if wf.protocol == Protocol::Ft4 {
    for (p, x) in payload.iter_mut().zip(FT4_XOR_SEQUENCE.iter()) {
      *p ^= *x;
    }
  }
  let status = DecodeStatus {
    ldpc_errors,
    crc_extracted,
    crc_calculated,
  };
  Some((Message { payload }, status))
}

/// 完整解码：从 waterfall 找候选并逐个尝试，返回所有成功解码的 `(消息, 频率 Hz, 时间 s, 分数)`。
pub fn decode_waterfall(
  wf: &Waterfall,
  symbol_period: f32,
  min_bin: usize,
  num_candidates: usize,
  min_score: i32,
  max_iterations: usize,
) -> Vec<(Message, f32, f32, i32)> {
  let mut out = Vec::new();
  for cand in find_candidates(wf, num_candidates, min_score) {
    if let Some((msg, _)) = decode_candidate(wf, &cand, max_iterations) {
      let freq_hz =
        (min_bin as f32 + cand.freq_offset as f32 + cand.freq_sub as f32 / wf.freq_osr as f32)
          / symbol_period;
      let time_sec =
        (cand.time_offset as f32 + cand.time_sub as f32 / wf.time_osr as f32) * symbol_period;
      out.push((msg, freq_hz, time_sec, cand.score));
    }
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::monitor::{Monitor, MonitorConfig};

  fn ft8_monitor() -> Monitor {
    Monitor::new(&MonitorConfig {
      f_min: 200.0,
      f_max: 3000.0,
      sample_rate: 12_000,
      time_osr: 2,
      freq_osr: 2,
      protocol: Protocol::Ft8,
    })
  }

  /// 静默音频：所有似然为 0、方差为 0，归一化必须拒绝，整段不应解出任何消息。
  #[test]
  fn silent_audio_decodes_nothing() {
    let mut mon = ft8_monitor();
    let silence = vec![0.0f32; mon.block_size];
    for _ in 0..mon.wf.max_blocks {
      assert!(mon.process(&silence), "时隙未收满就不该丢帧");
    }
    assert!(
      decode_waterfall(&mon.wf, mon.symbol_period, mon.min_bin, 20, 10, 25).is_empty(),
      "静默音频不应解出消息"
    );
  }

  /// 越界候选（频率起点超过 bin 数）必须被拒绝，而不是索引 panic。
  #[test]
  fn out_of_bounds_candidate_is_refused() {
    let mon = ft8_monitor();
    let cand = Candidate {
      score: 0,
      time_offset: 0,
      freq_offset: mon.wf.num_bins as i32,
      time_sub: 0,
      freq_sub: 0,
    };
    assert!(decode_candidate(&mon.wf, &cand, 25).is_none());
  }

  /// 帧长度不是 `block_size` 时 `process` 返回 `false`，不写任何数据。
  #[test]
  fn short_frame_is_rejected() {
    let mut mon = ft8_monitor();
    assert!(!mon.process(&[0.0f32; 16]));
    assert_eq!(mon.wf.num_blocks, 0);
  }
}
