//! FT8 / FT4 的 GFSK 音频合成。
//!
//! 转写自 MIT 许可的 ft8_lib `demo/gen_ft8.c`：把信道符号合成 8-FSK 的 GFSK 音频，
//! 用于测试（编码 → 合成 → 解码的往返对拍）与将来可能的「发」方向。

use crate::{FT4_NN, FT8_NN, ft4_encode, ft8_encode};

/// GFSK 平滑因子（FT8 = 2，FT4 = 1）。
const FT8_SYMBOL_BT: f32 = 2.0;
const FT4_SYMBOL_BT: f32 = 1.0;
/// `π·√(2/ln2)`。
const GFSK_CONST_K: f32 = 5.336_446;

/// erf 近似（Abramowitz-Stegun 7.1.26，误差 ~1e-7）。
fn erf(x: f32) -> f32 {
  // f64 计算，避免 f32 精度不足（A&S 系数有 9-10 位有效数字）。
  let x = f64::from(x);
  let sign = if x < 0.0 { -1.0 } else { 1.0 };
  let x = x.abs();
  let t = 1.0 / (1.0 + 0.327_591_1 * x);
  let y = 1.0
    - (((((1.061_405_429 * t - 1.453_152_027) * t) + 1.421_413_741) * t - 0.284_496_736) * t
      + 0.254_829_592)
      * t
      * (-x * x).exp();
  (sign * y) as f32
}

/// GFSK 平滑脉冲（截断到 3 个符号长度）。
fn gfsk_pulse(n_spsym: usize, symbol_bt: f32) -> Vec<f32> {
  (0..3 * n_spsym)
    .map(|i| {
      let t = i as f32 / n_spsym as f32 - 1.5;
      let arg1 = GFSK_CONST_K * symbol_bt * (t + 0.5);
      let arg2 = GFSK_CONST_K * symbol_bt * (t - 0.5);
      (erf(arg1) - erf(arg2)) / 2.0
    })
    .collect()
}

/// 把信道符号合成 GFSK 音频（`f0` 是符号 0 的基频，Hz）。
///
/// 空符号序列、或采样率 × 符号周期不足 1 个采样时返回空 `Vec`（下面要靠 `symbols[0]` /
/// `n_spsym` 做首尾扩展，不能在没有符号时硬算）。
pub fn synth_gfsk(
  symbols: &[u8],
  f0: f32,
  symbol_bt: f32,
  symbol_period: f32,
  signal_rate: usize,
) -> Vec<f32> {
  let n_spsym = (0.5 + signal_rate as f32 * symbol_period) as usize;
  let n_sym = symbols.len();
  if n_sym == 0 || n_spsym == 0 {
    return Vec::new();
  }
  let n_wave = n_sym * n_spsym;
  let hmod = 1.0f32;

  let dphi_peak = 2.0 * std::f32::consts::PI * hmod / n_spsym as f32;
  // 首尾各扩展一个符号（dummy）。
  let mut dphi = vec![2.0 * std::f32::consts::PI * f0 / signal_rate as f32; n_wave + 2 * n_spsym];
  let pulse = gfsk_pulse(n_spsym, symbol_bt);

  for (i, &sym) in symbols.iter().enumerate() {
    let ib = i * n_spsym;
    for j in 0..3 * n_spsym {
      dphi[j + ib] += dphi_peak * f32::from(sym) * pulse[j];
    }
  }
  // 首尾 dummy 符号（等于首尾符号）。
  for j in 0..2 * n_spsym {
    dphi[j] += dphi_peak * pulse[j + n_spsym] * f32::from(symbols[0]);
    dphi[j + n_sym * n_spsym] += dphi_peak * pulse[j] * f32::from(symbols[n_sym - 1]);
  }

  let mut signal = Vec::with_capacity(n_wave);
  let mut phi = 0.0f32;
  for k in 0..n_wave {
    signal.push(phi.sin());
    phi = (phi + dphi[k + n_spsym]) % (2.0 * std::f32::consts::PI);
  }

  // 首尾包络整形（各 n_spsym/8 采样淡入淡出）。
  let n_ramp = n_spsym / 8;
  for i in 0..n_ramp {
    let env = (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (2.0 * n_ramp as f32)).cos()) / 2.0;
    signal[i] *= env;
    signal[n_wave - 1 - i] *= env;
  }
  signal
}

