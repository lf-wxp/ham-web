//! 中继台数据库查询：按国家拉取 RepeaterBook 中继台，支持本地按呼号 / 城市筛选。

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::data;
use crate::i18n::{t, tf};
use crate::ui::{Size, Variant, button_class, input_class};

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
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("中继台数据库查询")}</h2>
      <p class="px-4 pt-3 text-xs text-muted-foreground">
        {move || t("按国家 / 地区拉取 RepeaterBook 收录的中继台（英文名，如 China、Japan、United States），可按呼号或城市筛选。")}
      </p>
      <div class="flex flex-wrap items-center gap-3 p-4">
        <input
          type="text"
          placeholder=move || t("国家 / 地区（如 China）")
          aria-label=move || t("国家")
          prop:value=move || country.get()
          on:input=move |e| country.set(event_target_value(&e))
          class=input_class("max-w-xs")
        />
        <button type="button" class=button_class(Variant::Default, Size::Default, "") on:click=move |_| run()>
          {move || if loading.get() { t("查询中…") } else { t("查询") }}
        </button>
        {move || payload.get().is_some().then(|| view! {
          <input
            type="text"
            placeholder=move || t("筛选呼号 / 城市")
            aria-label=move || t("筛选")
            prop:value=move || filter.get()
            on:input=move |e| filter.set(event_target_value(&e))
            class=input_class("max-w-xs")
          />
        })}
      </div>

      {move || {
        if failed.get() {
          return view! {
            <p class="px-4 pb-4 text-sm text-muted-foreground">
              {move || t("查询失败或该地区暂无数据，请确认已通过后端（dev-full / serve）访问。")}
            </p>
          }
          .into_any();
        }
        filtered().map(|(c, list)| view! {
          <div class="border-t">
            <div class="px-4 py-2 text-xs text-muted-foreground">
              {tf("{} · 共 {} 条", &[&(c).to_string(), &(list.len()).to_string()])}
            </div>
            <div class="overflow-x-auto">
              <table class="w-full min-w-[720px] border-collapse text-sm">
                <thead class="bg-muted/60 text-xs">
                  <tr>
                    <th class="border px-3 py-2 text-left">{move || t("呼号")}</th>
                    <th class="border px-3 py-2 text-left">{move || t("频率")}</th>
                    <th class="border px-3 py-2 text-left">{move || t("频差")}</th>
                    <th class="border px-3 py-2 text-left">{move || t("亚音")}</th>
                    <th class="border px-3 py-2 text-left">{move || t("城市")}</th>
                    <th class="border px-3 py-2 text-left">{move || t("使用")}</th>
                    <th class="border px-3 py-2 text-left">{move || t("状态")}</th>
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
        {move || t("数据来自 RepeaterBook 公开接口；中国的业余中继数据较少，可切换到其他国家 / 地区查看。")}
      </p>
    </section>
  }
}
