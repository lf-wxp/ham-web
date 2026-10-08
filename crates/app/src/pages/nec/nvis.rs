//! NVIS 仰角-距离覆盖图。
//!
//! 把「仰角 → 一跳落点距离」的几何（`d = 2h·cot Δ`）与天线的仰角切面拼在一起：
//! 横轴是地面距离，纵轴是该方向上能拿到的增益。判断一副天线能不能覆盖 0–500 km
//! （NVIS）看这张图就够了。
//!
//! **只有几何与天线增益**：不含电离层吸收、MUF/LUF、地反射损耗，平地近似、只算一跳。
//! 卡片里如实写着这条边界。

use ham_web_core::nec::{NecResult, coverage_peak, one_hop_coverage, one_hop_distance_km};
use leptos::prelude::*;

use crate::i18n::t;
use crate::ui::{ControlSize, Field, Input};
use crate::util::unique_id;

const W: f64 = 560.0;
const H: f64 = 220.0;
const PAD_L: f64 = 46.0;
const PAD_R: f64 = 12.0;
const PAD_T: f64 = 12.0;
const PAD_B: f64 = 30.0;
/// 纵轴下限（相对峰值）。
const FLOOR_DB: f64 = 30.0;

#[component]
pub(super) fn NvisSection(result: Memo<Option<NecResult>>) -> impl IntoView {
  // 电离层虚高与画到多远：都是用户参数，改它们不重新求解（只重算几何）。
  let layer_km = RwSignal::new(String::from("300"));
  let max_km = RwSignal::new(String::from("2000"));

  let parsed = move |sig: RwSignal<String>, fallback: f64| -> f64 {
    sig
      .get()
      .trim()
      .parse::<f64>()
      .ok()
      .filter(|v| *v > 0.0)
      .unwrap_or(fallback)
  };

  let data = Memo::new(move |_| {
    let r = result.get()?;
    let layer = parsed(layer_km, 300.0);
    let max = parsed(max_km, 2000.0);
    let points = one_hop_coverage(&r, layer, max);
    if points.is_empty() {
      return None;
    }
    let peak = coverage_peak(&points)?;
    Some((points, peak, layer, max))
  });

  // NVIS 常用仰角 60°–90° 对应的距离窗（60° 是 NVIS 的下边界）。
  let nvis_edge = move || {
    let layer = parsed(layer_km, 300.0);
    one_hop_distance_km(60.0, layer).unwrap_or(0.0)
  };

  // `Field` 的标签与控件是兄弟节点，`r#for` / `id` 必须配对才能点击标签聚焦输入框。
  let layer_id = unique_id("nec-nvis-layer");
  let max_id = unique_id("nec-nvis-max");

  view! {
    <div class="space-y-3 rounded-xl border bg-card p-4">
      <div class="flex flex-wrap items-center justify-between gap-2">
        <span class="text-sm font-semibold">{move || t("tools.nec-nvis")}</span>
        <div class="flex flex-wrap items-end gap-2">
          <Field label=Signal::derive(move || t("tools.nec-nvis-layer")) r#for=layer_id.clone()>
            <Input
              id=layer_id.clone()
              value=layer_km
              on_change=Callback::new(move |v: String| layer_km.set(v))
              size=ControlSize::Sm
              aria_label=Signal::derive(move || t("tools.nec-nvis-layer"))
              class="w-20"
            />
          </Field>
          <Field label=Signal::derive(move || t("tools.nec-nvis-max")) r#for=max_id.clone()>
            <Input
              id=max_id.clone()
              value=max_km
              on_change=Callback::new(move |v: String| max_km.set(v))
              size=ControlSize::Sm
              aria_label=Signal::derive(move || t("tools.nec-nvis-max"))
              class="w-20"
            />
          </Field>
        </div>
      </div>

      {move || {
        let Some((points, peak, _layer, max)) = data.get() else {
          return view! {
            <p class="text-xs text-muted-foreground">{move || t("tools.nec-nvis-none")}</p>
          }
            .into_any();
        };
        let lo = peak.gain_dbi - FLOOR_DB;
        let x = |d: f64| PAD_L + (d / max).clamp(0.0, 1.0) * (W - PAD_L - PAD_R);
        let y = |g: f64| {
          PAD_T + ((peak.gain_dbi - g) / FLOOR_DB).clamp(0.0, 1.0) * (H - PAD_T - PAD_B)
        };
        let path: String = points
          .iter()
          .enumerate()
          .map(|(i, p)| {
            format!(
              "{}{:.1} {:.1}",
              if i == 0 { "M" } else { " L" },
              x(p.distance_km),
              y(p.gain_dbi)
            )
          })
          .collect();
        let edge = nvis_edge().min(max);
        let edge_x = x(edge);
        let peak_x = x(peak.distance_km);
        let peak_y = y(peak.gain_dbi);
        let peak_km = peak.distance_km;
        let peak_elev = peak.elevation_deg;
        // 主瓣偏低就不是 NVIS 天线：如实说明，并给出该往哪调。
        let low_lobe = peak_elev < 45.0;
        let tick = |v: f64| format!("{v:.0}");
        view! {
          <div class="overflow-x-auto">
            <svg
              viewBox=format!("0 0 {W} {H}")
              class="w-full min-w-[420px]"
              role="img"
              aria-label=move || t("tools.nec-nvis-alt")
            >
              // NVIS 距离窗（0 – 60° 仰角的落点）
              <rect
                x=PAD_L
                y=PAD_T
                width=(edge_x - PAD_L).max(0.0)
                height=H - PAD_T - PAD_B
                class="fill-primary/10"
              />
              <text x=PAD_L + 4.0 y=PAD_T + 12.0 class="fill-muted-foreground text-[9px]">
                {move || t("tools.nec-nvis-window")}
              </text>
              // 纵轴：峰值与下限两条参考线
              <line
                x1=PAD_L
                y1=PAD_T
                x2=W - PAD_R
                y2=PAD_T
                class="stroke-muted-foreground/30"
                stroke-width="1"
              />
              <line
                x1=PAD_L
                y1=H - PAD_B
                x2=W - PAD_R
                y2=H - PAD_B
                class="stroke-muted-foreground/30"
                stroke-width="1"
              />
              <text x="2" y=PAD_T + 8.0 class="fill-muted-foreground text-[9px]">
                {format!("{:.1} dBi", peak.gain_dbi)}
              </text>
              <text x="2" y=H - PAD_B class="fill-muted-foreground text-[9px]">
                {format!("{:.0} dBi", lo)}
              </text>
              <path d=path class="fill-none stroke-primary" stroke-width="1.6" />
              <circle cx=peak_x cy=peak_y r="3.5" class="fill-primary" />
              <line
                x1=peak_x
                y1=peak_y
                x2=peak_x
                y2=H - PAD_B
                class="stroke-primary/40"
                stroke-width="1"
                stroke-dasharray="3 3"
              />
              <text
                x=(peak_x + 4.0).min(W - 90.0)
                y=PAD_T + 24.0
                class="fill-primary text-[10px]"
              >
                {format!("{peak_km:.0} km / {peak_elev:.0}°")}
              </text>
              // 横轴刻度
              <text x=PAD_L y=H - PAD_B + 14.0 class="fill-muted-foreground text-[9px]">
                "0"
              </text>
              <text
                x=x(max / 2.0) - 10.0
                y=H - PAD_B + 14.0
                class="fill-muted-foreground text-[9px]"
              >
                {tick(max / 2.0)}
              </text>
              <text
                x=W - PAD_R - 30.0
                y=H - PAD_B + 14.0
                class="fill-muted-foreground text-[9px]"
              >
                {format!("{} km", tick(max))}
              </text>
            </svg>
          </div>
          <p class="text-xs text-muted-foreground">
            {move || {
              crate::i18n::tf(
                "tools.nec-nvis-peak",
                &[&format!("{peak_km:.0}"), &format!("{peak_elev:.0}")],
              )
            }}
          </p>
          <p class=if low_lobe { "text-xs text-destructive" } else { "text-xs text-muted-foreground" }>
            {move || {
              if low_lobe {
                tf_low(peak_elev)
              } else {
                t("tools.nec-nvis-good")
              }
            }}
          </p>
        }
        .into_any()
      }}

      <p class="text-xs text-muted-foreground">{move || t("tools.nec-nvis-note")}</p>
    </div>
  }
}

fn tf_low(elev: f64) -> String {
  crate::i18n::tf("tools.nec-nvis-low", &[&format!("{elev:.0}")])
}
