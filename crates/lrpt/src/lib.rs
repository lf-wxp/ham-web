//! LRPT（Low Rate Picture Transmission）解码链路 —— 从 288 kHz I/Q 到图像。
//!
//! 参数与算法逐字转写自 **artlav/meteor_decoder**（GPL-3.0）与
//! **lrpt-encoder**（MIT，独立 Rust 实现、SatDump 可解码其输出）—— Meteor-M2
//! 系列（MetOp 的 LRPT 已退役，同一链路家族）。
//!
//! 完整链路（TX 视角）：
//! ```text
//! [ASM 1ACFFC1D | 1020 字节]
//!   → 数据区 XOR 去随机化序列（255 字节专用表）
//!   → RS(255,223) 交织 4（892 → 1020）
//!   → 卷积编码（CCSDS k=7, r=1/2, 0x79/0x5B）
//!   → NRZ-M → OQPSK（72k 符号/s，RRC 成形）
//! ```
//! RX = 逆序：RRC 匹配滤波（自动相位）→ 连续 Viterbi → NRZ-M → ASM 同步 →
//! 去随机化 → RS 译码 → VCDU/MPDU → MSU-MR 段 → JPEG 解码 → 三通道图像。
//!
//! 已实现：物理层（[`rrc`]/[`viterbi`]/[`diff`]）、帧层（[`cadu`]/[`rs`]）、
//! 封装层（[`vcdus`]）、图像层（[`jpeg`] + [`decode_image`]）。两个解调前端：
//! [`decode_stream288`]（288 kHz 整数采样 OQPSK，对齐 lrpt-encoder 合成信号）与
//! [`qpsk`]（任意采样率 QPSK + 定时 / 载波环路，Meteor-M2 真实过境录音已验证出图，
//! 见 `tests/real_iq.rs`）。M2-2（80k）的差分与卷积交织见 [`m2`]。
//! 未实现（如实说明）：M2-3 / M2-4 的 OQPSK 真实录音（缺样本）与软判决 Viterbi。

pub mod cadu;
pub mod diff;
pub mod huffman_tables;
pub mod jpeg;
pub mod m2;
pub mod oqpsk;
pub mod prand;
pub mod qpsk;
pub mod rrc;
pub mod rs;
pub mod vcdus;
pub mod viterbi;

pub use cadu::DATA_LEN;
use cadu::{ASM, FRAME_LEN, check_polarity, derandomize, find_asm};
use diff::nrzm_decode;
use viterbi::{conv_encode, viterbi_decode, viterbi_decode_continuous};

/// 每符号样本数（合成样本用 4）。
pub const DEFAULT_SPS: usize = 4;

/// 一帧数据（1020 字节）→ OQPSK 基带样本（合成端）。
#[must_use]
pub fn encode_frame(data: &[u8; DATA_LEN], sps: usize) -> Vec<f32> {
  let mut frame = [0u8; FRAME_LEN];
  frame[..ASM.len()].copy_from_slice(&ASM);
  frame[ASM.len()..].copy_from_slice(data);
  derandomize(&mut frame);
  let bits: Vec<u8> = frame
    .iter()
    .flat_map(|&b| (0..8).map(move |i| (b >> (7 - i)) & 1))
    .collect();
  let coded = conv_encode(&bits);
  oqpsk::modulate(&coded, sps)
}

