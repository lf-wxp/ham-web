//! 生成一段合成的 SSTV 音频样本（WAV），用于手动测试解码器。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-sstv --release --example generate_sample -- [输出.wav] [VIS码] [噪声幅度]
//! ```
//! - VIS 码：默认 44（Martin M1）；支持 44/40（Martin）、60/56/76（Scottie）、8/12（Robot）。
//! - 噪声幅度：默认 0.02，加在 FM 信号上模拟真实接收；0 表示无噪声。
//!
//! 图像内容（便于验证解码）：左 → 右的灰度渐变；Robot 的色差置中性（输出灰度）。
//!
//! 编码器本身在库里（[`ham_web_sstv::encode_image`]，页面的 SSTV 发端用的也是它）：它独立于解码器、
//! 按规格文本写，所以这里生成的样本仍是对解码器的独立检验；示例只负责造渐变图、加噪声、写 WAV。

use ham_web_sstv::{MODES, WORK_RATE, encode_image};

fn main() {
  let mut args = std::env::args().skip(1);
  let out_path = args.next().unwrap_or_else(|| "sstv-sample.wav".to_owned());
  let vis: u8 = args.next().and_then(|s| s.parse().ok()).unwrap_or(44);
  let noise: f32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(0.02);

  let mode = MODES.iter().find(|m| m.vis_code == vis).unwrap_or_else(|| {
    let list = MODES
      .iter()
      .map(|m| format!("{}（{}）", m.name, m.vis_code))
      .collect::<Vec<_>>()
      .join("、");
    panic!("未知 VIS 码 {vis}，支持：{list}");
  });

  // 左 → 右的灰度渐变。
  let rgb: Vec<u8> = (0..mode.width * mode.height)
    .flat_map(|i| {
      let col = i % mode.width;
      let v = (col * 255 / (mode.width - 1)) as u8;
      [v, v, v]
    })
    .collect();
  let mut signal = encode_image(mode, &rgb).unwrap_or_else(|e| panic!("编码失败：{}", e.code()));
  if noise > 0.0 {
    ham_web_dsp::add_noise(&mut signal, noise);
  }
  let wav = ham_web_wav::encode(&signal, WORK_RATE);
  std::fs::write(&out_path, &wav).expect("写入 WAV 失败");
  println!(
    "已生成 {out_path}：模式 {}，{}×{}，{} 样本 ≈ {:.1}s，噪声幅度 {noise:.2}",
    mode.name,
    mode.width,
    mode.height,
    signal.len(),
    signal.len() as f32 / WORK_RATE as f32
  );
}
