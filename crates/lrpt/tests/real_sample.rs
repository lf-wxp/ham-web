//! 与 **lrpt-encoder**（MIT，独立 Rust 实现，输出可被 SatDump 解码）的互操作对拍。
//!
//! 夹具（`samples/`）：
//! - `lrpt_encoder_head.wav`：lrpt-encoder 对 `a.png` 编码的 288 kHz 立体声 I/Q
//!   的**前 4 秒**（完整输出 ~110 MB / 95.46 s，进不了仓库 —— 样本 README 写明
//!   重新生成命令）；
//! - `lrpt_encoder_cadus.bin`：同一运行导出的前 200 帧原始 CADU（编码器
//!   `--dump-cadu-stream` 的真值）。
//!
//! 断言：本 crate 的接收链解出 ≥ 100 帧，且与编码器真值**逐字节一致**
//! （每帧 1020 字节 RS 码字 —— 解码端把扰码撤销后与 dump 的 `[4..]` 比较）。

use ham_web_lrpt::prand::PRAND;
use ham_web_lrpt::{DATA_LEN, decode_stream288};

fn read_wav_iq(path: &str) -> Vec<f32> {
  let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("读取 {path} 失败：{e}"));
  assert_eq!(&bytes[0..4], b"RIFF");
  assert_eq!(&bytes[8..12], b"WAVE");
  let mut off = 12usize;
  while off + 8 <= bytes.len() {
    let id = &bytes[off..off + 4];
    let size = u32::from_le_bytes(bytes[off + 4..off + 8].try_into().unwrap()) as usize;
    if id == b"data" {
      let mut out = Vec::with_capacity(size / 2);
      for k in (0..size).step_by(2) {
        let v = i16::from_le_bytes(bytes[off + 8 + k..off + 10 + k].try_into().unwrap());
        out.push(f32::from(v) / 32768.0);
      }
      return out;
    }
    off += 8 + size;
  }
  unreachable!("WAV 缺少 data 块");
}

#[test]
fn decodes_the_lrpt_encoder_signal_byte_exactly() {
  let iq = read_wav_iq(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/samples/lrpt_encoder_head.wav"
  ));
  let frames = decode_stream288(&iq);
  // 4 秒 ≈ 35 帧（72k 符号/s ÷ 2 ÷ 1024 字节/帧），首尾各被收敛区吃掉 ~1 帧。
  assert!(
    frames.len() >= 30,
    "4 秒样本应解出 30+ 帧，得到 {}",
    frames.len()
  );

  let dump = std::fs::read(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/samples/lrpt_encoder_cadus.bin"
  ))
  .expect("CADU 真值夹具");
  // 首帧被滑窗 Viterbi 的收敛区吃掉，比对从真值的第 2 帧起。
  let mut ok = 0usize;
  let mut total = 0usize;
  for (decoded, cadu) in frames
    .iter()
    .zip(dump.chunks(1024).skip(1).filter(|c| c.len() == 1024))
  {
    let mut expected = [0u8; DATA_LEN];
    expected.copy_from_slice(&cadu[4..]);
    for i in 0..DATA_LEN {
      expected[i] ^= PRAND[i % PRAND.len()];
    }
    total += 1;
    assert_eq!(decoded, &expected, "第 {total} 帧应与编码器真值逐字节一致");
    ok += 1;
  }
  assert!(ok >= 30, "至少比对 30 帧，实际 {ok}");
}
