use ham_web_core::intermod::{
  Analysis, FreqError, Kind, MAX_FREQS, analyze, image_frequencies, parse_freqs,
};
use leptos::prelude::*;

use super::RESULT;
use crate::i18n::{t, tf, tp};
use crate::ui::{
  Checkbox, Field, Label, NumberField, Table, TableBody, TableCell, TableHead, TableHeader,
  TableRow, Textarea,
};
use crate::util::unique_id;

/// 示例：同一波段里的两个相邻信号，打开页面就能看到经典的 `2·f1 − f2` / `2·f2 − f1`。
const EXAMPLE: &str = "14.200\n14.205";

/// MHz 频率：最多 6 位小数（1 Hz），去掉多余的 0。
///
/// 不用 `fmt_num`：它对 ≥ 1 的数只留两位小数，会把 14.195 显示成 14.20。
fn fmt_mhz(v: f64) -> String {
  let s = format!("{v:.6}");
  s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

fn error_message(e: FreqError) -> String {
  match e {
    FreqError::Empty => t("tools.intermod-error-empty"),
    FreqError::TooMany => tf("tools.intermod-error-too-many", &[&MAX_FREQS.to_string()]),
    FreqError::Invalid(n) => tf("tools.intermod-error-invalid", &[&n.to_string()]),
  }
}

/// 「落入 20m 波段」或「不在业余波段内」。
fn band_text(band: Option<&str>) -> String {
  match band {
    Some(b) => tf("tools.intermod-in-band", &[b]),
    None => t("tools.intermod-not-in-band"),
  }
}

/// 互调、谐波与镜频：给定若干发射频率，列出产物并标出落入业余波段的。
#[component]
pub(super) fn IntermodCalculator() -> impl IntoView {
  let text = RwSignal::new(EXAMPLE.to_owned());
  let only_bands = RwSignal::new(true);
  let rx = RwSignal::new(145.0);
  let if_mhz = RwSignal::new(10.7);

  let text_id = unique_id("intermod-freqs");
  let only_id = unique_id("intermod-only-bands");
  let rx_id = unique_id("intermod-rx");
  let if_id = unique_id("intermod-if");

  let outcome =
    Memo::new(move |_| -> Result<Analysis, FreqError> { analyze(&parse_freqs(&text.get())?) });
  let image = Memo::new(move |_| image_frequencies(rx.get(), if_mhz.get()));

  view! {
    // `[&>*]:min-w-0`：网格子项默认 `min-width:auto`，表格自带的 `min-w-[…]` 会把整列撑宽，
    // 手机宽度下整页横向滚动。
    <div class="grid gap-3 sm:grid-cols-2 [&>*]:min-w-0">
      <Field
        class="sm:col-span-2"
        label=Signal::derive(move || t("tools.intermod-freqs-label"))
        hint=Signal::derive(move || t("tools.intermod-freqs-hint"))
        r#for=text_id.clone()
      >
        <Textarea
          id=text_id
          value=text
          on_change=Callback::new(move |v: String| text.set(v))
          class="h-28 font-mono text-xs"
        />
      </Field>

      <div class="sm:col-span-2 flex items-center gap-2">
        <Checkbox
          id=only_id.clone()
          checked=only_bands
          on_change=Callback::new(move |v: bool| only_bands.set(v))
        />
        <Label r#for=only_id>{move || t("tools.intermod-only-bands")}</Label>
      </div>

      <div class="sm:col-span-2 space-y-3">
        {move || match outcome.get() {
          Err(e) => {
            view! { <p role="alert" class="text-xs text-destructive">{error_message(e)}</p> }
              .into_any()
          }
          Ok(a) => {
            let total = a.products.len();
            let in_band = a.products.iter().filter(|p| p.band.is_some()).count();
            let inputs = a
              .inputs
              .iter()
              .enumerate()
              .map(|(i, f)| format!("f{} = {} MHz", i + 1, fmt_mhz(*f)))
              .collect::<Vec<_>>()
              .join(" · ");
            let shown: Vec<_> = a
              .products
              .into_iter()
              .filter(|p| !only_bands.get() || p.band.is_some())
              .collect();
            let empty = shown.is_empty();
            let rows = shown
              .into_iter()
              .map(|p| {
                let kind = match p.kind {
                  Kind::Harmonic => t("tools.intermod-kind-harmonic"),
                  Kind::Im2 | Kind::Im3 => t("tools.intermod-kind-intermod"),
                };
                let row_class = if p.band.is_some() {
                  "border-t bg-primary/10"
                } else {
                  "border-t"
                };
                view! {
                  <TableRow class=row_class>
                    <TableCell class="border px-3 py-1.5">{kind}</TableCell>
                    <TableCell class="border px-3 py-1.5 tabular-nums">{p.order}</TableCell>
                    <TableCell class="border px-3 py-1.5 font-mono">{p.expr}</TableCell>
                    <TableCell class="border px-3 py-1.5 tabular-nums">{fmt_mhz(p.freq_mhz)}</TableCell>
                    <TableCell class="border px-3 py-1.5">{p.band.unwrap_or("—")}</TableCell>
                  </TableRow>
                }
              })
              .collect_view();
            view! {
              <div class=format!("{RESULT} space-y-1 tabular-nums")>
                <div>{tf("tools.intermod-inputs", &[&inputs])}</div>
                <div role="status">
                  {tp("tools.intermod-product-count", total as u32, &[&total.to_string()])}
                  "　｜　"
                  {tp("tools.intermod-band-count", in_band as u32, &[&in_band.to_string()])}
                </div>
              </div>
              {if empty {
                view! {
                  <p class="text-xs text-muted-foreground">{move || t("tools.intermod-none-in-band")}</p>
                }
                  .into_any()
              } else {
                view! {
                  <Table class="min-w-[480px] text-sm" wrapper_class="max-h-[28rem] border">
                      <TableHead>
                        <TableRow>
                          <TableHeader class="border px-3 py-2">{move || t("radio.type")}</TableHeader>
                          <TableHeader class="border px-3 py-2">{move || t("tools.intermod-col-order")}</TableHeader>
                          <TableHeader class="border px-3 py-2">{move || t("tools.intermod-col-expr")}</TableHeader>
                          <TableHeader class="border px-3 py-2">{move || t("log.frequency-mhz")}</TableHeader>
                          <TableHeader class="border px-3 py-2">{move || t("tools.intermod-col-band")}</TableHeader>
                        </TableRow>
                      </TableHead>
                      <TableBody>{rows}</TableBody>
                    </Table>
                }
                  .into_any()
              }}
            }
              .into_any()
          }
        }}
      </div>

      <h3 class="sm:col-span-2 pt-2 text-xs font-medium text-foreground">
        {move || t("tools.intermod-image-title")}
      </h3>
      <Field label=Signal::derive(move || t("tools.receive-frequency-mhz")) r#for=rx_id.clone()>
        <NumberField
          id=rx_id
          value=Signal::derive(move || rx.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              rx.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.intermod-if-mhz")) r#for=if_id.clone()>
        <NumberField
          id=if_id
          value=Signal::derive(move || if_mhz.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              if_mhz.set(v);
            }
          })
          controls=false
        />
      </Field>
      <div class=format!("{RESULT} space-y-1 tabular-nums")>
        {move || match image.get() {
          None => view! { <span>{move || t("tools.intermod-image-invalid")}</span> }.into_any(),
          Some(im) => {
            let high = tf(
              "tools.intermod-image-high",
              &[&fmt_mhz(im.high_side_mhz), &band_text(im.high_side_band)],
            );
            let low = match im.low_side_mhz {
              Some(f) => tf(
                "tools.intermod-image-low",
                &[&fmt_mhz(f), &band_text(im.low_side_band)],
              ),
              None => t("tools.intermod-image-low-none"),
            };
            view! {
              <div>{high}</div>
              <div>{low}</div>
            }
              .into_any()
          }
        }}
      </div>

      <p class="sm:col-span-2 text-xs text-muted-foreground">{move || t("tools.intermod-note")}</p>
    </div>
  }
}
