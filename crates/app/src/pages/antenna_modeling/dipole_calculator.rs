use leptos::prelude::*;

const INPUT: &str = "h-10 rounded-lg border bg-background px-3 text-sm tabular-nums outline-none focus:ring-2 focus:ring-ring/50";

/// 偶极天线尺寸估算计算器。
#[component]
pub(super) fn DipoleCalculator() -> impl IntoView {
  let freq = RwSignal::new(14.2);
  let k = RwSignal::new(0.95);

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">"频率（MHz）"</span>
        <input
          type="number"
          step="0.01"
          prop:value=move || freq.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              freq.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">"缩短系数 k（0.90–0.98）"</span>
        <input
          type="number"
          step="0.01"
          min="0.5"
          max="1.0"
          prop:value=move || k.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              k.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <div class="sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let f = freq.get();
          let kk = k.get().clamp(0.5, 1.0);
          if f <= 0.0 {
            "请输入正频率".to_owned()
          } else {
            let half = 150.0 / f * kk;
            format!(
              "半波偶极总长 ≈ {:.2} m（单臂 {:.2} m）　1/4 波长 ≈ {:.2} m　建议架高 ≈ {:.2} m",
              half,
              half / 2.0,
              75.0 / f * kk,
              75.0 / f,
            )
          }
        }}
      </div>
    </div>
  }
}
