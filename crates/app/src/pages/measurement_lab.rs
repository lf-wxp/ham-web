//! 测量实验室：驻波读数误差的交互演示 + SOLT 校准分步演示 + 读数陷阱速查。
//!
//! 数学在 `ham_web_core::meters`（有单测），这里只负责交互与画图。

use ham_web_core::meters::{SOLT_STEPS, SWR_PITFALLS, swr_at_antenna};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::data;
use crate::i18n::{t, tf};
use crate::ui::{Button, Chip, ChipGroup, Field, Size, Slider, Variant};
use crate::util::{set_title, unique_id};

/// 图表横轴：表头读数 1..6 → 40..305 px。
fn chart_x(meter_swr: f64) -> f64 {
  40.0 + (meter_swr - 1.0) * 53.0
}

/// 图表纵轴：天线端真实驻波 1..12 → 200..20 px。
fn chart_y(true_swr: f64) -> f64 {
  200.0 - (true_swr - 1.0) * (180.0 / 11.0)
}

/// 表头读数 → 天线端真实驻波（图表上截断到 12:1，避免曲线顶破画布）。
fn true_swr_clamped(meter_swr: f64, loss_db: f64) -> f64 {
  let true_swr = swr_at_antenna(meter_swr, loss_db);
  true_swr.min(12.0)
}

/// 演示 A 的曲线点（SVG `points` 字符串）。
fn curve_points(loss_db: f64) -> String {
  (1..=24)
    .map(|i| {
      let m = 1.0 + f64::from(i) * 5.0 / 24.0;
      format!(
        "{:.1},{:.1}",
        chart_x(m),
        chart_y(true_swr_clamped(m, loss_db))
      )
    })
    .collect::<Vec<_>>()
    .join(" ")
}

/// 知识库正文译文：先订阅加载状态（切语言后要重算），再查词典。
fn text(zh: &'static str) -> String {
  data::track_knowledge();
  data::kt(zh)
}

/// SOLT 当前步骤的界面文案。
fn solt_step_text(step: usize) -> (String, String) {
  let (title, desc) = SOLT_STEPS[step.min(SOLT_STEPS.len() - 1)];
  (text(title), text(desc))
}

