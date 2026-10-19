//! AFSK 1200（Bell 202）软 TNC：AX.25 帧 ↔ HDLC 位流 ↔ 基带音频采样。
//!
//! 补「零硬件玩 APRS / Packet」的最后一环——[`crate::kiss`] 已有 AX.25 UI 帧（无 FCS），
//! 本模块实现**空中链路层**：
//!
//! 1. **FCS**：CRC-16/X-25（反射多项式 0x8408、初值 0xFFFF、**输出取反**），两个字节低位在前追加；
//! 2. **HDLC**：`0x7E` 标志定界 + 位填充（连 5 个 1 后插 0）+ 字节低位在前；
//! 3. **NRZI**：`0` 翻转电平、`1` 保持；
//! 4. **AFSK1200**：mark（1 电平）= 1200 Hz、space（0 电平）= 2200 Hz，1200 波特。
//!
//! 全程确定性合成，用「帧 → 音频 → 帧」往返对拍；解调用逐位 Goertzel 相关器（相位无关），
//! 符号定时恢复把每个位相位都扫一遍并合并结果（[`demodulate_frames`]），因此可解任意
//! 起始采样偏移的录音；仍假设 1200 / 2200 Hz 精确频率与 1200 波特精确速率。
//!
//! # 互通性
//!
//! 往返对拍只能证明两端自洽，证明不了与真实设备互通——FCS 少一次取反这种错，自编自解永远
//! 全绿。所以 FCS 另有外部已知答案钉死（CRC-16/X-25 的公开校验值与 RFC 1662 的剩余值），
//! 但与 Direwolf / 硬件 TNC 的真机互通仍记在 `docs/hardware-verification.md` 的 C4（待验）。

use std::f64::consts::TAU;

/// 采样率（Hz；48 kHz 与 1200 波特成整数比，每符号 40 采样）。
pub const SAMPLE_RATE: u32 = 48_000;
/// 波特率。
pub const BAUD: f64 = 1200.0;
/// mark 音频（Hz，逻辑 1）。
pub const MARK_HZ: f64 = 1200.0;
/// space 音频（Hz，逻辑 0）。
pub const SPACE_HZ: f64 = 2200.0;

/// 一次解调最多处理的音频时长（秒）。
///
/// 符号定时恢复要穷举「位相位 × NRZI 初始电平」，成本与样本数成正比：48 kHz 下每分钟
/// 音频是 288 万样本 × 40 个相位 × 2 个音调。APRS / Packet 的一帧不到一秒，录音通常
/// 只有几十秒；超过这个长度一律截断（调用方要把"已截断"告诉用户），避免为一段几分钟的
/// 录音白算几分钟。
pub const MAX_DECODE_SECONDS: u32 = 60;

/// AX.25 的最短合法帧（字节）：目的地址 7 + 源地址 7 + 控制 1，FCS 不计。
///
/// 解调结果里更短的"帧"只能是噪声里碰巧通过 CRC 的残片（16 位 CRC 对随机数据有 1/65536
/// 的通过率，而 40 个相位 × 2 个初值会把这个概率放大几十倍），直接丢弃。
const MIN_AX25_FRAME: usize = 15;

/// 发送侧起始前导的 flag 个数（≈ 213 ms）。
///
/// 真实 TNC 在帧前先发一串 flag（TXDelay），让对端的 PLL / 静噪 / PTT 都稳定下来；只发一个
/// flag 的话 VOX 启动的那几百毫秒会把首帧吃掉。
pub const TX_PREAMBLE_FLAGS: usize = 32;
/// 发送侧帧后尾部的 flag 个数（≈ 27 ms）：保证最后一个字节完整发出再关 PTT。
pub const TX_TAIL_FLAGS: usize = 4;

