//! 能力注册表：全站页面 / 工具与导航分组的**唯一事实来源**。
//!
//! 新增页面的流程收敛为两步：
//! 1. 在 [`MODULES`] 追加一条（路径、标题、图标、分组、是否依赖后端）；
//! 2. 在 `crates/app/src/app/main_content.rs` 加一行 `<Route>` —— Leptos 的
//!    `<Route view=…>` 需要具体组件类型，无法由数据驱动，这一步无法省略。
//!
//! 其余全部由本表派生：顶部导航三个菜单（`crates/app/src/components/navigation/`）、
//! `sitemap.xml`（`ham-web-tools postbuild`）、以及后续的 `/developers` 接口清单。
//! 一致性由 `crates/app/src/registry_check.rs` 的测试把守：注册表路径 ↔ 导航 ↔
//! 实际 `<Route>` 三者集合必须完全相同，漏注册或写错路由会直接让 CI 失败。
//!
//! [`MODULES`] 的条目由原导航与工具清单一次性迁移而来（`path` / `title` / `icon`
//! 逐字复制），此后由本文件手工维护，不再在别处保留第二份清单。

/// 模块归属的一级模块。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleKind {
  /// 考试中心（练习、模拟考、错题…）。
  Exam,
  /// 知识库专题页。
  Knowledge,
  /// 工具与实时数据页。
  Tool,
}

/// 分组：考试中心（平铺菜单，无子标题）。
pub const GROUP_EXAM: &str = "考试中心";
/// 分组：备考速查。
pub const GROUP_EXAM_REF: &str = "备考速查";
/// 分组：模式 · 传播。
pub const GROUP_MODES: &str = "模式 · 传播";
/// 分组：天线 · 设备。
pub const GROUP_ANTENNAS: &str = "天线 · 设备";
/// 分组：通联 · 活动。
pub const GROUP_OPERATING: &str = "通联 · 活动";
/// 分组：进阶 · 关于。
pub const GROUP_ADVANCED: &str = "进阶 · 关于";
/// 分组：工具 · 计算 · 解码。
pub const GROUP_TOOL_CALC: &str = "计算 · 解码";
/// 分组：工具 · 日志 · 竞赛。
pub const GROUP_TOOL_LOG: &str = "日志 · 竞赛";
/// 分组：工具 · 实时数据。
pub const GROUP_TOOL_LIVE: &str = "实时数据";
/// 分组：工具 · 地图 · 卫星。
pub const GROUP_TOOL_MAP: &str = "地图 · 卫星";
/// 分组：工具 · 训练 · 辅助。
pub const GROUP_TOOL_TRAIN: &str = "训练 · 辅助";

/// 知识库分组（按导航展示顺序）。
pub const KNOWLEDGE_GROUPS: &[&str] = &[
  GROUP_EXAM_REF,
  GROUP_MODES,
  GROUP_ANTENNAS,
  GROUP_OPERATING,
  GROUP_ADVANCED,
];

/// 工具分组（按导航展示顺序）。
pub const TOOL_GROUPS: &[&str] = &[
  GROUP_TOOL_CALC,
  GROUP_TOOL_LOG,
  GROUP_TOOL_LIVE,
  GROUP_TOOL_MAP,
  GROUP_TOOL_TRAIN,
];

/// 按分组判断所属一级模块。
#[must_use]
pub fn kind_of(group: &str) -> ModuleKind {
  if group == GROUP_EXAM {
    ModuleKind::Exam
  } else if TOOL_GROUPS.contains(&group) {
    ModuleKind::Tool
  } else {
    ModuleKind::Knowledge
  }
}

/// 分组在菜单里的标题图标（Lucide 名称，映射到 `crates/app/src/icons`）。
#[must_use]
pub fn group_icon(group: &str) -> &'static str {
  match group {
    GROUP_EXAM => "timer",
    GROUP_EXAM_REF => "book-marked",
    GROUP_MODES => "binary",
    GROUP_ANTENNAS => "radio-tower",
    GROUP_OPERATING => "messages-square",
    GROUP_ADVANCED => "landmark",
    GROUP_TOOL_CALC => "calculator",
    GROUP_TOOL_LOG => "trophy",
    GROUP_TOOL_LIVE => "activity",
    GROUP_TOOL_MAP => "map",
    GROUP_TOOL_TRAIN => "headphones",
    _ => "circle",
  }
}

