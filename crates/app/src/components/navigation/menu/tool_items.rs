use super::types::NavItem;
use crate::icons::IconKind;

/// 工具菜单。
pub(crate) const TOOL_ITEMS: &[NavItem] = &[
  NavItem {
    href: "/dashboard",
    label: "实时仪表盘",
    icon: IconKind::LayoutGrid,
  },
  NavItem {
    href: "/tools",
    label: "小工具",
    icon: IconKind::Calculator,
  },
  NavItem {
    href: "/log",
    label: "通联日志",
    icon: IconKind::ClipboardList,
  },
  NavItem {
    href: "/contest-log",
    label: "竞赛录入",
    icon: IconKind::Trophy,
  },
  NavItem {
    href: "/grid-map",
    label: "网格地图",
    icon: IconKind::Map,
  },
  NavItem {
    href: "/dx-spots",
    label: "DX 实时热点",
    icon: IconKind::Radio,
  },
  NavItem {
    href: "/solar",
    label: "太阳活动",
    icon: IconKind::SunMedium,
  },
  NavItem {
    href: "/satellites",
    label: "业余卫星",
    icon: IconKind::Orbit,
  },
  NavItem {
    href: "/grayline",
    label: "灰线地图",
    icon: IconKind::Sun,
  },
  NavItem {
    href: "/stats",
    label: "通联统计",
    icon: IconKind::TrendingUp,
  },
];
