//! 真实录音诊断：帧层 → VCDU（VCID / 计数）→ 包（APID 分布）→ 段（质量因子 / JPEG 成败）。
//!
//! ```text
//! cargo run -p ham-web-lrpt --release --example real_iq_report -- 录音.wav
//! ```
use std::collections::BTreeMap;

use ham_web_lrpt::{jpeg, qpsk, rs, vcdus};

fn main() {
  let path = std::env::args()
    .nth(1)
    .expect("用法：real_iq_report <录音.wav>");
  let bytes = std::fs::read(&path).expect("读取录音");
  let (iq, rate, _) = ham_web_apt::parse_wav_interleaved(&bytes).expect("WAV");
  let (frames, _) = qpsk::frames_from_symbols(&qpsk::demodulate(&iq, f64::from(rate)));
  let mut mux = vcdus::MuxStream::new();
  let mut vcids: BTreeMap<u8, usize> = BTreeMap::new();
  let mut counters = Vec::new();
  for f in &frames {
    if let Some(v) = rs::decode_frame(f) {
      let h = vcdus::parse_header(&v);
      *vcids.entry(h.vcid).or_default() += 1;
      counters.push(h.counter);
      mux.push(&v);
    }
  }
  println!("帧 {}，VCID 分布 {vcids:?}", frames.len());
  println!(
    "计数 首 {:?} 末 {:?}，单调 {}",
    counters.first(),
    counters.last(),
    mux.counters_are_monotonic()
  );
  let back: Vec<_> = counters
    .windows(2)
    .filter(|w| w[1] <= w[0])
    .take(5)
    .collect();
  println!("非递增样例 {back:?}");
  let mut apids: BTreeMap<u16, usize> = BTreeMap::new();
  let mut jpeg_ok: BTreeMap<u16, (usize, usize)> = BTreeMap::new();
  let mut ms_sample: BTreeMap<u16, Vec<(u32, u8)>> = BTreeMap::new();
  for (apid, _, payload) in mux.packets() {
    *apids.entry(apid).or_default() += 1;
    if (64..=69).contains(&apid) && payload.len() >= 14 {
      let e = jpeg_ok.entry(apid).or_default();
      e.1 += 1;
      if jpeg::decode_segment(&payload[14..], payload[13]).is_some() {
        e.0 += 1;
      }
      let ms = u32::from_be_bytes([payload[2], payload[3], payload[4], payload[5]]);
      let s = ms_sample.entry(apid).or_default();
      if s.len() < 6 {
        s.push((ms, payload[8]));
      }
    }
  }
  println!("APID 分布 {apids:?}");
  println!("段 JPEG 成功 / 总数 {jpeg_ok:?}");
  println!("段 (ms, MCUN) 样例 {ms_sample:?}");
}
