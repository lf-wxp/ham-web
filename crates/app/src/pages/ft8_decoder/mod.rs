//! FT8 / FT4 弱信号数字模式解码器：上传 15 秒录音（WAV，12000 Hz），交由 Web Worker 后台解码出消息列表。

mod ft8_decoder_page;
mod ft8_worker;

pub use ft8_decoder_page::Ft8DecoderPage;
