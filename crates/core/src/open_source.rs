//! 开源项目与 DIY 教程索引：把社区里成熟的开源软件与站内自制教程汇总成一张清单。

/// 一个开源项目。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OssProject {
  /// 项目名。
  pub name: &'static str,
  /// 用途分类（见 [`OSS_CATEGORIES`]）。
  pub category: &'static str,
  /// 一句话说明。
  pub desc: &'static str,
  /// 项目官网或代码仓库（外部链接）。
  pub url: &'static str,
}

/// 分类展示顺序。
pub const OSS_CATEGORIES: &[&str] = &[
  "SDR 与信号处理",
  "数字模式",
  "日志与竞赛",
  "卫星与空间",
  "网络与网关",
  "硬件与工具",
];

/// 精选开源项目（链接指向各项目官方网站或代码仓库）。
pub const OSS_PROJECTS: &[OssProject] = &[
  OssProject {
    name: "GNU Radio",
    category: "SDR 与信号处理",
    desc: "开源 SDR 信号处理框架，用流程图搭建自己的收发链路。",
    url: "https://www.gnuradio.org/",
  },
  OssProject {
    name: "SDR++",
    category: "SDR 与信号处理",
    desc: "跨平台、界面现代的 SDR 接收软件，支持多种硬件。",
    url: "https://www.sdrpp.org/",
  },
  OssProject {
    name: "SDRangel",
    category: "SDR 与信号处理",
    desc: "收发一体的 SDR 软件，集成了多种数字模式解调。",
    url: "https://github.com/f4exb/sdrangel",
  },
  OssProject {
    name: "Gqrx",
    category: "SDR 与信号处理",
    desc: "Linux / macOS 常用 SDR 接收软件，基于 GNU Radio。",
    url: "https://gqrx.dk/",
  },
  OssProject {
    name: "CubicSDR",
    category: "SDR 与信号处理",
    desc: "跨平台 SDR 接收与解调软件，界面简洁。",
    url: "https://cubicsdr.com/",
  },
  OssProject {
    name: "rtl-sdr",
    category: "SDR 与信号处理",
    desc: "廉价 RTL-SDR 硬件的驱动与入门百科，新手首选。",
    url: "https://www.rtl-sdr.com/",
  },
  OssProject {
    name: "OpenWebRX",
    category: "SDR 与信号处理",
    desc: "可自建的开源 Web SDR 软件，支持多用户远程收听与解码。",
    url: "https://www.openwebrx.org/",
  },
  OssProject {
    name: "WSJT-X",
    category: "数字模式",
    desc: "FT8 / FT4 / JT65 等弱信号数字模式的事实标准，开源跨平台。",
    url: "https://wsjt.sourceforge.io/",
  },
  OssProject {
    name: "JTDX",
    category: "数字模式",
    desc: "基于 WSJT-X 改进的 FT8 / FT4 解码器，弱信号解码表现强。",
    url: "https://www.jtdx.tech/",
  },
  OssProject {
    name: "JS8Call",
    category: "数字模式",
    desc: "基于 FT8 波形改造的弱信号键盘聊天，适合应急与低功率通联。",
    url: "http://js8call.com/",
  },
  OssProject {
    name: "fldigi",
    category: "数字模式",
    desc: "跨平台声卡数字模式套件，覆盖 RTTY、PSK、CW、Olivia 等。",
    url: "https://www.w1hkj.com/",
  },
  OssProject {
    name: "Cloudlog",
    category: "日志与竞赛",
    desc: "自托管的 Web 通联日志，支持 LoTW、Club Log 等集成。",
    url: "https://github.com/magicbug/Cloudlog",
  },
  OssProject {
    name: "GridTracker",
    category: "日志与竞赛",
    desc: "WSJT-X 伴侣软件，在网格地图上实时显示 FT8 通联。",
    url: "https://gridtracker.org/",
  },
  OssProject {
    name: "Log4OM",
    category: "日志与竞赛",
    desc: "Windows 平台的通联日志与竞赛 / 奖状管理软件。",
    url: "https://www.log4om.com/",
  },
  OssProject {
    name: "QLog",
    category: "日志与竞赛",
    desc: "跨平台桌面通联日志，支持 ADIF 与数据库存储。",
    url: "https://github.com/foldynl/QLog",
  },
  OssProject {
    name: "GPredict",
    category: "卫星与空间",
    desc: "卫星轨道跟踪与过境预测，支持多颗卫星与电台联动。",
    url: "https://gpredict.oz9aec.net/",
  },
  OssProject {
    name: "SatDump",
    category: "卫星与空间",
    desc: "气象与业余卫星解码套件，支持 NOAA、Meteor 等。",
    url: "https://github.com/satdump/satdump",
  },
  OssProject {
    name: "Dire Wolf",
    category: "网络与网关",
    desc: "软件 APRS / AX.25 终端节点控制器，可配合声卡使用。",
    url: "https://github.com/wb2osz/direwolf",
  },
  OssProject {
    name: "Pat",
    category: "网络与网关",
    desc: "跨平台 Winlink 客户端，支持多种传输后端。",
    url: "https://getpat.io/",
  },
  OssProject {
    name: "M17 Project",
    category: "网络与网关",
    desc: "开源数字语音协议与实现，面向开放的语音通信生态。",
    url: "https://m17project.org/",
  },
  OssProject {
    name: "Pi-Star",
    category: "网络与网关",
    desc: "数字语音热点系统，支持 DMR、D-Star、YSF、P25 等。",
    url: "https://pistar.uk/",
  },
  OssProject {
    name: "Hamlib",
    category: "硬件与工具",
    desc: "电台 CAT 控制库，被大量日志与数字模式软件复用。",
    url: "https://hamlib.github.io/",
  },
  OssProject {
    name: "CHIRP",
    category: "硬件与工具",
    desc: "手台与车台的频率、信道管理软件，支持大量机型。",
    url: "https://chirpmyradio.com/",
  },
  OssProject {
    name: "KiCad",
    category: "硬件与工具",
    desc: "开源 EDA 套件，用于自制收发信机与电路板设计。",
    url: "https://www.kicad.org/",
  },
];

