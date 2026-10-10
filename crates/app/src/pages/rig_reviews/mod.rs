//! 设备评测文章：固定模板「测试条件 → 指标实测 → 主观评价 → 适用人群 → 结论」。
//!
//! 三条立场在这里落成界面：
//! - **条件在前**：每篇评测最先渲染测试条件，并写明「数字与感受都依赖条件」；
//! - **指标实测与机型库同源**：数值由 `gear_rx` 现场生成（带出处），评测不自己抄数字；
//! - **主观与实测分开**：主观条目明确标注，脚注写明「基于公开资料与社区共识，不是本站实测」。
//!
//! 本目录按「一个组件一个文件」组织：页面在这里，「测试条件」的一行在 `condition_row.rs`。

mod condition_row;

use ham_web_core::gear;
use ham_web_core::gear_rx;
use ham_web_core::gear_score::Axis;
use ham_web_core::rig_reviews::{REVIEW_GLOSSARY, REVIEW_METRICS, RIG_REVIEWS, metric_value};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::data;
use crate::i18n::{t, tf};
use crate::util::set_title;

use condition_row::ConditionRow;

const CELL: &str = "border px-3 py-2 text-left align-top";

/// 知识库正文译文：先订阅加载状态（切语言后要重算），再查词典。
fn text(zh: &'static str) -> String {
  data::track_knowledge();
  data::kt(zh)
}

/// 指标实测那一行的数值文本（含窄间隔的间隔本身）。
fn metric_text(glossary: usize, rx: Option<&gear_rx::GearRx>) -> String {
  let row = REVIEW_METRICS
    .iter()
    .find(|r| r.glossary == glossary)
    .expect("指标行与术语表一一对应（核心有单测）");
  let Some(v) = metric_value(row, rx) else {
    return t("knowledge.not-listed");
  };
  match row.axis {
    Axis::ImdNarrow => format!("{v:.0} dB（{:.0} kHz）", rx.map_or(0.0, |r| r.narrow_khz)),
    Axis::NoiseFloor => format!("{v:.0} dBm"),
    // 其余是 ImdWide；`metric_value` 对非实测轴返回 `None`（上面已提前返回「未收录」），
    // 这里不 panic —— wasm 里一个 `unreachable!()` 会让整页挂掉。
    _ => format!("{v:.0} dB"),
  }
}

