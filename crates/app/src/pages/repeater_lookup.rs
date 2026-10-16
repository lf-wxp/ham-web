//! 中继台数据库查询：按国家拉取 RepeaterBook 中继台，支持本地按呼号 / 城市筛选，
//! 并把当前筛选结果导出成 CHIRP 通用 CSV 用来给手台写频。

use ham_web_core::chirp_csv::{self, ExportOptions, RepeaterRow, SkipCounts};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::common::EmptyState;
use crate::data;
use crate::i18n::{t, tp};
use crate::ui::{
  Button, Checkbox, Input, Label, Size, Table, TableBody, TableCell, TableHead, TableHeader,
  TableRow, Variant,
};
use crate::util::unique_id;

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

/// 一次导出的结果摘要：导出几条、各原因跳过几条。
#[derive(Clone, Copy)]
struct ExportSummary {
  exported: usize,
  skipped: SkipCounts,
}

/// 文件名里只留 ASCII 字母数字，其余折成 `-`。
fn file_slug(country: &str) -> String {
  let slug: String = country
    .trim()
    .chars()
    .map(|c| {
      if c.is_ascii_alphanumeric() {
        c.to_ascii_lowercase()
      } else {
        '-'
      }
    })
    .collect();
  let slug = slug.trim_matches('-');
  if slug.is_empty() {
    "repeaters".to_owned()
  } else {
    slug.to_owned()
  }
}

