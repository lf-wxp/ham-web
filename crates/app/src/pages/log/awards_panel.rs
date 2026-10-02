use crate::i18n::{t, tf};
use ham_web_core::award_progress::{
  AwardProgress, CONTINENTS, DXCC_TARGET, IOTA_TARGET, Progress, WAS_TARGET, WAZ_TARGET,
  WPX_TARGET, award_gaps, vucc_target,
};
use ham_web_core::logbook::LogEntry;
use leptos::prelude::*;

const CHIP_ON: &str =
  "rounded-md bg-primary px-2.5 py-1 text-xs font-medium text-primary-foreground";
const CHIP_OFF: &str =
  "rounded-md px-2.5 py-1 text-xs font-medium text-muted-foreground hover:bg-accent";

fn count<T: Ord>(p: &Progress<T>, confirmed: bool) -> usize {
  if confirmed {
    p.confirmed.len()
  } else {
    p.worked.len()
  }
}

/// 一行「名称 · n / 目标」进度条。
fn bar_row(label: String, n: usize, target: usize) -> impl IntoView {
  let pct = (n as f64 / target.max(1) as f64 * 100.0).min(100.0);
  let done = n >= target;
  view! {
    <div>
      <div class="mb-1 flex items-center justify-between text-xs">
        <span class="font-medium">{label}</span>
        <span class="tabular-nums text-muted-foreground">
          {format!("{n} / {target}")}
          {done.then(|| view! { <span class="ml-1.5 font-semibold text-emerald-600 dark:text-emerald-400">{move || t("已达标")}</span> })}
        </span>
      </div>
      <div class="h-2 w-full overflow-hidden rounded-full bg-muted">
        <div
          class=if done { "h-full rounded-full bg-emerald-500" } else { "h-full rounded-full bg-primary" }
          style=format!("width: {pct:.1}%")
        ></div>
      </div>
    </div>
  }
}

