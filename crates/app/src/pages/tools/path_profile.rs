use ham_web_core::fresnel::{K_STANDARD, Verdict};
use ham_web_core::path_profile::{
  MAX_POINTS, ParseError, ParseErrorKind, ProfileError, ProfileParams, ProfileReport, analyze,
  parse_points,
};
use leptos::prelude::*;

use super::fmt_num;
use super::path_profile_chart::PathProfileChart;
use crate::i18n::{t, tf};
use crate::ui::{
  Field, NumberField, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, Textarea,
};
use crate::util::unique_id;

/// 示例剖面：一座 310 m 的山脊挡在 30 km 路径中段偏 A 端处，打开页面就能看到「被挡」的样子。
const EXAMPLE: &str = "0, 120\n5, 180\n10, 260\n15, 310\n20, 240\n25, 170\n30, 130";

/// 输入没法给出结论的原因：文本解析失败，或点 / 参数不满足分析条件。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Failure {
  Parse(ParseError),
  Analyze(ProfileError),
}

fn failure_message(f: Failure) -> String {
  match f {
    Failure::Parse(ParseError { line, kind }) => {
      let key = match kind {
        ParseErrorKind::FieldCount => "tools.profile-error-field-count",
        ParseErrorKind::NotANumber => "tools.profile-error-not-number",
      };
      tf(key, &[&line.to_string()])
    }
    Failure::Analyze(ProfileError::TooFewPoints) => t("tools.profile-error-too-few"),
    Failure::Analyze(ProfileError::TooManyPoints) => {
      tf("tools.profile-error-too-many", &[&MAX_POINTS.to_string()])
    }
    Failure::Analyze(ProfileError::NotIncreasing(n)) => {
      tf("tools.profile-error-not-increasing", &[&n.to_string()])
    }
    Failure::Analyze(ProfileError::BadParameter) => t("tools.profile-error-bad-parameter"),
  }
}

