//! 合成往返：本 crate 的发端 → 收端，覆盖全部子模式与多种采样率 / 中心频率 / 噪声。

use ham_web_dsp::{stretch, synth_drifting};
use ham_web_thor::{MODES, decode_audio, encode_audio, encode_symbols, parse_mode};

const TEXT: &str = "the quick brown fox jumps over the lazy dog 1234567890";

/// 正文后面补的空格：把管线里压着的最后几个字符推出来（见 `Receiver::flush`；这里的发端冲刷
/// 本身不够长，fldigi 自己的发端也一样 —— 它靠接收机收完信号后继续运行把尾巴推出来）。
fn padded() -> String {
  format!("{TEXT}   ")
}

#[test]
fn every_mode_round_trips_at_its_own_rate() {
  for mode in MODES {
    // 带宽 18 × 间隔：THOR 100 / 25x4 / 50x2 的带宽有 1800 Hz，中心放 2000 Hz 才放得下。
    let center = 2000.0;
    let audio = encode_audio(&padded(), mode, mode.rate, center).expect("编码");
    let d = decode_audio(&audio, mode.rate, mode, Some(center), false)
      .unwrap_or_else(|| panic!("THOR {} 应解出信号", mode.name));
    assert!(d.text.contains(TEXT), "THOR {}: {:?}", mode.name, d.text);
  }
}

#[test]
fn the_tail_of_the_message_is_flushed_out() {
  // 发端只补 4 个空闲符，管线延迟比它长：不 flush 的话最后几个字符出不来。
  for name in ["micro", "4", "16", "100"] {
    let mode = parse_mode(name).unwrap();
    let audio = encode_audio(TEXT, mode, mode.rate, 2000.0).unwrap();
    let d = decode_audio(&audio, mode.rate, mode, Some(2000.0), false).unwrap();
    assert!(d.text.contains(TEXT), "THOR {name}: {:?}", d.text);
  }
}

#[test]
fn auto_center_finds_the_carrier_and_decodes() {
  for (name, center) in [
    ("16", 1500.0),
    ("16", 1237.5),
    ("8", 1000.0),
    ("4", 900.0),
    ("22", 1700.0),
  ] {
    let mode = parse_mode(name).unwrap();
    let audio = encode_audio(&padded(), mode, mode.rate, center).unwrap();
    let d = decode_audio(&audio, mode.rate, mode, None, false)
      .unwrap_or_else(|| panic!("THOR {name} @ {center} 应解出信号"));
    assert!(
      (d.center_hz - center).abs() <= mode.spacing() / 8.0 + 1.0,
      "THOR {name}: 检测到 {} Hz，期望 {center}",
      d.center_hz
    );
    assert!(d.text.contains(TEXT), "THOR {name}: {:?}", d.text);
  }
}

#[test]
fn works_at_non_native_sample_rates() {
  let mode = parse_mode("16").unwrap();
  for rate in [11025u32, 12000, 22050, 44100, 48000] {
    let audio = encode_audio(&padded(), mode, rate, 1500.0).unwrap();
    let d = decode_audio(&audio, rate, mode, None, false)
      .unwrap_or_else(|| panic!("{rate} Hz 应解出信号"));
    assert!(d.text.contains(TEXT), "{rate} Hz: {:?}", d.text);
  }
}

#[test]
fn secondary_channel_text_goes_to_the_id_field() {
  // `\x01` 之后的字节走二次信道（ID 信标）：必须进 `id`，不能混进正文。
  let mode = parse_mode("16").unwrap();
  let audio = encode_audio(
    "CQ \u{1}f\u{1}l\u{1}d\u{1}i\u{1}g\u{1}i DE BG1AAA   ",
    mode,
    8000,
    1500.0,
  )
  .unwrap();
  let d = decode_audio(&audio, 8000, mode, Some(1500.0), false).expect("应解出信号");
  assert!(
    d.text.contains("CQ") && d.text.contains("DE BG1AAA"),
    "{:?}",
    d.text
  );
  assert!(d.id.contains("fldigi"), "id = {:?}", d.id);
  assert!(
    !d.text.contains("fldigi"),
    "二次信道文字不应出现在正文：{:?}",
    d.text
  );
}

#[test]
fn reversed_sideband_needs_the_reverse_flag() {
  // 正常信号按 reverse 解不出原文。
  let mode = parse_mode("16").unwrap();
  let audio = encode_audio(&padded(), mode, 8000, 1500.0).unwrap();
  let flipped = decode_audio(&audio, 8000, mode, Some(1500.0), true);
  assert!(flipped.is_none_or(|d| !d.text.contains(TEXT)));
}

