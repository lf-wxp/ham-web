use ham_web_core::contest::{CONTESTS, CabrilloHeader, cabrillo, contest};
use leptos::prelude::*;

use crate::pages::log::{LogEntry, use_log_store};
use crate::ui::{Size, Variant, button_class, input_class};
use crate::util::download_text;

/// 从通联日志按竞赛规则生成可提交的 Cabrillo 文件。
#[component]
pub(super) fn CabrilloGenerator() -> impl IntoView {
  let store = use_log_store();
  let station = store.station.get_untracked();
  let callsign = RwSignal::new(station.callsign.clone());
  let operator = RwSignal::new(station.operator.clone());
  let gridsquare = RwSignal::new(station.gridsquare.clone());
  let contest_id = RwSignal::new(CONTESTS[0].id.to_owned());
  let generated = RwSignal::new(String::new());

  let tagged = Memo::new(move |_| {
    let id = contest_id.get();
    store.logbook.with(|lb| {
      lb.entries
        .iter()
        .filter(|e| e.contest_id == id)
        .cloned()
        .collect::<Vec<LogEntry>>()
    })
  });

  let generate = move || {
    let Some(def) = contest(&contest_id.get_untracked()) else {
      return;
    };
    let header = CabrilloHeader {
      callsign: callsign.get_untracked().trim().to_owned(),
      operators: operator.get_untracked().trim().to_owned(),
      grid: gridsquare.get_untracked().trim().to_owned(),
      ..Default::default()
    };
    generated.set(tagged.with_untracked(|list| cabrillo(def, &header, list)));
  };

  let download = move || {
    let text = generated.get();
    if !text.is_empty() {
      download_text(
        &format!("{}.cbr", contest_id.get_untracked()),
        &text,
        "text/plain",
      );
    }
  };

  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">"竞赛日志生成器"</h2>
      <div class="space-y-3 p-4">
        <div class="grid gap-3 sm:grid-cols-2">
          <label class="flex flex-col gap-1.5 text-sm">
            <span class="text-xs text-muted-foreground">"本台呼号"</span>
            <input
              type="text"
              prop:value=move || callsign.get()
              on:input=move |e| callsign.set(event_target_value(&e).to_uppercase())
              class=input_class("uppercase")
            />
          </label>
          <label class="flex flex-col gap-1.5 text-sm">
            <span class="text-xs text-muted-foreground">"竞赛"</span>
            <select
              prop:value=move || contest_id.get()
              on:change=move |e| {
                contest_id.set(event_target_value(&e));
                generated.set(String::new());
              }
              class=input_class("")
            >
              {CONTESTS
                .iter()
                .map(|c| view! { <option value=c.id>{c.name}</option> })
                .collect_view()}
            </select>
          </label>
          <label class="flex flex-col gap-1.5 text-sm">
            <span class="text-xs text-muted-foreground">"操作员"</span>
            <input
              type="text"
              prop:value=move || operator.get()
              on:input=move |e| operator.set(event_target_value(&e))
              class=input_class("")
            />
          </label>
          <label class="flex flex-col gap-1.5 text-sm">
            <span class="text-xs text-muted-foreground">"本台网格"</span>
            <input
              type="text"
              prop:value=move || gridsquare.get()
              on:input=move |e| gridsquare.set(event_target_value(&e).to_uppercase())
              class=input_class("uppercase")
            />
          </label>
        </div>

        <div class="flex flex-wrap items-center gap-2">
          <button
            type="button"
            class=button_class(Variant::Default, Size::Default, "")
            on:click=move |_| generate()
          >
            "生成 Cabrillo"
          </button>
          <button
            type="button"
            class=button_class(Variant::Outline, Size::Default, "")
            prop:disabled=move || generated.with(String::is_empty)
            on:click=move |_| download()
          >
            "下载 .cbr"
          </button>
          <span class="text-xs text-muted-foreground">
            {move || format!("日志中标记为该竞赛的通联：{} 条", tagged.with(Vec::len))}
          </span>
        </div>

        {move || {
          let text = generated.get();
          if text.is_empty() {
            view! {
              <p class="text-xs text-muted-foreground">
                "读取通联日志中带对应 CONTEST_ID 的记录（含交换信息与自报分数）。比赛时推荐直接用 "
                <a href="/contest-log" class="font-medium text-foreground underline underline-offset-4">"竞赛录入"</a>
                "：自动序号、实时查重，结束后一键导出。"
              </p>
            }
            .into_any()
          } else {
            view! {
              <pre class="max-h-64 overflow-auto rounded-lg border bg-muted/40 p-3 font-mono text-xs leading-5">
                {text}
              </pre>
            }
            .into_any()
          }
        }}
      </div>
    </section>
  }
}
