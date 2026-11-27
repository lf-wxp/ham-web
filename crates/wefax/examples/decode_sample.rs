//! 命令行解码 WEFAX 样本 WAV。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-wefax --release --example decode_sample -- [输入.wav] [--fax [LPM] [IOC]] [--pgm 输出.pgm]
//! ```
//! - 默认按简化模型解（自动判断调幅 / 调频）；
//! - `--fax` 按**真实传真格式**解，后面可跟行速率（默认 120）与 IOC（默认 576）；
//! - `--pgm` 把解出的图存成 PGM（白 = 255），可用 `ffmpeg -i x.pgm x.png` 转成 PNG 看——
//!   这是人工目视核对真实录音的办法。

use ham_web_wefax::fax::{self, Ioc, Params};
use ham_web_wefax::{WefaxImage, decode_with};

fn main() {
  let args: Vec<String> = std::env::args().skip(1).collect();
  let path = args
    .first()
    .filter(|a| !a.starts_with("--"))
    .cloned()
    .unwrap_or_else(|| "wefax-sample.wav".to_owned());
  let value_after = |flag: &str| {
    args
      .iter()
      .position(|a| a == flag)
      .and_then(|i| args.get(i + 1))
  };
  let bytes = std::fs::read(&path).expect("读取 WAV 失败");

  let image: WefaxImage = if let Some(at) = args.iter().position(|a| a == "--fax") {
    let number = |offset: usize| args.get(at + offset).filter(|a| !a.starts_with("--"));
    let lpm = number(1).map_or(120, |v| v.parse().expect("LPM 要是整数"));
    let ioc = number(2)
      .and_then(|v| Ioc::from_code(v))
      .unwrap_or(Ioc::I576);
    match fax::decode(&bytes, Params { lpm, ioc }) {
      Ok(d) => {
        println!(
          "{}×{}（真实格式：实测 {:.3} LPM，相位线 {}，黑 {:.0} Hz / 白 {:.0} Hz，电平来源 {:?}）",
          d.image.width,
          d.image.height,
          d.lpm_measured,
          d.phasing_lines
            .map_or("未找到".to_owned(), |n| format!("{n} 条")),
          d.black_hz,
          d.white_hz,
          d.levels,
        );
        d.image
      }
      Err(e) => return eprintln!("解码失败：{e}"),
    }
  } else {
    match decode_with(&bytes, None) {
      Ok(d) => {
        println!(
          "{}×{}（简化模型：按{}解调）",
          d.image.width,
          d.image.height,
          match d.modulation {
            ham_web_wefax::Modulation::Fm => "调频副载波",
            ham_web_wefax::Modulation::Am => "调幅视频",
          }
        );
        d.image
      }
      Err(e) => return eprintln!("解码失败：{e}"),
    }
  };

  if let Some(out) = value_after("--pgm") {
    // 内部灰度 0 = 白、255 = 黑；PGM 反过来。
    let mut pgm = format!("P5\n{} {}\n255\n", image.width, image.height).into_bytes();
    pgm.extend(image.gray.iter().map(|g| 255 - g));
    std::fs::write(out, pgm).expect("写入 PGM 失败");
  }
}
