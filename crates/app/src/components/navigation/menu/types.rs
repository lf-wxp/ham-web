/// 桌面端下拉菜单种类。
///
/// 菜单项本身不再定义在这里 —— 唯一事实来源是 `ham_web_core::registry`，
/// 本模块只保留「当前展开的是哪个下拉」这一前端状态所需的枚举。
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum MenuKind {
  Exam,
  Knowledge,
  Tools,
}
