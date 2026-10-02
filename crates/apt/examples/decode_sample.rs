//! 命令行解码一个 APT WAV 文件并打印结果统计，用于验证解码器是否正常。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-apt --release --example decode_sample -- <wav 路径>
//! ```

use ham_web_apt::decode;

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
      // 用「中间」像素验证方向，避开紧邻 sync/遥测的边缘振铃。
      let a_left = img.channel_a[w / 8];
      let a_right = img.channel_a[w - w / 8];
      let b_top = img.channel_b[(h / 8) * w + w / 2];
      let b_bottom = img.channel_b[(h - h / 8) * w + w / 2];
      println!(
        "尺寸 {}×{}，行数 {}，原始采样率 {} Hz，时长 {:.1}s",
        img.width, img.height, img.lines, img.source_sample_rate, img.duration_seconds
      );
      println!("通道 A：左={a_left} 右={a_right}（合成样本应 左<右，即黑→白）");
      println!("通道 B：上={b_top} 下={b_bottom}（合成样本应 上<下，即黑→白）");
    }
    Err(e) => eprintln!("解码失败：{e}"),
  }
}