/// AX.25 FCS：CRC-16/X-25（反射多项式 0x8408，初值 0xFFFF，输出异或 0xFFFF）。
///
/// 输出异或不能省：公开校验值 `crc16_ax25(b"123456789") == 0x906E`，缺了它得到 `0x6F91`，
/// 生成的音频会被真实 TNC 一律判为 FCS 错误。
#[must_use]
pub fn crc16_ax25(bytes: &[u8]) -> u16 {
  let mut crc: u16 = 0xFFFF;
  for &b in bytes {
    crc ^= b as u16;
    for _ in 0..8 {
      crc = if crc & 1 != 0 {
        (crc >> 1) ^ 0x8408
      } else {
        crc >> 1
      };
    }
  }
  !crc
}

/// `0x7E` 标志的位序列（低位在前）：`01111110`。
fn flag_bits() -> [bool; 8] {
  [false, true, true, true, true, true, true, false]
}

/// 字节低位在前展开成位。
fn byte_bits(byte: u8) -> [bool; 8] {
  let mut b = [false; 8];
  for (i, v) in b.iter_mut().enumerate() {
    *v = (byte >> i) & 1 == 1;
  }
  b
}

/// 一个字节（低位在前）是否为 `0x7E` 标志。
fn is_flag(bits: &[bool]) -> bool {
  bits.len() >= 8 && bits[..8] == flag_bits()
}

/// AX.25 帧 → HDLC 位流（`0x7E` 标志 + 位填充 + FCS）。
#[must_use]
pub fn hdlc_encode(frame: &[u8]) -> Vec<bool> {
  let fcs = crc16_ax25(frame);
  let mut out = flag_bits().to_vec();
  let mut ones = 0usize;
  for byte in frame
    .iter()
    .copied()
    .chain([(fcs & 0xFF) as u8, (fcs >> 8) as u8])
  {
    for bit in byte_bits(byte) {
      if ones == 5 {
        out.push(false); // 填充位
        ones = 0;
      }
      out.push(bit);
      if bit {
        ones += 1;
      } else {
        ones = 0;
      }
    }
  }
  out.extend_from_slice(&flag_bits());
  out
}

/// HDLC 位流 → AX.25 帧列表（去标志 / 去填充 / 验 FCS；FCS 错或残帧丢弃）。
///
/// 结束标志**不消耗**：AX.25 允许一个 `0x7E` 同时当上一帧的结束与下一帧的起始，消耗掉它
/// 会让紧跟着的那一帧找不到起始标志、整帧丢失。连续多个标志（空闲填充）由此自然变成空帧，
/// 在长度校验处被丢弃，不会死循环。
#[must_use]
pub fn hdlc_decode(bits: &[bool]) -> Vec<Vec<u8>> {
  let mut frames = Vec::new();
  let mut i = 0usize;
  while i + 8 <= bits.len() {
    if !is_flag(&bits[i..i + 8]) {
      i += 1;
      continue;
    }
    i += 8;
    let mut data = Vec::new();
    let mut ones = 0usize;
    let mut byte = 0u8;
    let mut nbits = 0usize;
    loop {
      if i + 8 <= bits.len() && is_flag(&bits[i..i + 8]) {
        break;
      }
      if i >= bits.len() {
        break;
      }
      let bit = bits[i];
      i += 1;
      // 位去填充：连 5 个 1 后的 0 是填充位。
      if ones == 5 && !bit {
        ones = 0;
        continue;
      }
      if bit {
        ones += 1;
      } else {
        ones = 0;
      }
      if bit {
        byte |= 1 << nbits;
      }
      nbits += 1;
      if nbits == 8 {
        data.push(byte);
        byte = 0;
        nbits = 0;
      }
    }
    // 载荷必须非空：连续标志（空闲填充）得到空帧，而 CRC-16/X-25 对空载荷恰好是 0，
    // 空载荷 + 全零 FCS 会碰巧通过校验。
    if data.len() > 2 {
      let (body, fcs_bytes) = data.split_at(data.len() - 2);
      let fcs = (fcs_bytes[0] as u16) | ((fcs_bytes[1] as u16) << 8);
      if crc16_ax25(body) == fcs {
        frames.push(body.to_vec());
      }
    }
  }
  frames
}

