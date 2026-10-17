//! 整段录音的解码：按时隙切片 → 逐时隙跑 waterfall / 候选 / LDPC → 汇总、去重。
//!
//! 这段流程原先写在 Worker 里，但 Worker 依赖 `JsValue`，原生单测跑不了；放在内核 crate
//! 才能用合成信号覆盖「多时隙」「哈希表还原」「不支持的消息类型」这几个只有整段流程才有的行为。

use std::collections::HashSet;

use crate::decode::decode_waterfall;
use crate::message::{HashTable, Message, decode as decode_message};
use crate::monitor::{Monitor, MonitorConfig, Protocol};

/// 解码器唯一支持的采样率：`block_size`、FFT 长度与 bin 划分都由它推出。
pub const SAMPLE_RATE: u32 = 12_000;
/// 搜索的音频频带（Hz）。
const F_MIN: f32 = 200.0;
const F_MAX: f32 = 3000.0;
/// 时间 / 频率过采样倍数。
const TIME_OSR: usize = 2;
const FREQ_OSR: usize = 2;
/// 每个时隙最多尝试的候选数、同步分数下限、LDPC 迭代上限（ft8_lib demo 的取值）。
const NUM_CANDIDATES: usize = 20;
const MIN_SCORE: i32 = 10;
const MAX_ITERATIONS: usize = 25;

/// 一条解出的消息。
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedMessage {
  pub text: String,
  pub freq_hz: f32,
  /// 距录音开头的秒数（已加上所在时隙的起点）。
  pub time_sec: f32,
  /// 同步强度（dB）：Costas 同步音比相邻频点 / 相邻符号平均高出多少。
  ///
  /// **不是 WSJT-X 口径的 SNR**（那个以 2500 Hz 带宽的噪声为参照）；它只反映同步峰有多尖，
  /// 用来给结果排序和粗判强弱，不要当成信噪比展示。
  pub sync_db: f32,
}

/// 一段录音的解码结果。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Recording {
  /// 解出文本的消息，按时隙先后、同一时隙内按同步分数从高到低。
  pub messages: Vec<DecodedMessage>,
  /// CRC 通过、但消息类型本解码器还没有字段解析的条数（DXpedition / EU VHF / Field Day 等）。
  /// 它们不进 `messages`：否则界面上会多出只有频率的空行。
  pub unsupported: u32,
  /// 录音被切成了多少个时隙逐个解码。
  pub slots: usize,
}

/// 录音不满足解码前提。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingError {
  /// 采样率不是 [`SAMPLE_RATE`]（附实际采样率）。
  SampleRate(u32),
}

