//! WEFAX 短波气象传真解码器：上传录音（WAV），交由 Web Worker 后台解码出灰度图。

mod wefax_decoder_page;
mod wefax_worker;

pub use wefax_decoder_page::WefaxDecoderPage;
