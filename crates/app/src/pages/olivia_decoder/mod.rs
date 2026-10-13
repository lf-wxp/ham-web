//! Olivia 数字模式解码器：上传 Olivia 录音（WAV），交由 Web Worker 后台解码出文本。

mod olivia_decoder_page;
mod olivia_worker;

pub use olivia_decoder_page::OliviaDecoderPage;
