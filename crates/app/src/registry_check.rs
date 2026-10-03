//! 能力注册表的一致性校验。
//!
//! 新增页面原本要在三处同步：路由表（`app/main_content.rs`）→ 导航项 → 页面标题。
//! 现在导航由 `ham_web_core::registry` 派生，只剩两处；本模块的测试再把「注册表 ↔
//! 实际 `<Route>`」钉死 —— 漏注册或写错路径会直接让 `cargo make check` 失败，
//! 而不是等到线上点进 404 才发现。
//!
//! 做法是直接读路由表源码：Leptos 的 `<Route view=…>` 需要具体组件类型，无法由数据
//! 驱动，所以只能反查源码文本。

use ham_web_core::registry::{self, GROUP_EXAM, KNOWLEDGE_GROUPS, TOOL_GROUPS};

use crate::icons::icon_of;

/// 从路由表源码中提取全部 `path!("/…")`。
fn routes_from_source(src: &str) -> Vec<&str> {
  const MARK: &str = "path!(\"";
  let mut out = Vec::new();
  let mut rest = src;
  while let Some(start) = rest.find(MARK) {
    let after = &rest[start + MARK.len()..];
    let Some(end) = after.find('"') else {
      break;
    };
    out.push(&after[..end]);
    rest = &after[end..];
  }
  out
}

fn sorted_dedup(mut list: Vec<&str>) -> Vec<&str> {
  list.sort_unstable();
  list.dedup();
  list
}

/// 注册表路径集合 == 路由表路径集合（双向）。
#[test]
fn registry_matches_router() {
  let src = include_str!("app/main_content.rs");
  let routes = sorted_dedup(routes_from_source(src));
  assert!(
    routes.len() > 100,
    "路由解析异常，只找到 {} 条",
    routes.len()
  );

  let registered = sorted_dedup(registry::MODULES.iter().map(|m| m.path).collect());

  let not_registered: Vec<&str> = routes
    .iter()
    .copied()
    .filter(|r| !registered.contains(r))
    .collect();
  assert!(
    not_registered.is_empty(),
    "这些 <Route> 没有在 registry::MODULES 中注册：{not_registered:?}"
  );

  let no_route: Vec<&str> = registered
    .iter()
    .copied()
    .filter(|r| !routes.contains(r))
    .collect();
  assert!(
    no_route.is_empty(),
    "registry::MODULES 里的这些路径没有对应的 <Route>：{no_route:?}"
  );
}

/// 导航里的路径必须真的存在于路由表（防止死链）。
#[test]
fn nav_paths_are_routable() {
  let src = include_str!("app/main_content.rs");
  let routes = sorted_dedup(routes_from_source(src));
  for path in registry::nav_paths() {
    assert!(routes.contains(&path), "导航项 {path} 没有对应的路由");
  }
}

/// 注册表里的图标 key 必须能被前端解析回 `IconKind`（拼错图标名会让菜单空白）。
#[test]
fn nav_icons_resolve() {
  for m in registry::MODULES.iter().filter(|m| m.in_nav()) {
    assert_eq!(
      icon_of(m.icon).name(),
      m.icon,
      "{} 的图标 key `{}` 无效",
      m.path,
      m.icon
    );
  }
  for group in KNOWLEDGE_GROUPS.iter().chain(TOOL_GROUPS.iter()) {
    let key = registry::group_icon(group);
    assert_eq!(icon_of(key).name(), key, "分组 {group} 的图标 key 无效");
  }
  let exam_icon = registry::group_icon(GROUP_EXAM);
  assert_eq!(icon_of(exam_icon).name(), exam_icon);
}

/// 导航链接必须落在自己的路由上（只允许附带查询串，如 `/x?mode=1`）。
#[test]
fn nav_hrefs_stay_on_their_route() {
  for m in registry::MODULES {
    let href = m.nav_href();
    let ok = href == m.path
      || href
        .strip_prefix(m.path)
        .is_some_and(|rest| rest.starts_with('?'));
    assert!(ok, "{} 的导航链接 {href} 不在其路由上", m.path);
  }
}

/// 一级模块划分：考试 / 知识库 / 工具三组，且每个导航项都能归入其一。
#[test]
fn kinds_cover_every_nav_module() {
  use ham_web_core::registry::ModuleKind;
  let mut exams = 0;
  let mut knowledge = 0;
  let mut tools = 0;
  for m in registry::MODULES.iter().filter(|m| m.in_nav()) {
    match m.kind().expect("导航项必须有分组") {
      ModuleKind::Exam => exams += 1,
      ModuleKind::Knowledge => knowledge += 1,
      ModuleKind::Tool => tools += 1,
    }
  }
  assert_eq!(exams, registry::exam_items().len());
  assert!(knowledge >= 80 && tools >= 20 && exams >= 14);
}
