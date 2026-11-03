//! 交互签名与换页方向：路由 → 动作族的**唯一映射**。
//!
//! 全站 180 条路由，如果让每个页面自己挑动效，最后一定会漂移成「有的页有动效、有的页没有」。
//! 这里把「这条路由用哪一种动作」写成一张可测的表，页面与样式层都只消费结果：
//!
//! - 前端在路由变化时把 [`PageSignature::attr`] 写到 `#route-content` 的 `data-sig` 上；
//! - `style/modern/interaction.css` 只按 `[data-sig="…"]` 分支，不再逐页写动效；
//! - [`nav_direction`] 给出「前进 / 后退 / 平级」三种换页方向，让同一条路由动画带上空间感。
//!
//! 放在 core 而不是 `crates/app`：这张表必须能跑单测（覆盖率、唯一性、拼写），
//! 而且换页方向是纯函数，不该依赖浏览器。

/// 一个页面的交互签名（动作族）。
///
/// 签名只决定「动作」，不决定颜色、圆角、字号 —— 那三样由设计 token 统一。
/// 因此新增页面时挑一个签名即可，不会长出第二套设计语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageSignature {
  /// 首页：编排式落地（视差首屏、横向轨道、数字起跳）。
  Landing,
  /// 练题型：竖向翻题、跟手拖拽、选项「掐一下」。
  Practice,
  /// 考试型：横向推题、答题卡弹跳、倒计时呼吸。
  Exam,
  /// 检索/列表型：行标尺、筛选条吸附、键盘游走。
  Browse,
  /// 速查/索引型：字母与序号索引、条目悬停提亮。
  Index,
  /// 计算器/生成器型：目录落点、面板闪环、结果数字滚动。
  Tools,
  /// 解码器/波形型：扫描带、频谱呼吸、运行脉冲。
  Lab,
  /// 地图/网格型：定位脉冲、标记悬浮、拖拽平移。
  Map,
  /// 日志/时间型：时间轴脊柱随滚动生长、行展开。
  Log,
  /// 长文阅读型：页头短线、段落悬挂、链接下划线擦出。
  Read,
  /// 数据看板型：数字滚动、进度条增长、卡片上抬。
  Board,
  /// 兜底：只保留全站基础手感（淡入、按压、声呐点）。
  Quiet,
}

impl PageSignature {
  /// 全部签名，顺序即文档里的表格顺序。
  pub const ALL: [Self; 12] = [
    Self::Landing,
    Self::Practice,
    Self::Exam,
    Self::Browse,
    Self::Index,
    Self::Tools,
    Self::Lab,
    Self::Map,
    Self::Log,
    Self::Read,
    Self::Board,
    Self::Quiet,
  ];

