//! 预设结构按钮：`ChipGroup` 里的一项。
//!
//! 只是一个视图构造器：选中态由 [`ChipGroup`](crate::ui::ChipGroup) 按 `Preset::key()` 比对，
//! 不需要本地状态。
//! `t()` 的字面量必须写在调用点上，静态扫描才认得出来。

use leptos::prelude::*;

use crate::i18n::t;
use crate::ui::Chip;

use super::Preset;

/// 预设按钮。
pub(super) fn chip(value: Preset) -> impl IntoView {
  let label = match value {
    Preset::Dipole => view! { {move || t("tools.nec-horizontal-dipole")} }.into_any(),
    Preset::Vertical => view! { {move || t("tools.nec-quarter-wave-vertical")} }.into_any(),
    Preset::Yagi2 => view! { {move || t("tools.nec-two-element-yagi")} }.into_any(),
    Preset::Yagi3 => view! { {move || t("tools.nec-three-element-yagi")} }.into_any(),
    Preset::InvertedV => view! { {move || t("tools.nec-inverted-v")} }.into_any(),
  };
  view! {
    <Chip value=value.key().to_owned()>{label}</Chip>
  }
}
