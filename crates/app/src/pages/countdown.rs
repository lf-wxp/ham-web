//! 倒计时与提醒：管理多个目标时间（考试、执照到期等），本地持久化并实时刷新。

use std::cell::RefCell;
use std::collections::HashSet;
use std::time::Duration;

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::{t, tf};
use crate::ui::{Button, DatePicker, Input, Size, TimePicker, Variant};
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
    return t("exam.due");
  }
  let total = ms / 1000;
  let d = total / 86400;
  let h = (total % 86400) / 3600;
  let m = (total % 3600) / 60;
  let s = total % 60;
  if d > 0 {
    tf(
      "common.d",
      &[
        &(d).to_string(),
        &(h).to_string(),
        &(m).to_string(),
        &(s).to_string(),
      ],
    )
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
  set_title("shell.countdown");

  let list = RwSignal::new(load());
  let title = RwSignal::new(String::new());
  let target = RwSignal::new(String::new());
  let now = RwSignal::new(now_ms());

  // `target` 仍是 `datetime-local` 形式（存储 / `Date::parse` 都按它），
  // 界面上拆成日期与时间两个选择器，各自只改自己那一半。
  let target_date = Signal::derive(move || {
    target
      .get()
      .split('T')
      .next()
      .unwrap_or_default()
      .to_owned()
  });
  let target_time = Signal::derive(move || {
    target
      .get()
      .split('T')
      .nth(1)
      .unwrap_or_default()
      .to_owned()
  });

  // 每秒刷新当前时间用于倒计时显示；到期通知由全局 watcher 统一负责（见 start_global_watcher）。
  if let Ok(handle) = set_interval_with_handle(
    move || {
      now.set(now_ms());
    },
    Duration::from_secs(1),
  ) {
    on_cleanup(move || handle.clear());
  }

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
      <PageHeader
        title=move || t("exam.countdowns-and-reminders")
        subtitle=move || t("exam.exam-dates-licence-expiry")
      />

      <PageContainer class="space-y-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("exam.add-countdown")}</h2>
          <div class="grid gap-3 p-4 sm:grid-cols-[1fr_auto_auto_auto]">
            <Input
              value=title
              on_change=Callback::new(move |v: String| title.set(v))
              placeholder=Signal::derive(move || t("exam.title-e-g-class"))
              aria_label=Signal::derive(move || t("exam.countdown-title"))
            />
            <DatePicker
              value=target_date
              on_change=Callback::new(move |d: String| {
                let time = target
                  .get_untracked()
                  .split('T')
                  .nth(1)
                  .filter(|s| !s.is_empty())
                  .unwrap_or("00:00")
                  .to_owned();
                target.set(format!("{d}T{time}"));
              })
              aria_label=Signal::derive(move || t("exam.target-date"))
            />
            <TimePicker
              value=target_time
              on_change=Callback::new(move |v: String| {
                let date = target
                  .get_untracked()
                  .split('T')
                  .next()
                  .filter(|s| !s.is_empty())
                  .map_or_else(crate::util::local_today, str::to_owned);
                target.set(format!("{date}T{v}"));
              })
              aria_label=Signal::derive(move || t("exam.target-time"))
            />
            <Button
              variant=Variant::Default
              size=Size::Default
              on_click=Callback::new(move |_| add())
            >
              {move || t("exam.add")}
            </Button>
          </div>
        </section>

        <section class="space-y-3">
          {move || {
            let mut events = list.get().events.clone();
            events.sort_by_key(|e| e.target_ms);
            if events.is_empty() {
              view! {
                <div class="rounded-xl border bg-card px-4 py-10 text-center text-sm text-muted-foreground">
                  {move || t("exam.no-countdowns-yet-add")}
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
                          {move || t("log.delete")}
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
      </PageContainer>
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

/// 添加一条倒计时（供竞赛日历等页面复用，返回是否新增成功）。
pub(crate) fn add_countdown(title: &str, target_ms: i64) {
  let mut list = load();
  let id = next_id(&list.events);
  list.events.push(CountdownEvent {
    id,
    title: title.to_owned(),
    target_ms,
  });
  save(&list);
}

thread_local! {
  static NOTIFIED: RefCell<HashSet<u64>> = RefCell::new(HashSet::new());
}

/// 全局倒计时到期提醒：在根组件调用一次，应用打开期间任意页面都会触发通知。
pub(crate) fn start_global_watcher() {
  crate::util::request_notify_permission();
  set_interval(
    || {
      let n = now_ms();
      let list = load();
      NOTIFIED.with_borrow_mut(|set| {
        for e in &list.events {
          if e.target_ms <= n && set.insert(e.id) {
            crate::util::notify(&tf("common.countdown-due", &[&(e.title).to_string()]));
          }
        }
      });
    },
    Duration::from_secs(1),
  );
}
