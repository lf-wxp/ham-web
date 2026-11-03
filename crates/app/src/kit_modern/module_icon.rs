//! 页面入口的图标推断：先按**路由路径**匹配，再落回分组图标。
//!
//! 为什么不用中文标题匹配：标题是多语言文案（`crates/app` 里的中文字面量必须能查到词条，
//! 否则 en / es 界面会漏中文），而路径是语言无关的稳定标识 —— 用路径判断既不会被翻译
//! 影响，也不会触发 i18n 词典检查。关键词表全部是 ASCII。
//!
//! 匹配按「词首」而不是子串（`registry::path_has_word`）：子串会让 `/kiss-tnc` 命中 `iss`、
//! `/analog-modes` 命中 `log`。`frequen` 是有意截断的词干，同时覆盖 `frequency` / `frequencies`。

use ham_web_core::registry::{self, Module};

use super::MIcon;

/// 路径关键词 → 图标。**按顺序匹配**，越具体的规则放越前面
/// （`/satellites/map` 要命中卫星而不是地图）。
const PATH_ICONS: &[(&[&str], MIcon)] = &[
  (
    &["satellite", "eme", "moon", "space", "iss", "aurora"],
    MIcon::Satellite,
  ),
  (
    &[
      "antenna", "balun", "yagi", "loop", "tuner", "feedline", "swr",
    ],
    MIcon::Antenna,
  ),
  (
    &["formula", "calculator", "convert", "units", "ohm", "dbm"],
    MIcon::Calculator,
  ),
  (
    &["map", "grid", "dxcc", "grayline", "coverage", "iota"],
    MIcon::Map,
  ),
  (
    &[
      "decoder", "decode", "aprs", "ft8", "rtty", "psk", "morse", "sstv", "encode",
    ],
    MIcon::Code,
  ),
  (
    &["log", "contest", "qsl", "award", "cabrillo", "lotw"],
    MIcon::Trophy,
  ),
  (&["exam", "mock", "paper", "countdown"], MIcon::Clock),
  (
    &["mistake", "review", "flashcard", "cards", "bookmark"],
    MIcon::Refresh,
  ),
  (
    &[
      "daily",
      "challenge",
      "battle",
      "bestiary",
      "quest",
      "achievement",
    ],
    MIcon::Flame,
  ),
  (
    &["progress", "stats", "report", "dashboard", "heatmap"],
    MIcon::Chart,
  ),
  (
    &["band", "frequen", "propagation", "solar", "spectrum", "muf"],
    MIcon::Signal,
  ),
  (
    &["listen", "phonetic", "voice", "qso", "operating"],
    MIcon::Volume,
  ),
  (
    &["measure", "vna", "analyzer", "calibration", "meter"],
    MIcon::Gauge,
  ),
  (
    &["law", "regulation", "license", "compliance", "notice"],
    MIcon::Star,
  ),
  (&["backup", "settings", "print", "config"], MIcon::Settings),
];

/// 把 `registry::group_icon` 的 Lucide 名称映射到现代图标集。
fn group_micon(group: Option<&str>) -> MIcon {
  match group.map(registry::group_icon) {
    Some("play") => MIcon::Play,
    Some("timer") => MIcon::Clock,
    Some("refresh-cw") => MIcon::Refresh,
    Some("trending-up" | "activity") => MIcon::Chart,
    Some("map") => MIcon::Map,
    Some("binary") => MIcon::Code,
    Some("waves" | "radio-tower") => MIcon::Signal,
    Some("gauge") => MIcon::Gauge,
    Some("messages-square" | "headphones") => MIcon::Volume,
    Some("landmark") => MIcon::Star,
    Some("calculator") => MIcon::Tools,
    Some("trophy") => MIcon::Trophy,
    Some("book-marked") => MIcon::Book,
    _ => MIcon::Book,
  }
}

/// 一条页面入口的图标。
#[must_use]
pub fn module_icon(m: &Module) -> MIcon {
  let href = m.nav_href();
  for (keys, icon) in PATH_ICONS {
    if keys.iter().any(|k| registry::path_has_word(href, k)) {
      return *icon;
    }
  }
  group_micon(m.group)
}
