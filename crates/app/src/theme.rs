//! 明暗主题：`light` / `dark` / `system`，持久化到 `localStorage["theme"]`。

use ham_exam_core::saved_state::keys;
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
