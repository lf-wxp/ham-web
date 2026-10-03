//! 在线 SDR 接收站与公开目录索引：无需本地硬件即可在浏览器里收听短波。
//!
//! 只收录长期运行、地址稳定的公开接收站（可标注到地图）与接收机目录；
//! 单站链接可能随站主调整，具体以站点页面为准。

/// 一个可标注到地图的公开接收站。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SdrSite {
  /// 站名。
  pub name: &'static str,
  /// 平台类型（WebSDR / KiwiSDR / OpenWebRX）。
  pub kind: &'static str,
  /// 位置（国家 · 城市）。
  pub location: &'static str,
  /// 纬度。
  pub lat: f64,
  /// 经度。
  pub lon: f64,
  /// 覆盖频率范围。
  pub coverage: &'static str,
  /// 访问地址。
  pub url: &'static str,
  /// 一句话说明。
  pub desc: &'static str,
}

/// 长期运行的公开接收站（用于地图标注）。
pub const SDR_SITES: &[SdrSite] = &[
  SdrSite {
    name: "特温特大学 WebSDR",
    kind: "WebSDR",
    location: "荷兰 · 恩斯赫德",
    lat: 52.24,
    lon: 6.85,
    coverage: "0～29 MHz",
    url: "http://websdr.ewi.utwente.nl:8901/",
    desc: "最经典的公开 WebSDR 站点，覆盖长波到 10 米波段，界面与信号都很稳定。",
  },
  SdrSite {
    name: "KFS WebSDR",
    kind: "WebSDR",
    location: "美国 · 加州半月湾",
    lat: 37.40,
    lon: -122.45,
    coverage: "0～30 MHz",
    url: "https://www.kfsdr.com/",
    desc: "太平洋沿岸的 HF 接收阵列，多副方向天线分集，适合听北美与太平洋信号。",
  },
];

/// 公开接收机目录 / 平台（全球性，不标注到地图）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SdrDirectory {
  /// 名称。
  pub name: &'static str,
  /// 类型（目录 / 开源软件 / 远程控制）。
  pub kind: &'static str,
  /// 访问地址。
  pub url: &'static str,
  /// 一句话说明。
  pub desc: &'static str,
}

/// 公开接收机目录与自建平台。
pub const SDR_DIRECTORIES: &[SdrDirectory] = &[
  SdrDirectory {
    name: "WebSDR.org",
    kind: "目录",
    url: "http://websdr.org/",
    desc: "全球 WebSDR 站点汇总，按地区列出在线接收机。",
  },
  SdrDirectory {
    name: "KiwiSDR 公共列表",
    kind: "目录",
    url: "http://kiwisdr.com/public/",
    desc: "KiwiSDR 官方公共接收站列表，点地图上的站点即可收听。",
  },
  SdrDirectory {
    name: "Receiverbook",
    kind: "目录",
    url: "https://www.receiverbook.de/",
    desc: "社区维护的在线接收机目录，含地图与在线状态。",
  },
  SdrDirectory {
    name: "rx-tx.info",
    kind: "目录",
    url: "https://rx-tx.info/",
    desc: "在线接收机与收发站列表。",
  },
  SdrDirectory {
    name: "GlobalTuners",
    kind: "远程控制",
    url: "https://www.globaltuners.com/",
    desc: "远程收听公共接收机，部分站点还可远程控制频率与模式。",
  },
  SdrDirectory {
    name: "OpenWebRX",
    kind: "开源软件",
    url: "https://www.openwebrx.org/",
    desc: "可自建的开源 Web SDR 软件，支持多用户与内置解码。",
  },
];

/// 使用提示。
pub const SDR_MAP_TIPS: &[&str] = &[
  "在线接收机页面自带实时频谱与瀑布图：打开后拖动频率，即可看到各信号的强度随时间变化。",
  "出于安全策略，多数接收站禁止被其它网页以 iframe 内嵌，因此这里以新标签打开，而不是嵌进本页。",
  "选站先看「离发射地近不近」与「波段覆盖」：听本地中继和 FM 用 VHF 站，听 HF DX 用高纬 HF 站。",
  "可同时打开多个不同地区的接收机，对比同一条传播路径两端的信号强弱。",
  "公开接收机由站主自愿提供，请勿长时间占用单一频道，并遵守各站的使用规则。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn sdr_map_data_populated() {
    assert!(!SDR_SITES.is_empty());
    assert!(SDR_DIRECTORIES.len() >= 4);
    assert!(!SDR_MAP_TIPS.is_empty());
    for s in SDR_SITES {
      assert!(!s.name.is_empty());
      assert!(!s.location.is_empty());
      assert!(!s.coverage.is_empty());
      assert!(s.url.starts_with("http"));
      assert!(s.lat.abs() <= 90.0 && s.lon.abs() <= 180.0);
    }
    for d in SDR_DIRECTORIES {
      assert!(!d.name.is_empty());
      assert!(!d.desc.is_empty());
      assert!(d.url.starts_with("http"));
    }
  }
}
