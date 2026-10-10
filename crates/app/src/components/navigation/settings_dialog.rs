//! 外观与动效设置：主题 / 配色方案 / 像素动效 / 易读字体。放在对话框里，由顶栏的齿轮按钮打开。
//!
//! 主题（明暗）与配色方案是正交的两个选择：任何方案都有亮 / 暗两套。
//!
//! 为什么需要「像素动效」与「易读字体」两个开关：
//! - 逐帧动画对前庭敏感的用户不友好；系统「减少动态效果」之外，给一个应用内的开关，
//!   方便只想关掉这个站的动画、又不想改系统设置的人；
//! - 点阵字体在长篇题面与解析里读起来累，切回抗锯齿字体是最直接的可用性兜底。

use ham_web_core::color_scheme::SCHEMES;
use leptos::prelude::*;

use crate::i18n::t;
use crate::theme::{Theme, use_display_prefs, use_theme};
use crate::ui::{Dialog, DialogHeader, DialogTitle, RadioGroup, Switch};

use super::scheme_choice::SchemeChoice;
use super::theme_choice::ThemeChoice;

/// 设置对话框。`open` 由顶栏的设置按钮控制。
#[component]
pub fn SettingsDialog(open: RwSignal<bool>) -> impl IntoView {
  let theme = use_theme();
  let prefs = use_display_prefs();

  let theme_value = Signal::derive(move || theme.mode().as_str().to_owned());
  let scheme_value = Signal::derive(move || prefs.scheme().to_owned());
  let dark = Signal::derive(move || theme.is_dark());

  view! {
    <Dialog open=open class="sm:max-w-md">
      <DialogHeader>
        <DialogTitle>{move || t("shell.settings")}</DialogTitle>
      </DialogHeader>

      <div class="flex min-h-0 flex-col gap-5 overflow-y-auto">
        // 单选组没有单个控件可以挂 `for`：整组靠 `aria_label` 命名，标签文字只是可见的小标题。
        <p class="pxl-label text-xs text-muted-foreground">{move || t("shell.settings-theme")}</p>
        <div>
          <RadioGroup
            value=theme_value
            on_change=Callback::new(move |v: String| theme.set(Theme::parse(&v)))
            aria_label=Signal::derive(move || t("shell.settings-theme"))
            class="grid-cols-3"
          >
            <ThemeChoice value="light" label=Signal::derive(move || t("shell.theme-light")) />
            <ThemeChoice value="dark" label=Signal::derive(move || t("shell.theme-dark")) />
            <ThemeChoice value="system" label=Signal::derive(move || t("shell.theme-system")) />
          </RadioGroup>
        </div>

        <p class="pxl-label text-xs text-muted-foreground">{move || t("shell.scheme")}</p>
        <div class="space-y-2">
          <RadioGroup
            value=scheme_value
            on_change=Callback::new(move |v: String| prefs.set_scheme(&v))
            aria_label=Signal::derive(move || t("shell.scheme"))
            class="grid-cols-2"
          >
            {SCHEMES.iter().map(|s| view! { <SchemeChoice scheme=s dark=dark /> }).collect_view()}
          </RadioGroup>
          <p class="text-xs text-muted-foreground">{move || t("shell.scheme-hint")}</p>
        </div>

        <div class="flex items-start justify-between gap-4">
          <div class="space-y-1">
            <label for="settings-pixel-motion" class="text-sm">
              {move || t("shell.settings-pixel-motion")}
            </label>
            <p class="text-xs text-muted-foreground">
              {move || t("shell.settings-pixel-motion-hint")}
            </p>
          </div>
          <Switch
            id="settings-pixel-motion"
            checked=Signal::derive(move || prefs.pixel_motion())
            on_change=Callback::new(move |on| prefs.set_pixel_motion(on))
          />
        </div>

        <div class="flex items-start justify-between gap-4">
          <div class="space-y-1">
            <label for="settings-readable-font" class="text-sm">
              {move || t("shell.settings-readable-font")}
            </label>
            <p class="text-xs text-muted-foreground">
              {move || t("shell.settings-readable-font-hint")}
            </p>
          </div>
          <Switch
            id="settings-readable-font"
            checked=Signal::derive(move || prefs.readable_font())
            on_change=Callback::new(move |on| prefs.set_readable_font(on))
          />
        </div>
      </div>
    </Dialog>
  }
}
