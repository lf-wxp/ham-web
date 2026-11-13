//! 合成往返：本 crate 的发端 → 收端，覆盖全部子模式与多种采样率 / 中心频率。

use ham_web_dsp::{stretch, synth_drifting, warp};
use ham_web_mfsk::{MODES, decode_audio, encode_audio, encode_symbols, parse_mode};

const TEXT: &str = "the quick brown fox jumps over the lazy dog 1234567890";

fn text_ok(got: &str) -> bool {
  got.contains(TEXT)
}

#[test]
fn every_mode_round_trips_at_its_own_rate() {
  const CENTER: f64 = 1000.0;
  for mode in MODES {
    let audio = encode_audio(TEXT, mode, mode.rate, CENTER).expect("编码");
    let d = decode_audio(&audio, mode.rate, mode, Some(CENTER), false)
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

/// 短报文只用到少数几个音调：MFSK8 / MFSK4 的自动中心检测曾偏低 2–3 个音调、整段乱码
/// （`CQ CQ DE BG1AAA K ` 在 35 个中心频率里 MFSK8 错 6 个、MFSK4 错 5 个，`TEST ` 全错）；
/// `TEST ` / `73 ` 这种极短报文手填中心频率也乱码（深度 10 的乱码比深度 5 的正确文本更长，可读性分数偏向它）。
/// 变异验证：不用空闲音的锚（`idle_tone_shift` 恒返回 `None`）、深度只看可读性（`DEPTH_METRIC_GAP` 改很大），
/// 各自让下面某一行变红。
#[test]
fn short_messages_on_slow_modes_find_their_centre_and_depth() {
  for (name, msg, center) in [
    ("8", "CQ DE BG1AAA K ", 969.0),
    ("8", "CQ DE BG1AAA K ", 1314.0),
    ("8", "73 ", 1000.0),
    ("8", "73 ", 1500.0),
    ("8", "CQ ", 1000.0),
    ("8", "CQ ", 1337.0),
    ("4", "CQ DE BG1AAA K ", 946.0),
    ("4", "73 ", 1000.0),
  ] {
    let mode = parse_mode(name).unwrap();
    let audio = encode_audio(msg, mode, 8000, center).unwrap();
    let d = decode_audio(&audio, 8000, mode, None, false)
      .unwrap_or_else(|| panic!("MFSK{name} {msg:?} @{center} 应解出信号"));
    assert!(
      d.text.contains(msg.trim_end()),
      "MFSK{name} {msg:?} @{center}：{:?}（检测到 {} Hz）",
      d.text,
      d.center_hz
    );
    assert!(
      (d.center_hz - center).abs() <= mode.baud() / 8.0,
      "MFSK{name} {msg:?} @{center}：检测到 {} Hz",
      d.center_hz
    );
    assert_eq!(d.depth, 5, "MFSK{name} {msg:?} @{center}");
  }
}

/// 空闲音的锚在下边带录音里也要对：频谱反转后前导在梳的最高一齿。
/// 用 `(-1)^n` 调制做频谱反转（`f → 采样率 / 2 − f`）。
#[test]
fn the_idle_tone_anchor_works_on_a_reversed_recording() {
  let mode = parse_mode("8").unwrap();
  for center in [3000.0, 2663.0] {
    let audio = encode_audio("CQ ", mode, 8000, center).unwrap();
    let flipped: Vec<f32> = audio
      .iter()
      .enumerate()
      .map(|(i, &x)| if i % 2 == 0 { x } else { -x })
      .collect();
    let d = decode_audio(&flipped, 8000, mode, None, true)
      .unwrap_or_else(|| panic!("反转录音 @{center} 应解出信号"));
    assert!(d.text.contains("CQ"), "反转 @{center}：{:?}", d.text);
    assert!(
      (d.center_hz - (4000.0 - center)).abs() <= mode.baud() / 8.0,
      "反转 @{center}：检测到 {} Hz",
      d.center_hz
    );
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

/// 短录音 + 很大的时钟误差：块缩短后才凑得够块数去跟踪，MFSK4 这类慢速模式还要把录音重采样回名义时钟
/// 再解一遍（时钟误差会把音调频率一起缩放，中心频率检测在跟踪之前、不知道）。
/// 变异验证：`REDO_CLOCK_PPM` 改成 1e12、`MIN_BLOCK` 改成 `BLOCK`（MFSK4 的 178 个符号凑不够 4 块，
/// 没有估值就不会重来）、重采样后不把中心频率换回录音的轴、不把两次的时钟误差合起来，各自都让 MFSK4 一行变红；
/// MFSK16 / 32 两行钉死更大的误差（±12000 ppm，改动前这两行乱码或缺遍），上面几个变异不一定让它们变红。
#[test]
fn short_recordings_survive_a_large_clock_error() {
  for (name, reps, ppm) in [
    ("4", 2usize, 3000.0f64),
    ("4", 2, 5000.0),
    ("16", 3, -12000.0),
    ("32", 1, 12000.0),
  ] {
    let mode = parse_mode(name).unwrap();
    let msg = "CQ CQ DE BG1AAA K ".repeat(reps);
    let clean = encode_audio(&msg, mode, 8000, 1000.0).unwrap();
    let audio = stretch(&clean, 1.0 + ppm * 1e-6);
    let d = decode_audio(&audio, 8000, mode, None, false)
      .unwrap_or_else(|| panic!("MFSK{name} {ppm} ppm 应解出信号"));
    assert_eq!(
      d.text.matches("CQ CQ DE BG1AAA K").count(),
      reps,
      "MFSK{name} {ppm} ppm: {:?}",
      d.text
    );
    // 结果按录音自己的轴报：总时钟误差与实际拉伸一致，中心频率是录音里的 1000 / (1 + ppm)。
    assert!(
      (d.clock_ppm - ppm).abs() < 400.0,
      "MFSK{name} {ppm} ppm 估成 {}",
      d.clock_ppm
    );
    let center = 1000.0 / (1.0 + ppm * 1e-6);
    assert!(
      (d.center_hz - center).abs() < 2.0,
      "MFSK{name} 中心 {center}，报告 {}",
      d.center_hz
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

/// 报文严格周期重复时，等间距抽窗口会与周期混叠：几个边缘音调从没被抽到，梳评分在那里没有证据，
/// 梳就偏一个音调（整段乱码）。MFSK128 的 100 / 127 遍复读曾在自动检测里选成 `中心 − 125 Hz`；
/// 窗口改成格内抖动抽样之后，不给中心频率也要找对（`front::signal_windows`）。
#[test]
fn auto_center_survives_a_message_repeated_in_lockstep_with_the_sampling() {
  let mode = parse_mode("128").unwrap();
  for reps in [100usize, 127] {
    let text = format!("{TEXT} ").repeat(reps);
    let audio = encode_audio(&text, mode, mode.rate, 1500.0).unwrap();
    let d = decode_audio(&audio, mode.rate, mode, None, false).expect("应解出信号");
    assert!(
      (d.center_hz - 1500.0).abs() <= mode.baud() / 8.0,
      "重复 {reps} 遍：检测到 {} Hz，期望 1500 Hz",
      d.center_hz
    );
    assert_eq!(
      d.text.matches(TEXT).count(),
      reps,
      "重复 {reps} 遍应逐遍解出"
    );
  }
}

/// 恒定载波（窄带干扰）：fldigi 4.2.13 自己的接收机在载波与信号等幅时一遍报文也解不出
/// （差分对拍见 `crates/thor/examples/noise_sweep.rs` 与 `reference/fldigi_web/README.md`）。本仓整段录音里估出载波幅度，不比信号强的减掉，
/// 比信号强的把那个音调位抹掉。噪声取得足够大：噪声小的时候 fldigi 沿用的窄带干扰软删除自己就够了，
/// 测不出这一步（变异验证：去掉两步中任何一步，对应的用例都红）。
#[test]
fn a_carrier_the_size_of_the_signal_is_suppressed_and_a_stronger_one_is_erased() {
  let mode = parse_mode("16").unwrap();
  let center = 1500.0;
  let text = format!("{TEXT} ").repeat(6);
  for (amp, noise) in [(1.0f64, 3.0f32), (2.0, 3.0)] {
    let mut audio = encode_audio(&text, mode, 8000, center).unwrap();
    // 载波放在两个音调之间（距最近音调 4.7 Hz）。
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

/// 长交织变体（64L / 128L）：整段录音只用到少数几个音调（0 / 1 / 3 / 7 / 15），90% 以上的符号是最低音调。
/// 不给中心频率时粗检测会偏几个音调（见 `front::POLISH_REACH_LONG`），译出的正文头尾也不能带乱码
/// （头部是反交织器没填满时译出的「发送前历史」，尾部是给冲刷区补删除位的产物）。
#[test]
fn long_interleave_modes_find_the_carrier_and_decode_cleanly() {
  for name in ["64L", "128L"] {
    let mode = parse_mode(name).unwrap();
    for center in [1250.3, 1500.0, 1800.0, 2000.0] {
      let audio = encode_audio(TEXT, mode, 8000, center).unwrap();
      let d = decode_audio(&audio, 8000, mode, None, false)
        .unwrap_or_else(|| panic!("MFSK{name} @ {center} 应解出信号"));
      assert!(
        (d.center_hz - center).abs() <= mode.baud() / 8.0,
        "MFSK{name}: 检测到 {} Hz，期望 {center}",
        d.center_hz
      );
      assert_eq!(d.text.trim(), TEXT, "MFSK{name} @ {center}");
    }
  }
}

/// 同上，但带噪声底。无噪声的合成信号里反交织器没填满时译出的头部、冲刷区补删除位译出的尾部恰好
/// 什么字符都凑不出来（变异验证过：去掉丢头 / 补尾保护，上一个用例仍然通过），噪声底下它们才会变成乱码 ——
/// 真实录音都有噪声底。
#[test]
fn long_interleave_modes_decode_cleanly_in_noise() {
  for name in ["64L", "128L"] {
    let mode = parse_mode(name).unwrap();
    for amp in [1.0f32, 2.0] {
      let mut audio = encode_audio(TEXT, mode, 8000, 1500.0).unwrap();
      ham_web_dsp::add_noise(&mut audio, amp);
      let d = decode_audio(&audio, 8000, mode, None, false)
        .unwrap_or_else(|| panic!("MFSK{name} 噪声幅度 {amp} 应解出信号"));
      assert_eq!(d.text.trim(), TEXT, "MFSK{name} 噪声幅度 {amp}");
    }
  }
}

/// 发端与 fldigi 的帧结构一致：长交织变体没有 `preamble / 3` 个前导零位，其余模式有。
/// 每个码位变成 2 个码字位；`clearbits` 先推进 `preamble` 对零（不发符号，但可能留下不满一个符号的零头），
/// 之后每攒满 `symbits` 个码字位发一个符号。
#[test]
fn long_modes_send_no_lead_zeros() {
  use ham_web_mfsk::lookup::varienc;
  let char_bits = |c: u8| varienc(c).len();
  let body: usize = TEXT.bytes().map(char_bits).sum();
  let frame: usize = [b'\r', 2, b'\r', b'\r', 4, b'\r']
    .into_iter()
    .map(char_bits)
    .sum();
  for mode in MODES {
    let lead = if mode.long { 0 } else { mode.preamble / 3 };
    let sent_bits = lead + frame + body + 1 + mode.preamble;
    let cleared = mode.preamble * 2;
    let expected = (cleared + sent_bits * 2) / mode.symbits - cleared / mode.symbits;
    assert_eq!(
      encode_symbols(TEXT, mode).len(),
      expected,
      "MFSK{}",
      mode.name
    );
  }
}
