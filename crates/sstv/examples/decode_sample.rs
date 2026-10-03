//! 命令行解码一个 SSTV WAV 文件并打印结果统计，用于验证解码器是否正常。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-sstv --release --example decode_sample -- <wav 路径>
//! ```

use ham_web_sstv::decode;

fn main() {
  let Some(path) = std::env::args().nth(1) else {
    eprintln!("用法：decode_sample <wav 路径>");
    std::process::exit(1);
  };
  let bytes = std::fs::read(&path).expect("读取文件失败");

  match decode(&bytes) {
    Ok(img) => {
      let w = img.width as usize;
      let h = img.height as usize;
      // 用「中间行、左右 1/8」处的红色通道验证方向，避开边缘振铃。
      let left = img.rgba[((h / 2) * w + w / 8) * 4];
      let right = img.rgba[((h / 2) * w + w - w / 8) * 4];
      println!(
        "模式 {}，尺寸 {}×{}，原始采样率 {} Hz，时长 {:.1}s",
        img.mode, img.width, img.height, img.source_sample_rate, img.duration_seconds
      );
      println!("红色通道：左={left} 右={right}（合成样本应 左<右，即黑→白）");
    }
    Err(e) => eprintln!("解码失败：{e}"),
  }
}
