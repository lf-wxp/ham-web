//! 现代风规范页的数据表与纯函数。
//!
//! 项目维度（组数 / 条数 / 交互）与像素风 `spec_data.rs` 对齐，取值全部换成
//! `crates/app/style/modern/tokens.css` 里真实存在的 `--md-*` 变量与 `MIcon` 形状名。
//! 表里的中英标识都是代码标识符，不是界面文案；界面句子一律走 `t()`。

use crate::kit_modern::MIcon;

use super::super::spec_data::Dimension;

/// 一条设计原则：标题 / 说明走词条，取值与维度是代码数据。
pub struct MdPrinciple {
  pub icon: MIcon,
  pub title: &'static str,
  pub desc: &'static str,
  pub value: &'static str,
  pub group: Dimension,
  pub spec: &'static str,
}

/// 8 条原则，维度分布与像素风一致：几何 4 / 排版 1 / 动效 1 / 配色 1 / 无障碍 1。
pub const MD_PRINCIPLES: &[MdPrinciple] = &[
  MdPrinciple {
    icon: MIcon::Grid,
    title: "design.md-principle-radius",
    desc: "design.md-principle-radius-desc",
    value: "var(--md-r-xs) … var(--md-r-pill)",
    group: Dimension::Geometry,
    spec: "design.spec-radius",
  },
  MdPrinciple {
    icon: MIcon::Card,
    title: "design.md-principle-page",
    desc: "design.md-principle-page-desc",
    value: "var(--md-page-max)",
    group: Dimension::Geometry,
    spec: "design.spec-page-max",
  },
  MdPrinciple {
    icon: MIcon::Sliders,
    title: "design.md-principle-control",
    desc: "design.md-principle-control-desc",
    value: "var(--md-control-h-sm) / var(--md-control-h)",
    group: Dimension::Geometry,
    spec: "design.spec-control",
  },
  MdPrinciple {
    icon: MIcon::Shield,
    title: "design.md-principle-shadow",
    desc: "design.md-principle-shadow-desc",
    value: "var(--md-shadow-1) … var(--md-shadow-3)",
    group: Dimension::Geometry,
    spec: "design.spec-shadow",
  },
  MdPrinciple {
    icon: MIcon::Doc,
    title: "design.md-principle-type",
    desc: "design.md-principle-type-desc",
    value: "clamp(var(--md-text-*))",
    group: Dimension::Type,
    spec: "design.spec-scale",
  },
  MdPrinciple {
    icon: MIcon::Zap,
    title: "design.md-principle-motion",
    desc: "design.md-principle-motion-desc",
    value: "var(--md-dur-fast) / var(--md-dur-slow)",
    group: Dimension::Motion,
    spec: "design.spec-duration",
  },
  MdPrinciple {
    icon: MIcon::CircleDot,
    title: "design.md-principle-color",
    desc: "design.md-principle-color-desc",
    value: "var(--md-danger) / var(--md-warn-ink)",
    group: Dimension::Color,
    spec: "design.spec-scheme",
  },
  MdPrinciple {
    icon: MIcon::Check,
    title: "design.md-principle-a11y",
    desc: "design.md-principle-a11y-desc",
    value: "var(--md-ring) · aria-invalid",
    group: Dimension::A11y,
    spec: "design.spec-border",
  },
];

