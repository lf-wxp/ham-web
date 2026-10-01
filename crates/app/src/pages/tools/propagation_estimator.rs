use ham_web_core::muf::{estimate_fof2, estimate_muf, estimate_owf};
use leptos::prelude::*;

use super::INPUT;

/// 传播预测：SFI + K 指数 → foF2 / MUF / OWF 与可用波段建议。
#[component]
pub(super) fn PropagationEstimator() -> impl IntoView {
  let sfi = RwSignal::new(150.0);
  let k = RwSignal::new(2.0);

  /// 业余波段中心频率（MHz）。
  const BANDS: &[(&str, f64)] = &[
    ("160m", 1.9),
    ("80m", 3.7),
    ("40m", 7.15),
    ("30m", 10.1),
    ("20m", 14.2),
    ("17m", 18.1),
    ("15m", 21.2),
    ("12m", 24.9),
    ("10m", 28.5),
  ];

  view! {
    <div class="space-y-3">
      <div class="grid gap-3 sm:grid-cols-2">
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">"太阳通量 SFI"</span>
          <input
            type="number"
            prop:value=move || sfi.get().to_string()
            on:input=move |e| {
              if let Ok(v) = event_target_value(&e).parse::<f64>() {
                sfi.set(v);
              }
            }
            class=INPUT
          />
        </label>
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">"K 指数（0–9）"</span>
          <input
            type="number"
            step="1"
            prop:value=move || k.get().to_string()
            on:input=move |e| {
              if let Ok(v) = event_target_value(&e).parse::<f64>() {
                k.set(v);
              }
            }
            class=INPUT
          />
        </label>
      </div>
      <div class="rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let s = sfi.get();
          let kk = k.get();
          let quality = match kk {
            x if x < 2.0 => "磁情安静",
            x if x < 4.0 => "磁情活跃",
            x if x < 6.0 => "磁扰",
            _ => "强磁暴",
          };
          format!(
            "foF2 ≈ {:.1} MHz　单跳 MUF ≈ {:.1} MHz　OWF ≈ {:.1} MHz　（K = {:.0}，{quality}）",
            estimate_fof2(s),
            estimate_muf(s),
            estimate_owf(s),
            kk,
          )
        }}
      </div>
      <div>
        <div class="mb-1.5 text-xs font-medium text-muted-foreground">"预计可用波段（中心频率低于 MUF）"</div>
        <div class="flex flex-wrap gap-1.5">
          {move || {
            let muf = estimate_muf(sfi.get());
            view! {
              {BANDS
                .iter()
                .map(|&(name, f)| {
                  let ok = f <= muf;
                  view! {
                    <span
                      class=if ok {
                        "rounded-full border border-primary/40 bg-primary/10 px-2.5 py-0.5 text-xs font-medium text-foreground"
                      } else {
                        "rounded-full border px-2.5 py-0.5 text-xs text-muted-foreground line-through"
                      }
                    >
                      {name}
                    </span>
                  }
                })
                .collect_view()}
            }
          }}
        </div>
      </div>
    </div>
  }
}
