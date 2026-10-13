//! NAVTEX 解码器：上传 NAVTEX 录音（WAV），交由 Web Worker 后台解码出海事安全报文。

mod navtex_decoder_page;
mod navtex_worker;

pub use navtex_decoder_page::NavtexDecoderPage;
