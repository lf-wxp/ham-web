//! `IconKind`（像素图标集）→ [`MIcon`]（现代线性图标集）的映射。
//!
//! 为什么需要：`Icon` 组件被 `ui/` 里的控件大量使用（日历、chevron、勾选、下载…）。
//! 现代风下如果继续画像素精灵，表单里就会混着点阵小图标 —— 这是「一眼看出是上一套 UI」
//! 的地方。
//!
//! **覆盖要求：`IconKind` 的每个变体都要有映射**（`tests::every_kind_is_mapped` 守着）。
//! 语义相近的复用一个形状（`SunMedium → Sun`、`History → Clock`），但**不留回退**：
//! 回退到像素精灵等于在现代风里留一个点阵图标。

use super::IconKind;
use crate::kit_modern::MIcon;

/// 返回 `(现代图标, 附加类名)`；附加类名用于把 `ChevronDown` 旋转成左/右/上。
pub(super) fn to_modern(kind: IconKind) -> Option<(MIcon, &'static str)> {
  Some(match kind {
    // ── 导航与操作 ──────────────────────────────
    IconKind::Settings => (MIcon::Settings, ""),
    IconKind::Search => (MIcon::Search, ""),
    IconKind::X | IconKind::XCircle | IconKind::ListX => (MIcon::Close, ""),
    IconKind::ChevronDown => (MIcon::ChevronDown, ""),
    IconKind::ChevronLeft => (MIcon::ChevronDown, "rotate-90"),
    IconKind::ChevronRight => (MIcon::ChevronDown, "-rotate-90"),
    IconKind::Menu => (MIcon::Menu, ""),
    IconKind::Home => (MIcon::Home, ""),
    IconKind::ArrowUpDown => (MIcon::ArrowsUpDown, ""),
    IconKind::Plus => (MIcon::Plus, ""),
    IconKind::Minus => (MIcon::Minus, ""),
    IconKind::Send => (MIcon::Send, ""),
    IconKind::ExternalLink => (MIcon::ExternalLink, ""),
    IconKind::Download => (MIcon::Download, ""),
    IconKind::RefreshCw => (MIcon::Refresh, ""),
    IconKind::SlidersHorizontal | IconKind::Filter => (MIcon::Sliders, ""),
    IconKind::Wrench | IconKind::Hammer => (MIcon::Wrench, ""),
    IconKind::LayoutGrid => (MIcon::Grid, ""),

    // ── 状态与反馈 ──────────────────────────────
    IconKind::Check | IconKind::CheckCircle | IconKind::CheckCircle2 | IconKind::BadgeCheck => {
      (MIcon::Check, "")
    }
    IconKind::AlertCircle => (MIcon::Alert, ""),
    IconKind::ShieldAlert | IconKind::ShieldX | IconKind::Siren => (MIcon::Shield, ""),
    // 旋转的转圈图标不复用 `Refresh`：那颗有箭头缺口，形状本身不对称，
    // 看起来就像「绕着一个偏掉的圆心转」。`Loader` 是一条以网格中心为圆心的弧。
    IconKind::Loader2 => (MIcon::Loader, ""),
    IconKind::Circle => (MIcon::CircleDot, ""),
    IconKind::History | IconKind::Timer | IconKind::Clock => (MIcon::Clock, ""),
    IconKind::Zap | IconKind::PlugZap => (MIcon::Zap, ""),

    // ── 主题与天气 ──────────────────────────────
    IconKind::Moon => (MIcon::Moon, ""),
    IconKind::Sun | IconKind::SunMedium => (MIcon::Sun, ""),
    IconKind::Cloud => (MIcon::Cloud, ""),

    // ── 内容与文档 ──────────────────────────────
    IconKind::BookOpen | IconKind::BookMarked | IconKind::GraduationCap => (MIcon::Book, ""),
    IconKind::Bookmark => (MIcon::Bookmark, ""),
    IconKind::FileText | IconKind::Box | IconKind::Building2 => (MIcon::Doc, ""),
    IconKind::ClipboardList => (MIcon::List, ""),
    IconKind::CaseUpper | IconKind::Type => (MIcon::Doc, ""),
    IconKind::Mail => (MIcon::Mail, ""),
    IconKind::MessagesSquare => (MIcon::Message, ""),
    IconKind::Binary | IconKind::Cpu => (MIcon::Cpu, ""),
    IconKind::Camera => (MIcon::Camera, ""),
    IconKind::Calendar => (MIcon::Calendar, ""),

    // ── 无线与传播 ──────────────────────────────
    IconKind::Satellite | IconKind::Orbit => (MIcon::Satellite, ""),
    IconKind::RadioTower | IconKind::Radio => (MIcon::Antenna, ""),
    IconKind::Signal | IconKind::Waves | IconKind::Activity | IconKind::TrendingUp => {
      (MIcon::Signal, "")
    }
    IconKind::Gauge | IconKind::Scale => (MIcon::Gauge, ""),
    IconKind::Calculator => (MIcon::Calculator, ""),
    IconKind::Monitor | IconKind::Tv => (MIcon::Monitor, ""),
    IconKind::BatteryCharging | IconKind::BatteryLow => (MIcon::Battery, ""),
    IconKind::Waypoints | IconKind::Workflow | IconKind::Network => (MIcon::Network, ""),
    IconKind::Anchor => (MIcon::Anchor, ""),
    IconKind::Plane => (MIcon::Plane, ""),
    IconKind::Mountain => (MIcon::Mountain, ""),
    IconKind::Map | IconKind::Compass | IconKind::Globe => (MIcon::Map, ""),

    // ── 成就与身份 ──────────────────────────────
    IconKind::Trophy | IconKind::Award => (MIcon::Trophy, ""),
    IconKind::Star | IconKind::Sparkles => (MIcon::Star, ""),
    IconKind::Flame => (MIcon::Flame, ""),
    IconKind::User | IconKind::Landmark => (MIcon::User, ""),
    IconKind::CreditCard => (MIcon::Card, ""),
    IconKind::Play => (MIcon::Play, ""),
    IconKind::Volume2 | IconKind::AudioLines | IconKind::Headphones => (MIcon::Volume, ""),
  })
}

#[cfg(test)]
mod coverage {
  use super::*;

  /// `IconKind::ALL` 与源码里的变体声明数量一致：新加变体但忘了同步 `ALL` 时，
  /// 「每个 kind 都有现代图标」那条断言会漏检，所以这里直接比对源码文本。
  #[test]
  fn all_matches_source() {
    const SRC: &str = include_str!("icon_kind.rs");
    let block = SRC
      .split_once("pub enum IconKind {")
      .expect("枚举声明")
      .1
      .split_once("\n}")
      .expect("枚举结束")
      .0;
    let declared = block
      .lines()
      .filter(|l| {
        let t = l.trim();
        t.strip_suffix(',').is_some_and(|name| {
          !name.is_empty()
            && name.chars().all(|c| c.is_ascii_alphanumeric())
            && name.starts_with(|c: char| c.is_ascii_uppercase())
        })
      })
      .count();
    assert_eq!(
      declared,
      IconKind::ALL.len(),
      "IconKind 声明了 {declared} 个变体，ALL 里只有 {} 个 —— 补全 ALL",
      IconKind::ALL.len()
    );
  }

  /// 覆盖性：`ALL` 里每个 kind 都能映射到现代图标（不留像素回退）。
  #[test]
  fn every_listed_kind_is_mapped() {
    for kind in IconKind::ALL {
      assert!(
        to_modern(*kind).is_some(),
        "IconKind::{kind:?} 缺现代图标映射（icons/modern_map.rs）"
      );
    }
  }
}
