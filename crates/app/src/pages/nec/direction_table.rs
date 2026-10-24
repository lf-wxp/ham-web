//! 方向增益表：每个 DXCC 实体方向上的天线真实增益。
//!
//! 口径是「方向增益」而不是「覆盖」：只算天线本身（含地面模型与效率），
//! 不计传播、噪声、功率与对方天线。详见 `docs/antenna-coverage.md`。

use ham_web_core::antenna_coverage::{self, EntityGain, GainMode, PatternSlices};
use ham_web_core::logbook::StationInfo;
use ham_web_core::nec::NecResult;
use leptos::prelude::*;

use crate::i18n::{localized_name, t};
use crate::ui::{
  Button, Chip, ChipGroup, Field, Input, Size, Table, TableBody, TableCell, TableHead, TableHeader,
  TableRow, Variant,
};
use crate::util::storage;

const CELL: &str = "border px-3 py-2 align-top";

#[derive(Clone, PartialEq)]
enum State {
  NoResult,
  /// 还没填本台网格。
  NoGrid,
  /// 填了，但不是合法的 Maidenhead 网格——与「没填」是两种提示。
  BadGrid,
  Rows(Vec<EntityGain>),
}

#[component]
pub(super) fn DirectionTable(result: Memo<Option<NecResult>>) -> impl IntoView {
  let open = RwSignal::new(false);
  let best_elevation = RwSignal::new(true);
  let home = RwSignal::new(
    storage::get_json::<StationInfo>("station-info")
      .map(|s| s.gridsquare)
      .unwrap_or_default(),
  );
  // `with` 而不是 `get`：`NecResult` 里带着 `currents` / `pattern3d`（几百个 f64），
  // 而这段随网格输入每次按键重算，没必要整份 clone。
  let state = Memo::new(move |_| {
    result.with(|maybe| {
      let Some(result) = maybe.as_ref() else {
        return State::NoResult;
      };
      let grid = home.get();
      if grid.trim().is_empty() {
        return State::NoGrid;
      }
      let mode = if best_elevation.get() {
        GainMode::BestElevation
      } else {
        GainMode::Horizontal
      };
      let slices = PatternSlices::from(result);
      match antenna_coverage::direction_gains(&slices, &grid, mode) {
        Some(rows) if !rows.is_empty() => State::Rows(rows),
        // 实体表非空，所以只有「网格解析不出来」会走到这里。
        _ => State::BadGrid,
      }
    })
  });

  view! {
    // 与同级区块（`nvis.rs` 等）同一套卡片写法：`pxl-card border bg-card`。
    <div class="pxl-card border bg-card" data-slot="nec-direction">
      <Button
        variant=Variant::Text
        size=Size::Sm
        class="w-full justify-start px-4 py-3 text-sm"
        aria_expanded=Signal::derive(move || open.get())
        aria_controls="nec-direction-body"
        on_click=Callback::new(move |_| open.update(|v| *v = !*v))
      >
        {move || t("tools.nec-direction-title")}
      </Button>

      <Show when=move || open.get()>
        <div id="nec-direction-body" class="space-y-3 px-4 pb-4">
          <p class="text-xs text-muted-foreground">{move || t("tools.nec-direction-hint")}</p>

          <div class="flex flex-wrap items-end gap-3">
            // 页面上只有一张方向表，id 用固定值即可（`Field` 的 for 与控件的 id 必须配对）。
            <Field
              label=Signal::derive(|| t("tools.nec-direction-grid"))
              r#for="nec-direction-grid"
            >
              <Input
                id="nec-direction-grid"
                value=home
                on_change=Callback::new(move |v: String| home.set(v))
              />
            </Field>
            // 本台网格在站信息页改过之后，这里不会自动跟随（只在挂载时读一次）；
            // 给一个显式的「重新读取」，比让用户刷新整个页面好。
            <Button
              variant=Variant::Text
              size=Size::Xs
              on_click=Callback::new(move |_| {
                home.set(
                  storage::get_json::<StationInfo>("station-info")
                    .map(|s| s.gridsquare)
                    .unwrap_or_default(),
                );
              })
            >
              {move || t("tools.nec-direction-reload")}
            </Button>
            <div class="space-y-1.5">
              <span class="text-sm">{move || t("tools.nec-direction-mode")}</span>
              <ChipGroup
                value=Signal::derive(move || {
                  if best_elevation.get() { "best" } else { "horizontal" }.to_owned()
                })
                on_change=Callback::new(move |v: String| best_elevation.set(v == "best"))
                aria_label=Signal::derive(|| t("tools.nec-direction-mode"))
              >
                <Chip value="best">{move || t("tools.nec-direction-mode-best")}</Chip>
                <Chip value="horizontal">{move || t("tools.nec-direction-mode-horizontal")}</Chip>
              </ChipGroup>
            </div>
          </div>

          {move || match state.get() {
            State::NoResult => {
              view! {
                <p class="bg-inset px-3 py-2 text-sm text-muted-foreground">
                  {move || t("tools.nec-direction-no-result")}
                </p>
              }
                .into_any()
            }
            State::NoGrid => {
              view! {
                <p class="bg-inset px-3 py-2 text-sm text-muted-foreground">
                  {move || t("tools.nec-direction-no-grid")}
                </p>
              }
                .into_any()
            }
            State::BadGrid => {
              view! {
                <p class="bg-inset px-3 py-2 text-sm text-muted-foreground">
                  {move || t("tools.nec-direction-bad-grid")}
                </p>
              }
                .into_any()
            }
            State::Rows(rows) => {
              let body = rows
                .iter()
                .map(|row| {
                  let name = localized_name(row.name, row.name_en);
                  let azimuth = format!("{:.0}°", row.azimuth_deg);
                  let distance = format!("{:.0} km", row.distance_km);
                  let gain = format!("{:.1} dBi", row.gain_dbi);
                  let loss = format!("{:.1} dB", row.loss_db);
                  view! {
                    <TableRow>
                      <TableCell class=format!("{CELL} whitespace-nowrap")>
                        {name}
                      </TableCell>
                      <TableCell class=format!("{CELL} whitespace-nowrap tabular-nums")>
                        {azimuth}
                      </TableCell>
                      <TableCell class=format!("{CELL} whitespace-nowrap tabular-nums")>
                        {distance}
                      </TableCell>
                      <TableCell class=format!("{CELL} whitespace-nowrap tabular-nums")>
                        {gain}
                      </TableCell>
                      <TableCell class=format!("{CELL} whitespace-nowrap tabular-nums text-muted-foreground")>
                        {loss}
                      </TableCell>
                    </TableRow>
                  }
                })
                .collect_view();
              // 限高写在 `Table` 的 `wrapper_class` 上，而不是外面再包一层 `overflow-auto`：
              // 嵌套两层滚动容器会让横向滚动条落在 250 行表格的最底部，且外层不可聚焦，
              // 键盘用户滚不动。`tabindex` + `role="region"` + 名称让它可聚焦、可读出。
              view! {
                <Table
                  class="min-w-[560px] text-sm"
                  wrapper_class="max-h-[28rem]"
                  aria_label=Signal::derive(|| t("tools.nec-direction-title"))
                  tabindex="0"
                  role="region"
                >
                  <TableHead>
                    <TableRow>
                      <TableHeader class=CELL>{move || t("contest.entity")}</TableHeader>
                      <TableHeader class=CELL>
                        {move || t("tools.nec-direction-bearing")}
                      </TableHeader>
                      <TableHeader class=CELL>
                        {move || t("tools.nec-direction-distance")}
                      </TableHeader>
                      <TableHeader class=CELL>{move || t("tools.nec-direction-gain")}</TableHeader>
                      <TableHeader class=CELL>{move || t("tools.nec-direction-loss")}</TableHeader>
                    </TableRow>
                  </TableHead>
                  <TableBody>{body}</TableBody>
                </Table>
              }
                .into_any()
            }
          }}
        </div>
      </Show>
    </div>
  }
}