/// NRZI 编码：`0` 翻转、`1` 保持；返回电平序列（`true` = 高电平）。
#[must_use]
pub fn nrzi_encode(bits: &[bool]) -> Vec<bool> {
  let mut level = false;
  bits
    .iter()
    .map(|&b| {
      if !b {
        level = !level;
      }
      level
    })
    .collect()
}

/// NRZI 解码：电平翻转 → `0`、保持 → `1`（初始电平取 `false`）。
#[must_use]
pub fn nrzi_decode(levels: &[bool]) -> Vec<bool> {
  nrzi_decode_init(levels, false)
}

/// NRZI 解码（可指定初始电平；真实信号起始电平任意，两种都试）。
#[must_use]
pub fn nrzi_decode_init(levels: &[bool], init: bool) -> Vec<bool> {
  let mut prev = init;
  levels
    .iter()
    .map(|&l| {
      let same = l == prev;
      prev = l;
      same
    })
    .collect()
}

/// NRZI 电平序列 → AFSK1200 基带采样（连续相位 FSK）。
#[must_use]
pub fn afsk_modulate(levels: &[bool], sample_rate: u32) -> Vec<f32> {
  let per_bit = (sample_rate as f64 / BAUD).round() as usize;
  if per_bit == 0 {
    return Vec::new();
  }
  let mut out = Vec::with_capacity(levels.len() * per_bit);
  let mut phase = 0.0f64;
  for &level in levels {
    let f = if level { MARK_HZ } else { SPACE_HZ };
    for _ in 0..per_bit {
      out.push(phase.sin() as f32);
      phase = (phase + TAU * f / sample_rate as f64) % TAU;
    }
  }
  out
}

/// 采样 → NRZI 电平序列（逐位 Goertzel 相关器，相位无关）。
///
/// `phase` 为起始采样偏移（`0..每符号采样数`），供定时恢复扫描；位对齐时取 0。
#[must_use]
pub fn sample_levels(samples: &[f32], sample_rate: u32, phase: usize) -> Vec<bool> {
  let per_bit = (sample_rate as f64 / BAUD).round() as usize;
  // 采样率低于波特率时每符号不足一个采样：`start` 不会前进，循环永远不结束。
  if per_bit == 0 {
    return Vec::new();
  }
  let mut levels = Vec::new();
  let mut start = phase;
  // 相关系数算一次就够：逐样本算 `cos` / `sin` 会把这层从 1 乘 2 加变成 4 万次三角
  // 函数（每符号两个音调 × 40 个相位 × 每个采样），整段录音就是几十亿次。
  let mark = goertzel_coeff(MARK_HZ, sample_rate);
  let space = goertzel_coeff(SPACE_HZ, sample_rate);
  while start + per_bit <= samples.len() {
    let seg = &samples[start..start + per_bit];
    levels.push(tone_energy(seg, mark) > tone_energy(seg, space));
    start += per_bit;
  }
  levels
}

/// 位对齐的逐位解调（[`sample_levels`] 在 `phase = 0` 的特例）。
#[must_use]
pub fn afsk_demodulate(samples: &[f32], sample_rate: u32) -> Vec<bool> {
  sample_levels(samples, sample_rate, 0)
}

/// Goertzel 递推系数（`2cos(2πf/fs)`）：与 [`tone_energy`] 配套，调用方只算一次。
fn goertzel_coeff(f_hz: f64, sample_rate: u32) -> f64 {
  2.0 * (TAU * f_hz / sample_rate as f64).cos()
}