/// 已对齐的样本流（每帧 `2×(8192+6)×sps` 个复数样本）→ 解码出的数据帧。
///
/// 帧边界必须已知（合成样本即如此）；真实录音的对齐搜索是下一层的工作。
#[must_use]
pub fn decode_aligned(samples: &[f32], sps: usize) -> Vec<[u8; DATA_LEN]> {
  let coded = oqpsk::demodulate(samples, sps);
  let coded_len_per_frame = 2 * (FRAME_LEN * 8 + viterbi::K - 1);
  let mut out = Vec::new();
  let mut offset = 0usize;
  while offset + coded_len_per_frame <= coded.len() {
    let bits = viterbi_decode(&coded[offset..offset + coded_len_per_frame]);
    let mut frame = [0u8; FRAME_LEN];
    for (i, b) in frame.iter_mut().enumerate() {
      let mut byte = 0u8;
      for k in 0..8 {
        byte = (byte << 1) | bits[i * 8 + k];
      }
      *b = byte;
    }
    if check_polarity(&frame) {
      for b in &mut frame[ASM.len()..] {
        *b = !*b;
      }
    }
    derandomize(&mut frame);
    let mut data = [0u8; DATA_LEN];
    data.copy_from_slice(&frame[ASM.len()..]);
    out.push(data);
    offset += coded_len_per_frame;
  }
  out
}

/// 完整接收链（真实形式，对齐 lrpt-encoder / SatDump 的链路）：自动搜索采样
/// 相位 × 字节网格（各解一次，取解出帧数最多者），然后 288 kHz I/Q → RRC 匹配
/// 滤波 → 每符号采样 → 连续 Viterbi → NRZ-M 译码 → ASM 同步 → 逐帧去随机化
/// → 1020 字节 RS 码字。
///
/// RRC 匹配滤波与搜索参数无关，只做一次；循环内只改采样偏移与字节网格起点。
#[must_use]
pub fn decode_stream288(iq: &[f32]) -> Vec<[u8; DATA_LEN]> {
  let (fi, fq) = rrc::matched_filter(iq);
  if fi.is_empty() {
    return Vec::new();
  }
  let mut best: Vec<[u8; DATA_LEN]> = Vec::new();
  for phase in 0..rrc::SPS {
    for frames in decode_stream288_phase(&fi, &fq, phase) {
      if frames.len() > best.len() {
        best = frames;
      }
    }
  }
  best
}

/// MSU-MR 图像宽度（像素/行）。
pub const IMAGE_WIDTH: usize = 1568;
/// 每行的段数（14 段 × 14 MCU = 196 MCU = 1568 像素）。
pub const SEGMENTS_PER_LINE: usize = 14;

/// 一天的毫秒数；段载荷的时间戳是 UTC 当天毫秒（跨午夜会回绕）。
const DAY_MS: u32 = 86_400_000;
const HALF_DAY_MS: u32 = DAY_MS / 2;

/// 段时间戳是 UTC 当天毫秒：录音跨午夜时，午夜前的段（~86_400_000）比午夜后的段大得多，
/// 直接取 min 做基准会让午夜前的行排到午夜后之后（行序颠倒，静默输出错乱图像）。
/// 一次过境不会超过半天，所以极差超过半天即判定跨午夜，把「早半天」的值折到次日。
fn fold_midnight(times: &[u32]) -> Vec<u32> {
  let (Some(&min), Some(&max)) = (times.iter().min(), times.iter().max()) else {
    return Vec::new();
  };
  let wrapped = max - min > HALF_DAY_MS;
  times
    .iter()
    .map(|&t| {
      if wrapped && t < HALF_DAY_MS {
        t + DAY_MS
      } else {
        t
      }
    })
    .collect()
}

/// MSU-MR 六个通道的 APID 范围（64 = 通道 1 … 69 = 通道 6）。
const IMAGE_APIDS: std::ops::RangeInclusive<u16> = 64..=69;

/// 解码出的 LRPT 图像（三通道，按 APID 从小到大排）。
#[derive(Debug, Clone)]
pub struct LrptImage {
  /// 像素宽度（1568）。
  pub width: usize,
  /// 像素高度（保留的完整 MCU 行数 × 8）。
  pub height: usize,
  /// 首行在**本段录音**内的 MCU 行号（相对首个收到的行；头尾残缺行被丢弃后不为 0）
  /// —— 供真值比对定位。绝对行号还需要图像起始时刻，不在本层掌握。
  pub first_row_in_recording: usize,
  /// 三个通道各自的 APID（升序）。卫星每次过境只下传 MSU-MR 六个通道里的三个，
  /// 选哪三个由地面控制（白天常见 64/65/66，也有 64/65/68 这种带红外的组合）。
  pub apids: [u16; 3],
  /// 三通道灰度平面（每个 Vec 是一行），顺序与 [`Self::apids`] 一致。
  pub channels: [Vec<Vec<u8>>; 3],
}

