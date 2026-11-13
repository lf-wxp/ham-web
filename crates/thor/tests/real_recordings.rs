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

/// fldigi 4.2.13 自己的发端发的图片（`gen_all.sh` 生成 `fwpic_*.wav`，旁边的 `.rgb` 是原图；
/// `fldigi_web/gen_pic.mjs`）。门限是量过之后定的：同一段音频 fldigi 自己的接收机（`dec_pic.mjs`）的平均误差是
/// 2.2 / 32.5 / 8.9 / 39.2 / 23.4 / 40.7 / 32.3 / 21.8 / 32.5 / 30.4，本仓是 2.0 / 8.0 / 2.6 / 5.0 / 4.1 / 14.0 / 4.1 / 4.6 / 8.0 / 7.5
/// —— fldigi 实时接收的像素相位对不准，本仓整段录音里找同步音、边界对得准，误差小得多。
/// 不给中心频率：同时验证由同步音推出中心频率。
#[test]
#[ignore = "需要 THOR_REAL_DIR 指向 fldigi 生成的录音目录（见本文件头注释）"]
fn fldigi_encoder_pictures() {
  for (file, mode, color, limit) in [
    ("fwpic_t16_color_bands", "16", true, 4.0),
    ("fwpic_t16_color_ramp", "16", true, 11.0),
    ("fwpic_t16_gray_ramp", "16", false, 5.0),
    ("fwpic_t25_ramp", "25", true, 8.0),
    ("fwpic_t32_ramp", "32", true, 7.0),
    ("fwpic_t11_ramp", "11", true, 18.0),
    ("fwpic_t22_ramp", "22", true, 11.0),
    ("fwpic_t44_ramp", "44", true, 7.0),
    ("fwpic_t56_ramp", "56", true, 8.0),
    ("fwpic_t16_big_ramp", "16", true, 11.0),
  ] {
    let (samples, rate) = load(&format!("{file}.wav"));
    let want = std::fs::read(
      std::path::PathBuf::from(std::env::var_os("THOR_REAL_DIR").unwrap())
        .join(format!("{file}.wav.rgb")),
    )
    .unwrap_or_else(|e| panic!("读不到 {file}.wav.rgb：{e}"));
    let mode = parse_mode(mode).unwrap();
    let d = decode_audio(&samples, rate, mode, None, false)
      .unwrap_or_else(|| panic!("{file}：应解出信号"));
    assert_eq!(d.pictures.len(), 1, "{file}：{:?}", d.text);
    let p = &d.pictures[0];
    assert_eq!(p.color, color, "{file}");
    assert_eq!(p.rgb.len(), want.len(), "{file}");
    let error = p
      .rgb
      .iter()
      .zip(&want)
      .map(|(&a, &b)| f64::from(i32::from(a).abs_diff(i32::from(b))))
      .sum::<f64>()
      / want.len() as f64;
    assert!(error < limit, "{file}：平均误差 {error}，门限 {limit}");
  }
}

/// fldigi 4.2.13 **桌面版**的两次真实图片发射（THOR16 与 THOR25，59 × 74 彩色，四块纯色），录音取自
/// `crashdemons-external/fldigi-web` 的 `tests/fixtures/fldigi-picture-thor16.wav` / `-thor25.wav`（拷到 `THOR_REAL_DIR`）。
/// 与它自己的测试同一口径：四个象限中心的颜色与预期相差不超过 2，整幅图与预期图的平均误差小于 3。
#[test]
#[ignore = "需要 THOR_REAL_DIR 指向 fldigi 生成的录音目录（见本文件头注释）"]
fn fldigi_desktop_picture_recordings() {
  for (file, mode) in [
    ("fldigi-picture-thor16.wav", "16"),
    ("fldigi-picture-thor25.wav", "25"),
  ] {
    let (samples, rate) = load(file);
    let mode = parse_mode(mode).unwrap();
    for center in [Some(1500.0), None] {
      let d = decode_audio(&samples, rate, mode, center, false).expect("应解出信号");
      assert_eq!(d.pictures.len(), 1, "{file} {center:?}：{:?}", d.text);
      let p = &d.pictures[0];
      assert_eq!((p.width, p.height, p.color), (59, 74, true), "{file}");
      // 与 fldigi-web 的测试同一个判法：`x < 宽 / 2` 是浮点比较（59 / 2 = 29.5，第 29 列属于左半）。
      let expected = |x: usize, y: usize| {
        let (left, top) = ((x as f64) < 59.0 / 2.0, (y as f64) < 74.0 / 2.0);
        [
          if left { 220 } else { 40 },
          if top { 180 } else { 70 },
          if left { 30 } else { 210 },
        ]
      };
      for (xr, yr) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
        let (x, y) = ((xr * 59.0) as usize, (yr * 74.0) as usize);
        for (c, &want) in expected(x, y).iter().enumerate() {
          let got = i32::from(p.rgb[3 * (y * 59 + x) + c]);
          assert!(
            (got - want).abs() <= 2,
            "{file} {center:?}：({x}, {y}) 通道 {c}：{got}，预期 {want}"
          );
        }
      }
      let mut error = 0.0;
      for y in 0..74 {
        for x in 0..59 {
          for (c, &want) in expected(x, y).iter().enumerate() {
            error += f64::from((i32::from(p.rgb[3 * (y * 59 + x) + c]) - want).abs());
          }
        }
      }
      let error = error / (59.0 * 74.0 * 3.0);
      assert!(error < 3.0, "{file} {center:?}：整幅图平均误差 {error}");
      assert!(d.text.contains("pic%T"), "{file}：{:?}", d.text);
    }
  }
}
