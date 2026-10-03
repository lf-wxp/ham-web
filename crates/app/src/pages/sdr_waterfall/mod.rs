//! SDR 瀑布图：麦克风实时频谱 + 瀑布图，并支持离线上传音频分析。
//!
//! 把电台 / SDR 的音频输出接入麦克风，即可看到实时频谱折线与下滑的
//! 频率 - 时间 - 强度瀑布图，FFT 由浏览器原生 `AnalyserNode` 完成。
//! 离线部分上传 WAV 后用浏览器原生解码器整段分析并导出 PNG。
//! 全部在浏览器本地完成，音频不上传。

mod file_waterfall;
mod sdr_waterfall_page;

pub use sdr_waterfall_page::SdrWaterfallPage;

/// 显示窗口下限（dB）。
const FLOOR_DB: f32 = -100.0;
/// 显示窗口上限（dB）。
const CEIL_DB: f32 = 0.0;