/// 帧层之上：RS 译码 → VCDU → MPDU 源包 → MSU-MR 段 → JPEG 解码 → 图像。
///
/// 坏帧不再连坐整段录音：RS 无法纠正的帧跳过、帧计数允许空洞（真实过境录音里丢帧是
/// 常态），JPEG 段解不开只丢该段（该行不完整，随后被 `row_complete` 丢弃）。
#[must_use]
pub fn decode_image(frames: &[[u8; DATA_LEN]]) -> Option<LrptImage> {
  let mut mux = vcdus::MuxStream::new();
  for frame in frames {
    if let Some(vcdu) = rs::decode_frame(frame) {
      mux.push(&vcdu);
    }
  }
  // 计数允许空洞（丢帧），但必须单调递增：乱序说明同步的是错位的帧。
  if !mux.counters_are_monotonic() {
    return None;
  }
  // 段按「行索引」装配：段载荷的 ms 时间戳（ms_of_day + 行号×500，编码器
  // 约定）给出图像行号 —— 用它与通道无关地对齐，而不是依赖到达顺序。
  // 只保留**完整行**（14 段、MCUN 恰为 0,14,…,182）：录音头尾被收敛区 /
  // 帧边界吃掉的残缺行如实丢弃（短夹具上首帧丢失是常态）。
  type Row = Vec<(u8, Vec<[[u8; 8]; 8]>)>; // (MCUN, 14 个块)
  fn row_complete(row: &Row) -> bool {
    row.len() == SEGMENTS_PER_LINE
      && row
        .iter()
        .enumerate()
        .all(|(i, (mcu, _))| usize::from(*mcu) == i * 14)
  }
  let mut pending = Vec::new();
  for (apid, _, payload) in mux.packets() {
    if !IMAGE_APIDS.contains(&apid) || payload.len() < 14 {
      continue; // 遥测（APID 70）等非图像包
    }
    let ms = u32::from_be_bytes([payload[2], payload[3], payload[4], payload[5]]);
    pending.push((apid, ms, payload));
  }
  let times: Vec<u32> = pending.iter().map(|&(_, ms, _)| ms).collect();
  let times = fold_midnight(&times);
  for (entry, t) in pending.iter_mut().zip(&times) {
    entry.1 = *t;
  }
  let base = *times.iter().min()?;
  // 取包数最多的三个图像 APID（偶发的误码包不会凑成一个「通道」），再按 APID 升序排。
  let mut counts: std::collections::BTreeMap<u16, usize> = std::collections::BTreeMap::new();
  for &(apid, _, _) in &pending {
    *counts.entry(apid).or_default() += 1;
  }
  let mut ranked: Vec<(u16, usize)> = counts.into_iter().collect();
  ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
  if ranked.len() < 3 {
    return None;
  }
  let mut apids = [ranked[0].0, ranked[1].0, ranked[2].0];
  apids.sort_unstable();
  let mut rows: [std::collections::BTreeMap<usize, Row>; 3] = Default::default();
  for (apid, ms, payload) in pending {
    let Some(ch) = apids.iter().position(|&a| a == apid) else {
      continue;
    };
    let row_idx = ((ms - base) / 500) as usize;
    let mcu_number = payload[8];
    let quality = payload[13];
    // 段解不开只丢这一段（该行其余段仍保留，缺段的整行会被 `row_complete` 丢掉）。
    let Some(blocks) = jpeg::decode_segment(&payload[14..], quality) else {
      continue;
    };
    rows[ch]
      .entry(row_idx)
      .or_default()
      .push((mcu_number, blocks));
  }
  // 三通道都要有同一行索引的完整行（交集），按行号排序输出。
  let indices: Vec<usize> = rows[0]
    .iter()
    .filter(|(idx, row)| {
      row_complete(row)
        && rows[1].get(*idx).is_some_and(row_complete)
        && rows[2].get(*idx).is_some_and(row_complete)
    })
    .map(|(idx, _)| *idx)
    .collect();
  if indices.is_empty() {
    return None;
  }
  let first_row_in_recording = indices[0];

  // 行 → 像素平面：每行 14 段、每段 14 个 8×8 块、块按 MCUN 定位。
  let mut channels: [Vec<Vec<u8>>; 3] = [Vec::new(), Vec::new(), Vec::new()];
  for ch in 0..3 {
    let mut plane = Vec::with_capacity(indices.len() * 8);
    // 一整行只分配一块（8 × IMAGE_WIDTH）再分片写入，而不是 8 个独立 Vec：
    // 每个完整行几百个，每行省 8 次分配。
    let mut expanded = vec![0u8; 8 * IMAGE_WIDTH];
    for &idx in &indices {
      let row = &rows[ch][&idx];
      // 段 × 14 块 → 8 行像素：段按 MCUN 定位（每段 112 像素宽）。
      expanded.fill(0);
      for (seg_x, (_, blocks)) in row.iter().enumerate() {
        for (i, block) in blocks.iter().enumerate() {
          let x0 = seg_x * 112 + i * 8;
          for (y, brow) in block.iter().enumerate() {
            expanded[y * IMAGE_WIDTH + x0..y * IMAGE_WIDTH + x0 + 8].copy_from_slice(brow);
          }
        }
      }
      for y in 0..8 {
        plane.push(expanded[y * IMAGE_WIDTH..(y + 1) * IMAGE_WIDTH].to_vec());
      }
    }
    channels[ch] = plane;
  }
  Some(LrptImage {
    width: IMAGE_WIDTH,
    height: indices.len() * 8,
    first_row_in_recording,
    apids,
    channels,
  })
}

