//! DXCC 申请记录表：每个实体一条代表通联，可导出 CSV 作核对清单。

use ham_web_core::award_apply::{ApplyVia, dxcc_record_sheet, to_csv};
use ham_web_core::logbook::LogEntry;
use leptos::prelude::*;

use crate::i18n::{localized_name, t, tp};
use crate::ui::{
  Button, Size, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, Variant,
};
use crate::util::download_text;

const CELL: &str = "border px-3 py-2 align-top";

fn via_key(via: ApplyVia) -> &'static str {
  match via {
    ApplyVia::None => "log.via-none",
    ApplyVia::Card => "log.via-card",
    ApplyVia::Lotw => "log.via-lotw",
    ApplyVia::Eqsl => "log.via-eqsl",
    ApplyVia::Qrz => "log.via-qrz",
  }
}

#[component]
pub(super) fn DxccApplySheet(
  entries: Vec<LogEntry>,
  /// 展开态由上层持有：这个组件挂在日志页的响应式闭包里，日志一变就会整体重建，
  /// 展开态若放在组件内部，用户每记一条通联都会被收起。
  open: RwSignal<bool>,
) -> impl IntoView {
  let rows = StoredValue::new(dxcc_record_sheet(&entries));
  let export = Callback::new(move |_| {
    rows.with_value(|rows| {
      download_text("dxcc-apply.csv", &to_csv(rows), "text/csv;charset=utf-8");
    });
  });

  view! {
    // 收起时**不渲染**整张表：一百多行的表格一直挂在 DOM 里既拖慢页面，
    // 也会让按文本定位的用例撞上隐藏的同名单元格。
    <div class="border-t" data-slot="dxcc-apply">
      <Button
        variant=Variant::Text
        size=Size::Sm
        class="w-full justify-start px-4 py-3 text-sm font-semibold"
        aria_expanded=Signal::derive(move || open.get())
        aria_controls="dxcc-apply-body"
        on_click=Callback::new(move |_| open.update(|v| *v = !*v))
      >
        {move || rows.with_value(|rows| tp("log.apply-sheet", rows.len(), &[&rows.len().to_string()]))}
      </Button>
      <Show when=move || open.get()>
        <div id="dxcc-apply-body" class="space-y-3 px-4 pb-4">
        <p class="text-xs text-muted-foreground">{move || t("log.apply-sheet-desc")}</p>

        {move || {
          rows.with_value(|rows| {
            if rows.is_empty() {
              return view! {
                <p class="bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
                  {move || t("log.apply-sheet-empty")}
                </p>
              }
                .into_any();
            }
            let body = rows
              .iter()
              .map(|row| {
                let entity = localized_name(row.entity, row.entity_en);
                let callsign = row.callsign.clone();
                let date = row.date.clone();
                let band = row.band.clone();
                let mode = row.mode.clone();
                let via = t(via_key(row.via));
                view! {
                  <TableRow>
                    <TableCell class=format!("{CELL} whitespace-nowrap font-medium")>
                      {entity}
                    </TableCell>
                    <TableCell class=format!("{CELL} whitespace-nowrap font-mono")>
                      {callsign}
                    </TableCell>
                    <TableCell class=format!("{CELL} whitespace-nowrap tabular-nums")>
                      {date}
                    </TableCell>
                    <TableCell class=format!("{CELL} whitespace-nowrap")>{band}</TableCell>
                    <TableCell class=format!("{CELL} whitespace-nowrap")>{mode}</TableCell>
                    <TableCell class=format!("{CELL} whitespace-nowrap text-muted-foreground")>
                      {via}
                    </TableCell>
                  </TableRow>
                }
              })
              .collect_view();
            view! {
              <div class="space-y-2">
                <div class="flex justify-end">
                  <Button variant=Variant::Outline size=Size::Sm on_click=export>
                    {move || t("log.export-csv")}
                  </Button>
                </div>
                <Table
                  class="min-w-[640px] text-sm"
                  aria_label=Signal::derive(|| t("log.apply-sheet-table"))
                  tabindex="0"
                  role="region"
                >
                  <TableHead>
                    <TableRow>
                      <TableHeader class=CELL>{move || t("contest.entity")}</TableHeader>
                      <TableHeader class=CELL>{move || t("log.callsign")}</TableHeader>
                      <TableHeader class=CELL>{move || t("log.date")}</TableHeader>
                      <TableHeader class=CELL>{move || t("radio.band")}</TableHeader>
                      <TableHeader class=CELL>{move || t("log.mode")}</TableHeader>
                      <TableHeader class=CELL>{move || t("log.qsl-via")}</TableHeader>
                    </TableRow>
                  </TableHead>
                  <TableBody>{body}</TableBody>
                </Table>
              </div>
            }
              .into_any()
          })
        }}
        </div>
      </Show>
    </div>
  }
}