/// 规格速查 10 行，标签复用像素风那批中性词条，取值换成现代风变量。
///
/// 行与 `MD_PRINCIPLES` 的 `spec` 字段一一对应：点原则卡高亮的那一行，必须真的写着这张卡
/// 讲的那类取值（控件高度 / 阴影此前借用别行，点过去看到的是无关内容）。
pub const MD_SPEC_ROWS: &[(&str, &str)] = &[
  (
    "design.spec-page-max",
    "var(--md-page-max) = var(--page-max)",
  ),
  (
    "design.spec-breakpoints",
    "480 / 640 / 720 / 900 / 1024 (+768 / 360)",
  ),
  ("design.spec-radius", "var(--md-r-xs) … var(--md-r-pill)"),
  (
    "design.spec-control",
    "var(--md-control-h-sm) / var(--md-control-h)",
  ),
  (
    "design.spec-shadow",
    "var(--md-shadow-1) … var(--md-shadow-3)",
  ),
  ("design.spec-border", "1px solid var(--md-border)"),
  (
    "design.spec-scale",
    "var(--md-text-3xs) … var(--md-text-3xl)",
  ),
  (
    "design.spec-duration",
    "var(--md-dur-fast) / var(--md-dur) / var(--md-dur-slow)",
  ),
  (
    "design.spec-z",
    "var(--md-z-dock) / var(--md-z-bar) / var(--md-z-modal)",
  ),
  ("design.spec-scheme", "light / dark"),
];

/// 色板分组。
pub struct MdTokenGroup {
  pub key: &'static str,
  pub tokens: &'static [&'static str],
}

/// 6 组 / 45 个 token，切明暗时现场读计算样式。
pub const MD_TOKEN_GROUPS: &[MdTokenGroup] = &[
  MdTokenGroup {
    key: "design.md-color-group-surface",
    tokens: &[
      "--md-bg",
      "--md-surface",
      "--md-surface-2",
      "--md-surface-3",
      "--md-border",
      "--md-border-strong",
      "--md-rule",
      "--md-contrast-bg",
    ],
  },
  MdTokenGroup {
    key: "design.md-color-group-text",
    tokens: &["--md-fg", "--md-fg-muted", "--md-ink", "--md-contrast-fg"],
  },
  MdTokenGroup {
    key: "design.md-color-group-brand",
    tokens: &[
      "--md-primary",
      "--md-primary-soft",
      "--md-accent",
      "--md-accent-soft",
      "--md-accent-ink",
      "--md-accent-fg",
      "--md-orange",
      "--md-orange-soft",
    ],
  },
  MdTokenGroup {
    key: "design.md-color-group-status",
    tokens: &[
      "--md-danger",
      "--md-danger-ink",
      "--md-danger-fill",
      "--md-danger-fill-hover",
      "--md-danger-fill-fg",
      "--md-danger-face",
      "--md-danger-face-fg",
      "--md-warn-ink",
      "--md-warn-fill",
      "--md-warn-fill-fg",
    ],
  },
  MdTokenGroup {
    key: "design.md-color-group-hue",
    tokens: &[
      "--md-blue",
      "--md-blue-soft",
      "--md-blue-ink",
      "--md-blue-fg",
      "--md-green",
      "--md-green-soft",
      "--md-green-ink",
      "--md-green-fg",
      "--md-gold",
      "--md-gold-ink",
      "--md-violet",
      "--md-violet-ink",
    ],
  },
  MdTokenGroup {
    key: "design.md-color-group-glow",
    tokens: &["--md-accent-line", "--md-accent-wash", "--md-glow-1"],
  },
];

/// 展开成 45 个变量名，供运行时计算样式一次性快照。
#[must_use]
pub fn all_md_tokens() -> Vec<&'static str> {
  MD_TOKEN_GROUPS
    .iter()
    .flat_map(|group| group.tokens.iter().copied())
    .collect()
}

/// 预览字族 3 档：现代风自己的字体 token，不是像素风的「易读字体」偏好开关。
pub const MD_FONT_FAMILIES: &[(&str, &str)] = &[
  ("--md-font", "body"),
  ("--md-font-display", "display"),
  ("--md-font-mono", "mono"),
];

/// 字号阶梯 7 阶，带标称行高（与像素风 TYPE_STEPS 的条目数一致）。
pub struct MdTypeStep {
  pub name: &'static str,
  pub var: &'static str,
  pub line: &'static str,
}

