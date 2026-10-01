//! 倒计时与提醒：管理多个目标时间（考试、执照到期等），本地持久化并实时刷新。

use std::collections::HashSet;
use std::time::Duration;

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;

use crate::ui::{Size, Variant, button_class, input_class};
use crate::util::now_ms;
use crate::util::set_title;

/// 一个倒计时事件。
#[derive(Serialize, Deserialize, Clone)]
struct CountdownEvent {
  id: u64,
  title: String,
  /// 目标时间（毫秒时间戳）。
  target_ms: i64,
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct CountdownList {
  events: Vec<CountdownEvent>,
}

const KEY: &str = "countdowns";

fn load() -> CountdownList {
  crate::util::storage::get_json(KEY).unwrap_or_default()
}

fn save(list: &CountdownList) {
  crate::util::storage::set_json(KEY, list);
}

/// 基于已有事件计算下一个不冲突的 ID（刷新页面后仍能避免重复）。
fn next_id(events: &[CountdownEvent]) -> u64 {
  events.iter().map(|e| e.id).max().unwrap_or(0) + 1
}

/// 剩余时间文本。
fn format_remaining(ms: i64) -> String {
  if ms <= 0 {
    return "已到期".to_owned();
  }
  let total = ms / 1000;
  let d = total / 86400;
  let h = (total % 86400) / 3600;
  let m = (total % 3600) / 60;
  let s = total % 60;
  if d > 0 {
    format!("{d} 天 {h:02}:{m:02}:{s:02}")
  } else {
    format!("{h:02}:{m:02}:{s:02}")
  }
}

/// 解析 `datetime-local` 值（YYYY-MM-DDTHH:MM）为时间戳。
fn parse_local(s: &str) -> Option<i64> {
  if s.is_empty() {
    return None;
  }
  let ms = js_sys::Date::parse(s);
  if ms.is_nan() { None } else { Some(ms as i64) }
}

#[component]
pub fn CountdownPage() -> impl IntoView {
  set_title("倒计时");

  let list = RwSignal::new(load());
  let title = RwSignal::new(String::new());
  let target = RwSignal::new(String::new());
  let now = RwSignal::new(now_ms());

  let notified = StoredValue::new(HashSet::<u64>::new());
  set_interval(
    move || {
      let n = now_ms();
      now.set(n);
      // 事件到期时发送浏览器通知（去重）
      let mut set = notified.get_value();
      for e in &list.get_untracked().events {
        if e.target_ms <= n && set.insert(e.id) {
          crate::util::notify(&format!("倒计时到期：{}", e.title));
        }
      }
      notified.set_value(set);
    },
    Duration::from_secs(1),
  );
  // 页面加载时请求通知权限（用户可拒绝，不影响其他功能）
  crate::util::request_notify_permission();

  let add = move || {
    let t = title.get().trim().to_owned();
    let Some(ms) = parse_local(&target.get()) else {
      return;
    };
    if t.is_empty() {
      return;
    }
    list.update(|l| {
      let id = next_id(&l.events);
      l.events.push(CountdownEvent {
        id,
        title: t,
        target_ms: ms,
      })
    });
    save(&list.get_untracked());
    title.set(String::new());
    target.set(String::new());
  };

  let remove = move |id: u64| {
    list.update(|l| l.events.retain(|e| e.id != id));
    save(&list.get_untracked());
  };

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-3xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">"倒计时与提醒"</h1>
            <div class="text-xs text-muted-foreground">"考试日期 · 执照到期 · 活动提醒"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-3xl space-y-5 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"添加倒计时"</h2>
          <div class="grid gap-3 p-4 sm:grid-cols-[1fr_auto_auto]">
            <input
              type="text"
              placeholder="标题，如：A 类操作证考试"
              aria-label="倒计时标题"
              prop:value=move || title.get()
              on:input=move |e| title.set(event_target_value(&e))
              class=input_class("")
            />
            <input
              type="datetime-local"
              aria-label="目标时间"
              prop:value=move || target.get()
              on:input=move |e| target.set(event_target_value(&e))
              class=input_class("")
            />
            <button type="button" class=button_class(Variant::Default, Size::Default, "") on:click=move |_| add()>
              "添加"
            </button>
          </div>
        </section>

        <section class="space-y-3">
          {move || {
            let mut events = list.get().events.clone();
            events.sort_by_key(|e| e.target_ms);
            if events.is_empty() {
              view! {
                <div class="rounded-xl border bg-card px-4 py-10 text-center text-sm text-muted-foreground">
                  "暂无倒计时，添加一个目标时间吧。"
                </div>
              }
              .into_any()
            } else {
              view! {
                {events
                  .into_iter()
                  .map(|e| {
                    let id = e.id;
                    let remain = e.target_ms - now.get();
                    let expired = remain <= 0;
                    view! {
                      <div class="flex items-center gap-4 rounded-xl border bg-card px-4 py-4">
                        <div class="min-w-0 flex-1">
                          <div class="truncate font-medium">{e.title.clone()}</div>
                          <div class="mt-0.5 text-xs text-muted-foreground tabular-nums">
                            {format_local(e.target_ms)}
                          </div>
                        </div>
                        <div class=format!(
                          "shrink-0 text-right font-mono text-lg font-semibold tabular-nums {}",
                          if expired { "text-muted-foreground" } else { "text-primary" },
                        )>
                          {format_remaining(remain)}
                        </div>
                        <button
                          type="button"
                          class="shrink-0 text-xs text-muted-foreground transition-colors hover:text-destructive"
                          on:click=move |_| remove(id)
                        >
                          "删除"
                        </button>
                      </div>
                    }
                  })
                  .collect_view()}
              }
              .into_any()
            }
          }}
        </section>
      </div>
    </div>
  }
}

/// 本地时间戳的可读显示。
fn format_local(ms: i64) -> String {
  let d = js_sys::Date::new(&JsValue::from_f64(ms as f64));
  let y = d.get_full_year() as i32;
  let mo = d.get_month() as i32 + 1;
  let day = d.get_date() as i32;
  let h = d.get_hours() as i32;
  let m = d.get_minutes() as i32;
  format!("{y:04}-{mo:02}-{day:02} {h:02}:{m:02}")
}
