//! 明暗主题：`light` / `dark` / `system`，持久化到 `localStorage["theme"]`。

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
  fn parse(s: &str) -> Self {
    match s {
      "light" => Self::Light,
      "dark" => Self::Dark,
      _ => Self::System,
    }
  }

  const fn as_str(self) -> &'static str {
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

/// 初始化主题并提供上下文。
pub fn provide_theme() {
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
