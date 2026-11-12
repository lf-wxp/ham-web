//! 真实录音上的解码验证。录音不进仓库（版权不明），用 `MFSK_REAL_DIR` 指向存放它们的目录。
//! 用例都标了 `#[ignore]`（默认 `cargo test` 里显示为 ignored，而不是悄悄「通过」）；
//! 显式运行时没设目录或缺文件就 panic，不会空转。
//!
//! 样本来源（都是业余无线电社区公开提供的「标准测试报文」录音）：
//! - `gm_mfsk16.wav` / `gm_mfsk8.wav`：gMFSK 官方站点 <http://gmfsk.connect.fi/sounds.html>
//!   （MFSK 的原作者 OH2BNS 的实现，11025 Hz 单声道 WAV）；
//! - `g4wnc_MFSK16-test-message.wav` / `g4wnc_MFSK-32-test-message.wav` /
//!   `g4wnc_MFSK-64-test-message.wav`：G4WNC 的 <https://photobyte.org/raspberry-pi/data-sound-samples/>
//!   （原件是 MP3，用 `ffmpeg -ac 1 -ar 8000 -c:a pcm_s16le` 转成 WAV；MP3 的有损压缩也一并被验证了）。
//!
//! 报文都是 `the quick brown fox jumps over the lazy dog 1234567890`（G4WNC 的重复三遍）。
//!
//! 另有 `fw_MFSK<模式>.wav`：**fldigi 4.2.13 自己的发端**（编译成 WebAssembly 的原版代码，来自
//! <https://github.com/crashdemons-external/fldigi-web>）生成的 9 个子模式录音，生成方法见
//! `reference/fldigi_web/README.md`。这是「别人家的发端 → 本 crate 的收端」的验证；反方向
//! （本 crate 的发端 → fldigi 自己的接收机）由同一个目录里的脚本验证。
//!
//! ```text
//! MFSK_REAL_DIR=/tmp/real cargo test -p ham-web-mfsk --release --test real_recordings -- --include-ignored
//! ```

use std::path::PathBuf;

use ham_web_mfsk::{decode_audio, parse_mode};

const TEXT: &str = "the quick brown fox jumps over the lazy dog 1234567890";

/// 读样本；没设目录或没这个文件就 panic（这些用例是 `#[ignore]` 的，显式运行就该有录音）。
fn load(name: &str) -> (Vec<f32>, u32) {
  let dir = PathBuf::from(
    std::env::var_os("MFSK_REAL_DIR").expect("请设置 MFSK_REAL_DIR 指向存放录音的目录"),
  );
  let path = dir.join(name);
  let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("读不到 {}：{e}", path.display()));
  ham_web_wav::parse(&bytes).expect("WAV 解析")
}

fn check(file: &str, mode: &str, repeats: usize, expect_center: f64) {
  let (samples, rate) = load(file);
  let mode = parse_mode(mode).unwrap();
  // 不给中心频率：同时验证自动检测。
  let d =
    decode_audio(&samples, rate, mode, None, false).unwrap_or_else(|| panic!("{file}：应解出信号"));
  assert!(
    (d.center_hz - expect_center).abs() <= mode.baud() / 8.0,
    "{file}：中心频率 {} Hz，期望 {expect_center}",
    d.center_hz
  );
  assert_eq!(
    d.text.matches(TEXT).count(),
    repeats,
    "{file}：{:?}",
    d.text
  );
}

#[test]
#[ignore = "需要 MFSK_REAL_DIR 指向真实录音目录（见本文件头注释）"]
fn gmfsk_mfsk16_sample() {
  check("gm_mfsk16.wav", "16", 1, 1000.0);
}

#[test]
#[ignore = "需要 MFSK_REAL_DIR 指向真实录音目录（见本文件头注释）"]
fn gmfsk_mfsk8_sample_uses_interleaver_depth_10() {
  check("gm_mfsk8.wav", "8", 1, 1000.0);
  let (samples, rate) = load("gm_mfsk8.wav");
  let d = decode_audio(&samples, rate, parse_mode("8").unwrap(), None, false).unwrap();
  assert_eq!(d.depth, 10, "gMFSK 的 MFSK8 录音要用交织深度 10 才能译出");
}

#[test]
#[ignore = "需要 MFSK_REAL_DIR 指向真实录音目录（见本文件头注释）"]
fn g4wnc_mfsk16_sample() {
  check("g4wnc_MFSK16-test-message.wav", "16", 3, 734.375);
}

#[test]
#[ignore = "需要 MFSK_REAL_DIR 指向真实录音目录（见本文件头注释）"]
fn g4wnc_mfsk32_sample() {
  check("g4wnc_MFSK-32-test-message.wav", "32", 3, 734.375);
}

#[test]
#[ignore = "需要 MFSK_REAL_DIR 指向真实录音目录（见本文件头注释）"]
fn g4wnc_mfsk64_sample() {
  check("g4wnc_MFSK-64-test-message.wav", "64", 3, 734.375);
}

/// fldigi 自己的发端生成的各子模式录音（中心 1500 Hz）：每个子模式都要逐字解出报文。
#[test]
#[ignore = "需要 MFSK_REAL_DIR 指向真实录音目录（见本文件头注释）"]
fn fldigi_encoder_output_for_every_mode() {
  for mode in ham_web_mfsk::MODES {
    let file = format!("fw_MFSK{}.wav", mode.name);
    let (samples, rate) = load(&file);
    let d = decode_audio(&samples, rate, mode, None, false)
      .unwrap_or_else(|| panic!("{file}：应解出信号"));
    assert!(
      (d.center_hz - 1500.0).abs() <= mode.baud() / 8.0,
      "{file}：中心频率 {} Hz，期望 1500",
      d.center_hz
    );
    assert!(d.text.contains(TEXT), "{file}：{:?}", d.text);
    // fldigi 4.2.13 的 MFSK4 / 8 交织深度是 5（gMFSK 的录音是 10，见上面那个用例）。
    if matches!(mode.name, "4" | "8") {
      assert_eq!(d.depth, 5, "{file}");
    }
  }
}