/// 一个页面 / 工具。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Module {
  /// 路由路径（不含查询串），与 `main_content.rs` 的 `<Route>` 一一对应。
  pub path: &'static str,
  /// 导航链接；`None` 时即 `path`（少数入口需要带默认查询串，如练习页的只练多选）。
  pub href: Option<&'static str>,
  /// 页面标题（中文原文，同时是 i18n key，与页面里的 `set_title` 保持一致）。
  pub title: &'static str,
  /// 图标 key（Lucide 名称）。
  pub icon: &'static str,
  /// 所属导航分组；`None` 表示不在导航菜单中（仅通过站内链接进入）。
  pub group: Option<&'static str>,
  /// 是否依赖后端 `/api/*`：纯静态托管时该页会降级为「数据暂不可用」。
  pub backend: bool,
}

impl Module {
  /// 导航链接（无覆盖时等于 `path`）。
  #[must_use]
  pub fn nav_href(&self) -> &'static str {
    self.href.unwrap_or(self.path)
  }

  /// 所属一级模块；不在导航中返回 `None`。
  #[must_use]
  pub fn kind(&self) -> Option<ModuleKind> {
    self.group.map(kind_of)
  }

  /// 是否出现在顶部导航菜单。
  #[must_use]
  pub fn in_nav(&self) -> bool {
    self.group.is_some()
  }
}

