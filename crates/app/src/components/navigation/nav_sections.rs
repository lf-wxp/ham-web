//! 移动端菜单的三大段清单：段标签、slug 与所属一级模块。
//!
//! 段标签栏、段内分组与条目都读这一份表；分组清单不在这里存，由 `registry::groups_of(kind)`
//! 给出 —— 分成两份写，新增分组时就会有一处静默漏掉（导航里少一项，没人发现）。
//!
//! 表里存词条 key，渲染处再 `t(key)`：词条本身在 `data/i18n/{zh,en,es}/shell.json`，
//! 与顶栏桌面下拉用的那一组相同。

use ham_web_core::registry::{self, ModuleKind};

/// 移动端菜单里的一大段。
#[derive(Debug, Clone, Copy)]
pub(crate) struct NavSection {
  /// 标签值（ascii slug）：`TabsTrigger` 拿它拼 DOM id，别写中文。
  pub(crate) slug: &'static str,
  /// 段标签的词条 key（`shell.exam-center` / `shell.knowledge` / `shell.tools`）。
  pub(crate) label: &'static str,
  /// 这一段对应的一级模块。
  pub(crate) kind: ModuleKind,
}

impl NavSection {
  /// 这一段包含的子分组（按导航展示顺序）。
  pub(crate) fn groups(&self) -> &'static [&'static str] {
    registry::groups_of(self.kind)
  }
}

/// 三段，按顶栏从左到右的顺序。
pub(crate) const NAV_SECTIONS: &[NavSection] = &[
  NavSection {
    slug: "exam",
    label: "shell.exam-center",
    kind: ModuleKind::Exam,
  },
  NavSection {
    slug: "knowledge",
    label: "shell.knowledge",
    kind: ModuleKind::Knowledge,
  },
  NavSection {
    slug: "tools",
    label: "shell.tools",
    kind: ModuleKind::Tool,
  },
];

/// `pathname` 所在的段；不属于任何段（`/backup`、`/achievements` 这类不挂分组的页面、
/// 注册表里没有的路径）落到第一段 —— 菜单打开时总得有一段是选中的，不能空着。
pub(crate) fn section_of(pathname: &str) -> &'static NavSection {
  registry::by_path(pathname)
    .and_then(registry::Module::kind)
    .and_then(|kind| NAV_SECTIONS.iter().find(|s| s.kind == kind))
    .unwrap_or(&NAV_SECTIONS[0])
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn every_kind_has_exactly_one_section() {
    // 新增一级模块时这里会红：手机菜单少一段，没人发现。
    for kind in [ModuleKind::Exam, ModuleKind::Knowledge, ModuleKind::Tool] {
      let n = NAV_SECTIONS.iter().filter(|s| s.kind == kind).count();
      assert_eq!(n, 1, "{kind:?} 应恰好有一段，实际 {n}");
    }
  }

  #[test]
  fn slugs_are_unique_ascii() {
    // `TabsTrigger` 拿 slug 拼 DOM id：中文或重复都会让 `aria-controls` 指错。
    let mut seen = std::collections::HashSet::new();
    for s in NAV_SECTIONS {
      assert!(
        s.slug.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
        "slug {:?} 必须是小写 ascii",
        s.slug
      );
      assert!(seen.insert(s.slug), "slug {:?} 重复", s.slug);
    }
  }

  #[test]
  fn every_nav_module_lands_in_the_section_of_its_group() {
    for m in registry::MODULES.iter().filter(|m| m.in_nav()) {
      let section = section_of(m.path);
      let group = m.group.expect("in_nav 的模块都有分组");
      assert!(
        section.groups().contains(&group),
        "{} 的分组 {group} 不在它落入的段「{}」里",
        m.path,
        section.slug
      );
    }
  }

  #[test]
  fn pages_outside_any_group_fall_back_to_the_first_section() {
    assert_eq!(section_of("/backup").slug, NAV_SECTIONS[0].slug);
    assert_eq!(section_of("/no-such-page").slug, NAV_SECTIONS[0].slug);
  }
}