/// 已滤波 I/Q 在指定采样相位下的候选帧集：每个候选对应一个字节网格起点。
///
/// 采样相位只解决「符号采样点」，而解调输出相对真实编码流的延迟还取决于发送端成形
/// （合成端与 lrpt-encoder 实测在 0–66 编码位之间）—— **字节网格会跟着平移**，而相位
/// 搜索只能挪 0..SPS 个采样点（≈ 1.5 个编码位），补不了这种错位。帧长是 8 位的整数倍，
/// 所以真正要试的只是 8 个字节网格起点：昂贵的部分（解调 / Viterbi / NRZ-M）只做一次，
/// 随后按起点各跑一遍 ASM 命中。
fn decode_stream288_phase(fi: &[f32], fq: &[f32], phase: usize) -> Vec<Vec<[u8; DATA_LEN]>> {
  let coded = rrc::demodulate_288_phase(fi, fq, phase);
  // 丢掉滤波延迟对应的前导编码位（偶数位：OQPSK 的 I/Q 配对不能错位）—— 它只决定
  // 从哪里开始译码，字节网格由下面的起点搜索负责。
  let delay_bits = rrc::FILTER_DELAY_SYMBOLS * 2;
  let drop_bits = (16 - delay_bits % 16) % 16;
  // 短输入（< 约 1.1 ms 的录音或纯噪声）解不出任何符号：直接返回空，
  // 不能让切片越界 panic 传播成 wasm trap。
  if coded.len() <= drop_bits {
    return Vec::new();
  }
  let coded = &coded[drop_bits..];
  let nrzm_bits = viterbi_decode_continuous(coded);
  let mut last = 0u8;
  let decoded_bits = nrzm_decode(&nrzm_bits, &mut last);
  (0..8)
    .map(|shift| frames_from_bits(decoded_bits.get(shift..).unwrap_or_default()))
    .collect()
}

