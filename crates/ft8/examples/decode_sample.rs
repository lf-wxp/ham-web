//! 命令行解码一个 FT8 / FT4 WAV 文件并打印消息，用于验证解码器。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-ft8 --release --example decode_sample -- <wav 路径> [ft4]
//! ```

use ham_web_ft8::{Protocol, RecordingError, decode_recording};

fn main() {
  let mut args = std::env::args().skip(1);
  let Some(path) = args.next() else {
    eprintln!("用法：decode_sample <wav 路径> [ft4]");
    std::process::exit(1);
  };
  let protocol = if args.next().as_deref() == Some("ft4") {
    Protocol::Ft4
  } else {
    Protocol::Ft8
  };
  let bytes = std::fs::read(&path).expect("读取文件失败");
  let (samples, rate) = ham_web_apt::parse_wav(&bytes).expect("解析 WAV 失败");

  // 与页面 / Worker 走同一个入口，样例的结果就是页面上会看到的结果。
  let recording = match decode_recording(&samples, rate, protocol) {
    Ok(r) => r,
    Err(RecordingError::SampleRate(rate)) => {
      eprintln!("采样率须为 12000 Hz（当前 {rate} Hz）");
      std::process::exit(1);
    }
  };

  if recording.messages.is_empty() {
    eprintln!("未解码到消息（共 {} 个时隙）", recording.slots);
  } else {
    for m in &recording.messages {
      println!(
        "  {:6.1}s {:6.0}Hz 同步 {:+.1} dB -> {}",
        m.time_sec, m.freq_hz, m.sync_db, m.text
      );
    }
    println!(
      "共解码 {} 条消息（{} 个时隙）",
      recording.messages.len(),
      recording.slots
    );
  }
  if recording.unsupported > 0 {
    println!("另有 {} 条消息的类型暂不支持解析", recording.unsupported);
  }
}