#[component]
pub fn RigReviewsPage() -> impl IntoView {
  set_title("knowledge.rig-reviews");
  // 本页自带页头、没套 KnowledgePage 外壳，正文译文（条件 / 主观 / 结论等）得自己确保在拉。
  Effect::new(move |_| data::ensure_knowledge_i18n());

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("knowledge.rig-reviews")
        subtitle=move || t("knowledge.rig-reviews-subtitle")
        actions=ViewFn::from(move || {
          view! {
            <a
              href="/gear"
              class="rounded-full border px-3 py-1 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
            >
              {move || t("knowledge.rig-reviews-back-to-gear")}
            </a>
          }
        })
      />

      <PageContainer>
        <p class="text-xs text-muted-foreground">
          {move || t("knowledge.rig-reviews-stance")}
        </p>

        // ── 每篇评测 ──────────────────────────────────────────
        {RIG_REVIEWS
          .iter()
          .map(|review| {
            let gear = gear::by_id(review.rig_id).expect("核心有单测：评测只挂真实机型");
            let rx = gear_rx::for_gear(review.rig_id);
            view! {
              <article class="rounded-xl border bg-card">
                <div class="flex flex-wrap items-baseline gap-2 border-b px-4 py-3">
                  <h2 class="text-sm font-semibold">{format!("{} {}", gear.brand, gear.model)}</h2>
                  <span class="text-xs text-muted-foreground">
                    {move || format!("{} · {}", text(gear.tier), text(gear.bands))}
                  </span>
                </div>

                // 一、测试条件 —— 结构上永远在最前。
                <section class="border-b px-4 py-3">
                  <h3 class="text-xs font-semibold text-primary">
                    {move || t("knowledge.review-conditions")}
                  </h3>
                  <p class="mt-0.5 mb-2 text-[11px] text-muted-foreground">
                    {move || t("knowledge.review-conditions-hint")}
                  </p>
                  <div class="grid gap-1 sm:grid-cols-2">
                    // 「天线」「仪表」「指标」这几个词词典里已有（log.antenna / common.meters /
                    // knowledge.metrics）：中文释义全库唯一，同义的 key 只能复用。
                    <ConditionRow
                      label=Signal::derive(move || t("log.antenna"))
                      value=review.conditions.antenna
                    />
                    <ConditionRow
                      label=Signal::derive(move || t("knowledge.review-environment"))
                      value=review.conditions.environment
                    />
                    <ConditionRow
                      label=Signal::derive(move || t("common.meters"))
                      value=review.conditions.instruments
                    />
                    <ConditionRow
                      label=Signal::derive(move || t("knowledge.review-when"))
                      value=review.conditions.when
                    />
                  </div>
                </section>

                // 二、指标实测 —— 数值与机型库同源，不抄一份。
                <section class="border-b px-4 py-3">
                  <h3 class="text-xs font-semibold text-primary">
                    {move || t("knowledge.review-measured")}
                  </h3>
                  <div class="mt-2 overflow-x-auto">
                    <table class="w-full border-collapse text-sm">
                      <thead class="bg-muted/60 text-xs">
                        <tr>
                          <th class=CELL>{move || t("knowledge.metrics")}</th>
                          <th class=CELL>{move || t("knowledge.review-value")}</th>
                          <th class=CELL>{move || t("knowledge.review-how-to-measure")}</th>
                        </tr>
                      </thead>
                      <tbody>
                        {REVIEW_METRICS
                          .iter()
                          .map(|row| {
                            let glossary = row.glossary;
                            let href = format!("#glossary-{glossary}");
                            let name = REVIEW_GLOSSARY[glossary].0;
                            let meaning = REVIEW_GLOSSARY[glossary].1;
                            let how = REVIEW_GLOSSARY[glossary].2;
                            view! {
                              <tr class="border-t transition-colors hover:bg-muted/40">
                                <td class=format!("{CELL} whitespace-nowrap font-medium")>
                                  // 指标名互链到页面底部的术语表；悬停出释义。
                                  <a
                                    href=href
                                    title=move || text(meaning)
                                    class="underline decoration-dotted underline-offset-2 hover:text-foreground"
                                  >
                                    {move || text(name)}
                                  </a>
                                </td>
                                <td class=format!("{CELL} font-mono")>{move || metric_text(glossary, rx)}</td>
                                <td class=format!("{CELL} text-muted-foreground")>{move || text(how)}</td>
                              </tr>
                            }
                          })
                          .collect_view()}
                      </tbody>
                    </table>
                  </div>
                  {if rx.is_some() {
                    view! {
                      <p class="mt-2 text-[11px] text-muted-foreground">
                        {move || {
                          tf(
                            "knowledge.rx-metrics-from",
                            &[&format!("{}({})", gear_rx::SOURCE, gear_rx::CHECKED_ON)],
                          )
                        }}
                        " "
                        <a
                          href=gear_rx::SOURCE_URL
                          target="_blank"
                          rel="noreferrer"
                          class="underline underline-offset-2 hover:text-foreground"
                        >
                          {gear_rx::SOURCE_URL}
                        </a>
                      </p>
                    }
                      .into_any()
                  } else {
                    view! {
                      <p class="mt-2 text-[11px] text-muted-foreground">
                        {move || t("knowledge.review-measure-yourself")}
                        " "
                        <a
                          href="/gear"
                          class="underline underline-offset-2 hover:text-foreground"
                        >
                          {move || t("knowledge.review-go-to-gear")}
                        </a>
                      </p>
                    }
                      .into_any()
                  }}
                </section>

                // 三、主观评价（明确标注，不冒充测量）
                <section class="border-b px-4 py-3">
                  <h3 class="text-xs font-semibold text-primary">
                    {move || t("knowledge.review-subjective")}
                  </h3>
                  <ul class="mt-2 space-y-1.5">
                    {review
                      .subjective
                      .iter()
                      .map(|line| {
                        let line = *line;
                        view! {
                          <li class="flex gap-2 text-sm text-muted-foreground">
                            <span class="mt-0.5 shrink-0 text-primary">"•"</span>
                            <span>{move || text(line)}</span>
                          </li>
                        }
                      })
                      .collect_view()}
                  </ul>
                </section>

                // 四、适用人群 / 五、结论
                <section class="px-4 py-3">
                  <div class="grid gap-2 sm:grid-cols-2">
                    <div class="flex flex-col gap-0.5 rounded-lg px-3 py-1.5">
                      <span class="text-[11px] text-muted-foreground">
                        {move || t("knowledge.review-for-whom")}
                      </span>
                      <span class="text-sm">{move || text(review.for_whom)}</span>
                    </div>
                    <div class="flex flex-col gap-0.5 rounded-lg px-3 py-1.5">
                      <span class="text-[11px] text-muted-foreground">
                        {move || t("knowledge.review-conclusion")}
                      </span>
                      <span class="text-sm font-medium">{move || text(review.conclusion)}</span>
                    </div>
                  </div>
                </section>
              </article>
            }
          })
          .collect_view()}

        // ── 指标名词表（互链目标）──────────────────────────────
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">
            {move || t("knowledge.review-glossary")}
          </h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {REVIEW_GLOSSARY
              .iter()
              .enumerate()
              .map(|(i, (name, meaning, how))| {
                let name = *name;
                let meaning = *meaning;
                let how = *how;
                view! {
                  <div id=format!("glossary-{i}") class="flex flex-col gap-1 rounded-lg px-3 py-2">
                    <span class="text-sm font-medium">{move || text(name)}</span>
                    <span class="text-sm text-muted-foreground">{move || text(meaning)}</span>
                    <span class="text-xs text-muted-foreground">{move || text(how)}</span>
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
