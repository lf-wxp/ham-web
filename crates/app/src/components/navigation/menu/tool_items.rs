use super::types::{NavGroup, NavItem};
use crate::icons::IconKind;

/// 工具菜单（分组展示，与知识库对称）。
pub(crate) const TOOL_GROUPS: &[NavGroup] = &[
  NavGroup {
    label: "计算 · 解码",
    icon: IconKind::Calculator,
    items: &[
      NavItem {
        href: "/tools",
        label: "小工具",
        icon: IconKind::Calculator,
      },
      NavItem {
        href: "/apt-decoder",
        label: "APT 云图解码",
        icon: IconKind::Satellite,
      },
      NavItem {
        href: "/psk-decode",
        label: "PSK31 解码",
        icon: IconKind::AudioLines,
      },
      NavItem {
        href: "/photo-processor",
        label: "照片处理",
        icon: IconKind::Camera,
      },
    ],
  },
  NavGroup {
    label: "日志 · 竞赛",
    icon: IconKind::Trophy,
    items: &[
      NavItem {
        href: "/log",
        label: "通联日志",
        icon: IconKind::ClipboardList,
      },
      NavItem {
        href: "/qsl-labels",
        label: "QSL 标签打印",
        icon: IconKind::Mail,
      },
      NavItem {
        href: "/qsl-designer",
        label: "QSL 卡片设计",
        icon: IconKind::CreditCard,
      },
      NavItem {
        href: "/contest-log",
        label: "竞赛录入",
        icon: IconKind::Trophy,
      },
      NavItem {
        href: "/contest-calendar",
        label: "竞赛日历",
        icon: IconKind::Trophy,
      },
      NavItem {
        href: "/stats",
        label: "通联统计",
        icon: IconKind::TrendingUp,
      },
    ],
  },
  NavGroup {
    label: "实时数据",
    icon: IconKind::Activity,
    items: &[
      NavItem {
        href: "/dashboard",
        label: "实时仪表盘",
        icon: IconKind::LayoutGrid,
      },
      NavItem {
        href: "/dx-spots",
        label: "DX 实时热点",
        icon: IconKind::Radio,
      },
      NavItem {
        href: "/psk-reporter",
        label: "PSK Reporter",
        icon: IconKind::Radio,
      },
      NavItem {
        href: "/rbn",
        label: "RBN 信标网络",
        icon: IconKind::RadioTower,
      },
      NavItem {
        href: "/solar",
        label: "太阳活动",
        icon: IconKind::SunMedium,
      },
    ],
  },
  NavGroup {
    label: "地图 · 卫星",
    icon: IconKind::Map,
    items: &[
      NavItem {
        href: "/grid-map",
        label: "网格地图",
        icon: IconKind::Map,
      },
      NavItem {
        href: "/grayline",
        label: "灰线地图",
        icon: IconKind::Sun,
      },
      NavItem {
        href: "/dxcc-map",
        label: "DXCC 世界地图",
        icon: IconKind::Globe,
      },
      NavItem {
        href: "/portable-map",
        label: "SOTA / POTA 地图",
        icon: IconKind::Mountain,
      },
      NavItem {
        href: "/satellites",
        label: "业余卫星",
        icon: IconKind::Orbit,
      },
    ],
  },
  NavGroup {
    label: "训练 · 辅助",
    icon: IconKind::Headphones,
    items: &[
      NavItem {
        href: "/callsign-copy",
        label: "呼号抄收",
        icon: IconKind::Headphones,
      },
      NavItem {
        href: "/countdown",
        label: "倒计时",
        icon: IconKind::Timer,
      },
      NavItem {
        href: "/notifications",
        label: "通知中心",
        icon: IconKind::Siren,
      },
      NavItem {
        href: "/print",
        label: "打印版",
        icon: IconKind::FileText,
      },
    ],
  },
];
