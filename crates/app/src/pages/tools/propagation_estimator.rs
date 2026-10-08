use ham_web_core::muf::{estimate_fof2, estimate_muf, estimate_owf};
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

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

  let sfi_id = unique_id("propagation-sfi");
  let k_id = unique_id("propagation-k");

  view! {
    <div class="space-y-3">
      <div class="grid gap-3 sm:grid-cols-2">
        <Field label=Signal::derive(move || t("tools.solar-flux-sfi")) r#for=sfi_id.clone()>
          <NumberField
            id=sfi_id
            value=Signal::derive(move || sfi.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                sfi.set(v);
              }
            })
            controls=false
          />
        </Field>
        <Field label=Signal::derive(move || t("tools.k-index-0-9")) r#for=k_id.clone()>
          <NumberField
            id=k_id
            step=1.0
            value=Signal::derive(move || k.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                k.set(v);
              }
            })
            controls=false
          />
        </Field>
      </div>
      <div class="rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let s = sfi.get();
          let kk = k.get();
          let quality = match kk {
            x if x < 2.0 => t("tools.quiet"),
            x if x < 4.0 => t("tools.active"),
            x if x < 6.0 => t("tools.disturbed"),
            _ => t("tools.storm"),
          };
          tf(
            "tools.fof2-mhz-single-hop",
            &[
              &format!("{:.1}", estimate_fof2(s)),
              &format!("{:.1}", estimate_muf(s)),
              &format!("{:.1}", estimate_owf(s)),
              &format!("{kk:.0}"),
              &quality,
            ],
          )
        }}
      </div>
      <div>
        <div class="mb-1.5 text-xs font-medium text-muted-foreground">{move || t("tools.likely-usable-bands-centre")}</div>
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
