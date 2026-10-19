//! RBN（Reverse Beacon Network）：查询「谁收到了我发的 CW / RTTY 信号」（实时监听）。

use ham_web_core::logbook::StationInfo;
use ham_web_core::spot_coverage::Spot;
use ham_web_core::{dxcc, grid};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::components::common::{EmptyState, PageContainer, PageHeader};
use crate::components::spot_coverage::SpotCoverage;
use crate::data;
use crate::i18n::t;
use crate::ui::{
  Button, Input, Size, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, Variant,
};
use crate::util::{set_title, storage};

/// 一条信标报告。
#[derive(Deserialize, Clone)]
struct RbnSpot {
  spotter: String,
  freq_khz: f64,
  band: String,
  mode: String,
  snr: i32,
  wpm: u32,
}

/// `/api/rbn` 返回。
#[derive(Deserialize, Clone)]
struct RbnPayload {
  spots: Vec<RbnSpot>,
}

#[component]
pub fn RbnPage() -> impl IntoView {
  set_title("shell.rbn");

  let default_call = storage::get_json::<StationInfo>("station-info")
    .map(|s| s.callsign)
    .unwrap_or_default();
  let callsign = RwSignal::new(default_call);
  let spots = RwSignal::new(Vec::<RbnSpot>::new());
  let loading = RwSignal::new(false);
  let failed = RwSignal::new(false);
  let queried = RwSignal::new(false);
  let home_grid = RwSignal::new(
    storage::get_json::<StationInfo>("station-info")
      .map(|s| s.gridsquare)
      .unwrap_or_default(),
  );
  // RBN 的 spot 只有 skimmer 呼号、没有网格：按 DXCC 实体中心换算成 4 位网格（约 100 km），
  // 只能算「粗方位」——组件里用 `coarse` 如实标注，不冒充真实网格。
  // `Memo`：每条 spot 都要做一次 DXCC 前缀查询 + 网格换算，`Signal::derive` 会在 `SpotCoverage`
  // 的每个 chip 读取时各重做一遍。
  let spot_list = Memo::new(move |_| {
    spots.with(|list| {
      list
        .iter()
        .map(|s| Spot {
          locator: dxcc::lookup(&s.spotter)
            .and_then(|entity| grid::grid_from_lat_lon(entity.lat, entity.lon))
            .map(|g| g.chars().take(4).collect::<String>())
            .unwrap_or_default(),
          band: s.band.clone(),
          snr: f64::from(s.snr),
          // RBN 数据无时间戳：`None` 让 core 把它排除在 `hourly` 之外，而不是堆到 0 点
          // （页面也传 `show_hourly=false`，两处口径一致）。
          hour_utc: None,
        })
        .collect::<Vec<_>>()
    })
  });

  let run = move || {
    let call = callsign.get().trim().to_uppercase();
    if call.is_empty() {
      crate::ui::push_error(t("common.enter-a-callsign"));
      return;
    }
    // 站信息里的网格可能在别的页面改过：查询时重读，别用挂载时的旧值算方位。
    home_grid.set(
      storage::get_json::<StationInfo>("station-info")
        .map(|s| s.gridsquare)
        .unwrap_or_default(),
    );
    loading.set(true);
    failed.set(false);
    let url = format!("/api/rbn?callsign={call}");
    spawn_local(async move {
      match data::fetch_external_json::<RbnPayload>(&url).await {
        Ok(p) => {
          spots.set(p.spots);
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
        title=move || t("shell.rbn")
        subtitle=move || t("radio.live-cw-rtty-skimmer")
      />

      <PageContainer>
        <section class="pxl-card border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.look-up-reception-reports")}</h2>
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
              {move || if loading.get() { t("radio.listening") } else { t("log.look-up") }}
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
          let list = spots.get();
          if queried.get() && list.is_empty() {
            return view! {
              <EmptyState title=Signal::derive(move || t("radio.no-skimmer-reports-for")) />
            }
            .into_any();
          }
          if list.is_empty() {
            return view! { <div></div> }.into_any();
          }
          view! {
            <SpotCoverage home=home_grid spots=spot_list show_hourly=false coarse=true />
            <section class="pxl-card border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">
                {t("radio.beacon-reports")} {list.len()} {t("radio.entry")}
              </h2>
              <Table class="min-w-[560px] text-sm">
                <TableHead>
                  <TableRow>
                    <TableHeader class="border px-3 py-2">{move || t("radio.skimmers")}</TableHeader>
                    <TableHeader class="border px-3 py-2">{move || t("contest.freq")}</TableHeader>
                    <TableHeader class="border px-3 py-2">{move || t("radio.band")}</TableHeader>
                    <TableHeader class="border px-3 py-2">{move || t("log.mode")}</TableHeader>
                    <TableHeader class="border px-3 py-2 text-right">"SNR"</TableHeader>
                    <TableHeader class="border px-3 py-2 text-right">"WPM"</TableHeader>
                  </TableRow>
                </TableHead>
                <TableBody>
                  {list
                    .iter()
                    .map(|s| {
                      let s = s.clone();
                      view! {
                        <TableRow>
                          <TableCell class="border px-3 py-2 font-medium">{s.spotter.clone()}</TableCell>
                          <TableCell class="border px-3 py-2 tabular-nums text-muted-foreground">{format!("{:.1} kHz", s.freq_khz)}</TableCell>
                          <TableCell class="border px-3 py-2">{s.band.clone()}</TableCell>
                          <TableCell class="border px-3 py-2">{s.mode.clone()}</TableCell>
                          <TableCell class=format!("border px-3 py-2 text-right tabular-nums {}", if s.snr >= 0 { "text-emerald-600" } else { "text-muted-foreground" })>
                            {format!("{} dB", s.snr)}
                          </TableCell>
                          <TableCell class="border px-3 py-2 text-right tabular-nums text-muted-foreground">{s.wpm}</TableCell>
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
          {move || t("radio.data-from-the-reverse")}
        </p>
      </PageContainer>
    </div>
  }
}