pub const MD_TYPE_STEPS: &[MdTypeStep] = &[
  MdTypeStep {
    name: "display-1",
    var: "--md-display-1",
    line: "1.05",
  },
  MdTypeStep {
    name: "display-2",
    var: "--md-display-2",
    line: "1.1",
  },
  MdTypeStep {
    name: "display-3",
    var: "--md-display-3",
    line: "1.15",
  },
  MdTypeStep {
    name: "display-4",
    var: "--md-display-4",
    line: "1.2",
  },
  MdTypeStep {
    name: "text-lede",
    var: "--md-text-lede",
    line: "1.5",
  },
  MdTypeStep {
    name: "text-prose",
    var: "--md-text-prose",
    line: "1.65",
  },
  MdTypeStep {
    name: "text-base",
    var: "--md-text-base",
    line: "1.55",
  },
];

/// 字体族 4 档（3 个变量 + 1 个已登记的等宽类）。
pub const MD_FONTS: &[&str] = &[
  "--md-font",
  "--md-font-display",
  "--md-font-mono",
  "md-mono",
];

/// 排版类 5 个，全部是已登记的 `md-*`。
pub const MD_TYPE_CLASSES: &[&str] = &["md-h2", "md-lede", "md-mono", "md-muted", "md-sm"];

/// 动效 token 4 档。
pub const MD_MOTION_TOKENS: &[(&str, &str)] = &[
  ("--md-dur-fast", "var(--md-dur-fast)"),
  ("--md-dur", "var(--md-dur)"),
  ("--md-dur-slow", "var(--md-dur-slow)"),
  ("--md-ease", "var(--md-ease-out)"),
];

/// 6 个动效演示：每个都引用现代风 CSS 里真实存在的 keyframes，名称与效果一一对应。
///
/// 时长与缓动全部走 `--md-dur*` / `--md-ease`，不写死毫秒；`animation` 值同时存进
/// `data-animation`，重播时由 DOM 摘掉再写回来触发新一轮播放。
#[derive(Clone, Copy)]
pub struct MdMotionDemo {
  pub slug: &'static str,
  pub label: &'static str,
  pub animation: &'static str,
}

pub const MD_MOTION_DEMOS: &[MdMotionDemo] = &[
  MdMotionDemo {
    slug: "ping",
    label: "design.md-demo-ping",
    animation: "md-signal-ring var(--md-dur-slow) var(--md-ease) both",
  },
  MdMotionDemo {
    slug: "sonar",
    label: "design.md-demo-sonar",
    animation: "md-sonar var(--md-dur-slow) var(--md-ease) both",
  },
  MdMotionDemo {
    slug: "wave",
    label: "design.md-demo-wave",
    animation: "md-current var(--md-dur-slow) linear infinite",
  },
  MdMotionDemo {
    slug: "ring-wave",
    label: "design.md-demo-ring",
    animation: "md-ring-out var(--md-dur-slow) var(--md-ease) both",
  },
  MdMotionDemo {
    slug: "spotlight",
    label: "design.md-demo-spotlight",
    animation: "md-fade-in var(--md-dur) var(--md-ease) both",
  },
  MdMotionDemo {
    slug: "skeleton",
    label: "design.md-demo-skeleton",
    animation: "md-skel-shimmer var(--md-dur-slow) linear infinite",
  },
];

/// 现代风指示灯 4 档（对应像素风 LedTone 的 4 种）。
pub const MD_LED_TONES: &[&str] = &["rx", "tx", "warn", "info"];

/// 电台读数区的 16 个展示项，对应像素风 radio_gallery 的 16 项。
pub const MD_RADIO_ITEMS: &[(&str, MIcon)] = &[
  ("md-readout", MIcon::Gauge),
  ("md-live-level", MIcon::Signal),
  ("md-live-metric", MIcon::Chart),
  ("md-live-band", MIcon::Antenna),
  ("md-dial", MIcon::CircleDot),
  ("md-dial-arc", MIcon::Refresh),
  ("md-dial-tick", MIcon::List),
  ("md-dial-needle", MIcon::ArrowRight),
  ("md-led", MIcon::Sun),
  ("md-badge-dot", MIcon::CircleDot),
  ("md-bar-track", MIcon::Sliders),
  ("md-bar-fill", MIcon::Battery),
  ("md-spectrum", MIcon::Network),
  ("md-wave", MIcon::Cloud),
  ("md-ring-wave", MIcon::Satellite),
  ("md-stat-value", MIcon::Trophy),
];

