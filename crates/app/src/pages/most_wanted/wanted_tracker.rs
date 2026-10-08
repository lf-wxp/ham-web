use std::collections::HashSet;

use ham_web_core::most_wanted::{WANTED_ENTITIES, wanted_prefix};
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::pages::log::use_log_store;
use crate::ui::Input;
use crate::util::storage;

const STORAGE_KEY: &str = "dxcc_wanted_done";

/// 本地通联进度追踪：勾选已通联实体，进度存 localStorage。
#[component]
pub(super) fn WantedTracker() -> impl IntoView {
  let done: Vec<String> = storage::get_json(STORAGE_KEY).unwrap_or_default();
  let done = RwSignal::new(done);
  let query = RwSignal::new(String::new());

  // 从通联日志自动识别已通联的稀有实体。
  let logged = RwSignal::new(
    use_log_store()
      .logbook
      .get_untracked()
      .entries
      .iter()
      .filter_map(|e| wanted_prefix(&e.callsign).map(str::to_owned))
      .collect::<HashSet<String>>(),
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
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("log.contact-progress-tracking")}</h2>
      <div class="space-y-3 p-4">
        <Input
          value=query
          on_change=Callback::new(move |v: String| query.set(v))
          placeholder=Signal::derive(move || t("log.search-prefix-or-entity"))
          clearable=true
        />
        <div>
          <div class="mb-1 flex items-center justify-between text-xs text-muted-foreground">
            <span>{move || t("log.worked")}</span>
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
                        aria-label=tf("common.marker", &[(name)])
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
                          <span class="shrink-0 rounded bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground">{move || t("radio.log")}</span>
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
