use std::collections::HashSet;

use ham_web_core::most_wanted::{WANTED_ENTITIES, wanted_prefix};
use leptos::prelude::*;
use serde::Deserialize;

use crate::util::storage;

const STORAGE_KEY: &str = "dxcc_wanted_done";

/// 日志精简结构（仅读取呼号，用于自动识别已通联稀有实体）。
#[derive(Deserialize)]
struct LogbookLite {
  entries: Vec<LogEntryLite>,
}

#[derive(Deserialize)]
struct LogEntryLite {
  #[serde(default)]
  callsign: String,
}

/// 本地通联进度追踪：勾选已通联实体，进度存 localStorage。
#[component]
pub(super) fn WantedTracker() -> impl IntoView {
  let done: Vec<String> = storage::get_json(STORAGE_KEY).unwrap_or_default();
  let done = RwSignal::new(done);
  let query = RwSignal::new(String::new());

  // 从通联日志自动识别已通联的稀有实体。
  let logged = RwSignal::new(
    storage::get_json::<LogbookLite>("logbook")
      .map(|lb| {
        lb.entries
          .iter()
          .filter_map(|e| wanted_prefix(&e.callsign).map(str::to_owned))
          .collect::<HashSet<String>>()
      })
      .unwrap_or_default(),
  );

  let toggle = move |prefix: &'static str| {
    let mut v = done.get();
    if let Some(pos) = v.iter().position(|p| p == prefix) {
      v.remove(pos);
    } else {
      v.push(prefix.to_owned());
    }
    storage::set_json(STORAGE_KEY, &v);
    done.set(v);
  };

  let total = WANTED_ENTITIES.len();

  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">"通联进度追踪"</h2>
      <div class="space-y-3 p-4">
        <input
          type="text"
          placeholder="搜索前缀或实体名…"
          prop:value=move || query.get()
          on:input=move |e| query.set(event_target_value(&e))
          class="h-10 w-full rounded-lg border bg-background px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
        />
        <div>
          <div class="mb-1 flex items-center justify-between text-xs text-muted-foreground">
            <span>"已通联"</span>
            <span class="tabular-nums">
              {move || {
                let d = done.get();
                let n = WANTED_ENTITIES
                  .iter()
                  .filter(|(p, _, _)| d.iter().any(|x| x == p) || logged.get().contains(*p))
                  .count();
                format!("{n} / {total}")
              }}
            </span>
          </div>
          <div class="h-2 w-full overflow-hidden rounded-full bg-muted">
            <div
              class="h-full rounded-full bg-primary transition-all"
              style=move || {
                let d = done.get();
                let n = WANTED_ENTITIES
                  .iter()
                  .filter(|(p, _, _)| d.iter().any(|x| x == p) || logged.get().contains(*p))
                  .count();
                format!("width: {:.1}%", n as f64 / total as f64 * 100.0)
              }
            ></div>
          </div>
        </div>
        <div class="divide-y">
          {move || {
            let q = query.get().trim().to_lowercase();
            let items = WANTED_ENTITIES.iter().filter(|(p, n, _)| {
              q.is_empty() || p.to_lowercase().contains(&q) || n.to_lowercase().contains(&q)
            });
            view! {
              {items
                .map(|&(prefix, name, level)| {
                  let from_log = logged.get().contains(prefix);
                  let checked =
                    move || done.get().iter().any(|p| p == prefix) || logged.get().contains(prefix);
                  view! {
                    <div class="flex items-center gap-3 py-2">
                      <button
                        type="button"
                        aria-label=format!("标记 {name}")
                        on:click=move |_| toggle(prefix)
                        class=move || {
                          if checked() {
                            "flex h-5 w-5 shrink-0 items-center justify-center rounded border bg-primary text-xs text-primary-foreground"
                          } else {
                            "flex h-5 w-5 shrink-0 items-center justify-center rounded border hover:bg-accent"
                          }
                        }
                      >
                        {move || if checked() { "✓" } else { "" }}
                      </button>
                      <span class="w-16 shrink-0 font-mono text-sm font-semibold">{prefix}</span>
                      <span class="flex-1 text-sm">{name}</span>
                      {if from_log {
                        view! {
                          <span class="shrink-0 rounded bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground">"日志"</span>
                        }
                        .into_any()
                      } else {
                        view! { <span></span> }.into_any()
                      }}
                      <span class="shrink-0 text-xs text-muted-foreground">{level}</span>
                    </div>
                  }
                })
                .collect_view()}
            }
          }}
        </div>
      </div>
    </section>
  }
}
