//! 由 `cargo make pixel-sprites`（`crates/tools/src/pixel/sprites.rs`）生成，请勿手改。
//!
//! 图标来源：pixelarticons（MIT License，Copyright (c) 2019 Gerrit Halfmann，
//! https://github.com/halfmage/pixelarticons）；其余图标与全部精灵为本项目手绘。

use crate::icons::IconKind;

/// 24×24 网格里的像素图标路径（`fill="currentColor"`）。
pub(in crate::icons) const fn icon_path(kind: IconKind) -> &'static str {
  match kind {
    IconKind::Settings => {
      r#"M4 20h3v-2h4v4h2v-4h4v2h-2v4H9v-4H7v2H2v-5h2v3Zm18 2h-5v-2h3v-3h2v5ZM6 11H2v2h4v4H4v-2H0V9h4V7h2v4Zm14-2h4v6h-4v2h-2v-4h4v-2h-4V7h2v2Zm-6 7h-4v-2h4v2Zm-4-2H8v-4h2v4Zm6 0h-2v-4h2v4Zm-2-4h-4V8h4v2ZM7 4H4v3H2V2h5v2Zm8 0h2V2h5v5h-2V4h-3v2h-4V2h-2v4H7V4h2V0h6v4Z"#
    }
    IconKind::Search => {
      r#"M22 22h-2v-2h2v2Zm-2-2h-2v-2h2v2Zm-6-2H6v-2h8v2Zm4 0h-2v-2h2v2ZM6 16H4v-2h2v2Zm10 0h-2v-2h2v2ZM4 14H2V6h2v8Zm14 0h-2V6h2v8ZM6 6H4V4h2v2Zm10 0h-2V4h2v2Zm-2-2H6V2h8v2Z"#
    }
    IconKind::X => {
      r#"M7 19H5V17H7V19ZM19 19H17V17H19V19ZM9 15V17H7V15H9ZM17 17H15V15H17V17ZM11 15H9V13H11V15ZM15 15H13V13H15V15ZM13 13H11V11H13V13ZM11 11H9V9H11V11ZM15 11H13V9H15V11ZM9 9H7V7H9V9ZM17 9H15V7H17V9ZM7 7H5V5H7V7ZM19 7H17V5H19V7Z"#
    }
    IconKind::ChevronDown => {
      r#"M13 16h-2v-2h2v2Zm-2-2H9v-2h2v2Zm4 0h-2v-2h2v2Zm-6-2H7v-2h2v2Zm8 0h-2v-2h2v2ZM7 10H5V8h2v2Zm12 0h-2V8h2v2Z"#
    }
    IconKind::Check => {
      r#"M10 18H8v-2h2v2Zm-2-2H6v-2h2v2Zm4-2v2h-2v-2h2Zm-6 0H4v-2h2v2Zm8 0h-2v-2h2v2Zm2-2h-2v-2h2v2Zm2-2h-2V8h2v2Zm2-2h-2V6h2v2Z"#
    }
    IconKind::CheckCircle => {
      r#"M4 0h16v2h-16zM2 2h4v2h-4zM18 2h4v2h-4zM0 4h4v2h-4zM20 4h4v4h-4zM0 6h2v10h-2zM16 6h2v2h-2zM14 8h4v2h-4zM22 8h2v8h-2zM4 10h2v2h-2zM12 10h4v2h-4zM6 12h2v2h-2zM10 12h4v2h-4zM8 14h6v2h-6zM0 16h4v2h-4zM10 16h2v2h-2zM20 16h4v2h-4zM2 18h4v2h-4zM18 18h4v2h-4zM4 20h16v2h-16z"#
    }
    IconKind::CheckCircle2 => {
      r#"M6 0h12v2h-12zM4 2h4v2h-4zM16 2h4v2h-4zM2 4h2v2h-2zM20 4h2v2h-2zM0 6h2v8h-2zM16 6h2v2h-2zM22 6h2v8h-2zM14 8h4v2h-4zM4 10h2v2h-2zM12 10h4v2h-4zM6 12h2v2h-2zM10 12h4v2h-4zM2 14h2v2h-2zM8 14h4v2h-4zM20 14h2v2h-2zM4 16h4v2h-4zM16 16h4v2h-4zM6 18h12v2h-12z"#
    }
    IconKind::ExternalLink => {
      r#"M11 5H5v2h6V5ZM5 7H3v12h2V7Zm12 12H5v2h12v-2Zm2-6h-2v6h2v-6Zm-8 0H9v2h2v-2Zm2-2h-2v2h2v-2Zm2-2h-2v2h2V9Zm2-2h-2v2h2V7Zm2-2h-2v2h2V5Zm2-2h-2v8h2V3ZM21 3h-8v2h8V3Z"#
    }
    IconKind::Loader2 => {
      r#"M13 22h-2v-6h2v6Zm-6-3H5v-2h2v2Zm12 0h-2v-2h2v2ZM9 17H7v-2h2v2Zm8 0h-2v-2h2v2Zm-9-4H2v-2h6v2Zm14 0h-6v-2h6v2ZM9 9H7V7h2v2Zm8 0h-2V7h2v2Zm-4-1h-2V2h2v6ZM7 7H5V5h2v2Zm12 0h-2V5h2v2Z"#
    }
    IconKind::XCircle => {
      r#"M6 0h12v2h-12zM4 2h4v2h-4zM16 2h4v2h-4zM2 4h2v2h-2zM6 4h2v2h-2zM16 4h2v2h-2zM20 4h2v2h-2zM0 6h2v8h-2zM8 6h2v2h-2zM14 6h2v2h-2zM22 6h2v8h-2zM10 8h4v4h-4zM8 12h2v2h-2zM14 12h2v2h-2zM2 14h2v2h-2zM6 14h2v2h-2zM16 14h2v2h-2zM20 14h2v2h-2zM4 16h4v2h-4zM16 16h4v2h-4zM6 18h12v2h-12z"#
    }
    IconKind::AlertCircle => {
      r#"M2 10h2v2H2zm0 4h2v-2H2zm20-4h-2v2h2zm0 4h-2v-2h2zM4 8h2v2H4zm0 8h2v-2H4zm16-8h-2v2h2zm0 8h-2v-2h2zM6 6h2v2H6zm0 12h2v-2H6zM18 6h-2v2h2zm0 12h-2v-2h2zM8 4h2v2H8zm0 16h2v-2H8zm8-16h-2v2h2zm0 16h-2v-2h2zM10 2h2v2h-2zm0 20h2v-2h-2zm4-20h-2v2h2zm0 20h-2v-2h2zm-3-5h2v-2h-2zm0-4h2V7h-2z"#
    }
    IconKind::RefreshCw => {
      r#"M16 4h2v6h-2zm-2-2h2v2h-2zm0 2h2v8h-2zM4 8H2v5h2zM4 6h16v2H4zm4 14H6v-6h2zm2 2H8v-2h2zm0-2H8v-8h2zm10-4h2v-5h-2zM20 18H4v-2h16z"#
    }
    IconKind::Camera => {
      r#"M4 5h4v2H4zm4-2h8v2H8zm8 2h4v2h-4zM2 7h2v12H2zm2 12h16v2H4zM20 7h2v12h-2zM10 8h4v2h-4zm0 6h4v2h-4zm-2-4h2v4H8zm6 0h2v4h-2z"#
    }
    IconKind::Home => {
      r#"M4 20h16v2H4zm16-10h2v10h-2zM2 10h2v10H2zm2-2h2v2H4zm2-2h2v2H6zm2-2h2v2H8zm2-2h4v2h-4zm4 2h2v2h-2zm2 2h2v2h-2zm2 2h2v2h-2zM8 14h2v6H8zm2-2h4v2h-4zm4 2h2v6h-2z"#
    }
    IconKind::Moon => {
      r#"M18 22H8v-2h10v2ZM8 20H6v-2h2v2Zm12 0h-2v-2h2v2ZM6 18H4v-2h2v2Zm16 0h-2v-4h-2v-2h2v-2h2v8ZM4 16H2V6h2v10Zm14 0h-6v-2h6v2Zm-6-2h-2v-2h2v2Zm-2-2H8V6h2v6ZM6 6H4V4h2v2Zm8-2h-2v2h-2V4H6V2h8v2Z"#
    }
    IconKind::Sun => {
      r#"M13 22h-2v-3h2v3Zm-6-3H5v-2h2v2Zm12 0h-2v-2h2v2Zm-4-2H9v-2h6v2Zm-6-2H7V9h2v6Zm8 0h-2V9h2v6ZM5 13H2v-2h3v2Zm17 0h-3v-2h3v2Zm-7-4H9V7h6v2ZM7 7H5V5h2v2Zm12 0h-2V5h2v2Zm-6-2h-2V2h2v3Z"#
    }
    IconKind::Circle => {
      r#"M6 2h12v2H6zm0 18h12v2H6zM2 6h2v12H2zm18 0h2v12h-2zm-2-2h2v2h-2zm0 14h2v2h-2zM4 4h2v2H4zm0 14h2v2H4z"#
    }
    IconKind::CreditCard => {
      r#"M4 4h16v2H4zm0 14h16v2H4zM2 6h2v12H2zm18 0h2v12h-2zM4 8h16v4H4zm2 6h6v2H6z"#
    }
    IconKind::User => {
      r#"M9 2h6v2H9zm0 8h6v2H9zm6-6h2v6h-2zM7 4h2v6H7zM4 18h2v4H4zm14 0h2v4h-2zM8 14h8v2H8zm-2 2h2v2H6zm10 0h2v2h-2z"#
    }
    IconKind::Download => {
      r#"M21 15v4h-2v-4zm-2 4v2H5v-2zM5 15v4H3v-4zm8-12v14h-2V3zM7 11v2h10v-2zm2 2v2h2v-2zm4 0v2h2v-2zM15 11v2h2v-2z"#
    }
    IconKind::Satellite => {
      r#"M8 0h8v2h-8zM10 2h4v4h-4zM0 6h6v4h-6zM8 6h8v4h-8zM18 6h6v4h-6zM0 10h24v4h-24zM0 14h6v4h-6zM8 14h8v4h-8zM18 14h6v4h-6z"#
    }
    IconKind::Menu => r#"M20 18H4v-2h16v2Zm0-5H4v-2h16v2Zm0-5H4V6h16v2Z"#,
    IconKind::ClipboardList => {
      r#"M20 12h2v8h-2zm-8-2h8v2h-8zm0 10h8v2h-8zm-2-8h2v8h-2zM6 2h8v2H6zm0 4h8v2H6zm0-2h2v2H6zm6 0h2v2h-2zm2 0h2v2h-2zM16 6h2v5h-2zM4 4h2v2H4zM2 6h2v12H2zm2 12h6v2H4zm2-8h4v2H6zm0 4h2v2H6zm11 2h5v2h-5zM16 16h2v6h-2z"#
    }
    IconKind::Timer => {
      r#"M16 22H8v-2h8v2Zm-8-2H6v-4h2v4Zm10 0h-2v-4h2v4Zm-8-4H8v-2h2v2Zm6 0h-2v-2h2v2Zm-6-6h4v4h-4v-4Zm0 0H8V8h2v2Zm6 0h-2V8h2v2ZM8 8H6V4h2v4Zm10 0h-2V4h2v4Zm-2-4H8V2h8v2Z"#
    }
    IconKind::BookOpen => {
      r#"M2 3h9v2H2zM0 19h11v2H0zM13 3h9v2h-9zm0 16h11v2H13zM11 5h2v18h-2zM0 5h2v14H0zm22 0h2v14h-2zm-7 2h5v2h-5zm0 4h5v2h-5zm0 4h2v2h-2z"#
    }
    IconKind::LayoutGrid => {
      r#"M4 2h16v2H4zM2 4h2v16H2zm2 7h16v2H4zm16-7h2v16h-2zM11 4h2v18h-2zM4 20h16v2H4z"#
    }
    IconKind::Radio => {
      r#"M11 9h2v2h-2zm0 4h2v2h-2zm-2-2h2v2H9zm4 0h2v2h-2zm6-2h-2v6h2zM5 9h2v6H5zm18-2h-2v10h2zM1 7h2v10H1zm16 0h-2v2h2zM7 7h2v2H7zm14-2h-2v2h2zM3 5h2v2H3zm14 10h-2v2h2zM7 15h2v2H7zm14 2h-2v2h2zM3 17h2v2H3z"#
    }
    IconKind::AudioLines => {
      r#"M3 7h2v5H3zm4 0h2v13H7zm4-3h2v16h-2zm4 0h2v13h-2zM5 5h2v2H5zm4 15h2v2H9zm4-18h2v2h-2zm4 15h2v2h-2zm2-5h2v5h-2zm2-2h2v2h-2zM1 12h2v2H1z"#
    }
    IconKind::Play => {
      r#"M15 11h-2V9h2zm0 4h-2v-2h2zm-2 2h-2v-2h2zm0-8h-2V7h2zm-2-2H9V5h2zM9 21H7V3h2zm6-8h2v-2h-2zm-6 4h2v2H9z"#
    }
    IconKind::CaseUpper => r#"M8 12H16V5H18V21H16V14H8V21H6V5H8V12ZM16 5H8V3H16V5Z"#,
    IconKind::BookMarked => {
      r#"M6 2h14v2H6zm0 18h14v2H6zM20 4h2v16h-2zM4 4h2v16H4zM2 7h6v2H2zm0 4h6v2H2zm0 4h6v2H2zM16 4h2v16h-2z"#
    }
    IconKind::RadioTower => {
      r#"M0 0h2v12h-2zM10 0h4v14h-4zM22 0h2v12h-2zM6 2h2v2h-2zM16 2h2v2h-2zM4 4h2v4h-2zM18 4h2v4h-2zM6 8h2v2h-2zM16 8h2v2h-2zM8 14h8v2h-8zM6 16h4v2h-4zM14 16h4v2h-4zM4 18h4v2h-4zM16 18h4v2h-4zM2 20h4v2h-4zM18 20h4v2h-4zM0 22h4v2h-4zM20 22h4v2h-4z"#
    }
    IconKind::Calculator => {
      r#"M5 2h14v2H5zm0 18h14v2H5zM3 4h2v16H3zm16 0h2v16h-2zM7 6h10v4H7zm0 6h2v2H7zm4 0h2v2h-2zm4 0h2v2h-2zm-8 4h2v2H7zm4 0h2v2h-2zm4 0h2v2h-2z"#
    }
    IconKind::Globe => {
      r#"M6 2h12v2H6zm0 18h12v2H6zM18 4h2v2h-2zM4 18h2v2H4zM4 4h2v2H4zm14 14h2v2h-2zM2 6h2v12H2zm18 0h2v12h-2zM8 4h2v4H8zm2 4h4v2h-4zm4 2h4v2h-4zm4-2h2v2h-2zM4 12h2v2H4zm6 4h2v4h-2zm-4-2h4v2H6zm8 2h2v4h-2zm2-2h4v2h-4z"#
    }
    IconKind::SlidersHorizontal => {
      r#"M17 18h5v2h-5v2h-2v-6h2v2Zm-4 2H2v-2h11v2Zm-4-5H7v-2H2v-2h5V9h2v6Zm13-2H11v-2h11v2Zm-7-9h7v2h-7v2h-2V2h2v2Zm-4 2H2V4h9v2Z"#
    }
    IconKind::Binary => {
      r#"M7 3h2v2H7zm8 10h2v2h-2zM5 5h2v4H5zm8 10h2v4h-2zM9 5h2v4H9zm8 10h2v4h-2zM7 9h2v2H7zm8 10h2v2h-2zM13 3h4v2h-4zM5 13h4v2H5zm10-8h2v4h-2zM7 15h2v4H7zm6-6h6v2h-6zM5 19h6v2H5z"#
    }
    IconKind::Gauge => {
      r#"M5 19H3v-2h2v2Zm16 0h-2v-2h2v2ZM3 17H1v-6h2v6Zm11 0h-4v-4h4v4Zm9 0h-2v-6h2v6Zm-7-4h-2v-2h2v2ZM5 11H3V9h2v2Zm13 0h-2V9h2v2ZM9 9H5V7h4v2Zm11 0h-2V7h2v2Zm-5-2H9V5h6v2Z"#
    }
    IconKind::Orbit => {
      r#"M8 0h8v2h-8zM4 2h4v2h-4zM16 2h4v2h-4zM2 4h2v2h-2zM8 4h8v2h-8zM20 4h2v2h-2zM0 6h2v12h-2zM6 6h4v2h-4zM14 6h4v2h-4zM22 6h2v12h-2zM4 8h4v2h-4zM16 8h4v2h-4zM4 10h2v4h-2zM10 10h4v4h-4zM18 10h2v4h-2zM4 14h4v2h-4zM16 14h4v2h-4zM6 16h4v2h-4zM14 16h4v2h-4zM2 18h2v2h-2zM8 18h8v2h-8zM20 18h2v2h-2zM4 20h4v2h-4zM16 20h4v2h-4zM8 22h8v2h-8z"#
    }
    IconKind::MessagesSquare => {
      r#"M20 2H4v2h16zm0 14H6v2h14zm2-12h-2v12h2zM4 4H2v18h2zm2 14H4v2h2zm0-6h4v2H6zm0-4h8v2H6z"#
    }
    IconKind::Signal => r#"M19 3h2v18h-2zm-4 4h2v14h-2zm-4 4h2v10h-2zm-4 4h2v6H7zm-4 4h2v2H3z"#,
    IconKind::Waves => {
      r#"M2 18h4v-2H2zm0-6h4v-2H2zm0-6h4V4H2zm4 14h4v-2H6zm0-6h4v-2H6zm0-6h4V6H6zm4 10h4v-2h-4zm0-6h4v-2h-4zm0-6h4V4h-4zm4 14h4v-2h-4zm0-6h4v-2h-4zm0-6h4V6h-4zm4 10h4v-2h-4zm0-6h4v-2h-4zm0-6h4V4h-4z"#
    }
    IconKind::ListX => {
      r#"M0 2h4v2h-4zM6 2h14v2h-14zM0 6h4v2h-4zM6 6h14v2h-14zM0 10h4v2h-4zM6 10h8v2h-8zM18 10h2v2h-2zM22 10h2v4h-2zM20 14h2v2h-2zM12 16h2v2h-2zM18 16h2v2h-2zM22 16h2v4h-2zM20 20h2v2h-2z"#
    }
    IconKind::SunMedium => {
      r#"M13 22h-2v-3h2v3Zm-6-3H5v-2h2v2Zm12 0h-2v-2h2v2Zm-4-2H9v-2h6v2Zm-6-2H7V9h2v6Zm8 0h-2V9h2v6ZM5 13H2v-2h3v2Zm17 0h-3v-2h3v2Zm-7-4H9V7h6v2ZM7 7H5V5h2v2Zm12 0h-2V5h2v2Zm-6-2h-2V2h2v3Z"#
    }
    IconKind::ShieldAlert => {
      r#"M4 2h16v2H4zM2 4h2v10H2zm18 0h2v10h-2zM4 14h2v2H4zm2 2h2v2H6zm4 4h4v2h-4zm10-6h-2v2h2zm-2 2h-2v2h2zm-2 2h-2v2h2zm-6 0H8v2h2z"#
    }
    IconKind::BadgeCheck => {
      r#"M4 2h16v2H4zM2 4h2v10H2zm18 0h2v10h-2zM4 14h2v2H4zm2 2h2v2H6zm4 4h4v2h-4zm10-6h-2v2h2zm-2 2h-2v2h2zm-2 2h-2v2h2zm-6 0H8v2h2z"#
    }
    IconKind::Volume2 => {
      r#"M13 22h-2v-2H9v-2h2V6H9V4h2V2h2v20Zm-4-4H7v-2h2v2Zm10 0h-4v-2h4v2ZM7 10H5v4h2v2H3V8h4v2Zm14 6h-2V8h2v8Zm-4-2h-2v-4h2v4ZM9 8H7V6h2v2Zm10 0h-4V6h4v2Z"#
    }
    IconKind::Bookmark => {
      r#"M6 2h12v2H6zM4 4h2v18H4zm14 0h2v18h-2zm-2 16h2v2h-2zm-2-2h2v2h-2zm-8 2h2v2H6zm2-2h2v2H8zm2-2h4v2h-4z"#
    }
    IconKind::Zap => {
      r#"M4 13h8v6h2v2h-2v2h-2v-8H2v-4h2v2Zm12 6h-2v-2h2v2Zm2-2h-2v-2h2v2Zm2-2h-2v-2h2v2Zm-6-6h8v4h-2v-2h-8V5h-2V3h2V1h2v8Zm-8 2H4V9h2v2Zm2-2H6V7h2v2Zm2-2H8V5h2v2Z"#
    }
    IconKind::Trophy => {
      r#"M16 17H13V19H15V21H9V19H11V17H8V15H16V17ZM18 5H22V11H20V7H18V11H20V13H18V15H16V5H8V15H6V13H4V11H6V7H4V11H2V5H6V3H18V5Z"#
    }
    IconKind::Cpu => {
      r#"M5 3h14v2H5zm0 16h14v2H5zM3 5h2v14H3zm16 0h2v14h-2zM9 7h6v2H9zm0 8h6v2H9zM7 9h2v6H7zm8 0h2v6h-2zm-4-8h2v2h-2zm0 20h2v2h-2zM1 11h2v2H1zm20 0h2v2h-2zm0-4h2v2h-2zm0 8h2v2h-2zM1 15h2v2H1zm0-8h2v2H1zm6-6h2v2H7zm8 0h2v2h-2zm0 20h2v2h-2zm-8 0h2v2H7z"#
    }
    IconKind::PlugZap => {
      r#"M16 18h-3v4h-2v-4H8v-2h8v2Zm-8-2H6v-2h2v2Zm10 0h-2v-2h2v2Zm-8-9h4V2h2v5h5v2h-1v5h-2V9H6v5H4V9H3V7h5V2h2v5Z"#
    }
    IconKind::Activity => {
      r#"M22 22H4v-2h18v2ZM4 20H2V2h2v18Zm4-6H6v-2h2v2Zm8 0h-2v-2h2v2Zm-6-2H8v-2h2v2Zm4 0h-2v-2h2v2Zm4 0h-2v-2h2v2Zm-6-2h-2V8h2v2Zm8 0h-2V8h2v2Zm2-2h-2V6h2v2Z"#
    }
    IconKind::BatteryCharging => {
      r#"M4 5h14v2H4zm0 12h14v2H4zM2 7h2v10H2zm16-2h2v14h-2zm2 4h2v6h-2zM6 9h2v6H6zm4 0h2v6h-2zm4 0h2v6h-2z"#
    }
    IconKind::Award => {
      r#"M3 3h2v12H3zm16 0h2v12h-2zm-8 0h2v2h-2zM9 5h2v2H9zM5 5h2v2H5zM3 3h2v2H3zm4 4h2v2H7zm6-2h2v2h-2zm2 2h2v2h-2zm2-2h2v2h-2zM5 15h14v2H5zm-2 4h18v2H3z"#
    }
    IconKind::Map => {
      r#"M4 20h2v2H2V6h2v14Zm12 0h2v2h-4v-2h-2v-2h2V8h-2V6h4v14Zm-8 0H6v-2h2v2Zm12 0h-2v-2h2v2ZM10 4h2v2h-2v10h2v2H8V4H6V2h4v2Zm12 14h-2V4h-2V2h4v16ZM6 6H4V4h2v2Zm12 0h-2V4h2v2Z"#
    }
    IconKind::Monitor => {
      r#"M4 2h16v2H4zm0 14h16v2H4zM2 4h2v12H2zm18 0h2v12h-2zm-9 14h2v2h-2zm-3 2h8v2H8z"#
    }
    IconKind::Siren => {
      r#"M6 11h2v5H6zm2-2h2v2H8zm2-2h4v2h-4zm4 2h2v2h-2zm2 2h2v5h-2zM6 16h12v2H6zm-2 4h16v2H4zm0-2h2v2H4zm14 0h2v2h-2zm-7-6h2v4h-2zm9-1h3v2h-3zM1 11h3v2H1zm5-6h2v2H6zm10 0h2v2h-2zm2-2h2v2h-2zM4 3h2v2H4zm7-1h2v3h-2z"#
    }
    IconKind::GraduationCap => {
      r#"M10 0h4v2h-4zM6 2h12v2h-12zM2 4h20v2h-20zM0 6h24v2h-24zM2 8h20v2h-20zM6 10h12v2h-12zM4 12h2v10h-2zM8 12h8v2h-8zM18 12h2v6h-2zM10 14h4v2h-4zM16 18h4v2h-4z"#
    }
    IconKind::Landmark => {
      r#"M3 4h2v17H3zm4 4h2v13H7zm4-2h2v15h-2zm4 0h2v5h-2zm2 5h2v5h-2zm2 5h2v5h-2z"#
    }
    IconKind::Compass => {
      r#"M18 22H6V20H18V22ZM6 20H4V18H6V20ZM20 20H18V18H20V20ZM10 16H12V18H10V19H8V10H10V16ZM4 18H2V6H4V18ZM22 18H20V6H22V18ZM14 16H12V14H14V16ZM16 14H14V8H12V6H14V5H16V14ZM13 13H11V11H13V13ZM12 10H10V8H12V10ZM6 6H4V4H6V6ZM20 6H18V4H20V6ZM18 4H6V2H18V4Z"#
    }
    IconKind::Sparkles => {
      r#"M11 1h2v4h-2zm0 22h2v-4h-2zM9 5h2v4H9zm0 14h2v-4H9zm4-14h2v4h-2zm0 14h2v-4h-2zM5 9h4v2H5zm14 0h-4v2h4zM1 11h4v2H1zm22 0h-4v2h4zM5 13h4v2H5zm14 0h-4v2h4zm0-12h2v6h-2zM17 3h6v2h-6zM3 17h2v2H3zm-2 2h2v2H1zm2 2h2v2H3zm2-2h2v2H5z"#
    }
    IconKind::Wrench => {
      r#"M9 22H7v-2h2v2Zm12-6h2v6h-6v-6h2v-6h2v6Zm-2 2v2h2v-2h-2ZM7 20H5v-8h2v8Zm4 0H9v-8h2v8Zm-6-8H3v-2h2v2Zm8 0h-2v-2h2v2ZM3 10H1V4h2v6Zm12 0h-2V4h2v6Zm4 0h-2V4h2v6Zm4 0h-2V4h2v6ZM7 6h2V2h4v2h-2v4H5V4H3V2h4v4Zm14-2h-2V2h2v2Z"#
    }
    IconKind::Send => {
      r#"M4 19h4v2H2v-8h2v6Zm8 0H8v-2h4v2Zm4-2h-4v-2h4v2Zm4-2h-4v-2h4v2Zm-10-2H4v-2h6v2Zm12 0h-2v-2h2v2ZM8 5H4v6H2V3h6v2Zm12 6h-4V9h4v2Zm-4-2h-4V7h4v2Zm-4-2H8V5h4v2Z"#
    }
    IconKind::History => {
      r#"M18 20h-6v-2h6v2Zm2-2h-2v-8h2v8Zm-10-4H8v-2H6v-2H4V8h2V6h2V4h2v4h8v2h-8v4Z"#
    }
    IconKind::TrendingUp => {
      r#"M4 18H2v-2h2v2Zm2-2H4v-2h2v2Zm8 0h-2v-2h2v2Zm-6-2H6v-2h2v2Zm4 0h-2v-2h2v2Zm4 0h-2v-2h2v2Zm6-8v8h-2v-4h-2V8h-4V6h8Zm-12 6H8v-2h2v2Zm8 0h-2v-2h2v2Z"#
    }
    IconKind::Mountain => {
      r#"M12 4h4v2h-4zM10 6h8v2h-8zM8 8h12v2h-12zM6 10h6v2h-6zM16 10h6v2h-6zM4 12h6v2h-6zM18 12h6v2h-6zM2 14h6v2h-6zM20 14h4v2h-4zM0 16h6v2h-6zM0 18h2v2h-2z"#
    }
    IconKind::Hammer => {
      r#"M2 11h20v2H2zm0 2h2v8H2zm2 8h16v2H4zm16-8h2v8h-2zM9 15h6v2H9zM4 8h2v3H4zm2-2h6v2H6zm6 2h2v3h-2zM8 4h2v2H8zm10 0h2v7h-2zm-8-2h8v2h-8z"#
    }
    IconKind::Mail => {
      r#"M6 8h2v2H6zm2 2h2v2H8zm10-2h-2v2h2zm-2 2h-2v2h2zm-6 2h4v2h-4zM2 6h2v12H2zm18 0h2v12h-2zM4 4h16v2H4zm0 14h16v2H4z"#
    }
    IconKind::ShieldX => {
      r#"M4 2h16v2H4zM2 4h2v10H2zm18 0h2v10h-2zM4 14h2v2H4zm2 2h2v2H6zm4 4h4v2h-4zm10-6h-2v2h2zm-2 2h-2v2h2zm-2 2h-2v2h2zm-6 0H8v2h2z"#
    }
    IconKind::BatteryLow => {
      r#"M4 5h14v2H4zm0 12h14v2H4zM2 7h2v10H2zm16-2h2v14h-2zm2 4h2v6h-2zM6 9h2v6H6z"#
    }
    IconKind::Plane => r#"M10 0h4v4h-4zM8 4h8v4h-8zM0 8h24v4h-24zM8 12h8v6h-8zM6 18h12v4h-12z"#,
    IconKind::Scale => {
      r#"M13 9h2v2h-2zm2-2h2v2h-2zm2-2h2v2h-2zm2-2h2v8h-2zM13 3h8v2h-8zm-2 12H9v-2h2zm-2 2H7v-2h2zm-2 2H5v-2h2zm-2 2H3v-8h2zM11 21H3v-2h8z"#
    }
    IconKind::Waypoints => {
      r#"M4 14h4v2H4zm0 6h4v2H4zm-2-4h2v4H2zm6 0h2v4H8zm8-14h4v2h-4zm0 6h4v2h-4zm-2-4h2v4h-2zm6 0h2v4h-2zm-8 13h5v2h-5zm5-5h2v5h-2zM5 2h2v10H5z"#
    }
    IconKind::Cloud => {
      r#"M22 10h-4v2h4v-2Zm2 2h-2v6h2v-6Zm-2 6H2v2h20v-2ZM2 12H0v6h2v-6Zm2-2H2v2h2v-2Zm4-2H4v2h4V8Zm8-4h-6v2h6V4Zm-6 2H8v2h2V6Zm0 4H8v2h2v-2Zm8-4h-2v2h2V6ZM20 8h-2v4h2V8Zm-2 4h-2v2h2v-2Z"#
    }
    IconKind::Type => r#"M0 0h24v4h-24zM8 4h8v16h-8z"#,
    IconKind::Anchor => {
      r#"M10 2h4v2h-4zm0 6h4v2h-4zM8 4h2v4H8zm6 0h2v4h-2zM11 9h2v12h-2zM5 20h14v2H5zm-2-8h2v8H3zm16 0h2v8h-2zM5 12h2v2H5zm12 0h2v2h-2z"#
    }
    IconKind::Workflow => {
      r#"M0 0h8v2h-8zM0 2h2v4h-2zM6 2h2v4h-2zM0 6h8v2h-8zM4 8h4v2h-4zM4 10h20v2h-20zM4 12h2v6h-2zM22 12h2v2h-2zM16 14h8v2h-8zM16 16h2v2h-2zM4 18h14v2h-14z"#
    }
    IconKind::Headphones => {
      r#"M14 13h7v2h-7zm2 6h3v2h-3zM14 13h2v8h-2zm5-6h2v12h-2zM3 13h7v2H3zm2 6h3v2H5zM3 7h2v12H3zm5 6h2v8H8zM7 3h10v2H7zM5 5h2v2H5zm12 0h2v2h-2z"#
    }
    IconKind::Flame => {
      r#"M9 2h2v4H9zM7 6h2v2H7zM5 8h2v2H5zm8 2h2v2h-2zm2-2h2v2h-2zm2 2h2v2h-2zm2 2h2v6h-2zM3 10h2v8H3zm8-4h2v4h-2zm6 12h2v2h-2zM7 20h10v2H7zm-2-2h2v2H5zm4-2h6v4H9zM11 14h2v3h-2z"#
    }
    IconKind::Tv => {
      r#"M4 3h16v2H4zM2 5h2v10H2zm2 10h16v2H4zM20 5h2v10h-2zM6 19h12v2H6zm3-2h2v2H9zm4 0h2v2h-2z"#
    }
    IconKind::Filter => {
      r#"M11 20H13V22H9V12H11V20ZM15 20H13V12H15V20ZM9 12H7V10H9V12ZM17 12H15V10H17V12ZM7 10H5V8H7V10ZM19 10H17V8H19V10ZM21 8H19V4H5V8H3V2H21V8Z"#
    }
    IconKind::Box => {
      r#"M14 4h4v2h-4zm-4-2h4v2h-4zM6 8h4v2H6zm0 10h4v2H6zm4-8h4v2h-4zm0 10h4v2h-4zm4-12h4v2h-4zm0 10h4v2h-4zM6 4h4v2H6zM2 6h4v2H2zm0 10h4v2H2zM18 6h4v2h-4zm0 10h4v2h-4zM2 6h2v12H2zm18 0h2v12h-2zm-8 6h2v8h-2z"#
    }
    IconKind::Star => {
      r#"M5 20H8V22H3V16H5V20ZM21 22H16V20H19V16H21V22ZM10 20H8V18H10V20ZM16 20H14V18H16V20ZM14 18H10V16H14V18ZM7 16H5V13H7V16ZM19 16H17V13H19V16ZM5 13H3V11H5V13ZM21 13H19V11H21V13ZM9 9H3V11H1V7H9V9ZM23 11H21V9H15V7H23V11ZM11 7H9V3H11V7ZM15 7H13V3H15V7ZM13 3H11V1H13V3Z"#
    }
    IconKind::Building2 => {
      r#"M5 2h14v2H5zm0 18h14v2H5zM3 4h2v16H3zm16 0h2v16h-2zM7 6h2v2H7zm4 0h2v2h-2zm4 0h2v2h-2zm-8 4h2v2H7zm4 0h2v2h-2zm4 0h2v2h-2zm-8 4h2v2H7zm4 0h2v2h-2zm-1 4h4v2h-4zm5-4h2v2h-2z"#
    }
    IconKind::FileText => {
      r#"M6 4H4v16h2zm10-2H6v2h10zm4 4h-2v14h2zm-2 14H6v2h12zM16 4h2v2h-2zm-4 0h2v6h-2zM12 8h6v2h-6zm-4 8h8v2H8zm0-4h8v2H8zm0-4h2v2H8z"#
    }
    IconKind::ArrowUpDown => {
      r#"M16 4h2v16h-2zm-2 10h2v4h-2zm-2 0h2v2h-2zm6 0h2v4h-2zm2 0h2v2h-2zM6 20h2V4H6zM4 10h2V6H4zm-2 0h2V8H2zm6 0h2V6H8zm2 0h2V8h-2z"#
    }
    IconKind::Network => {
      r#"M10 0h4v2h-4zM8 2h8v2h-8zM10 4h4v6h-4zM0 8h8v2h-8zM16 8h8v2h-8zM0 10h2v2h-2zM6 10h12v2h-12zM22 10h2v2h-2zM0 12h8v2h-8zM10 12h4v4h-4zM16 12h8v2h-8zM6 16h12v2h-12zM6 18h2v2h-2zM16 18h2v2h-2zM6 20h12v2h-12z"#
    }
    IconKind::Plus => r#"M13 11h7v2h-7v7h-2v-7H4v-2h7V4h2v7Z"#,
    IconKind::Minus => r#"M4 11h16v2H4z"#,
    IconKind::ChevronLeft => {
      r#"M8 13v-2h2v2H8Zm2-2V9h2v2h-2Zm0 4v-2h2v2h-2Zm2-6V7h2v2h-2Zm0 8v-2h2v2h-2Zm2-10V5h2v2h-2Zm0 12v-2h2v2h-2Z"#
    }
    IconKind::ChevronRight => {
      r#"M16 13v-2h-2v2h2Zm-2-2V9h-2v2h2Zm0 4v-2h-2v2h2Zm-2-6V7h-2v2h2Zm0 8v-2h-2v2h2ZM10 7V5H8v2h2Zm0 12v-2H8v2h2Z"#
    }
    IconKind::Calendar => {
      r#"M5 4h14v2H5zm0 16h14v2H5zM3 10h2v10H3zm0-4h2v2H3zm16 0h2v2h-2zm0 4h2v10h-2zM3 8h18v2H3zm12-6h2v2h-2zM7 2h2v2H7z"#
    }
    IconKind::Clock => {
      r#"M6 2h12v2H6zM2 6h2v12H2zm18 0h2v12h-2zm-2-2h2v2h-2zM4 4h2v2H4zm2 18h12v-2H6zm12-2h2v-2h-2zM4 20h2v-2H4zm7-14h2v7h-2zm2 7h2v2h-2zm2 2h2v2h-2z"#
    }
  }
}
