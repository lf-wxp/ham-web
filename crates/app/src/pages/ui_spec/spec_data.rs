//! 展示页的静态数据表与纯函数。
//!
//! 一文件一组件：这里只放常量表和不返回视图的辅助函数；所有视图都在各自的组件文件里。
//! 表里的 `title` / `desc` / `key` 字段是 i18n 语义 key，`value` / `var` 字段是代码与取值，
//! 不是界面文案 —— 后者不进词典，源码里也就不会出现没有译文的中文。

use crate::icons::IconKind;
use crate::util::{document, window};

/// 原则卡片的筛选维度。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Dimension {
  Geometry,
  Color,
  Motion,
  Type,
  A11y,
}

impl Dimension {
  pub const fn slug(self) -> &'static str {
    match self {
      Self::Geometry => "geometry",
      Self::Color => "color",
      Self::Motion => "motion",
      Self::Type => "type",
      Self::A11y => "a11y",
    }
  }
}

/// 筛选器的取值与词条 key。
pub const FILTERS: &[(&str, &str)] = &[
  ("all", "design.filter-all"),
  ("geometry", "design.filter-geometry"),
  ("color", "design.filter-color"),
  ("motion", "design.filter-motion"),
  ("type", "design.filter-type"),
  ("a11y", "design.filter-a11y"),
];

/// 一条设计原则：标题、说明、可复制的取值，以及它在速查表里对应的行。
pub struct Principle {
  pub icon: IconKind,
  pub title: &'static str,
  pub desc: &'static str,
  pub value: &'static str,
  pub group: Dimension,
  pub spec: &'static str,
}

pub const PRINCIPLES: &[Principle] = &[
  Principle {
    icon: IconKind::Box,
    title: "design.principle-corners",
    desc: "design.principle-corners-desc",
    value: "radius: 0 · shadow: hard",
    group: Dimension::Geometry,
    spec: "design.spec-radius",
  },
  Principle {
    icon: IconKind::LayoutGrid,
    title: "design.principle-grid",
    desc: "design.principle-grid-desc",
    value: "24 / 32 / 40 / 48px",
    group: Dimension::Geometry,
    spec: "design.spec-breakpoints",
  },
  Principle {
    icon: IconKind::Sparkles,
    title: "design.principle-icon",
    desc: "design.principle-icon-desc",
    value: "24 × 24 · crispEdges",
    group: Dimension::Geometry,
    spec: "design.spec-breakpoints",
  },
  Principle {
    icon: IconKind::Type,
    title: "design.principle-weight",
    desc: "design.principle-weight-desc",
    value: "font-weight: 400",
    group: Dimension::Type,
    spec: "design.spec-scale",
  },
  Principle {
    icon: IconKind::Box,
    title: "design.principle-shadow",
    desc: "design.principle-shadow-desc",
    value: "2px 2px 0 0",
    group: Dimension::Geometry,
    spec: "design.spec-radius",
  },
  Principle {
    icon: IconKind::Zap,
    title: "design.principle-steps",
    desc: "design.principle-steps-desc",
    value: "steps(4, end)",
    group: Dimension::Motion,
    spec: "design.spec-duration",
  },
  Principle {
    icon: IconKind::Circle,
    title: "design.principle-token",
    desc: "design.principle-token-desc",
    value: "var(--primary)",
    group: Dimension::Color,
    spec: "design.spec-scheme",
  },
  Principle {
    icon: IconKind::ShieldAlert,
    title: "design.principle-aria",
    desc: "design.principle-aria-desc",
    value: "aria-invalid",
    group: Dimension::A11y,
    spec: "design.spec-border",
  },
];

/// 规格速查表的一行。
pub struct SpecRow {
  pub key: &'static str,
  pub value: &'static str,
}