/// 全部页面 / 工具（含不在导航菜单中的页面）。
pub const MODULES: &[Module] = &[
  Module {
    path: "/practice",
    href: None,
    title: "练习",
    icon: "clipboard-list",
    group: Some(GROUP_EXAM),
    backend: false,
  },
  Module {
    path: "/exam",
    href: None,
    title: "模拟考试",
    icon: "timer",
    group: Some(GROUP_EXAM),
    backend: false,
  },
  Module {
    path: "/daily-challenge",
    href: None,
    title: "每日挑战",
    icon: "flame",
    group: Some(GROUP_EXAM),
    backend: false,
  },
  Module {
    path: "/browse",
    href: None,
    title: "分类浏览",
    icon: "layout-grid",
    group: Some(GROUP_EXAM),
    backend: false,
  },
  Module {
    path: "/flashcards",
    href: None,
    title: "闪卡刷题",
    icon: "zap",
    group: Some(GROUP_EXAM),
    backend: false,
  },
  Module {
    path: "/listen",
    href: None,
    title: "听题模式",
    icon: "headphones",
    group: Some(GROUP_EXAM),
    backend: false,
  },
  Module {
    path: "/cards",
    href: None,
    title: "知识卡片",
    icon: "credit-card",
    group: Some(GROUP_EXAM),
    backend: false,
  },
  Module {
    path: "/mistakes",
    href: None,
    title: "错题集",
    icon: "list-x",
    group: Some(GROUP_EXAM),
    backend: false,
  },
  Module {
    path: "/mistake-topics",
    href: None,
    title: "易错知识点",
    icon: "flame",
    group: Some(GROUP_EXAM),
    backend: false,
  },
  Module {
    path: "/bookmarks",
    href: None,
    title: "收藏集",
    icon: "bookmark",
    group: Some(GROUP_EXAM),
    backend: false,
  },
  Module {
    path: "/weekly",
    href: None,
    title: "学习周报",
    icon: "activity",
    group: Some(GROUP_EXAM),
    backend: false,
  },
  Module {
    path: "/progress",
    href: None,
    title: "学习进度",
    icon: "trending-up",
    group: Some(GROUP_EXAM),
    backend: false,
  },
  Module {
    path: "/study-calendar",
    href: None,
    title: "备考日历",
    icon: "timer",
    group: Some(GROUP_EXAM),
    backend: false,
  },
  Module {
    path: "/exam-review",
    href: None,
    title: "考后复盘",
    icon: "clipboard-list",
    group: Some(GROUP_EXAM),
    backend: false,
  },
  Module {
    path: "/reference",
    href: None,
    title: "考试速查",
    icon: "book-marked",
    group: Some(GROUP_EXAM_REF),
    backend: false,
  },
  Module {
    path: "/cheat-sheet",
    href: None,
    title: "考点速查手册",
    icon: "clipboard-list",
    group: Some(GROUP_EXAM_REF),
    backend: false,
  },
  Module {
    path: "/confusables",
    href: None,
    title: "易混淆辨析",
    icon: "arrow-up-down",
    group: Some(GROUP_EXAM_REF),
    backend: false,
  },
  Module {
    path: "/formulas",
    href: None,
    title: "公式速查",
    icon: "calculator",
    group: Some(GROUP_EXAM_REF),
    backend: false,
  },
  Module {
    path: "/prefixes",
    href: None,
    title: "呼号前缀",
    icon: "globe",
    group: Some(GROUP_EXAM_REF),
    backend: false,
  },
  Module {
    path: "/glossary",
    href: None,
    title: "术语表",
    icon: "book-open",
    group: Some(GROUP_EXAM_REF),
    backend: false,
  },
  Module {
    path: "/q-code",
    href: None,
    title: "简语",
    icon: "radio",
    group: Some(GROUP_EXAM_REF),
    backend: false,
  },
  Module {
    path: "/phonetic",
    href: None,
    title: "字母解释法",
    icon: "case-upper",
    group: Some(GROUP_EXAM_REF),
    backend: false,
  },
  Module {
    path: "/rst",
    href: None,
    title: "RST 信号报告",
    icon: "signal",
    group: Some(GROUP_EXAM_REF),
    backend: false,
  },
  Module {
    path: "/morse",
    href: None,
    title: "莫尔斯电码",
    icon: "audio-lines",
    group: Some(GROUP_EXAM_REF),
    backend: false,
  },
  Module {
    path: "/cw-operating",
    href: None,
    title: "CW 操作（等幅波）",
    icon: "audio-lines",
    group: Some(GROUP_EXAM_REF),
    backend: false,
  },
  Module {
    path: "/license-classes",
    href: None,
    title: "操作证权限",
    icon: "badge-check",
    group: Some(GROUP_EXAM_REF),
    backend: false,
  },
  Module {
    path: "/analog-modes",
    href: None,
    title: "模拟模式",
    icon: "volume-2",
    group: Some(GROUP_MODES),
    backend: false,
  },
  Module {
    path: "/atv",
    href: None,
    title: "业余电视",
    icon: "tv",
    group: Some(GROUP_MODES),
    backend: false,
  },
  Module {
    path: "/modes",
    href: None,
    title: "数字模式",
    icon: "binary",
    group: Some(GROUP_MODES),
    backend: false,
  },
  Module {
    path: "/modulation",
    href: None,
    title: "调制理论",
    icon: "activity",
    group: Some(GROUP_MODES),
    backend: false,
  },
  Module {
    path: "/digital-comms",
    href: None,
    title: "数字通信原理",
    icon: "binary",
    group: Some(GROUP_MODES),
    backend: false,
  },
  Module {
    path: "/dsp",
    href: None,
    title: "数字信号处理基础",
    icon: "audio-lines",
    group: Some(GROUP_MODES),
    backend: false,
  },
  Module {
    path: "/dv-network",
    href: None,
    title: "数字语音组网",
    icon: "network",
    group: Some(GROUP_MODES),
    backend: false,
  },
  Module {
    path: "/rtty",
    href: None,
    title: "RTTY 无线电传 / PSK31",
    icon: "type",
    group: Some(GROUP_MODES),
    backend: false,
  },
  Module {
    path: "/ft8",
    href: None,
    title: "FT8 / FT4（数字模式）",
    icon: "binary",
    group: Some(GROUP_MODES),
    backend: false,
  },
  Module {
    path: "/sdr",
    href: None,
    title: "SDR 软件定义无线电",
    icon: "monitor",
    group: Some(GROUP_MODES),
    backend: false,
  },
  Module {
    path: "/gnuradio",
    href: None,
    title: "GNU Radio",
    icon: "workflow",
    group: Some(GROUP_MODES),
    backend: false,
  },
  Module {
    path: "/aprs",
    href: None,
    title: "APRS 自动位置报告",
    icon: "map",
    group: Some(GROUP_MODES),
    backend: false,
  },
  Module {
    path: "/frequencies",
    href: None,
    title: "常用频率",
    icon: "gauge",
    group: Some(GROUP_MODES),
    backend: false,
  },
  Module {
    path: "/coordination",
    href: None,
    title: "频率协调",
    icon: "scale",
    group: Some(GROUP_MODES),
    backend: false,
  },
  Module {
    path: "/zone-map",
    href: None,
    title: "CQ / ITU 分区地图",
    icon: "globe",
    group: Some(GROUP_MODES),
    backend: false,
  },
  Module {
    path: "/propagation",
    href: None,
    title: "传播与电离层",
    icon: "waves",
    group: Some(GROUP_MODES),
    backend: false,
  },
  Module {
    path: "/beacons",
    href: None,
    title: "国际信标网络",
    icon: "radio-tower",
    group: Some(GROUP_MODES),
    backend: false,
  },
  Module {
    path: "/special-prop",
    href: None,
    title: "特殊传播",
    icon: "sparkles",
    group: Some(GROUP_MODES),
    backend: false,
  },
  Module {
    path: "/meteor-scatter",
    href: None,
    title: "流星散射",
    icon: "waves",
    group: Some(GROUP_MODES),
    backend: false,
  },
  Module {
    path: "/aurora",
    href: None,
    title: "极光通信",
    icon: "sparkles",
    group: Some(GROUP_MODES),
    backend: false,
  },
  Module {
    path: "/eme",
    href: None,
    title: "EME 月面反射（地月地）",
    icon: "orbit",
    group: Some(GROUP_MODES),
    backend: false,
  },
  Module {
    path: "/muf",
    href: None,
    title: "传播预测",
    icon: "trending-up",
    group: Some(GROUP_MODES),
    backend: true,
  },
  Module {
    path: "/wspr",
    href: None,
    title: "WSPR 弱信号传播",
    icon: "waves",
    group: Some(GROUP_MODES),
    backend: false,
  },
  Module {
    path: "/sstv",
    href: None,
    title: "SSTV 慢扫描电视",
    icon: "camera",
    group: Some(GROUP_MODES),
    backend: false,
  },
  Module {
    path: "/weather-sat",
    href: None,
    title: "气象卫星接收",
    icon: "satellite",
    group: Some(GROUP_MODES),
    backend: false,
  },
  Module {
    path: "/packet",
    href: None,
    title: "Packet 分组无线电",
    icon: "network",
    group: Some(GROUP_MODES),
    backend: false,
  },
  Module {
    path: "/antennas",
    href: None,
    title: "天线型式",
    icon: "radio-tower",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/antenna-array",
    href: None,
    title: "天线阵列 / 相控阵",
    icon: "waypoints",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/polarization",
    href: None,
    title: "天线极化",
    icon: "arrow-up-down",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/feedline",
    href: None,
    title: "天线匹配与馈线",
    icon: "plug-zap",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/connectors",
    href: None,
    title: "线材与连接器",
    icon: "plug-zap",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/balun",
    href: None,
    title: "巴伦与不平衡变压器",
    icon: "waypoints",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/antenna-diy",
    href: None,
    title: "天线 DIY",
    icon: "wrench",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/practical-antennas",
    href: None,
    title: "实用天线专题",
    icon: "radio-tower",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/antenna-installation",
    href: None,
    title: "天线架设",
    icon: "hammer",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/antenna-farm",
    href: None,
    title: "天线农场",
    icon: "waypoints",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/antenna-tuning",
    href: None,
    title: "天线调试",
    icon: "wrench",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/antenna-analyzer",
    href: None,
    title: "天线分析仪",
    icon: "gauge",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/vna",
    href: None,
    title: "VNA 矢量网络分析仪",
    icon: "gauge",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/antenna-modeling",
    href: None,
    title: "天线建模",
    icon: "box",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/nvis",
    href: None,
    title: "NVIS 近垂直入射天波",
    icon: "cloud",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/electronics",
    href: None,
    title: "电子电路基础",
    icon: "cpu",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/filters",
    href: None,
    title: "滤波器与双工器",
    icon: "filter",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/meters",
    href: None,
    title: "测量仪表",
    icon: "activity",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/power",
    href: None,
    title: "电源与电池",
    icon: "battery-charging",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/power-supply",
    href: None,
    title: "电源供应",
    icon: "zap",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/transceiver",
    href: None,
    title: "收发信机",
    icon: "radio",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/receiver",
    href: None,
    title: "接收机指标",
    icon: "gauge",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/gear",
    href: None,
    title: "设备评测与选购",
    icon: "box",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/amplifier",
    href: None,
    title: "功率放大器",
    icon: "flame",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/bands",
    href: None,
    title: "波段表",
    icon: "satellite",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/bandplan",
    href: None,
    title: "波段规划",
    icon: "sliders-horizontal",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/microwave",
    href: None,
    title: "微波通信",
    icon: "radio",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/mobile",
    href: None,
    title: "车载 / 移动电台",
    icon: "radio-tower",
    group: Some(GROUP_ANTENNAS),
    backend: false,
  },
  Module {
    path: "/operating",
    href: None,
    title: "通联实务",
    icon: "messages-square",
    group: Some(GROUP_OPERATING),
    backend: false,
  },
  Module {
    path: "/sat-operation",
    href: None,
    title: "卫星通联操作",
    icon: "satellite",
    group: Some(GROUP_OPERATING),
    backend: false,
  },
  Module {
    path: "/contest",
    href: None,
    title: "通联竞赛",
    icon: "trophy",
    group: Some(GROUP_OPERATING),
    backend: false,
  },
  Module {
    path: "/cabrillo",
    href: None,
    title: "竞赛日志 Cabrillo",
    icon: "file-text",
    group: Some(GROUP_OPERATING),
    backend: false,
  },
  Module {
    path: "/awards",
    href: None,
    title: "DX 奖状",
    icon: "award",
    group: Some(GROUP_OPERATING),
    backend: false,
  },
  Module {
    path: "/iota",
    href: None,
    title: "IOTA 海岛通联（空中岛屿）",
    icon: "anchor",
    group: Some(GROUP_OPERATING),
    backend: false,
  },
  Module {
    path: "/dx",
    href: None,
    title: "DX 远距离通信技巧",
    icon: "globe",
    group: Some(GROUP_OPERATING),
    backend: false,
  },
  Module {
    path: "/dxpedition",
    href: None,
    title: "DX 远征",
    icon: "plane",
    group: Some(GROUP_OPERATING),
    backend: false,
  },
  Module {
    path: "/most-wanted",
    href: None,
    title: "DXCC 世纪俱乐部",
    icon: "star",
    group: Some(GROUP_OPERATING),
    backend: true,
  },
  Module {
    path: "/qrp",
    href: None,
    title: "QRP 低功率",
    icon: "battery-low",
    group: Some(GROUP_OPERATING),
    backend: false,
  },
  Module {
    path: "/eqsl",
    href: None,
    title: "电子 QSL",
    icon: "send",
    group: Some(GROUP_OPERATING),
    backend: false,
  },
  Module {
    path: "/qsl-card",
    href: None,
    title: "QSL 卡片设计",
    icon: "mail",
    group: Some(GROUP_OPERATING),
    backend: false,
  },
  Module {
    path: "/ardf",
    href: None,
    title: "无线电测向",
    icon: "compass",
    group: Some(GROUP_OPERATING),
    backend: false,
  },
  Module {
    path: "/emcomm",
    href: None,
    title: "应急通信",
    icon: "siren",
    group: Some(GROUP_OPERATING),
    backend: false,
  },
  Module {
    path: "/winlink",
    href: None,
    title: "Winlink 无线邮件",
    icon: "mail",
    group: Some(GROUP_OPERATING),
    backend: false,
  },
  Module {
    path: "/events",
    href: None,
    title: "活动日历",
    icon: "timer",
    group: Some(GROUP_OPERATING),
    backend: false,
  },
  Module {
    path: "/portable",
    href: None,
    title: "SOTA 山顶 / POTA 公园",
    icon: "mountain",
    group: Some(GROUP_OPERATING),
    backend: true,
  },
  Module {
    path: "/grid",
    href: None,
    title: "网格定位",
    icon: "map",
    group: Some(GROUP_OPERATING),
    backend: false,
  },
  Module {
    path: "/callsign",
    href: None,
    title: "呼号查询",
    icon: "search",
    group: Some(GROUP_OPERATING),
    backend: true,
  },
  Module {
    path: "/repeater",
    href: None,
    title: "中继台与网关",
    icon: "radio-tower",
    group: Some(GROUP_OPERATING),
    backend: true,
  },
  Module {
    path: "/repeater-build",
    href: None,
    title: "中继台建设",
    icon: "building-2",
    group: Some(GROUP_OPERATING),
    backend: false,
  },
  Module {
    path: "/logging-software",
    href: None,
    title: "日志与竞赛软件",
    icon: "clipboard-list",
    group: Some(GROUP_OPERATING),
    backend: false,
  },
  Module {
    path: "/organizations",
    href: None,
    title: "国际组织与分区",
    icon: "landmark",
    group: Some(GROUP_ADVANCED),
    backend: false,
  },
  Module {
    path: "/safety",
    href: None,
    title: "射频安全",
    icon: "shield-alert",
    group: Some(GROUP_ADVANCED),
    backend: false,
  },
  Module {
    path: "/grounding",
    href: None,
    title: "接地与防雷",
    icon: "zap",
    group: Some(GROUP_ADVANCED),
    backend: false,
  },
  Module {
    path: "/rfi",
    href: None,
    title: "射频干扰排查",
    icon: "shield-x",
    group: Some(GROUP_ADVANCED),
    backend: false,
  },
  Module {
    path: "/noise",
    href: None,
    title: "接收环境与噪声",
    icon: "waves",
    group: Some(GROUP_ADVANCED),
    backend: false,
  },
  Module {
    path: "/beginner",
    href: None,
    title: "新手入门",
    icon: "graduation-cap",
    group: Some(GROUP_ADVANCED),
    backend: false,
  },
  Module {
    path: "/learning-path",
    href: None,
    title: "学习路径",
    icon: "waypoints",
    group: Some(GROUP_ADVANCED),
    backend: false,
  },
  Module {
    path: "/learning-resources",
    href: None,
    title: "学习资源",
    icon: "graduation-cap",
    group: Some(GROUP_ADVANCED),
    backend: false,
  },
  Module {
    path: "/swl",
    href: None,
    title: "SWL 短波监听",
    icon: "headphones",
    group: Some(GROUP_ADVANCED),
    backend: false,
  },
  Module {
    path: "/license",
    href: None,
    title: "执照申办",
    icon: "badge-check",
    group: Some(GROUP_ADVANCED),
    backend: false,
  },
  Module {
    path: "/regulations",
    href: None,
    title: "法规与管理",
    icon: "scale",
    group: Some(GROUP_ADVANCED),
    backend: false,
  },
  Module {
    path: "/history",
    href: None,
    title: "业余无线电历史",
    icon: "history",
    group: Some(GROUP_ADVANCED),
    backend: false,
  },
  Module {
    path: "/remote",
    href: None,
    title: "远程电台",
    icon: "globe",
    group: Some(GROUP_ADVANCED),
    backend: false,
  },
  Module {
    path: "/open-source",
    href: None,
    title: "开源项目与 DIY",
    icon: "workflow",
    group: Some(GROUP_ADVANCED),
    backend: false,
  },
  Module {
    path: "/community",
    href: None,
    title: "火腿社区",
    icon: "messages-square",
    group: Some(GROUP_ADVANCED),
    backend: false,
  },
  Module {
    path: "/diy-projects",
    href: None,
    title: "DIY 实战项目",
    icon: "wrench",
    group: Some(GROUP_ADVANCED),
    backend: false,
  },
  Module {
    path: "/developers",
    href: None,
    title: "开放 API",
    icon: "network",
    group: Some(GROUP_ADVANCED),
    // 文档页本身是纯静态，不依赖后端；接口在无后端时不可用，页面已说明。
    backend: false,
  },
  Module {
    path: "/tools",
    href: None,
    title: "小工具",
    icon: "calculator",
    group: Some(GROUP_TOOL_CALC),
    backend: false,
  },
  Module {
    path: "/nec",
    href: None,
    title: "天线矩量法求解",
    icon: "activity",
    group: Some(GROUP_TOOL_CALC),
    backend: false,
  },
  Module {
    path: "/waveform-lab",
    href: None,
    title: "波形实验室",
    icon: "activity",
    group: Some(GROUP_TOOL_CALC),
    backend: false,
  },
  Module {
    path: "/smith",
    href: None,
    title: "史密斯圆图与匹配",
    icon: "orbit",
    group: Some(GROUP_TOOL_CALC),
    backend: false,
  },
  Module {
    path: "/apt-decoder",
    href: None,
    title: "APT 云图解码",
    icon: "satellite",
    group: Some(GROUP_TOOL_CALC),
    backend: false,
  },
  Module {
    path: "/sstv-decode",
    href: None,
    title: "SSTV 解码器",
    icon: "camera",
    group: Some(GROUP_TOOL_CALC),
    backend: false,
  },
  Module {
    path: "/wspr-decode",
    href: None,
    title: "WSPR 解码器",
    icon: "waves",
    group: Some(GROUP_TOOL_CALC),
    backend: false,
  },
  Module {
    path: "/sdr-waterfall",
    href: None,
    title: "SDR 瀑布图",
    icon: "audio-lines",
    group: Some(GROUP_TOOL_CALC),
    backend: false,
  },
  Module {
    path: "/psk-decode",
    href: None,
    title: "PSK31 解码",
    icon: "audio-lines",
    group: Some(GROUP_TOOL_CALC),
    backend: false,
  },
  Module {
    path: "/photo-processor",
    href: None,
    title: "照片处理",
    icon: "camera",
    group: Some(GROUP_TOOL_CALC),
    backend: false,
  },
  Module {
    path: "/log",
    href: None,
    title: "通联日志",
    icon: "clipboard-list",
    group: Some(GROUP_TOOL_LOG),
    backend: true,
  },
  Module {
    path: "/qsl-labels",
    href: None,
    title: "QSL 标签打印",
    icon: "mail",
    group: Some(GROUP_TOOL_LOG),
    backend: false,
  },
  Module {
    path: "/qsl-designer",
    href: None,
    title: "QSL 卡片设计",
    icon: "credit-card",
    group: Some(GROUP_TOOL_LOG),
    backend: false,
  },
  Module {
    path: "/contest-log",
    href: None,
    title: "竞赛录入",
    icon: "trophy",
    group: Some(GROUP_TOOL_LOG),
    backend: false,
  },
  Module {
    path: "/contest-calendar",
    href: None,
    title: "竞赛日历",
    icon: "trophy",
    group: Some(GROUP_TOOL_LOG),
    backend: false,
  },
  Module {
    path: "/stats",
    href: None,
    title: "通联统计",
    icon: "trending-up",
    group: Some(GROUP_TOOL_LOG),
    backend: false,
  },
  Module {
    path: "/dashboard",
    href: None,
    title: "实时仪表盘",
    icon: "layout-grid",
    group: Some(GROUP_TOOL_LIVE),
    backend: true,
  },
  Module {
    path: "/dx-spots",
    href: None,
    title: "DX 实时热点",
    icon: "radio",
    group: Some(GROUP_TOOL_LIVE),
    backend: true,
  },
  Module {
    path: "/psk-reporter",
    href: None,
    title: "PSK Reporter",
    icon: "radio",
    group: Some(GROUP_TOOL_LIVE),
    backend: true,
  },
  Module {
    path: "/rbn",
    href: None,
    title: "RBN 信标网络",
    icon: "radio-tower",
    group: Some(GROUP_TOOL_LIVE),
    backend: true,
  },
  Module {
    path: "/solar",
    href: None,
    title: "太阳活动",
    icon: "sun-medium",
    group: Some(GROUP_TOOL_LIVE),
    backend: true,
  },
  Module {
    path: "/grid-map",
    href: None,
    title: "网格地图",
    icon: "map",
    group: Some(GROUP_TOOL_MAP),
    backend: true,
  },
  Module {
    path: "/grayline",
    href: None,
    title: "灰线地图",
    icon: "sun",
    group: Some(GROUP_TOOL_MAP),
    backend: false,
  },
  Module {
    path: "/dxcc-map",
    href: None,
    title: "DXCC 世界地图",
    icon: "globe",
    group: Some(GROUP_TOOL_MAP),
    backend: false,
  },
  Module {
    path: "/portable-map",
    href: None,
    title: "SOTA / POTA 地图",
    icon: "mountain",
    group: Some(GROUP_TOOL_MAP),
    backend: true,
  },
  Module {
    path: "/satellites",
    href: None,
    title: "业余卫星",
    icon: "orbit",
    group: Some(GROUP_TOOL_MAP),
    backend: true,
  },
  Module {
    path: "/sdr-map",
    href: None,
    title: "在线 SDR 接收站",
    icon: "map",
    group: Some(GROUP_TOOL_MAP),
    backend: false,
  },
  Module {
    path: "/callsign-copy",
    href: None,
    title: "呼号抄收",
    icon: "headphones",
    group: Some(GROUP_TOOL_TRAIN),
    backend: false,
  },
  Module {
    path: "/countdown",
    href: None,
    title: "倒计时",
    icon: "timer",
    group: Some(GROUP_TOOL_TRAIN),
    backend: false,
  },
  Module {
    path: "/notifications",
    href: None,
    title: "通知中心",
    icon: "siren",
    group: Some(GROUP_TOOL_TRAIN),
    backend: false,
  },
  Module {
    path: "/print",
    href: None,
    title: "打印版",
    icon: "file-text",
    group: Some(GROUP_TOOL_TRAIN),
    backend: false,
  },
  Module {
    path: "/",
    href: None,
    title: "首页",
    icon: "house",
    group: None,
    backend: true,
  },
  Module {
    path: "/achievements",
    href: None,
    title: "成就墙",
    icon: "award",
    group: None,
    backend: false,
  },
  Module {
    path: "/report",
    href: None,
    title: "学习报告",
    icon: "file-text",
    group: None,
    backend: false,
  },
];

