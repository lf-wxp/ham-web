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

/// 交互工具页尚无专属 e2e 的存量欠账（**只许减不许增**）。
///
/// 冒烟（`smoke.spec.ts`）只证明页面能渲染、无障碍无违规，证明不了「调参数会出结果」。
/// 所以工具类路由（[`ModuleKind::Tool`] 与考试中心）必须至少被一条功能用例显式
/// `page.goto` 过。门禁上线时这里有 12 页欠账，已于 2026-10 全部补齐（依赖 `/api/*` 的页面
/// 用 `fixtures.ts` 的 `stubApi` 打桩）—— 表保留为空，新增工具页不能再往这里加。
const TOOLS_WITHOUT_E2E: &[&str] = &[];

/// 从 spec 源码里取出显式 `goto("/…")` 的路由（去掉查询串与锚点）。
fn gotos_from_spec(src: &str) -> Vec<String> {
  let mut out = Vec::new();
  for quote in ['"', '\'', '`'] {
    let mark = format!("goto({quote}/");
    let mut rest = src;
    while let Some(start) = rest.find(&mark) {
      let after = &rest[start + mark.len() - 1..];
      let Some(end) = after[1..].find(quote) else {
        break;
      };
      let raw = &after[..=end];
      let path = raw.split(['?', '#']).next().unwrap_or(raw);
      out.push(path.to_owned());
      rest = &after[end + 1..];
    }
  }
  out
}

/// 工具类路由（工具分组 + 考试中心）必须有功能用例显式访问；存量欠账走棘轮。
///
/// 这是**最低门槛**：只证明「有用例到过这个页面」，不证明用例断言了什么 —— 断言质量靠
/// `e2e/coverage-targets.json` 登记的旅程与评审把关。模板字符串里带 `${…}` 的动态路径
/// 解析出来不会等于任何注册路由，所以只会漏记、不会让一个没测过的页面蒙混过关。
#[test]
fn interactive_routes_have_functional_e2e() {
  use ham_web_core::registry::ModuleKind;
  let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../e2e/tests");
  // 全量体检用例遍历路由表，不算「功能用例」（否则恒为全覆盖）。与 coverage.mjs 同口径。
  const SWEEP_FILES: &[&str] = &["smoke.spec.ts", "i18n_layout.spec.ts"];
  let mut visited: Vec<String> = Vec::new();
  for entry in std::fs::read_dir(&dir).expect("读取 e2e/tests") {
    let path = entry.expect("目录项").path();
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if !name.ends_with(".spec.ts") || SWEEP_FILES.contains(&name) {
      continue;
    }
    let src = std::fs::read_to_string(&path).expect("读取 spec");
    visited.extend(gotos_from_spec(&src));
  }
  assert!(
    visited.len() > 100,
    "goto 解析异常，只找到 {} 处",
    visited.len()
  );

  let interactive: Vec<&str> = registry::MODULES
    .iter()
    .filter(|m| matches!(m.kind(), Some(ModuleKind::Tool | ModuleKind::Exam)))
    .map(|m| m.path)
    .collect();
  let missing: Vec<&str> = interactive
    .iter()
    .copied()
    .filter(|p| !visited.iter().any(|v| v == p) && !TOOLS_WITHOUT_E2E.contains(p))
    .collect();
  assert!(
    missing.is_empty(),
    "这些交互页没有任何功能用例 `page.goto` 过（冒烟不算）：{missing:?}\n\
     补一条 e2e，并把旅程登记进 e2e/coverage-targets.json"
  );
  let paid_off: Vec<&str> = TOOLS_WITHOUT_E2E
    .iter()
    .copied()
    .filter(|p| visited.iter().any(|v| v == p) || !interactive.contains(p))
    .collect();
  assert!(
    paid_off.is_empty(),
    "这些路由已有功能用例（或已不是交互页），请从 TOOLS_WITHOUT_E2E 删掉：{paid_off:?}"
  );
}

#[test]
fn spec_goto_parser_handles_all_quote_styles() {
  let src = r#"page.goto("/a?x=1"); page.goto('/b#c'); page.goto(`/c`); page.goto(url);"#;
  assert_eq!(gotos_from_spec(src), ["/a", "/b", "/c"]);
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
