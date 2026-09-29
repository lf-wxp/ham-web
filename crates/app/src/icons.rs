//! Lucide 图标（与原项目 `lucide-react@0.539` 的 SVG 路径一致）。

use leptos::prelude::*;

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
}

impl IconKind {
  const fn name(self) -> &'static str {
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
    }
  }

  const fn body(self) -> &'static str {
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
    }
  }
}

/// 渲染一个 Lucide 图标。
#[component]
pub fn Icon(kind: IconKind, #[prop(optional, into)] class: Signal<String>) -> impl IntoView {
  let class = move || format!("lucide lucide-{} {}", kind.name(), class.get());
  view! {
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
      class=class
      aria-hidden="true"
      inner_html=kind.body()
    ></svg>
  }
}
