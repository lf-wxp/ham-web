//! 真实 Meteor-M2 过境录音的端到端测试（**录音不进仓库**，由环境变量给路径）。
//!
//! 用到的录音：SigIDWiki「Low Rate Picture Transmission (LRPT)」词条信息框里的
//! "I/Q Raw Recording"（mega.nz，`LRPT_16-29-02_137900kHz.zip`，压缩包内日期 2018-05-11）——
//! SDR# 基带，137.9 MHz（Meteor-M N2）、187.5 kHz、双声道 16 位、105 s。来源与授权都没有写明，
//! 所以只在本地跑：
//!
//! ```text
//! LRPT_REAL_IQ=/path/to/LRPT_16-29-02_137900kHz.wav \
//!   cargo test -p ham-web-lrpt --release --test real_iq -- --nocapture
//! ```
//!
//! 没有真值帧可比，判据是解码器自己能自证的量：RS 纠错通过的帧数、VCDU 计数单调、
//! 三个图像通道都能拼出完整行。未设置环境变量时直接跳过（CI 不受影响）。

use ham_web_lrpt::{decode_image, qpsk, rs};

fn load() -> Option<(Vec<f32>, u32)> {
  let path = std::env::var_os("LRPT_REAL_IQ")?;
  let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("读取 {path:?} 失败：{e}"));
  let (samples, rate, channels) = ham_web_apt::parse_wav_interleaved(&bytes).expect("WAV 解析");
  assert_eq!(channels, 2, "需要双声道 I/Q");
  Some((samples, rate))
}

#[test]
fn decodes_a_real_meteor_m2_pass() {
  let Some((iq, rate)) = load() else {
    eprintln!("未设置 LRPT_REAL_IQ，跳过真实录音测试");
    return;
  };
  let syms = qpsk::demodulate(&iq, f64::from(rate));
  let (frames, variants) = qpsk::frames_from_symbols(&syms);
  let rs_ok = frames
    .iter()
    .filter(|f| rs::decode_frame(f).is_some())
    .count();
  let mut switches = 0;
  for w in variants.windows(2) {
    switches += usize::from(w[0] != w[1]);
  }
  // 满载帧数 = 符号数 / 8192。
  let capacity = syms.len() / 8192;
  eprintln!(
    "{rate} Hz：{} 个符号，帧容量 {capacity}，ASM 命中 {}，RS 通过 {rs_ok}，相位假设切换 {switches} 次",
    syms.len(),
    frames.len()
  );
  assert!(frames.len() * 10 >= capacity * 9, "ASM 命中不足九成");
  assert!(rs_ok * 10 >= frames.len() * 9, "RS 通过不足九成");

  let image = decode_image(&frames).expect("应拼出图像");
  eprintln!(
    "图像 {}×{}（首行 {}）",
    image.width, image.height, image.first_row_in_recording
  );
  // 105 s ≈ 每 1.2 s 一个 8 像素 MCU 行 → 至少 600 行像素。
  assert!(image.height >= 600, "图像只有 {} 行", image.height);
  // 这段录音下传的是 64 / 65 / 68（两个可见光 + 一个红外），不是默认假设的 64 / 65 / 66。
  assert_eq!(image.apids, [64, 65, 68]);
  // 不是噪声：真实云图相邻像素高度相关（噪声或错位解码的相关系数接近 0）。
  for (ch, plane) in image.channels.iter().enumerate() {
    let (mut sxy, mut sxx, mut n) = (0f64, 0f64, 0f64);
    let mean = plane.iter().flatten().map(|&v| f64::from(v)).sum::<f64>()
      / (image.width * image.height) as f64;
    for row in plane {
      for w in row.windows(2) {
        let (a, b) = (f64::from(w[0]) - mean, f64::from(w[1]) - mean);
        sxy += a * b;
        sxx += a * a;
        n += 1.0;
      }
    }
    let r = sxy / sxx;
    eprintln!("通道 {} 相邻像素相关 {r:.3}（{n} 对）", image.apids[ch]);
    assert!(r > 0.8, "通道 {} 相邻像素相关只有 {r:.3}", image.apids[ch]);
  }

  if let Some(out) = std::env::var_os("LRPT_REAL_PPM") {
    let mut ppm = format!("P6\n{} {}\n255\n", image.width, image.height).into_bytes();
    for y in 0..image.height {
      for x in 0..image.width {
        for ch in 0..3 {
          ppm.push(image.channels[ch][y][x]);
        }
      }
    }
    std::fs::write(&out, ppm).expect("写 PPM");
    eprintln!("已写出 {out:?}");
  }
}
