use ham_web_core::contest::{ContestDef, MultKind};
use ham_web_core::contest_analysis::analyze;
use ham_web_core::dxcc::{Entity, entity_by_dxcc};
use ham_web_core::logbook::stamp;
use leptos::prelude::*;

use crate::i18n::{Locale, locale, t, tf, tp};
use crate::pages::log::LogEntry;
use crate::ui::{
  Field, NativeSelect, SelectOption, Table, TableBody, TableCell, TableHead, TableHeader, TableRow,
};
use crate::util::unique_id;

/// 空闲段下限可选值（分钟）。
const IDLE_CHOICES: [i64; 3] = [15, 30, 60];
/// 跨波段缺口最多列几条（已按「缺得最少」排序，列前面的就是最值得补的）。
const MAX_CROSS_ROWS: usize = 30;

/// `MM-DD HH:MM`（UTC）。
fn clock(minutes: i64) -> String {
  let (date, hm) = stamp(minutes);
  format!("{} {hm}", date.get(5..).unwrap_or(&date))
}

/// `2 h 10 min` / `45 min`：各语言通用，不需要词条。
fn duration(minutes: i64) -> String {
  if minutes >= 60 {
    format!("{} h {} min", minutes / 60, minutes % 60)
  } else {
    format!("{minutes} min")
  }
}

fn has_cjk(s: &str) -> bool {
  s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
}

/// 实体名：有译文用译文，没有时非中文界面退回英文名，而不是漏出中文。
fn entity_label(e: &Entity) -> String {
  let name = t(e.name);
  if locale().get() != Locale::Zh && has_cjk(&name) {
    format!("{} ({})", e.name_en, e.prefix)
  } else {
    format!("{name} ({})", e.prefix)
  }
}