/// 路径剖面：粘贴沿路径的高程点，逐点判定地球曲率与第一菲涅尔区净空，找出最危险的一点。
#[component]
pub(super) fn PathProfile() -> impl IntoView {
  let text = RwSignal::new(EXAMPLE.to_string());
  let freq = RwSignal::new(145.0);
  let tx_height = RwSignal::new(30.0);
  let rx_height = RwSignal::new(30.0);
  // 取 4 位小数，输入框里才不会显示 1.3333333333333333。
  let k = RwSignal::new((K_STANDARD * 1e4).round() / 1e4);

  let text_id = unique_id("profile-points");
  let freq_id = unique_id("profile-freq");
  let tx_id = unique_id("profile-tx-height");
  let rx_id = unique_id("profile-rx-height");
  let k_id = unique_id("profile-k");

  let outcome = Memo::new(move |_| -> Result<ProfileReport, Failure> {
    let points = parse_points(&text.get()).map_err(Failure::Parse)?;
    analyze(
      &points,
      &ProfileParams {
        freq_mhz: freq.get(),
        tx_height_m: tx_height.get(),
        rx_height_m: rx_height.get(),
        k: k.get(),
      },
    )
    .map_err(Failure::Analyze)
  });

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field
        class="sm:col-span-2"
        label=Signal::derive(move || t("tools.profile-points-label"))
        hint=Signal::derive(move || t("tools.profile-points-hint"))
        r#for=text_id.clone()
      >
        <Textarea
          id=text_id
          value=text
          on_change=Callback::new(move |v: String| text.set(v))
          class="h-40 font-mono text-xs"
        />
      </Field>
      <Field label=Signal::derive(move || t("log.frequency-mhz")) r#for=freq_id.clone()>
        <NumberField
          id=freq_id
          value=Signal::derive(move || freq.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              freq.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.fresnel-k-factor")) r#for=k_id.clone()>
        <NumberField
          id=k_id
          value=Signal::derive(move || k.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              k.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.fresnel-antenna-a-height-m")) r#for=tx_id.clone()>
        <NumberField
          id=tx_id
          value=Signal::derive(move || tx_height.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              tx_height.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.fresnel-antenna-b-height-m")) r#for=rx_id.clone()>
        <NumberField
          id=rx_id
          value=Signal::derive(move || rx_height.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              rx_height.set(v);
            }
          })
          controls=false
        />
      </Field>

      // `min-w-0`：网格子项默认 `min-width: auto`，里面的剖面表有 `min-w-[560px]`，不加这个
      // 就会把整列撑到 560px 以上，手机宽度下整页出现横向滚动（表格自己的 `overflow-x-auto` 不起作用）。
      <div class="min-w-0 space-y-3 sm:col-span-2">
        {move || match outcome.get() {
          Err(f) => {
            view! {
              <div
                role="alert"
                class="bg-muted/40 px-3 py-2 text-sm text-destructive"
              >
                {failure_message(f)}
              </div>
            }
              .into_any()
          }
          Ok(r) => {
            let verdict = match r.verdict {
              Verdict::Full => t("tools.fresnel-verdict-full"),
              Verdict::Sufficient => t("tools.fresnel-verdict-sufficient"),
              Verdict::Partial => t("tools.fresnel-verdict-partial"),
              Verdict::Blocked => t("tools.fresnel-verdict-blocked"),
            };
            let worst = r.rows[r.worst_index];
            let worst_clearance = worst.clearance_m.unwrap_or_default();
            let advice = if r.raise_60_m > 0.0 {
              tf(
                "tools.profile-raise-result",
                &[&fmt_num(r.raise_60_m), &fmt_num(r.raise_full_m), &fmt_num(r.raise_los_m)],
              )
            } else {
              tf("tools.profile-margin-spare", &[&fmt_num(r.margin_60_m)])
            };
            let summary = tf(
              "tools.profile-summary",
              &[
                &fmt_num(r.total_km),
                &fmt_num(r.tx_abs_m),
                &fmt_num(r.rx_abs_m),
                &fmt_num(r.wavelength_m),
              ],
            );
            let worst_text = tf(
              "tools.profile-worst-result",
              &[
                &fmt_num(worst.dist_km),
                &fmt_num(worst.elev_m),
                &fmt_num(worst_clearance),
                &format!("{:.0}", worst.clearance_ratio.unwrap_or_default() * 100.0),
              ],
            );
            let worst_index = r.worst_index;
            let rows = r
              .rows
              .iter()
              .enumerate()
              .map(|(i, row)| {
                let dash = "—".to_string();
                let dist = fmt_num(row.dist_km);
                let elev = fmt_num(row.elev_m);
                let bulge = if row.clearance_m.is_some() {
                  fmt_num(row.bulge_m)
                } else {
                  dash.clone()
                };
                let fresnel = if row.clearance_m.is_some() {
                  fmt_num(row.fresnel1_m)
                } else {
                  dash.clone()
                };
                let clearance = row.clearance_m.map(fmt_num).unwrap_or_else(|| dash.clone());
                let ratio = row
                  .clearance_ratio
                  .map(|v| format!("{:.0}%", v * 100.0))
                  .unwrap_or_else(|| dash.clone());
                let emphasized = if i == worst_index { "bg-accent" } else { "" };
                view! {
                  <TableRow class=emphasized>
                    <TableCell class="border px-3 py-2 font-mono tabular-nums">{dist}</TableCell>
                    <TableCell class="border px-3 py-2 font-mono tabular-nums">{elev}</TableCell>
                    <TableCell class="border px-3 py-2 font-mono tabular-nums">{bulge}</TableCell>
                    <TableCell class="border px-3 py-2 font-mono tabular-nums">{fresnel}</TableCell>
                    <TableCell class="border px-3 py-2 font-mono tabular-nums">{clearance}</TableCell>
                    <TableCell class="border px-3 py-2 font-mono tabular-nums">{ratio}</TableCell>
                  </TableRow>
                }
              })
              .collect_view();
            view! {
              <div class="space-y-1 bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
                <div role="status" class="font-medium text-foreground">{verdict}</div>
                <div>{summary}</div>
                <div>{worst_text}</div>
                <div>{advice}</div>
              </div>
              <PathProfileChart report=r />
              <p class="text-xs text-muted-foreground">{move || t("tools.profile-chart-note")}</p>
              <Table class="min-w-[560px] text-sm" wrapper_class="border">
                <TableHead>
                  <TableRow>
                      <TableHeader class="border px-3 py-2">{move || t("tools.distance-km")}</TableHeader>
                      <TableHeader class="border px-3 py-2">{move || t("tools.profile-col-elevation")}</TableHeader>
                      <TableHeader class="border px-3 py-2">{move || t("tools.profile-col-bulge")}</TableHeader>
                      <TableHeader class="border px-3 py-2">{move || t("tools.profile-col-fresnel")}</TableHeader>
                      <TableHeader class="border px-3 py-2">{move || t("tools.profile-col-clearance")}</TableHeader>
                      <TableHeader class="border px-3 py-2">{move || t("tools.profile-col-ratio")}</TableHeader>
                    </TableRow>
                  </TableHead>
                  <TableBody>{rows}</TableBody>
                </Table>
            }
              .into_any()
          }
        }}
      </div>
      <p class="sm:col-span-2 text-xs text-muted-foreground">
        {move || t("tools.profile-assumptions")}
      </p>
    </div>
  }
}
