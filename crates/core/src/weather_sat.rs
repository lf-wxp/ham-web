//! 气象卫星接收：NOAA APT、METEOR LRPT 与静止卫星图像。

/// 一颗气象卫星。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeatherSat {
  pub name: &'static str,
  pub signal: &'static str,
  pub freq: &'static str,
  pub note: &'static str,
}

/// 常见可接收的气象卫星。
pub const WEATHER_SATS: &[WeatherSat] = &[
  WeatherSat {
    name: "NOAA-19",
    signal: "APT",
    freq: "137.100 MHz",
    note: "自动图像传输，信号稳定，入门首选。",
  },
  WeatherSat {
    name: "NOAA-15",
    signal: "APT",
    freq: "137.620 MHz",
    note: "在轨较久，APT 信号间歇、时有时无，接收需耐心。",
  },
  WeatherSat {
    name: "NOAA-18",
    signal: "APT",
    freq: "137.9125 MHz",
    note: "已于 2025 年 6 月 6 日退役，仅作历史参考。",
  },
  WeatherSat {
    name: "METEOR-M2-3 / M2-4",
    signal: "LRPT",
    freq: "137.900 / 137.100 MHz",
    note: "俄罗斯卫星，LRPT 高分辨率彩色图像，现役主力。",
  },
  WeatherSat {
    name: "NOAA / METEOR HRPT",
    signal: "HRPT",
    freq: "1698–1707 MHz",
    note: "L 波段高分辨率图像，需定向高增益天线与下变频。",
  },
  WeatherSat {
    name: "GOES 系列",
    signal: "HRIT",
    freq: "1694 MHz",
    note: "静止气象卫星，需抛物面天线接收。",
  },
];

/// 核心概念。
pub const WEATHER_CONCEPTS: &[(&str, &str)] = &[
  (
    "APT",
    "Automatic Picture Transmission，模拟图像，普通 SDR 即可接收。",
  ),
  ("LRPT", "Low Rate Picture Transmission，数字高分辨率图像。"),
  ("WEFAX", "短波气象传真，用 SSB 接收各地气象台的气象图。"),
  ("QFH 天线", "四臂螺旋天线，气象卫星接收的优选全向天线。"),
];

/// 接收要点。
pub const WEATHER_TIPS: &[&str] = &[
  "用 RTL-SDR（数十元）+ V 型偶极或 QFH 天线即可入门。",
  "解码软件：SatDump、WXtoIMG（APT）；配合 Gpredict 预报过境。",
  "卫星每次过境仅约 10–15 分钟，需提前查过境时间并守听。",
  "接收时选开阔地、避开高楼遮挡，天线朝向卫星来向。",
];

/// APT 接收上手步骤：(标题, 描述, 可选跳转链接)。
pub const APT_GUIDE: &[(&str, &str, Option<&str>)] = &[
  (
    "① 准备设备",
    "RTL-SDR 接收棒 + 电脑即可接收 137 MHz 气象卫星，无需执照。",
    None,
  ),
  (
    "② 制作天线",
    "V 形偶极：两臂各约 53 cm、夹角约 120°，水平架设、开口朝南。",
    None,
  ),
  (
    "③ 安装软件",
    "SDR# / GQRX / SatDump，设为 WFM 模式、带宽 40 kHz。",
    Some("/sdr"),
  ),
  (
    "④ 等待过境",
    "查 NOAA 15/19 过境时间，开启 APT 录制提醒（提前几分钟通知）。",
    Some("/satellites"),
  ),
  (
    "⑤ 录制并解码",
    "过境时录制 10–15 分钟 WAV，上传到 APT 解码器本地重建云图。",
    Some("/apt-decoder"),
  ),
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn weather_sat_populated() {
    assert!(WEATHER_SATS.len() >= 4);
    for s in WEATHER_SATS {
      assert!(!s.name.is_empty());
      assert!(!s.freq.is_empty());
    }
    assert!(!WEATHER_CONCEPTS.is_empty());
    assert!(!WEATHER_TIPS.is_empty());
    assert!(APT_GUIDE.len() >= 5);
    for (title, desc, _) in APT_GUIDE {
      assert!(!title.is_empty() && !desc.is_empty());
    }
  }
}