/// 日志联动的奖状进度：DXCC / WAZ / WAC / VUCC。
#[component]
pub(super) fn AwardsPanel(entries: Vec<LogEntry>) -> impl IntoView {
  let progress = StoredValue::new(AwardProgress::from_entries(&entries));
  let confirmed = RwSignal::new(false);

  view! {
    <section id="awards" class="scroll-mt-24 rounded-xl border bg-card">
      <div class="flex flex-wrap items-center gap-2 border-b px-4 py-3">
        <h3 class="mr-auto text-sm font-semibold">{move || t("奖状进度")}</h3>
        <button type="button" class=move || if confirmed.get() { CHIP_OFF } else { CHIP_ON } on:click=move |_| confirmed.set(false)>
          {move || t("已通联")}
        </button>
        <button type="button" class=move || if confirmed.get() { CHIP_ON } else { CHIP_OFF } on:click=move |_| confirmed.set(true)>
          {move || t("已确认（QSL）")}
        </button>
      </div>
      {move || {
        let c = confirmed.get();
        progress.with_value(|p| {
          let mut bands: Vec<(String, usize)> = p
            .dxcc_by_band
            .iter()
            .map(|(b, x)| (b.clone(), count(x, c)))
            .filter(|(_, n)| *n > 0)
            .collect();
          bands.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
          let modes: Vec<(&'static str, usize)> = ["CW", "Phone", "Digital"]
            .into_iter()
            .map(|m| (m, p.dxcc_by_mode.get(m).map_or(0, |x| count(x, c))))
            .collect();
          let vucc: Vec<(String, usize, usize)> = p
            .vucc
            .iter()
            .filter_map(|(b, x)| Some((b.clone(), count(x, c), vucc_target(b)?)))
            .filter(|(_, n, _)| *n > 0)
            .collect();
          let zones = if c { &p.waz.confirmed } else { &p.waz.worked };
          let conts = if c { &p.wac.confirmed } else { &p.wac.worked };
          let gaps = award_gaps(p, c);
          view! {
            <div class="space-y-5 p-4">
              {(!gaps.is_empty()).then(|| view! {
                <div class="rounded-lg border bg-muted/30 p-3">
                  <div class="mb-1.5 text-xs font-semibold">{move || t("冲刺建议")}</div>
                  <div class="flex flex-wrap gap-1.5">
                    {gaps
                      .iter()
                      .take(3)
                      .map(|g| {
                        view! {
                          <span class="rounded-full border bg-background px-2.5 py-1 text-xs">
                            {tf("{} 还差 {}", &[g.label, &g.remaining().to_string()])}
                          </span>
                        }
                      })
                      .collect_view()}
                  </div>
                  <p class="mt-1.5 text-[11px] text-muted-foreground">{move || t("越靠前越接近达标，优先冲刺")}</p>
                </div>
              })}
              <div class="space-y-3">
                {bar_row("DXCC".to_owned(), count(&p.dxcc, c), DXCC_TARGET)}
                <div class="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
                  {modes
                    .into_iter()
                    .map(|(m, n)| view! {
                      <span>{m} " " <span class="font-semibold tabular-nums text-foreground">{n}</span></span>
                    })
                    .collect_view()}
                </div>
                {(!bands.is_empty()).then(|| view! {
                  <div class="flex flex-wrap gap-1.5">
                    {bands
                      .into_iter()
                      .map(|(b, n)| view! {
                        <span class="rounded-full border bg-muted/40 px-2.5 py-0.5 text-xs">
                          {b} " " <span class="font-semibold tabular-nums">{n}</span>
                        </span>
                      })
                      .collect_view()}
                  </div>
                })}
              </div>

              <div class="space-y-2">
                {bar_row(t("WAZ（CQ 分区）"), zones.len(), WAZ_TARGET)}
                <div class="grid grid-cols-10 gap-1">
                  {(1..=40u8)
                    .map(|z| {
                      let on = zones.contains(&z);
                      view! {
                        <span
                          class=if on {
                            "rounded bg-primary py-0.5 text-center text-[10px] font-medium tabular-nums text-primary-foreground"
                          } else {
                            "rounded bg-muted py-0.5 text-center text-[10px] tabular-nums text-muted-foreground"
                          }
                          title=tf("CQ {} 区", &[&z.to_string()])
                        >
                          {z}
                        </span>
                      }
                    })
                    .collect_view()}
                </div>
              </div>

              <div class="space-y-2">
                {bar_row(t("WAC（六大洲）"), conts.len(), CONTINENTS.len())}
                <div class="flex flex-wrap gap-1.5">
                  {CONTINENTS
                    .iter()
                    .map(|(code, name)| {
                      let on = conts.contains(code);
                      view! {
                        <span class=if on {
                          "rounded-full bg-primary px-2.5 py-0.5 text-xs text-primary-foreground"
                        } else {
                          "rounded-full border px-2.5 py-0.5 text-xs text-muted-foreground"
                        }>
                          {format!("{name} {code}")}
                        </span>
                      }
                    })
                    .collect_view()}
                </div>
              </div>

              {(!vucc.is_empty()).then(|| view! {
                <div class="space-y-3">
                  {vucc
                    .into_iter()
                    .map(|(b, n, target)| {
                      let label = if b == "SAT" { t("VUCC 卫星") } else { format!("VUCC {b}") };
                      bar_row(label, n, target)
                    })
                    .collect_view()}
                </div>
              })}

              <div class="space-y-3">
                <div class="flex items-center justify-between text-xs">
                  <span class="font-medium">{"DXCC Challenge"}</span>
                  <span class="tabular-nums text-muted-foreground">{p.dxcc_challenge}</span>
                </div>
                {bar_row("WPX".to_owned(), count(&p.wpx, c), WPX_TARGET)}
                {bar_row(t("WAS（美国州）"), count(&p.was, c), WAS_TARGET)}
                {bar_row(t("IOTA（岛屿组）"), count(&p.iota, c), IOTA_TARGET)}
              </div>

              <p class="text-xs text-muted-foreground">
                {move || t("未填写 CQ / ITU 分区的记录按 DXCC 实体的主分区估算，跨多个分区的国家（如美国、俄罗斯、中国）可能不准，可在记录的「更多字段」里手动填写。VUCC 按 6m 及以上波段的 4 位网格统计。正式申请以 ARRL / CQ 的确认规则为准。")}
              </p>
            </div>
          }
        })
      }}
    </section>
  }
}
