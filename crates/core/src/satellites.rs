//! 业余卫星入门：常用 FM 中继与线性转发器卫星、操作要点。

/// 一颗业余卫星。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Satellite {
  pub name: &'static str,
  pub callsign: &'static str,
  /// FM 中继 / 线性转发器。
  pub kind: &'static str,
  pub uplink: &'static str,
  pub downlink: &'static str,
  pub note: &'static str,
}

/// 常用业余卫星。
pub const SATELLITES: &[Satellite] = &[
  Satellite {
    name: "国际空间站",
    callsign: "RS0ISS / NA1SS",
    kind: "FM 中继 / APRS / SSTV",
    uplink: "145.990 MHz（67Hz 亚音）",
    downlink: "437.800 MHz（FM 中继下行）；SSTV 活动下行 145.800 MHz",
    note: "最常见也最容易收到；SSTV 图像活动使用 145.800 MHz FM，与 437.800 MHz 中继下行不同。",
  },
  Satellite {
    name: "SO-50",
    callsign: "SaudiSat-1C",
    kind: "FM 中继",
    uplink: "145.850 MHz（先发 74.4Hz 激活，再 67Hz）",
    downlink: "436.795 MHz",
    note: "入门首选 FM 中继卫星，需要掌握亚音设置。",
  },
  Satellite {
    name: "AO-91",
    callsign: "RadFxSat (Fox-1B)",
    kind: "FM 中继",
    uplink: "435.250 MHz（67Hz 亚音）",
    downlink: "145.960 MHz",
    note: "上行在 UHF、下行在 VHF，适合手持八木天线操作。",
  },
  Satellite {
    name: "AO-92",
    callsign: "Fox-1D",
    kind: "FM 中继",
    uplink: "435.350 MHz（67Hz 亚音）",
    downlink: "145.880 MHz",
    note: "与 AO-91 同属 Fox 系列，操作方式相近。",
  },
  Satellite {
    name: "AO-7",
    callsign: "AO-7",
    kind: "线性转发器",
    uplink: "432.125–432.175 MHz（模式 B）",
    downlink: "145.975–145.925 MHz",
    note: "1974 年发射的老卫星，至今仍可间歇工作，适合 SSB/CW。",
  },
  Satellite {
    name: "FO-29",
    callsign: "JAS-2",
    kind: "线性转发器",
    uplink: "145.900–146.000 MHz",
    downlink: "435.800–435.900 MHz",
    note: "日本线性转发器卫星，适合做 SSB/CW 双向通联。",
  },
  Satellite {
    name: "希望二号系列",
    callsign: "XW-2A–2F",
    kind: "线性转发器",
    uplink: "VHF / UHF（依卫星）",
    downlink: "VHF / UHF（依卫星）",
    note: "中国业余卫星系列，多颗在轨，转发器为线性（SSB/CW）。",
  },
  Satellite {
    name: "AO-73",
    callsign: "FUNcube-1",
    kind: "线性转发器 + 遥测",
    uplink: "435.150–435.130 MHz",
    downlink: "145.970–145.950 MHz",
    note: "英国 FUNcube 教育卫星，带遥测下行。",
  },
];

/// 操作要点。
pub const SATELLITE_TIPS: &[&str] = &[
  "低轨卫星每次过境约 10 分钟，需提前查过境时间并提前守听。",
  "多普勒效应会使频率漂移，通联中需微调收发频率。",
  "FM 中继卫星用 FM 并注意上行亚音；线性转发器用 SSB/CW。",
  "使用能同时收发的电台和指向性天线（如手持八木）。",
  "通联务必简短，只报呼号、信号报告和网格坐标即可。",
];

/// 卫星追踪与过境预报软件。
pub const TRACKING_SOFTWARE: &[(&str, &str)] = &[
  ("Gpredict", "开源跨平台，实时追踪、过境预报与多普勒显示。"),
  ("Orbitron", "Windows 经典卫星追踪软件，配合 SDR 接收。"),
  ("Heavens-Above", "在线过境预报，适合快速查询。"),
  ("HamSatDroid / ISS Detector", "手机 App，随时查看过境提醒。"),
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn satellites_populated() {
    assert!(SATELLITES.len() >= 5);
    for s in SATELLITES {
      assert!(!s.name.is_empty());
      assert!(!s.uplink.is_empty());
      assert!(!s.downlink.is_empty());
    }
    assert!(!SATELLITE_TIPS.is_empty());
    assert!(!TRACKING_SOFTWARE.is_empty());
  }
}
