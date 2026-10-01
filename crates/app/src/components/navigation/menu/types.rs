use crate::icons::IconKind;

/// 一条导航项。
pub(crate) struct NavItem {
  pub(crate) href: &'static str,
  pub(crate) label: &'static str,
  pub(crate) icon: IconKind,
}

/// 一组导航项（用于知识库分组）。
pub(crate) struct NavGroup {
  pub(crate) label: &'static str,
  pub(crate) icon: IconKind,
  pub(crate) items: &'static [NavItem],
}

/// 桌面端下拉菜单种类。
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum MenuKind {
  Exam,
  Knowledge,
  Tools,
}
