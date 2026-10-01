//! 图标种类枚举与 Lucide SVG 路径映射。

/// 用到的图标。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconKind {
  Settings,
  Search,
  X,
  ChevronDown,
  Check,
  /// `CheckCircle`（circle-check-big）
  CheckCircle,
  /// `CheckCircle2`（circle-check）
  CheckCircle2,
  ExternalLink,
  /// `Loader2`（loader-circle）
  Loader2,
  /// `XCircle`（circle-x）
  XCircle,
  /// `AlertCircle`（circle-alert）
  AlertCircle,
  RefreshCw,
  Camera,
  Home,
  Moon,
  Sun,
  Circle,
  CreditCard,
  User,
  Download,
  Satellite,
  Menu,
  ClipboardList,
  Timer,
  BookOpen,
  LayoutGrid,
  /// `Radio`（radio）
  Radio,
  /// `AudioLines`（audio-lines）
  AudioLines,
  /// `Play`（play）
  Play,
  /// `CaseUpper`（case-upper）
  CaseUpper,
  /// `BookMarked`（book-marked）
  BookMarked,
  /// `RadioTower`（radio-tower）
  RadioTower,
  /// `Calculator`（calculator）
  Calculator,
  /// `Globe`（globe）
  Globe,
  /// `SlidersHorizontal`（sliders-horizontal）
  SlidersHorizontal,
  /// `Binary`（binary）
  Binary,
  /// `Gauge`（gauge）
  Gauge,
  /// `Orbit`（orbit）
  Orbit,
  /// `MessagesSquare`（messages-square）
  MessagesSquare,
  /// `Signal`（signal）
  Signal,
  /// `Waves`（waves）
  Waves,
  /// `ListX`（list-x）
  ListX,
  /// `SunMedium`（sun-medium）
  SunMedium,
  /// `ShieldAlert`（shield-alert）
  ShieldAlert,
  /// `BadgeCheck`（badge-check）
  BadgeCheck,
  /// `Volume2`（volume-2）
  Volume2,
  /// `Bookmark`（bookmark）
  Bookmark,
  /// `Zap`（zap）
  Zap,
  /// `Trophy`（trophy）
  Trophy,
  /// `Cpu`（cpu）
  Cpu,
  /// `PlugZap`（plug-zap）
  PlugZap,
  /// `Activity`（activity）
  Activity,
  /// `BatteryCharging`（battery-charging）
  BatteryCharging,
  /// `Award`（award）
  Award,
  /// `Map`（map）
  Map,
  /// `Monitor`（monitor）
  Monitor,
  /// `Siren`（siren）
  Siren,
  /// `GraduationCap`（graduation-cap）
  GraduationCap,
  /// `Landmark`（landmark）
  Landmark,
  /// `Compass`（compass）
  Compass,
  /// `Sparkles`（sparkles）
  Sparkles,
  /// `Wrench`（wrench）
  Wrench,
  /// `Send`（send）
  Send,
  /// `History`（history）
  History,
  /// `TrendingUp`（trending-up）
  TrendingUp,
  /// `Mountain`（mountain）
  Mountain,
  /// `Hammer`（hammer）
  Hammer,
  /// `Mail`（mail）
  Mail,
  /// `ShieldX`（shield-x）
  ShieldX,
  /// `BatteryLow`（battery-low）
  BatteryLow,
  /// `Plane`（plane）
  Plane,
  /// `Scale`（scale）
  Scale,
  /// `Waypoints`（waypoints）
  Waypoints,
  /// `Cloud`（cloud）
  Cloud,
  /// `Type`（type）
  Type,
  /// `Anchor`（anchor）
  Anchor,
  /// `Workflow`（workflow）
  Workflow,
  /// `Headphones`（headphones）
  Headphones,
  /// `Flame`（flame）
  Flame,
  /// `Tv`（tv）
  Tv,
  /// `Filter`（filter）
  Filter,
  /// `Box`（box）
  Box,
  /// `Star`（star）
  Star,
  /// `Building2`（building-2）
  Building2,
  /// `FileText`（file-text）
  FileText,
  /// `ArrowUpDown`（arrow-up-down）
  ArrowUpDown,
  /// `Network`（network）
  Network,
}

