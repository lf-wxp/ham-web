//! 业余无线电数字语音与数据模式速查。

/// 一种数字模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DigitalMode {
  pub name: &'static str,
  pub abbr: &'static str,
  /// 类型：数字语音 / 数据。
  pub kind: &'static str,
  pub bandwidth: &'static str,
  pub desc: &'static str,
}

const fn mode(
  name: &'static str,
  abbr: &'static str,
  kind: &'static str,
  bandwidth: &'static str,
  desc: &'static str,
) -> DigitalMode {
  DigitalMode {
    name,
    abbr,
    kind,
    bandwidth,
    desc,
  }
}

/// 常见数字语音与数据模式。
pub const DIGITAL_MODES: &[DigitalMode] = &[
  mode(
    "D-STAR",
    "D-STAR",
    "数字语音 / 数据",
    "6.25 kHz",
    "日本业余无线电联盟制定的数字语音与数据标准，支持中继台联网。",
  ),
  mode(
    "数字对讲",
    "DMR",
    "数字语音 / 数据",
    "12.5 kHz（TDMA 两路）",
    "欧洲电信标准协会标准，采用时分多址，一个信道可同时传两路语音。",
  ),
  mode(
    "融合数字语音",
    "YSF / C4FM",
    "数字语音 / 数据",
    "12.5 kHz",
    "八重洲 System Fusion，采用 C4FM 调制，可与模拟调频兼容。",
  ),
  mode(
    "开源数字语音",
    "FreeDV",
    "数字语音",
    "约 1.1 kHz",
    "基于开源 Codec2 声码器，可在 HF 窄带中传输数字话音。",
  ),
  mode(
    "FT8",
    "FT8",
    "数据",
    "约 50 Hz",
    "15 秒一个周期的弱信号数字模式，能在很弱的信号下完成简短通联。",
  ),
  mode(
    "FT4",
    "FT4",
    "数据",
    "约 90 Hz",
    "7.5 秒一个周期，适合竞赛等快速交换的弱信号模式。",
  ),
  mode(
    "JT65",
    "JT65",
    "数据",
    "约 180 Hz",
    "1 分钟一个周期，常用于 EME 月面反射与 HF 小功率。",
  ),
  mode(
    "PSK31",
    "PSK31",
    "数据",
    "约 31 Hz",
    "31.25 波特的窄带相移键控文字聊天模式。",
  ),
  mode(
    "无线电传",
    "RTTY",
    "数据",
    "约 270 Hz",
    "用频移键控传送文字，业余常用 45.45 波特、频移 170Hz。",
  ),
  mode(
    "慢扫描电视",
    "SSTV",
    "数据（图像）",
    "话音带宽内",
    "在话音带宽内逐行传送静止图像，国际空间站常开展 SSTV 活动。",
  ),
  mode(
    "位置报告",
    "APRS",
    "数据（AX.25）",
    "分组信道",
    "通过分组数据自动上报位置和短信息，国内常用 144.640MHz。",
  ),
  mode(
    "分组无线电",
    "Packet",
    "数据（AX.25）",
    "分组信道",
    "把数据分成带地址的数据包传输，常用 AX.25 协议。",
  ),
  mode(
    "弱信号报告",
    "WSPR",
    "数据",
    "约 6 Hz",
    "超窄带弱信号信标模式，用于探测与报告电波传播。",
  ),
  mode(
    "弱信号聊天",
    "JS8Call",
    "数据",
    "约 50 Hz",
    "基于 FT8 的信号结构，支持键盘实时文字聊天。",
  ),
];

/// 按用途选择数字模式。
pub const DIGITAL_USAGE: &[(&str, &str)] = &[
  ("弱信号远距离", "FT8 / FT4 / Q65 —— 极低信噪比下可靠解码。"),
  ("键盘实时聊天", "JS8Call / PSK31 / RTTY —— 逐字实时交流。"),
  ("竞赛快速交换", "FT4 / RTTY —— 周期短、交换快。"),
  ("EME 月面反射", "JT65 / Q65 —— 对极弱信号与多普勒优化。"),
  ("图像传输", "SSTV / 数字 SSTV —— 静止图像。"),
  ("位置与信标", "APRS / WSPR —— 广播位置或探测传播。"),
  (
    "应急邮件",
    "Winlink（Pactor / ARDOP / VARA）—— 收发电子邮件。",
  ),
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn modes_are_populated() {
    assert!(DIGITAL_MODES.len() >= 10);
    for m in DIGITAL_MODES {
      assert!(!m.name.is_empty());
      assert!(!m.abbr.is_empty());
      assert!(!m.bandwidth.is_empty());
    }
    assert!(DIGITAL_USAGE.len() >= 5);
  }
}