/// 把一条文本消息合成 15 秒（FT8）或 7.5 秒（FT4）的音频（12000 Hz），基频 `f0` Hz。
///
/// 返回 `None` 当消息无法编码。用于「编出去再解回来」的往返测试。
pub fn synthesize(
  text: &str,
  f0: f32,
  is_ft4: bool,
  hash: &mut crate::HashTable,
) -> Option<Vec<f32>> {
  let msg = crate::encode(text, hash).ok()?;
  synthesize_payload(&msg.payload, f0, is_ft4)
}

/// 把 77 位 payload 直接合成一个时隙长度的音频（12000 Hz），基频 `f0` Hz。
///
/// 与 [`synthesize`] 的区别是跳过文本编码：测试要造「CRC 合法、但本解码器不解析的消息类型」
/// （DXpedition / Field Day 等）时只能从 payload 起步。返回 `None` 当合成长度不够。
pub fn synthesize_payload(payload: &[u8; 10], f0: f32, is_ft4: bool) -> Option<Vec<f32>> {
  let (tones, symbol_bt, symbol_period, slot_time) = if is_ft4 {
    let mut t = [0u8; FT4_NN];
    ft4_encode(payload, &mut t);
    (t.to_vec(), FT4_SYMBOL_BT, 0.048f32, 7.5f32)
  } else {
    let mut t = [0u8; FT8_NN];
    ft8_encode(payload, &mut t);
    (t.to_vec(), FT8_SYMBOL_BT, 0.160f32, 15.0f32)
  };

  let signal_rate = 12000usize;
  let num_samples = (0.5 + tones.len() as f32 * symbol_period * signal_rate as f32) as usize;
  let num_silence = (slot_time * signal_rate as f32 - num_samples as f32) as usize / 2;
  let mut signal = vec![0.0f32; num_silence * 2 + num_samples];
  let wave = synth_gfsk(&tones, f0, symbol_bt, symbol_period, signal_rate);
  // `n_spsym` 是四舍五入出来的，与 `num_samples` 可能差 1 个采样：取够用的前缀，
  // 不够就放弃（宁可返回 None 也不越界拷贝）。
  if wave.len() < num_samples {
    return None;
  }
  signal[num_silence..num_silence + num_samples].copy_from_slice(&wave[..num_samples]);
  Some(signal)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::decode_waterfall;
  use crate::monitor::{Monitor, MonitorConfig, Protocol};
  use crate::{HashTable, decode};

  /// 完整往返：消息 → 编码 → GFSK 合成 → waterfall → 候选 → 似然 → LDPC → CRC → 消息。
  /// 证明「编得出去、解得回来」，覆盖整条解码链。
  #[test]
  fn synthesized_signal_decodes_back_to_the_same_text() {
    for (text, expect) in [
      ("CQ K7IHZ DM43", "CQ K7IHZ DM43"),
      ("CQ EA8/G5LSI", "CQ EA8/G5LSI"),
      ("R2RFE/P EA8/G5LSI R+12", "R2RFE/P <EA8/G5LSI> R+12"),
    ] {
      let mut hash = HashTable::new();
      let signal = synthesize(text, 1000.0, false, &mut hash).expect("编码失败");
      let cfg = MonitorConfig {
        f_min: 200.0,
        f_max: 3000.0,
        sample_rate: 12000,
        time_osr: 2,
        freq_osr: 2,
        protocol: Protocol::Ft8,
      };
      let mut mon = Monitor::new(&cfg);
      for chunk in signal.chunks(mon.block_size) {
        if !mon.process(chunk) {
          break;
        }
      }
      let decoded = decode_waterfall(&mon.wf, mon.symbol_period, mon.min_bin, 10, 10, 25);
      let hit = decoded.iter().any(|(msg, freq, _, _)| {
        if let Ok(t) = decode(msg, &mut hash) {
          t == expect && (*freq - 1000.0).abs() < 20.0
        } else {
          false
        }
      });
      let got: Vec<_> = decoded
        .iter()
        .map(|(m, f, _, _)| (decode(m, &mut HashTable::new()).unwrap_or_default(), f))
        .collect();
      assert!(hit, "{text:?} 未解回，实际 {got:?}");
    }
  }
}
