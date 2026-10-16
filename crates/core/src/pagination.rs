//! 分页页码窗口计算（纯逻辑，不依赖浏览器；供 `ui::Pagination` 渲染）。

/// 页码序列里的一项：一个页码，或一段省略号。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageItem {
  Page(u32),
  Ellipsis,
}

/// 计算要显示的页码窗口：首尾两页固定，中间给当前页前后各 1 页的邻域，
/// 邻域与首尾之间有空隙时插省略号。页数少（≤ 7）时全列，不加省略号。
///
/// `current` 越界时按 `1..=total` 夹取；`total == 0` 返回空序列。
pub fn page_window(current: u32, total: u32) -> Vec<PageItem> {
  if total == 0 {
    return Vec::new();
  }
  let c = current.clamp(1, total);
  if total <= 7 {
    return (1..=total).map(PageItem::Page).collect();
  }
  let mut items = Vec::with_capacity(7);
  items.push(PageItem::Page(1));
  let start = c.saturating_sub(1).max(2);
  let end = c.saturating_add(1).min(total - 1);
  if start > 2 {
    items.push(PageItem::Ellipsis);
  }
  for p in start..=end {
    items.push(PageItem::Page(p));
  }
  if end < total - 1 {
    items.push(PageItem::Ellipsis);
  }
  items.push(PageItem::Page(total));
  items
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn empty_total_gives_no_items() {
    assert!(page_window(1, 0).is_empty());
  }

  #[test]
  fn small_total_lists_all_pages() {
    assert_eq!(
      page_window(3, 7),
      (1..=7).map(PageItem::Page).collect::<Vec<_>>()
    );
  }

  #[test]
  fn current_clamped_into_range() {
    assert_eq!(
      page_window(99, 7),
      (1..=7).map(PageItem::Page).collect::<Vec<_>>()
    );
    assert_eq!(
      page_window(0, 7),
      (1..=7).map(PageItem::Page).collect::<Vec<_>>()
    );
  }

  #[test]
  fn middle_page_has_both_ellipses() {
    assert_eq!(
      page_window(10, 50),
      vec![
        PageItem::Page(1),
        PageItem::Ellipsis,
        PageItem::Page(9),
        PageItem::Page(10),
        PageItem::Page(11),
        PageItem::Ellipsis,
        PageItem::Page(50),
      ]
    );
  }

  #[test]
  fn near_start_keeps_leading_run_without_ellipsis() {
    assert_eq!(
      page_window(2, 50),
      vec![
        PageItem::Page(1),
        PageItem::Page(2),
        PageItem::Page(3),
        PageItem::Ellipsis,
        PageItem::Page(50),
      ]
    );
  }

  #[test]
  fn near_end_keeps_trailing_run_without_ellipsis() {
    assert_eq!(
      page_window(49, 50),
      vec![
        PageItem::Page(1),
        PageItem::Ellipsis,
        PageItem::Page(48),
        PageItem::Page(49),
        PageItem::Page(50),
      ]
    );
  }
}
