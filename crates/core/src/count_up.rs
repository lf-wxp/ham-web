//! 数字滚动的缓动：把「第 i 步该显示几」做成可测的纯函数。
//!
//! 放在 core 的原因：现代风的数字滚动曾用未归一化的 expo-out（`1 - 2^(-10t)`），`t = 1` 时
//! 只到 0.99902，末步显示值比目标少 —— 变化量 ≥ 512 就差 1 以上（XP 5000 会永久停在 4995）。
//! 「最后一步恒等于目标」这条不变量必须有单测守着，而不是靠肉眼看动画。

/// expo-out 缓动，**归一化到 `t = 1` 时恰为 1**。
///
/// 前 1/3 段跑掉八成行程、收尾拖得很慢，数字是「落」下来而不是「跳」过去。
fn expo_out(t: f64) -> f64 {
  let t = t.clamp(0.0, 1.0);
  let tail = (-10.0_f64).exp2();
  (1.0 - (-10.0 * t).exp2()) / (1.0 - tail)
}

/// 从 `from` 滚到 `target` 的第 `step` 步（共 `steps` 步）应显示的值。
///
/// - `step >= steps`（含 `steps == 0`）一律返回 `target`：末步不依赖浮点舍入；
/// - 结果夹在 `[min(from, target), max(from, target)]` 内，舍入误差不会让数字短暂冲出起止范围。
#[must_use]
pub fn count_step(from: usize, target: usize, step: u32, steps: u32) -> usize {
  if steps == 0 || step >= steps {
    return target;
  }
  let t = f64::from(step) / f64::from(steps);
  let value = from as f64 + (target as f64 - from as f64) * expo_out(t);
  let (lo, hi) = (from.min(target), from.max(target));
  (value.round().max(0.0) as usize).clamp(lo, hi)
}

#[cfg(test)]
mod tests {
  use super::count_step;

  const STEPS: u32 = 16;

  #[test]
  fn last_step_is_exactly_the_target() {
    // 回归：未归一化时 0 → 5000 的末步是 4995。
    for (from, target) in [
      (0, 5000),
      (0, 513),
      (5000, 0),
      (120, 119_999),
      (7, 7),
      (3, 4),
    ] {
      assert_eq!(
        count_step(from, target, STEPS, STEPS),
        target,
        "{from} → {target}"
      );
    }
  }

  #[test]
  fn starts_at_the_origin_and_never_leaves_the_range() {
    for (from, target) in [(0, 5000), (5000, 0), (10, 11), (11, 10)] {
      assert_eq!(count_step(from, target, 0, STEPS), from);
      let (lo, hi) = (from.min(target), from.max(target));
      for step in 0..=STEPS {
        let v = count_step(from, target, step, STEPS);
        assert!(
          (lo..=hi).contains(&v),
          "{from} → {target} 第 {step} 步得到 {v}"
        );
      }
    }
  }

  #[test]
  fn rising_sequence_is_monotonic_and_front_loaded() {
    let seq: Vec<usize> = (0..=STEPS).map(|s| count_step(0, 1000, s, STEPS)).collect();
    assert!(seq.windows(2).all(|w| w[0] <= w[1]), "应单调不减：{seq:?}");
    // expo-out：前 1/3 段就跑完大半行程，这是「有惯性」手感的来源。
    assert!(
      seq[STEPS as usize / 3] > 700,
      "前 1/3 段只到 {}",
      seq[STEPS as usize / 3]
    );
  }

  #[test]
  fn falling_sequence_is_monotonic() {
    let seq: Vec<usize> = (0..=STEPS).map(|s| count_step(1000, 0, s, STEPS)).collect();
    assert!(seq.windows(2).all(|w| w[0] >= w[1]), "应单调不增：{seq:?}");
  }

  #[test]
  fn zero_steps_jump_straight_to_the_target() {
    assert_eq!(count_step(3, 9, 0, 0), 9);
  }
}
