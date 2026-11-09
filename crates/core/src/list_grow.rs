//! 列表渐进展开的步进判定：首帧只放一小批，之后每画完一帧多放一批，放满为止。
//!
//! 这里只算「下一步该是多少」，不碰信号与帧回调（那是 `crates/app` 的 `after_paint::grow_cap`）。
//! 单独成纯函数是为了钉住一条性质：**不管步长、目标取什么值，反复套用必然在有限步内结束**。
//! 渐进展开挂在帧回调上，一旦某种取值让它不收敛，就是每帧空转的后台循环 ——
//! 页面隐藏时帧回调还会同步执行，空转直接变成无终止的递归。

/// 一步的结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrowStep {
  /// 把上限放宽到这个值，下一帧再来。
  Next(usize),
  /// 已经放到目标：撤掉上限，不再需要下一帧。
  Done,
}

/// 当前上限 `cur`、每帧多放 `step` 项、目标 `target`（通常是「这一页要显示几项」）→ 下一步。
///
/// `step` 为 0 按 1 算：步长为 0 永远到不了目标，等于没有终止条件。
/// `cur` 已经不小于 `target` 时直接 [`GrowStep::Done`]，不再多放一帧。
pub fn next_cap(cur: usize, step: usize, target: usize) -> GrowStep {
  let next = cur.saturating_add(step.max(1));
  if next >= target {
    GrowStep::Done
  } else {
    GrowStep::Next(next)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 从 `cur` 起反复套用 `next_cap`，返回走了几步结束；超过 `limit` 步还没结束返回 `None`。
  fn steps_to_finish(mut cur: usize, step: usize, target: usize, limit: usize) -> Option<usize> {
    for n in 1..=limit {
      match next_cap(cur, step, target) {
        GrowStep::Done => return Some(n),
        GrowStep::Next(next) => {
          // 每一步必须真的往前走，否则不收敛。
          assert!(next > cur, "cur={cur} step={step} target={target}");
          cur = next;
        }
      }
    }
    None
  }

  #[test]
  fn grows_by_step_until_target() {
    assert_eq!(next_cap(8, 8, 40), GrowStep::Next(16));
    assert_eq!(next_cap(16, 8, 40), GrowStep::Next(24));
    assert_eq!(next_cap(32, 8, 40), GrowStep::Done);
  }

  #[test]
  fn reaching_the_target_exactly_is_done() {
    // 恰好放到目标那一步就收工，不再多排一帧。
    assert_eq!(next_cap(34, 6, 40), GrowStep::Done);
    assert_eq!(next_cap(33, 6, 40), GrowStep::Next(39));
  }

  #[test]
  fn already_past_the_target_is_done() {
    assert_eq!(next_cap(40, 8, 40), GrowStep::Done);
    assert_eq!(next_cap(100, 8, 40), GrowStep::Done);
    // 目标为 0（列表空）也立刻收工，而不是等一个永远不来的数据。
    assert_eq!(next_cap(4, 6, 0), GrowStep::Done);
  }

  #[test]
  fn zero_step_still_makes_progress() {
    assert_eq!(next_cap(4, 0, 40), GrowStep::Next(5));
    assert!(steps_to_finish(4, 0, 40, 100).is_some());
  }

  #[test]
  fn never_overflows() {
    assert_eq!(next_cap(usize::MAX, 8, usize::MAX), GrowStep::Done);
    assert_eq!(next_cap(usize::MAX - 1, 8, usize::MAX), GrowStep::Done);
  }

  #[test]
  fn always_terminates_within_a_bounded_number_of_steps() {
    // 穷举一片参数：任意起点、步长（含 0）、目标，步数不超过 ⌈(target − cur) / max(step, 1)⌉。
    for cur in [0usize, 1, 4, 8, 39, 40, 41, 1000] {
      for step in [0usize, 1, 2, 6, 8, 16, 1000] {
        for target in [0usize, 1, 8, 40, 60, 1100] {
          let bound = target.saturating_sub(cur).div_ceil(step.max(1)).max(1);
          let n = steps_to_finish(cur, step, target, bound + 1)
            .unwrap_or_else(|| panic!("不收敛：cur={cur} step={step} target={target}"));
          assert!(
            n <= bound,
            "cur={cur} step={step} target={target} n={n} bound={bound}"
          );
        }
      }
    }
  }
}
