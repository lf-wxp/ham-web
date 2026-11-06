//! 现代风设置面板：只放「界面风格」与「明暗模式」两项。
//!
//! 与像素风的 `SettingsDialog` 是两份实现（这是双设计系统的正常状态，不是重复代码）：
//! 像素版是居中对话框 + 配色方案九宫格，现代版是右侧抽屉 + 两行分段选择。
//! 迁移期这里刻意只保留两项 —— 其余偏好（像素动效 / 易读字体 / 配色方案）属于像素风，
//! 在现代风里没有意义；数据备份入口在页脚与移动端菜单里都有。

use super::backdrop::Backdrop;
use crate::focus::use_escape_close;
use crate::presence::use_presence;

use ham_web_core::ui_style::UiStyle;
use leptos::portal::Portal;
use leptos::prelude::*;

use crate::i18n::t;
use crate::theme::{Theme, style_defaults_theme, use_display_prefs, use_theme};

#[component]
pub fn ModernSettings(open: RwSignal<bool>) -> impl IntoView {
  let prefs = use_display_prefs();
  let theme = use_theme();
  let presence = use_presence(open);
  let backdrop = Backdrop::new(open);
  use_escape_close(open);
  crate::overlay::use_scroll_lock(open);
  crate::focus::use_modal_focus(open);

  view! {
    {move || {
      presence.visible.get().then(|| {
        view! {
          <Portal>
            <div
              class="md-sheet"
              data-state=presence.state()
              inert=presence.inert()
              aria-hidden=presence.aria_hidden()
              on:pointerdown=backdrop.on_pointerdown()
              on:click=backdrop.on_click()
            >
              <div
                class="md-sheet-panel"
                data-modal-root=""
                tabindex="-1"
                role="dialog"
                aria-modal="true"
                aria-label=move || t("shell.settings")
              >
                <div class="md-sheet-head">
                  <h2 class="md-h2">{move || t("shell.settings")}</h2>
                  <button
                    type="button"
                    class="md-text-btn"
                    aria-label=move || t("shell.close-settings")
                    on:click=move |_| open.set(false)
                  >
                    {move || t("shell.close-settings")}
                  </button>
                </div>

                <div class="md-sheet-body">
                  <p class="md-group-title">{move || t("shell.settings-style")}</p>
                  <div class="md-choices">
                    {UiStyle::ALL
                      .iter()
                      .map(|&style| {
                        let selected = move || prefs.ui_style() == style;
                        view! {
                          <button
                            type="button"
                            class="md-choice"
                            aria-pressed=move || selected().to_string()
                            on:click=move |_| {
                              prefs.set_ui_style(style);
                              if let Some(mode) = style_defaults_theme(style) {
                                theme.set(mode);
                              }
                            }
                          >
                            <span class="md-choice-title">
                              {if style.is_modern() {
                                t("shell.style-modern")
                              } else {
                                t("shell.style-pixel")
                              }}
                            </span>
                          </button>
                        }
                      })
                      .collect_view()}
                  </div>
                  <p class="md-muted md-sm mt-2">{move || t("shell.settings-style-hint")}</p>

                  <p class="md-group-title mt-4">{move || t("shell.settings-theme")}</p>
                  <div class="md-choices md-choices-3">
                    {[Theme::Light, Theme::Dark, Theme::System]
                      .iter()
                      .map(|&mode| {
                        let selected = move || theme.mode() == mode;
                        view! {
                          <button
                            type="button"
                            class="md-choice"
                            aria-pressed=move || selected().to_string()
                            on:click=move |_| theme.set(mode)
                          >
                            <span class="md-choice-title">
                              {match mode {
                                Theme::Light => t("shell.theme-light"),
                                Theme::Dark => t("shell.theme-dark"),
                                Theme::System => t("shell.theme-system"),
                              }}
                            </span>
                          </button>
                        }
                      })
                      .collect_view()}
                  </div>
                </div>
              </div>
            </div>
          </Portal>
        }
      })
    }}
  }
}