pub const SPECS: &[SpecRow] = &[
  SpecRow {
    key: "design.spec-page-max",
    value: "1200px · --page-max",
  },
  SpecRow {
    key: "design.spec-breakpoints",
    value: "400 / 640 / 720 / 900 / 1024",
  },
  SpecRow {
    key: "design.spec-radius",
    value: "0px · --radius-*",
  },
  SpecRow {
    key: "design.spec-border",
    value: "2px solid var(--pxl-ink)",
  },
  SpecRow {
    key: "design.spec-scale",
    value: "12 / 18 / 24 / 30 / 36 / 48 / 60",
  },
  SpecRow {
    key: "design.spec-duration",
    value: "80 / 120 / 240 / 480ms",
  },
  SpecRow {
    key: "design.spec-z",
    value: "40 / 50 / 55 / 60 / 70 / 75 / 80",
  },
  SpecRow {
    key: "design.spec-scheme",
    value: "11 × data-scheme",
  },
];

/// 一组色彩 token。`tokens` 里是完整的 CSS 变量名。
pub struct TokenGroup {
  pub key: &'static str,
  pub tokens: &'static [&'static str],
}

pub const TOKEN_GROUPS: &[TokenGroup] = &[
  TokenGroup {
    key: "design.colors-group-surface",
    tokens: &[
      "--background",
      "--foreground",
      "--card",
      "--popover",
      "--primary",
      "--secondary",
      "--muted",
      "--accent",
      "--destructive",
      "--border",
      "--input",
      "--ring",
      "--pxl-ink",
      "--pxl-field",
    ],
  },
  TokenGroup {
    key: "design.colors-group-state",
    tokens: &[
      "--pxl-hp",
      "--pxl-hp-hi",
      "--pxl-xp",
      "--pxl-xp-hi",
      "--pxl-gold",
      "--pxl-win",
      "--pxl-win-hi",
    ],
  },
  TokenGroup {
    key: "design.colors-group-text",
    tokens: &[
      "--pxl-hp-text",
      "--pxl-xp-text",
      "--pxl-gold-text",
      "--pxl-win-text",
    ],
  },
  TokenGroup {
    key: "design.colors-group-category",
    tokens: &[
      "--cat-1", "--cat-2", "--cat-3", "--cat-4", "--cat-5", "--cat-6", "--cat-7", "--cat-8",
    ],
  },
  TokenGroup {
    key: "design.colors-group-display",
    tokens: &[
      "--display-bg",
      "--display-fg",
      "--display-dim",
      "--display-hot",
      "--display-alarm",
      "--display-ok",
      "--display-edge",
    ],
  },
  TokenGroup {
    key: "design.colors-group-led",
    tokens: &[
      "--led-rx",
      "--led-tx",
      "--led-warn",
      "--led-info",
      "--led-off",
    ],
  },
];

/// 所有色板的变量名，顺序与 `TOKEN_GROUPS` 展开后一致（供计算样式批量读取）。
pub fn all_tokens() -> Vec<&'static str> {
  TOKEN_GROUPS
    .iter()
    .flat_map(|g| g.tokens.iter().copied())
    .collect()
}

/// 字阶的一级：`var` 是字号变量，`line` 是配套的行高变量，`px` 是展示用的标称值。
pub struct TypeStep {
  pub name: &'static str,
  pub var: &'static str,
  pub line: &'static str,
  pub line_value: &'static str,
  pub px: &'static str,
}

pub const TYPE_STEPS: &[TypeStep] = &[
  TypeStep {
    name: "xs",
    var: "--text-xs",
    line: "--text-xs--line-height",
    line_value: "1.5",
    px: "12px",
  },
  TypeStep {
    name: "base",
    var: "--text-base",
    line: "--text-base--line-height",
    line_value: "1.5",
    px: "18px",
  },
  TypeStep {
    name: "lg",
    var: "--text-lg",
    line: "--text-lg--line-height",
    line_value: "1.375",
    px: "24px",
  },
  TypeStep {
    name: "2xl",
    var: "--text-2xl",
    line: "--text-2xl--line-height",
    line_value: "1.25",
    px: "30px",
  },
  TypeStep {
    name: "3xl",
    var: "--text-3xl",
    line: "--text-3xl--line-height",
    line_value: "1.2",
    px: "36px",
  },
  TypeStep {
    name: "4xl",
    var: "--text-4xl",
    line: "--text-4xl--line-height",
    line_value: "1.1",
    px: "48px",
  },
  TypeStep {
    name: "5xl",
    var: "--text-5xl",
    line: "--text-5xl--line-height",
    line_value: "1",
    px: "60px",
  },
];

