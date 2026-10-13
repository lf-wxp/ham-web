//! DominoEX 数字模式解码器：上传 DominoEX 录音（WAV），交由 Web Worker 后台解码出文本。

mod dominoex_decoder_page;
mod dominoex_worker;

pub use dominoex_decoder_page::DominoexDecoderPage;
