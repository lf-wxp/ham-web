//! 带标签的数值输入框（NEC 页面的几何与材料参数都用它）。

use leptos::prelude::*;

use crate::ui::{ControlSize, NumberField};

use super::NOTE;

/// 带标签的数值输入框。`on_input` 在写回自身信号之后额外调用一次（用于「改参数
/// 就按预设重建几何」这类联动）。
#[component]
pub(super) fn NumField(
  label: Signal<String>,
  value: RwSignal<String>,
  step: f64,
  min: f64,
  max: f64,
  #[prop(optional_no_strip)] on_input: Option<Callback<String, ()>>,
) -> impl IntoView {
  view! {
    <label class="flex flex-col gap-1.5">
      <span class=NOTE>{move || label.get()}</span>
      <NumberField
        value=Signal::derive(move || value.get())
        on_change=Callback::new(move |v: String| {
          value.set(v.clone());
          if let Some(cb) = on_input {
            cb.run(v);
          }
        })
        step=step
        min=min
        max=max
        size=ControlSize::Sm
        aria_label=label
      />
    </label>
  }
}
