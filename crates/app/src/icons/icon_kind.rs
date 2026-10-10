//! 图标种类枚举与 Lucide 名称映射。
//!
//! 名称（`name()`）是注册表与 `icon_of` 反向查表的契约；渲染用的像素路径在 `pixel/`。

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
  /// `Plus`（plus）：数字输入的「加」按钮
  Plus,
  /// `Minus`（minus）：数字输入的「减」按钮
  Minus,
  /// `ChevronLeft`（chevron-left）：日期选择器切换到上个月
  ChevronLeft,
  /// `ChevronRight`（chevron-right）：日期选择器切换到下个月
  ChevronRight,
  /// `Calendar`（calendar）：日期选择器触发按钮
  Calendar,
  /// `Clock`（clock）：时间选择器触发按钮
  Clock,
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
      Self::Plus => "plus",
      Self::Minus => "minus",
      Self::ChevronLeft => "chevron-left",
      Self::ChevronRight => "chevron-right",
      Self::Calendar => "calendar",
      Self::Clock => "clock",
    }
  }
}
