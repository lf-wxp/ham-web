//! 合成往返：本 crate 的发端 → 收端，覆盖全部子模式与多种采样率 / 中心频率。

use ham_web_dsp::{stretch, synth_drifting, warp};
use ham_web_mfsk::{MODES, decode_audio, encode_audio, encode_symbols, parse_mode};

const TEXT: &str = "the quick brown fox jumps over the lazy dog 1234567890";

fn text_ok(got: &str) -> bool {
  got.contains(TEXT)
}

#[test]
fn every_mode_round_trips_at_its_own_rate() {
  for mode in MODES {
    let audio = encode_audio(TEXT, mode, mode.rate, 1000.0).expect("编码");
    let d = decode_audio(&audio, mode.rate, mode, Some(1000.0), false)
      .unwrap_or_else(|| panic!("MFSK{} 应解出信号", mode.name));
    assert!(text_ok(&d.text), "MFSK{}: {:?}", mode.name, d.text);
  }
}

#[test]
fn auto_center_finds_the_carrier_and_decodes() {
  for (name, center) in [("16", 1500.0), ("16", 900.0), ("32", 1200.0), ("8", 1000.0)] {
    let mode = parse_mode(name).unwrap();
    let audio = encode_audio(TEXT, mode, 8000, center).unwrap();
    let d = decode_audio(&audio, 8000, mode, None, false)
      .unwrap_or_else(|| panic!("MFSK{name} @ {center} 应解出信号"));
    assert!(
      (d.center_hz - center).abs() <= mode.baud() / 8.0,
      "MFSK{name}: 检测到 {} Hz，期望 {center}",
      d.center_hz
    );
    assert!(text_ok(&d.text), "MFSK{name}: {:?}", d.text);
  }
}

#[test]
fn works_at_non_native_sample_rates() {
  // 44100 / 48000 / 12000 与 baud 的比值都不是整数：符号边界按小数位置落点，不累积偏移。
  let mode = parse_mode("16").unwrap();
  for rate in [11025u32, 12000, 22050, 44100, 48000] {
    let audio = encode_audio(TEXT, mode, rate, 1000.0).unwrap();
    let d = decode_audio(&audio, rate, mode, None, false)
      .unwrap_or_else(|| panic!("{rate} Hz 应解出信号"));
    assert!(text_ok(&d.text), "{rate} Hz: {:?}", d.text);
  }
}

#[test]
fn reversed_sideband_needs_the_reverse_flag() {
  let mode = parse_mode("16").unwrap();
  let audio = encode_audio(TEXT, mode, 8000, 1000.0).unwrap();
  // 在频域镜像：以中心为轴翻转音调序。用发端的 tone 序列构造不方便，改为
  // 直接把「反转」当作收端选项验证：正常信号按 reverse 解不出原文。
  let flipped = decode_audio(&audio, 8000, mode, Some(1000.0), true);
  assert!(flipped.is_none_or(|d| !text_ok(&d.text)));
}

#[test]
fn mfsk8_decodes_both_the_fldigi_and_the_gmfsk_interleaver_depth() {
  // fldigi 的 MFSK8 交织深度是 5，gMFSK 发出的是 10（见 `depth_candidates`）：两种都要能解。
  let mode = parse_mode("8").unwrap();
  for depth in [5usize, 10] {
    let tx = ham_web_mfsk::Mode { depth, ..*mode };
    // 深度 10 的译码延迟（交织器 40 个符号 + Viterbi 回溯约 18 个符号）比发端的收尾冲刷（107 个零位
    // ≈ 43 个符号）还长，正文末尾几个字符会压在管线里 —— 靠收端的 `flush` 推出来，所以这里不补任何尾巴。
    let audio = encode_audio(TEXT, &tx, 8000, 1000.0).unwrap();
    let d = decode_audio(&audio, 8000, mode, Some(1000.0), false).expect("应解出信号");
    assert!(text_ok(&d.text), "深度 {depth}: {:?}", d.text);
    assert_eq!(d.depth, depth);
  }
}

