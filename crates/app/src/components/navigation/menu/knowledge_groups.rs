use super::types::{NavGroup, NavItem};
use crate::icons::IconKind;

/// 知识库分组（桌面端以分组多列展示）。
pub(crate) const KNOWLEDGE_GROUPS: &[NavGroup] = &[
  NavGroup {
    label: "备考速查",
    icon: IconKind::BookMarked,
    items: &[
      NavItem {
        href: "/reference",
        label: "考试速查",
        icon: IconKind::BookMarked,
      },
      NavItem {
        href: "/cheat-sheet",
        label: "考点速查手册",
        icon: IconKind::ClipboardList,
      },
      NavItem {
        href: "/confusables",
        label: "易混淆辨析",
        icon: IconKind::ArrowUpDown,
      },
      NavItem {
        href: "/formulas",
        label: "公式速查",
        icon: IconKind::Calculator,
      },
      NavItem {
        href: "/prefixes",
        label: "呼号前缀",
        icon: IconKind::Globe,
      },
      NavItem {
        href: "/glossary",
        label: "术语表",
        icon: IconKind::BookOpen,
      },
      NavItem {
        href: "/q-code",
        label: "简语",
        icon: IconKind::Radio,
      },
      NavItem {
        href: "/phonetic",
        label: "字母解释法",
        icon: IconKind::CaseUpper,
      },
      NavItem {
        href: "/rst",
        label: "RST 信号报告",
        icon: IconKind::Signal,
      },
      NavItem {
        href: "/morse",
        label: "莫尔斯电码",
        icon: IconKind::AudioLines,
      },
      NavItem {
        href: "/cw-operating",
        label: "CW 操作（等幅波）",
        icon: IconKind::AudioLines,
      },
      NavItem {
        href: "/license-classes",
        label: "操作证权限",
        icon: IconKind::BadgeCheck,
      },
    ],
  },
  NavGroup {
    label: "模式 · 传播",
    icon: IconKind::Binary,
    items: &[
      NavItem {
        href: "/analog-modes",
        label: "模拟模式",
        icon: IconKind::Volume2,
      },
      NavItem {
        href: "/atv",
        label: "业余电视",
        icon: IconKind::Tv,
      },
      NavItem {
        href: "/modes",
        label: "数字模式",
        icon: IconKind::Binary,
      },
      NavItem {
        href: "/dv-network",
        label: "数字语音组网",
        icon: IconKind::Network,
      },
      NavItem {
        href: "/rtty",
        label: "RTTY 无线电传 / PSK31",
        icon: IconKind::Type,
      },
      NavItem {
        href: "/ft8",
        label: "FT8 / FT4（数字模式）",
        icon: IconKind::Binary,
      },
      NavItem {
        href: "/sdr",
        label: "SDR 软件定义无线电",
        icon: IconKind::Monitor,
      },
      NavItem {
        href: "/gnuradio",
        label: "GNU Radio",
        icon: IconKind::Workflow,
      },
      NavItem {
        href: "/aprs",
        label: "APRS 自动位置报告",
        icon: IconKind::Map,
      },
      NavItem {
        href: "/frequencies",
        label: "常用频率",
        icon: IconKind::Gauge,
      },
      NavItem {
        href: "/propagation",
        label: "传播与电离层",
        icon: IconKind::Waves,
      },
      NavItem {
        href: "/special-prop",
        label: "特殊传播",
        icon: IconKind::Sparkles,
      },
      NavItem {
        href: "/eme",
        label: "EME 月面反射（地月地）",
        icon: IconKind::Orbit,
      },
      NavItem {
        href: "/muf",
        label: "传播预测",
        icon: IconKind::TrendingUp,
      },
      NavItem {
        href: "/wspr",
        label: "WSPR 弱信号传播",
        icon: IconKind::Waves,
      },
      NavItem {
        href: "/sstv",
        label: "SSTV 慢扫描电视",
        icon: IconKind::Camera,
      },
      NavItem {
        href: "/weather-sat",
        label: "气象卫星接收",
        icon: IconKind::Satellite,
      },
      NavItem {
        href: "/packet",
        label: "Packet 分组无线电",
        icon: IconKind::Network,
      },
    ],
  },
  NavGroup {
    label: "天线 · 设备",
    icon: IconKind::RadioTower,
    items: &[
      NavItem {
        href: "/antennas",
        label: "天线型式",
        icon: IconKind::RadioTower,
      },
      NavItem {
        href: "/antenna-array",
        label: "天线阵列 / 相控阵",
        icon: IconKind::Waypoints,
      },
      NavItem {
        href: "/polarization",
        label: "天线极化",
        icon: IconKind::ArrowUpDown,
      },
      NavItem {
        href: "/feedline",
        label: "天线匹配与馈线",
        icon: IconKind::PlugZap,
      },
      NavItem {
        href: "/antenna-diy",
        label: "天线 DIY",
        icon: IconKind::Wrench,
      },
      NavItem {
        href: "/antenna-installation",
        label: "天线架设",
        icon: IconKind::Hammer,
      },
      NavItem {
        href: "/antenna-farm",
        label: "天线农场",
        icon: IconKind::Waypoints,
      },
      NavItem {
        href: "/antenna-tuning",
        label: "天线调试",
        icon: IconKind::Wrench,
      },
      NavItem {
        href: "/antenna-analyzer",
        label: "天线分析仪",
        icon: IconKind::Gauge,
      },
      NavItem {
        href: "/antenna-modeling",
        label: "天线建模",
        icon: IconKind::Box,
      },
      NavItem {
        href: "/nvis",
        label: "NVIS 近垂直入射天波",
        icon: IconKind::Cloud,
      },
      NavItem {
        href: "/electronics",
        label: "电子电路基础",
        icon: IconKind::Cpu,
      },
      NavItem {
        href: "/filters",
        label: "滤波器与双工器",
        icon: IconKind::Filter,
      },
      NavItem {
        href: "/meters",
        label: "测量仪表",
        icon: IconKind::Activity,
      },
      NavItem {
        href: "/power",
        label: "电源与电池",
        icon: IconKind::BatteryCharging,
      },
      NavItem {
        href: "/power-supply",
        label: "电源供应",
        icon: IconKind::Zap,
      },
      NavItem {
        href: "/transceiver",
        label: "收发信机",
        icon: IconKind::Radio,
      },
      NavItem {
        href: "/receiver",
        label: "接收机指标",
        icon: IconKind::Gauge,
      },
      NavItem {
        href: "/amplifier",
        label: "功率放大器",
        icon: IconKind::Flame,
      },
      NavItem {
        href: "/bands",
        label: "波段表",
        icon: IconKind::Satellite,
      },
      NavItem {
        href: "/bandplan",
        label: "波段规划",
        icon: IconKind::SlidersHorizontal,
      },
      NavItem {
        href: "/microwave",
        label: "微波通信",
        icon: IconKind::Radio,
      },
      NavItem {
        href: "/mobile",
        label: "车载 / 移动电台",
        icon: IconKind::RadioTower,
      },
    ],
  },
  NavGroup {
    label: "通联 · 活动",
    icon: IconKind::MessagesSquare,
    items: &[
      NavItem {
        href: "/operating",
        label: "通联实务",
        icon: IconKind::MessagesSquare,
      },
      NavItem {
        href: "/contest",
        label: "通联竞赛",
        icon: IconKind::Trophy,
      },
      NavItem {
        href: "/cabrillo",
        label: "竞赛日志 Cabrillo",
        icon: IconKind::FileText,
      },
      NavItem {
        href: "/awards",
        label: "DX 奖状",
        icon: IconKind::Award,
      },
      NavItem {
        href: "/iota",
        label: "IOTA 海岛通联（空中岛屿）",
        icon: IconKind::Anchor,
      },
      NavItem {
        href: "/dx",
        label: "DX 远距离通信技巧",
        icon: IconKind::Globe,
      },
      NavItem {
        href: "/dxpedition",
        label: "DX 远征",
        icon: IconKind::Plane,
      },
      NavItem {
        href: "/most-wanted",
        label: "DXCC 世纪俱乐部",
        icon: IconKind::Star,
      },
      NavItem {
        href: "/qrp",
        label: "QRP 低功率",
        icon: IconKind::BatteryLow,
      },
      NavItem {
        href: "/eqsl",
        label: "电子 QSL",
        icon: IconKind::Send,
      },
      NavItem {
        href: "/qsl-card",
        label: "QSL 卡片设计",
        icon: IconKind::Mail,
      },
      NavItem {
        href: "/ardf",
        label: "无线电测向",
        icon: IconKind::Compass,
      },
      NavItem {
        href: "/emcomm",
        label: "应急通信",
        icon: IconKind::Siren,
      },
      NavItem {
        href: "/portable",
        label: "SOTA 山顶 / POTA 公园",
        icon: IconKind::Mountain,
      },
      NavItem {
        href: "/grid",
        label: "网格定位",
        icon: IconKind::Map,
      },
      NavItem {
        href: "/repeater",
        label: "中继台与网关",
        icon: IconKind::RadioTower,
      },
      NavItem {
        href: "/repeater-build",
        label: "中继台建设",
        icon: IconKind::Building2,
      },
      NavItem {
        href: "/logging-software",
        label: "日志与竞赛软件",
        icon: IconKind::ClipboardList,
      },
    ],
  },
  NavGroup {
    label: "进阶 · 关于",
    icon: IconKind::Landmark,
    items: &[
      NavItem {
        href: "/organizations",
        label: "国际组织与分区",
        icon: IconKind::Landmark,
      },
      NavItem {
        href: "/safety",
        label: "射频安全",
        icon: IconKind::ShieldAlert,
      },
      NavItem {
        href: "/grounding",
        label: "接地与防雷",
        icon: IconKind::Zap,
      },
      NavItem {
        href: "/rfi",
        label: "射频干扰排查",
        icon: IconKind::ShieldX,
      },
      NavItem {
        href: "/beginner",
        label: "新手入门",
        icon: IconKind::GraduationCap,
      },
      NavItem {
        href: "/swl",
        label: "SWL 短波监听",
        icon: IconKind::Headphones,
      },
      NavItem {
        href: "/license",
        label: "执照申办",
        icon: IconKind::BadgeCheck,
      },
      NavItem {
        href: "/regulations",
        label: "法规与管理",
        icon: IconKind::Scale,
      },
      NavItem {
        href: "/history",
        label: "业余无线电历史",
        icon: IconKind::History,
      },
      NavItem {
        href: "/remote",
        label: "远程电台",
        icon: IconKind::Globe,
      },
    ],
  },
];