  /// 写进 `data-sig` 的字符串。样式层只认这个值。
  #[must_use]
  pub const fn attr(self) -> &'static str {
    match self {
      Self::Landing => "landing",
      Self::Practice => "practice",
      Self::Exam => "exam",
      Self::Browse => "browse",
      Self::Index => "index",
      Self::Tools => "tools",
      Self::Lab => "lab",
      Self::Map => "map",
      Self::Log => "log",
      Self::Read => "read",
      Self::Board => "board",
      Self::Quiet => "quiet",
    }
  }

  /// 由 `data-sig` 反解；不认识的值给 [`Self::Quiet`]（宁可少动效，也不要乱动效）。
  ///
  /// 是 [`Self::attr`] 的逆运算，用于「从 DOM 读回来」的场合（e2e 断言、调试面板）。
  /// 生产路径上没有人调用它 —— 但 [`tests::attr_round_trips`] 靠它保证 12 个签名的字符串
  /// 两两不同，改 `attr` 时漏改一个的代价就是两个动作族悄悄合并。
  #[must_use]
  pub fn parse(value: &str) -> Self {
    Self::ALL
      .into_iter()
      .find(|sig| sig.attr() == value)
      .unwrap_or(Self::Quiet)
  }

  /// 该签名覆盖的路由（**精确匹配**，不含查询串）。
  ///
  /// 用精确表而不是前缀表：`/antenna-theory`（读）与 `/antenna-analyzer`（算）只差一个词，
  /// 前缀匹配会把它们吸到同一族里，而这两类页面的动作本来就该不同。
  #[must_use]
  pub const fn paths(self) -> &'static [&'static str] {
    match self {
      Self::Landing => &["/"],
      Self::Practice => &[
        "/practice",
        "/flashcards",
        "/listen",
        "/cards",
        "/daily-challenge",
        "/battle",
        "/morse",
      ],
      Self::Exam => &["/exam", "/exam-review", "/exam-comparison"],
      Self::Browse => &[
        "/browse",
        "/mistakes",
        "/mistake-topics",
        "/bookmarks",
        "/most-wanted",
      ],
      Self::Index => &[
        "/glossary",
        "/q-code",
        "/phonetic",
        "/prefixes",
        "/confusables",
        "/callsign",
        "/callsign-copy",
        "/rst",
        "/cheat-sheet",
        "/formulas",
        "/frequencies",
        "/bands",
        "/bandplan",
        "/meters",
        "/modes",
        "/analog-modes",
        "/modern-digital-modes",
        "/license-classes",
        "/organizations",
        "/bestiary",
        "/logging-software",
      ],
      Self::Tools => &[
        "/tools",
        "/antenna-analyzer",
        "/antenna-array",
        "/antenna-farm",
        "/antenna-modeling",
        "/antenna-tuning",
        "/vna",
        "/vna-reader",
        "/qsl-designer",
        "/qsl-card",
        "/radiogram",
        "/photo-processor",
        "/coordination",
        "/spectrum-admin",
        "/ardf",
        "/measurement-lab",
        "/power",
        "/power-supply",
        "/feedline",
        "/filters",
        "/amplifier",
      ],
      Self::Lab => &[
        "/nec",
        "/smith",
        "/waveform-lab",
        "/signal-id",
        "/sdr-waterfall",
        "/apt-decoder",
        "/sstv-decode",
        "/wspr-decode",
        "/psk-decode",
        "/navtex-decode",
        "/lrpt-decode",
        "/dominoex-decode",
        "/olivia-decode",
        "/rtty-decode",
        "/hell-decode",
        "/wefax-decode",
        "/ft8-decode",
        "/weather-sat",
      ],
      Self::Map => &[
        "/map",
        "/grid-map",
        "/dxcc-map",
        "/zone-map",
        "/portable-map",
        "/sdr-map",
        "/grid",
        "/grayline",
        "/satellites",
      ],
      Self::Log => &[
        "/log",
        "/contest-log",
        "/contest",
        "/contest-calendar",
        "/cabrillo",
        "/eqsl",
        "/qsl-labels",
        "/awards",
        "/events",
        "/countdown",
      ],
      Self::Read => &[
        "/reference",
        "/radio-law",
        "/regulations",
        "/history",
        "/safety",
        "/grounding",
        "/rfi",
        "/noise",
        "/beginner",
        "/learning-path",
        "/learning-resources",
        "/antenna-theory",
        "/antennas",
        "/practical-antennas",
        "/antenna-diy",
        "/antenna-installation",
        "/electronics",
        "/transceiver",
        "/receiver",
        "/rig-reviews",
        "/gear",
        "/mobile",
        "/operating",
        "/china-ham",
        "/emcomm",
        "/winlink",
        "/aprs",
        "/sdr",
        "/gnuradio",
        "/dsp",
        "/modulation",
        "/digital-comms",
        "/lora",
        "/dv-network",
        "/kiss-tnc",
        "/rtty",
        "/ft8",
        "/wspr",
        "/sstv",
        "/atv",
        "/nvis",
        "/special-prop",
        "/meteor-scatter",
        "/aurora",
        "/eme",
        "/muf",
        "/propagation",
        "/beacons",
        "/microwave",
        "/qrp",
        "/polarization",
        "/connectors",
        "/balun",
        "/community",
        "/developers",
        "/open-source",
        "/diy-projects",
        "/license",
        "/license.html",
        "/compliance",
        "/swl",
        "/remote",
        "/print",
        "/cw-operating",
        "/dx",
        "/dxpedition",
        "/iota",
        "/portable",
        "/sat-operation",
        "/packet",
        "/repeater",
        "/repeater-build",
      ],
      Self::Board => &[
        "/progress",
        "/stats",
        "/dashboard",
        "/achievements",
        "/report",
        "/study-calendar",
        "/weekly",
        "/notifications",
        "/backup",
        "/solar",
        "/rbn",
        "/psk-reporter",
        "/dx-spots",
      ],
      Self::Quiet => &[],
    }
  }
}

