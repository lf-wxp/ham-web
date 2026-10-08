//! 中继台数据库查询：按国家拉取 RepeaterBook 中继台，支持本地按呼号 / 城市筛选。

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::data;
use crate::i18n::{t, tp};
use crate::ui::{Button, Input, Size, Variant};

/// 一条中继台。
#[derive(serde::Deserialize, Clone)]
struct Repeater {
  callsign: String,
  frequency_mhz: String,
  offset_mhz: String,
  pl: String,
  city: String,
  r#use: String,
  status: String,
}

/// `/api/repeaters` 返回。
#[derive(serde::Deserialize, Clone)]
struct RepeaterPayload {
  country: String,
  results: Vec<Repeater>,
}

#[component]
pub fn RepeaterLookup() -> impl IntoView {
  let country = RwSignal::new("China".to_owned());
  let filter = RwSignal::new(String::new());
  let payload = RwSignal::new(None::<RepeaterPayload>);
  let loading = RwSignal::new(false);
  let failed = RwSignal::new(false);

  let run = move || {
    let c = country.get().trim().to_owned();
    if c.is_empty() {
      crate::util::alert("请输入国家 / 地区名称（英文，如 China）");
      return;
    }
    loading.set(true);
    failed.set(false);
    let url = format!(
      "/api/repeaters?country={}",
      js_sys::encode_uri_component(&c)
    );
    spawn_local(async move {
      match data::fetch_external_json::<RepeaterPayload>(&url).await {
        Ok(p) => payload.set(Some(p)),
        Err(_) => failed.set(true),
      }
      loading.set(false);
    });
  };

  // 本地筛选后的结果。
  let filtered = move || {
    let q = filter.get().trim().to_ascii_uppercase();
    payload.get().map(|p| {
      let list: Vec<Repeater> = p
        .results
        .into_iter()
        .filter(|r| {
          q.is_empty()
            || r.callsign.to_ascii_uppercase().contains(&q)
            || r.city.to_ascii_uppercase().contains(&q)
        })
        .collect();
      (p.country, list)
    })
  };

  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.repeater-database-lookup")}</h2>
      <p class="px-4 pt-3 text-xs text-muted-foreground">
        {move || t("radio.fetch-repeaters-listed-in")}
      </p>
      <div class="flex flex-wrap items-center gap-3 p-4">
        <Input
          value=country
          on_change=Callback::new(move |v: String| country.set(v))
          placeholder=Signal::derive(move || t("radio.country-region-e-g"))
          aria_label=Signal::derive(move || t("radio.country"))
          class="max-w-xs"
        />
        <Button
          variant=Variant::Default
          size=Size::Default
          on_click=Callback::new(move |_| run())
        >
          {move || if loading.get() { t("log.looking-up") } else { t("log.look-up") }}
        </Button>
        {move || payload.get().is_some().then(|| view! {
          <Input
            value=filter
            on_change=Callback::new(move |v: String| filter.set(v))
            placeholder=Signal::derive(move || t("radio.filter-callsign-city"))
            aria_label=Signal::derive(move || t("radio.filter"))
            class="max-w-xs"
          />
        })}
      </div>

      {move || {
        if failed.get() {
          return view! {
            <p class="px-4 pb-4 text-sm text-muted-foreground">
              {move || t("radio.lookup-failed-or-no")}
            </p>
          }
          .into_any();
        }
        filtered().map(|(c, list)| view! {
          <div class="border-t">
            <div class="px-4 py-2 text-xs text-muted-foreground">
              {tp(
                "radio.records",
                list.len() as u32,
                &[&(c).to_string(), &(list.len()).to_string()],
              )}
            </div>
            <div class="overflow-x-auto">
              <table class="w-full min-w-[720px] border-collapse text-sm">
                <thead class="bg-muted/60 text-xs">
                  <tr>
                    <th class="border px-3 py-2 text-left">{move || t("log.callsign")}</th>
                    <th class="border px-3 py-2 text-left">{move || t("contest.freq")}</th>
                    <th class="border px-3 py-2 text-left">{move || t("radio.offset")}</th>
                    <th class="border px-3 py-2 text-left">{move || t("radio.ctcss")}</th>
                    <th class="border px-3 py-2 text-left">{move || t("radio.city")}</th>
                    <th class="border px-3 py-2 text-left">{move || t("radio.use")}</th>
                    <th class="border px-3 py-2 text-left">{move || t("radio.status-2")}</th>
                  </tr>
                </thead>
                <tbody>
                  {list.into_iter().map(|r| view! {
                    <tr class="border-t transition-colors hover:bg-muted/40">
                      <td class="border px-3 py-1.5 font-mono font-semibold">{r.callsign}</td>
                      <td class="border px-3 py-1.5 tabular-nums text-muted-foreground">{r.frequency_mhz}</td>
                      <td class="border px-3 py-1.5 tabular-nums text-muted-foreground">{r.offset_mhz}</td>
                      <td class="border px-3 py-1.5 tabular-nums text-muted-foreground">{if r.pl.is_empty() { "—".to_owned() } else { r.pl }}</td>
                      <td class="border px-3 py-1.5 text-muted-foreground">{r.city}</td>
                      <td class="border px-3 py-1.5 text-muted-foreground">{r.r#use}</td>
                      <td class="border px-3 py-1.5 text-muted-foreground">{r.status}</td>
                    </tr>
                  }).collect_view()}
                </tbody>
              </table>
            </div>
          </div>
        })
        .into_any()
      }}

      <p class="px-4 pb-4 text-xs text-muted-foreground">
        {move || t("radio.data-from-the-public")}
      </p>
    </section>
  }
}