/// 动效时长 token。
pub struct MotionToken {
  pub key: &'static str,
  pub var: &'static str,
  pub value: &'static str,
}

pub const MOTION_TOKENS: &[MotionToken] = &[
  MotionToken {
    key: "design.motion-instant",
    var: "--motion-instant",
    value: "0.08s",
  },
  MotionToken {
    key: "design.motion-fast",
    var: "--motion-fast",
    value: "0.12s",
  },
  MotionToken {
    key: "design.motion-normal",
    var: "--motion-normal",
    value: "0.24s",
  },
  MotionToken {
    key: "design.motion-slow",
    var: "--motion-slow",
    value: "0.48s",
  },
];

/// 状态试验台的档位。
pub struct StateDef {
  pub slug: &'static str,
  pub key: &'static str,
}

pub const STATES: &[StateDef] = &[
  StateDef {
    slug: "normal",
    key: "design.states-normal",
  },
  StateDef {
    slug: "hover",
    key: "design.states-hover",
  },
  StateDef {
    slug: "focus",
    key: "design.states-focus",
  },
  StateDef {
    slug: "active",
    key: "design.states-active",
  },
  StateDef {
    slug: "disabled",
    key: "design.states-disabled",
  },
  StateDef {
    slug: "loading",
    key: "design.states-loading",
  },
  StateDef {
    slug: "error",
    key: "design.states-error",
  },
  StateDef {
    slug: "readonly",
    key: "design.states-readonly",
  },
];

/// 精灵展示用的名字（取自 `icons::pixel::sprite_data::sprite_by_name` 的清单）。
pub const SPRITES: &[&str] = &[
  "hero",
  "boss",
  "mon_static",
  "mon_swr",
  "mon_noise",
  "badge_star",
  "badge_trophy",
  "node_flag",
  "node_heart",
  "node_coin",
];

/// 方案 id → 已有的 `shell.scheme-*` 词条 key。
pub fn scheme_key(id: &str) -> &'static str {
  match id {
    "forest" => "shell.scheme-forest",
    "ocean" => "shell.scheme-ocean",
    "sunset" => "shell.scheme-sunset",
    "graphite" => "shell.scheme-graphite",
    "pico" => "shell.scheme-pico",
    "gameboy" => "shell.scheme-gameboy",
    "nes" => "shell.scheme-nes",
    "commodore" => "shell.scheme-commodore",
    "cga" => "shell.scheme-cga",
    "spectrum" => "shell.scheme-spectrum",
    _ => "shell.scheme-classic",
  }
}

/// 「通用组件」板块的子标签。
pub const COMPONENT_GROUPS: &[(&str, &str)] = &[
  ("basics", "design.comp-tab-basics"),
  ("advanced", "design.comp-tab-advanced"),
  ("data", "design.comp-tab-data"),
  ("overlays", "design.comp-tab-overlays"),
  ("radio", "design.comp-tab-radio"),
  ("placeholders", "design.comp-tab-placeholders"),
];

/// 组件总览：一族组件的名字清单，全是代码标识符，不进词典。
pub struct ComponentFamily {
  pub key: &'static str,
  pub items: &'static [&'static str],
}

