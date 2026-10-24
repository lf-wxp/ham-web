//! 呼号结构图解 + 交互式解析（原来在 `/prefixes`，现归到呼号知识总页）。

use ham_web_core::callsign::parse_callsign;
use leptos::prelude::*;

use crate::data;
use crate::i18n::{t, tf};
use crate::ui::Input;

/// 示例呼号 `BG4XYZ` 的分段：`(字符, 文案 key, 配色)`。
const EXAMPLE_PARTS: &[(&str, &str, &str)] = &[
  ("B", "radio.prefix", "border-xp/40 bg-xp/10 text-xp-text"),
  (
    "G",
    "radio.station-class",
    "border-win/40 bg-win/10 text-win-text",
  ),
  (
    "4",
    "radio.zone-label",
    "border-gold/40 bg-gold/10 text-gold-text",
  ),
  (
    "XYZ",
    "radio.suffix",
    "border-border bg-inset text-foreground",
  ),
];

#[component]
pub fn CallsignStructure() -> impl IntoView {
  let input = RwSignal::new("BG4XYZ".to_owned());

  view! {
    <section class="pxl-card border bg-card">
      <h2 class="pxl-section-title">{move || t("radio.callsign-breakdown")}</h2>
      <div class="space-y-5 p-4">
        <div>
          <p class="mb-2 text-xs text-muted-foreground">
            {move || t("radio.taking-bg4xyz-as-an")}
          </p>
          <div class="grid grid-cols-2 gap-2 sm:grid-cols-4">
            {EXAMPLE_PARTS
              .iter()
              .map(|&(ch, key, color)| {
                view! {
                  <div class=format!("border p-3 text-center {color}")>
                    <div class="font-mono text-2xl">{ch}</div>
                    <div class="mt-1 text-xs">{move || t(key)}</div>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </div>

        <div>
          <label class="mb-1.5 block text-xs text-muted-foreground">{move || t("radio.enter-a-callsign-to-analyze")}</label>
          <Input
            value=input
            on_change=Callback::new(move |v: String| input.set(v))
            placeholder=Signal::derive(move || t("radio.e-g-bg4xyz-ja1abc"))
            aria_label=Signal::derive(move || t("radio.enter-a-callsign-to-analyze"))
            class="font-mono uppercase"
          />
          <div class="mt-3 space-y-1.5 bg-inset p-3">
            {move || {
              let p = parse_callsign(&input.get());
              let entity = p.entity.map(t).unwrap_or_else(|| t("radio.unrecognised"));
              // 类别与斜杠后缀文案来自 core，译文在知识库词典（`kt`），不在界面词典（`t`）。
              let station = p
                .station_type
                .map(data::kt_tracked)
                .unwrap_or_else(|| t("radio.non-chinese-callsign"));
              let area = match (p.area, p.area_regions) {
                // 地区串是知识库正文，要过 `kt` 才翻；直接拼 `tf` 会在英文界面漏出中文省名。
                (Some(a), Some(r)) => {
                  let regions = data::kt_tracked(r);
                  tf("common.zone-with-regions", &[a, &regions])
                }
                _ => "—".to_owned(),
              };
              let suffix = if p.suffix.is_empty() {
                "—".to_owned()
              } else {
                p.suffix.clone()
              };
              let slash = p
                .slash
                .map(data::kt_tracked)
                .unwrap_or_else(|| t("radio.none"));
              view! {
                <div class="flex items-baseline gap-2">
                  <span class="w-24 shrink-0 text-xs text-muted-foreground">{move || t("radio.country-region-label")}</span>
                  <span class="text-sm">{entity}</span>
                </div>
                <div class="flex items-baseline gap-2">
                  <span class="w-24 shrink-0 text-xs text-muted-foreground">{move || t("radio.station-class")}</span>
                  <span class="text-sm">{station}</span>
                </div>
                <div class="flex items-baseline gap-2">
                  <span class="w-24 shrink-0 text-xs text-muted-foreground">{move || t("radio.zone-label")}</span>
                  <span class="text-sm">{area}</span>
                </div>
                <div class="flex items-baseline gap-2">
                  <span class="w-24 shrink-0 text-xs text-muted-foreground">{move || t("radio.suffix")}</span>
                  <span class="text-sm font-mono">{suffix}</span>
                </div>
                <div class="flex items-baseline gap-2">
                  <span class="w-24 shrink-0 text-xs text-muted-foreground">{move || t("radio.slash-suffix")}</span>
                  <span class="text-sm">{slash}</span>
                </div>
              }
            }}
          </div>
        </div>
      </div>
    </section>
  }
}