/// 解码一段单声道录音。
///
/// 录音按协议的时隙长度（FT8 15 s / FT4 7.5 s）从头切片，每个时隙独立建 waterfall 解码；
/// 呼号哈希表则**跨时隙共享**。所以录音应从时隙边界附近开始：候选搜索只容忍起点偏差约
/// −1.6 s～+3.2 s（FT8），偏得更多的信号会跨在两个时隙之间而解不出。
///
/// # Errors
/// 采样率不是 12000 Hz 时返回 [`RecordingError::SampleRate`]。
pub fn decode_recording(
  samples: &[f32],
  sample_rate: u32,
  protocol: Protocol,
) -> Result<Recording, RecordingError> {
  if sample_rate != SAMPLE_RATE {
    return Err(RecordingError::SampleRate(sample_rate));
  }
  let cfg = MonitorConfig {
    f_min: F_MIN,
    f_max: F_MAX,
    sample_rate: sample_rate as usize,
    time_osr: TIME_OSR,
    freq_osr: FREQ_OSR,
    protocol,
  };
  let slot_time = protocol.slot_time();
  let slot_samples = ((slot_time * sample_rate as f32) as usize).max(1);

  // 第一遍：逐时隙解出原始 payload。每个时隙新建 `Monitor`：滑窗里还留着上一时隙的尾巴，
  // 不清掉就会污染下一时隙开头的几个符号。
  let slots: Vec<Vec<(Message, f32, f32, i32)>> = samples
    .chunks(slot_samples)
    .map(|segment| {
      let mut mon = Monitor::new(&cfg);
      for frame in segment.chunks(mon.block_size) {
        // 最后不足一个 block 的尾巴 `process` 会拒绝，也不会再有后续，直接收工。
        if !mon.process(frame) {
          break;
        }
      }
      decode_waterfall(
        &mon.wf,
        mon.symbol_period,
        mon.min_bin,
        NUM_CANDIDATES,
        MIN_SCORE,
        MAX_ITERATIONS,
      )
    })
    .collect();

  // 第二遍：先把所有消息过一遍，只为填呼号哈希表。type 4 消息里的 12 位呼号要靠同一会话里
  // 另一条标准消息还原，而那条消息可能排在它后面（候选按分数排、时隙按时间排）。
  let mut hash = HashTable::new();
  for (msg, ..) in slots.iter().flatten() {
    let _ = decode_message(msg, &mut hash);
  }

  // 第三遍：出文本。去重只在**同一时隙内**做 —— 同一信号会被相邻的 freq_sub / time_sub
  // 候选反复命中；而同一台站在下一个时隙重复发同样的话是两条真实的接收，要各列一条。
  let mut out = Recording {
    slots: slots.len(),
    ..Recording::default()
  };
  for (index, decoded) in slots.iter().enumerate() {
    let slot_start = index as f32 * slot_time;
    let mut seen = HashSet::new();
    for (msg, freq_hz, time_sec, score) in decoded {
      if !seen.insert(msg.payload) {
        continue;
      }
      match decode_message(msg, &mut hash) {
        Ok(text) if !text.is_empty() => out.messages.push(DecodedMessage {
          text,
          freq_hz: *freq_hz,
          time_sec: slot_start + time_sec,
          sync_db: *score as f32 * 0.5,
        }),
        _ => out.unsupported += 1,
      }
    }
  }
  Ok(out)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::audio::{synthesize, synthesize_payload};

  /// FT8 一个时隙的采样数（15 s × 12000 Hz）。
  const SLOT: usize = 180_000;

  fn signal(text: &str, f0: f32) -> Vec<f32> {
    synthesize(text, f0, false, &mut HashTable::new()).expect("编码失败")
  }

  fn texts(r: &Recording) -> Vec<&str> {
    r.messages.iter().map(|m| m.text.as_str()).collect()
  }

  #[test]
  fn wrong_sample_rate_is_refused() {
    assert_eq!(
      decode_recording(&[0.0; 16], 8_000, Protocol::Ft8),
      Err(RecordingError::SampleRate(8_000))
    );
  }

  #[test]
  fn empty_recording_has_no_slots() {
    let r = decode_recording(&[], SAMPLE_RATE, Protocol::Ft8).unwrap();
    assert_eq!(r, Recording::default());
  }

  /// 超过一个时隙的录音必须逐时隙解码，而不是在第一个时隙收满后静默丢掉后面的部分。
  #[test]
  fn long_recording_is_decoded_slot_by_slot() {
    let mut samples = signal("CQ K7IHZ DM43", 1000.0);
    samples.extend(signal("CQ JA LB2JK JO59", 2000.0));
    assert_eq!(samples.len(), 2 * SLOT);

    let r = decode_recording(&samples, SAMPLE_RATE, Protocol::Ft8).unwrap();
    assert_eq!(r.slots, 2);
    assert_eq!(texts(&r), ["CQ K7IHZ DM43", "CQ JA LB2JK JO59"]);
    // 第二条在第二个时隙：时间要加上时隙起点，而不是都落在 0–15 s。
    assert!(r.messages[0].time_sec < 15.0, "{:?}", r.messages[0]);
    assert!(r.messages[1].time_sec >= 15.0, "{:?}", r.messages[1]);
    assert!((r.messages[1].freq_hz - 2000.0).abs() < 20.0);
  }

  /// 去重只在时隙内做：同一台站下一个时隙重复发同样的话是两条真实接收。
  #[test]
  fn same_message_in_two_slots_is_listed_twice() {
    let one = signal("CQ K7IHZ DM43", 1000.0);
    let samples = [one.as_slice(), one.as_slice()].concat();
    let r = decode_recording(&samples, SAMPLE_RATE, Protocol::Ft8).unwrap();
    assert_eq!(texts(&r), ["CQ K7IHZ DM43", "CQ K7IHZ DM43"]);
  }

  /// type 4 的 12 位呼号靠同会话另一条标准消息还原，与两条消息的先后无关。
  #[test]
  fn hash_table_is_learned_before_text_is_rendered() {
    let std_first = [
      signal("CQ K7IHZ/R DM43", 1000.0),
      signal("K7IHZ/R W1AW/P 73", 1500.0),
    ]
    .concat();
    let type4_first = [
      signal("K7IHZ/R W1AW/P 73", 1500.0),
      signal("CQ K7IHZ/R DM43", 1000.0),
    ]
    .concat();
    for samples in [std_first, type4_first] {
      let r = decode_recording(&samples, SAMPLE_RATE, Protocol::Ft8).unwrap();
      let all = texts(&r);
      assert!(all.contains(&"<K7IHZ/R> W1AW/P 73"), "{all:?}");
      assert!(all.contains(&"CQ K7IHZ/R DM43"), "{all:?}");
    }
  }

  /// CRC 合法但本解码器不解析的消息类型：只计数，不进消息列表（否则是只有频率的空行）。
  #[test]
  fn unsupported_message_types_are_counted_not_listed() {
    // i3 = 0、n3 = 1（DXpedition / Fox-Hound）；最后 3 位是 77 位 payload 之外的补零。
    let payload = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x00, 0x40];
    let samples = synthesize_payload(&payload, 1200.0, false).expect("合成失败");
    let r = decode_recording(&samples, SAMPLE_RATE, Protocol::Ft8).unwrap();
    assert_eq!(r.slots, 1);
    assert!(r.messages.is_empty(), "{:?}", r.messages);
    assert_eq!(r.unsupported, 1);
  }

  #[test]
  fn silence_decodes_nothing() {
    let r = decode_recording(&vec![0.0; 2 * SLOT], SAMPLE_RATE, Protocol::Ft8).unwrap();
    assert_eq!(r.slots, 2);
    assert!(r.messages.is_empty());
    assert_eq!(r.unsupported, 0);
  }
}
