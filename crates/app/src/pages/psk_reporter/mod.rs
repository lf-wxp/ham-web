//! PSK Reporter：查询「谁收到了我发的信号」（数字模式实时接收报告）。

mod coverage_map;

use coverage_map::CoverageMap;
use ham_web_core::grid::lat_lon_from_grid;
use ham_web_core::logbook::StationInfo;
use ham_web_core::spot_coverage::Spot;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use wasm_bindgen::JsValue;

use crate::components::common::{EmptyState, PageContainer, PageHeader};
use crate::components::spot_coverage::SpotCoverage;
use crate::data;
use crate::i18n::t;
use crate::ui::{
  Button, Input, Size, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, Variant,
};
use crate::util::{set_title, storage};

/// 单条接收报告。
#[derive(Deserialize, Clone)]
struct PskReport {
  receiver: String,
  freq_khz: f64,
  band: String,
  mode: String,
  snr: i32,
  flow_start_seconds: i64,
  receiver_locator: String,
}

/// `/api/psk-reporter` 返回。
#[derive(Deserialize, Clone)]
struct PskPayload {
  reports: Vec<PskReport>,
}

fn format_utc(unix_sec: i64) -> String {
  let d = js_sys::Date::new(&JsValue::from_f64(unix_sec as f64 * 1000.0));
  format!(
    "{:04}-{:02}-{:02} {:02}:{:02} UTC",
    d.get_utc_full_year(),
    d.get_utc_month() + 1,
    d.get_utc_date(),
    d.get_utc_hours(),
    d.get_utc_minutes()
  )
}

