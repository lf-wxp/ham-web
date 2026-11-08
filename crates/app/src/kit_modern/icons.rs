//! 现代风线性图标（1.8px 描边、24 网格）。
//!
//! 与 `icons/pixel` 是两套资产，**不共用**：像素图标源自 24px 点阵网格，放大后会露出方块边缘，
//! 在现代风里就是最明显的「上一套 UI 的影子」。这里只收录现代页面实际用到的形状，
//! 用到新图标就在 [`MIcon`] 加一个分支（穷尽 `match`，漏了直接编译失败）。

use leptos::prelude::*;

/// 现代风图标集（按形状命名，不按业务命名：同一个图标可以被多处复用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MIcon {
  Antenna,
  ArrowRight,
  Chart,
  Clock,
  Code,
  Gauge,
  Loader,
  Map,
  Refresh,
  Satellite,
  Signal,
  Star,
  Trophy,
  Check,
  ChevronDown,
  Play,
  Close,
  Book,
  Calculator,
  Calendar,
  Download,
  ExternalLink,
  Minus,
  Plus,
  Search,
  Sliders,
  Settings,
  Flame,
  Home,
  Menu,
  Tools,
  Volume,
  Alert,
  Bookmark,
  Camera,
  Moon,
  Sun,
  CircleDot,
  Card,
  User,
  Grid,
  Message,
  Shield,
  Zap,
  Cpu,
  Battery,
  Monitor,
  Send,
  Mountain,
  Wrench,
  Mail,
  Plane,
  Cloud,
  Doc,
  ArrowsUpDown,
  Network,
  Anchor,
  List,
}

