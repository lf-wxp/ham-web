//! 触屏左右滑动手势（练习 / 考试切题、闪卡自评）。

use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// 水平位移至少这么多像素才算滑动。
const MIN_DX: f64 = 60.0;
/// 垂直位移不超过水平位移的这个比例，避免与页面上下滚动冲突。
const MAX_SLOPE: f64 = 0.6;
/// 超过这么久（毫秒）的拖动不算滑动。
const MAX_MS: f64 = 700.0;

/// 滑动方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Swipe {
  /// 手指向左划（通常表示「下一个」）。
  Left,
  /// 手指向右划（通常表示「上一个」）。
  Right,
}

#[derive(Clone, Copy)]
struct Start {
  x: f64,
  y: f64,
  t: f64,
}

/// 起点在输入控件或可横向滚动的区域内时不识别滑动，以免抢走控件自身的拖动。
fn starts_in_scrollable(target: Option<web_sys::EventTarget>) -> bool {
  let mut el = target.and_then(|t| t.dyn_into::<web_sys::Element>().ok());
  while let Some(e) = el {
    let tag = e.tag_name().to_lowercase();
    if matches!(tag.as_str(), "input" | "textarea" | "select") {
      return true;
    }
    if e.scroll_width() > e.client_width() + 1
      && let Ok(Some(style)) = crate::util::window().get_computed_style(&e)
      && matches!(
        style.get_property_value("overflow-x").as_deref(),
        Ok("auto" | "scroll")
      )
    {
      return true;
    }
    el = e.parent_element();
  }
  false
}

fn point(e: &web_sys::TouchEvent, changed: bool) -> Option<(f64, f64)> {
  let list = if changed {
    e.changed_touches()
  } else {
    e.touches()
  };
  let t = list.get(0)?;
  Some((f64::from(t.client_x()), f64::from(t.client_y())))
}

/// 返回 `(on:touchstart, on:touchend)` 处理函数；识别到滑动时轻微振动并调用 `on_swipe`。
pub fn swipe_handlers(
  on_swipe: Callback<Swipe>,
) -> (
  impl Fn(web_sys::TouchEvent) + Copy + 'static,
  impl Fn(web_sys::TouchEvent) + Copy + 'static,
) {
  let start = StoredValue::new(None::<Start>);
  let on_start = move |e: web_sys::TouchEvent| {
    if e.touches().length() != 1 || starts_in_scrollable(e.target()) {
      start.set_value(None);
      return;
    }
    start.set_value(point(&e, false).map(|(x, y)| Start {
      x,
      y,
      t: js_sys::Date::now(),
    }));
  };
  let on_end = move |e: web_sys::TouchEvent| {
    let Some(s) = start.get_value() else { return };
    start.set_value(None);
    let Some((x, y)) = point(&e, true) else {
      return;
    };
    let (dx, dy) = (x - s.x, y - s.y);
    if dx.abs() < MIN_DX || dy.abs() > dx.abs() * MAX_SLOPE || js_sys::Date::now() - s.t > MAX_MS {
      return;
    }
    let _ = crate::util::window().navigator().vibrate_with_duration(10);
    on_swipe.run(if dx < 0.0 { Swipe::Left } else { Swipe::Right });
  };
  (on_start, on_end)
}
