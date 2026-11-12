//! THOR 数字模式解码器：上传 THOR 录音（WAV），交由 Web Worker 后台解码出文本。

mod thor_decoder_page;
mod thor_worker;

pub use thor_decoder_page::ThorDecoderPage;
