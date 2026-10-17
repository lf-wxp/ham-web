//! 现代数字模式：JS8Call / FreeDV / VARA / ARDOP / M17。
//!
//! 五个都是「用软件（声卡或专用硬件）实现的现代数字模式」，覆盖两类需求：
//! - **数字语音**：FreeDV（HF）、M17（VHF/UHF）；
//! - **数据 / 邮件**：JS8Call（弱信号键盘聊天）、VARA / ARDOP（Winlink 邮件与文件）。
//!
//! 带宽与类型口径与 [`crate::modes`] 里的 [`crate::modes::DIGITAL_MODES`] 一致；这里展开用途、
//! 开源 / 闭源与选择建议。开源状态与开发者以公开资料为准（VARA 闭源免费、ARDOP / JS8Call / FreeDV / M17 开源）。

/// 核心概念（名称 / 说明）。
pub const MODERN_DIGITAL_CONCEPTS: &[(&str, &str)] = &[
  (
    "JS8Call",
    "基于 FT8 信号结构的弱信号键盘聊天：15 秒帧、约 50 Hz 带宽，支持自由文本、消息转发与组网，是应急通信（NBEMS）常用的开源软件。",
  ),
  (
    "FreeDV",
    "基于开源 Codec2 声码器的 HF 数字语音，约 1.1 kHz 带宽，用数字方式在窄带信道里传话音，低信噪比下比 SSB 更可懂。",
  ),
  (
    "VARA",
    "EA5HVK 开发的自适应速率 ARQ 调制解调器（VARA HF / VARA FM），基于 OFDM；Winlink 邮件与文件传输的主力，闭源但免费、仅 Windows。",
  ),
  (
    "ARDOP",
    "Amateur Radio Digital Open Protocol，Winlink 团队发起的开源 ARQ 数据协议，声卡实现，可作为 PACTOR 的开源替代；开发重心已让位于 VARA。",
  ),
  (
    "M17",
    "完全开源的新一代数字语音与数据协议，基于 Codec2 声码器，4FSK、4800 符号/秒、约 9 kHz，是 VHF/UHF 上 DMR / C4FM 的开放替代方案。",
  ),
  (
    "Codec2 声码器",
    "开源的低比特率语音声码器，把话音压到几百到几千比特每秒；FreeDV 与 M17 共用，是两者「开源数字语音」的技术基座。",
  ),
  (
    "ARQ",
    "自动重传请求：接收方校验发现错误就要求重发，直到完整收到。邮件、文件这类「一个比特都不能错」的传输靠它保证完整。",
  ),
];

/// 模式对比（模式 / 类型与带宽 / 开源与用途）。
pub const MODERN_DIGITAL_TABLE: &[(&str, &str, &str)] = &[
  (
    "JS8Call",
    "数据 · 约 50 Hz",
    "开源；弱信号键盘聊天、消息转发，应急 NBEMS。",
  ),
  (
    "FreeDV",
    "数字语音 · 约 1.1 kHz",
    "开源（Codec2）；HF 窄带数字话音。",
  ),
  (
    "VARA",
    "数据 · 约 500 Hz / 2.3 kHz",
    "闭源（免费，仅 Windows）；Winlink 邮件、文件，自适应速率。",
  ),
  (
    "ARDOP",
    "数据 · 约 500 Hz / 1.6 kHz",
    "开源；Winlink 邮件，可替代 PACTOR。",
  ),
  (
    "M17",
    "数字语音 / 数据 · 约 9 kHz",
    "开源（Codec2）；VHF/UHF 数字语音，替代 DMR / C4FM。",
  ),
];

/// 要点。
pub const MODERN_DIGITAL_TIPS: &[&str] = &[
  "它们不互通：同为数字语音的 FreeDV 与 M17、同为邮件的 VARA 与 ARDOP，都只能和同模式的台站通信。",
  "开源与闭源的差别：FreeDV / M17 / ARDOP / JS8Call 开源、可审计；VARA 闭源但免费、目前只有 Windows。",
  "应急通信两条线：JS8Call 弱信号组网聊天，Winlink（VARA / ARDOP）收发邮件。",
  "数字语音按频段选：HF 用 FreeDV，VHF/UHF 用 M17（或各厂家的 DMR / C4FM）。",
  "选哪个先看对端用什么：这些模式都是「两端得一致」，不是通用通话模式。",
];