/// 赛后分析：每小时速率、空闲时段、乘数缺口。数字全部来自 `ham_web_core::contest_analysis`，
/// 与自报分数共用同一套记分规则。
#[component]
pub(super) fn ContestAnalysis(
  entries: Memo<Vec<LogEntry>>,
  def: Memo<&'static ContestDef>,
  my_call: Memo<String>,
) -> impl IntoView {
  let idle_min = RwSignal::new(30_i64);
  let idle_id = unique_id("contest-idle-min");
  let idle_options: Vec<SelectOption> = IDLE_CHOICES
    .iter()
    .map(|&m| {
      SelectOption::new(
        m.to_string(),
        Signal::derive(move || tf("radio.min", &[&m.to_string()])),
      )
    })
    .collect();

  let analysis = Memo::new(move |_| {
    entries.with(|list| analyze(def.get(), list, &my_call.get(), idle_min.get()))
  });

  view! {
    <section class="pxl-card border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("contest.analysis-heading")}</h2>
      {move || match analysis.get() {
        None => view! {
          <p class="px-4 py-6 text-sm text-muted-foreground">{move || t("contest.analysis-empty")}</p>
        }
        .into_any(),
        Some(a) => {
          let max_qsos = a.hours.iter().map(|h| h.qsos).max().unwrap_or(0).max(1);
          let bands = a.bands.len();
          // `Field` 的子节点闭包会把 id 整个 move 走，所以每次渲染各取一份克隆。
          let field_id = idle_id.clone();
          let select_id = idle_id.clone();
          let options = idle_options.clone();
          view! {
            <div class="space-y-6 p-4">
              {(a.untimed > 0).then(|| view! {
                <p class="border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-xs">
                  {tp("contest.analysis-untimed", a.untimed, &[&a.untimed.to_string()])}
                </p>
              })}

              // ── 每小时速率 ──────────────────────────────
              <div class="space-y-2">
                <h3 class="text-sm font-semibold">{move || t("contest.analysis-hourly")}</h3>
                {a.best_hour.map(|b| view! {
                  <p class="text-sm">
                    {tp("contest.analysis-best-hour", b.qsos, &[&b.qsos.to_string(), &clock(b.start)])}
                  </p>
                })}
                <Table class="text-sm" wrapper_class="max-h-96 border">
                  <TableHead class="sticky top-0 z-10 bg-card text-xs text-muted-foreground">
                    <TableRow class="border-b">
                      <TableHeader class="whitespace-nowrap px-3 py-2 font-normal">{move || t("contest.analysis-hour")}</TableHeader>
                      <TableHeader class="px-2 py-2 text-right font-normal">{move || t("contest.analysis-qsos")}</TableHeader>
                      <TableHeader class="px-2 py-2 text-right font-normal">{move || t("contest.points")}</TableHeader>
                      <TableHeader class="px-2 py-2 text-right font-normal">{move || t("contest.analysis-new-mults")}</TableHeader>
                      <TableHeader class="hidden w-1/3 px-3 py-2 font-normal sm:table-cell"><span class="sr-only">{move || t("contest.analysis-rate-bar")}</span></TableHeader>
                    </TableRow>
                  </TableHead>
                  <TableBody class="font-mono">
                    {a.hours.iter().map(|h| {
                      // 组件 children 需要 `'static`：把借来的字段先拷贝成值。
                      let pct = u64::from(h.qsos) * 100 / u64::from(max_qsos);
                      let muted = if h.qsos == 0 { "text-muted-foreground" } else { "" };
                      let start = clock(h.start);
                      let qsos = h.qsos;
                      let points = h.points;
                      let new_mults = h.new_mults;
                      view! {
                        <TableRow class=format!("border-b last:border-0 {muted}")>
                          <TableCell class="whitespace-nowrap px-3 py-1.5 tabular-nums">{start}</TableCell>
                          <TableCell class="px-2 py-1.5 text-right tabular-nums">{qsos}</TableCell>
                          <TableCell class="px-2 py-1.5 text-right tabular-nums">{points}</TableCell>
                          <TableCell class="px-2 py-1.5 text-right tabular-nums">{new_mults}</TableCell>
                          <TableCell class="hidden px-3 py-1.5 sm:table-cell" aria_hidden=true>
                            <div class="h-2 bg-primary/70" style=format!("width:{pct}%")></div>
                          </TableCell>
                        </TableRow>
                      }
                    }).collect_view()}
                  </TableBody>
                </Table>
              </div>

              // ── 空闲时段 ───────────────────────────────
              <div class="space-y-2">
                <h3 class="text-sm font-semibold">{move || t("contest.analysis-idle-heading")}</h3>
                <p class="text-xs text-muted-foreground">{move || t("contest.analysis-idle-note")}</p>
                <Field label=Signal::derive(move || t("contest.analysis-idle-min")) r#for=field_id class="max-w-xs">
                  <NativeSelect
                    id=select_id
                    value=Signal::derive(move || idle_min.get().to_string())
                    on_change=Callback::new(move |v: String| {
                      if let Ok(m) = v.parse::<i64>() {
                        idle_min.set(m);
                      }
                    })
                    options=options
                  />
                </Field>
                {if a.idle.is_empty() {
                  view! {
                    <p class="text-sm text-muted-foreground">
                      {tf("contest.analysis-idle-none", &[&duration(idle_min.get())])}
                    </p>
                  }
                  .into_any()
                } else {
                  let n = a.idle.len();
                  view! {
                    <p class="text-sm">
                      {tp("contest.analysis-idle-summary", n as u64, &[&n.to_string(), &duration(a.idle_minutes)])}
                    </p>
                    <ul class="max-h-60 divide-y overflow-y-auto border font-mono text-sm">
                      {a.idle.iter().map(|g| view! {
                        <li class="flex flex-wrap justify-between gap-x-3 px-3 py-1.5">
                          <span class="tabular-nums">{format!("{} → {}", clock(g.from), clock(g.to))}</span>
                          <span class="tabular-nums text-muted-foreground">{duration(g.minutes())}</span>
                        </li>
                      }).collect_view()}
                    </ul>
                  }
                  .into_any()
                }}
              </div>

              // ── 乘数缺口 ───────────────────────────────
              <div class="space-y-2">
                <h3 class="text-sm font-semibold">{move || t("contest.analysis-mult-gaps")}</h3>
                {if !a.per_band_mults {
                  view! {
                    <p class="text-sm text-muted-foreground">{move || t("contest.analysis-no-universe")}</p>
                  }
                  .into_any()
                } else {
                  let total = a.cross_band.len();
                  let shown = total.min(MAX_CROSS_ROWS);
                  view! {
                    <p class="text-xs text-muted-foreground">{move || t("contest.analysis-caveat")}</p>
                    {if bands < 2 {
                      view! {
                        <p class="text-sm text-muted-foreground">{move || t("contest.analysis-one-band")}</p>
                      }
                      .into_any()
                    } else if total == 0 {
                      view! {
                        <p class="text-sm text-muted-foreground">{move || t("contest.analysis-cross-complete")}</p>
                      }
                      .into_any()
                    } else {
                      view! {
                        <h4 class="pt-1 text-xs font-medium text-muted-foreground">{move || t("contest.analysis-cross-band")}</h4>
                        <ul class="max-h-72 divide-y overflow-y-auto border text-sm">
                          {a.cross_band.iter().take(MAX_CROSS_ROWS).map(|g| {
                            let label = match g.mult.kind {
                              MultKind::Zone => format!("{} {}", t("contest.cq-zone"), g.mult.value),
                              MultKind::Entity => g
                                .mult
                                .value
                                .parse::<u16>()
                                .ok()
                                .and_then(entity_by_dxcc)
                                .map_or_else(|| g.mult.value.clone(), entity_label),
                              MultKind::Prefix | MultKind::State => g.mult.value.clone(),
                            };
                            let text = tf(
                              "contest.analysis-cross-row",
                              &[&label, &g.worked.join(", "), &g.missing.join(", ")],
                            );
                            view! { <li class="px-3 py-1.5">{text}</li> }
                          }).collect_view()}
                        </ul>
                        {(total > shown).then(|| view! {
                          <p class="text-xs text-muted-foreground">
                            {tf("contest.analysis-cross-more", &[&(total - shown).to_string()])}
                          </p>
                        })}
                      }
                      .into_any()
                    }}
                    {(!a.zones.is_empty()).then(|| view! {
                      <h4 class="pt-2 text-xs font-medium text-muted-foreground">{move || t("contest.analysis-zones")}</h4>
                      <ul class="divide-y border text-sm">
                        {a.zones.iter().map(|z| {
                          let text = if z.missing.is_empty() {
                            tf("contest.analysis-zone-complete", &[&z.band])
                          } else {
                            let list = z.missing.iter().map(u8::to_string).collect::<Vec<_>>().join(", ");
                            tf(
                              "contest.analysis-zone-row",
                              &[&z.band, &z.worked.to_string(), &z.missing.len().to_string(), &list],
                            )
                          };
                          view! { <li class="px-3 py-1.5">{text}</li> }
                        }).collect_view()}
                      </ul>
                    })}
                  }
                  .into_any()
                }}
              </div>
            </div>
          }
          .into_any()
        }
      }}
    </section>
  }
}
