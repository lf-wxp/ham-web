use ham_web_core::contest::{CONTESTS, CabrilloHeader, cabrillo, contest};
use leptos::prelude::*;

use crate::i18n::{t, tp};
use crate::pages::log::{LogEntry, use_log_store};
use crate::ui::{Button, Input, NativeSelect, SelectOption, Size, Variant};
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

  let contest_options: Vec<SelectOption> = CONTESTS
    .iter()
    .map(|c| SelectOption::new(c.id, Signal::derive(move || t(c.name))))
    .collect();

  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("log.contest-log-generator")}</h2>
      <div class="space-y-3 p-4">
        <div class="grid gap-3 sm:grid-cols-2">
          <label class="flex flex-col gap-1.5 text-sm">
            <span class="text-xs text-muted-foreground">{move || t("log.station-callsign")}</span>
            <Input
              value=callsign
              on_change=Callback::new(move |v: String| callsign.set(v.to_uppercase()))
              class="uppercase"
            />
          </label>
          <label class="flex flex-col gap-1.5 text-sm">
            <span class="text-xs text-muted-foreground">{move || t("contest.contest")}</span>
            <NativeSelect
              value=contest_id
              on_change=Callback::new(move |v: String| {
                contest_id.set(v);
                generated.set(String::new());
              })
              options=contest_options
              aria_label=Signal::derive(move || t("contest.contest"))
            />
          </label>
          <label class="flex flex-col gap-1.5 text-sm">
            <span class="text-xs text-muted-foreground">{move || t("log.operator")}</span>
            <Input value=operator on_change=Callback::new(move |v: String| operator.set(v)) />
          </label>
          <label class="flex flex-col gap-1.5 text-sm">
            <span class="text-xs text-muted-foreground">{move || t("log.station-grid")}</span>
            <Input
              value=gridsquare
              on_change=Callback::new(move |v: String| gridsquare.set(v.to_uppercase()))
              class="uppercase"
            />
          </label>
        </div>

        <div class="flex flex-wrap items-center gap-2">
          <Button
            variant=Variant::Default
            size=Size::Default
            on_click=Callback::new(move |_| generate())
          >
            {move || t("log.generate-cabrillo")}
          </Button>
          <Button
            variant=Variant::Outline
            size=Size::Default
            disabled=Signal::derive(move || generated.with(String::is_empty))
            on_click=Callback::new(move |_| download())
          >
            {move || t("log.download-cbr")}
          </Button>
          <span class="text-xs text-muted-foreground">
            { move || { let n = tagged.with(Vec::len); tp("log.qsos-tagged-with-this", n, &[&n.to_string()]) } }
          </span>
        </div>

        {move || {
          let text = generated.get();
          if text.is_empty() {
            view! {
              <p class="text-xs text-muted-foreground">
                {move || t("log.reads-records-in-the")}
                " "
                <a href="/contest-log" class="font-medium text-foreground underline underline-offset-4">{move || t("shell.contest-log")}</a>
                {move || t("log.automatic-serials-live-dupe")}
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
