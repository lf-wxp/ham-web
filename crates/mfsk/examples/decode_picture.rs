//! 命令行解码 MFSK 图片录音（WAV），把图片写成 PPM；旁边有同名的 `<wav>.rgb`（`reference/fldigi_web/gen_pic.mjs`
//! 生成的原图）时，顺带报告逐字节误差。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-mfsk --release --example decode_picture -- <MODE> <输入.wav> [中心频率Hz|auto] [输出.ppm]
//! ```
//! MODE ∈ 16 / 32 / 64 / 128。

use ham_web_mfsk::{decode_audio, parse_mode};

fn main() {
  let mut args = std::env::args().skip(1);
  let mode_name = args.next().unwrap_or_else(|| "16".to_owned());
  let path = args.next().expect("输入 WAV");
  let center = args.next().and_then(|c| c.parse::<f64>().ok());
  let out = args.next();

  let mode = parse_mode(&mode_name).expect("子模式（16/32/64/128）");
  let bytes = std::fs::read(&path).expect("读取 WAV 失败");
  let (samples, rate) = ham_web_wav::parse(&bytes).expect("解析 WAV 失败");
  let Some(d) = decode_audio(&samples, rate, mode, center, false) else {
    eprintln!("没有可解调的信号");
    std::process::exit(1);
  };
  println!("文本：{:?}", d.text);
  println!("中心 {:.1} Hz · 图片 {} 张", d.center_hz, d.pictures.len());
  let truth = std::fs::read(format!("{path}.rgb")).ok();
  for (i, p) in d.pictures.iter().enumerate() {
    println!(
      "图片 {i}：{} × {} {}",
      p.width,
      p.height,
      if p.color { "彩色" } else { "灰度" }
    );
    if let Some(want) = &truth
      && want.len() == p.rgb.len()
    {
      let errs: Vec<u32> = p
        .rgb
        .iter()
        .zip(want)
        .map(|(&a, &b)| u32::from(a).abs_diff(u32::from(b)))
        .collect();
      let mean = errs.iter().sum::<u32>() as f64 / errs.len() as f64;
      let max = errs.iter().max().copied().unwrap_or(0);
      let bad = errs.iter().filter(|&&e| e > 24).count();
      println!(
        "  与原图比：平均误差 {mean:.2}，最大 {max}，误差 > 24 的字节 {bad} / {}",
        errs.len()
      );
    }
    if let Some(out) = &out {
      let mut ppm = format!("P6\n{} {}\n255\n", p.width, p.height).into_bytes();
      ppm.extend_from_slice(&p.rgb);
      let name = if i == 0 {
        out.clone()
      } else {
        format!("{out}.{i}")
      };
      std::fs::write(name, ppm).expect("写 PPM 失败");
    }
  }
}
