//! 可拖拽的史密斯圆图：拖动指针即改变负载阻抗，并画出匹配元件的轨迹。

use ham_web_core::smith::{
  constant_g_arc, constant_r_arc, gamma, reactance_arc, resistance_circle, series_reactance,
  shunt_susceptance, swr_and_return_loss, z_from_gamma,
};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::i18n::t;

/// 画布边长。
const SIZE: f64 = 360.0;
/// 圆心。
const C: f64 = SIZE / 2.0;
/// 单位圆半径。
const R: f64 = 150.0;
/// 特性阻抗（Ω）。
const Z0: f64 = 50.0;

const R_CIRCLES: [f64; 5] = [0.2, 0.5, 1.0, 2.0, 5.0];
const X_ARCS: [f64; 8] = [-5.0, -2.0, -1.0, -0.5, 0.5, 1.0, 2.0, 5.0];

fn to_path(pts: &[(f64, f64)]) -> String {
  let mut d = String::with_capacity(pts.len() * 12);
  for (i, &(gr, gi)) in pts.iter().enumerate() {
    let x = C + gr * R;
    let y = C - gi * R;
    if i == 0 {
      d.push_str(&format!("M {x:.1} {y:.1}"));
    } else {
      d.push_str(&format!(" L {x:.1} {y:.1}"));
    }
  }
  d
}

fn px(gr: f64) -> f64 {
  C + gr * R
}

fn py(gi: f64) -> f64 {
  C - gi * R
}

/// 一次匹配的结果：两条轨迹弧与匹配后的阻抗。
#[derive(Clone, PartialEq)]
struct Trajectory {
  arcs: Vec<Vec<(f64, f64)>>,
  final_z: (f64, f64),
}

/// 按「先并后串 / 先串后并」把负载推进两步。
fn trajectory(load: (f64, f64), x_add: f64, b_add: f64, shunt_first: bool) -> Trajectory {
  let mut arcs = Vec::new();
  // 只用匹配后的阻抗；中间点的阻抗只用于画第二段弧。
  let (_, final_z) = if shunt_first {
    let d = load.0 * load.0 + load.1 * load.1;
    let (g, b) = if d > 1e-12 {
      (load.0 / d, -load.1 / d)
    } else {
      (0.0, 0.0)
    };
    arcs.push(constant_g_arc(g, b, b + b_add, 36));
    let z1 = shunt_susceptance(load, b_add);
    arcs.push(constant_r_arc(z1.0, z1.1, z1.1 + x_add, 36));
    (z1, series_reactance(z1, x_add))
  } else {
    arcs.push(constant_r_arc(load.0, load.1, load.1 + x_add, 36));
    let z1 = series_reactance(load, x_add);
    let d = z1.0 * z1.0 + z1.1 * z1.1;
    let (g, b) = if d > 1e-12 {
      (z1.0 / d, -z1.1 / d)
    } else {
      (0.0, 0.0)
    };
    arcs.push(constant_g_arc(g, b, b + b_add, 36));
    (z1, shunt_susceptance(z1, b_add))
  };
  Trajectory { arcs, final_z }
}

