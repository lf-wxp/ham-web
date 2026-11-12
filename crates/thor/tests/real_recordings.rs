//! fldigi 自己的发端生成的 THOR 录音上的解码验证。录音不进仓库，用 `THOR_REAL_DIR` 指向存放目录。
//! 用例标了 `#[ignore]`（默认 `cargo test` 里显示为 ignored，而不是悄悄「通过」）；
//! 显式运行时没设目录或缺文件就 panic，不会空转。
//!
//! 录音来自 **fldigi 4.2.13 的原版发端**（编译成 WebAssembly，见
//! <https://github.com/crashdemons-external/fldigi-web>），生成方法见 `../mfsk/reference/fldigi_web/README.md`：
//! 中心 1500 Hz，报文 `the quick brown fox jumps over the lazy dog 1234567890`，文件名 `fw_THOR<模式>.wav`
//! （THOR micro 是 `fw_THORM.wav`）。
//!
//! ```text
//! THOR_REAL_DIR=/tmp/fw cargo test -p ham-web-thor --release --test real_recordings -- --include-ignored
//! ```

use std::path::PathBuf;

use ham_web_dsp::warp;
use ham_web_thor::{MODES, decode_audio, parse_mode};

const TEXT: &str = "the quick brown fox jumps over the lazy dog 1234567890";

/// 读样本；没设目录或没这个文件就 panic（用例是 `#[ignore]` 的，显式运行就该有录音）。
fn load(name: &str) -> (Vec<f32>, u32) {
  let dir = PathBuf::from(
    std::env::var_os("THOR_REAL_DIR").expect("请设置 THOR_REAL_DIR 指向存放录音的目录"),
  );
  let path = dir.join(name);
  let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("读不到 {}：{e}", path.display()));
  ham_web_wav::parse(&bytes).expect("WAV 解析")
}

#[test]
#[ignore = "需要 THOR_REAL_DIR 指向 fldigi 生成的录音目录（见本文件头注释）"]
fn fldigi_encoder_output_for_every_mode() {
  for mode in MODES {
    let tag = if mode.name == "micro" { "M" } else { mode.name };
    let file = format!("fw_THOR{tag}.wav");
    let (samples, rate) = load(&file);
    // 不给中心频率：同时验证自动检测。
    let d = decode_audio(&samples, rate, mode, None, false)
      .unwrap_or_else(|| panic!("{file}：应解出信号"));
    assert!(
      (d.center_hz - 1500.0).abs() <= mode.spacing() / 8.0 + 1.0,
      "{file}：中心频率 {} Hz，期望 1500",
      d.center_hz
    );
    assert!(d.text.contains(TEXT), "{file}：{:?}", d.text);
  }
}

/// 时钟误差与时变转速的组合：`(ratio0, slope)`。
const WARPS: &[(f64, f64)] = &[
  (1e-3, 0.0),
  (-1.5e-3, 0.0),
  (0.0, 2e-5),
  (0.0, -2e-5),
  (5e-4, 3e-5),
];

#[test]
#[ignore = "需要 THOR_REAL_DIR 指向 fldigi 生成的录音目录（见本文件头注释）"]
fn fldigi_encoder_output_survives_clock_and_frequency_drift() {
  // `fwlong_THOR*.wav`：fldigi 的原版发端、报文重复 8 遍（`gen_all.sh` 生成）。漂移由重采样加上去，
  // 不跟踪时这些都会丢掉 1–4 遍报文。
  for (file, name) in [("fwlong_THOR16.wav", "16"), ("fwlong_THOR8.wav", "8")] {
    let (samples, rate) = load(file);
    let mode = parse_mode(name).unwrap();
    for &(ratio0, slope) in WARPS {
      let audio = warp(&samples, f64::from(rate), ratio0, slope);
      let d = decode_audio(&audio, rate, mode, None, false)
        .unwrap_or_else(|| panic!("{file} ratio0 {ratio0} slope {slope}：应解出信号"));
      assert_eq!(
        d.text.matches(TEXT).count(),
        8,
        "{file} ratio0 {ratio0} slope {slope}: {:?}",
        d.text
      );
      let secs = samples.len() as f64 / f64::from(rate);
      let expected_ppm = (ratio0 + slope * secs / 2.0) * 1e6;
      assert!(
        (d.clock_ppm - expected_ppm).abs() < 150.0,
        "{file} ratio0 {ratio0} slope {slope}：时钟误差估成 {} ppm，期望约 {expected_ppm}",
        d.clock_ppm
      );
    }
  }
}
