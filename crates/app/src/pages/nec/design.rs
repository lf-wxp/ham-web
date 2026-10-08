//! Yagi 设计向导卡片：选单元数 → 跑坐标下降 → 给出尺寸并一键加载到导线表。
//!
//! 优化是同步的、要跑几十次求解（前端约 1–2 秒），所以点击后先让出一帧让浏览器
//! 画出「正在优化」，再开始算 —— 否则页面会先卡住、连提示都看不到。

use ham_web_core::nec::{YagiDesign, YagiSearch, yagi_design_input};
use leptos::prelude::*;
use leptos::task::spawn_local;

use super::{Wire, WireRow};
use crate::i18n::{t, tf};
use crate::ui::{Chip, ChipGroup};
use crate::util::sleep;

const NOTE: &str = "text-xs text-muted-foreground";
const RESULT: &str = "rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground";

const CELL: &str = "border px-2 py-1 tabular-nums";

/// 可选的单元数。
const ELEMENT_CHOICES: [usize; 4] = [2, 3, 4, 5];

#[component]
pub(super) fn DesignSection(
  /// 中心频率输入（向导按它设计，加载时也写回它）。
  freq_mhz: RwSignal<String>,
  /// 导线表（加载时整体替换）。
  wires: RwSignal<Vec<WireRow>>,
  /// 馈电点（加载时指向有源振子）。
  feed: RwSignal<(usize, f64)>,
  /// 由核心的 `Wire` 造一行导线（借用页面统一的 id 生成器，保证 `<For>` 键稳定）。
  row_factory: Callback<Wire, WireRow>,
  /// 导线半径（m）。
  radius_m: Signal<f64>,
  /// 地面模型：向导在自由空间里设计，加载时也切到自由空间 —— 否则导线全在 `z = 0`
  /// 而页面还挂着「理想导体地面」，水平导线会被镜像完全抵消，结果直接崩掉。
  ground_kind: RwSignal<usize>,
) -> impl IntoView {
  let elements = RwSignal::new(3usize);
  let result = RwSignal::new(None::<YagiDesign>);
  let running = RwSignal::new(false);
  let loaded = RwSignal::new(false);
  // 优化进度 0.0–1.0（分帧推进时显示；百分比与语言无关，不必走 i18n）。
  let progress = RwSignal::new(0.0_f64);

  // 分帧推进的优化循环会一直跑到搜完；用户在过程中离开页面时信号已释放，
  // 继续写信号就是 panic，也不该再白跑几十次矩量法求解（见 `util::mount_guard`）。
  let alive = crate::util::mount_guard();
  let run = move |_| {
    if running.get_untracked() {
      return;
    }
    let n = elements.get_untracked();
    let freq = freq_mhz
      .get_untracked()
      .trim()
      .parse::<f64>()
      .unwrap_or(14.1)
      * 1e6;
    let radius = radius_m.get_untracked();
    running.set(true);
    loaded.set(false);
    progress.set(0.0);
    let alive = alive.clone();
    spawn_local(async move {
      // 先让出一帧：让「正在优化…」有机会渲染出来再开始算（建起点也要解一次）。
      sleep(0).await;
      if !alive() {
        return;
      }
      let Some(mut search) = YagiSearch::new(freq, n, radius) else {
        result.set(None);
        running.set(false);
        return;
      };
      loop {
        // 每次只评估 2 个模型（约 40–80 ms）就让出一帧。WASM 是单线程：一口气跑完
        // 三十多次求解会把页面冻住 1–2 秒，连「正在优化」都画不出来。
        if let Some(design) = search.step(2) {
          result.set(Some(design));
          running.set(false);
          return;
        }
        progress.set(search.progress());
        if search.is_done() {
          // 搜完了却拿不到结果（终点求解失败）：给出「无结果」，别卡在「正在优化」。
          result.set(None);
          running.set(false);
          return;
        }
        sleep(0).await;
        // 用户可能在优化途中离开页面：此后的信号访问都不再有效，直接收工。
        if !alive() {
          return;
        }
      }
    });
  };

  let load = move |_| {
    let Some(d) = result.get_untracked() else {
      return;
    };
    let Some(input) = yagi_design_input(&d, radius_m.get_untracked()) else {
      return;
    };
    wires.set(input.wires.iter().map(|w| row_factory.run(*w)).collect());
    ground_kind.set(0);
    // 有源振子是第二根导线（索引 1），中心馈电。
    feed.set((1, 0.5));
    freq_mhz.set(format!("{:.4}", d.freq_hz / 1e6));
    loaded.set(true);
  };

  view! {
    <div class="space-y-3 rounded-xl border bg-card p-4">
      <div class="text-sm font-semibold">{move || t("tools.nec-design-title")}</div>
      <p class=NOTE>{move || t("tools.nec-design-note")}</p>

      <div class="flex flex-wrap items-center gap-2">
        <span class=NOTE>{move || t("tools.nec-design-elements")}</span>
        <ChipGroup
          value=Signal::derive(move || elements.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(n) = v.parse::<usize>() {
              elements.set(n);
            }
          })
          aria_label=Signal::derive(move || t("tools.nec-design-elements"))
        >
          {ELEMENT_CHOICES
            .iter()
            .map(|&n| view! { <Chip value=n.to_string()>{n}</Chip> })
            .collect_view()}
        </ChipGroup>
        <button
          type="button"
          class="cursor-pointer rounded-md border bg-background px-3 py-1 text-xs font-medium hover:bg-accent disabled:opacity-50"
          disabled=move || running.get()
          on:click=run
        >
          {move || t("tools.nec-design-run")}
        </button>
      </div>

      {move || {
        if running.get() {
          // 分帧推进时给出百分比：用户能看出它在动，而不是「点了没反应」。
          return view! {
            <p class=NOTE>{move || t("tools.nec-design-running")}</p>
            <p class=NOTE>{move || format!("{:.0}%", progress.get() * 100.0)}</p>
          }
          .into_any();
        }
        let Some(d) = result.get() else {
          return view! { <p class=NOTE>{move || t("tools.nec-design-need-run")}</p> }
            .into_any();
        };
        let lam = d.wavelength_m;
        let rows = d
          .lengths
          .iter()
          .enumerate()
          .map(|(i, &l)| {
            let role = match i {
              0 => t("tools.nec-design-reflector"),
              1 => t("tools.nec-design-driven"),
              k => tf("tools.nec-design-director", &[&(k - 1).to_string()]),
            };
            // 单位换算是语言无关的，括号用半角 —— 全角括号在英/西语界面里会显得突兀。
            let len = format!("{l:.3} m ({:.3}λ)", l / lam);
            let gap = d
              .spacings
              .get(i.wrapping_sub(1))
              .filter(|_| i > 0)
              .map_or_else(String::new, |s| format!("{s:.3} m ({:.3}λ)", s / lam));
            (i + 1, role, len, gap)
          })
          .collect::<Vec<_>>();
        let gain = format!("{:.2}", d.gain_dbi);
        let fb = format!("{:.1}", d.front_to_back_db);
        let swr = format!("{:.2}", d.swr_50);
        let evals = d.evaluations.to_string();
        view! {
          <div class="overflow-x-auto">
            <table data-slot="nec-design" class="w-full border-collapse text-xs">
              <thead class="bg-muted/50">
                <tr>
                  <th class="border px-2 py-1 text-left">"#"</th>
                  <th class="border px-2 py-1 text-left">
                    {move || t("tools.nec-design-element-col")}
                  </th>
                  <th class="border px-2 py-1 text-left">
                    {move || t("tools.nec-design-length-col")}
                  </th>
                  <th class="border px-2 py-1 text-left">
                    {move || t("tools.nec-design-spacing-col")}
                  </th>
                </tr>
              </thead>
              <tbody>
                {rows
                  .into_iter()
                  .map(|(idx, role, len, gap)| {
                    view! {
                      <tr>
                        <td class=CELL>{idx}</td>
                        <td class=CELL>{role}</td>
                        <td class=CELL>{len}</td>
                        <td class=CELL>{gap}</td>
                      </tr>
                    }
                  })
                  .collect_view()}
              </tbody>
            </table>
          </div>
          <div class=RESULT>
            {tf("tools.nec-design-result", &[&gain, &fb, &swr, &evals])}
          </div>
          <div class="flex flex-wrap items-center gap-2">
            <button
              type="button"
              class="cursor-pointer rounded-md border bg-background px-3 py-1 text-xs font-medium hover:bg-accent"
              on:click=load
            >
              {move || t("tools.nec-design-load")}
            </button>
            {move || {
              if loaded.get() {
                view! { <span class=NOTE>{move || t("tools.nec-design-loaded")}</span> }
                  .into_any()
              } else {
                ().into_any()
              }
            }}
          </div>
        }
        .into_any()
      }}
    </div>
  }
}
