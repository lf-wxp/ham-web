//! 几何画布：把导线表画成可拖拽的二维视图，并提供阵列复制。
//!
//! 画布**只是快捷方式**：真正的数值仍在下面的导线表里逐格可编辑，拖拽等价于改那几格。
//! 因此 SVG 整体给一个 `role="img"` 与说明，内部的线与手柄对辅助技术隐藏 ——
//! 拖拽无法用键盘操作，把它宣传成可交互控件是误导；表格才是无障碍入口。
//!
//! 投影平面自动取几何跨度最大的两个轴（见 [`View::fit`]），并在标题里写明是哪两个轴，
//! 免得用户拖动时不知道自己在改哪个坐标。

use ham_web_core::nec::Wire;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::i18n::t;
use crate::ui::{ControlSize, Field, Input, NativeSelect, SelectOption};
use crate::util::unique_id;

use super::WireRow;

/// 画布边长（SVG viewBox 用的模型无关单位）。
const SIZE: f64 = 320.0;
/// 四周留白。
const PAD: f64 = 18.0;

/// 当前视图：投影轴、缩放与中心。
#[derive(Clone, Copy, PartialEq)]
struct View {
  /// 水平轴（0 = X，1 = Y，2 = Z）。
  h: usize,
  /// 垂直轴。
  v: usize,
  /// 每个模型单位对应的像素数。
  scale: f64,
  /// 视图中心（模型坐标，`h` / `v` 轴上的分量）。
  hc: f64,
  vc: f64,
}

impl Default for View {
  fn default() -> Self {
    Self {
      h: 0,
      v: 1,
      scale: 10.0,
      hc: 0.0,
      vc: 0.0,
    }
  }
}

/// 把半跨度吸附到 1–2–5 序列：小幅拖动不会让视图不停地重新缩放。
fn nice_half(extent: f64) -> f64 {
  let e = (extent / 2.0).max(0.05);
  let p = 10f64.powf(e.log10().floor());
  let m = e / p;
  let step = if m <= 1.0 {
    1.0
  } else if m <= 2.0 {
    2.0
  } else if m <= 5.0 {
    5.0
  } else {
    10.0
  };
  step * p
}

impl View {
  /// 由导线表算出投影平面与缩放。
  ///
  /// 取跨度最大的轴做水平轴，剩下两个里更大的做垂直轴（并列时取下标小的），
  /// 这样单根沿 X 的偶极会画成一条横线，上下方向留给了 Y。
  fn fit(rows: &[WireRow]) -> Self {
    let pts: Vec<[f64; 3]> = rows
      .iter()
      .filter_map(|r| r.to_wire())
      .flat_map(|w| [w.a, w.b])
      .collect();
    if pts.is_empty() {
      return Self::default();
    }
    let mut lo = pts[0];
    let mut hi = pts[0];
    for p in &pts {
      for k in 0..3 {
        lo[k] = lo[k].min(p[k]);
        hi[k] = hi[k].max(p[k]);
      }
    }
    let span = |k: usize| hi[k] - lo[k];
    let mut axes = [0usize, 1, 2];
    // 按跨度从大到小排序（稳定排序 ⇒ 并列时保持下标顺序）。
    axes.sort_by(|&a, &b| span(b).total_cmp(&span(a)));
    let (h, v) = (axes[0], axes[1]);
    let half = nice_half(span(h).max(span(v)).max(1.0));
    // 中心也吸附到同一档，避免拖动时视图整体漂移。
    let snap = |x: f64| (x / half).round() * half;
    Self {
      h,
      v,
      scale: (SIZE / 2.0 - PAD) / half,
      hc: snap(0.5 * (lo[h] + hi[h])),
      vc: snap(0.5 * (lo[v] + hi[v])),
    }
  }

  fn to_screen(self, p: [f64; 3]) -> (f64, f64) {
    (
      SIZE / 2.0 + (p[self.h] - self.hc) * self.scale,
      SIZE / 2.0 - (p[self.v] - self.vc) * self.scale,
    )
  }

  /// 屏幕位移换算成模型位移（`h` / `v` 轴分量）。
  fn to_model(self, dx: f64, dy: f64) -> (f64, f64) {
    (dx / self.scale, -dy / self.scale)
  }
}

