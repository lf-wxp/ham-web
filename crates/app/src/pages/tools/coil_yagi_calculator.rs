use leptos::prelude::*;

use super::{INPUT, fmt_num};

/// 线圈电感与 Yagi 振子计算。
#[component]
pub(super) fn CoilYagiCalculator() -> impl IntoView {
  let coil_d = RwSignal::new(2.0);
  let coil_l = RwSignal::new(2.0);
  let coil_n = RwSignal::new(10.0);
  let freq = RwSignal::new(14.2);

  view! {
    <div class="space-y-4">
      <div>
        <div class="mb-1.5 text-xs font-medium text-muted-foreground">"空心线圈电感（Wheeler 近似）"</div>
        <div class="grid gap-3 sm:grid-cols-3">
          <label class="flex flex-col gap-1.5 text-sm">
            <span class="text-xs text-muted-foreground">"直径 D（cm）"</span>
            <input
              type="number"
              step="0.1"
              prop:value=move || coil_d.get().to_string()
              on:input=move |e| {
                if let Ok(v) = event_target_value(&e).parse::<f64>() {
                  coil_d.set(v);
                }
              }
              class=INPUT
            />
          </label>
          <label class="flex flex-col gap-1.5 text-sm">
            <span class="text-xs text-muted-foreground">"长度 l（cm）"</span>
            <input
              type="number"
              step="0.1"
              prop:value=move || coil_l.get().to_string()
              on:input=move |e| {
                if let Ok(v) = event_target_value(&e).parse::<f64>() {
                  coil_l.set(v);
                }
              }
              class=INPUT
            />
          </label>
          <label class="flex flex-col gap-1.5 text-sm">
            <span class="text-xs text-muted-foreground">"匝数 N"</span>
            <input
              type="number"
              step="1"
              prop:value=move || coil_n.get().to_string()
              on:input=move |e| {
                if let Ok(v) = event_target_value(&e).parse::<f64>() {
                  coil_n.set(v);
                }
              }
              class=INPUT
            />
          </label>
        </div>
        <div class="mt-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums">
          {move || {
            let d = coil_d.get();
            let l = coil_l.get();
            let n = coil_n.get();
            if d <= 0.0 || n <= 0.0 {
              "请输入正的直径与匝数".to_owned()
            } else {
              let denom = 45.4 * d + 100.0 * l;
              let ind = if denom > 0.0 { d * d * n * n / denom } else { 0.0 };
              format!("电感 ≈ {} μH", fmt_num(ind))
            }
          }}
        </div>
      </div>
      <div>
        <div class="mb-1.5 text-xs font-medium text-muted-foreground">"Yagi 振子长度（3 单元近似，米）"</div>
        <div class="grid gap-3 sm:grid-cols-2">
          <label class="flex flex-col gap-1.5 text-sm">
            <span class="text-xs text-muted-foreground">"频率（MHz）"</span>
            <input
              type="number"
              prop:value=move || freq.get().to_string()
              on:input=move |e| {
                if let Ok(v) = event_target_value(&e).parse::<f64>() {
                  freq.set(v);
                }
              }
              class=INPUT
            />
          </label>
        </div>
        <div class="mt-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums">
          {move || {
            let f = freq.get();
            if f <= 0.0 {
              "请输入正频率".to_owned()
            } else {
              format!(
                "反射器 ≈ {} m　激励振子 ≈ {} m　引向器 ≈ {} m",
                fmt_num(150.0 / f),
                fmt_num(143.0 / f),
                fmt_num(136.0 / f),
              )
            }
          }}
        </div>
      </div>
    </div>
  }
}
