use leptos::prelude::*;

use super::{INPUT, RESULT, fmt_num};

/// 电池续航估算：续航 = 容量 / 电流。
#[component]
pub(super) fn BatteryRuntime() -> impl IntoView {
  let capacity = RwSignal::new(2000.0);
  let current = RwSignal::new(500.0);

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">"电池容量（mAh）"</span>
        <input
          type="number"
          prop:value=move || capacity.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              capacity.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">"设备电流（mA）"</span>
        <input
          type="number"
          prop:value=move || current.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              current.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <div class=RESULT>
        {move || {
          let i = current.get();
          if i <= 0.0 {
            "请输入正电流".to_owned()
          } else {
            format!("续航 ≈ {} 小时", fmt_num(capacity.get() / i))
          }
        }}
      </div>
    </div>
  }
}
