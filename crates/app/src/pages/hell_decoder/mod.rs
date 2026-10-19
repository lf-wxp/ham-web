//! Feld Hell 解码器：上传 Feld Hell 录音（WAV），交由 Web Worker 后台解码出文本。

mod hell_decoder_page;
mod hell_worker;

pub use hell_decoder_page::HellDecoderPage;