/// 位流（已 NRZ-M 译码）→ 逐帧数据：ASM 同步 + 容错复检 + 逐帧去随机化。
///
/// 容错 1 位是必要的：流首第一位没有真实前驱（约 50% 概率出错），它会落在首个 ASM
/// 的第一位上；复检同样允许 1 位错（错在 ASM 内、帧数据区不受影响），否则容错搜索
/// 会被精确复检全部抵消。
fn frames_from_bits(bits: &[u8]) -> Vec<[u8; DATA_LEN]> {
  let mut bytes = Vec::with_capacity(bits.len() / 8);
  for chunk in bits.chunks(8).filter(|c| c.len() == 8) {
    let mut b = 0u8;
    for &bit in chunk {
      b = (b << 1) | bit;
    }
    bytes.push(b);
  }
  let hits = find_asm(&bytes, 1);
  let mut out = Vec::new();
  for &start in &hits {
    if start + FRAME_LEN > bytes.len() {
      continue;
    }
    let frame = &bytes[start..start + FRAME_LEN];
    let asm_bad = (0..ASM.len() * 8)
      .filter(|&bit| {
        ((frame[bit / 8] >> (7 - bit % 8)) & 1) != ((ASM[bit / 8] >> (7 - bit % 8)) & 1)
      })
      .count();
    if asm_bad > 1 {
      continue;
    }
    let mut data = [0u8; DATA_LEN];
    data.copy_from_slice(&frame[ASM.len()..]);
    derandomize_data(&mut data);
    out.push(data);
  }
  out
}

/// 数据区去随机化（`PRAND[i % 255]`，ASM 之外的 1020 字节）。
fn derandomize_data(data: &mut [u8; DATA_LEN]) {
  for (i, b) in data.iter_mut().enumerate() {
    *b ^= prand::PRAND[i % prand::PRAND.len()];
  }
}