/// 拖动目标。
#[derive(Clone, Copy, PartialEq)]
enum Drag {
  /// 某个端点：`(行 id, 是否终点)`。
  End(usize, bool),
  /// 整根导线。
  Wire(usize),
}

impl Drag {
  fn id(self) -> usize {
    match self {
      Drag::End(id, _) | Drag::Wire(id) => id,
    }
  }
}

/// 拖动期间的**本地预览**：只记录从按下那一刻起的累计位移，不写回 `wires`。
///
/// 为什么要这样：`wires` 一变，页面的 `active_input` / `result` 记忆值就会重算 ——
/// 那是一次完整的矩量法求解（矩阵装配 + 方向图的球面扫描 + 三维网格 + 仰角切面）。
/// 拖动时指针每秒产生几十次事件，每次都求解的话主线程被占满，画布跟不上鼠标，
/// 连页面上的 CSS 动画都会一起卡住。松手（或取消）时才提交这一次位移。
#[derive(Clone, Copy, PartialEq)]
struct Preview {
  id: usize,
  target: Drag,
  /// 累计位移（模型坐标，三个分量）。
  shift: [f64; 3],
}

/// 轴名。`t()` 的字面量必须写在**调用点** —— 走常量数组传进去静态扫描认不出来，
/// 那三条词条会被判成死条目（踩过一次）。
fn axis_label(k: usize) -> String {
  match k {
    0 => t("tools.nec-axis-x"),
    1 => t("tools.nec-axis-y"),
    _ => t("tools.nec-axis-z"),
  }
}