/// 按路径查找（同名路径返回第一个，如 `/practice` 的两个入口）。
#[must_use]
pub fn by_path(path: &str) -> Option<&'static Module> {
  MODULES.iter().find(|m| m.path == path)
}

/// 某个分组下的模块，保持注册顺序。
#[must_use]
pub fn in_group(group: &str) -> Vec<&'static Module> {
  MODULES.iter().filter(|m| m.group == Some(group)).collect()
}

/// 考试中心菜单项（平铺，无子标题）。
#[must_use]
pub fn exam_items() -> Vec<&'static Module> {
  in_group(GROUP_EXAM)
}

/// 导航中出现的全部路径（按注册顺序去重）。
#[must_use]
pub fn nav_paths() -> Vec<&'static str> {
  let mut out: Vec<&'static str> = Vec::new();
  for m in MODULES.iter().filter(|m| m.in_nav()) {
    if !out.contains(&m.path) {
      out.push(m.path);
    }
  }
  out
}

/// 不进 sitemap 的路径：`/print` 依赖 `?src=` 参数，单独访问没有意义。
const SITEMAP_EXCLUDE: &[&str] = &["/print"];

/// 实时数据类页面：更新频率更高，sitemap 里给更高权重。
const SITEMAP_LIVE: &[&str] = &["/dashboard", "/dx-spots", "/solar", "/rbn", "/psk-reporter"];

