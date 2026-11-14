use leptos::prelude::*;

/// 刻度尺 + 红指针（canvas 的 TuningScale，`docs/ui/pixel.md` §3.4）：顶栏下沿的换台刻度、搜索面板的扫描刻度。
///
/// `marks` 是印在刻度上的丝印标签与位置（0–100）；`needle` 是指针停的位置。`sweeping` 时指针在整条刻度上
/// 来回逐格扫（`motion.css` 的 `pxl-scale-sweep`，静态降级时停在 `needle`）。纯装饰（`aria-hidden`）：
/// 指针表达的状态在旁边都有文字。
#[component]
pub fn TuningScale(
  #[prop(into)] marks: Signal<Vec<(String, u8)>>,
  #[prop(into)] needle: Signal<u8>,
  #[prop(optional, into)] sweeping: Signal<bool>,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  view! {
    <div class=format!("pxl-scale {class}") data-sweep=move || sweeping.get().then_some("") aria-hidden="true">
      {move || {
        marks
          .get()
          .into_iter()
          .map(|(label, at)| {
            view! { <span class="pxl-scale-label" style=format!("--at: {}", f64::from(at) / 100.0)>{label}</span> }
          })
          .collect_view()
      }}
      // 指针挂在一条整宽的轨道上、平移轨道：`translate` 的百分比按轨道自身宽度算，正好是刻度位置。
      <span class="pxl-scale-track" style=move || format!("translate: {}% 0", needle.get())>
        <span class="pxl-scale-needle"></span>
      </span>
    </div>
  }
}
