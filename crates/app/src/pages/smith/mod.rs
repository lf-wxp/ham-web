//! 史密斯圆图与匹配：可拖拽的阻抗定位 + L 型匹配网络求解。

mod chart;

use ham_web_core::smith::{
  Element, gamma, l_match, matched_bandwidth, series_element, series_reactance, shunt_element,
  shunt_susceptance, swr_and_return_loss,
};
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::ui::{
  Button, ControlSize, NumberField, RadioGroup, RadioGroupItem, Size, Slider, Variant,
};
use crate::util::set_title;

use chart::InteractiveChart;

/// 特性阻抗（Ω）。
const Z0: f64 = 50.0;

/// 判断「可用带宽」的驻波比上限（业余界常用的工程判据）。
const BAND_SWR_LIMIT: f64 = 2.0;

const NOTE: &str = "text-xs text-muted-foreground";
const RESULT: &str = "rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground";

/// 元件文案。
fn element_text(e: Element) -> String {
  if let Some(l) = e.l_uh {
    format!("L = {l:.3} µH")
  } else if let Some(c) = e.c_pf {
    format!("C = {c:.2} pF")
  } else {
    "—".to_owned()
  }
}

/// 复阻抗文案：`R ± jX`（固定一位小数）。符号逻辑与 `/nec` 共用实现。
fn fmt_z(r: f64, x: f64) -> String {
  crate::util::fmt_z(r, x, |v| format!("{v:.1}"))
}