/// 现代风徽记 10 个（对应像素风 SPRITES 的 10 个精灵）。
pub const MD_EMBLEMS: &[(&str, MIcon)] = &[
  ("hero", MIcon::Antenna),
  ("boss", MIcon::Shield),
  ("mon_static", MIcon::Monitor),
  ("mon_swr", MIcon::Gauge),
  ("mon_noise", MIcon::Volume),
  ("badge_star", MIcon::Star),
  ("badge_trophy", MIcon::Trophy),
  ("node_flag", MIcon::Bookmark),
  ("node_heart", MIcon::Zap),
  ("node_coin", MIcon::Card),
];

/// 现代风图标全集（按形状命名，统一 24×24 视框）。
pub const MD_ICONS: &[(MIcon, &str)] = &[
  (MIcon::Antenna, "Antenna"),
  (MIcon::ArrowRight, "ArrowRight"),
  (MIcon::Chart, "Chart"),
  (MIcon::Clock, "Clock"),
  (MIcon::Code, "Code"),
  (MIcon::Gauge, "Gauge"),
  (MIcon::Loader, "Loader"),
  (MIcon::Map, "Map"),
  (MIcon::Refresh, "Refresh"),
  (MIcon::Satellite, "Satellite"),
  (MIcon::Signal, "Signal"),
  (MIcon::Star, "Star"),
  (MIcon::Trophy, "Trophy"),
  (MIcon::Check, "Check"),
  (MIcon::ChevronDown, "ChevronDown"),
  (MIcon::Play, "Play"),
  (MIcon::Close, "Close"),
  (MIcon::Book, "Book"),
  (MIcon::Calculator, "Calculator"),
  (MIcon::Calendar, "Calendar"),
  (MIcon::Download, "Download"),
  (MIcon::ExternalLink, "ExternalLink"),
  (MIcon::Minus, "Minus"),
  (MIcon::Plus, "Plus"),
  (MIcon::Search, "Search"),
  (MIcon::Sliders, "Sliders"),
  (MIcon::Settings, "Settings"),
  (MIcon::Flame, "Flame"),
  (MIcon::Home, "Home"),
  (MIcon::Menu, "Menu"),
  (MIcon::Tools, "Tools"),
  (MIcon::Volume, "Volume"),
  (MIcon::Alert, "Alert"),
  (MIcon::Bookmark, "Bookmark"),
  (MIcon::Camera, "Camera"),
  (MIcon::Moon, "Moon"),
  (MIcon::Sun, "Sun"),
  (MIcon::CircleDot, "CircleDot"),
  (MIcon::Card, "Card"),
  (MIcon::User, "User"),
  (MIcon::Grid, "Grid"),
  (MIcon::Message, "Message"),
  (MIcon::Shield, "Shield"),
  (MIcon::Zap, "Zap"),
  (MIcon::Cpu, "Cpu"),
  (MIcon::Battery, "Battery"),
  (MIcon::Monitor, "Monitor"),
  (MIcon::Send, "Send"),
  (MIcon::Mountain, "Mountain"),
  (MIcon::Wrench, "Wrench"),
  (MIcon::Mail, "Mail"),
  (MIcon::Plane, "Plane"),
  (MIcon::Cloud, "Cloud"),
  (MIcon::Doc, "Doc"),
  (MIcon::ArrowsUpDown, "ArrowsUpDown"),
  (MIcon::Network, "Network"),
  (MIcon::Anchor, "Anchor"),
  (MIcon::List, "List"),
];
