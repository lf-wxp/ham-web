//! 列表框（下拉选项）的键盘导航：把「按一下方向键，高亮落在第几行」做成可测的纯函数。
//!
//! 放在 core 的原因：下拉有两套外壳（像素 / 现代）和两种用法（数据式 / 组合式），但键盘行为必须是
//! 同一个答案。这条规则以前内联在数据式下拉里（当时叫 `NativeSelect`），组合式的 `Select` 根本没有 —— 于是语言切换（用的是
//! `Select`）键盘完全不可用。拆成纯函数后两边共用同一份，边界（空列表、越界的旧下标、首尾回绕）
//! 也能用单测守住，而不是靠在浏览器里按方向键试。

/// 一次导航动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nav {
  /// 下一项（`↓`）：到末尾后回绕到第一项。
  Next,
  /// 上一项（`↑`）：到开头后回绕到最后一项。
  Prev,
  /// 第一项（`Home`）。
  First,
  /// 最后一项（`End`）。
  Last,
}

/// 从 `current`（当前高亮的下标，`None` 表示还没有高亮）按 `nav` 移动后的下标。
///
/// - `len == 0` 一律返回 `None`：没有可高亮的行；
/// - `current` 越界（选项列表刚缩短、旧下标已经不存在）按「还没有高亮」处理，
///   而不是 panic 或停在一个不存在的行上；
/// - 还没有高亮时，`Next` 落在第一项、`Prev` 落在最后一项（与原生 `<select>` 一致）。
#[must_use]
pub fn navigate(current: Option<usize>, len: usize, nav: Nav) -> Option<usize> {
  if len == 0 {
    return None;
  }
  let current = current.filter(|&i| i < len);
  Some(match (nav, current) {
    (Nav::First, _) => 0,
    (Nav::Last, _) => len - 1,
    (Nav::Next, None) => 0,
    (Nav::Prev, None) => len - 1,
    // 不写成 `(i + 1) % len` / `(i + len - 1) % len`：后者在 `len` 接近 `usize::MAX` 时会溢出。
    (Nav::Next, Some(i)) if i + 1 == len => 0,
    (Nav::Next, Some(i)) => i + 1,
    (Nav::Prev, Some(0)) => len - 1,
    (Nav::Prev, Some(i)) => i - 1,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn next_and_prev_wrap_around() {
    assert_eq!(navigate(Some(0), 3, Nav::Next), Some(1));
    assert_eq!(
      navigate(Some(2), 3, Nav::Next),
      Some(0),
      "末尾再往下回到第一项"
    );
    assert_eq!(navigate(Some(1), 3, Nav::Prev), Some(0));
    assert_eq!(
      navigate(Some(0), 3, Nav::Prev),
      Some(2),
      "开头再往上回到最后一项"
    );
  }

  #[test]
  fn without_a_highlight_next_starts_at_top_and_prev_at_bottom() {
    assert_eq!(navigate(None, 4, Nav::Next), Some(0));
    assert_eq!(navigate(None, 4, Nav::Prev), Some(3));
  }

  #[test]
  fn home_and_end_ignore_where_you_are() {
    for current in [None, Some(0), Some(2), Some(9)] {
      assert_eq!(navigate(current, 5, Nav::First), Some(0));
      assert_eq!(navigate(current, 5, Nav::Last), Some(4));
    }
  }

  #[test]
  fn empty_list_never_highlights_anything() {
    for nav in [Nav::Next, Nav::Prev, Nav::First, Nav::Last] {
      assert_eq!(navigate(None, 0, nav), None);
      assert_eq!(navigate(Some(3), 0, nav), None);
    }
  }

  #[test]
  fn a_stale_out_of_range_index_is_treated_as_no_highlight() {
    // 选项列表缩短后，旧下标 7 已经不存在：行为等同「还没有高亮」。
    assert_eq!(navigate(Some(7), 3, Nav::Next), Some(0));
    assert_eq!(navigate(Some(7), 3, Nav::Prev), Some(2));
    // 恰好等于 len 也算越界（下标从 0 起）。
    assert_eq!(navigate(Some(3), 3, Nav::Next), Some(0));
  }

  #[test]
  fn single_item_list_stays_put() {
    assert_eq!(navigate(Some(0), 1, Nav::Next), Some(0));
    assert_eq!(navigate(Some(0), 1, Nav::Prev), Some(0));
    assert_eq!(navigate(None, 1, Nav::Next), Some(0));
  }

  #[test]
  fn usize_extremes_do_not_overflow() {
    // 越界的旧下标：先按「没有高亮」处理，不能再做 `+ 1`。
    assert_eq!(navigate(Some(usize::MAX), 3, Nav::Next), Some(0));
    assert_eq!(navigate(Some(usize::MAX), 3, Nav::Prev), Some(2));
    // 列表长度本身就是 `usize::MAX`：末项往下回绕、首项往上回绕都不能溢出。
    assert_eq!(
      navigate(Some(usize::MAX - 1), usize::MAX, Nav::Next),
      Some(0)
    );
    assert_eq!(
      navigate(Some(0), usize::MAX, Nav::Prev),
      Some(usize::MAX - 1)
    );
  }
}
