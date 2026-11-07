//! 待回卡清单（日志统计明细里的一节）。

use leptos::prelude::*;

use ham_web_core::logbook::pending_qsl;

use super::LogEntry;
use crate::i18n::{t, tf, tp};
use crate::ui::{Button, Size, Variant};

/// 先列这么多条，其余收在「显示全部」后面：几千条日志里待回卡可能上千条，
/// 一次全部铺开既难读、也是日志页最重的一块 DOM。
const PREVIEW: usize = 20;

/// 一行待回卡（只取渲染要用的几个字段，不克隆整条记录）。
struct Row {
  callsign: String,
  date: String,
  band: String,
  mode: String,
  via: String,
}

/// 待回卡清单。自成一个响应式块：点「显示全部」只重建这一节，不连带重建奖项与网格地图。
#[component]
pub(super) fn PendingQslList(
  scoped: Memo<Vec<LogEntry>>,
  show_all: RwSignal<bool>,
) -> impl IntoView {
  move || {
    let all_shown = show_all.get();
    let (rows, total) = scoped.with(|entries| {
      let all = pending_qsl(entries);
      let total = all.len();
      let rows: Vec<Row> = all
        .into_iter()
        .take(if all_shown { usize::MAX } else { PREVIEW })
        .map(|e| Row {
          callsign: e.callsign.clone(),
          date: e.date.clone(),
          band: e.band_label(),
          mode: e.mode.clone(),
          via: if e.qsl_sent {
            t("knowledge.paper")
          } else if e.lotw_sent {
            "LoTW".to_owned()
          } else {
            "eQSL".to_owned()
          },
        })
        .collect();
      (rows, total)
    });
    let hidden = total - rows.len();
    view! {
      <section class=crate::kit::card_pad_class()>
        <h3 class="mb-2 text-sm">
          {move || t("log.qsls-pending")}
          <span class="ml-2 text-xs font-normal text-muted-foreground">
            {tp("common.cards-to-chase", total, &[&total.to_string()])}
          </span>
        </h3>
        {if rows.is_empty() {
          view! { <div class="text-xs text-muted-foreground">{move || t("log.all-sent-qsls-are")}</div> }.into_any()
        } else {
          view! {
            <div class="divide-y">
              {rows
                .into_iter()
                .map(|row| {
                  view! {
                    <div class="flex items-center gap-2 py-1.5 text-sm">
                      <span class="font-mono">{row.callsign}</span>
                      <span class="text-xs text-muted-foreground">
                        {row.date} " · " {row.band} " · " {row.mode}
                      </span>
                      <span class="ml-auto border px-2 py-0.5 text-xs text-muted-foreground">{row.via}</span>
                    </div>
                  }
                })
                .collect_view()}
            </div>
            {(hidden > 0).then(|| {
              view! {
                // 文字色与下划线显式给：`Variant::Text` 默认是弱化的 `muted-foreground`，
                // 而这是这一节唯一的动作，要看得出能点。
                <Button
                  variant=Variant::Text
                  size=Size::Xs
                  class="mt-2 text-primary underline underline-offset-4"
                  on_click=Callback::new(move |_| show_all.set(true))
                >
                  {move || tf("log.show-all-pending", &[&total.to_string()])}
                </Button>
              }
            })}
          }
          .into_any()
        }}
      </section>
    }
  }
}
