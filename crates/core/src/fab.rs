//! 悬浮导航按钮（FAB）拖动的位置计算。
//!
//! 拖动判定与视口收拢都是纯计算，放这里配单测；Pointer 事件监听与位置持久化
//! 留在前端（`crates/app/src/ui/mobile_nav_drawer.rs`）。

/// 一次拖动手势：记住按下时的指针位置与按钮位置，随指针移动累计**路径长度**并算出新位置。
#[derive(Debug, Clone, Copy)]
pub struct FabDrag {
  /// 按下时的指针位置：新位置 = `origin + (当前指针 − press)`。
  press: (f64, f64),
  /// 上一次指针位置：路径长度按相邻两次移动的增量累计。
  last: (f64, f64),
  origin: (f64, f64),
  dist: f64,
}

impl FabDrag {
  /// `pointer` 是按下时指针的视口坐标，`origin` 是按钮左上角当前位置。
  #[must_use]
  pub const fn new(pointer: (f64, f64), origin: (f64, f64)) -> Self {
    Self {
      press: pointer,
      last: pointer,
      origin,
      dist: 0.0,
    }
  }

  /// 指针移动到 `pointer`；`max` 是按钮左上角的可达范围（视口尺寸减去按钮尺寸）。
  /// 返回收拢进 `[0, max]` 的新位置；本次增量位移计入累计路径。
  ///
  /// 路径按增量累计（回到起点也算走过），位置按按下点算总位移 —— 两者分开记，
  /// 所以「拖出去又拖回来」路径超阈值、位置归位，判定仍是拖动而不是轻点。
  #[must_use]
  pub fn drag_to(&mut self, pointer: (f64, f64), max: (f64, f64)) -> (f64, f64) {
    let dx = pointer.0 - self.last.0;
    let dy = pointer.1 - self.last.1;
    self.dist += dx.hypot(dy);
    self.last = pointer;
    let x = (self.origin.0 + pointer.0 - self.press.0).clamp(0.0, max.0.max(0.0));
    let y = (self.origin.1 + pointer.1 - self.press.1).clamp(0.0, max.1.max(0.0));
    (x, y)
  }

  /// 是否已构成拖动：累计位移超过 `threshold`（像素）。
  #[must_use]
  pub const fn is_drag(self, threshold: f64) -> bool {
    self.dist > threshold
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn small_movement_is_a_tap() {
    let mut d = FabDrag::new((10.0, 20.0), (100.0, 200.0));
    let next = d.drag_to((13.0, 24.0), (300.0, 500.0));
    assert_eq!(next, (103.0, 204.0));
    assert!(!d.is_drag(6.0));
  }

  #[test]
  fn large_movement_is_a_drag() {
    let mut d = FabDrag::new((10.0, 20.0), (100.0, 200.0));
    let _ = d.drag_to((100.0, 20.0), (300.0, 500.0));
    assert!(d.is_drag(6.0));
  }

  #[test]
  fn clamped_into_viewport() {
    let mut d = FabDrag::new((100.0, 100.0), (10.0, 10.0));
    assert_eq!(d.drag_to((0.0, 0.0), (50.0, 40.0)), (0.0, 0.0));
    assert_eq!(d.drag_to((1000.0, 1000.0), (50.0, 40.0)), (50.0, 40.0));
  }

  #[test]
  fn zero_sized_viewport_clamps_to_zero() {
    let mut d = FabDrag::new((0.0, 0.0), (5.0, 5.0));
    assert_eq!(d.drag_to((0.0, 0.0), (-10.0, -10.0)), (0.0, 0.0));
  }

  #[test]
  fn distance_accumulates_across_moves() {
    let mut d = FabDrag::new((0.0, 0.0), (0.0, 0.0));
    let _ = d.drag_to((4.0, 0.0), (100.0, 100.0));
    let _ = d.drag_to((8.0, 0.0), (100.0, 100.0));
    // 累计 4 + 4 = 8，超过 6px 阈值
    assert!(d.is_drag(6.0));
  }

  #[test]
  fn path_accumulates_even_when_returning_to_the_start() {
    // 拖出去又拖回来：位置归位，但路径走了 4 + 4 = 8px，仍是拖动不是轻点。
    let mut d = FabDrag::new((0.0, 0.0), (0.0, 0.0));
    let _ = d.drag_to((4.0, 0.0), (100.0, 100.0));
    let back = d.drag_to((0.0, 0.0), (100.0, 100.0));
    assert_eq!(back, (0.0, 0.0));
    assert!(d.is_drag(6.0));
  }
}
