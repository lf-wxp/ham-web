//! VNA 读数：导入 `.s1p` 或经 Web Serial 读 NanoVNA，查看实测 S11。
//!
//! 解析、阻抗换算、最低驻波与可用带宽都在核心 [`ham_web_core::touchstone`]（有单测）；
//! 这里只有串口收发、文件读取与两张图。页面一个文件一个组件：页面壳
//! [`vna_reader_page::VnaReaderPage`]、Smith 圆图轨迹 [`locus::S11Locus`]、
//! 驻波曲线 [`swr_plot::SwrPlot`]。

mod locus;
mod swr_plot;
mod tdr_panel;
mod vna_reader_page;

pub use vna_reader_page::VnaReaderPage;

use ham_web_core::touchstone::ParseError;

use crate::i18n::{t, tf};

/// 图上的说明文字样式。
const NOTE: &str = "text-xs text-muted-foreground";

/// 频率文案：≥ 1 MHz 用 MHz（3 位小数），否则 kHz。
fn fmt_hz(hz: f64) -> String {
  if hz >= 1e6 {
    format!("{:.3} MHz", hz / 1e6)
  } else {
    format!("{:.1} kHz", hz / 1e3)
  }
}

/// 解析失败的原因 → 界面文案（每种都写成字面量 key，`check-i18n` 才认得出）。
fn parse_error_text(e: &ParseError) -> String {
  match e {
    ParseError::NoData => t("tools.vna-err-no-data"),
    ParseError::Unsupported(what) => tf("tools.vna-err-unsupported", &[what]),
    ParseError::NotOnePort(line) => tf("tools.vna-err-not-one-port", &[&line.to_string()]),
    ParseError::BadNumber(line) => tf("tools.vna-err-bad-number", &[&line.to_string()]),
    ParseError::NotIncreasing(line) => tf("tools.vna-err-not-increasing", &[&line.to_string()]),
    ParseError::LengthMismatch(a, b) => {
      tf("tools.vna-err-length", &[&a.to_string(), &b.to_string()])
    }
  }
}
