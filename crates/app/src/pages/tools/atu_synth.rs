//! ATU 覆盖综合：一段负载阻抗范围 × 160–6 m，Pi / T 网络的元件范围够不够用。

use ham_web_core::atu_synth::{self, BandReport, Limits, LoadRegion, Topology};
use leptos::prelude::*;

use super::result_class;
use crate::i18n::{t, tf};
use crate::ui::{
  Field, NumberField, RadioGroup, RadioGroupItem, Table, TableBody, TableCell, TableDensity,
  TableHead, TableHeader, TableRow,
};
use crate::util::unique_id;

/// 电容（pF）：一位小数，无解（∞）显示 `—`，不把 `inf` 漏给用户。
fn fmt_pf(v: f64) -> String {
  if v.is_finite() {
    format!("{v:.1}")
  } else {
    "—".to_string()
  }
}

/// 电感（μH）：两位小数，规则同 [`fmt_pf`]。
fn fmt_uh(v: f64) -> String {
  if v.is_finite() {
    format!("{v:.2}")
  } else {
    "—".to_string()
  }
}

#[component]
pub(super) fn AtuSynth() -> impl IntoView {
  let pi = RwSignal::new(true);
  let z0 = RwSignal::new(String::from("50"));
  let r_min = RwSignal::new(String::from("10"));
  let r_max = RwSignal::new(String::from("500"));
  let x_min = RwSignal::new(String::from("-200"));
  let x_max = RwSignal::new(String::from("200"));
  let c_min = RwSignal::new(String::from("10"));
  let c_max = RwSignal::new(String::from("500"));
  let l_min = RwSignal::new(String::from("0.3"));
  let l_max = RwSignal::new(String::from("30"));

  let z0_id = unique_id("atu-z0");
  let r_min_id = unique_id("atu-r-min");
  let r_max_id = unique_id("atu-r-max");
  let x_min_id = unique_id("atu-x-min");
  let x_max_id = unique_id("atu-x-max");
  let c_min_id = unique_id("atu-c-min");
  let c_max_id = unique_id("atu-c-max");
  let l_min_id = unique_id("atu-l-min");
  let l_max_id = unique_id("atu-l-max");
  let pi_id = unique_id("atu-pi");
  let t_id = unique_id("atu-t");

  // 任一输入解析失败 / 范围不合法都落到 `None`，由下面统一给提示；
  // 用 `Memo` 是为了切换拓扑以外的无关重渲染（如语言切换）不重算几十万次闭式求解。
  let report = Memo::new(move |_| -> Option<Vec<BandReport>> {
    let num = |s: RwSignal<String>| s.get().trim().parse::<f64>().ok();
    let region = LoadRegion {
      r_min: num(r_min)?,
      r_max: num(r_max)?,
      x_min: num(x_min)?,
      x_max: num(x_max)?,
    };
    let limits = Limits {
      c_min_pf: num(c_min)?,
      c_max_pf: num(c_max)?,
      l_min_uh: num(l_min)?,
      l_max_uh: num(l_max)?,
    };
    let topology = if pi.get() { Topology::Pi } else { Topology::T };
    atu_synth::analyze(topology, &region, num(z0)?, &limits)
  });

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("tools.atu-synth-z0")) r#for=z0_id.clone()>
        <NumberField id=z0_id value=Signal::derive(move || z0.get()) on_change=Callback::new(move |v: String| z0.set(v)) controls=false />
      </Field>
      <RadioGroup
        value=Signal::derive(move || if pi.get() { "pi" } else { "t" }.to_string())
        on_change=Callback::new(move |v: String| pi.set(v == "pi"))
        aria_label=Signal::derive(move || t("tools.atu-synth-topology"))
        class="flex flex-wrap items-end gap-4 pb-2"
      >
        <label class=format!("{} text-sm", crate::kit::check_row_class())>
          <RadioGroupItem value="pi" id=pi_id />
          {move || t("tools.atu-synth-pi")}
        </label>
        <label class=format!("{} text-sm", crate::kit::check_row_class())>
          <RadioGroupItem value="t" id=t_id />
          {move || t("tools.atu-synth-t")}
        </label>
      </RadioGroup>

      <Field label=Signal::derive(move || t("tools.atu-synth-r-min")) r#for=r_min_id.clone()>
        <NumberField id=r_min_id value=Signal::derive(move || r_min.get()) on_change=Callback::new(move |v: String| r_min.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.atu-synth-r-max")) r#for=r_max_id.clone()>
        <NumberField id=r_max_id value=Signal::derive(move || r_max.get()) on_change=Callback::new(move |v: String| r_max.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.atu-synth-x-min")) r#for=x_min_id.clone()>
        <NumberField id=x_min_id value=Signal::derive(move || x_min.get()) on_change=Callback::new(move |v: String| x_min.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.atu-synth-x-max")) r#for=x_max_id.clone()>
        <NumberField id=x_max_id value=Signal::derive(move || x_max.get()) on_change=Callback::new(move |v: String| x_max.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.atu-synth-c-min")) r#for=c_min_id.clone()>
        <NumberField id=c_min_id value=Signal::derive(move || c_min.get()) on_change=Callback::new(move |v: String| c_min.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.atu-synth-c-max")) r#for=c_max_id.clone()>
        <NumberField id=c_max_id value=Signal::derive(move || c_max.get()) on_change=Callback::new(move |v: String| c_max.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.atu-synth-l-min")) r#for=l_min_id.clone()>
        <NumberField id=l_min_id value=Signal::derive(move || l_min.get()) on_change=Callback::new(move |v: String| l_min.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.atu-synth-l-max")) r#for=l_max_id.clone()>
        <NumberField id=l_max_id value=Signal::derive(move || l_max.get()) on_change=Callback::new(move |v: String| l_max.set(v)) controls=false />
      </Field>

      {move || match report.get() {
        None => {
          view! {
            <div class=result_class()>
              <p>{t("tools.atu-synth-invalid")}</p>
            </div>
          }
            .into_any()
        }
        Some(rows) => {
          let covered: usize = rows.iter().map(|r| r.covered).sum();
          let total: usize = rows.iter().map(|r| r.total).sum();
          let full = rows.iter().filter(|r| r.covered == r.total).count();
          view! {
            <div class=format!("{} tabular-nums", result_class())>
              <p>
                {tf(
                  "tools.atu-synth-summary",
                  &[
                    &covered.to_string(),
                    &total.to_string(),
                    &full.to_string(),
                    &rows.len().to_string(),
                  ],
                )}
              </p>
            </div>
            // 不要再给 `Table` 套一层 `div.sm:col-span-2`：那层 `div` 是网格子项且不裁剪溢出，
            // 自动最小宽度会取表格的 `min-w`（420px），把整列撑宽、连带 `/tools` 其余区块一起溢出
            // 手机宽度。让 `Table` 自己的滚动容器当网格子项（`overflow-x-auto` 使其最小宽度为 0），
            // 跨列写进 `wrapper_class`。
            <Table
              aria_label=Signal::derive(move || t("tools.atu-synth-band-table"))
              class="min-w-[420px]"
              wrapper_class="border sm:col-span-2"
              density=TableDensity::Compact
            >
              <TableHead>
                <TableRow>
                  <TableHeader class=crate::kit::table_cell_class("")>{move || t("radio.band")}</TableHeader>
                  <TableHeader class=crate::kit::table_cell_class("")>{move || t("tools.atu-synth-col-covered")}</TableHeader>
                  <TableHeader class=crate::kit::table_cell_class("")>{move || t("tools.atu-synth-col-need-c")}</TableHeader>
                  <TableHeader class=crate::kit::table_cell_class("")>{move || t("tools.atu-synth-col-need-l")}</TableHeader>
                </TableRow>
              </TableHead>
              <TableBody>
                {rows
                  .into_iter()
                  .map(|r| {
                    view! {
                      <TableRow>
                        <TableCell class=crate::kit::table_cell_class("font-mono")>{r.band}</TableCell>
                        <TableCell class=crate::kit::table_cell_class("font-mono tabular-nums")>
                          {format!("{} / {}", r.covered, r.total)}
                        </TableCell>
                        <TableCell class=crate::kit::table_cell_class("font-mono tabular-nums text-muted-foreground")>
                          {fmt_pf(r.need_c_pf)}
                        </TableCell>
                        <TableCell class=crate::kit::table_cell_class("font-mono tabular-nums text-muted-foreground")>
                          {fmt_uh(r.need_l_uh)}
                        </TableCell>
                      </TableRow>
                    }
                  })
                  .collect_view()}
              </TableBody>
            </Table>
          }
            .into_any()
        }
      }}

      <p class="sm:col-span-2 text-xs text-muted-foreground">{move || t("tools.atu-synth-notes")}</p>
    </div>
  }
}
