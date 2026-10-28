use ham_web_core::kp_forecast::{
  KpForecast, KpKind, KpLevel, Outlook, SLOT_LABELS, SLOTS_PER_DAY, Slot, level,
};
use leptos::prelude::*;

use crate::data::kt_tracked;
use crate::i18n::{t, tf};
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};

/// 磁暴级别的显示名词条（G1–G5）。
const fn g_name_key(g: u8) -> &'static str {
  match g {
    1 => "radio.kp-g1",
    2 => "radio.kp-g2",
    3 => "radio.kp-g3",
    4 => "radio.kp-g4",
    _ => "radio.kp-g5",
  }
}

/// `2026-10-16` → `10-16`。
fn short_date(date: &str) -> &str {
  date.get(5..).unwrap_or(date)
}

/// 一格的底色。颜色只是辅助：数值本身写在格子里，磁暴格另带 `G*` 字样。
const fn level_class(l: KpLevel) -> &'static str {
  match l {
    KpLevel::Quiet => "bg-inset",
    KpLevel::Active => "bg-gold/25",
    KpLevel::Storm(1 | 2) => "bg-destructive/30",
    KpLevel::Storm(_) => "bg-destructive/50",
  }
}

/// 实测 / 估计 / 预报在样式上要看得出区别（边框与斜体，不靠颜色）。
const fn kind_class(k: KpKind) -> &'static str {
  match k {
    KpKind::Observed => "border-solid",
    KpKind::Estimated => "border-solid italic",
    KpKind::Predicted => "border-dashed",
  }
}

const fn kind_key(k: KpKind) -> &'static str {
  match k {
    KpKind::Observed => "radio.kp-kind-observed",
    KpKind::Estimated => "radio.kp-kind-estimated",
    KpKind::Predicted => "radio.kp-kind-predicted",
  }
}

/// 有数据时的内容：摘要、表格、图例与说明。
#[component]
pub(super) fn KpOutlook(forecast: KpForecast, outlook: Outlook) -> impl IntoView {
  // 组件体不是响应式上下文：t()/tf() 直接调用会在挂载时冻结，切语言后停在旧语言。
  // 这里把文案包成派生信号，与页面其余 `move || t(...)` 的更新节奏一致。
  let summary = {
    let peak = outlook.peak.clone();
    Signal::derive(move || {
      peak.as_ref().map(|p| {
        let slot = SLOT_LABELS[p.slot];
        let tail = match p.g {
          Some(g) => tf(
            "radio.kp-forecast-storm",
            &[
              &g.to_string(),
              &crate::data::core_text_tracked(g_name_key(g)),
            ],
          ),
          None => t("radio.kp-forecast-below-scale"),
        };
        tf(
          "radio.kp-forecast-peak",
          &[&format!("{:.2}", p.kp), short_date(&p.date), slot, &tail],
        )
      })
    })
  };
  let updated = {
    let updated = forecast.updated.clone();
    Signal::derive(move || {
      updated.as_deref().map(|u| {
        let u = u.get(..16).unwrap_or(u).replace('T', " ");
        tf("radio.kp-updated-utc", &[&u])
      })
    })
  };
  // 组件的子节点需要 `'static`：表头文字先拷成拥有所有权的字符串。
  let dates: Vec<String> = outlook
    .days
    .iter()
    .map(|d| short_date(&d.date).to_owned())
    .collect();

  view! {
    {move || match summary.get() {
      Some(s) => view! { <p>{move || kt_tracked(&s)}</p> }.into_any(),
      None => view! { <p class="text-muted-foreground">{move || t("radio.kp-forecast-no-forecast")}</p> }.into_any(),
    }}
    <Table aria_label=Signal::derive(move || t("radio.kp-forecast-table")) class="text-center font-mono text-xs">
      <TableHead class="text-muted-foreground">
        <TableRow>
          <TableHeader scope="col" class="px-2 py-1.5 text-left font-normal">
            {move || t("radio.kp-forecast-slot")}
          </TableHeader>
          {dates
            .iter()
            .cloned()
            .map(|d| view! {
              <TableHeader scope="col" class="px-2 py-1.5 text-center font-normal">{move || kt_tracked(&d)}</TableHeader>
            })
            .collect_view()}
        </TableRow>
      </TableHead>
      <TableBody>
        {(0..SLOTS_PER_DAY)
          .map(|i| {
            let cells: Vec<Option<Slot>> = outlook.days.iter().map(|d| d.slots[i]).collect();
            view! {
              <TableRow>
                <TableHeader scope="row" class="whitespace-nowrap px-2 py-1 text-left font-normal text-muted-foreground">
                  {SLOT_LABELS[i]}
                </TableHeader>
                {cells
                  .into_iter()
                  .map(|c| match c {
                    None => view! { <TableCell class="px-1 py-1 text-muted-foreground">"—"</TableCell> }.into_any(),
                    Some(s) => {
                      let l = level(s.kp);
                      let class = format!("border px-1 py-1 tabular-nums {} {}", level_class(l), kind_class(s.kind));
                      let g = match l {
                        KpLevel::Storm(g) => Some(format!(" G{g}")),
                        _ => None,
                      };
                      view! {
                        <TableCell class=class>
                          {format!("{:.2}", s.kp)}
                          {g.map(|g| view! { <span>{move || kt_tracked(&g)}</span> })}
                          <span class="sr-only">{move || format!(" {}", crate::data::core_text_tracked(kind_key(s.kind)))}</span>
                        </TableCell>
                      }
                      .into_any()
                    }
                  })
                  .collect_view()}
              </TableRow>
            }
          })
          .collect_view()}
      </TableBody>
    </Table>
    <p class="text-xs text-muted-foreground">{move || t("radio.kp-forecast-legend")}</p>
    {move || updated.get().map(|u| view! { <p class="text-xs text-muted-foreground">{move || kt_tracked(&u)}</p> }.into_any())}
    <p class="text-xs text-muted-foreground">{move || t("radio.kp-forecast-note")}</p>
  }
}