/// `sitemap.xml` 的条目：`(路径, changefreq, priority)`，按路径去重且有序。
///
/// 权重按分组给策略，避免逐页手工维护一份 140 条、又总会过期的清单
/// （此前 `postbuild.rs` 里的手写列表就漏掉了大批新页面）。
#[must_use]
pub fn sitemap_entries() -> Vec<(&'static str, &'static str, &'static str)> {
  let mut seen: Vec<&'static str> = Vec::new();
  let mut out = Vec::new();
  for m in MODULES {
    if SITEMAP_EXCLUDE.contains(&m.path) || seen.contains(&m.path) {
      continue;
    }
    seen.push(m.path);
    let meta = if m.path == "/" {
      ("weekly", "1.0")
    } else if SITEMAP_LIVE.contains(&m.path) {
      ("weekly", "0.8")
    } else if m.backend {
      ("monthly", "0.7")
    } else if m.in_nav() {
      ("monthly", "0.8")
    } else {
      ("monthly", "0.6")
    };
    out.push((m.path, meta.0, meta.1));
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn every_module_is_well_formed() {
    for m in MODULES {
      assert!(m.path.starts_with('/'), "路径必须以 / 开头：{}", m.path);
      assert!(!m.title.is_empty(), "{} 缺少标题", m.path);
      assert!(!m.icon.is_empty(), "{} 缺少图标", m.path);
      if let Some(g) = m.group {
        assert!(
          g == GROUP_EXAM || KNOWLEDGE_GROUPS.contains(&g) || TOOL_GROUPS.contains(&g),
          "{} 的分组 {g} 未在分组列表中登记",
          m.path
        );
      }
    }
  }

  #[test]
  fn nav_paths_are_unique() {
    let paths = nav_paths();
    let mut sorted = paths.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), paths.len(), "导航路径应已去重");
  }

  #[test]
  fn every_group_has_modules_and_an_icon() {
    for g in KNOWLEDGE_GROUPS.iter().chain(TOOL_GROUPS.iter()) {
      assert!(!in_group(g).is_empty(), "分组 {g} 没有任何模块");
      assert_ne!(group_icon(g), "circle", "分组 {g} 缺少图标映射");
      assert_eq!(
        kind_of(g),
        if TOOL_GROUPS.contains(g) {
          ModuleKind::Tool
        } else {
          ModuleKind::Knowledge
        }
      );
    }
    assert!(!exam_items().is_empty());
    assert_eq!(kind_of(GROUP_EXAM), ModuleKind::Exam);
  }

  #[test]
  fn sitemap_is_deduped_and_excludes_param_only_pages() {
    let entries = sitemap_entries();
    let mut seen: Vec<&str> = entries.iter().map(|(p, _, _)| *p).collect();
    let total = seen.len();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), total, "sitemap 不应出现重复路径");
    assert!(!entries.iter().any(|(p, _, _)| *p == "/print"));
    assert!(entries.iter().any(|(p, _, _)| *p == "/"));
    // 除排除项外，每个不同路径都应进 sitemap（`/practice` 有两个入口但只算一条）。
    let distinct: Vec<&str> = {
      let mut v: Vec<&str> = MODULES.iter().map(|m| m.path).collect();
      v.sort_unstable();
      v.dedup();
      v
    };
    assert_eq!(total, distinct.len() - 1);
  }

  #[test]
  fn by_path_and_in_group_work() {
    assert_eq!(by_path("/morse").map(|m| m.title), Some("莫尔斯电码"));
    assert_eq!(by_path("/nope"), None);
    assert!(in_group(GROUP_MODES).iter().any(|m| m.path == "/ft8"));
    assert!(in_group("不存在的分组").is_empty());
  }
}
