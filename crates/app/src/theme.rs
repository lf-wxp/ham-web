//! 明暗主题：`light` / `dark` / `system`，持久化到 `localStorage["theme"]`；
//! 以及三个像素风相关的显示偏好：「配色方案」「像素动效」与「易读字体」。
//!
//! 三个偏好都以 `<html>` 上的 `data-*` 属性落地（见 `style/pixel/`），CSS 一处响应，
//! 不需要组件各自订阅信号。首屏前由 `index.html` 的内联脚本先写一遍，避免闪烁。
//!
//! 配色方案与明暗是**正交**的两个选择：方案决定色相（经典 / 森林 / 海洋……），
//! 明暗决定亮度，每个方案都有亮 / 暗两套（见 `ham_web_core::color_scheme`）。

use ham_web_core::color_scheme;
use ham_web_core::saved_state::keys;
use leptos::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

use crate::util::{document, storage, window};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
  Light,
  Dark,
  System,
}

impl Theme {
  pub fn parse(s: &str) -> Self {
    match s {
      "light" => Self::Light,
      "dark" => Self::Dark,
      _ => Self::System,
    }
  }

  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Light => "light",
      Self::Dark => "dark",
      Self::System => "system",
    }
  }
}

/// 主题上下文。
#[derive(Clone, Copy)]
pub struct ThemeCtx {
  theme: RwSignal<Theme>,
  dark: RwSignal<bool>,
}

impl ThemeCtx {
  /// 用户选择的主题模式（含「跟随系统」）。与 [`Self::is_dark`] 不同：后者是实际生效的明暗。
  pub fn mode(self) -> Theme {
    self.theme.get()
  }

  /// 当前实际是否为深色。
  pub fn is_dark(self) -> bool {
    self.dark.get()
  }

  /// 设置并持久化主题。
  pub fn set(self, theme: Theme) {
    storage::set(keys::THEME, theme.as_str());
    self.theme.set(theme);
    self.dark.set(apply(theme));
  }
}

fn system_dark() -> bool {
  window()
    .match_media("(prefers-color-scheme: dark)")
    .ok()
    .flatten()
    .is_some_and(|m| m.matches())
}

fn apply(theme: Theme) -> bool {
  let dark = match theme {
    Theme::Dark => true,
    Theme::Light => false,
    Theme::System => system_dark(),
  };
  if let Some(root) = document().document_element() {
    let _ = root.class_list().toggle_with_force("dark", dark);
    if let Some(el) = root.dyn_ref::<web_sys::HtmlElement>() {
      let _ = el
        .style()
        .set_property("color-scheme", if dark { "dark" } else { "light" });
    }
  }
  dark
}

/// 把「像素动效」偏好写到 `<html data-pixel-motion>`：`off` 时 CSS 把所有动画降为静态帧。
fn apply_pixel_motion(on: bool) {
  if let Some(root) = document().document_element() {
    let _ = root.set_attribute("data-pixel-motion", if on { "on" } else { "off" });
  }
}

/// 把「易读字体」偏好写到 `<html data-readable-font>`：`on` 时正文切回抗锯齿字体。
fn apply_readable_font(on: bool) {
  if let Some(root) = document().document_element() {
    let _ = root.set_attribute("data-readable-font", if on { "on" } else { "off" });
  }
}

/// 把配色方案写到 `<html data-scheme>`：`schemes.css` 里按它选择变量块（经典方案没有块，沿用 `tokens.css`）。
fn apply_scheme(id: &str) {
  if let Some(root) = document().document_element() {
    let _ = root.set_attribute("data-scheme", id);
  }
}

/// 「像素动效」当前是否开启（缺省开启）。
///
/// 直接读存储而不是读信号：`motion::motion_allowed` 在组件树之外也会被调用。
pub(crate) fn pixel_motion_enabled() -> bool {
  storage::get(keys::PIXEL_MOTION).as_deref() != Some("off")
}

/// 显示偏好上下文：像素动效 / 易读字体。
#[derive(Clone, Copy)]
pub struct DisplayPrefs {
  pixel_motion: RwSignal<bool>,
  readable_font: RwSignal<bool>,
  scheme: RwSignal<&'static str>,
}