impl MIcon {
  /// 图标的绘制指令（`<path>` / `<circle>` 混合，统一 24×24 视框）。
  const fn body(self) -> &'static [&'static str] {
    match self {
      Self::Antenna => &["M12 20V9", "M8 3l4 6 4-6", "M6 20h12", "M9 13h6"],
      Self::Calendar => &["M4 6h16v14H4z", "M8 3v4", "M16 3v4", "M4 10h16"],
      Self::Download => &["M12 4v10", "M8 11l4 4 4-4", "M5 19h14"],
      Self::ExternalLink => &["M14 4h6v6", "M20 4l-8 8", "M19 14v5H5V5h5"],
      Self::Minus => &["M5 12h14"],
      Self::Plus => &["M12 5v14", "M5 12h14"],
      Self::Sliders => &[
        "M4 7h10", "M18 7h2", "M16 5v4", "M4 17h6", "M14 17h6", "M10 15v4",
      ],
      Self::Calculator => &[
        "M6 3h12v18H6z",
        "M9 7h6",
        "M9 11h1.5",
        "M13.5 11H15",
        "M9 15h1.5",
        "M13.5 15H15",
        "M9 18.5h6",
      ],
      Self::Satellite => &[
        "M9.5 6.5h5v5h-5z",
        "M3.5 8.5h3v4h-3z",
        "M17.5 8.5h3v4h-3z",
        "M12 11.5v3",
        "M8 19l8-4.5",
      ],
      Self::ArrowRight => &["M4 12h15", "M13 6l6 6-6 6"],
      Self::Chart => &["M4 20V11", "M10 20V4", "M16 20v-6", "M22 20H2"],
      Self::Clock => &["M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z", "M12 7v5l3.5 2"],
      Self::Code => &["M9 8l-4 4 4 4", "M15 8l4 4-4 4"],
      Self::Gauge => &["M4 18a9 9 0 1 1 16 0", "M12 14l4-4"],
      Self::Map => &["M9 4 3 6v14l6-2 6 2 6-2V4l-6 2-6-2z", "M9 4v14", "M15 6v14"],
      // 圆弧圆心必须正好落在 24 网格中心 (12,12)：`animate-spin` 绕元素盒中心转，
      // 圆心一偏就会看到整个圆环在「公转」。下面两条 Refresh / Loader 都由 (12,12)、
      // r=9 的圆构成，别再手写偏移的近似弧。
      Self::Refresh => &[
        "M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8",
        "M21 3v5h-5",
      ],
      Self::Loader => &["M21 12a9 9 0 1 1-6.219-8.56"],
      Self::Signal => &[
        "M4 20a16 16 0 0 1 16-16",
        "M7.5 20a11 11 0 0 1 11-11",
        "M11 20a6 6 0 0 1 6-6",
      ],
      Self::Star => &["M12 3l2.7 5.8 6.3.8-4.6 4.3 1.2 6.1L12 17.8 6.4 20l1.2-6.1L3 9.6l6.3-.8z"],
      Self::Trophy => &[
        "M8 4h8v5a4 4 0 0 1-8 0z",
        "M8 5H5v2a3 3 0 0 0 3 3",
        "M16 5h3v2a3 3 0 0 1-3 3",
        "M12 13v4",
        "M9 21h6",
      ],
      Self::Check => &["M20 6 9 17l-5-5"],
      Self::ChevronDown => &["M6 9l6 6 6-6"],
      Self::Play => &["M8 5.5v13l11-6.5z"],
      Self::Close => &["M18 6 6 18", "M6 6l12 12"],
      Self::Book => &[
        "M4 5.5A2.5 2.5 0 0 1 6.5 3H20v18H6.5A2.5 2.5 0 0 1 4 18.5z",
        "M8 3v18",
      ],
      Self::Search => &["M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14z", "M16.5 16.5 21 21"],
      // 齿轮：内圈 + 齿根圈 + 八颗齿（四正四斜）。原来是「圆 + 四根短线」，3× 放大下就是准星，
      // 顶栏唯一的设置入口靠形状读不出语义（台账 A2）。齿用短线段而非闭合多边形：
      // 20px 下 1.8px 描边的闭合齿会糊成一圈毛边。
      Self::Settings => &[
        "M12 9.4a2.6 2.6 0 1 0 0 5.2 2.6 2.6 0 0 0 0-5.2z",
        "M12 6.6a5.4 5.4 0 1 0 0 10.8 5.4 5.4 0 0 0 0-10.8z",
        "M12 6.8V3.4",
        "M12 17.2v3.4",
        "M6.8 12H3.4",
        "M17.2 12h3.4",
        "M15.7 8.3l2.4-2.4",
        "M15.7 15.7l2.4 2.4",
        "M8.3 15.7l-2.4 2.4",
        "M8.3 8.3L5.9 5.9",
      ],
      Self::Home => &[
        "M4 11.5 12 5l8 6.5",
        "M6.5 10.5V19h11v-8.5",
        "M10 19v-4.5h4V19",
      ],
      Self::Flame => &[
        "M12 3c2.2 2.6 5 4.6 5 8a5 5 0 0 1-10 0c0-2 1-3.4 2.2-4.4.4 1.4 1.3 2 2.3 2C11 7 12 5 12 3z",
      ],
      Self::Menu => &["M4 7h16", "M4 12h16", "M4 17h16"],
      Self::Tools => &[
        "M4 7h10", "M18 7h2", "M16 5v4", "M4 17h6", "M14 17h6", "M10 15v4",
      ],
      Self::Volume => &[
        "M4 9.5h3.5L12 5.5v13L7.5 14.5H4z",
        "M16 9a4 4 0 0 1 0 6",
        "M18.5 6.5a7.5 7.5 0 0 1 0 11",
      ],
      Self::Alert => &[
        "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z",
        "M12 8v5",
        "M12 16.5v.01",
      ],
      Self::Bookmark => &["M7 4h10v17l-5-4-5 4z"],
      Self::Camera => &[
        "M4 8h3l1.5-2h7L17 8h3v11H4z",
        "M12 11a3 3 0 1 0 0 6 3 3 0 0 0 0-6z",
      ],
      Self::Moon => &["M20 14.5A8 8 0 0 1 9.5 4a6.8 6.8 0 1 0 10.5 10.5z"],
      Self::Sun => &[
        "M12 8.5a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7",
        "M12 3v2",
        "M12 19v2",
        "M3 12h2",
        "M19 12h2",
        "M5.6 5.6l1.5 1.5",
        "M16.9 16.9l1.5 1.5",
        "M18.4 5.6l-1.5 1.5",
        "M7.1 16.9l-1.5 1.5",
      ],
      Self::CircleDot => &[
        "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z",
        "M12 10a2 2 0 1 0 0 4 2 2 0 0 0 0-4z",
      ],
      Self::Card => &["M3 6h18v12H3z", "M3 10h18"],
      Self::User => &[
        "M12 4a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7z",
        "M5 20c0-3.4 3.1-5.6 7-5.6s7 2.2 7 5.6",
      ],
      Self::Grid => &[
        "M4 4h7v7H4z",
        "M13 4h7v7h-7z",
        "M4 13h7v7H4z",
        "M13 13h7v7h-7z",
      ],
      Self::Message => &["M4 5h16v10H9l-5 4z"],
      Self::Shield => &["M12 3l8 3v6c0 5-3.8 8.2-8 9-4.2-.8-8-4-8-9V6z"],
      Self::Zap => &["M13 3L5 14h6l-1 7 8-11h-6z"],
      Self::Cpu => &[
        "M7 7h10v10H7z",
        "M10 4v3",
        "M14 4v3",
        "M10 17v3",
        "M14 17v3",
        "M4 10h3",
        "M4 14h3",
        "M17 10h3",
        "M17 14h3",
      ],
      Self::Battery => &["M3 8h14v8H3z", "M20 11v2", "M10 10l-2 3h3l-1 3"],
      Self::Monitor => &["M3 5h18v11H3z", "M9 20h6", "M12 16v4"],
      Self::Send => &["M4 12l16-8-6 16-4-7z"],
      Self::Mountain => &["M3 18l6-9 4 6 3-4 5 7z"],
      Self::Wrench => &["M14 4l6 6-3 3-6-6z", "M11 10l-6 6 3 3 6-6"],
      Self::Mail => &["M3 6h18v12H3z", "M3 7l9 6 9-6"],
      Self::Plane => &["M12 3l2 6.5 7 3-7 2-2 6.5-2-6.5-7-2 7-3z"],
      Self::Cloud => &["M7 17a4 4 0 0 1 0-8 5 5 0 0 1 10 1 3.5 3.5 0 0 1 0 7z"],
      Self::Doc => &["M6 3h9l3 3v15H6z", "M9 10h6", "M9 14h6"],
      Self::ArrowsUpDown => &["M8 4v16", "M5 8l3-4 3 4", "M16 20V4", "M13 16l3 4 3-4"],
      Self::Network => &["M5 4h4v4H5z", "M15 16h4v4h-4z", "M9 6h4v10h2"],
      Self::Anchor => &[
        "M12 5v14",
        "M6 19h12",
        "M8 9h8",
        "M12 7a2 2 0 1 0 0-4 2 2 0 0 0 0 4z",
      ],
      Self::List => &["M5 7h14", "M5 12h14", "M5 17h10"],
    }
  }
}

/// 现代风图标。默认 20px；只允许改尺寸与颜色，不参与像素规范的 24px 网格约束。
#[component]
pub fn ModernIcon(
  kind: MIcon,
  /// 响应式类名：下拉箭头这类图标要随展开状态加 / 去 `rotate-180`。
  #[prop(optional, into)]
  class: Signal<String>,
  /// 有可访问名称时才画成 `img`，否则对读屏隐藏（装饰图标）。
  #[prop(optional, into)]
  label: Option<String>,
) -> impl IntoView {
  let a11y = if label.is_some() {
    // 画成 `img` 时必须是 `aria-hidden="false"`：`"0"` 不是合法取值（只认 true / false）。
    ("img", "false")
  } else {
    ("presentation", "true")
  };
  view! {
    <svg
      class=move || class.get()
      width="20"
      height="20"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.8"
      stroke-linecap="round"
      stroke-linejoin="round"
      role=a11y.0
      aria-hidden=a11y.1
    >
      {label.map(|l| view! { <title>{l}</title> })}
      {kind.body().iter().map(|d| view! { <path d=*d /> }).collect_view()}
    </svg>
  }
}