impl IconKind {
  pub(crate) const fn name(self) -> &'static str {
    match self {
      Self::Settings => "settings",
      Self::Search => "search",
      Self::X => "x",
      Self::ChevronDown => "chevron-down",
      Self::Check => "check",
      Self::CheckCircle => "circle-check-big",
      Self::CheckCircle2 => "circle-check",
      Self::ExternalLink => "external-link",
      Self::Loader2 => "loader-circle",
      Self::XCircle => "circle-x",
      Self::AlertCircle => "circle-alert",
      Self::RefreshCw => "refresh-cw",
      Self::Camera => "camera",
      Self::Home => "house",
      Self::Moon => "moon",
      Self::Sun => "sun",
      Self::Circle => "circle",
      Self::CreditCard => "credit-card",
      Self::User => "user",
      Self::Download => "download",
      Self::Satellite => "satellite",
      Self::Menu => "menu",
      Self::ClipboardList => "clipboard-list",
      Self::Timer => "timer",
      Self::BookOpen => "book-open",
      Self::LayoutGrid => "layout-grid",
      Self::Radio => "radio",
      Self::AudioLines => "audio-lines",
      Self::Play => "play",
      Self::CaseUpper => "case-upper",
      Self::BookMarked => "book-marked",
      Self::RadioTower => "radio-tower",
      Self::Calculator => "calculator",
      Self::Globe => "globe",
      Self::SlidersHorizontal => "sliders-horizontal",
      Self::Binary => "binary",
      Self::Gauge => "gauge",
      Self::Orbit => "orbit",
      Self::MessagesSquare => "messages-square",
      Self::Signal => "signal",
      Self::Waves => "waves",
      Self::ListX => "list-x",
      Self::SunMedium => "sun-medium",
      Self::ShieldAlert => "shield-alert",
      Self::BadgeCheck => "badge-check",
      Self::Volume2 => "volume-2",
      Self::Bookmark => "bookmark",
      Self::Zap => "zap",
      Self::Trophy => "trophy",
      Self::Cpu => "cpu",
      Self::PlugZap => "plug-zap",
      Self::Activity => "activity",
      Self::BatteryCharging => "battery-charging",
      Self::Award => "award",
      Self::Map => "map",
      Self::Monitor => "monitor",
      Self::Siren => "siren",
      Self::GraduationCap => "graduation-cap",
      Self::Landmark => "landmark",
      Self::Compass => "compass",
      Self::Sparkles => "sparkles",
      Self::Wrench => "wrench",
      Self::Send => "send",
      Self::History => "history",
      Self::TrendingUp => "trending-up",
      Self::Mountain => "mountain",
      Self::Hammer => "hammer",
      Self::Mail => "mail",
      Self::ShieldX => "shield-x",
      Self::BatteryLow => "battery-low",
      Self::Plane => "plane",
      Self::Scale => "scale",
      Self::Waypoints => "waypoints",
      Self::Cloud => "cloud",
      Self::Type => "type",
      Self::Anchor => "anchor",
      Self::Workflow => "workflow",
      Self::Headphones => "headphones",
      Self::Flame => "flame",
      Self::Tv => "tv",
      Self::Filter => "filter",
      Self::Box => "box",
      Self::Star => "star",
      Self::Building2 => "building-2",
      Self::FileText => "file-text",
      Self::ArrowUpDown => "arrow-up-down",
      Self::Network => "network",
    }
  }

  pub(crate) const fn body(self) -> &'static str {
    match self {
      Self::Settings => {
        r#"<path d="M9.671 4.136a2.34 2.34 0 0 1 4.659 0 2.34 2.34 0 0 0 3.319 1.915 2.34 2.34 0 0 1 2.33 4.033 2.34 2.34 0 0 0 0 3.831 2.34 2.34 0 0 1-2.33 4.033 2.34 2.34 0 0 0-3.319 1.915 2.34 2.34 0 0 1-4.659 0 2.34 2.34 0 0 0-3.32-1.915 2.34 2.34 0 0 1-2.33-4.033 2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915"/><circle cx="12" cy="12" r="3"/>"#
      }
      Self::Search => r#"<path d="m21 21-4.34-4.34"/><circle cx="11" cy="11" r="8"/>"#,
      Self::X => r#"<path d="M18 6 6 18"/><path d="m6 6 12 12"/>"#,
      Self::ChevronDown => r#"<path d="m6 9 6 6 6-6"/>"#,
      Self::Check => r#"<path d="M20 6 9 17l-5-5"/>"#,
      Self::CheckCircle => {
        r#"<path d="M21.801 10A10 10 0 1 1 17 3.335"/><path d="m9 11 3 3L22 4"/>"#
      }
      Self::CheckCircle2 => r#"<circle cx="12" cy="12" r="10"/><path d="m9 12 2 2 4-4"/>"#,
      Self::ExternalLink => {
        r#"<path d="M15 3h6v6"/><path d="M10 14 21 3"/><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>"#
      }
      Self::Loader2 => r#"<path d="M21 12a9 9 0 1 1-6.219-8.56"/>"#,
      Self::XCircle => {
        r#"<circle cx="12" cy="12" r="10"/><path d="m15 9-6 6"/><path d="m9 9 6 6"/>"#
      }
      Self::AlertCircle => {
        r#"<circle cx="12" cy="12" r="10"/><line x1="12" x2="12" y1="8" y2="12"/><line x1="12" x2="12.01" y1="16" y2="16"/>"#
      }
      Self::RefreshCw => {
        r#"<path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M8 16H3v5"/>"#
      }
      Self::Camera => {
        r#"<path d="M14.5 4h-5L7 7H4a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-3l-2.5-3z"/><circle cx="12" cy="13" r="3"/>"#
      }
      Self::Home => {
        r#"<path d="M15 21v-8a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1v8"/><path d="M3 10a2 2 0 0 1 .709-1.528l7-5.999a2 2 0 0 1 2.582 0l7 5.999A2 2 0 0 1 21 10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>"#
      }
      Self::Moon => {
        r#"<path d="M20.985 12.486a9 9 0 1 1-9.473-9.472c.405-.022.617.46.402.803a6 6 0 0 0 8.268 8.268c.344-.215.825-.004.803.401"/>"#
      }
      Self::Sun => {
        r#"<circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="m4.93 4.93 1.41 1.41"/><path d="m17.66 17.66 1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="m6.34 17.66-1.41 1.41"/><path d="m19.07 4.93-1.41 1.41"/>"#
      }
      Self::Circle => r#"<circle cx="12" cy="12" r="10"/>"#,
      Self::CreditCard => {
        r#"<rect width="20" height="14" x="2" y="5" rx="2"/><line x1="2" x2="22" y1="10" y2="10"/>"#
      }
      Self::User => {
        r#"<path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/>"#
      }
      Self::Download => {
        r#"<path d="M12 15V3"/><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><path d="m7 10 5 5 5-5"/>"#
      }
      Self::Satellite => {
        r#"<path d="M13 7 9 3 5 7l4 4"/><path d="m17 11 4 4-4 4-4-4"/><path d="m8 12 4 4 6-6-4-4Z"/><path d="m16 8 3-3"/><path d="M9 21a6 6 0 0 0-6-6"/>"#
      }
      Self::Menu => {
        r#"<line x1="4" x2="20" y1="6" y2="6"/><line x1="4" x2="20" y1="12" y2="12"/><line x1="4" x2="20" y1="18" y2="18"/>"#
      }
      Self::ClipboardList => {
        r#"<rect width="8" height="4" x="8" y="2" rx="1" ry="1"/><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/><path d="M12 11h4"/><path d="M12 16h4"/><path d="M8 11h.01"/><path d="M8 16h.01"/>"#
      }
      Self::Timer => {
        r#"<line x1="10" x2="14" y1="2" y2="2"/><line x1="12" x2="15" y1="14" y2="11"/><circle cx="12" cy="14" r="8"/>"#
      }
      Self::BookOpen => {
        r#"<path d="M12 7v14"/><path d="M3 18a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h5a4 4 0 0 1 4 4 4 4 0 0 1 4-4h5a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1h-6a3 3 0 0 0-3 3 3 3 0 0 0-3-3z"/>"#
      }
      Self::LayoutGrid => {
        r#"<rect width="7" height="7" x="3" y="3" rx="1"/><rect width="7" height="7" x="14" y="3" rx="1"/><rect width="7" height="7" x="14" y="14" rx="1"/><rect width="7" height="7" x="3" y="14" rx="1"/>"#
      }
      Self::Radio => {
        r#"<path d="M4.9 19.1C1 15.2 1 8.8 4.9 4.9"/><path d="M7.8 16.2c-2.3-2.3-2.3-6.1 0-8.5"/><circle cx="12" cy="12" r="2"/><path d="M16.2 7.8c2.3 2.3 2.3 6.1 0 8.5"/><path d="M19.1 4.9C23 8.8 23 15.2 19.1 19.1"/>"#
      }
      Self::AudioLines => {
        r#"<path d="M2 10v3"/><path d="M6 6v11"/><path d="M10 3v18"/><path d="M14 8v7"/><path d="M18 5v13"/><path d="M22 10v3"/>"#
      }
      Self::Play => r#"<polygon points="6 3 20 12 6 21 6 3" fill="currentColor" stroke="none"/>"#,
      Self::CaseUpper => {
        r#"<path d="m3 15 4-8 4 8"/><path d="M4 13h6"/><circle cx="18" cy="12" r="3"/><path d="M21 9v6"/>"#
      }
      Self::BookMarked => {
        r#"<path d="M10 2v8l3-3 3 3V2"/><path d="M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H19a1 1 0 0 1 1 1v18a1 1 0 0 1-1 1H6.5a1 1 0 0 1 0-5H20"/>"#
      }
      Self::RadioTower => {
        r#"<path d="M4.9 16.1C1 12.2 1 5.8 4.9 1.9"/><path d="M7.8 13.8c-2.3-2.3-2.3-6.1 0-8.5"/><circle cx="12" cy="9" r="2"/><path d="M16.2 6.2c2.3 2.3 2.3 6.1 0 8.5"/><path d="M19.1 1.9C23 5.8 23 12.2 19.1 16.1"/><path d="M9.9 21.1 12 14l2.1 7.1"/>"#
      }
      Self::Calculator => {
        r#"<rect width="16" height="20" x="4" y="2" rx="2"/><line x1="8" x2="16" y1="6" y2="6"/><line x1="16" x2="16" y1="14" y2="18"/><path d="M16 10h.01"/><path d="M12 10h.01"/><path d="M8 10h.01"/><path d="M12 14h.01"/><path d="M8 14h.01"/><path d="M12 18h.01"/><path d="M8 18h.01"/>"#
      }
      Self::Globe => {
        r#"<circle cx="12" cy="12" r="10"/><path d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20"/><path d="M2 12h20"/>"#
      }
      Self::SlidersHorizontal => {
        r#"<line x1="21" x2="14" y1="4" y2="4"/><line x1="10" x2="3" y1="4" y2="4"/><line x1="21" x2="12" y1="12" y2="12"/><line x1="8" x2="3" y1="12" y2="12"/><line x1="21" x2="16" y1="20" y2="20"/><line x1="12" x2="3" y1="20" y2="20"/><line x1="14" x2="14" y1="2" y2="6"/><line x1="8" x2="8" y1="10" y2="14"/><line x1="16" x2="16" y1="18" y2="22"/>"#
      }
      Self::Binary => {
        r#"<rect x="14" y="14" width="4" height="6" rx="2"/><rect x="6" y="4" width="4" height="6" rx="2"/><path d="M6 20h4"/><path d="M14 10h4"/><path d="M6 14h2v6"/><path d="M14 4h2v6"/>"#
      }
      Self::Gauge => r#"<path d="m12 14 4-4"/><path d="M3.34 19a10 10 0 1 1 17.32 0"/>"#,
      Self::Orbit => {
        r#"<circle cx="12" cy="12" r="3"/><circle cx="19" cy="5" r="2"/><circle cx="5" cy="19" r="2"/><path d="M10.4 21.9a10 10 0 0 0 9.941-15.416"/><path d="M13.5 2.1a10 10 0 0 0-9.841 15.416"/>"#
      }
      Self::MessagesSquare => {
        r#"<path d="M14 9a2 2 0 0 1-2 2H6l-4 4V4a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2z"/><path d="M18 9h2a2 2 0 0 1 2 2v11l-4-4h-6a2 2 0 0 1-2-2v-1"/>"#
      }
      Self::Signal => {
        r#"<path d="M2 20h.01"/><path d="M7 20v-4"/><path d="M12 20v-8"/><path d="M17 20V8"/><path d="M22 4v16"/>"#
      }
      Self::Waves => {
        r#"<path d="M2 6c.6.5 1.2 1 2.5 1C7 7 7 5 9.5 5c2.6 0 2.4 2 5 2 2.5 0 2.5-2 5-2 1.3 0 1.9.5 2.5 1"/><path d="M2 12c.6.5 1.2 1 2.5 1 2.5 0 2.5-2 5-2 2.6 0 2.4 2 5 2 2.5 0 2.5-2 5-2 1.3 0 1.9.5 2.5 1"/><path d="M2 18c.6.5 1.2 1 2.5 1 2.5 0 2.5-2 5-2 2.6 0 2.4 2 5 2 2.5 0 2.5-2 5-2 1.3 0 1.9.5 2.5 1"/>"#
      }
      Self::ListX => {
        r#"<path d="M11 12H3"/><path d="M16 6H3"/><path d="M16 18H3"/><path d="m19 10-4 4"/><path d="m15 10 4 4"/>"#
      }
      Self::SunMedium => {
        r#"<circle cx="12" cy="12" r="4"/><path d="M12 3v1"/><path d="M12 20v1"/><path d="M3 12h1"/><path d="M20 12h1"/><path d="m18.364 5.636-.707.707"/><path d="m6.343 17.657-.707.707"/><path d="m5.636 5.636.707.707"/><path d="m17.657 17.657.707.707"/>"#
      }
      Self::ShieldAlert => {
        r#"<path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1 1 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/><path d="m9 12 2 2 4-4"/>"#
      }
      Self::BadgeCheck => {
        r#"<path d="M3.85 8.62a4 4 0 0 1 4.78-4.77 4 4 0 0 1 6.74 0 4 4 0 0 1 4.78 4.78 4 4 0 0 1 0 6.74 4 4 0 0 1-4.77 4.78 4 4 0 0 1-6.75 0 4 4 0 0 1-4.78-4.77 4 4 0 0 1 0-6.76Z"/><path d="m9 12 2 2 4-4"/>"#
      }
      Self::Volume2 => {
        r#"<polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/><path d="M15.54 8.46a5 5 0 0 1 0 7.07"/><path d="M19.07 4.93a10 10 0 0 1 0 14.14"/>"#
      }
      Self::Bookmark => r#"<path d="m19 21-7-4-7 4V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v16z"/>"#,
      Self::Zap => r#"<polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/>"#,
      Self::Trophy => {
        r#"<path d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6"/><path d="M18 9h1.5a2.5 2.5 0 0 0 0-5H18"/><path d="M4 22h16"/><path d="M10 14.66V17c0 .55-.47.98-.97 1.21C7.85 18.75 7 20.24 7 22"/><path d="M14 14.66V17c0 .55.47.98.97 1.21C16.15 18.75 17 20.24 17 22"/><path d="M18 2H6v7a6 6 0 0 0 12 0V2Z"/>"#
      }
      Self::Cpu => {
        r#"<rect x="4" y="4" width="16" height="16" rx="2"/><rect x="9" y="9" width="6" height="6"/><path d="M15 2v2"/><path d="M15 20v2"/><path d="M2 15h2"/><path d="M2 9h2"/><path d="M20 15h2"/><path d="M20 9h2"/><path d="M9 2v2"/><path d="M9 20v2"/>"#
      }
      Self::PlugZap => {
        r#"<path d="M6.3 20.3a2.4 2.4 0 0 0 3.4 0L12 18l-6-6-2.3 2.3a2.4 2.4 0 0 0 0 3.4Z"/><path d="m2 22 3-3"/><path d="M7.5 13.5 10 11"/><path d="M10.5 16.5 13 14"/><path d="m18 3-4 4h6l-4 4"/>"#
      }
      Self::Activity => {
        r#"<path d="M22 12h-2.48a2 2 0 0 0-1.93 1.46l-2.35 8.36a.25.25 0 0 1-.48 0L9.24 2.18a.25.25 0 0 0-.48 0l-2.35 8.36A2 2 0 0 1 4.49 12H2"/>"#
      }
      Self::BatteryCharging => {
        r#"<path d="M15 7h1a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2h-2"/><path d="M6 7H4a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2h1"/><path d="m11 7-3 5h4l-3 5"/><line x1="22" x2="22" y1="11" y2="13"/>"#
      }
      Self::Award => {
        r#"<circle cx="12" cy="8" r="6"/><path d="M15.477 12.89 17 22l-5-3-5 3 1.523-9.11"/>"#
      }
      Self::Map => {
        r#"<path d="M14.106 5.553a2 2 0 0 0 1.788 0l3.659-1.83A1 1 0 0 1 21 4.619v12.764a1 1 0 0 1-.553.894l-4.553 2.277a2 2 0 0 1-1.788 0l-4.212-2.106a2 2 0 0 0-1.788 0l-3.659 1.83A1 1 0 0 1 3 19.381V6.618a1 1 0 0 1 .553-.894l4.553-2.277a2 2 0 0 1 1.788 0z"/><path d="M15 5.764v15"/><path d="M9 3.236v15"/>"#
      }
      Self::Monitor => {
        r#"<rect width="20" height="14" x="2" y="3" rx="2"/><line x1="8" x2="16" y1="21" y2="21"/><line x1="12" x2="12" y1="17" y2="21"/>"#
      }
      Self::Siren => {
        r#"<path d="M7 18v-6a5 5 0 1 1 10 0v6"/><path d="M5 21a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-1a2 2 0 0 0-2-2H7a2 2 0 0 0-2 2z"/><path d="M21 12h1"/><path d="M18.5 4.5 18 5"/><path d="M2 12h1"/><path d="M12 2v1"/><path d="m4.929 4.929.707.707"/><path d="M12 12v6"/>"#
      }
      Self::GraduationCap => {
        r#"<path d="M21.42 10.922a1 1 0 0 0-.019-1.838L12.83 5.18a2 2 0 0 0-1.66 0L2.6 9.08a1 1 0 0 0 0 1.832l8.57 3.908a2 2 0 0 0 1.66 0z"/><path d="M22 10v6"/><path d="M6 12.5V16a6 3 0 0 0 12 0v-3.5"/>"#
      }
      Self::Landmark => {
        r#"<line x1="3" x2="21" y1="22" y2="22"/><line x1="6" x2="6" y1="18" y2="11"/><line x1="10" x2="10" y1="18" y2="11"/><line x1="14" x2="14" y1="18" y2="11"/><line x1="18" x2="18" y1="18" y2="11"/><polygon points="12 2 20 7 4 7"/>"#
      }
      Self::Compass => {
        r#"<path d="m16.24 7.76-1.804 5.411a2 2 0 0 1-1.265 1.265L7.76 16.24l1.804-5.411a2 2 0 0 1 1.265-1.265z"/><circle cx="12" cy="12" r="10"/>"#
      }
      Self::Sparkles => {
        r#"<path d="M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z"/><path d="M20 3v4"/><path d="M22 5h-4"/><path d="M4 17v2"/><path d="M5 18H3"/>"#
      }
      Self::Wrench => {
        r#"<path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/>"#
      }
      Self::Send => r#"<path d="m22 2-7 20-4-9-9-4Z"/><path d="M22 2 11 13"/>"#,
      Self::History => {
        r#"<path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M12 7v5l4 2"/>"#
      }
      Self::TrendingUp => {
        r#"<polyline points="22 7 13.5 15.5 8.5 10.5 2 17"/><polyline points="16 7 22 7 22 13"/>"#
      }
      Self::Mountain => r#"<path d="m8 3 4 8 5-5 5 15H2L8 3z"/>"#,
      Self::Hammer => {
        r#"<path d="m15 12-8.373 8.373a1 1 0 1 1-3-3L12 9"/><path d="m18 15 4-4"/><path d="m21.5 11.5-1.914-1.914A2 2 0 0 1 19 8.172V7l-2.26-2.26a6 6 0 0 0-4.202-1.756L9 2.96l.92.82A6.18 6.18 0 0 1 12 8.4V10l2 2h1.172a2 2 0 0 1 1.414.586L18.5 14.5"/>"#
      }
      Self::Mail => {
        r#"<rect width="20" height="16" x="2" y="4" rx="2"/><path d="m22 7-8.97 5.7a1.94 1.94 0 0 1-2.06 0L2 7"/>"#
      }
      Self::ShieldX => {
        r#"<path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1 1 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/><path d="m14.5 9.5-5 5"/><path d="m9.5 9.5 5 5"/>"#
      }
      Self::BatteryLow => {
        r#"<rect width="16" height="10" x="2" y="7" rx="2" ry="2"/><line x1="22" x2="22" y1="11" y2="13"/><line x1="6" x2="6" y1="11" y2="13"/>"#
      }
      Self::Plane => {
        r#"<path d="M17.8 19.2 16 11l3.5-3.5C21 6 21.5 4 21 3c-1-.5-3 0-4.5 1.5L13 8 4.8 6.2c-.5-.1-.9.1-1.1.5l-.3.5c-.2.5-.1 1 .3 1.3L9 12l-2 3H4l-1 1 3 2 2 3 1-1v-3l3-2 3.5 5.3c.3.4.8.5 1.3.3l.5-.2c.4-.3.6-.7.5-1.2z"/>"#
      }
      Self::Scale => {
        r#"<path d="m16 16 3-8 3 8c-.87.65-1.92 1-3 1s-2.13-.35-3-1Z"/><path d="m2 16 3-8 3 8c-.87.65-1.92 1-3 1s-2.13-.35-3-1Z"/><path d="M7 21h10"/><path d="M12 3v18"/><path d="M3 7h2c2 0 5-1 7-2 2 1 5 2 7 2h2"/>"#
      }
      Self::Waypoints => {
        r#"<circle cx="12" cy="4.5" r="2.5"/><path d="m10.2 6.3-3.9 3.9"/><circle cx="4.5" cy="12" r="2.5"/><path d="M7 12h10"/><circle cx="19.5" cy="12" r="2.5"/><path d="m13.8 17.7 3.9-3.9"/><circle cx="12" cy="19.5" r="2.5"/>"#
      }
      Self::Cloud => r#"<path d="M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z"/>"#,
      Self::Type => {
        r#"<polyline points="4 7 4 4 20 4 20 7"/><line x1="9" x2="15" y1="20" y2="20"/><line x1="12" x2="12" y1="4" y2="20"/>"#
      }
      Self::Anchor => {
        r#"<path d="M12 22V8"/><path d="M5 12H2a10 10 0 0 0 20 0h-3"/><circle cx="12" cy="5" r="3"/>"#
      }
      Self::Workflow => {
        r#"<rect width="8" height="8" x="3" y="3" rx="2"/><path d="M7 11v4a2 2 0 0 0 2 2h4"/><rect width="8" height="8" x="13" y="13" rx="2"/>"#
      }
      Self::Headphones => {
        r#"<path d="M3 14h3a2 2 0 0 1 2 2v3a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-7a9 9 0 0 1 18 0v7a2 2 0 0 1-2 2h-1a2 2 0 0 1-2-2v-3a2 2 0 0 1 2-2h3"/>"#
      }
      Self::Flame => {
        r#"<path d="M8.5 14.5A2.5 2.5 0 0 0 11 12c0-1.38-.5-2-1-3-1.072-2.143-.224-4.054 2-6 .5 2.5 2 4.9 4 6.5 2 1.6 3 3.5 3 5.5a7 7 0 1 1-14 0c0-1.153.433-2.294 1-3a2.5 2.5 0 0 0 2.5 2.5z"/>"#
      }
      Self::Tv => {
        r#"<rect width="20" height="15" x="2" y="7" rx="2" ry="2"/><polyline points="17 2 12 7 7 2"/>"#
      }
      Self::Filter => r#"<polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3"/>"#,
      Self::Box => {
        r#"<path d="M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z"/><path d="m3.3 7 8.7 5 8.7-5"/><path d="M12 22V12"/>"#
      }
      Self::Star => {
        r#"<path d="M11.525 2.295a.53.53 0 0 1 .95 0l2.31 4.679a2.123 2.123 0 0 0 1.595 1.16l5.166.756a.53.53 0 0 1 .294.904l-3.736 3.638a2.123 2.123 0 0 0-.611 1.878l.882 5.14a.53.53 0 0 1-.771.56l-4.618-2.428a2.122 2.122 0 0 0-1.973 0L6.396 21.01a.53.53 0 0 1-.77-.56l.881-5.139a2.122 2.122 0 0 0-.611-1.879L2.16 9.795a.53.53 0 0 1 .294-.906l5.165-.755a2.122 2.122 0 0 0 1.597-1.16z"/>"#
      }
      Self::Building2 => {
        r#"<path d="M6 22V4a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v18Z"/><path d="M6 12H4a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2h2"/><path d="M18 9h2a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2h-2"/><path d="M10 6h4"/><path d="M10 10h4"/><path d="M10 14h4"/><path d="M10 18h4"/>"#
      }
      Self::FileText => {
        r#"<path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/><path d="M10 9H8"/><path d="M16 13H8"/><path d="M16 17H8"/>"#
      }
      Self::ArrowUpDown => {
        r#"<path d="m21 16-4 4-4-4"/><path d="M17 20V4"/><path d="m3 8 4-4 4 4"/><path d="M7 4v16"/>"#
      }
      Self::Network => {
        r#"<rect x="16" y="16" width="6" height="6" rx="1"/><rect x="2" y="16" width="6" height="6" rx="1"/><rect x="9" y="2" width="6" height="6" rx="1"/><path d="M5 16v-3a1 1 0 0 1 1-1h12a1 1 0 0 1 1 1v3"/><path d="M12 12V8"/>"#
      }
    }
  }
}