#[component]
pub(super) fn WireCanvas(
  wires: RwSignal<Vec<WireRow>>,
  /// 新增行（阵列复制）用的 id 生成器。
  new_id: Callback<(), usize>,
  /// 馈电点 `(导线序号, 相对位置)`，画布上标出来。
  feed: RwSignal<(usize, f64)>,
) -> impl IntoView {
  let view = RwSignal::new(View::default());
  let dragging = RwSignal::new(false);
  let target = StoredValue::new(None::<Drag>);
  // 拖动开始时的指针坐标与**冻结**的视图：拖动过程中几何在变，
  // 若每次都重新拟合视图，位移换算就会跟着变，手感会飘。
  let frozen = StoredValue::new(View::default());
  let last = StoredValue::new((0.0f64, 0.0f64));
  // 只有画布读这条信号：拖动时改它不会牵动求解器，所以拖起来跟手。
  let preview = RwSignal::new(None::<Preview>);

  // 只在「没在拖动」时重新拟合视图。
  Effect::new(move |_| {
    let rows = wires.get();
    if !dragging.get() {
      view.set(View::fit(&rows));
    }
  });

  // 阵列复制控件
  let axis = RwSignal::new(0usize);
  let spacing = RwSignal::new(String::from("2.0"));
  let count = RwSignal::new(String::from("3"));

  // `Field` 的标签与控件是兄弟节点，`r#for` / `id` 必须配对才能点击标签聚焦输入框。
  let spacing_id = unique_id("nec-array-spacing");
  let count_id = unique_id("nec-array-count");

  let axis_options: Vec<SelectOption> = (0..3usize)
    .map(|k| SelectOption::new(k.to_string(), Signal::derive(move || axis_label(k))))
    .collect();

  let duplicate = move |_| {
    let rows = wires.get_untracked();
    let Some(template) = rows.last().and_then(|r| r.to_wire()) else {
      return;
    };
    let Ok(step) = spacing.get_untracked().trim().parse::<f64>() else {
      return;
    };
    let n = count
      .get_untracked()
      .trim()
      .parse::<usize>()
      .unwrap_or(2)
      .clamp(2, 8);
    let k = axis.get_untracked();
    let mut added = Vec::with_capacity(n - 1);
    for i in 1..n {
      let d = step * i as f64;
      let mut a = template.a;
      let mut b = template.b;
      a[k] += d;
      b[k] += d;
      added.push(WireRow::from_wire(
        new_id.run(()),
        &Wire::new(a, b, template.radius, template.segments),
      ));
    }
    wires.update(|rs| rs.extend(added));
  };

  // 把预览的位移真正写回导线表 —— 每次拖动只调用一次。
  let commit = move |p: Preview| {
    wires.update(|rows| {
      let Some(row) = rows.iter_mut().find(|r| r.id == p.id) else {
        return;
      };
      let Some(w) = row.to_wire() else {
        return;
      };
      let move_by = |q: [f64; 3]| [q[0] + p.shift[0], q[1] + p.shift[1], q[2] + p.shift[2]];
      let (a, b) = match p.target {
        Drag::End(_, false) => (move_by(w.a), w.b),
        Drag::End(_, true) => (w.a, move_by(w.b)),
        Drag::Wire(_) => (move_by(w.a), move_by(w.b)),
      };
      *row = WireRow::from_wire(row.id, &Wire::new(a, b, w.radius, w.segments));
    });
  };

  let start = move |e: &web_sys::PointerEvent, t: Drag| {
    dragging.set(true);
    target.set_value(Some(t));
    preview.set(Some(Preview {
      id: t.id(),
      target: t,
      shift: [0.0; 3],
    }));
    frozen.set_value(view.get_untracked());
    last.set_value((e.client_x() as f64, e.client_y() as f64));
    if let Some(el) = e
      .current_target()
      .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
    {
      let _ = el.set_pointer_capture(e.pointer_id());
    }
  };

  let move_handler = move |e: web_sys::PointerEvent| {
    if !dragging.get() {
      return;
    }
    let (x0, y0) = last.get_value();
    let (x, y) = (e.client_x() as f64, e.client_y() as f64);
    last.set_value((x, y));
    let v = frozen.get_value();
    let (dh, dv) = v.to_model(x - x0, y - y0);
    // 只改预览：这一条信号只有画布在读，不触发任何求解。
    preview.update(|p| {
      if let Some(p) = p {
        p.shift[v.h] += dh;
        p.shift[v.v] += dv;
      }
    });
  };

  let end = move |e: web_sys::PointerEvent| {
    dragging.set(false);
    target.set_value(None);
    if let Some(p) = preview.get_untracked() {
      commit(p);
    }
    preview.set(None);
    if let Some(el) = e
      .current_target()
      .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
    {
      let _ = el.release_pointer_capture(e.pointer_id());
    }
  };

  view! {
    <div class="space-y-3 rounded-xl border bg-card p-4">
      <div class="flex flex-wrap items-center justify-between gap-2">
        <span class="text-sm font-semibold">{move || t("tools.nec-canvas")}</span>
        <span class="text-xs text-muted-foreground">
          {move || {
            let v = view.get();
            format!("{} – {}", axis_label(v.h), axis_label(v.v))
          }}
        </span>
      </div>

      <div class="flex flex-wrap items-end gap-2">
        <label class="flex flex-col gap-1">
          <span class="text-xs text-muted-foreground">
            {move || t("tools.nec-array-axis")}
          </span>
          <NativeSelect
            value=Signal::derive(move || axis.get().to_string())
            on_change=Callback::new(move |v: String| {
              axis.set(v.parse::<usize>().unwrap_or(0));
            })
            options=axis_options
            size=ControlSize::Sm
            aria_label=Signal::derive(move || t("tools.nec-array-axis"))
            class="w-auto"
          />
        </label>
        <Field label=Signal::derive(move || t("tools.nec-array-spacing")) r#for=spacing_id.clone()>
          <Input
            id=spacing_id.clone()
            value=spacing
            on_change=Callback::new(move |v: String| spacing.set(v))
            size=ControlSize::Sm
            aria_label=Signal::derive(move || t("tools.nec-array-spacing"))
            class="w-16"
          />
        </Field>
        <Field label=Signal::derive(move || t("tools.nec-array-count")) r#for=count_id.clone()>
          <Input
            id=count_id.clone()
            value=count
            on_change=Callback::new(move |v: String| count.set(v))
            size=ControlSize::Sm
            aria_label=Signal::derive(move || t("tools.nec-array-count"))
            class="w-12"
          />
        </Field>
        <button
          type="button"
          class="cursor-pointer rounded-md border bg-background px-2 py-1 text-xs font-medium hover:bg-accent"
          on:click=duplicate
        >
          {move || t("tools.nec-array-copy")}
        </button>
      </div>
      <p class="text-xs text-muted-foreground">{move || t("tools.nec-array-note")}</p>

      <div
        class="mx-auto w-full max-w-[360px] cursor-move touch-none select-none"
        on:pointermove=move_handler
        on:pointerup=end
        on:pointercancel=move |_| {
          // 取消 = 丢弃这次拖动，不提交。
          dragging.set(false);
          target.set_value(None);
          preview.set(None);
        }
      >
        <svg
          viewBox=format!("0 0 {SIZE} {SIZE}")
          class="w-full rounded border bg-muted/20"
          role="img"
          aria-label=move || t("tools.nec-canvas-alt")
        >
          <line
            x1=SIZE / 2.0
            y1=PAD / 2.0
            x2=SIZE / 2.0
            y2=SIZE - PAD / 2.0
            class="stroke-muted-foreground/20"
            stroke-width="1"
          />
          <line
            x1=PAD / 2.0
            y1=SIZE / 2.0
            x2=SIZE - PAD / 2.0
            y2=SIZE / 2.0
            class="stroke-muted-foreground/20"
            stroke-width="1"
          />
          {move || {
            let v = view.get();
            let rows = wires.get();
            let (fw, fa) = feed.get();
            let mut out: Vec<AnyView> = Vec::new();
            for (i, row) in rows.iter().enumerate() {
              let Some(w) = row.to_wire() else {
                continue;
              };
              // 正在拖动的那一根：图形按预览位移画，虚线表示「还没提交」。
              let (mut pa, mut pb) = (w.a, w.b);
              let mut pending = false;
              if let Some(p) = preview.get()
                && p.id == row.id
              {
                pending = true;
                let move_by =
                  |q: [f64; 3]| [q[0] + p.shift[0], q[1] + p.shift[1], q[2] + p.shift[2]];
                match p.target {
                  Drag::End(_, false) => pa = move_by(pa),
                  Drag::End(_, true) => pb = move_by(pb),
                  Drag::Wire(_) => {
                    pa = move_by(pa);
                    pb = move_by(pb);
                  }
                }
              }
              let (ax, ay) = v.to_screen(pa);
              let (bx, by) = v.to_screen(pb);
              let id = row.id;
              let stroke = if pending { "stroke-primary/50" } else { "stroke-primary" };
              let dash = if pending { "4 3" } else { "" };
              out.push(
                view! {
                  <line
                    x1=ax
                    y1=ay
                    x2=bx
                    y2=by
                    class=stroke
                    stroke-width="2"
                    stroke-dasharray=dash
                    aria-hidden="true"
                  />
                  // 更粗的透明命中区：细线不好抓。
                  <line
                    x1=ax
                    y1=ay
                    x2=bx
                    y2=by
                    class="stroke-transparent"
                    stroke-width="12"
                    aria-hidden="true"
                    data-wire=id.to_string()
                    on:pointerdown=move |e: web_sys::PointerEvent| start(&e, Drag::Wire(id))
                  />
                  <circle
                    cx=ax
                    cy=ay
                    r="5"
                    class="fill-primary"
                    aria-hidden="true"
                    data-handle=format!("{i}-a")
                    on:pointerdown=move |e: web_sys::PointerEvent| start(&e, Drag::End(id, false))
                  />
                  <circle
                    cx=bx
                    cy=by
                    r="5"
                    class="fill-primary"
                    aria-hidden="true"
                    data-handle=format!("{i}-b")
                    on:pointerdown=move |e: web_sys::PointerEvent| start(&e, Drag::End(id, true))
                  />
                }
                .into_any(),
              );
              // 馈电点：画一个小方块。
              if i == fw {
                let p = [
                  pa[0] + (pb[0] - pa[0]) * fa,
                  pa[1] + (pb[1] - pa[1]) * fa,
                  pa[2] + (pb[2] - pa[2]) * fa,
                ];
                let (fx, fy) = v.to_screen(p);
                out.push(
                  view! {
                    <rect
                      x=fx - 4.0
                      y=fy - 4.0
                      width="8"
                      height="8"
                      class="fill-none stroke-primary"
                      stroke-width="2"
                      aria-hidden="true"
                      data-feed="1"
                    />
                  }
                  .into_any(),
                );
              }
            }
            out.collect_view()
          }}
        </svg>
      </div>
      <p class="text-xs text-muted-foreground">{move || t("tools.nec-canvas-note")}</p>
    </div>
  }
}