#[component]
pub fn MeasurementLabPage() -> impl IntoView {
  set_title("measurement.lab-title");
  // 本页自带页头、没套 KnowledgePage 外壳，正文译文（SOLT / 读数陷阱）得自己确保在拉。
  Effect::new(move |_| data::ensure_knowledge_i18n());
  // 演示 A：馈线单程损耗（dB）。
  let loss_db = RwSignal::new(0.5f64);
  // 演示 B：SOLT 步骤下标（0..SOLT_STEPS.len()）。
  let solt_step = RwSignal::new(0usize);
  let loss_id = unique_id("measurement-line-loss");
  let loss_label =
    Signal::derive(move || tf("measurement.line-loss", &[&format!("{:.1}", loss_db.get())]));

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=Signal::derive(move || t("measurement.lab-title"))
        subtitle=Signal::derive(move || t("measurement.lab-subtitle"))
      />

      <PageContainer>
        // ── 演示 A：馈线掩盖 ──────────────────────────────────
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">
            {move || t("measurement.swr-demo-title")}
          </h2>
          <div class="grid gap-4 px-4 py-3 md:grid-cols-[240px_1fr]">
            <div class="space-y-3">
              <p class="text-xs text-muted-foreground">
                {move || t("measurement.swr-demo-hint")}
              </p>
              // 标签随滑块实时显示当前损耗；无障碍名称不带数值（数值走 valuetext），
              // 否则读屏会读出模板里的 `{}`。
              <Field label=loss_label r#for=loss_id.clone()>
                <Slider
                  id=loss_id
                  value=Signal::derive(move || loss_db.get())
                  on_change=Callback::new(move |v: f64| loss_db.set(v))
                  min=0.2
                  max=3.0
                  step=0.1
                  aria_label=Signal::derive(move || t("measurement.line-loss-name"))
                  aria_valuetext=loss_label
                />
              </Field>
              <p class="text-sm">
                {move || {
                  // 定点读值：表头读到 3:1 时，天线端到底是多少。
                  tf(
                    "measurement.meter-vs-true",
                    &[
                      "3.0",
                      &format!("{:.1}", swr_at_antenna(3.0, loss_db.get()).min(999.0)),
                    ],
                  )
                }}
              </p>
              <p class="text-xs text-muted-foreground">
                {move || t("measurement.directivity-note")}
              </p>
            </div>
            <svg viewBox="0 0 320 230" class="w-full" role="img" aria-label=move || t("measurement.swr-chart")>
              <title>{move || t("measurement.swr-chart")}</title>
              // 网格：真实驻波 1 / 3 / 6 / 9 / 12。
              {[1.0f64, 3.0, 6.0, 9.0, 12.0].iter().map(|&g| {
                let y = chart_y(g);
                view! {
                  <g>
                    <line x1="40" x2="305" y1=y y2=y class="stroke-border" stroke-width="1" />
                    <text x="0" y=y + 3.0 class="fill-muted-foreground" font-size="9">
                      {format!("{g:.0}")}
                    </text>
                  </g>
                }
              }).collect_view()}
              // 对角线 = 无损馈线（读数 = 真实）：横轴到 6:1 的终点，纵轴也落在真实 6:1。
              <line
                x1=chart_x(1.0)
                y1=chart_y(1.0)
                x2=chart_x(6.0)
                y2=chart_y(6.0)
                stroke-width="1"
                stroke-dasharray="4 3"
                vector-effect="non-scaling-stroke"
                class="stroke-muted-foreground"
              />
              // 有损耗时的曲线。
              <polyline
                points=move || curve_points(loss_db.get())
                fill="none"
                class="stroke-primary"
                stroke-width="1.5"
                stroke-linejoin="round"
              />
              <text x="40" y="220" class="fill-muted-foreground" font-size="9">
                {move || t("measurement.meter-reading")}
              </text>
              <text x="13" y="115" class="fill-muted-foreground" font-size="9" text-anchor="middle" transform="rotate(-90 13 115)">
                {move || t("measurement.true-swr")}
              </text>
            </svg>
          </div>
        </section>

        // ── 演示 B：SOLT 校准分步 ─────────────────────────────
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">
            {move || t("measurement.solt-title")}
          </h2>
          <div class="px-4 py-3">
            <p class="text-xs text-muted-foreground">
              {move || t("measurement.solt-hint")}
            </p>
            <ChipGroup
              class="mt-3"
              value=Signal::derive(move || solt_step.get().to_string())
              on_change=Callback::new(move |v: String| {
                if let Ok(i) = v.parse::<usize>() {
                  solt_step.set(i.min(SOLT_STEPS.len() - 1));
                }
              })
              aria_label=Signal::derive(move || t("measurement.solt-title"))
            >
              {SOLT_STEPS
                .iter()
                .enumerate()
                .map(|(i, (title, _))| {
                  let title = *title;
                  view! { <Chip value=i.to_string()>{move || text(title)}</Chip> }
                })
                .collect_view()}
            </ChipGroup>
            <div class="mt-3 rounded-lg bg-muted/50 px-3 py-2">
              {move || {
                let (title, desc) = solt_step_text(solt_step.get());
                view! {
                  <p class="text-sm font-medium">{title}</p>
                  <p class="mt-1 text-sm text-muted-foreground">{desc}</p>
                }
              }}
            </div>
            <div class="mt-3 flex items-center gap-2">
              <Button
                variant=Variant::Outline
                size=Size::Sm
                on_click=Callback::new(move |()| {
                  solt_step.update(|s| *s = (*s + 1) % SOLT_STEPS.len());
                })
              >
                {move || t("measurement.solt-next")}
              </Button>
            </div>
          </div>
        </section>

        // ── 读数陷阱速查 ──────────────────────────────────────
        // 标题是界面文案（t），条目是知识内容（kt）：两类词典不同，不能混用 ConceptsSection 的 ui 开关。
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">
            {move || t("measurement.pitfalls-title")}
          </h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {SWR_PITFALLS
              .iter()
              .map(|(title, desc)| {
                let title = *title;
                let desc = *desc;
                view! {
                  <div class="flex flex-col gap-1 rounded-lg px-3 py-2">
                    <span class="text-sm font-medium">{move || text(title)}</span>
                    <span class="text-sm text-muted-foreground">{move || text(desc)}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>
      </PageContainer>
    </div>
  }
}
