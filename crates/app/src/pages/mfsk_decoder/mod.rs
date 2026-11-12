//! MFSK 数字模式解码器：上传 MFSK 录音（WAV），交由 Web Worker 后台解码出文本。

mod mfsk_decoder_page;
mod mfsk_worker;

pub use mfsk_decoder_page::MfskDecoderPage;