#[test]
fn silence_and_noise_are_rejected() {
  for mode in MODES {
    let rate = mode.rate;
    assert!(
      decode_audio(
        &vec![0.0f32; rate as usize * 20],
        rate,
        mode,
        Some(1000.0),
        false
      )
      .is_none(),
      "MFSK{} 静音不应解出",
      mode.name
    );
    assert!(
      decode_audio(&vec![0.0f32; rate as usize * 20], rate, mode, None, false).is_none(),
      "MFSK{} 静音（自动中心）不应解出",
      mode.name
    );
    let mut noise = vec![0.0f32; rate as usize * 20];
    ham_web_dsp::add_noise(&mut noise, 1.0);
    for center in [Some(1000.0), None] {
      assert!(
        decode_audio(&noise, rate, mode, center, false).is_none(),
        "MFSK{} 噪声不应解出（center {center:?}）",
        mode.name
      );
    }
  }
}

#[test]
fn decodes_through_strong_noise() {
  // 均匀噪声幅度 A 的功率是 A²/3：A = 6 时总信噪比约 −13.8 dB，但落在一个 15.6 Hz 的音调箱里
  // 仍有约 +10 dB —— MFSK16 的设计工作点就在这一带。
  let mode = parse_mode("16").unwrap();
  let clean = encode_audio(TEXT, mode, 8000, 1000.0).unwrap();
  for amp in [3.0f32, 6.0] {
    let mut audio = clean.clone();
    ham_web_dsp::add_noise(&mut audio, amp);
    let d = decode_audio(&audio, 8000, mode, None, false)
      .unwrap_or_else(|| panic!("噪声幅度 {amp} 下应解出信号"));
    assert!(text_ok(&d.text), "噪声幅度 {amp}: {:?}", d.text);
  }
}

#[test]
fn lead_in_and_tail_silence_do_not_matter() {
  let mode = parse_mode("16").unwrap();
  let clean = encode_audio(TEXT, mode, 8000, 1300.0).unwrap();
  let mut audio = vec![0.0f32; 8000 * 3 + 137];
  audio.extend_from_slice(&clean);
  audio.extend(std::iter::repeat_n(0.0f32, 8000 * 5));
  let d = decode_audio(&audio, 8000, mode, None, false).expect("应解出信号");
  assert!(text_ok(&d.text), "{:?}", d.text);
}

#[test]
fn rejects_unusable_parameters() {
  let mode = parse_mode("16").unwrap();
  let audio = encode_audio(TEXT, mode, 8000, 1000.0).unwrap();
  assert!(decode_audio(&audio, 0, mode, Some(1000.0), false).is_none());
  assert!(decode_audio(&audio, 8000, mode, Some(f64::NAN), false).is_none());
  // 信号带越过 Nyquist / 越过 0 Hz：直接拒绝，不去解混叠后的东西。
  assert!(decode_audio(&audio, 8000, mode, Some(3990.0), false).is_none());
  assert!(decode_audio(&audio, 8000, mode, Some(10.0), false).is_none());
  assert!(decode_audio(&[], 8000, mode, None, false).is_none());
  assert!(encode_audio(TEXT, mode, 8000, 3990.0).is_none());
  assert!(encode_audio(TEXT, mode, 0, 1000.0).is_none());
  assert!(encode_audio(TEXT, mode, 8000, f64::NAN).is_none());
}

#[test]
fn noise_never_decodes_across_many_seeds() {
  let mode = parse_mode("16").unwrap();
  for seed0 in [
    0x1234_5678u32,
    0x9E37_79B9,
    0x5BD1_E995,
    0x85EB_CA6B,
    0xC2B2_AE35,
    0x27D4_EB2F,
  ] {
    let mut seed = seed0;
    let noise: Vec<f32> = (0..8000 * 30)
      .map(|_| {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        ((seed >> 8) as f32 / 8_388_608.0) - 1.0
      })
      .collect();
    assert!(
      decode_audio(&noise, 8000, mode, None, false).is_none(),
      "seed {seed0:#x}"
    );
  }
}

#[test]
fn survives_sample_clock_error() {
  // 录音机 / 声卡时钟与发端差几十 ppm 是常态；更大的偏差见 `tracks_large_clock_error`。
  let mode = parse_mode("16").unwrap();
  let text = format!("{TEXT} ").repeat(8);
  let clean = encode_audio(&text, mode, 8000, 1000.0).unwrap();
  for ppm in [100.0f64, 300.0] {
    let audio = stretch(&clean, 1.0 + ppm * 1e-6);
    let d = decode_audio(&audio, 8000, mode, None, false).expect("应解出信号");
    assert_eq!(d.text.matches(TEXT).count(), 8, "{ppm} ppm: {:?}", d.text);
  }
}

