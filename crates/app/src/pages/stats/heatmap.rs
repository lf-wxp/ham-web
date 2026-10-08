use crate::i18n::tp;
use leptos::prelude::*;

/// 根据计数值与最大值返回色阶类名（完整字面量，供 Tailwind 扫描）。
fn cell_class(count: u32, max: u32) -> &'static str {
  if count == 0 {
    return "bg-muted/50";
  }
  let ratio = count as f64 / max as f64;
  if ratio >= 0.8 {
    "bg-primary"
  } else if ratio >= 0.6 {
    "bg-primary/70"
  } else if ratio >= 0.4 {
    "bg-primary/40"
  } else if ratio >= 0.2 {
    "bg-primary/20"
  } else {
    "bg-primary/10"
  }
}

/// UTC 时段 × 波段的通联密度热力图。
#[component]
pub(super) fn Heatmap(rows: Vec<(String, [u32; 24])>) -> impl IntoView {
  let max = rows
    .iter()
    .flat_map(|(_, h)| h.iter().copied())
    .max()
    .unwrap_or(0)
    .max(1);

  view! {
    <div class="space-y-1.5">
      <div class="flex items-center gap-1">
        <span class="w-12 shrink-0 text-right text-[10px] text-muted-foreground">"UTC"</span>
        <div class="flex flex-1 gap-px">
          {(0..24)
            .map(|h| {
              view! {
                <div class="flex-1 text-center text-[9px] leading-none text-muted-foreground">
                  {if h % 3 == 0 { h.to_string() } else { String::new() }}
                </div>
              }
            })
            .collect_view()}
        </div>
      </div>
      {rows
        .into_iter()
        .map(|(band, hours)| {
          let total: u32 = hours.iter().sum();
          view! {
            <div class="flex items-center gap-1">
              <span class="w-12 shrink-0 text-right text-xs text-muted-foreground">{band.clone()}</span>
              <div class="flex flex-1 gap-px">
                {hours
                  .iter()
                  .enumerate()
                  .map(|(h, &c)| {
                    view! {
                      <div
                        class=format!("h-5 flex-1 rounded-sm {}", cell_class(c, max))
                        title=tp("common.00-utc-records", c, &[&(band).to_string(), &(h).to_string(), &(c).to_string()])
                      ></div>
                    }
                  })
                  .collect_view()}
              </div>
              <span class="w-8 shrink-0 text-right text-[10px] tabular-nums text-muted-foreground">
                {total}
              </span>
            </div>
          }
        })
        .collect_view()}
    </div>
  }
}