/// 可拖拽的史密斯圆图。
#[component]
pub(super) fn InteractiveChart(
  /// 负载电阻（Ω）。
  r_ohm: RwSignal<f64>,
  /// 负载电抗（Ω，感性为正）。
  x_ohm: RwSignal<f64>,
  /// 串入的电抗（Ω，感性为正）。
  series_x: RwSignal<f64>,
  /// 并入的电纳（mS，容性为正）。
  shunt_b: RwSignal<f64>,
  /// 是否先并联后串联。
  shunt_first: RwSignal<bool>,
) -> impl IntoView {
  let dragging = RwSignal::new(false);

  let load = move || (r_ohm.get() / Z0, x_ohm.get() / Z0);
  let traj = Memo::new(move |_| {
    let x_add = series_x.get() / Z0;
    let b_add = shunt_b.get() / 1000.0 * Z0;
    trajectory(load(), x_add, b_add, shunt_first.get())
  });
  let load_gamma = move || gamma(load().0, load().1);

  // 换算用**事件目标自身**的矩形，所以指针事件挂在 `<svg>` 上（见下面的 view）：
  // 外层 `div` 里还有一行说明文字，它把 div 的高度撑大、宽高比不再是 1:1，
  // 拿 div 的 rect 换算会把纵向压缩约 5%（拖到圆周底部会得到 |Γ| < 1）。
  let set_from_event = move |e: &web_sys::PointerEvent| {
    let Some(el) = e
      .current_target()
      .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
    else {
      return;
    };
    let rect = el.get_bounding_client_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
      return;
    }
    let x = (f64::from(e.client_x()) - rect.left()) / rect.width() * SIZE;
    let y = (f64::from(e.client_y()) - rect.top()) / rect.height() * SIZE;
    let mut gr = (x - C) / R;
    let mut gi = -(y - C) / R;
    // 夹在单位圆内：Γ = 1 对应无穷大阻抗，拖到边界外没有意义。
    let mag = gr.hypot(gi);
    if mag > 0.995 {
      gr *= 0.995 / mag;
      gi *= 0.995 / mag;
    }
    if let Some((r, xx)) = z_from_gamma(gr, gi) {
      r_ohm.set((r * Z0).clamp(0.5, 5000.0));
      x_ohm.set((xx * Z0).clamp(-5000.0, 5000.0));
    }
  };

  view! {
    <div
      class="relative mx-auto w-full max-w-[360px] cursor-crosshair touch-none select-none"
      role="group"
      aria-label=move || t("tools.smith-chart-drag-to")
    >
      // 拖动事件挂在 svg 上（而不是外层 div）：换算要用事件目标自身的 rect，
      // 见 `set_from_event` 上方的说明。
      <svg
        viewBox=format!("0 0 {SIZE} {SIZE}")
        class="block w-full"
        role="img"
        aria-label=move || t("tools.smith-chart")
        data-slot="smith-chart"
        on:pointerdown=move |e: web_sys::PointerEvent| {
          dragging.set(true);
          if let Some(el) = e
            .current_target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
          {
            let _ = el.set_pointer_capture(e.pointer_id());
          }
          set_from_event(&e);
        }
        on:pointermove=move |e: web_sys::PointerEvent| {
          if dragging.get() {
            set_from_event(&e);
          }
        }
        on:pointerup=move |e: web_sys::PointerEvent| {
          dragging.set(false);
          if let Some(el) = e
            .current_target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
          {
            let _ = el.release_pointer_capture(e.pointer_id());
          }
        }
        on:pointercancel=move |_| dragging.set(false)
      >
        <circle
          cx=C
          cy=C
          r=R
          class="fill-none stroke-muted-foreground/50"
          stroke-width="1.5"
          vector-effect="non-scaling-stroke"
        />
        {R_CIRCLES
          .iter()
          .map(|&r| {
            let d = to_path(&resistance_circle(r, 120));
            view! {
              <path
                d=d
                class="fill-none stroke-muted-foreground/20"
                stroke-width="1"
                vector-effect="non-scaling-stroke"
              />
            }
          })
          .collect_view()}
        {X_ARCS
          .iter()
          .map(|&x| {
            let d = to_path(&reactance_arc(x, 120));
            view! {
              <path
                d=d
                class="fill-none stroke-muted-foreground/20"
                stroke-width="1"
                vector-effect="non-scaling-stroke"
              />
            }
          })
          .collect_view()}
        <line
          x1=C - R
          y1=C
          x2=C + R
          y2=C
          class="stroke-muted-foreground/40"
          stroke-width="1"
          vector-effect="non-scaling-stroke"
        />
        // 当前负载的等驻波比圆：圆上任意一点驻波比相同。
        {move || {
          let (gr, gi) = load_gamma();
          let mag = gr.hypot(gi).min(0.999);
          view! {
            <circle
              cx=C
              cy=C
              r=mag * R
              class="fill-none stroke-amber-500/40"
              stroke-dasharray="4 3"
              stroke-width="1"
              vector-effect="non-scaling-stroke"
            />
          }
        }}
        // 匹配元件的轨迹。
        {move || {
          traj.get()
            .arcs
            .into_iter()
            .map(|arc| {
              view! {
                <path
                  d=to_path(&arc)
                  class="fill-none stroke-sky-500"
                  stroke-width="2"
                  vector-effect="non-scaling-stroke"
                />
              }
            })
            .collect_view()
        }}
        // 匹配后的落点。
        {move || {
          let (gr, gi) = gamma(traj.get().final_z.0, traj.get().final_z.1);
          let (swr, _) = swr_and_return_loss(gr, gi);
          let matched = swr < 1.15;
          view! {
            <rect
              x=px(gr) - 4.0
              y=py(gi) - 4.0
              width="8"
              height="8"
              class=if matched { "fill-emerald-500" } else { "fill-sky-500" }
            />
          }
        }}
        // 负载点（可拖动）。
        {move || {
          let (gr, gi) = load_gamma();
          view! {
            <circle
              cx=px(gr)
              cy=py(gi)
              r="6"
              class="fill-primary stroke-background"
              stroke-width="2"
            />
          }
        }}
      </svg>
      <p class="mt-1 text-center text-xs text-muted-foreground">
        {move || if dragging.get() { t("tools.dragging") } else { t("tools.drag-the-dot-to") }}
      </p>
    </div>
  }
}