#[test]
fn auto_center_survives_noise_in_every_mode() {
  // 均匀噪声幅度 2：音调箱里的信噪比在 +17 dB 以上（MFSK128 的 125 Hz 箱最宽，噪声最多）。
  // 正文后补空格：MFSK4 / 8 深度 5 时延迟比发端冲刷长一点，尾巴要靠 `flush` 推出来。
  let padded = format!("{TEXT}   ");
  for mode in MODES {
    let mut audio = encode_audio(&padded, mode, mode.rate, 1700.0).unwrap();
    ham_web_dsp::add_noise(&mut audio, 2.0);
    let d = decode_audio(&audio, mode.rate, mode, None, false)
      .unwrap_or_else(|| panic!("MFSK{} 噪声下应解出信号", mode.name));
    assert!(
      (d.center_hz - 1700.0).abs() <= mode.baud() / 8.0 + 1.0,
      "MFSK{}: 检测到 {} Hz",
      mode.name,
      d.center_hz
    );
    assert!(text_ok(&d.text), "MFSK{}: {:?}", mode.name, d.text);
  }
}

fn long_text() -> String {
  format!("{TEXT} ").repeat(8)
}

#[test]
fn tracks_large_clock_error() {
  // 1000 ppm 起不跟踪就开头或结尾出错；跟踪之后正反两个方向都要稳。
  let mode = parse_mode("16").unwrap();
  let clean = encode_audio(&long_text(), mode, 8000, 1000.0).unwrap();
  for ppm in [1000.0f64, 2500.0, -1500.0] {
    let audio = stretch(&clean, 1.0 + ppm * 1e-6);
    let d = decode_audio(&audio, 8000, mode, None, false).expect("应解出信号");
    assert_eq!(d.text.matches(TEXT).count(), 8, "{ppm} ppm: {:?}", d.text);
    // 报告的时钟误差要是真的：符号周期在录音里被拉长了 `ppm`。
    assert!(
      (d.clock_ppm - ppm).abs() < 60.0,
      "{ppm} ppm 估成 {}",
      d.clock_ppm
    );
  }
}

#[test]
fn tracks_linear_frequency_drift() {
  // 0.2 Hz/s、80 秒 = 偏出 16 Hz（约一个音调间隔）：不跟踪时两头的音调已经落到邻位上。
  let mode = parse_mode("16").unwrap();
  let low = 1000.0 - mode.bandwidth() / 2.0;
  for hz_per_s in [0.2f64, -0.3] {
    let freqs: Vec<f64> = encode_symbols(&long_text(), mode)
      .iter()
      .map(|&t| low + f64::from(t) * mode.baud())
      .collect();
    let audio = synth_drifting(&freqs, 8000, 8000.0 / mode.baud(), hz_per_s);
    let d = decode_audio(&audio, 8000, mode, None, false).expect("应解出信号");
    assert_eq!(
      d.text.matches(TEXT).count(),
      8,
      "{hz_per_s} Hz/s: {:?}",
      d.text
    );
    assert!(
      (d.drift_hz_per_s - hz_per_s).abs() < 0.02,
      "{hz_per_s} Hz/s 估成 {}",
      d.drift_hz_per_s
    );
    assert!(
      d.clock_ppm.abs() < 30.0,
      "没有时钟误差却估成 {} ppm",
      d.clock_ppm
    );
    // 中心频率取信号区间中点处的值：整段录音漂了 hz_per_s × 时长，中点在一半。
    let mid = 1000.0 + hz_per_s * (d.symbols as f64 / mode.baud()) / 2.0;
    assert!(
      (d.center_hz - mid).abs() < 1.5,
      "中点 {mid}，报告 {}",
      d.center_hz
    );
  }
}

