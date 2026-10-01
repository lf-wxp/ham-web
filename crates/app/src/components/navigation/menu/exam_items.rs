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
    href: "/exam",
    label: "模拟考试",
    icon: IconKind::Timer,
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
    href: "/bookmarks",
    label: "收藏集",
    icon: IconKind::Bookmark,
  },
  NavItem {
    href: "/photo-processor",
    label: "照片处理",
    icon: IconKind::Camera,
  },
  NavItem {
    href: "/countdown",
    label: "倒计时",
    icon: IconKind::Timer,
  },
  NavItem {
    href: "/progress",
    label: "学习进度",
    icon: IconKind::TrendingUp,
  },
];