#[component]
pub fn PskReporterPage() -> impl IntoView {
  // 标题也走词条：`shell.psk-reporter` 三语都是这个专名，但统一起见不写字面量。
  set_title("shell.psk-reporter");

  // 默认呼号取本台信息。
  let default_call = storage::get_json::<StationInfo>("station-info")
    .map(|s| s.callsign)
    .unwrap_or_default();
  let callsign = RwSignal::new(default_call);
  let reports = RwSignal::new(Vec::<PskReport>::new());
  let loading = RwSignal::new(false);
  let failed = RwSignal::new(false);
  let queried = RwSignal::new(false);
  // 存活守卫必须在组件体里创建（要挂在当前 Owner 的 `on_cleanup` 上）：`spawn_local`
  // 的续体不随卸载取消，`await` 之后再写已释放的信号会 panic 并带走整个 wasm 实例。
  let alive = StoredValue::new(crate::util::mount_guard());
  // 覆盖玫瑰只在有本台网格时才有意义；网格从本台信息读一次，页面内不改。
  let home_grid = RwSignal::new(
    storage::get_json::<StationInfo>("station-info")
      .map(|s| s.gridsquare)
      .unwrap_or_default(),
  );
  // 两个都是 `Memo`：`Signal::derive` 每次读取都重算，而 `SpotCoverage` 里每个 chip 都会读到
  // 它——几百条报告 × 几十个 chip 就是白白重建几十遍。`with` 借用 `reports`，不整份 clone。
  let spot_list = Memo::new(move |_| {
    reports.with(|list| {
      list
        .iter()
        .map(|r| Spot {
          locator: r.receiver_locator.clone(),
          band: r.band.clone(),
          snr: f64::from(r.snr),
          // 接口给的是 Unix 秒，UTC 小时是纯算术，不经过本地时区。
          #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
          hour_utc: Some((r.flow_start_seconds.rem_euclid(86_400) / 3_600) as u8),
        })
        .collect::<Vec<_>>()
    })
  });
  // 地图只落「有网格的接收台」：没有网格的报告无法定位，宁可不画。
  //
  // 按网格去重：同一个接收台会在不同时间 / 频点重复上报，不去重时地图上叠着几百个完全
  // 重合的点（文案说的是「共 N 个接收台」，条数与它也对不上）。
  let map_points = Memo::new(move |_| {
    reports.with(|list| {
      let mut seen = std::collections::HashSet::new();
      list
        .iter()
        .filter(|r| !r.receiver_locator.trim().is_empty())
        .filter(|r| seen.insert(r.receiver_locator.trim().to_ascii_uppercase()))
        .filter_map(|r| lat_lon_from_grid(&r.receiver_locator))
        .collect::<Vec<_>>()
    })
  });

  let run = move || {
    let call = callsign.get().trim().to_uppercase();
    if call.is_empty() {
      crate::ui::push_error(t("common.enter-a-callsign"));
      return;
    }
    // 站信息里的网格可能在别的页面改过：查询时重读，别用挂载时的旧值算扇区。
    home_grid.set(
      storage::get_json::<StationInfo>("station-info")
        .map(|s| s.gridsquare)
        .unwrap_or_default(),
    );
    loading.set(true);
    failed.set(false);
    let url = format!("/api/psk-reporter?callsign={call}");
    let alive = alive.get_value();
    spawn_local(async move {
      let fetched = data::fetch_external_json::<PskPayload>(&url).await;
      if !alive() {
        return;
      }
      match fetched {
        Ok(p) => {
          reports.set(p.reports);
          queried.set(true);
        }
        Err(_) => failed.set(true),
      }
      loading.set(false);
    });
  };

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("shell.psk-reporter")
        subtitle=move || t("radio.digital-mode-reception-reports")
      />

      <PageContainer>
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.look-up-reception-reports")}</h2>
          <div class="flex flex-wrap gap-3 p-4">
            <Input
              value=callsign
              on_change=Callback::new(move |v: String| callsign.set(v))
              placeholder=Signal::derive(move || t("radio.callsign-e-g-bg1xxx"))
              aria_label=Signal::derive(move || t("log.callsign"))
              class="max-w-xs"
            />
            <Button
              variant=Variant::Default
              size=Size::Default
              on_click=Callback::new(move |_| run())
            >
              {move || if loading.get() { t("log.looking-up") } else { t("log.look-up") }}
            </Button>
          </div>
        </section>

        {move || {
          if failed.get() {
            return view! {
              <EmptyState title=Signal::derive(move || t("radio.lookup-unavailable-make-sure")) />
            }
            .into_any();
          }
          let list = reports.get();
          if queried.get() && list.is_empty() {
            return view! {
              <EmptyState title=Signal::derive(move || t("radio.no-recent-reception-reports")) />
            }
            .into_any();
          }
          if list.is_empty() {
            // 还没查过：给一句「怎么开始」，而不是留一片空白。
            return view! {
              <EmptyState title=Signal::derive(move || t("radio.enter-a-callsign-to-begin")) />
            }
            .into_any();
          }
          view! {
            <SpotCoverage home=home_grid spots=spot_list />
            <CoverageMap points=map_points />
            <section class="pxl-card border bg-card">
              <h2 class="pxl-section-title">
                {t("radio.reception-reports")} {list.len()} {t("radio.entry")}
              </h2>
              <Table class="min-w-[640px] text-sm">
                <TableHead>
                  <TableRow>
                    <TableHeader class="border px-3 py-2">{move || t("log.time")}</TableHeader>
                    <TableHeader class="border px-3 py-2">{move || t("radio.receivers")}</TableHeader>
                    <TableHeader class="border px-3 py-2">{move || t("contest.freq")}</TableHeader>
                    <TableHeader class="border px-3 py-2">{move || t("radio.band")}</TableHeader>
                    <TableHeader class="border px-3 py-2">{move || t("log.mode")}</TableHeader>
                    <TableHeader class="border px-3 py-2 text-right">{move || t("radio.snr")}</TableHeader>
                  </TableRow>
                </TableHead>
                <TableBody>
                  {list
                    .iter()
                    .map(|r| {
                      let r = r.clone();
                      view! {
                        <TableRow>
                          <TableCell class="border px-3 py-2 tabular-nums text-muted-foreground">{format_utc(r.flow_start_seconds)}</TableCell>
                          <TableCell class="border px-3 py-2 font-medium">
                            {r.receiver.clone()}
                            {(!r.receiver_locator.is_empty()).then(|| view! {
                              <span class="ml-1.5 font-mono text-xs text-muted-foreground">{r.receiver_locator.clone()}</span>
                            })}
                          </TableCell>
                          <TableCell class="border px-3 py-2 tabular-nums text-muted-foreground">{format!("{:.1} kHz", r.freq_khz)}</TableCell>
                          <TableCell class="border px-3 py-2">{r.band.clone()}</TableCell>
                          <TableCell class="border px-3 py-2">{r.mode.clone()}</TableCell>
                          <TableCell class=format!("border px-3 py-2 text-right tabular-nums {}", if r.snr >= 0 { "text-emerald-600" } else { "text-muted-foreground" })>
                            {format!("{} dB", r.snr)}
                          </TableCell>
                        </TableRow>
                      }
                    })
                    .collect_view()}
                </TableBody>
              </Table>
            </section>
          }
          .into_any()
        }}

        <p class="text-xs text-muted-foreground">
          {move || t("radio.data-from-psk-reporter")}
        </p>
      </PageContainer>
    </div>
  }
}