/// 合成端（真实链路形式）：帧 → 扰码 → NRZ-M → 连续卷积 → RRC OQPSK @ 288 kHz。
///
/// 与 [`decode_stream288`] 互逆，供无外部依赖的端到端测试。
#[must_use]
pub fn encode_stream288(frames: &[[u8; DATA_LEN]]) -> Vec<f32> {
  use cadu::ASM;
  use diff::nrzm_encode;
  use viterbi::conv_encode_continuous;

  let mut nrzm_state = 0u8;
  let mut conv_state = 0u8;
  let mut coded = Vec::new();
  for data in frames {
    let mut cadu = Vec::with_capacity(FRAME_LEN);
    cadu.extend_from_slice(&ASM);
    let mut d = *data;
    derandomize_data(&mut d);
    cadu.extend_from_slice(&d);
    let bits: Vec<u8> = cadu
      .iter()
      .flat_map(|&b| (0..8).map(move |i| (b >> (7 - i)) & 1))
      .collect();
    let enc = nrzm_encode(&bits, &mut nrzm_state);
    coded.extend(conv_encode_continuous(&enc, &mut conv_state));
  }
  // OQPSK + 根升余弦成形 —— 与 lrpt-encoder 的调制器一致：I/Q 各自是
  // **冲激串**（±1 在符号起点、间隔 SPS、Q 滞后 SPS/2），过同一个 RRC 滤波器
  // （方框脉冲会让接收端匹配滤波后的群延迟不同 —— 对拍实测抓到的差异）。
  let taps = rrc::rrc_taps();
  let n = taps.len();
  let symbols = coded.len() / 2;
  let mut i_imp = vec![0.0f32; symbols * rrc::SPS];
  let mut q_imp = vec![0.0f32; symbols * rrc::SPS];
  for k in 0..symbols {
    let bi = coded[2 * k];
    let bq = coded[2 * k + 1];
    i_imp[k * rrc::SPS] = if bi == 0 { 1.0 } else { -1.0 };
    let qpos = k * rrc::SPS + rrc::SPS / 2;
    if qpos < q_imp.len() {
      q_imp[qpos] = if bq == 0 { 1.0 } else { -1.0 };
    }
  }
  let conv = |ch: &[f32]| -> Vec<f32> {
    let len = ch.len().saturating_sub(n - 1);
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
      let mut acc = 0.0f32;
      for (k, &t) in taps.iter().enumerate() {
        acc += ch[i + k] * t;
      }
      out.push(acc);
    }
    out
  };
  let fi = conv(&i_imp);
  let fq = conv(&q_imp);
  let mut out = Vec::with_capacity(fi.len() * 2);
  for (a, b) in fi.iter().zip(&fq) {
    out.push(*a);
    out.push(*b);
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  fn sample_data() -> [u8; DATA_LEN] {
    let mut d = [0u8; DATA_LEN];
    for (i, b) in d.iter_mut().enumerate() {
      *b = (i * 17 % 256) as u8;
    }
    d
  }

  #[test]
  fn full_chain_round_trips() {
    let data = sample_data();
    let samples = encode_frame(&data, DEFAULT_SPS);
    let decoded = decode_aligned(&samples, DEFAULT_SPS);
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0], data, "无噪声整链往返");
  }

  #[test]
  fn multiple_frames_round_trip() {
    let mut stream = Vec::new();
    let data = sample_data();
    for _ in 0..3 {
      stream.extend(encode_frame(&data, DEFAULT_SPS));
    }
    let decoded = decode_aligned(&stream, DEFAULT_SPS);
    assert_eq!(decoded.len(), 3);
    for d in &decoded {
      assert_eq!(d, &data);
    }
  }

  #[test]
  fn scattered_noise_is_corrected_by_viterbi() {
    let data = sample_data();
    let mut samples = encode_frame(&data, DEFAULT_SPS);
    // 每个复数样本加 ±0.25 抖动（合成端没有信道编码之外的冗余，噪声要在
    // 卷积码的纠正能力内）。
    for (i, s) in samples.iter_mut().enumerate() {
      *s += ((i * 37 % 10) as f32 - 4.5) * 0.11;
    }
    let decoded = decode_aligned(&samples, DEFAULT_SPS);
    assert_eq!(decoded[0], data, "轻噪声下整链仍应还原");
  }

  #[test]
  fn real_chain_round_trips_with_rrc() {
    // 真实链路形式：NRZ-M + 连续卷积（无尾位）+ RRC 滤波 @ 288 kHz。
    // 滑窗译码的首帧被收敛前缀吃掉、末帧被无前瞻尾部吃掉 —— 10 帧验证
    // 中间帧逐字节一致。
    let frames: Vec<[u8; DATA_LEN]> = (0..10)
      .map(|k| {
        let mut d = [0u8; DATA_LEN];
        for (i, b) in d.iter_mut().enumerate() {
          *b = (i * 13 + k * 7) as u8;
        }
        d
      })
      .collect();
    let iq = encode_stream288(&frames);
    let decoded = decode_stream288(&iq);
    assert!(
      decoded.len() >= 8,
      "至少解出中间 8 帧，得到 {}",
      decoded.len()
    );
    for (d, f) in decoded.iter().zip(&frames[1..]) {
      assert_eq!(d, f, "逐帧一致");
    }
  }

  #[test]
  fn byte_grid_search_recovers_symbol_level_shifts() {
    // 发送端成形延迟变化会让解调流整体挪动若干**整符号**（= 1 个数据位）：字节网格随
    // 之平移，只搜采样相位 + 常量丢弃会一帧都找不到。
    let frames: Vec<[u8; DATA_LEN]> = (0..10)
      .map(|k| {
        let mut d = [0u8; DATA_LEN];
        for (i, b) in d.iter_mut().enumerate() {
          *b = (i * 13 + k * 7) as u8;
        }
        d
      })
      .collect();
    let iq = encode_stream288(&frames);
    // 丢掉前 3 个符号（3 个数据位）：字节网格整体后移 3 位。
    let shifted = &iq[3 * rrc::SPS * 2..];
    let decoded = decode_stream288(shifted);
    assert!(
      decoded.len() >= 8,
      "至少解出中间 8 帧，得到 {}",
      decoded.len()
    );
    for (d, f) in decoded.iter().zip(&frames[1..]) {
      assert_eq!(d, f, "逐帧一致");
    }
  }

  #[test]
  fn timestamps_crossing_midnight_keep_row_order() {
    // R2-10：23:59:59.500 与 00:00:00.500 同一过境，后者必须排在前者之后。
    let before = DAY_MS - 500;
    let after = 500;
    let folded = fold_midnight(&[before, after, before - 500]);
    assert_eq!(folded, vec![before, after + DAY_MS, before - 500]);
    // 不跨午夜时原样返回；空输入不 panic。
    assert_eq!(
      fold_midnight(&[1_000, 1_500, 2_000]),
      vec![1_000, 1_500, 2_000]
    );
    assert!(fold_midnight(&[]).is_empty());
  }

  #[test]
  fn oversized_sps_and_phase_are_rejected() {
    // R2-05：sps = 2^31（wasm32 上 2*sps 回绕为 0）/ 2^63+1。
    let samples = vec![0.5f32; 64];
    for sps in [oqpsk::MAX_SPS + 1, 1 << 31, (1usize << 63) + 1, usize::MAX] {
      assert!(oqpsk::demodulate(&samples, sps).is_empty(), "sps={sps}");
      assert!(oqpsk::modulate(&[0, 1, 1, 0], sps).is_empty(), "sps={sps}");
    }
    assert!(oqpsk::modulate(&[0, 1], 0).is_empty());
    // R2-06：phase 越界不得回绕后索引越界。
    let fi = vec![1.0f32; 64];
    for phase in [rrc::SPS, rrc::SPS + 1, usize::MAX - 1, usize::MAX] {
      assert!(
        rrc::demodulate_288_phase(&fi, &fi, phase).is_empty(),
        "phase={phase}"
      );
    }
  }

  #[test]
  fn zero_samples_per_symbol_is_rejected() {
    // sps=0 曾是整数除零 panic（公共 API，release 也会 panic）。
    assert!(oqpsk::demodulate(&[0.0f32; 8], 0).is_empty());
    assert!(decode_aligned(&[0.0f32; 8], 0).is_empty());
  }

  #[test]
  fn empty_and_short_inputs_do_not_panic() {
    // 空输入与短于一个滤波器窗口的输入（损坏 / 纯噪声录音）：应返回空
    // 而不是切片越界 panic（worker 场景里那会变成 wasm trap）。
    assert!(decode_stream288(&[]).is_empty());
    let short = vec![0.0f32; 64]; // 32 个复数样本，远不足 133 抽头
    assert!(decode_stream288(&short).is_empty());
    assert!(m2::decode_stream80k(&[]).is_empty());
  }

  #[test]
  fn asm_constants_match_reference() {
    // 参考实现（artlav/meteor_decoder）的同步字与多项式。
    assert_eq!(ASM, [0x1A, 0xCF, 0xFC, 0x1D]);
    // 多项式对齐 lrpt-encoder（SatDump 互操作）：171₈/133₈ = 0x79/0x5B。
    assert_eq!(viterbi::POLYA, 0x79);
    assert_eq!(viterbi::POLYB, 0x5B);
    assert_eq!(
      &prand::PRAND[..4],
      &[0xFF, 0x48, 0x0E, 0xC0],
      "prand 表逐字转写自 met_to_data.pas"
    );
  }
}
