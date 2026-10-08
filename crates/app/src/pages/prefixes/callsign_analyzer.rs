use ham_web_core::callsign::{SLASH_SUFFIXES, STATION_TYPES, parse_callsign};
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::ui::Input;

/// 示例呼号分块说明：`(字符, 名称, 说明, 配色)`。
const EXAMPLE_PARTS: &[(&str, &str, &str, &str)] = &[
  (
    "B",
    "前缀",
    "中国",
    "border-sky-500/40 bg-sky-500/10 text-sky-700 dark:text-sky-400",
  ),
  (
    "G",
    "台站类别",
    "个人业余电台",
    "border-emerald-500/40 bg-emerald-500/10 text-emerald-700 dark:text-emerald-400",
  ),
  (
    "4",
    "分区号",
    "上海 · 山东 · 江苏",
    "border-amber-500/40 bg-amber-500/10 text-amber-700 dark:text-amber-400",
  ),
  (
    "XYZ",
    "后缀",
    "台站唯一标识",
    "border-border bg-muted/40 text-foreground",
  ),
];

/// 呼号结构图解 + 交互式解析。
#[component]
pub(super) fn CallsignAnalyzer() -> impl IntoView {
  let input = RwSignal::new("BG4XYZ".to_owned());

  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.callsign-breakdown")}</h2>
      <div class="space-y-5 p-4">
        // 图解示例
        <div>
          <p class="mb-2 text-xs text-muted-foreground">
            {move || t("radio.taking-bg4xyz-as-an")}
          </p>
          <div class="grid grid-cols-2 gap-2 sm:grid-cols-4">
            {EXAMPLE_PARTS
              .iter()
              .map(|&(ch, label, desc, color)| {
                view! {
                  <div class=format!("rounded-lg border p-3 text-center {color}")>
                    <div class="font-mono text-2xl font-bold">{ch}</div>
                    <div class="mt-1 text-xs font-semibold">{label}</div>
                    <div class="mt-0.5 text-[11px]">{desc}</div>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </div>

        // 交互解析器
        <div>
          <label class="mb-1.5 block text-xs text-muted-foreground">{move || t("radio.enter-a-callsign-to-3")}</label>
          <Input
            value=input
            on_change=Callback::new(move |v: String| input.set(v))
            placeholder=Signal::derive(move || t("radio.e-g-bg4xyz-ja1abc"))
            aria_label=Signal::derive(move || t("radio.enter-a-callsign-to-3"))
            class="font-mono uppercase"
          />
          <div class="mt-3 space-y-1.5 rounded-lg bg-muted/40 p-3">
            {move || {
              let p = parse_callsign(&input.get());
              let entity = p.entity.map(t).unwrap_or_else(|| t("radio.unrecognised"));
              let station = p
                .station_type
                .map(t)
                .unwrap_or_else(|| t("radio.non-chinese-callsign"));
              let area = match (p.area, p.area_regions) {
                (Some(a), Some(r)) => tf("common.zone-2", &[(a), (r)]),
                _ => "—".to_owned(),
              };
              let suffix = if p.suffix.is_empty() {
                "—".to_owned()
              } else {
                p.suffix.clone()
              };
              let slash = p.slash.map(t).unwrap_or_else(|| t("radio.none"));
              view! {
                <div class="flex items-baseline gap-2">
                  <span class="w-24 shrink-0 text-xs text-muted-foreground">{move || t("radio.country-region-2")}</span>
                  <span class="text-sm font-medium">{entity}</span>
                </div>
                <div class="flex items-baseline gap-2">
                  <span class="w-24 shrink-0 text-xs text-muted-foreground">{move || t("radio.station-class")}</span>
                  <span class="text-sm">{station}</span>
                </div>
                <div class="flex items-baseline gap-2">
                  <span class="w-24 shrink-0 text-xs text-muted-foreground">{move || t("radio.zone-3")}</span>
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

        // 台站类别 + 斜杠后缀说明
        <div class="grid gap-4 sm:grid-cols-2">
          <div>
            <h3 class="mb-2 text-xs font-semibold text-muted-foreground">{move || t("radio.chinese-station-class-second")}</h3>
            <div class="space-y-1">
              {STATION_TYPES
                .iter()
                .map(|&(k, v)| {
                  view! {
                    <div class="flex items-baseline gap-2 text-sm">
                      <span class="w-16 shrink-0 font-mono font-semibold text-primary">{k}</span>
                      <span class="text-muted-foreground">{v}</span>
                    </div>
                  }
                })
                .collect_view()}
            </div>
          </div>
          <div>
            <h3 class="mb-2 text-xs font-semibold text-muted-foreground">{move || t("radio.slash-suffix")}</h3>
            <div class="space-y-1">
              {SLASH_SUFFIXES
                .iter()
                .map(|&(k, v)| {
                  view! {
                    <div class="flex items-baseline gap-2 text-sm">
                      <span class="w-16 shrink-0 font-mono font-semibold text-primary">{k}</span>
                      <span class="text-muted-foreground">{v}</span>
                    </div>
                  }
                })
                .collect_view()}
            </div>
          </div>
        </div>
      </div>
    </section>
  }
}