pub const COMPONENT_FAMILIES: &[ComponentFamily] = &[
  ComponentFamily {
    key: "design.index-actions",
    items: &[
      "Button",
      "ButtonLink",
      "ButtonKind",
      "Chip",
      "ChipGroup",
      "ChipToggle",
      "Variant",
      "Size",
      "Separator",
    ],
  },
  ComponentFamily {
    key: "design.index-forms",
    items: &[
      "Input",
      "InputType",
      "Textarea",
      "NumberField",
      "NumberKind",
      "Checkbox",
      "Switch",
      "RadioGroup",
      "RadioGroupItem",
      "Slider",
      "Knob",
      "SimpleSelect",
      "SelectOption",
      "Select",
      "SelectGroup",
      "SelectItem",
      "DatePicker",
      "TimePicker",
      "FileInput",
      "FileDropZone",
      "Field",
      "Label",
      "ValidateOn",
      "ControlSize",
      "TextValue",
    ],
  },
  ComponentFamily {
    key: "design.index-overlays",
    items: &[
      "Dialog",
      "DialogHeader",
      "DialogTitle",
      "DialogDescription",
      "DialogFooter",
      "Sheet",
      "Tooltip",
      "ToastRoot",
      "ToastKind",
      "ConfirmRoot",
      "ConfirmTone",
      "MessageDialog",
      "MobileNavDrawer",
    ],
  },
  ComponentFamily {
    key: "design.index-data",
    items: &[
      "Table",
      "TableHead",
      "TableBody",
      "TableRow",
      "TableHeader",
      "TableCell",
      "TableEmpty",
      "ScrollRegion",
      "ScrollBar",
      "SortOrder",
      "TableDensity",
      "Pagination",
      "Progress",
      "Stat",
      "Tabs",
      "TabsList",
      "TabsTrigger",
      "TabsPanel",
    ],
  },
  ComponentFamily {
    key: "design.index-states",
    items: &[
      "EmptyState",
      "ErrorState",
      "Loading",
      "LoadingBlock",
      "Skeleton",
      "ResultCount",
      "LoadMore",
    ],
  },
  ComponentFamily {
    key: "design.index-radio",
    items: &[
      "Readout",
      "ReadoutTone",
      "Led",
      "LedTone",
      "LedState",
      "SignalMeter",
      "SignalBars",
      "Battery",
      "QrmSignature",
      "TuningScale",
      "PixelSprite",
    ],
  },
];

/// 七段读色的五档，与 `ui::ReadoutTone` 一一对应。
pub const READOUT_TONES: &[(&str, &str)] = &[
  ("normal", "Normal"),
  ("dim", "Dim"),
  ("hot", "Hot"),
  ("alarm", "Alarm"),
  ("ok", "Ok"),
];

/// 屏幕层四档字色，与 `radio.css` 的 `pxl-d-*` 一一对应。
pub const SCREEN_TONES: &[(&str, &str)] = &[
  ("pxl-d-dim", "DIM"),
  ("pxl-d-hot", "HOT"),
  ("pxl-d-alarm", "ALARM"),
  ("pxl-d-ok", "OK"),
];

/// 滑块回调的 `f64` 收窄成 `u8`：先四舍五入再夹到 `0..=max`，范围以后改了也不会静默截断。
pub fn slider_u8(value: f64, max: u8) -> u8 {
  // 夹紧后一定落在 0..=255 内，`as` 不会丢信息。
  value.round().clamp(0.0, f64::from(max)) as u8
}

/// 读 `<html>` 上某个 CSS 自定义属性当前真正生效的值。
///
/// 两套皮肤的规范页都要展示「运行时计算值」，实现只留这一份；读不到时返回 `-`，
/// 让页面显示一个明确的占位而不是空白。
#[must_use]
pub(crate) fn computed(name: &str) -> String {
  let Some(root) = document().document_element() else {
    return String::from("-");
  };
  let Ok(Some(style)) = window().get_computed_style(&root) else {
    return String::from("-");
  };
  let value = style.get_property_value(name).unwrap_or_default();
  let value = value.trim();
  if value.is_empty() {
    String::from("-")
  } else {
    value.to_owned()
  }
}

/// 只在像素风生效的**展示性**类名（标签 / 标题 / 读数屏外壳），不是控件本体。
///
/// 两套皮肤的规范页要展示同一批共享控件实样，所以这些外围容器按皮肤换类名；
/// 现代侧留空时由共享语义工具类 + 变量层接管，不会把 `pxl-*` 漏进现代 DOM。
#[must_use]
pub(crate) fn pixel_only(
  pixel_class: &'static str,
  modern_class: &'static str,
  extra: &str,
) -> String {
  crate::cn::cn(&[
    if crate::kit::is_modern_at_mount() {
      modern_class
    } else {
      pixel_class
    },
    extra,
  ])
}
