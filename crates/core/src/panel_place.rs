//! 浮层（下拉 / 日历 / 时间面板）相对触发器的落位计算。
//!
//! 为什么放在 core：这段判断是纯算术，出错的表现却很隐蔽 ——「面板伸出屏幕，
//! 用户以为没有选项」。放在这里可以用单测把四种边界（下方不够、上方不够、
//! 贴右边缘、面板比视口还高）逐条钉死，而不是等 e2e 在某个视口尺寸上碰运气。
//!
//! 坐标系与 `DOMRect` 一致：原点在**视口**左上角，单位为 CSS 像素。
//! 调用方用 `position: fixed` 定位面板，于是这里算出来的值可以直接写进 `left` / `top`，
//! 不受页面滚动与祖先 `overflow` 影响。

/// 视口里的矩形（与 `DOMRect` 同口径，原点在视口左上角）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
  pub left: f64,
  pub top: f64,
  pub width: f64,
  pub height: f64,
}

impl Rect {
  #[must_use]
  pub fn right(self) -> f64 {
    self.left + self.width
  }

  #[must_use]
  pub fn bottom(self) -> f64 {
    self.top + self.height
  }
}

/// 面板落在触发器的哪一侧。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
  Below,
  Above,
}

/// 落位结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
  pub left: f64,
  pub top: f64,
  pub side: Side,
}

/// 触发器与面板之间的间距（与 `.pxl-popover` 的 `mt-2` 同值）。
pub const GAP: f64 = 8.0;

/// 面板与视口边缘的最小留白。
pub const MARGIN: f64 = 8.0;

/// 算出面板的 `left` / `top` 与朝向。
///
/// 规则（按优先级）：
/// 1. 下方放得下就放下面；
/// 2. 否则上方放得下就翻到上面；
/// 3. 都放不下时选空间更大的一侧，再把 `top` 钳进视口；
/// 4. 水平方向左对齐触发器，再钳进视口（右侧空间不够时向左收）。
///
/// `panel_w` / `panel_h` 是面板的**实际**尺寸（调用方先量再算）；
/// 传 0 时退化成「只算 left」的退化情况，由调用方避免。
#[must_use]
pub fn place(
  anchor: Rect,
  panel_w: f64,
  panel_h: f64,
  viewport_w: f64,
  viewport_h: f64,
) -> Placement {
  let room_below = viewport_h - anchor.bottom() - GAP - MARGIN;
  let room_above = anchor.top - GAP - MARGIN;
  let below_fits = panel_h <= room_below;
  let above_fits = panel_h <= room_above;

  // 下方放得下就向下；否则只要上方放得下、或上方空间更大，就翻上去。
  let side = if below_fits {
    Side::Below
  } else if above_fits || room_above > room_below {
    Side::Above
  } else {
    Side::Below
  };

  let raw_top = match side {
    Side::Below => anchor.bottom() + GAP,
    Side::Above => anchor.top - GAP - panel_h,
  };
  let max_top = (viewport_h - panel_h - MARGIN).max(MARGIN);
  let top = raw_top.clamp(MARGIN, max_top);

  let max_left = (viewport_w - panel_w - MARGIN).max(MARGIN);
  let left = anchor.left.clamp(MARGIN, max_left);

  Placement { left, top, side }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn anchor(left: f64, top: f64, width: f64, height: f64) -> Rect {
    Rect {
      left,
      top,
      width,
      height,
    }
  }

  #[test]
  fn below_when_there_is_room() {
    let p = place(
      anchor(100.0, 100.0, 200.0, 40.0),
      200.0,
      300.0,
      1200.0,
      800.0,
    );
    assert_eq!(p.side, Side::Below);
    assert!((p.left - 100.0).abs() < f64::EPSILON);
    // 100 + 40 + 8
    assert!((p.top - 148.0).abs() < f64::EPSILON);
  }

  #[test]
  fn flips_above_near_the_bottom() {
    // 触发器在 800 高的视口底部，下方只剩 32px，放不下 300 高的面板。
    let p = place(
      anchor(100.0, 700.0, 200.0, 40.0),
      200.0,
      300.0,
      1200.0,
      800.0,
    );
    assert_eq!(p.side, Side::Above);
    // 700 - 8 - 300
    assert!((p.top - 392.0).abs() < f64::EPSILON);
  }

  #[test]
  fn picks_the_larger_side_when_neither_fits() {
    // 上下都放不下：触发器居中，上下各约 380px；面板 700px。
    let p = place(
      anchor(100.0, 380.0, 200.0, 40.0),
      200.0,
      700.0,
      1200.0,
      800.0,
    );
    // 上方空间 364、下方空间 364 —— 相等时保持向下；428 会被钳到
    // `800 - 700 - 8 = 92`（面板本身就比视口高，只能贴住上下留白）。
    assert_eq!(p.side, Side::Below);
    assert!((p.top - 92.0).abs() < f64::EPSILON);
  }

  #[test]
  fn clamps_horizontally_near_the_right_edge() {
    // 触发器贴右边缘，面板同宽：left 被收紧到 1200 - 200 - 8。
    let p = place(
      anchor(1100.0, 100.0, 200.0, 40.0),
      200.0,
      100.0,
      1200.0,
      800.0,
    );
    assert!((p.left - 992.0).abs() < f64::EPSILON);
  }

  #[test]
  fn never_goes_negative_on_a_narrow_viewport() {
    // 320 宽的视口、300 宽的面板：左右留白各 8，left 夹在 [8, 12]。
    let p = place(anchor(0.0, 100.0, 300.0, 40.0), 300.0, 200.0, 320.0, 600.0);
    assert!(p.left >= MARGIN);
    assert!(p.left <= 320.0 - 300.0 - MARGIN + f64::EPSILON);
  }

  #[test]
  fn a_panel_taller_than_the_viewport_starts_at_the_margin() {
    let p = place(anchor(10.0, 10.0, 100.0, 20.0), 100.0, 900.0, 400.0, 600.0);
    assert!((p.top - MARGIN).abs() < f64::EPSILON);
    assert_eq!(p.side, Side::Below);
  }
}