/// 去掉查询串、锚点与尾部斜杠，得到用于查表的路径。
///
/// 刷新时 `/log?tab=2` 与 `/log` 必须是同一个签名，否则带参数进来会悄悄退化成兜底动作。
#[must_use]
pub fn normalize_path(path: &str) -> &str {
  let path = path.split(['?', '#']).next().unwrap_or(path);
  if path.len() <= 1 {
    return path;
  }
  let trimmed = path.trim_end_matches('/');
  // `"//"` 这类退化输入修剪后是空串：回落到根路径，别让它在表里查不到而退成兜底动作。
  if trimmed.is_empty() { "/" } else { trimmed }
}

/// 路由对应的交互签名；未登记的路由回落到 [`PageSignature::Quiet`]。
#[must_use]
pub fn signature_for(path: &str) -> PageSignature {
  let path = normalize_path(path);
  PageSignature::ALL
    .into_iter()
    .find(|sig| sig.paths().contains(&path))
    .unwrap_or(PageSignature::Quiet)
}

/// 路由层级：段数。`/` 是 0，`/log` 是 1。
///
/// 只用来判断「这次导航是往里走还是往回退」，不做面包屑。
#[must_use]
pub fn depth(path: &str) -> usize {
  normalize_path(path)
    .split('/')
    .filter(|seg| !seg.is_empty())
    .count()
}

/// 换页方向：决定路由动画往哪个方向、以什么方式退场。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavDirection {
  /// 首次进入（首屏不播换页动画，交给页面自己的入场编排）。
  Initial,
  /// 往里走（层级变深，或从别的模块进来）：内容自下而上「升起」。
  Forward,
  /// 往回退（浏览器后退，或层级变浅）：内容自上而下「落回」。
  Back,
  /// 平级横向跳转：只做交叉淡入，不位移 —— 同级页面之间没有「深入」的语义。
  Lateral,
}

impl NavDirection {
  /// 写进 `data-nav-dir` 的字符串。
  #[must_use]
  pub const fn attr(self) -> &'static str {
    match self {
      Self::Initial => "initial",
      Self::Forward => "forward",
      Self::Back => "back",
      Self::Lateral => "lateral",
    }
  }
}