#[component]
pub fn SmithPage() -> impl IntoView {
  set_title("tools.smith-chart-and-matching");

  let r_ohm = RwSignal::new(100.0);
  let x_ohm = RwSignal::new(-50.0);
  let series_x = RwSignal::new(0.0);
  let shunt_b = RwSignal::new(0.0);
  let shunt_first = RwSignal::new(false);
  let freq_mhz = RwSignal::new(String::from("7.1"));

  let freq_hz = move || {
    freq_mhz
      .get()
      .trim()
      .parse::<f64>()
      .unwrap_or(7.1)
      .clamp(0.1, 6000.0)
      * 1e6
  };

  let load_z = move || (r_ohm.get() / Z0, x_ohm.get() / Z0);
  let load_stats = move || {
    let (gr, gi) = gamma(load_z().0, load_z().1);
    swr_and_return_loss(gr, gi)
  };

  // 匹配后的阻抗：与圆图上画的轨迹用同一套运算，避免两处给出不同答案。
  let matched_z = move || {
    let x_add = series_x.get() / Z0;
    let b_add = shunt_b.get() / 1000.0 * Z0;
    let z = load_z();
    if shunt_first.get() {
      series_reactance(shunt_susceptance(z, b_add), x_add)
    } else {
      shunt_susceptance(series_reactance(z, x_add), b_add)
    }
  };
  let matched_stats = move || {
    let z = matched_z();
    let (gr, gi) = gamma(z.0, z.1);
    swr_and_return_loss(gr, gi)
  };

  let series_el = move || series_element(series_x.get() / Z0, freq_hz(), Z0);
  let shunt_el = move || shunt_element(shunt_b.get() * Z0 / 1000.0, freq_hz(), Z0);
  // SWR ≤ 2 的可用带宽（Hz）；中心频率上就没达标时返回 None。
  let bandwidth = move || {
    matched_bandwidth(
      (r_ohm.get(), x_ohm.get()),
      series_x.get(),
      shunt_b.get() / 1000.0,
      shunt_first.get(),
      freq_hz(),
      Z0,
      BAND_SWR_LIMIT,
    )
  };

  // 自动匹配：把 L 型网络的第一组解填进两个元件。
  let auto_match = move |_| {
    let z = load_z();
    let Some(sols) = l_match(z.0, z.1) else {
      return;
    };
    let m = sols[0];
    shunt_first.set(m.shunt_first);
    series_x.set(m.x * Z0);
    shunt_b.set(m.b / Z0 * 1000.0);
  };

  let reset = move |_| {
    series_x.set(0.0);
    shunt_b.set(0.0);
  };

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("tools.smith-chart-and-matching")}</h1>
            <div class="text-xs text-muted-foreground">
              {move || t("tools.drag-to-place-the")}
            </div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section id="chart" class="scroll-mt-24 rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.impedance-location-and-matching")}</h2>
          <p class="px-4 pt-3 text-xs text-muted-foreground">
            {move || t("tools.the-chart-is-normalised")}
          </p>

          <div class="grid gap-5 p-4 sm:grid-cols-[minmax(0,360px)_1fr] sm:items-start">
            <InteractiveChart
              r_ohm=r_ohm
              x_ohm=x_ohm
              series_x=series_x
              shunt_b=shunt_b
              shunt_first=shunt_first
            />

            <div class="flex flex-col gap-4">
              <div class="flex flex-col gap-2">
                <span class=NOTE>{move || tf("tools.load-resistance-r-log", &[&format!("{:.1}", r_ohm.get())])}</span>
                <Slider
                  value=Signal::derive(move || r_ohm.get().max(1.0).log10())
                  on_change=Callback::new(move |v: f64| r_ohm.set(10f64.powf(v)))
                  min=0.0
                  max=3.7
                  step=0.01
                  aria_label=Signal::derive(move || t("tools.load-resistance"))
                  aria_valuetext=Signal::derive(move || format!("{:.1} Ω", r_ohm.get()))
                />
              </div>
              <div class="flex flex-col gap-2">
                <span class=NOTE>{move || tf("tools.load-reactance-x-inductive", &[&format!("{:.1}", x_ohm.get())])}</span>
                <Slider
                  value=Signal::derive(move || x_ohm.get())
                  on_change=Callback::new(move |v: f64| x_ohm.set(v))
                  min=-500.0
                  max=500.0
                  step=1.0
                  aria_label=Signal::derive(move || t("tools.load-reactance"))
                  aria_valuetext=Signal::derive(move || format!("{:.0} Ω", x_ohm.get()))
                />
              </div>
              <div class=RESULT>
                {move || {
                  let z = load_z();
                  let (gr, gi) = gamma(z.0, z.1);
                  let (swr, rl) = load_stats();
                  let z_text = fmt_z(r_ohm.get(), x_ohm.get());
                  let gamma_mag = format!("{:.3}", gr.hypot(gi));
                  let gamma_deg = format!("{:.1}", gi.atan2(gr).to_degrees());
                  let swr_s = format!("{swr:.2}");
                  let rl_s = format!("{rl:.1}");
                  tf(
                    "tools.load-z-swr-return",
                    &[&z_text, &gamma_mag, &gamma_deg, &swr_s, &rl_s],
                  )
                }}
              </div>
            </div>
          </div>
        </section>

        <section id="match" class="scroll-mt-24 rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.matching-network")}</h2>
          <div class="flex flex-wrap items-center gap-2 px-4 pt-3">
            // 传 `t("key")` 的闭包而不是中文原文：`check-i18n` 判死条目时认「`t("key")` 的
            // 直接实参」或「中文原文出现在源码里」，传中文会一直依赖反向索引这条兜底路径
            // （`crates/tools/src/i18n.rs` 的 `scan_calls`）。
            <RadioGroup
              value=Signal::derive(move || {
                if shunt_first.get() { "shunt-first" } else { "series-first" }.to_owned()
              })
              on_change=Callback::new(move |v: String| shunt_first.set(v == "shunt-first"))
              aria_label=Signal::derive(move || t("tools.matching-network"))
              class="flex items-center gap-4"
            >
              <label class="flex cursor-pointer items-center gap-2 text-xs">
                <RadioGroupItem value="series-first" />
                {move || t("tools.series-then-shunt")}
              </label>
              <label class="flex cursor-pointer items-center gap-2 text-xs">
                <RadioGroupItem value="shunt-first" />
                {move || t("tools.shunt-then-series")}
              </label>
            </RadioGroup>
            <Button
              variant=Variant::Default
              size=Size::Sm
              class="ml-2"
              on_click=Callback::new(move |_| auto_match(()))
            >
              {move || t("tools.auto-l-match")}
            </Button>
            <Button
              variant=Variant::Outline
              size=Size::Sm
              on_click=Callback::new(move |_| reset(()))
            >
              {move || t("tools.clear-elements")}
            </Button>
          </div>

          <div class="grid gap-4 p-4 sm:grid-cols-3">
            <div class="flex flex-col gap-2">
              <span class=NOTE>{move || tf("tools.series-reactance-2", &[&format!("{:.0}", series_x.get())])}</span>
              <Slider
                value=Signal::derive(move || series_x.get())
                on_change=Callback::new(move |v: f64| series_x.set(v))
                min=-500.0
                max=500.0
                step=1.0
                aria_label=Signal::derive(move || t("tools.series-reactance"))
                aria_valuetext=Signal::derive(move || format!("{:.0} Ω", series_x.get()))
              />
            </div>
            <div class="flex flex-col gap-2">
              <span class=NOTE>{move || tf("tools.shunt-susceptance-ms-capacitive", &[&format!("{:.1}", shunt_b.get())])}</span>
              <Slider
                value=Signal::derive(move || shunt_b.get())
                on_change=Callback::new(move |v: f64| shunt_b.set(v))
                min=-40.0
                max=40.0
                step=0.2
                aria_label=Signal::derive(move || t("tools.shunt-susceptance"))
                aria_valuetext=Signal::derive(move || format!("{:.1} mS", shunt_b.get()))
              />
            </div>
            <label class="flex flex-col gap-1.5">
              <span class=NOTE>{move || t("tools.conversion-frequency-mhz")}</span>
              <NumberField
                value=Signal::derive(move || freq_mhz.get())
                on_change=Callback::new(move |v: String| freq_mhz.set(v))
                step=0.1
                min=0.1
                max=6000.0
                size=ControlSize::Sm
                aria_label=Signal::derive(move || t("tools.conversion-frequency"))
              />
            </label>
          </div>

          <div class="space-y-2 px-4 pb-4">
            <div class=RESULT>
              {move || {
                tf(
                  "tools.series-element-shunt-element",
                  &[&element_text(series_el()), &element_text(shunt_el())],
                )
              }}
            </div>
            <div class=RESULT>
              {move || {
                let z = matched_z();
                let (swr, rl) = matched_stats();
                // 全部先绑定成局部量：`tf` 只接受 `&[&str]`，直接在数组里写
                // `&format!(…)` / `&t(…)` 会让临时值在语句结束时就被释放。
                let z_text = fmt_z(z.0 * Z0, z.1 * Z0);
                let swr_s = format!("{swr:.3}");
                let rl_s = format!("{rl:.1}");
                let verdict = if swr < 1.1 { t("tools.matched") } else { t("tools.still-needs-tuning") };
                tf(
                  "tools.matched-z-swr-return",
                  &[&z_text, &swr_s, &rl_s, &verdict],
                )
              }}
            </div>
            <div class=RESULT>
              {move || match bandwidth() {
                Some((lo, hi)) => {
                  let lo_s = format!("{:.3}", lo / 1e6);
                  let hi_s = format!("{:.3}", hi / 1e6);
                  let span_s = format!("{:.0}", (hi - lo) / 1e3);
                  let rel_s = format!("{:.1}", (hi - lo) / freq_hz() * 100.0);
                  tf(
                    "common.matched-bandwidth-swr-2",
                    &[&lo_s, &hi_s, &span_s, &rel_s],
                  )
                }
                None => t("tools.the-current-component-set"),
              }}
            </div>
            <p class=NOTE>
              {move || t("tools.a-series-capacitor-or")}
            </p>
            <p class=NOTE>
              {move || t("tools.the-bandwidth-is-derived")}
            </p>
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.related-tools")}</h2>
          <div class="grid gap-2 p-4 sm:grid-cols-2">
            <a class="rounded-lg px-3 py-2 text-sm hover:bg-accent" href="/tools#antenna-matcher">
              {move || t("tools.calculators-l-network-antenna")}
            </a>
            <a class="rounded-lg px-3 py-2 text-sm hover:bg-accent" href="/tools#swr">
              {move || t("tools.calculators-swr-reflection-coefficient")}
            </a>
            <a class="rounded-lg px-3 py-2 text-sm hover:bg-accent" href="/waveform-lab#filter">
              {move || t("tools.waveform-lab-filter-response")}
            </a>
            <a class="rounded-lg px-3 py-2 text-sm hover:bg-accent" href="/feedline">
              {move || t("tools.knowledge-matching-and-feedline")}
            </a>
          </div>
        </section>
      </div>
    </div>
  }
}