/// 单频能量（Goertzel 递推，相位无关）；`coeff` 来自 [`goertzel_coeff`]。
///
/// 与直接算 DFT 相比少两次三角函数、每样本只做一次乘加；两者相差一个与段长相关的
/// 常数因子，而这里只比较同一段的两个音调，比值不受影响。
fn tone_energy(seg: &[f32], coeff: f64) -> f64 {
  let (mut s1, mut s2) = (0.0f64, 0.0f64);
  for &s in seg {
    let s0 = f64::from(s) + coeff * s1 - s2;
    s2 = s1;
    s1 = s0;
  }
  s1 * s1 + s2 * s2 - coeff * s1 * s2
}

/// 线性重采样到目标采样率（实现只有一份，在 [`ham_web_wav::resample`]）。
#[must_use]
pub fn resample(samples: &[f32], from: u32, to: u32) -> Vec<f32> {
  ham_web_wav::resample(samples, from, to)
}

/// 便捷：AX.25 帧 → AFSK1200 基带采样（含 NRZI + HDLC）。
#[must_use]
pub fn modulate_frame(frame: &[u8], sample_rate: u32) -> Vec<f32> {
  afsk_modulate(&nrzi_encode(&hdlc_encode(frame)), sample_rate)
}

/// 便捷：AX.25 帧 → **可直接馈入电台**的基带采样（前导 flag + 帧 + 尾部 flag）。
///
/// 与 [`modulate_frame`] 的区别只在两头多出的 `0x7E`：`modulate_frame` 是给对拍用的最小
/// 形态，实发必须带前导（见 [`TX_PREAMBLE_FLAGS`]），否则 VOX / PTT 启动会吃掉首帧。
#[must_use]
pub fn modulate_frame_for_tx(frame: &[u8], sample_rate: u32) -> Vec<f32> {
  let mut bits = Vec::with_capacity(8 * (TX_PREAMBLE_FLAGS + TX_TAIL_FLAGS) + 16 * frame.len());
  for _ in 0..TX_PREAMBLE_FLAGS {
    bits.extend_from_slice(&flag_bits());
  }
  bits.extend_from_slice(&hdlc_encode(frame));
  for _ in 0..TX_TAIL_FLAGS {
    bits.extend_from_slice(&flag_bits());
  }
  afsk_modulate(&nrzi_encode(&bits), sample_rate)
}

/// 便捷：AFSK1200 基带采样 → AX.25 帧列表（含解调 + NRZI + HDLC + 符号定时恢复）。
///
/// 把每个起始采样偏移（`0..每符号采样数`）都扫一遍，再按帧内容合并：只取第一个有结果的
/// 相位会在多电台 / 时钟有偏差的录音里漏帧（别处正好对齐的帧被丢掉）。成本与样本数成正比，
/// 调用方应先按 [`MAX_DECODE_SECONDS`] 截断并把截断这件事告诉用户。
#[must_use]
pub fn demodulate_frames(samples: &[f32], sample_rate: u32) -> Vec<Vec<u8>> {
  let per_bit = (sample_rate as f64 / BAUD).round() as usize;
  let mut merged: Vec<Vec<u8>> = Vec::new();
  for phase in 0..per_bit {
    // 最贵的一步（Goertzel 扫描）每个相位只做一次；NRZI 初始电平只影响第 1 位，两种都解
    // 一遍只是多一趟廉价的 HDLC 扫描，用来兜「录音恰好从第一个 flag 的第一位开始」。
    let levels = sample_levels(samples, sample_rate, phase);
    for init in [false, true] {
      let mut found = hdlc_decode(&nrzi_decode_init(&levels, init));
      found.retain(|frame| frame.len() >= MIN_AX25_FRAME);
      merge_frames(&mut merged, found);
    }
  }
  merged
}

