//! 地面模型与导线材质：直接决定求解器里的 `Ground` 与电导率。

use leptos::prelude::*;

use crate::i18n::t;
use crate::ui::{Chip, ChipGroup};

use super::num_field::NumField;
use super::{MATERIALS, NOTE};

/// 地面与材质选择。有耗地面会多出 εr / σ 两个输入框（其余模型用不到）。
#[component]
pub(super) fn GroundMaterialSection(
  ground_kind: RwSignal<usize>,
  eps_r: RwSignal<String>,
  sigma_ground: RwSignal<String>,
  material: RwSignal<usize>,
) -> impl IntoView {
  view! {
    <div class="space-y-2 rounded-xl border bg-card p-4">
      <div class="flex flex-wrap items-center gap-2">
        <span class=NOTE>{move || t("tools.nec-ground")}</span>
        <ChipGroup
          value=Signal::derive(move || ground_kind.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(n) = v.parse::<usize>() {
              ground_kind.set(n);
            }
          })
          aria_label=Signal::derive(move || t("tools.nec-ground"))
        >
          {(0..3usize)
            .map(|k| {
              // 译文取值函数而非中文原文：key 必须是 `t()` 的字面量实参，否则会被
              // `check-i18n` 判成死条目（见 `pages::waveform_lab::wave_section::kind_text`）。
              let label: fn() -> String = match k {
                1 => || t("tools.nec-ground-perfect"),
                2 => || t("tools.nec-ground-lossy"),
                _ => || t("tools.nec-ground-free"),
              };
              view! { <Chip value=k.to_string()>{move || label()}</Chip> }
            })
            .collect_view()}
        </ChipGroup>
      </div>
      {move || {
        if ground_kind.get() != 2 {
          ().into_any()
        } else {
          view! {
            <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
              <NumField label=Signal::derive(move || t("tools.nec-eps-r"))
                value=eps_r step=1.0 min=1.0 max=100.0 />
              <NumField label=Signal::derive(move || t("tools.nec-sigma-ground"))
                value=sigma_ground step=0.001 min=0.000_001 max=10.0 />
            </div>
          }
          .into_any()
        }
      }}
      <div class="flex flex-wrap items-center gap-2">
        <span class=NOTE>{move || t("tools.nec-material")}</span>
        <ChipGroup
          value=Signal::derive(move || material.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(n) = v.parse::<usize>() {
              material.set(n);
            }
          })
          aria_label=Signal::derive(move || t("tools.nec-material"))
        >
          {MATERIALS
            .iter()
            .enumerate()
            .map(|(k, (label, _))| view! { <Chip value=k.to_string()>{move || label()}</Chip> })
            .collect_view()}
        </ChipGroup>
      </div>
      <p class=NOTE>{move || t("tools.nec-loss-note")}</p>
    </div>
  }
}
