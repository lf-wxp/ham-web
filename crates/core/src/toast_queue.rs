//! Toast 队列的入队规则（纯逻辑，不依赖浏览器；供 `ui::toast` 渲染）。
//!
//! 规则放在这里而不是组件里，是因为它们有边界值得单测：什么算「同一条」、退场中的队首
//! 能不能被合并、计数会不会溢出。

use std::collections::VecDeque;

/// Toast 类别（决定图标、强调色与停留时长）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
  Success,
  Error,
  Info,
}

impl ToastKind {
  /// 停留时长（毫秒）：错误要留够读完一句话的时间（往往还带一段原因），成功则越短越好。
  pub const fn display_ms(self) -> u64 {
    match self {
      Self::Success => 2_400,
      Self::Info => 3_500,
      Self::Error => 6_000,
    }
  }
}

/// 一条待展示的 Toast。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToastEntry {
  pub id: u64,
  pub kind: ToastKind,
  /// 已翻译的文案（入队时由调用点取好，队列不持有词条 key）。
  pub text: String,
  /// 同一条消息合并的次数：连点三次「保存」只弹一条 `×3`，而不是排队等三次。
  pub count: u32,
}

/// 入队一条 Toast，返回是否并入了已有的那一条（`true`）而不是新排一条（`false`）。
///
/// 同类别、同文案视为同一条：只加计数。例外是**正在退场的队首**（`front_leaving`）——
/// 它已经在播放退出动画、几十毫秒后就消失，把 `×N` 加在它身上，用户永远看不到第二次
/// 反馈；这种情况下改为新排一条，等它出队后接着展示。队首之后的条目不受影响，照常合并。
///
/// `next_id` 只在确实要新建时才调用（合并时不消耗 id）。
pub fn enqueue(
  queue: &mut VecDeque<ToastEntry>,
  kind: ToastKind,
  text: String,
  front_leaving: bool,
  next_id: impl FnOnce() -> u64,
) -> bool {
  let skip = usize::from(front_leaving);
  if let Some(m) = queue
    .iter_mut()
    .skip(skip)
    .find(|m| m.kind == kind && m.text == text)
  {
    m.count = m.count.saturating_add(1);
    return true;
  }
  queue.push_back(ToastEntry {
    id: next_id(),
    kind,
    text,
    count: 1,
  });
  false
}

#[cfg(test)]
mod tests {
  use super::*;

  fn push(
    q: &mut VecDeque<ToastEntry>,
    kind: ToastKind,
    text: &str,
    leaving: bool,
    id: u64,
  ) -> bool {
    enqueue(q, kind, text.to_owned(), leaving, || id)
  }

  #[test]
  fn same_kind_and_text_merge_into_one_with_a_count() {
    let mut q = VecDeque::new();
    assert!(!push(&mut q, ToastKind::Success, "已保存", false, 1));
    assert!(push(&mut q, ToastKind::Success, "已保存", false, 2));
    assert!(push(&mut q, ToastKind::Success, "已保存", false, 3));
    assert_eq!(q.len(), 1);
    assert_eq!(q[0].count, 3);
    assert_eq!(q[0].id, 1, "合并不改变队首的 id（计时器只认 id）");
  }

  #[test]
  fn different_kind_or_text_is_a_separate_entry() {
    let mut q = VecDeque::new();
    push(&mut q, ToastKind::Success, "已保存", false, 1);
    assert!(!push(&mut q, ToastKind::Error, "已保存", false, 2));
    assert!(!push(&mut q, ToastKind::Success, "已复制", false, 3));
    assert_eq!(q.len(), 3);
    assert!(q.iter().all(|m| m.count == 1));
  }

  #[test]
  fn leaving_front_is_never_merged_into() {
    let mut q = VecDeque::new();
    push(&mut q, ToastKind::Success, "已保存", false, 1);
    // 队首正在退场：同文案再来一条，应当另排一条，而不是给即将消失的那条加计数。
    assert!(!push(&mut q, ToastKind::Success, "已保存", true, 2));
    assert_eq!(q.len(), 2);
    assert_eq!(q[0].count, 1);
    assert_eq!(q[1].count, 1);
  }

  #[test]
  fn leaving_front_does_not_block_merging_into_later_entries() {
    let mut q = VecDeque::new();
    push(&mut q, ToastKind::Success, "甲", false, 1);
    push(&mut q, ToastKind::Error, "乙", false, 2);
    assert!(push(&mut q, ToastKind::Error, "乙", true, 3));
    assert_eq!(q.len(), 2);
    assert_eq!(q[1].count, 2);
  }

  #[test]
  fn id_is_only_consumed_when_a_new_entry_is_created() {
    let mut q = VecDeque::new();
    push(&mut q, ToastKind::Info, "提示", false, 1);
    let merged = enqueue(&mut q, ToastKind::Info, "提示".to_owned(), false, || {
      panic!("合并时不该申请新 id")
    });
    assert!(merged);
  }

  #[test]
  fn count_saturates_instead_of_overflowing() {
    let mut q = VecDeque::new();
    push(&mut q, ToastKind::Info, "提示", false, 1);
    q[0].count = u32::MAX;
    assert!(push(&mut q, ToastKind::Info, "提示", false, 2));
    assert_eq!(q[0].count, u32::MAX);
  }

  #[test]
  fn errors_stay_longer_than_info_and_success() {
    assert!(ToastKind::Error.display_ms() > ToastKind::Info.display_ms());
    assert!(ToastKind::Info.display_ms() > ToastKind::Success.display_ms());
  }
}
