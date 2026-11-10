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

/// 按前缀跳到匹配项（下拉的 typeahead）。
///
/// 长列表（DXCC 实体、工具类型、题库版本有几十上百项）逐项按方向键不现实 ——
/// 原生 `<select>` 与各桌面工具都支持「连打几个字母跳过去」。
///
/// - 不区分大小写；
/// - 从 `from` 的**下一项**开始找并环形回绕，最后才落到自己（与原生 `<select>` 一致：
///   连按同一个字母时在「以该字母开头的项」之间轮转）；
/// - 查询串是同一个字符的重复（`jj`、`444`）时按单个字符匹配：调用方用时间窗口累积按键，
///   快速连按同一个字母会得到 `jj`，没有这条规则就匹配不到任何项，轮转也就失效了
///   （原生 `<select>` 同样如此）；
/// - `query` 为空或没有任何匹配返回 `None`（调用方据此保持原高亮，不要乱跳）。
#[must_use]
pub fn typeahead(labels: &[&str], query: &str, from: Option<usize>) -> Option<usize> {
  let mut needle = query.trim().to_lowercase();
  if needle.is_empty() || labels.is_empty() {
    return None;
  }
  if let Some(first) = needle.chars().next()
    && needle.chars().count() > 1
    && needle.chars().all(|c| c == first)
  {
    needle = first.to_string();
  }
  let len = labels.len();
  let start = from.filter(|&i| i < len).map_or(0, |i| (i + 1) % len);
  (0..len)
    .map(|offset| (start + offset) % len)
    .find(|&i| labels[i].to_lowercase().starts_with(&needle))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn typeahead_jumps_to_the_first_prefix_match_after_the_current_row() {
    let labels = ["20m", "40m", "80m", "160m"];
    // 从「还没高亮」开始：落在第一个匹配项。
    assert_eq!(typeahead(&labels, "4", None), Some(1));
    // 已经在 40m：从下一项开始环形找，回到自己。
    assert_eq!(typeahead(&labels, "4", Some(1)), Some(1));
    // 大小写不敏感。
    assert_eq!(
      typeahead(&["Japan", "Jersey", "Kenya"], "je", None),
      Some(1)
    );
  }

  #[test]
  fn typeahead_rotates_between_items_sharing_the_prefix() {
    let labels = ["Japan", "Jersey", "Kenya"];
    assert_eq!(typeahead(&labels, "j", None), Some(0));
    assert_eq!(typeahead(&labels, "j", Some(0)), Some(1));
    assert_eq!(typeahead(&labels, "j", Some(1)), Some(0));
  }

  #[test]
  fn typeahead_repeated_same_char_keeps_rotating() {
    // 快速连按 `j`：缓冲累积成 `jj` / `jjj`，仍按单个字符轮转。
    let labels = ["Japan", "Jersey", "Kenya"];
    assert_eq!(typeahead(&labels, "jj", Some(0)), Some(1));
    assert_eq!(typeahead(&labels, "jjj", Some(1)), Some(0));
    // 不是同一个字符的重复仍是正常前缀：`je` 只命中 Jersey。
    assert_eq!(typeahead(&labels, "je", Some(1)), Some(1));
  }

  #[test]
  fn typeahead_returns_none_when_nothing_matches() {
    let labels = ["20m", "40m"];
    assert_eq!(typeahead(&labels, "z", None), None);
    assert_eq!(typeahead(&labels, "", None), None);
    assert_eq!(typeahead(&[], "a", None), None);
  }

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
