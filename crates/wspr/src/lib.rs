//! WSPR（Weak Signal Propagation Reporter）Type 1 消息编码与解码。
//!
//! 纯 Rust 实现、无外部依赖，可同时用于 WASM 前端与命令行/测试。
//!
//! # 协议速览
//! - 消息 50 bit：呼号 28 bit + 网格 15 bit + 功率 7 bit；
//! - 卷积码 K=32、r=1/2，50 bit + 31 bit 零尾 → 162 bit；
//! - 交织（8 bit 反转）→ 与 162 bit 同步向量合并为 162 个符号（0–3）；
//! - 连续相位 4-FSK，音调间隔 1.46484375 Hz，符号时长 0.682667 s，总时长 110.592 s。

pub mod audio;
pub mod decode;
pub mod encode;

pub use audio::{decode_audio, synthesize};
pub use decode::{Message, decode_symbols};
pub use encode::{POWER_LEVELS, encode, sync_bit};

/// 信道符号数。
pub const SYMBOL_COUNT: usize = 162;
/// 音调间隔（Hz）。
pub const TONE_SPACING_HZ: f64 = 12000.0 / 8192.0;
/// 每个符号时长（秒）。
pub const SYMBOL_SECONDS: f64 = 8192.0 / 12000.0;
/// 参考采样率（Hz）。
pub const SAMPLE_RATE: u32 = 48_000;
/// 每符号样本数（48000 × 8192 / 12000）。
pub const SAMPLES_PER_SYMBOL: usize = 32_768;
