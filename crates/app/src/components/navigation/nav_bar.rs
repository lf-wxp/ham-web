use leptos::ev;
use leptos::prelude::*;
use leptos_router::hooks::use_location;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;

use crate::cn::cn;
use crate::icons::{Icon, IconKind};
use crate::ui::{Size, Variant, button_class};

use super::theme_toggle::ThemeToggle;

/// 一条导航项。
struct NavItem {
  href: &'static str,
  label: &'static str,
  icon: IconKind,
}

/// 一组导航项（用于知识库分组）。
struct NavGroup {
  label: &'static str,
  icon: IconKind,
  items: &'static [NavItem],
}

/// 桌面端下拉菜单种类。
#[derive(Clone, Copy, PartialEq)]
enum MenuKind {
  Exam,
  Knowledge,
  Tools,
}

/// 考试中心菜单。
const EXAM_ITEMS: &[NavItem] = &[
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

/// 知识库分组（桌面端以分组多列展示）。
const KNOWLEDGE_GROUPS: &[NavGroup] = &[
  NavGroup {
    label: "备考速查",
    icon: IconKind::BookMarked,
    items: &[
      NavItem {
        href: "/reference",
        label: "考试速查",
        icon: IconKind::BookMarked,
      },
      NavItem {
        href: "/prefixes",
        label: "呼号前缀",
        icon: IconKind::Globe,
      },
      NavItem {
        href: "/glossary",
        label: "术语表",
        icon: IconKind::BookOpen,
      },
      NavItem {
        href: "/q-code",
        label: "简语",
        icon: IconKind::Radio,
      },
      NavItem {
        href: "/phonetic",
        label: "字母解释法",
        icon: IconKind::CaseUpper,
      },
      NavItem {
        href: "/rst",
        label: "RST 信号报告",
        icon: IconKind::Signal,
      },
      NavItem {
        href: "/morse",
        label: "莫尔斯电码",
        icon: IconKind::AudioLines,
      },
      NavItem {
        href: "/cw-operating",
        label: "CW 操作（等幅波）",
        icon: IconKind::AudioLines,
      },
      NavItem {
        href: "/license-classes",
        label: "操作证权限",
        icon: IconKind::BadgeCheck,
      },
    ],
  },
  NavGroup {
    label: "模式 · 传播",
    icon: IconKind::Binary,
    items: &[
      NavItem {
        href: "/analog-modes",
        label: "模拟模式",
        icon: IconKind::Volume2,
      },
      NavItem {
        href: "/atv",
        label: "业余电视",
        icon: IconKind::Tv,
      },
      NavItem {
        href: "/modes",
        label: "数字模式",
        icon: IconKind::Binary,
      },
      NavItem {
        href: "/dv-network",
        label: "数字语音组网",
        icon: IconKind::Network,
      },
      NavItem {
        href: "/rtty",
        label: "RTTY 无线电传 / PSK31",
        icon: IconKind::Type,
      },
      NavItem {
        href: "/ft8",
        label: "FT8 / FT4（数字模式）",
        icon: IconKind::Binary,
      },
      NavItem {
        href: "/sdr",
        label: "SDR 软件定义无线电",
        icon: IconKind::Monitor,
      },
      NavItem {
        href: "/gnuradio",
        label: "GNU Radio",
        icon: IconKind::Workflow,
      },
      NavItem {
        href: "/aprs",
        label: "APRS 自动位置报告",
        icon: IconKind::Map,
      },
      NavItem {
        href: "/frequencies",
        label: "常用频率",
        icon: IconKind::Gauge,
      },
      NavItem {
        href: "/propagation",
        label: "传播与电离层",
        icon: IconKind::Waves,
      },
      NavItem {
        href: "/special-prop",
        label: "特殊传播",
        icon: IconKind::Sparkles,
      },
      NavItem {
        href: "/eme",
        label: "EME 月面反射（地月地）",
        icon: IconKind::Orbit,
      },
      NavItem {
        href: "/muf",
        label: "传播预测",
        icon: IconKind::TrendingUp,
      },
      NavItem {
        href: "/wspr",
        label: "WSPR 弱信号传播",
        icon: IconKind::Waves,
      },
      NavItem {
        href: "/sstv",
        label: "SSTV 慢扫描电视",
        icon: IconKind::Camera,
      },
      NavItem {
        href: "/weather-sat",
        label: "气象卫星接收",
        icon: IconKind::Satellite,
      },
      NavItem {
        href: "/packet",
        label: "Packet 分组无线电",
        icon: IconKind::Network,
      },
    ],
  },
  NavGroup {
    label: "天线 · 设备",
    icon: IconKind::RadioTower,
    items: &[
      NavItem {
        href: "/antennas",
        label: "天线型式",
        icon: IconKind::RadioTower,
      },
      NavItem {
        href: "/antenna-array",
        label: "天线阵列 / 相控阵",
        icon: IconKind::Waypoints,
      },
      NavItem {
        href: "/polarization",
        label: "天线极化",
        icon: IconKind::ArrowUpDown,
      },
      NavItem {
        href: "/feedline",
        label: "天线匹配与馈线",
        icon: IconKind::PlugZap,
      },
      NavItem {
        href: "/antenna-diy",
        label: "天线 DIY",
        icon: IconKind::Wrench,
      },
      NavItem {
        href: "/antenna-installation",
        label: "天线架设",
        icon: IconKind::Hammer,
      },
      NavItem {
        href: "/antenna-farm",
        label: "天线农场",
        icon: IconKind::Waypoints,
      },
      NavItem {
        href: "/antenna-tuning",
        label: "天线调试",
        icon: IconKind::Wrench,
      },
      NavItem {
        href: "/antenna-analyzer",
        label: "天线分析仪",
        icon: IconKind::Gauge,
      },
      NavItem {
        href: "/antenna-modeling",
        label: "天线建模",
        icon: IconKind::Box,
      },
      NavItem {
        href: "/nvis",
        label: "NVIS 近垂直入射天波",
        icon: IconKind::Cloud,
      },
      NavItem {
        href: "/electronics",
        label: "电子电路基础",
        icon: IconKind::Cpu,
      },
      NavItem {
        href: "/filters",
        label: "滤波器与双工器",
        icon: IconKind::Filter,
      },
      NavItem {
        href: "/meters",
        label: "测量仪表",
        icon: IconKind::Activity,
      },
      NavItem {
        href: "/power",
        label: "电源与电池",
        icon: IconKind::BatteryCharging,
      },
      NavItem {
        href: "/power-supply",
        label: "电源供应",
        icon: IconKind::Zap,
      },
      NavItem {
        href: "/transceiver",
        label: "收发信机",
        icon: IconKind::Radio,
      },
      NavItem {
        href: "/receiver",
        label: "接收机指标",
        icon: IconKind::Gauge,
      },
      NavItem {
        href: "/amplifier",
        label: "功率放大器",
        icon: IconKind::Flame,
      },
      NavItem {
        href: "/bands",
        label: "波段表",
        icon: IconKind::Satellite,
      },
      NavItem {
        href: "/bandplan",
        label: "波段规划",
        icon: IconKind::SlidersHorizontal,
      },
      NavItem {
        href: "/microwave",
        label: "微波通信",
        icon: IconKind::Radio,
      },
      NavItem {
        href: "/mobile",
        label: "车载 / 移动电台",
        icon: IconKind::RadioTower,
      },
    ],
  },
  NavGroup {
    label: "通联 · 活动",
    icon: IconKind::MessagesSquare,
    items: &[
      NavItem {
        href: "/operating",
        label: "通联实务",
        icon: IconKind::MessagesSquare,
      },
      NavItem {
        href: "/contest",
        label: "通联竞赛",
        icon: IconKind::Trophy,
      },
      NavItem {
        href: "/cabrillo",
        label: "竞赛日志 Cabrillo",
        icon: IconKind::FileText,
      },
      NavItem {
        href: "/awards",
        label: "DX 奖状",
        icon: IconKind::Award,
      },
      NavItem {
        href: "/iota",
        label: "IOTA 海岛通联（空中岛屿）",
        icon: IconKind::Anchor,
      },
      NavItem {
        href: "/dx",
        label: "DX 远距离通信技巧",
        icon: IconKind::Globe,
      },
      NavItem {
        href: "/dxpedition",
        label: "DX 远征",
        icon: IconKind::Plane,
      },
      NavItem {
        href: "/most-wanted",
        label: "DXCC 世纪俱乐部",
        icon: IconKind::Star,
      },
      NavItem {
        href: "/qrp",
        label: "QRP 低功率",
        icon: IconKind::BatteryLow,
      },
      NavItem {
        href: "/eqsl",
        label: "电子 QSL",
        icon: IconKind::Send,
      },
      NavItem {
        href: "/qsl-card",
        label: "QSL 卡片设计",
        icon: IconKind::Mail,
      },
      NavItem {
        href: "/ardf",
        label: "无线电测向",
        icon: IconKind::Compass,
      },
      NavItem {
        href: "/emcomm",
        label: "应急通信",
        icon: IconKind::Siren,
      },
      NavItem {
        href: "/portable",
        label: "SOTA 山顶 / POTA 公园",
        icon: IconKind::Mountain,
      },
      NavItem {
        href: "/grid",
        label: "网格定位",
        icon: IconKind::Map,
      },
      NavItem {
        href: "/repeater",
        label: "中继台与网关",
        icon: IconKind::RadioTower,
      },
      NavItem {
        href: "/repeater-build",
        label: "中继台建设",
        icon: IconKind::Building2,
      },
      NavItem {
        href: "/logging-software",
        label: "日志与竞赛软件",
        icon: IconKind::ClipboardList,
      },
    ],
  },
  NavGroup {
    label: "进阶 · 关于",
    icon: IconKind::Landmark,
    items: &[
      NavItem {
        href: "/organizations",
        label: "国际组织与分区",
        icon: IconKind::Landmark,
      },
      NavItem {
        href: "/safety",
        label: "射频安全",
        icon: IconKind::ShieldAlert,
      },
      NavItem {
        href: "/grounding",
        label: "接地与防雷",
        icon: IconKind::Zap,
      },
      NavItem {
        href: "/rfi",
        label: "射频干扰排查",
        icon: IconKind::ShieldX,
      },
      NavItem {
        href: "/beginner",
        label: "新手入门",
        icon: IconKind::GraduationCap,
      },
      NavItem {
        href: "/swl",
        label: "SWL 短波监听",
        icon: IconKind::Headphones,
      },
      NavItem {
        href: "/license",
        label: "执照申办",
        icon: IconKind::BadgeCheck,
      },
      NavItem {
        href: "/regulations",
        label: "法规与管理",
        icon: IconKind::Scale,
      },
      NavItem {
        href: "/history",
        label: "业余无线电历史",
        icon: IconKind::History,
      },
      NavItem {
        href: "/remote",
        label: "远程电台",
        icon: IconKind::Globe,
      },
    ],
  },
];

/// 工具菜单。
const TOOL_ITEMS: &[NavItem] = &[
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

#[component]
pub fn Navigation() -> impl IntoView {
  let location = use_location();
  let menu_open = RwSignal::new(false);
  let open_menu = RwSignal::new(None::<MenuKind>);

  // 点击导航外部时关闭桌面端下拉。
  let outside_handle = window_event_listener(ev::click, move |e| {
    if let Some(el) = e
      .target()
      .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
    {
      let inside = el.closest("[data-nav]").ok().flatten().is_some();
      if !inside {
        open_menu.set(None);
      }
    }
  });
  let outside_handle = SendWrapper::new(Some(outside_handle));
  on_cleanup(move || {
    if let Some(h) = outside_handle.take() {
      h.remove();
    }
  });

  // 全局「/」快捷键：非输入框内按下打开站内搜索面板。
  let search_open = expect_context::<RwSignal<bool>>();
  let search_handle = window_event_listener(ev::keydown, move |e| {
    if let Some(el) = e
      .target()
      .and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok())
    {
      let tag = el.tag_name().to_lowercase();
      if tag == "input" || tag == "textarea" || el.is_content_editable() {
        return;
      }
    }
    if e.key() == "/" {
      e.prevent_default();
      search_open.set(true);
    }
  });
  let search_handle = SendWrapper::new(Some(search_handle));
  on_cleanup(move || {
    if let Some(h) = search_handle.take() {
      h.remove();
    }
  });

  let active = move |href: &'static str| location.pathname.get() == href;
  let exam_active = move || EXAM_ITEMS.iter().any(|i| active(i.href));
  let knowledge_active = move || {
    KNOWLEDGE_GROUPS
      .iter()
      .any(|g| g.items.iter().any(|i| active(i.href)))
  };
  let tool_active = move || TOOL_ITEMS.iter().any(|i| active(i.href));

  view! {
    <nav data-nav class="sticky top-0 z-50 border-b bg-background/80 backdrop-blur-xl supports-[backdrop-filter]:bg-background/60">
      <div class="pointer-events-none absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-primary/50 to-transparent"></div>
      <div class="container mx-auto px-4">
        <div class="flex h-16 items-center justify-between gap-3">
          // 品牌标识
          <a
            href="/"
            class="group flex shrink-0 items-center gap-2.5"
            on:click=move |_| menu_open.set(false)
          >
            <div class="relative flex size-9 items-center justify-center overflow-hidden rounded-lg bg-primary text-primary-foreground shadow-xs shadow-primary/30 transition-transform duration-300 group-hover:rotate-6 group-hover:scale-110">
              <span class="absolute inset-0 bg-gradient-to-b from-white/20 to-black/15"></span>
              <Icon kind=IconKind::Satellite class="relative h-5 w-5" />
            </div>
            <div class="hidden flex-col whitespace-nowrap leading-tight sm:flex">
              <span class="text-sm font-semibold text-foreground">"业余无线电"</span>
              <span class="text-[11px] text-muted-foreground">"题库 · 知识 · 工具"</span>
            </div>
          </a>

          // 桌面端导航
          <div class="hidden items-center gap-1 xl:flex">
            <a
              href="/"
              data-slot="button"
              class=move || {
                button_class(
                  if active("/") { Variant::Default } else { Variant::Ghost },
                  Size::Sm,
                  "inline-flex items-center gap-2 whitespace-nowrap",
                )
              }
            >
              <Icon kind=IconKind::Home class="h-4 w-4" />
              "首页"
            </a>

            // 考试中心下拉
            <div class="group relative">
              <button
                type="button"
                data-slot="button"
                aria-expanded=move || open_menu.get() == Some(MenuKind::Exam)
                class=move || {
                  button_class(
                    if exam_active() { Variant::Default } else { Variant::Ghost },
                    Size::Sm,
                    "inline-flex items-center gap-2 whitespace-nowrap",
                  )
                }
                on:click=move |_| {
                  open_menu.update(|m| {
                    *m = if *m == Some(MenuKind::Exam) { None } else { Some(MenuKind::Exam) };
                  });
                }
              >
                <Icon kind=IconKind::Timer class="h-4 w-4" />
                "考试中心"
                <Icon kind=IconKind::ChevronDown class="h-3.5 w-3.5" />
              </button>
              <div class=move || {
                cn(&[
                  "absolute left-0 top-full pt-1.5 transition",
                  if open_menu.get() == Some(MenuKind::Exam) {
                    "visible opacity-100"
                  } else {
                    "invisible opacity-0"
                  },
                ])
              }>
                <div class="w-44 rounded-lg border bg-popover p-1 shadow-md">
                  {EXAM_ITEMS
                    .iter()
                    .map(|item| {
                      view! {
                        <a
                          href=item.href
                          on:click=move |_| open_menu.set(None)
                          class=move || {
                            cn(&[
                              "flex items-center gap-2 rounded-md px-3 py-2 text-sm transition-colors",
                              if active(item.href) {
                                "bg-accent text-foreground"
                              } else {
                                "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
                              },
                            ])
                          }
                        >
                          <Icon kind=item.icon class="h-4 w-4" />
                          {item.label}
                        </a>
                      }
                    })
                    .collect_view()}
                </div>
              </div>
            </div>

            // 知识库分组菜单
            <div class="group relative">
              <button
                type="button"
                data-slot="button"
                aria-expanded=move || open_menu.get() == Some(MenuKind::Knowledge)
                class=move || {
                  button_class(
                    if knowledge_active() { Variant::Default } else { Variant::Ghost },
                    Size::Sm,
                    "inline-flex items-center gap-2 whitespace-nowrap",
                  )
                }
                on:click=move |_| {
                  open_menu.update(|m| {
                    *m = if *m == Some(MenuKind::Knowledge) {
                      None
                    } else {
                      Some(MenuKind::Knowledge)
                    };
                  });
                }
              >
                <Icon kind=IconKind::BookOpen class="h-4 w-4" />
                "知识库"
                <Icon kind=IconKind::ChevronDown class="h-3.5 w-3.5" />
              </button>
              <div class=move || {
                cn(&[
                  "absolute left-1/2 top-full -translate-x-1/2 pt-1.5 transition",
                  if open_menu.get() == Some(MenuKind::Knowledge) {
                    "visible opacity-100"
                  } else {
                    "invisible opacity-0"
                  },
                ])
              }>
                <div class="grid w-[900px] max-w-[calc(100vw-2rem)] grid-cols-2 gap-x-4 gap-y-3 rounded-lg border bg-popover p-4 shadow-md md:grid-cols-3 lg:grid-cols-5">
                  {KNOWLEDGE_GROUPS
                    .iter()
                    .map(|g| {
                      view! {
                        <div>
                          <div class="mb-1.5 flex items-center gap-1.5 px-2 text-xs font-semibold text-muted-foreground">
                            <Icon kind=g.icon class="h-3.5 w-3.5" />
                            <span>{g.label}</span>
                          </div>
                          <div class="space-y-0.5">
                            {g
                              .items
                              .iter()
                              .map(|item| {
                                view! {
                                  <a
                                    href=item.href
                                    on:click=move |_| open_menu.set(None)
                                    class=move || {
                                      cn(&[
                                        "flex items-center gap-2 rounded-md px-2.5 py-1.5 text-sm transition-colors",
                                        if active(item.href) {
                                          "bg-accent text-foreground"
                                        } else {
                                          "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
                                        },
                                      ])
                                    }
                                  >
                                    <Icon kind=item.icon class="h-4 w-4 shrink-0" />
                                    <span class="min-w-0 leading-snug">{item.label}</span>
                                  </a>
                                }
                              })
                              .collect_view()}
                          </div>
                        </div>
                      }
                    })
                    .collect_view()}
                </div>
              </div>
            </div>

            // 工具下拉
            <div class="group relative">
              <button
                type="button"
                data-slot="button"
                aria-expanded=move || open_menu.get() == Some(MenuKind::Tools)
                class=move || {
                  button_class(
                    if tool_active() { Variant::Default } else { Variant::Ghost },
                    Size::Sm,
                    "inline-flex items-center gap-2 whitespace-nowrap",
                  )
                }
                on:click=move |_| {
                  open_menu.update(|m| {
                    *m = if *m == Some(MenuKind::Tools) { None } else { Some(MenuKind::Tools) };
                  });
                }
              >
                <Icon kind=IconKind::Calculator class="h-4 w-4" />
                "工具"
                <Icon kind=IconKind::ChevronDown class="h-3.5 w-3.5" />
              </button>
              <div class=move || {
                cn(&[
                  "absolute left-0 top-full pt-1.5 transition",
                  if open_menu.get() == Some(MenuKind::Tools) {
                    "visible opacity-100"
                  } else {
                    "invisible opacity-0"
                  },
                ])
              }>
                <div class="w-44 rounded-lg border bg-popover p-1 shadow-md">
                  {TOOL_ITEMS
                    .iter()
                    .map(|item| {
                      view! {
                        <a
                          href=item.href
                          on:click=move |_| open_menu.set(None)
                          class=move || {
                            cn(&[
                              "flex items-center gap-2 rounded-md px-3 py-2 text-sm transition-colors",
                              if active(item.href) {
                                "bg-accent text-foreground"
                              } else {
                                "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
                              },
                            ])
                          }
                        >
                          <Icon kind=item.icon class="h-4 w-4" />
                          {item.label}
                        </a>
                      }
                    })
                    .collect_view()}
                </div>
              </div>
            </div>
          </div>

          // 右侧：搜索 + 主题切换 + 移动端菜单按钮
          <div class="flex items-center gap-1">
            <button
              type="button"
              data-slot="button"
              class=button_class(Variant::Ghost, Size::Icon, "")
              aria-label="搜索"
              title="搜索（/）"
              on:click=move |_| search_open.set(true)
            >
              <Icon kind=IconKind::Search class="h-5 w-5" />
            </button>
            <ThemeToggle />
            <button
              type="button"
              data-slot="button"
              class=button_class(Variant::Ghost, Size::Icon, "xl:hidden")
              aria-label=move || if menu_open.get() { "关闭菜单" } else { "打开菜单" }
              aria-expanded=move || menu_open.get()
              on:click=move |_| menu_open.update(|v| *v = !*v)
            >
              {move || {
                if menu_open.get() {
                  view! { <Icon kind=IconKind::X class="h-5 w-5" /> }
                } else {
                  view! { <Icon kind=IconKind::Menu class="h-5 w-5" /> }
                }
              }}
            </button>
          </div>
        </div>

        // 移动端下拉菜单
        {move || {
          menu_open.get().then(|| {
            view! {
              <div class="absolute inset-x-0 top-full z-40 max-h-[calc(100vh-4rem)] overflow-y-auto border-b bg-background shadow-md xl:hidden animate-in slide-in-from-top-2 fade-in duration-200">
                <nav class="container mx-auto grid grid-cols-1 gap-1 px-4 py-3">
                  <a
                    href="/"
                    class=move || {
                      cn(&[
                        "flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-colors",
                        if active("/") {
                          "bg-accent text-foreground"
                        } else {
                          "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
                        },
                      ])
                    }
                    on:click=move |_| menu_open.set(false)
                  >
                    <Icon kind=IconKind::Home class="h-5 w-5" />
                    "首页"
                  </a>

                  <div class="px-3 pt-2 text-xs font-semibold text-muted-foreground">"考试中心"</div>
                  {EXAM_ITEMS
                    .iter()
                    .map(|item| {
                      view! {
                        <a
                          href=item.href
                          class=move || {
                            cn(&[
                              "flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-colors",
                              if active(item.href) {
                                "bg-accent text-foreground"
                              } else {
                                "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
                              },
                            ])
                          }
                          on:click=move |_| menu_open.set(false)
                        >
                          <Icon kind=item.icon class="h-5 w-5" />
                          {item.label}
                        </a>
                      }
                    })
                    .collect_view()}

                  <div class="px-3 pt-2 text-xs font-semibold text-muted-foreground">"知识库"</div>
                  {KNOWLEDGE_GROUPS
                    .iter()
                    .map(|g| {
                      view! {
                        <div class="px-3 pt-1 text-xs font-medium text-muted-foreground/70">{g.label}</div>
                        {g
                          .items
                          .iter()
                          .map(|item| {
                            view! {
                              <a
                                href=item.href
                                class=move || {
                                  cn(&[
                                    "flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-colors",
                                    if active(item.href) {
                                      "bg-accent text-foreground"
                                    } else {
                                      "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
                                    },
                                  ])
                                }
                                on:click=move |_| menu_open.set(false)
                              >
                                <Icon kind=item.icon class="h-5 w-5" />
                                {item.label}
                              </a>
                            }
                          })
                          .collect_view()}
                      }
                    })
                    .collect_view()}

                  <div class="px-3 pt-2 text-xs font-semibold text-muted-foreground">"工具"</div>
                  {TOOL_ITEMS
                    .iter()
                    .map(|item| {
                      view! {
                        <a
                          href=item.href
                          class=move || {
                            cn(&[
                              "flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-colors",
                              if active(item.href) {
                                "bg-accent text-foreground"
                              } else {
                                "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
                              },
                            ])
                          }
                          on:click=move |_| menu_open.set(false)
                        >
                          <Icon kind=item.icon class="h-5 w-5" />
                          {item.label}
                        </a>
                      }
                    })
                    .collect_view()}
                </nav>
              </div>
            }
          })
        }}
      </div>
    </nav>
  }
}