#[component]
pub fn RepeaterLookup() -> impl IntoView {
  let country = RwSignal::new("China".to_owned());
  let filter = RwSignal::new(String::new());
  let payload = RwSignal::new(None::<RepeaterPayload>);
  let loading = RwSignal::new(false);
  let failed = RwSignal::new(false);
  let only_bands = RwSignal::new(true);
  let skip_off_air = RwSignal::new(true);
  let summary = RwSignal::new(None::<ExportSummary>);

  let bands_id = unique_id("chirp-only-bands");
  let off_air_id = unique_id("chirp-skip-off-air");

  let run = move || {
    let c = country.get().trim().to_owned();
    if c.is_empty() {
      crate::ui::push_error(t("common.enter-a-country-region"));
      return;
    }
    loading.set(true);
    failed.set(false);
    summary.set(None);
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

  // 导出当前筛选出的中继台；一条都导不出来时不下载空文件，只给出原因。
  let export = move || {
    let Some((c, list)) = filtered() else { return };
    let rows: Vec<RepeaterRow<'_>> = list
      .iter()
      .map(|r| RepeaterRow {
        callsign: &r.callsign,
        frequency_mhz: &r.frequency_mhz,
        offset_mhz: &r.offset_mhz,
        pl: &r.pl,
        city: &r.city,
        status: &r.status,
      })
      .collect();
    let result = chirp_csv::export(
      &rows,
      ExportOptions {
        only_amateur_bands: only_bands.get(),
        skip_off_air: skip_off_air.get(),
      },
    );
    if result.exported > 0 {
      crate::util::download_text(
        &format!("chirp-{}.csv", file_slug(&c)),
        &result.csv,
        "text/csv;charset=utf-8",
      );
    }
    summary.set(Some(ExportSummary {
      exported: result.exported,
      skipped: result.skipped,
    }));
  };

  view! {
    <section class="pxl-card border bg-card">
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

      {move || payload.get().is_some().then(|| view! {
        <div class="space-y-2 border-t px-4 py-3">
          <p class="text-xs text-muted-foreground">{move || t("radio.chirp-hint")}</p>
          <div class="flex flex-wrap items-center gap-x-6 gap-y-2">
            <div class="flex items-center gap-2">
              <Checkbox
                id=bands_id.clone()
                checked=only_bands
                on_change=Callback::new(move |v: bool| only_bands.set(v))
              />
              <Label r#for=bands_id.clone()>{move || t("radio.chirp-only-amateur-bands")}</Label>
            </div>
            <div class="flex items-center gap-2">
              <Checkbox
                id=off_air_id.clone()
                checked=skip_off_air
                on_change=Callback::new(move |v: bool| skip_off_air.set(v))
              />
              <Label r#for=off_air_id.clone()>{move || t("radio.chirp-skip-off-air")}</Label>
            </div>
            <Button
              variant=Variant::Outline
              size=Size::Sm
              on_click=Callback::new(move |_| export())
            >
              {move || t("radio.chirp-export")}
            </Button>
          </div>
          {move || summary.get().map(|s| view! {
            <ul class="space-y-0.5 text-xs text-muted-foreground" role="status">
              <li>{tp("radio.chirp-exported", s.exported as u32, &[&s.exported.to_string()])}</li>
              {(s.skipped.bad_frequency > 0).then(|| view! {
                <li>{tp(
                  "radio.chirp-skipped-bad-frequency",
                  s.skipped.bad_frequency as u32,
                  &[&s.skipped.bad_frequency.to_string()],
                )}</li>
              })}
              {(s.skipped.off_air > 0).then(|| view! {
                <li>{tp(
                  "radio.chirp-skipped-off-air",
                  s.skipped.off_air as u32,
                  &[&s.skipped.off_air.to_string()],
                )}</li>
              })}
              {(s.skipped.out_of_band > 0).then(|| view! {
                <li>{tp(
                  "radio.chirp-skipped-out-of-band",
                  s.skipped.out_of_band as u32,
                  &[&s.skipped.out_of_band.to_string()],
                )}</li>
              })}
              {(s.skipped.unsupported_tone > 0).then(|| view! {
                <li>{tp(
                  "radio.chirp-skipped-unsupported-tone",
                  s.skipped.unsupported_tone as u32,
                  &[&s.skipped.unsupported_tone.to_string()],
                )}</li>
              })}
            </ul>
          })}
        </div>
      })}

      {move || {
        if failed.get() {
          return view! {
            <EmptyState title=Signal::derive(move || t("radio.lookup-failed-or-no")) />
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
            <Table class="min-w-[720px] text-sm">
              <TableHead>
                <TableRow>
                  <TableHeader class="border px-3 py-2">{move || t("log.callsign")}</TableHeader>
                  <TableHeader class="border px-3 py-2">{move || t("contest.freq")}</TableHeader>
                  <TableHeader class="border px-3 py-2">{move || t("radio.offset")}</TableHeader>
                  <TableHeader class="border px-3 py-2">{move || t("radio.ctcss")}</TableHeader>
                  <TableHeader class="border px-3 py-2">{move || t("radio.city")}</TableHeader>
                  <TableHeader class="border px-3 py-2">{move || t("radio.use")}</TableHeader>
                  <TableHeader class="border px-3 py-2">{move || t("radio.status-column")}</TableHeader>
                </TableRow>
              </TableHead>
              <TableBody>
                {list.into_iter().map(|r| view! {
                  <TableRow>
                    <TableCell class="border px-3 py-1.5 font-mono font-semibold">{r.callsign}</TableCell>
                    <TableCell class="border px-3 py-1.5 tabular-nums text-muted-foreground">{r.frequency_mhz}</TableCell>
                    <TableCell class="border px-3 py-1.5 tabular-nums text-muted-foreground">{r.offset_mhz}</TableCell>
                    <TableCell class="border px-3 py-1.5 tabular-nums text-muted-foreground">{if r.pl.is_empty() { "—".to_owned() } else { r.pl }}</TableCell>
                    <TableCell class="border px-3 py-1.5 text-muted-foreground">{r.city}</TableCell>
                    <TableCell class="border px-3 py-1.5 text-muted-foreground">{r.r#use}</TableCell>
                    <TableCell class="border px-3 py-1.5 text-muted-foreground">{r.status}</TableCell>
                  </TableRow>
                }).collect_view()}
              </TableBody>
            </Table>
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
