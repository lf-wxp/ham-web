//! 电台读数与仪表的现代风等价物：16 个展示项 + 5 个联动滑杆。
//!
//! 像素风的 `radio_gallery` 全是 `pxl-*` 组合件，现代风没有这类 DOM；这里按同样的
//! 项目数量列出已登记的 `md-*` 读数 / 刻度 / 波形类，并用共享 `Slider` 演示联动。

use leptos::prelude::*;

use crate::components::common::SectionCard;
use crate::i18n::{t, tf};
use crate::kit_modern::ModernIcon;
use crate::ui::Slider;

use super::data::MD_RADIO_ITEMS;

#[component]
pub fn ModernRadio() -> impl IntoView {
  // 五个联动读数对应像素风那五个 Slider（units / bars / charge / cells / strength）。
  // 单位是百分比：共享 `Slider` 默认就是 0–100，两者同量纲，旋钮位置 / 读数 / 进度条才对得上。
  let levels = RwSignal::new([30.0_f64, 60.0, 45.0, 80.0, 20.0]);

  view! {
    <SectionCard title=Signal::derive(move || t("design.comp-tab-radio"))>
      <div class="space-y-4 p-4">
        <p class="md-muted md-sm">{move || t("design.md-radio-hint")}</p>
        <div data-spec-list="radio-items" class="grid grid-cols-2 gap-2 sm:grid-cols-4">
          <For each=move || MD_RADIO_ITEMS key=|row| row.0 let:row>
            <div class=crate::kit::tile_class("flex items-center gap-2 px-3 py-2")>
              <ModernIcon kind=row.1 />
              <code class="md-muted md-sm">{row.0}</code>
            </div>
          </For>
        </div>
        <div data-spec-list="radio-levels" class="grid gap-3 sm:grid-cols-2">
          <For each=move || 0..5usize key=|i| *i let:i>
            <div class=crate::kit::tile_class("space-y-2 p-3")>
              <div class="flex items-center justify-between gap-2 md-sm">
                <code class="md-muted md-sm">
                  {move || tf("design.md-radio-level", &[&(i + 1).to_string()])}
                </code>
                <span>{move || format!("{:.0}%", levels.with(|v| v[i]))}</span>
              </div>
              <Slider
                value=Signal::derive(move || levels.with(|v| v[i]))
                on_change=Callback::new(move |v: f64| {
                  levels.update(|all| all[i] = v.clamp(0.0, 100.0));
                })
                aria_label=Signal::derive(move || {
                  format!("{} {}", t("design.comp-slider-label"), i + 1)
                })
                aria_valuetext=Signal::derive(move || format!("{:.0}%", levels.with(|v| v[i])))
              />
              <div class="md-bar-track">
                <div class="md-bar-fill" style=move || format!("width:{:.0}%", levels.with(|v| v[i]))></div>
              </div>
            </div>
          </For>
        </div>
      </div>
    </SectionCard>
  }
}
