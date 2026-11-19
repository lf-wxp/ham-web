//! 字体体系：字族、字阶、可输入的预览文本、易读字体开关与组件排版类。

use leptos::prelude::*;

use crate::components::common::SectionCard;
use crate::i18n::t;
use crate::theme::use_display_prefs;
use crate::ui::{Field, Input, Switch};

use super::spec_data::TYPE_STEPS;

const FAMILIES: &[(&str, &str)] = &[
  ("design.type-family-body", "--font-pixel-body"),
  ("design.type-family-mono", "--font-pixel-mono"),
  ("design.type-family-display", "--font-pixel-display"),
  ("design.type-family-label", "--font-pixel-label"),
];

const CLASSES: &[&str] = &[
  "pxl-title",
  "pxl-section-title",
  "pxl-label",
  "pxl-sub-head",
  "pxl-stat",
];

#[component]
pub fn TypographySection() -> impl IntoView {
  let prefs = use_display_prefs();
  let sample = RwSignal::new(String::new());
  let preview = move || {
    let typed = sample.get();
    if typed.is_empty() {
      t("design.type-sample")
    } else {
      typed
    }
  };

  view! {
    <SectionCard title=Signal::derive(move || t("design.type-title"))>
      <div class="space-y-5 p-4">
        <p class="text-xs leading-relaxed text-muted-foreground">
          {move || t("design.type-hint")}
        </p>

        <section class="space-y-2">
          <h3 class="pxl-sub-head">{move || t("design.type-families")}</h3>
          <div class="grid gap-2 md:grid-cols-2">
            {FAMILIES
              .iter()
              .map(|(key, var)| {
                view! {
                  <div class=crate::kit::tile_class("space-y-1 p-3 min-w-0")>
                    <code class="block text-xs text-muted-foreground">{*var}</code>
                    <span class="block text-base text-foreground" style=format!("font-family: var({var})")>
                      {move || t(key)}
                    </span>
                    <span class="block text-base break-all text-foreground" style=format!("font-family: var({var})")>
                      {move || preview()}
                    </span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="space-y-2">
          <h3 class="pxl-sub-head">{move || t("design.type-scale")}</h3>
          <div class="divide-y border-y-2 border-ink">
            {TYPE_STEPS
              .iter()
              .map(|step| {
                view! {
                  <div class="flex flex-wrap items-baseline gap-3 py-2">
                    <code class="w-14 shrink-0 text-xs text-muted-foreground">{step.name}</code>
                    <code class="w-12 shrink-0 text-xs text-muted-foreground">{step.px}</code>
                    <code class="w-14 shrink-0 text-xs text-muted-foreground">
                      {step.line_value}
                    </code>
                    <span
                      class="min-w-0 flex-1 truncate text-foreground"
                      style=format!(
                        "font-size: var({}); line-height: var({})",
                        step.var, step.line
                      )
                    >
                      {move || preview()}
                    </span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <div class=crate::kit::tile_class("space-y-3 p-3")>
          <Field
            label=Signal::derive(move || t("design.type-sample-label"))
            r#for=String::from("design-preview")
          >
            <Input
              id=String::from("design-preview")
              value=sample
              on_change=Callback::new(move |v: String| sample.set(v))
              placeholder=Signal::derive(move || t("design.type-sample-placeholder"))
            />
          </Field>
          <div class="pxl-display p-3">
            <span class="block break-all text-lg" style="line-height: var(--text-lg--line-height)">
              {move || preview()}
            </span>
          </div>
        </div>

        <section class="space-y-2">
          <h3 class="pxl-sub-head">{move || t("design.type-weight")}</h3>
          <code class=crate::kit::tag_class("text-xs")>"font-weight: 400"</code>
          <p class="text-xs leading-relaxed text-muted-foreground">
            {move || t("design.type-weight-desc")}
          </p>
          <label class="flex cursor-pointer items-center gap-2 text-xs text-muted-foreground">
            <Switch
              checked=Signal::derive(move || prefs.readable_font())
              on_change=Callback::new(move |on: bool| prefs.set_readable_font(on))
              aria_label=Signal::derive(move || t("design.type-readable"))
            />
            {move || t("design.type-readable")}
          </label>
          <p class="text-xs text-muted-foreground">
            {move || t("design.type-readable-hint")}
          </p>
        </section>

        <section class="space-y-2">
          <h3 class="pxl-sub-head">{move || t("design.type-classes")}</h3>
          <div class="grid gap-2 sm:grid-cols-2">
            {CLASSES
              .iter()
              .map(|class| {
                view! {
                  <div class=crate::kit::tile_class("space-y-1 p-3")>
                    <code class="block text-xs text-muted-foreground">{*class}</code>
                    <span class=*class>{move || preview()}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>
      </div>
    </SectionCard>
  }
}