/// 站内 DIY 教程索引：`(路由, 标题, 说明)`。
pub const DIY_GUIDES: &[(&str, &str, &str)] = &[
  (
    "/antenna-diy",
    "偶极与简易天线制作",
    "从半波偶极到倒 V、垂直天线的 DIY 步骤与尺寸计算。",
  ),
  (
    "/balun",
    "巴伦与不平衡变压器绕制",
    "1:1 / 4:1 / 9:1 巴伦与共模扼流圈的绕制参数。",
  ),
  (
    "/feedline",
    "馈线与匹配",
    "同轴电缆损耗、驻波与匹配网络的选择与计算。",
  ),
  (
    "/antenna-modeling",
    "天线建模入门",
    "用建模软件仿真方向图与阻抗，动手前先算一算。",
  ),
  (
    "/antenna-tuning",
    "天线调试与分析",
    "用分析仪测量谐振点、带宽与驻波曲线。",
  ),
  (
    "/filters",
    "滤波器与双工器设计",
    "低通 / 高通 / 带通 / 陷波的元件计算与制作。",
  ),
  (
    "/electronics",
    "电子电路基础",
    "电阻、电容、晶体管与常用电路，动手前的必备知识。",
  ),
  (
    "/qrp",
    "QRP 低功率收发信机",
    "低功率自制与套件路线，小功率也能通 DX。",
  ),
  (
    "/sdr",
    "SDR 软件定义无线电",
    "从 RTL-SDR 到 GNU Radio，搭建自己的接收链路。",
  ),
  (
    "/gnuradio",
    "GNU Radio 入门",
    "用流程图搭建自己的调制解调与解码。",
  ),
  (
    "/packet",
    "Packet 分组无线电与 TNC",
    "用声卡与软件 TNC 搭建分组数据链路。",
  ),
  (
    "/repeater-build",
    "中继台建设与维护",
    "中继台的选型、链路与供电搭建要点。",
  ),
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn oss_projects_populated_and_categorized() {
    assert!(OSS_PROJECTS.len() >= 15);
    for p in OSS_PROJECTS {
      assert!(!p.name.is_empty());
      assert!(!p.desc.is_empty());
      assert!(p.url.starts_with("http"), "项目链接应为 URL：{}", p.url);
      assert!(
        OSS_CATEGORIES.contains(&p.category),
        "未知分类：{}",
        p.category
      );
    }
    // 每个分类都应有项目。
    for cat in OSS_CATEGORIES {
      assert!(
        OSS_PROJECTS.iter().any(|p| p.category == *cat),
        "分类无项目：{cat}"
      );
    }
  }

  #[test]
  fn diy_guides_are_internal_routes() {
    assert!(DIY_GUIDES.len() >= 8);
    for (href, name, desc) in DIY_GUIDES {
      assert!(href.starts_with('/'), "DIY 教程应为站内路由：{href}");
      assert!(!name.is_empty());
      assert!(!desc.is_empty());
    }
  }
}
