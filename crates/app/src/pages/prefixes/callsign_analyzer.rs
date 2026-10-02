use ham_web_core::callsign::{SLASH_SUFFIXES, STATION_TYPES, parse_callsign};
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::ui::input_class;

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
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("呼号解析")}</h2>
      <div class="space-y-5 p-4">
        // 图解示例
        <div>
          <p class="mb-2 text-xs text-muted-foreground">
            {move || t("以「BG4XYZ」为例，中国业余电台呼号由四部分组成：")}
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
          <label class="mb-1.5 block text-xs text-muted-foreground">{move || t("输入呼号，解析各部分含义")}</label>
          <input
            type="text"
            placeholder=move || t("如 BG4XYZ、JA1ABC、K1ZZ/QRP")
            class=input_class("font-mono uppercase")
            prop:value=move || input.get()
            on:input=move |e| input.set(event_target_value(&e))
          />
          <div class="mt-3 space-y-1.5 rounded-lg bg-muted/40 p-3">
            {move || {
              let p = parse_callsign(&input.get());
              let entity = p.entity.map(t).unwrap_or_else(|| t("未识别"));
              let station = p
                .station_type
                .map(t)
                .unwrap_or_else(|| t("—（非中国呼号）"));
              let area = match (p.area, p.area_regions) {
                (Some(a), Some(r)) => tf("{} 区（{}）", &[(a), (r)]),
                _ => "—".to_owned(),
              };
              let suffix = if p.suffix.is_empty() {
                "—".to_owned()
              } else {
                p.suffix.clone()
              };
              let slash = p.slash.map(t).unwrap_or_else(|| t("无"));
              view! {
                <div class="flex items-baseline gap-2">
                  <span class="w-24 shrink-0 text-xs text-muted-foreground">{move || t("国家/地区")}</span>
                  <span class="text-sm font-medium">{entity}</span>
                </div>
                <div class="flex items-baseline gap-2">
                  <span class="w-24 shrink-0 text-xs text-muted-foreground">{move || t("台站类别")}</span>
                  <span class="text-sm">{station}</span>
                </div>
                <div class="flex items-baseline gap-2">
                  <span class="w-24 shrink-0 text-xs text-muted-foreground">{move || t("分区")}</span>
                  <span class="text-sm">{area}</span>
                </div>
                <div class="flex items-baseline gap-2">
                  <span class="w-24 shrink-0 text-xs text-muted-foreground">{move || t("后缀")}</span>
                  <span class="text-sm font-mono">{suffix}</span>
                </div>
                <div class="flex items-baseline gap-2">
                  <span class="w-24 shrink-0 text-xs text-muted-foreground">{move || t("斜杠后缀")}</span>
                  <span class="text-sm">{slash}</span>
                </div>
              }
            }}
          </div>
        </div>

        // 台站类别 + 斜杠后缀说明
        <div class="grid gap-4 sm:grid-cols-2">
          <div>
            <h3 class="mb-2 text-xs font-semibold text-muted-foreground">{move || t("中国台站类别（第二位）")}</h3>
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
            <h3 class="mb-2 text-xs font-semibold text-muted-foreground">{move || t("斜杠后缀")}</h3>
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