#[test]
fn tracks_clock_error_that_changes_over_time() {
  // 转速逐渐变化：符号边界漂成二次曲线（80 秒偏出约 1 个符号），一条直线抓不住。
  let mode = parse_mode("16").unwrap();
  let clean = encode_audio(&long_text(), mode, 8000, 1500.0).unwrap();
  for (ratio0, slope) in [(0.0, 2e-5), (0.0, -2e-5), (5e-4, 3e-5)] {
    let audio = warp(&clean, 8000.0, ratio0, slope);
    let d = decode_audio(&audio, 8000, mode, None, false).expect("应解出信号");
    assert_eq!(
      d.text.matches(TEXT).count(),
      8,
      "ratio0 {ratio0}, slope {slope}: {:?}",
      d.text
    );
    assert!(
      d.clock_ppm.abs() > 300.0,
      "该看到时钟误差：{} ppm",
      d.clock_ppm
    );
    assert!(
      d.drift_hz_per_s.abs() > 0.01,
      "转速变化也会带出频漂：{}",
      d.drift_hz_per_s
    );
  }
}

#[test]
fn clean_recordings_are_left_exactly_as_they_were() {
  // 没有漂移时不应凭空修正：`track` 退回 `Drift::fixed`，报告的两个量都是精确的 0。
  for mode in MODES {
    let mut text = long_text();
    while encode_symbols(&text, mode).len() < 700 {
      text.push_str(&long_text());
    }
    let audio = encode_audio(&text, mode, mode.rate, 1500.0).unwrap();
    let d = decode_audio(&audio, mode.rate, mode, Some(1500.0), false).expect("应解出信号");
    assert_eq!(d.clock_ppm, 0.0, "MFSK{}", mode.name);
    assert_eq!(d.drift_hz_per_s, 0.0, "MFSK{}", mode.name);
  }
}

#[test]
fn tracking_works_in_every_mode() {
  // 1000 ppm 的时钟误差；每个子模式的文本长到至少有 700 个符号（够 14 块）。
  for mode in MODES {
    let mut text = long_text();
    while encode_symbols(&text, mode).len() < 700 {
      text.push_str(&long_text());
    }
    let repeats = text.matches(TEXT).count();
    let clean = encode_audio(&text, mode, mode.rate, 1500.0).unwrap();
    for ppm in [1000.0f64, -1000.0] {
      let audio = stretch(&clean, 1.0 + ppm * 1e-6);
      let d = decode_audio(&audio, mode.rate, mode, None, false)
        .unwrap_or_else(|| panic!("MFSK{} {ppm} ppm 应解出信号", mode.name));
      assert_eq!(
        d.text.matches(TEXT).count(),
        repeats,
        "MFSK{} {ppm} ppm: {:?}",
        mode.name,
        d.text
      );
    }
  }
}

#[test]
fn tracking_holds_up_in_noise() {
  // 噪声幅度 4（音调箱里约 +12 dB）+ 1000 ppm：跟踪靠的是每块的估计再拟合，噪声下不能崩。
  let mode = parse_mode("16").unwrap();
  let clean = encode_audio(&long_text(), mode, 8000, 1000.0).unwrap();
  let mut audio = stretch(&clean, 1.001);
  ham_web_dsp::add_noise(&mut audio, 4.0);
  let d = decode_audio(&audio, 8000, mode, None, false).expect("应解出信号");
  assert_eq!(d.text.matches(TEXT).count(), 8, "{:?}", d.text);
  assert!((d.clock_ppm - 1000.0).abs() < 100.0, "{}", d.clock_ppm);
}

#[test]
fn a_clean_recording_with_noise_only_blocks_is_not_derailed() {
  // 信号两头各有一大段纯噪声：`track` 只看「有信号」的块，噪声块不能把拟合带偏。
  let mode = parse_mode("16").unwrap();
  let clean = encode_audio(&long_text(), mode, 8000, 1000.0).unwrap();
  let mut audio = vec![0.0f32; 8000 * 20];
  audio.extend_from_slice(&stretch(&clean, 1.001));
  audio.extend(std::iter::repeat_n(0.0f32, 8000 * 20));
  ham_web_dsp::add_noise(&mut audio, 1.0);
  let d = decode_audio(&audio, 8000, mode, None, false).expect("应解出信号");
  assert_eq!(d.text.matches(TEXT).count(), 8, "{:?}", d.text);
}
