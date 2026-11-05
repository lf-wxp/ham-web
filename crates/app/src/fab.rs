//! 页内菜单悬浮按钮（FAB）共用的那一点：视口尺寸、位置收拢、位置持久化。
//!
//! 两套皮肤各有一颗「页内菜单」悬浮按钮（像素 `ui::MobileNavDrawer`、现代
//! `kit_modern::ModernMenuDrawer`），都可以按住拖动改位置。拖动的**数学**在
//! `ham_web_core::fab`；位置的口径（收拢进视口、存在哪个 key）也必须是同一份 ——
//! 同一个用户在同一个浏览器里换皮肤，按钮不该跳到别处去。
//!
//! 两边的定位方式不同：像素风一开始就是 `left / top` 像素定位，现代风默认交给 CSS
//! （右下角、含 `safe-area-inset`），用户拖过一次之后才换成像素定位。所以这里读位置
//! 返回 `Option`：`None` 就是「还没拖过，用 CSS 的默认位置」。

use ham_web_core::fab::parse_pos;

use crate::util::{storage, window};

/// 位置持久化的 key。两套皮肤共用；设备专属、不进同步（见 `docs/reference/storage.md`）。
pub const POS_KEY: &str = "fab-position";

/// 视口尺寸（读不到时为 0，收拢逻辑会退化成「钉在原点」）。
pub fn viewport() -> (f64, f64) {
  let w = window();
  let width = w.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
  let height = w
    .inner_height()
    .ok()
    .and_then(|v| v.as_f64())
    .unwrap_or(0.0);
  (width, height)
}

/// 把位置收拢进当前视口（旋转 / 换设备后旧位置可能出界）。
///
/// `size` 是按钮边长：位置记的是左上角，右 / 下边界要减去按钮自身。
#[must_use]
pub fn clamp_to_viewport(pos: (f64, f64), size: f64) -> (f64, f64) {
  let (vw, vh) = viewport();
  let max_x = (vw - size).max(0.0);
  let max_y = (vh - size).max(0.0);
  (pos.0.clamp(0.0, max_x), pos.1.clamp(0.0, max_y))
}

/// 读回持久化的位置（已收拢进当前视口）；没有存过、或存坏了 → `None`。
#[must_use]
pub fn load_pos(size: f64) -> Option<(f64, f64)> {
  let pos = parse_pos(&storage::get(POS_KEY)?)?;
  Some(clamp_to_viewport(pos, size))
}

/// 写回位置。只在拖动结束时调一次，不要每帧写。
pub fn save_pos(pos: (f64, f64)) {
  storage::set(POS_KEY, &format!("{},{}", pos.0, pos.1));
}
