use super::types::NavItem;
use crate::icons::IconKind;

/// 考试中心菜单。
pub(crate) const EXAM_ITEMS: &[NavItem] = &[
  NavItem {
    href: "/practice",
    label: "练习",
    icon: IconKind::ClipboardList,
  },
  NavItem {
    href: "/practice?multi=1",
    label: "多选专项",
    icon: IconKind::CheckCircle2,
  },
  NavItem {
    href: "/exam",
    label: "模拟考试",
    icon: IconKind::Timer,
  },
  NavItem {
    href: "/daily-challenge",
    label: "每日挑战",
    icon: IconKind::Flame,
  },
  NavItem {
    href: "/browse",
    label: "分类浏览",
    icon: IconKind::LayoutGrid,
  },
  NavItem {
    href: "/flashcards",
    label: "闪卡刷题",
    icon: IconKind::Zap,
  },
  NavItem {
    href: "/listen",
    label: "听题模式",
    icon: IconKind::Headphones,
  },
  NavItem {
    href: "/cards",
    label: "知识卡片",
    icon: IconKind::CreditCard,
  },
  NavItem {
    href: "/mistakes",
    label: "错题集",
    icon: IconKind::ListX,
  },
  NavItem {
    href: "/mistake-topics",
    label: "易错知识点",
    icon: IconKind::Flame,
  },
  NavItem {
    href: "/bookmarks",
    label: "收藏集",
    icon: IconKind::Bookmark,
  },
  NavItem {
    href: "/weekly",
    label: "学习周报",
    icon: IconKind::Activity,
  },
  NavItem {
    href: "/progress",
    label: "学习进度",
    icon: IconKind::TrendingUp,
  },
  NavItem {
    href: "/study-calendar",
    label: "备考日历",
    icon: IconKind::Timer,
  },
  NavItem {
    href: "/exam-review",
    label: "考后复盘",
    icon: IconKind::ClipboardList,
  },
];