impl DisplayPrefs {
  /// 当前配色方案的 id（始终是方案表里存在的一个）。
  pub fn scheme(self) -> &'static str {
    self.scheme.get()
  }

  /// 设置并持久化配色方案；不认识的 id 回退默认方案。
  ///
  /// 先改 `<html>` 属性、后写信号：订阅者（如 3D 方向图要重读 `--primary`）被唤醒时，
  /// 读到的已经是新方案的颜色。
  pub fn set_scheme(self, id: &str) {
    let id = color_scheme::resolve(id).id;
    storage::set(keys::COLOR_SCHEME, id);
    apply_scheme(id);
    self.scheme.set(id);
  }

  /// 像素动效是否开启。
  pub fn pixel_motion(self) -> bool {
    self.pixel_motion.get()
  }

  /// 易读字体是否开启。
  pub fn readable_font(self) -> bool {
    self.readable_font.get()
  }

  /// 设置并持久化「像素动效」。
  ///
  /// 已经挂上的滚动浮现观察器不会因此重建；下次刷新或路由切换后才完全生效，
  /// CSS 侧的静态降级则是即时的。
  pub fn set_pixel_motion(self, on: bool) {
    storage::set(keys::PIXEL_MOTION, if on { "on" } else { "off" });
    apply_pixel_motion(on);
    self.pixel_motion.set(on);
  }

  /// 设置并持久化「易读字体」。
  pub fn set_readable_font(self, on: bool) {
    storage::set(keys::READABLE_FONT, if on { "on" } else { "off" });
    apply_readable_font(on);
    self.readable_font.set(on);
  }
}

/// 读取显示偏好上下文。
pub fn use_display_prefs() -> DisplayPrefs {
  expect_context::<DisplayPrefs>()
}

/// 初始化主题并提供上下文。
pub fn provide_theme() {
  let pixel_motion = pixel_motion_enabled();
  let readable_font = storage::get(keys::READABLE_FONT).as_deref() == Some("on");
  let scheme = color_scheme::resolve(&storage::get(keys::COLOR_SCHEME).unwrap_or_default()).id;
  apply_pixel_motion(pixel_motion);
  apply_readable_font(readable_font);
  apply_scheme(scheme);
  provide_context(DisplayPrefs {
    pixel_motion: RwSignal::new(pixel_motion),
    readable_font: RwSignal::new(readable_font),
    scheme: RwSignal::new(scheme),
  });

  let stored = storage::get(keys::THEME).map_or(Theme::System, |s| Theme::parse(&s));
  let ctx = ThemeCtx {
    theme: RwSignal::new(stored),
    dark: RwSignal::new(apply(stored)),
  };

  if let Ok(Some(mql)) = window().match_media("(prefers-color-scheme: dark)") {
    let on_change = Closure::<dyn Fn()>::new(move || {
      if let Some(theme) = ctx.theme.try_get_untracked() {
        ctx.dark.set(apply(theme));
      }
    });
    let _ = mql.add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref());
    let guard = SendWrapper::new((mql, on_change));
    on_cleanup(move || {
      let (mql, cb) = guard.take();
      let _ = mql.remove_event_listener_with_callback("change", cb.as_ref().unchecked_ref());
    });
  }
  provide_context(ctx);
}

pub fn use_theme() -> ThemeCtx {
  expect_context::<ThemeCtx>()
}

/// 读取主题 CSS 变量并解析成 sRGB 分量（`0.0..=1.0`）。
///
/// 借一个 1×1 的 2D canvas 把颜色真正**光栅化**成 sRGB：`fillStyle` 的字符串序列化对
/// 现代色彩空间（`oklch()` / `color(display-p3)` 等）会按原空间返回，直接解析字符串会
/// 失败；而 `getImageData` 读到的是浏览器已完成色彩空间转换后的 sRGB 分量，因此能覆盖
/// 所有 CSS 颜色写法，让 WebGL 画布与页面配色严丝合缝，不依赖硬编码。
///
/// 变量不存在或无法解析时返回 `None`，由调用方回退到静态色值。
#[must_use]
pub fn resolve_theme_color(name: &str) -> Option<(f32, f32, f32)> {
  let root = document().document_element()?;
  let computed = window().get_computed_style(&root).ok()??;
  let raw = computed.get_property_value(name).ok()?;
  let raw = raw.trim();
  if raw.is_empty() {
    return None;
  }
  let probe: web_sys::HtmlCanvasElement =
    document().create_element("canvas").ok()?.dyn_into().ok()?;
  probe.set_width(1);
  probe.set_height(1);
  let ctx: web_sys::CanvasRenderingContext2d = probe.get_context("2d").ok()??.dyn_into().ok()?;
  // 先写入一个哨兵色并取像素：若 `raw` 无法解析，`set_fill_style_str` 会静默忽略，
  // 光栅化出的仍是哨兵色，据此判定失败。
  ctx.set_fill_style_str("#010203");
  ctx.fill_rect(0.0, 0.0, 1.0, 1.0);
  ctx.set_fill_style_str(raw);
  ctx.fill_rect(0.0, 0.0, 1.0, 1.0);
  let px = ctx.get_image_data(0.0, 0.0, 1.0, 1.0).ok()?.data();
  if px.len() < 4 {
    return None;
  }
  let (r, g, b) = (px[0], px[1], px[2]);
  if (r, g, b) == (1, 2, 3) {
    return None;
  }
  Some((
    f32::from(r) / 255.0,
    f32::from(g) / 255.0,
    f32::from(b) / 255.0,
  ))
}
