//! 频带扫描卡片：整段频带上的驻波比曲线，以及 SWR ≤ 2 的可用带宽。
//!
//! 扫频要为每个频点重装一遍阻抗矩阵，是页面上最贵的一项 —— 所以默认折叠，只有
//! 展开时才求解（折叠状态下这个 memo 立刻返回，一次求解都不做）。

use ham_web_core::nec::{NecInput, TransmissionLine, sweep_with_lines, swr_bandwidth_with_lines};
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::ui::{Chip, ChipGroup};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

const NOTE: &str = "text-xs text-muted-foreground";
const RESULT: &str = "rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground";

/// 判断「可用带宽」的驻波比上限（业余界常用判据）。
const SWR_LIMIT: f64 = 2.0;
/// 扫频图尺寸与内边距。
const PLOT_W: f64 = 640.0;
const PLOT_H: f64 = 200.0;
const PAD: f64 = 40.0;
/// 纵轴至少画到该值，SWR 更大时按实际最大值抬高。
const MIN_Y_MAX: f64 = 4.0;
/// 可选频点数。
const POINT_CHOICES: [usize; 3] = [11, 21, 41];

/// `input` 是页面传进来的**防抖后**的求解输入（只读，取 `Signal` 以同时接受
/// `Memo` 与 `RwSignal`）；否则每敲一个字符都会把整段频带重扫一遍。
#[component]
pub(super) fn SweepSection(
  #[prop(into)] input: Signal<Option<NecInput>>,
  /// 当前模型里的传输线（扫频同样要带上，否则两条口径不一致）。
  lines: Signal<Vec<TransmissionLine>>,
) -> impl IntoView {
  let open = RwSignal::new(false);
  let points = RwSignal::new(21usize);
  let span_pct = RwSignal::new(String::from("10"));

  let span_frac = move || {
    span_pct
      .get()
      .trim()
      .parse::<f64>()
      .unwrap_or(10.0)
      .clamp(0.5, 90.0)
      / 100.0
  };

  // `(中心频率, 扫描点, SWR ≤ 2 的带宽)`；折叠时不做任何求解。
  let data = Memo::new(move |_| {
    if !open.get() {
      return None;
    }
    let model = input.get()?;
    let f0 = model.freq_hz;
    let span = span_frac();
    let n = points.get();
    let lines = lines.get();
    let pts = sweep_with_lines(&model, &lines, f0 * (1.0 - span), f0 * (1.0 + span), n);
    if pts.len() < 2 {
      return None;
    }
    let band = swr_bandwidth_with_lines(&model, &lines, SWR_LIMIT, span, n.max(21));
    Some((f0, pts, band))
  });

  view! {
    <div class="space-y-3 rounded-xl border bg-card p-4">
      <div class="flex flex-wrap items-center justify-between gap-2">
        <span class="text-sm font-semibold">{move || t("tools.nec-sweep-title")}</span>
        <button
          type="button"
          class="cursor-pointer rounded-md border bg-background px-3 py-1 text-xs font-medium hover:bg-accent"
          aria-expanded=move || if open.get() { "true" } else { "false" }
          on:click=move |_| open.update(|v| *v = !*v)
        >
          {move || {
            if open.get() {
              t("tools.nec-sweep-hide")
            } else {
              t("tools.nec-sweep-show")
            }
          }}
        </button>
      </div>

      {move || {
        if !open.get() {
          return view! {
            <p class=NOTE>{move || t("tools.nec-sweep-collapsed")}</p>
          }
            .into_any();
        }
        // `Field` 的标签与控件是兄弟节点，`r#for` / `id` 必须配对才能点击标签聚焦输入框。
        // 声明在这个闭包里：外层 `move ||` 必须保持 `Fn`，捕获 id 会让它退化成 `FnOnce`。
        let span_id = unique_id("nec-sweep-span");
        view! {
          <div class="flex flex-wrap items-end gap-2">
            <Field
              label=Signal::derive(move || t("tools.nec-sweep-span"))
              r#for=span_id.clone()
            >
              <NumberField
                id=span_id.clone()
                value=span_pct
                on_change=Callback::new(move |v: String| span_pct.set(v))
                step=0.5
                min=0.5
                max=90.0
                aria_label=Signal::derive(move || t("tools.nec-sweep-span"))
                class="w-20"
                controls=false
              />
            </Field>
            <span class=NOTE>{move || t("tools.nec-sweep-points")}</span>
            <ChipGroup
              value=Signal::derive(move || points.get().to_string())
              on_change=Callback::new(move |v: String| {
                if let Ok(n) = v.parse::<usize>() {
                  points.set(n);
                }
              })
              aria_label=Signal::derive(move || t("tools.nec-sweep-points"))
            >
              {POINT_CHOICES
                .iter()
                .map(|&n| view! { <Chip value=n.to_string()>{n}</Chip> })
                .collect_view()}
            </ChipGroup>
          </div>
        }
        .into_any()
      }}

      {move || match data.get() {
        None => ().into_any(),
        Some((f0, pts, band)) => {
          let (f_lo, f_hi) = (pts[0].freq_hz, pts[pts.len() - 1].freq_hz);
          let y_max = pts
            .iter()
            .map(|p| p.swr_50)
            .fold(MIN_Y_MAX, f64::max)
            .min(20.0);
          let w = PLOT_W - 2.0 * PAD;
          let h = PLOT_H - 2.0 * PAD;
          let px = move |f: f64| PAD + (f - f_lo) / (f_hi - f_lo).max(1.0) * w;
          let py = move |s: f64| {
            PAD + (y_max - s.clamp(1.0, y_max)) / (y_max - 1.0).max(1e-9) * h
          };
          let curve = pts
            .iter()
            .enumerate()
            .map(|(i, p)| {
              format!(
                "{}{:.1} {:.1}",
                if i == 0 { "M" } else { "L" },
                px(p.freq_hz),
                py(p.swr_50)
              )
            })
            .collect::<Vec<_>>()
            .join(" ");
          let y2 = py(SWR_LIMIT);
          let x0 = px(f0);
          // 可用带宽区间底色（没有找到时为空）。
          let band_rect = band.map(|(lo, hi)| {
            let (x1, x2) = (px(lo), px(hi));
            (x1.min(x2), (x2 - x1).abs())
          });
          let best = pts
            .iter()
            .copied()
            .reduce(|a, b| if b.swr_50 < a.swr_50 { b } else { a })
            .unwrap_or(pts[0]);
          let center_swr = pts
            .iter()
            .copied()
            .min_by(|a, b| {
              (a.freq_hz - f0)
                .abs()
                .total_cmp(&(b.freq_hz - f0).abs())
            })
            .unwrap_or(pts[0])
            .swr_50;
          let mhz = |f: f64| format!("{:.4}", f / 1e6);
          let ticks = [f_lo, f0, f_hi];
          let label = |f: f64| {
            if (f - f0).abs() < 1.0 {
              format!("{:.3}*", f / 1e6)
            } else {
              format!("{:.3}", f / 1e6)
            }
          };
          let (lo_s, hi_s, span_khz, rel_s) = match band {
            Some((lo, hi)) => (
              mhz(lo),
              mhz(hi),
              format!("{:.0}", (hi - lo) / 1e3),
              format!("{:.1}", (hi - lo) / f0 * 100.0),
            ),
            None => (String::new(), String::new(), String::new(), String::new()),
          };
          let center_s = format!("{center_swr:.2}");
          let best_s = format!("{:.2}", best.swr_50);
          let best_f = mhz(best.freq_hz);
          let span_s = format!("{:.0}", span_frac() * 100.0);
          view! {
            <svg
              viewBox=format!("0 0 {PLOT_W} {PLOT_H}")
              class="mx-auto w-full"
              role="img"
              aria-label=move || t("tools.nec-sweep-chart")
            >
              <rect
                x=PAD
                y=PAD
                width=w
                height=h
                class="fill-none stroke-muted-foreground/25"
                stroke-width="1"
              />
              {band_rect
                .map(|(x, width)| {
                  view! {
                    <rect
                      x=x
                      y=PAD
                      width=width
                      height=h
                      class="fill-emerald-500/10"
                      stroke="none"
                    />
                  }
                })}
              <line
                x1=PAD
                y1=y2
                x2=PAD + w
                y2=y2
                class="stroke-amber-500/70"
                stroke-dasharray="4 3"
                stroke-width="1"
              />
              <text x=4.0 y=y2 + 3.0 class="fill-muted-foreground text-[10px]">
                {format!("{SWR_LIMIT:.1}")}
              </text>
              <line
                x1=x0
                y1=PAD
                x2=x0
                y2=PAD + h
                class="stroke-primary/50"
                stroke-dasharray="3 3"
                stroke-width="1"
              />
              <path
                d=curve
                class="fill-none stroke-primary"
                stroke-width="1.8"
                vector-effect="non-scaling-stroke"
              />
              {ticks
                .iter()
                .map(|&f| {
                  view! {
                    <text
                      x=px(f)
                      y=PLOT_H - 8.0
                      text-anchor="middle"
                      class="fill-muted-foreground text-[10px]"
                    >
                      {label(f)}
                    </text>
                  }
                })
                .collect_view()}
              <text x=4.0 y=PAD + 4.0 class="fill-muted-foreground text-[10px]">
                {format!("{y_max:.1}")}
              </text>
              <text x=PAD + w y=PLOT_H - 8.0 text-anchor="end" class="fill-muted-foreground text-[10px]">
                {move || t("tools.frequency-mhz")}
              </text>
            </svg>
            <div class=RESULT>
              {tf(
                "tools.nec-sweep-center-swr",
                &[&center_s, &best_s, &best_f],
              )}
            </div>
            {match band {
              Some(_) => {
                view! {
                  <div class=RESULT>
                    {tf(
                      "tools.nec-sweep-band",
                      &[&lo_s, &hi_s, &span_khz, &rel_s],
                    )}
                  </div>
                }
                .into_any()
              }
              None => {
                // 中心频率就不达标、或带宽比扫描范围还宽 —— 两种情况的提示不同。
                let centered = center_swr <= SWR_LIMIT;
                view! {
                  <div class=RESULT>
                    {if centered {
                      tf("tools.nec-sweep-too-wide", &[&span_s])
                    } else {
                      t("tools.nec-sweep-no-center")
                    }}
                  </div>
                }
                .into_any()
              }
            }}
            <p class=NOTE>{move || t("tools.nec-sweep-note")}</p>
          }
          .into_any()
        }
      }}
    </div>
  }
}