/// 计算这次导航的方向。
///
/// `history_back` 由外壳的 `popstate` 标记给出 —— 回退时层级可能没变（`/a` → `/b` 再退回来），
/// 只比层级会误判成平级，方向感就丢了。
///
/// 浏览器「前进」也走 `popstate`，这里与「后退」同向（都算 [`NavDirection::Back`]）：
/// 区分两者要读 `history.state` 里的序号，而这一版的收益只有「前进时动画方向正确」，
/// 不值得为此给每次导航塞状态。已知取舍，不是遗漏。
#[must_use]
pub fn nav_direction(from: Option<&str>, to: &str, history_back: bool) -> NavDirection {
  let Some(from) = from else {
    return NavDirection::Initial;
  };
  if normalize_path(from) == normalize_path(to) {
    // 同页只改查询串（筛选、翻页）：不换页，也不该播换页动画。
    return NavDirection::Lateral;
  }
  if history_back {
    return NavDirection::Back;
  }
  let (from_depth, to_depth) = (depth(from), depth(to));
  if to_depth > from_depth {
    NavDirection::Forward
  } else if to_depth < from_depth {
    NavDirection::Back
  } else {
    NavDirection::Lateral
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::registry;

  #[test]
  fn every_registered_route_has_a_signature() {
    let missing: Vec<&str> = registry::MODULES
      .iter()
      .map(|m| m.path)
      .filter(|p| signature_for(p) == PageSignature::Quiet)
      .collect();
    assert!(
      missing.is_empty(),
      "这些路由还没挑交互签名（去 PageSignature::paths 里登记）：{missing:?}"
    );
  }

  #[test]
  fn each_path_belongs_to_exactly_one_signature() {
    let mut seen = std::collections::HashMap::new();
    for sig in PageSignature::ALL {
      for path in sig.paths() {
        if let Some(previous) = seen.insert(*path, sig) {
          panic!("{path} 同时登记在 {previous:?} 与 {sig:?}");
        }
      }
    }
  }

  #[test]
  fn no_dead_paths_in_the_table() {
    let dead: Vec<&str> = PageSignature::ALL
      .into_iter()
      .flat_map(|sig| sig.paths().to_vec())
      .filter(|p| registry::by_path(p).is_none())
      .collect();
    assert!(dead.is_empty(), "表里有不存在于 registry 的路由：{dead:?}");
  }

  #[test]
  fn representative_routes() {
    assert_eq!(signature_for("/"), PageSignature::Landing);
    assert_eq!(signature_for("/practice"), PageSignature::Practice);
    assert_eq!(signature_for("/exam"), PageSignature::Exam);
    assert_eq!(signature_for("/browse"), PageSignature::Browse);
    assert_eq!(signature_for("/glossary"), PageSignature::Index);
    assert_eq!(signature_for("/tools"), PageSignature::Tools);
    assert_eq!(signature_for("/ft8-decode"), PageSignature::Lab);
    assert_eq!(signature_for("/grid-map"), PageSignature::Map);
    assert_eq!(signature_for("/log"), PageSignature::Log);
    assert_eq!(signature_for("/radio-law"), PageSignature::Read);
    assert_eq!(signature_for("/dashboard"), PageSignature::Board);
    assert_eq!(signature_for("/not-a-route"), PageSignature::Quiet);
  }

  #[test]
  fn query_and_trailing_slash_do_not_change_the_signature() {
    assert_eq!(signature_for("/log?tab=2"), PageSignature::Log);
    assert_eq!(signature_for("/glossary/"), PageSignature::Index);
    assert_eq!(signature_for("/log#top"), PageSignature::Log);
    assert_eq!(normalize_path("/"), "/");
    assert_eq!(normalize_path("/a//b/"), "/a//b");
    // 退化输入：修剪后是空串时回落到根，否则查表会退成兜底动作。
    assert_eq!(normalize_path("//"), "/");
    assert_eq!(normalize_path(""), "");
  }

  #[test]
  fn attr_round_trips() {
    for sig in PageSignature::ALL {
      assert_eq!(PageSignature::parse(sig.attr()), sig);
    }
    assert_eq!(PageSignature::parse("nope"), PageSignature::Quiet);
  }

  #[test]
  fn depth_counts_segments() {
    assert_eq!(depth("/"), 0);
    assert_eq!(depth("/log"), 1);
    assert_eq!(depth("/log/detail"), 2);
    assert_eq!(depth("/log/"), 1);
  }

  #[test]
  fn nav_direction_prefers_history_over_depth() {
    assert_eq!(nav_direction(None, "/log", false), NavDirection::Initial);
    // 回退时哪怕层级没变也按「退」处理。
    assert_eq!(
      nav_direction(Some("/log"), "/stats", true),
      NavDirection::Back
    );
    // 首页（0 段）进内页（≥1 段）：算往里走。
    assert_eq!(
      nav_direction(Some("/"), "/stats", false),
      NavDirection::Forward
    );
  }

  #[test]
  fn nav_direction_uses_depth_for_forward_and_back() {
    // `/log` 是 1 段、`/log/detail` 是 2 段：往里走。
    assert_eq!(
      nav_direction(Some("/log"), "/log/detail", false),
      NavDirection::Forward
    );
    assert_eq!(
      nav_direction(Some("/log/detail"), "/log", false),
      NavDirection::Back
    );
    // 同层级：平级横向。
    assert_eq!(
      nav_direction(Some("/log"), "/stats", false),
      NavDirection::Lateral
    );
    // 同页改查询串：不换页。
    assert_eq!(
      nav_direction(Some("/log?tab=1"), "/log?tab=2", false),
      NavDirection::Lateral
    );
  }

  #[test]
  fn the_table_covers_every_registered_route_exactly_once() {
    let total: usize = PageSignature::ALL
      .into_iter()
      .map(|sig| sig.paths().len())
      .sum();
    assert_eq!(total, registry::MODULES.len());
  }
}