/// 把一轮解出的帧并进总表：每个不同的帧保留「各轮里出现次数的最大值」份。
///
/// 同一帧在相邻几个相位上都解得出来，只能算一次（去重）；而真实重发的两帧在**同一轮**里
/// 各出现一次，仍要保留两份——所以是多重集的并，不是集合的并。
fn merge_frames(merged: &mut Vec<Vec<u8>>, found: Vec<Vec<u8>>) {
  let mut seen: Vec<(Vec<u8>, usize)> = Vec::new();
  for frame in found {
    let nth = match seen.iter_mut().find(|(f, _)| *f == frame) {
      Some((_, n)) => {
        *n += 1;
        *n
      }
      None => {
        seen.push((frame.clone(), 1));
        1
      }
    };
    let have = merged.iter().filter(|f| **f == frame).count();
    if have < nth {
      merged.push(frame);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::kiss::{ax25_ui_frame, parse_ui_frame};

  #[test]
  fn crc_is_sensitive_to_every_byte() {
    let a = crc16_ax25(b"APRS");
    assert_ne!(crc16_ax25(b"APRT"), a);
    assert_ne!(crc16_ax25(b""), crc16_ax25(b"\x00"));
  }

  #[test]
  fn crc_matches_the_published_check_values() {
    // 往返对拍证明不了 FCS 与真实设备一致（两端错得一样也全绿），所以这里只用外部已知答案。
    //
    // CRC-16/X-25（即 AX.25 / PPP 的 FCS-16）的公开校验值：对 ASCII "123456789" 是 0x906E。
    // 少了最后的取反会得到 0x6F91，生成的音频会被 Direwolf / 硬件 TNC 一律判为 FCS 错误。
    assert_eq!(crc16_ax25(b"123456789"), 0x906E);
    assert_eq!(crc16_ax25(b""), 0x0000);

    // RFC 1662 附录 C：对「载荷 + FCS（低位在前）」整体再算一遍，寄存器必为 0xF0B8，
    // 取反后即 0x0F47——这条与具体载荷无关，任何帧都成立。
    for body in [&b"123456789"[..], b"\x03\xF0hello", &[0u8; 20]] {
      let fcs = crc16_ax25(body);
      let mut framed = body.to_vec();
      framed.push((fcs & 0xFF) as u8);
      framed.push((fcs >> 8) as u8);
      assert_eq!(crc16_ax25(&framed), 0x0F47, "{body:?}");
    }
  }

  #[test]
  fn a_flag_shared_by_two_frames_ends_one_and_starts_the_other() {
    // AX.25 允许一个 0x7E 同时当上一帧的结束与下一帧的起始。把结束标志消耗掉，
    // 第二帧就找不到起始标志、整帧丢失。
    let a = hdlc_encode(b"frame-A-payload!");
    let b = hdlc_encode(b"frame-B-payload!");
    let mut shared = a.clone();
    shared.extend_from_slice(&b[8..]); // 去掉 B 自带的起始标志，复用 A 的结束标志
    assert_eq!(
      hdlc_decode(&shared),
      vec![b"frame-A-payload!".to_vec(), b"frame-B-payload!".to_vec()]
    );
    // 各带独立标志的常规形态不受影响。
    let mut separate = a;
    separate.extend_from_slice(&b);
    assert_eq!(hdlc_decode(&separate).len(), 2);
  }

  #[test]
  fn idle_flags_between_frames_are_not_frames() {
    // 前导 / 空闲填充是一串连续的 0x7E：每对相邻标志之间是空帧，必须被跳过而不是卡住。
    let mut bits = Vec::new();
    for _ in 0..5 {
      bits.extend_from_slice(&flag_bits());
    }
    bits.extend_from_slice(&hdlc_encode(b"payload-after-idle"));
    for _ in 0..5 {
      bits.extend_from_slice(&flag_bits());
    }
    assert_eq!(hdlc_decode(&bits), vec![b"payload-after-idle".to_vec()]);
    // 全是标志：没有任何帧，也不能死循环。
    let only_flags: Vec<bool> = (0..10).flat_map(|_| flag_bits()).collect();
    assert!(hdlc_decode(&only_flags).is_empty());
  }

  #[test]
  fn the_tx_waveform_carries_a_preamble_and_still_decodes() {
    let frame = ax25_ui_frame("APRS", "BG1AAA-7", &[], b">hello").unwrap();
    let bare = modulate_frame(&frame, SAMPLE_RATE);
    let tx = modulate_frame_for_tx(&frame, SAMPLE_RATE);
    // 多出来的就是 (前导 + 尾部) 个 flag，每个 8 位、每位 40 个采样。
    let extra = (TX_PREAMBLE_FLAGS + TX_TAIL_FLAGS) * 8 * 40;
    assert_eq!(tx.len(), bare.len() + extra);
    // 前导 ≈ 213 ms：低于这个长度 VOX 启动会吃掉首帧。
    let preamble_ms = TX_PREAMBLE_FLAGS as f64 * 8.0 / BAUD * 1000.0;
    assert!(preamble_ms >= 200.0, "前导只有 {preamble_ms} ms");
    assert_eq!(demodulate_frames(&tx, SAMPLE_RATE), vec![frame]);
  }

  #[test]
  fn a_retransmitted_frame_is_kept_twice_but_one_frame_is_not_duplicated() {
    // 同一帧会在相邻的好几个位相位上都解得出来，只能算一次；
    // 但真实重发的两帧各自独立，必须保留两份。
    let frame = ax25_ui_frame("APRS", "BG1AAA-7", &[], b">beacon").unwrap();
    let once = modulate_frame_for_tx(&frame, SAMPLE_RATE);
    assert_eq!(
      demodulate_frames(&once, SAMPLE_RATE).len(),
      1,
      "跨相位合并不该把同一帧算成多份"
    );
    let mut twice = once.clone();
    twice.extend_from_slice(&modulate_frame_for_tx(&frame, SAMPLE_RATE));
    assert_eq!(demodulate_frames(&twice, SAMPLE_RATE).len(), 2);
  }

  #[test]
  fn frames_that_only_decode_off_the_first_phase_are_not_lost() {
    // 前一段信号先在相位 0 上解出一帧；后一段整体错开了 20 个采样（半个符号），
    // 它只能在别的相位上解出。只取第一个有结果的相位会把它丢掉。
    let first = ax25_ui_frame("APRS", "BG1AAA-7", &[], b">first").unwrap();
    let second = ax25_ui_frame("APRS", "BG1BBB-9", &[], b">second").unwrap();
    let mut samples = modulate_frame_for_tx(&first, SAMPLE_RATE);
    samples.extend(std::iter::repeat_n(0.0f32, 20));
    samples.extend(modulate_frame_for_tx(&second, SAMPLE_RATE));
    let found = demodulate_frames(&samples, SAMPLE_RATE);
    assert!(found.contains(&first), "第一帧丢了");
    assert!(found.contains(&second), "错开半个符号的第二帧丢了");
  }

  #[test]
  fn noise_does_not_produce_frames() {
    // 白噪声：40 个相位 × 2 个初值的海量候选里不能冒出「碰巧过 CRC」的假帧。
    let mut seed = 0x9E37_79B9u32;
    let noise: Vec<f32> = (0..SAMPLE_RATE as usize * 2)
      .map(|_| {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        ((seed >> 8) as f32 / 16_777_216.0) * 2.0 - 1.0
      })
      .collect();
    assert!(demodulate_frames(&noise, SAMPLE_RATE).is_empty());
  }

  #[test]
  fn hdlc_round_trips_with_stuffing() {
    // 用一帧可能触发位填充的载荷（0xFF 是 8 个连续 1）。
    let frame = [0x03u8, 0xF0, 0xFF, 0xFF, 0x7E, 0x00];
    let bits = hdlc_encode(&frame);
    // 帧内不应出现裸标志（位填充保证）。
    let mid = &bits[8..bits.len() - 8];
    assert!(!mid.windows(8).any(is_flag));
    assert_eq!(hdlc_decode(&bits), vec![frame.to_vec()]);
  }

  #[test]
  fn nrzi_round_trips() {
    let bits = vec![true, false, false, true, true, false, true];
    assert_eq!(nrzi_decode(&nrzi_encode(&bits)), bits);
  }

  #[test]
  fn afsk_round_trips_through_audio() {
    // 帧 → HDLC → NRZI → 音频 → 解调 → NRZI → HDLC → 帧。
    let frame = ax25_ui_frame("APRS", "BG1AAA-7", &[], b">hello").unwrap();
    let samples = modulate_frame(&frame, SAMPLE_RATE);
    let frames = demodulate_frames(&samples, SAMPLE_RATE);
    assert_eq!(frames, vec![frame.clone()]);
    // 解出的帧仍能解析回呼号与信息。
    let ui = parse_ui_frame(&frame).unwrap();
    assert_eq!(ui.source, "BG1AAA");
    assert_eq!(ui.destination, "APRS");
    assert_eq!(ui.info, b">hello");
  }

  #[test]
  fn timing_recovery_decodes_a_phase_shifted_signal() {
    // 前置半位静音（20 采样），位边界整体错开：定时恢复应仍能解出。
    let frame = ax25_ui_frame("APRS", "BG1AAA-7", &[], b">hello").unwrap();
    let samples = modulate_frame(&frame, SAMPLE_RATE);
    let mut shifted = vec![0.0f32; 20];
    shifted.extend_from_slice(&samples);
    assert_eq!(demodulate_frames(&shifted, SAMPLE_RATE), vec![frame]);
  }

  #[test]
  fn timing_recovery_handles_inverted_initial_level() {
    // 翻转全部 NRZI 电平（等价于起始电平取反）：两种初始电平搜索应覆盖。
    let frame = ax25_ui_frame("APRS", "BG1AAA", &[], b"x").unwrap();
    let levels: Vec<bool> = nrzi_encode(&hdlc_encode(&frame))
      .iter()
      .map(|&l| !l)
      .collect();
    let samples = afsk_modulate(&levels, SAMPLE_RATE);
    assert_eq!(demodulate_frames(&samples, SAMPLE_RATE), vec![frame]);
  }

  #[test]
  fn resample_scales_length() {
    let samples: Vec<f32> = (0..48000).map(|i| (i as f32).sin()).collect();
    let down = resample(&samples, 48000, 24000);
    assert_eq!(down.len(), 24000);
    assert_eq!(resample(&down, 24000, 48000).len(), 48000);
    assert_eq!(resample(&samples, 48000, 48000).len(), 48000);
  }

  #[test]
  fn corrupted_audio_drops_the_frame() {
    // 把中间一个符号改成相反音（翻转一位）：FCS 校验失败，帧应被丢弃而不是解出乱码。
    // 注意解调是相位无关的，反转符号（相位翻转）骗不过它，必须改频率。
    let frame = ax25_ui_frame("APRS", "BG1AAA", &[], b"test").unwrap();
    let samples = modulate_frame(&frame, SAMPLE_RATE);
    let levels = afsk_demodulate(&samples, SAMPLE_RATE);
    let per_bit = (SAMPLE_RATE as f64 / BAUD).round() as usize;
    let k = levels.len() / 2;
    let start = k * per_bit;
    let mut corrupted = samples.clone();
    let f = if levels[k] { SPACE_HZ } else { MARK_HZ }; // 相反频率
    for (n, s) in corrupted[start..start + per_bit].iter_mut().enumerate() {
      *s = (std::f64::consts::TAU * f * n as f64 / SAMPLE_RATE as f64).sin() as f32;
    }
    assert!(!demodulate_frames(&corrupted, SAMPLE_RATE).contains(&frame));
  }
}