#[test]
fn silence_and_noise_are_rejected() {
  for mode in MODES {
    let rate = mode.rate;
    assert!(decode_audio(&vec![0.0f32; rate as usize * 20], rate, mode, None, false).is_none());
    let mut seed = 0x1234_5678u32;
    let noise: Vec<f32> = (0..rate as usize * 20)
      .map(|_| {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        ((seed >> 8) as f32 / 8_388_608.0) - 1.0
      })
      .collect();
    for center in [Some(2000.0), None] {
      assert!(
        decode_audio(&noise, rate, mode, center, false).is_none(),
        "THOR {} 噪声不应解出（center {center:?}）",
        mode.name
      );
    }
  }
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
fn decodes_through_noise() {
  // THOR 是硬位判决（fldigi 的软位路径对弱信号略好）：这里钉死「音调箱信噪比约 +16 dB」仍然逐字还原，
  // 不替更弱的信号作保证。均匀噪声幅度 A 的功率是 A²/3，落在 15.6 Hz 一个箱里：A = 3 → 约 +16 dB。
  let mode = parse_mode("16").unwrap();
  let clean = encode_audio(&padded(), mode, 8000, 1500.0).unwrap();
  let mut audio = clean;
  ham_web_dsp::add_noise(&mut audio, 3.0);
  let d = decode_audio(&audio, 8000, mode, None, false).expect("噪声下应解出信号");
  assert!(d.text.contains(TEXT), "{:?}", d.text);
}

#[test]
fn lead_in_and_tail_silence_do_not_matter() {
  let mode = parse_mode("16").unwrap();
  let clean = encode_audio(&padded(), mode, 8000, 1300.0).unwrap();
  let mut audio = vec![0.0f32; 8000 * 3 + 137];
  audio.extend_from_slice(&clean);
  audio.extend(std::iter::repeat_n(0.0f32, 8000 * 5));
  let d = decode_audio(&audio, 8000, mode, None, false).expect("应解出信号");
  assert!(d.text.contains(TEXT), "{:?}", d.text);
}

#[test]
fn rejects_unusable_parameters() {
  let mode = parse_mode("16").unwrap();
  let audio = encode_audio(&padded(), mode, 8000, 1500.0).unwrap();
  assert!(decode_audio(&audio, 0, mode, Some(1500.0), false).is_none());
  assert!(decode_audio(&audio, 8000, mode, Some(f64::NAN), false).is_none());
  // 信号带越过 Nyquist / 越过 0 Hz：直接拒绝。
  assert!(decode_audio(&audio, 8000, mode, Some(3990.0), false).is_none());
  assert!(decode_audio(&audio, 8000, mode, Some(10.0), false).is_none());
  assert!(decode_audio(&[], 8000, mode, None, false).is_none());
  assert!(encode_audio(TEXT, mode, 8000, 3990.0).is_none());
  assert!(encode_audio(TEXT, mode, 0, 1500.0).is_none());
  assert!(encode_audio(TEXT, mode, 8000, f64::NAN).is_none());
  assert!(parse_mode("nope").is_none());
}

#[test]
fn secondary_codes_are_all_in_the_secondary_range() {
  // 接收端靠「码字 ≥ 0xB80」把主、次信道分开：次信道表的每个码字都必须满足。
  for (i, c) in ham_web_thor::varicode::SECONDARY.iter().enumerate() {
    let v = u32::from_str_radix(c, 2).unwrap();
    assert!(v >= 0xB80, "第 {i} 项 {c} = {v:#x}");
    // 与 MFSK 主码表不冲突。
    assert!(
      ham_web_mfsk::lookup::varidec(v).is_none(),
      "第 {i} 项与主码表冲突"
    );
  }
}

#[test]
fn auto_center_survives_noise_in_every_mode() {
  // 回归：THOR 的音调间隔是波特率的 2 / 4 倍，音调落在粗网格两点正中时，矩形窗在两边都是零点，
  // 干净信号靠泄漏侥幸检出，加一点点噪声就选到 Nyquist 附近去了（THOR8 / THOR4 实测过）。
  // 均匀噪声幅度 2 → 音调箱里的信噪比约 +20 dB（15.6 Hz 箱）。
  for mode in MODES {
    let clean = encode_audio(&padded(), mode, mode.rate, 2000.0).unwrap();
    let mut audio = clean;
    ham_web_dsp::add_noise(&mut audio, 2.0);
    let d = decode_audio(&audio, mode.rate, mode, None, false)
      .unwrap_or_else(|| panic!("THOR {} 噪声下应解出信号", mode.name));
    assert!(
      (d.center_hz - 2000.0).abs() <= mode.spacing() / 8.0 + 1.0,
      "THOR {}: 检测到 {} Hz",
      mode.name,
      d.center_hz
    );
    assert!(d.text.contains(TEXT), "THOR {}: {:?}", mode.name, d.text);
  }
}

#[test]
fn carrier_between_coarse_grid_points_is_found() {
  // 载频让音调落在「半个间隔」的位置（15.625 的整数倍加 7.8 Hz）：THOR8 / 4 的最坏情形。
  for (name, center) in [("8", 1007.8125), ("4", 1003.90625), ("16", 1007.8125)] {
    let mode = parse_mode(name).unwrap();
    let mut audio = encode_audio(&padded(), mode, 8000, center).unwrap();
    ham_web_dsp::add_noise(&mut audio, 1.0);
    let d = decode_audio(&audio, 8000, mode, None, false)
      .unwrap_or_else(|| panic!("THOR {name} @ {center} 应解出信号"));
    assert!(d.text.contains(TEXT), "THOR {name}: {:?}", d.text);
  }
}

fn long_text() -> String {
  format!("{TEXT} ").repeat(8)
}

#[test]
fn tracks_large_clock_error() {
  // 1000 ppm 起不跟踪就开头或结尾出错；THOR 的 IFK+ 看相邻符号的音调差，一次错位会连错两个符号。
  for name in ["16", "8"] {
    let mode = parse_mode(name).unwrap();
    let clean = encode_audio(&long_text(), mode, mode.rate, 1500.0).unwrap();
    for ppm in [1000.0f64, -1500.0] {
      let audio = stretch(&clean, 1.0 + ppm * 1e-6);
      let d = decode_audio(&audio, mode.rate, mode, None, false)
        .unwrap_or_else(|| panic!("THOR {name} {ppm} ppm 应解出信号"));
      assert_eq!(
        d.text.matches(TEXT).count(),
        8,
        "THOR {name} {ppm} ppm: {:?}",
        d.text
      );
      assert!(
        (d.clock_ppm - ppm).abs() < 60.0,
        "THOR {name} {ppm} ppm 估成 {}",
        d.clock_ppm
      );
    }
  }
}

#[test]
fn tracks_linear_frequency_drift() {
  // THOR16 的音调间隔 15.6 Hz：0.2 Hz/s、80 秒偏出 16 Hz。
  let mode = parse_mode("16").unwrap();
  for hz_per_s in [0.2f64, -0.3] {
    let freqs: Vec<f64> = encode_symbols(&long_text(), mode)
      .iter()
      .map(|&t| 1500.0 + mode.tone_offset(usize::from(t)))
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
  }
}

#[test]
fn clean_recordings_are_left_exactly_as_they_were() {
  // 没有漂移时不应凭空修正：报告的两个量都是精确的 0。
  for mode in MODES {
    let mut text = long_text();
    while encode_symbols(&text, mode).len() < 700 {
      text.push_str(&long_text());
    }
    let audio = encode_audio(&text, mode, mode.rate, 2000.0).unwrap();
    let d = decode_audio(&audio, mode.rate, mode, Some(2000.0), false).expect("应解出信号");
    assert_eq!(d.clock_ppm, 0.0, "THOR {}", mode.name);
    assert_eq!(d.drift_hz_per_s, 0.0, "THOR {}", mode.name);
  }
}

#[test]
fn tracking_works_in_every_mode() {
  for mode in MODES {
    let mut text = long_text();
    while encode_symbols(&text, mode).len() < 700 {
      text.push_str(&long_text());
    }
    let repeats = text.matches(TEXT).count();
    let clean = encode_audio(&text, mode, mode.rate, 2000.0).unwrap();
    for ppm in [1000.0f64, -1000.0] {
      let audio = stretch(&clean, 1.0 + ppm * 1e-6);
      let d = decode_audio(&audio, mode.rate, mode, None, false)
        .unwrap_or_else(|| panic!("THOR {} {ppm} ppm 应解出信号", mode.name));
      assert_eq!(
        d.text.matches(TEXT).count(),
        repeats,
        "THOR {} {ppm} ppm: {:?}",
        mode.name,
        d.text
      );
    }
  }
}

/// 恒定载波（窄带干扰）：载波不比信号强的在音调幅度里减掉，比信号强的把那个音调位抹掉。
/// fldigi 4.2.13 自己的接收机在载波与信号等幅时一遍报文也解不出（差分对拍见 `crates/thor/examples/noise_sweep.rs` 与 `reference/fldigi_web/README.md`）。
/// 噪声取得足够大，噪声小时硬判决的接收机自己还能应付（变异验证：去掉任何一步，对应的载波幅度都会红）。
#[test]
fn a_constant_carrier_is_suppressed_or_erased() {
  let mode = parse_mode("16").unwrap();
  let center = 1500.0;
  let text = format!("{TEXT} ").repeat(6);
  for (amp, noise) in [(1.0f64, 3.0f32), (2.0, 3.0)] {
    let mut audio = encode_audio(&text, mode, 8000, center).unwrap();
    let w = 2.0 * std::f64::consts::PI * (center + 50.0) / 8000.0;
    for (i, s) in audio.iter_mut().enumerate() {
      *s += (amp * (w * i as f64).sin()) as f32;
    }
    ham_web_dsp::add_noise(&mut audio, noise);
    let d = decode_audio(&audio, 8000, mode, Some(center), false).expect("应解出信号");
    assert!(
      d.text.matches(TEXT).count() >= 5,
      "载波幅度 {amp}、噪声 {noise}：{:?}",
      d.text
    );
  }
}
