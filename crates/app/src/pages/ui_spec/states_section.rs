//! 状态示例：一个「状态试验台」把整组控件切到同一档状态，再看 ARIA 对照表。

use leptos::prelude::*;

use crate::components::common::SectionCard;
use crate::i18n::t;
use crate::ui::{Button, Checkbox, Chip, ChipGroup, Field, Input, Switch, Variant};

use super::spec_data::STATES;

const ARIA_ROWS: &[(&str, &str)] = &[
  ("hover", ":hover:not(:disabled)"),
  ("focus", ":focus-visible"),
  ("active", ":active"),
  ("disabled", ":disabled + aria-disabled"),
  ("loading", "aria-busy='true'"),
  ("error", "aria-invalid='true'"),
  ("readonly", "readonly"),
  ("normal", "aria-pressed / aria-checked"),
];

#[component]
pub fn StatesSection() -> impl IntoView {
  let state = RwSignal::new(String::from("normal"));
  let disabled = Signal::derive(move || state.get() == "disabled");
  let loading = Signal::derive(move || state.get() == "loading");
  let invalid = Signal::derive(move || state.get() == "error");
  let readonly = Signal::derive(move || state.get() == "readonly");
  let text = RwSignal::new(String::from("73"));
  let toggle = RwSignal::new(true);
  let checked = RwSignal::new(true);
  let error = Signal::derive(move || {
    if state.get() == "error" {
      t("design.states-error-hint")
    } else {
      String::new()
    }
  });

  view! {
    <SectionCard title=Signal::derive(move || t("design.states-title"))>
      <div class="space-y-4 p-4">
        <p class="text-xs leading-relaxed text-muted-foreground">
          {move || t("design.states-hint")}
        </p>
        <div class=crate::kit::tile_class("space-y-2 p-3")>
          <span class="pxl-label text-xs text-muted-foreground">
            {move || t("design.states-lab")}
          </span>
          <ChipGroup
            value=state
            on_change=Callback::new(move |v: String| state.set(v))
            aria_label=Signal::derive(move || t("design.states-lab"))
          >
            {STATES
              .iter()
              .map(|item| {
                let key = item.key;
                view! { <Chip value=item.slug>{move || t(key)}</Chip> }
              })
              .collect_view()}
          </ChipGroup>
          <p class="text-xs text-muted-foreground">
            {move || t("design.states-lab-hint")}
          </p>
        </div>

        <div class=crate::kit::tile_class("flex flex-wrap items-center gap-3 p-3")>
          <Button
            disabled=disabled
            loading=loading
            on_click=Callback::new(move |_| state.set(String::from("normal")))
          >
            {move || t("design.states-normal")}
          </Button>
          <Button variant=Variant::Outline disabled=disabled loading=loading>
            {move || t("design.states-active")}
          </Button>
          <Button variant=Variant::Destructive disabled=disabled loading=loading>
            {move || t("design.states-error")}
          </Button>
        </div>

        <div class=crate::kit::tile_class("space-y-3 p-3")>
          <Field
            label=Signal::derive(move || t("design.comp-text-label"))
            r#for=String::from("design-state-input")
            error=error
            invalid=invalid
          >
            <Input
              id=String::from("design-state-input")
              value=text
              on_change=Callback::new(move |v: String| text.set(v))
              disabled=disabled
              readonly=readonly
              invalid=invalid
            />
          </Field>
          <div class="flex flex-wrap items-center gap-4">
            <label class="flex cursor-pointer items-center gap-2 text-xs text-muted-foreground">
              <Switch
                checked=toggle
                on_change=Callback::new(move |v: bool| toggle.set(v))
                disabled=disabled
                invalid=invalid
                aria_label=Signal::derive(move || t("design.comp-switch-label"))
              />
              {move || t("design.comp-switch-label")}
            </label>
            <label class="flex cursor-pointer items-center gap-2 text-xs text-muted-foreground">
              <Checkbox
                checked=checked
                on_change=Callback::new(move |v: bool| checked.set(v))
                disabled=disabled
                invalid=invalid
                aria_label=Signal::derive(move || t("design.comp-check-label"))
              />
              {move || t("design.comp-check-label")}
            </label>
          </div>
          {move || {
            // 错误说明已由 `Field` 的 `error` 渲染，这里只补悬停 / 焦点提示，避免同一句出现两次。
            let current = state.get();
            (current == "hover" || current == "focus").then(|| {
              view! {
                <p class="text-xs leading-relaxed text-muted-foreground">
                  {t("design.states-hover-hint")}
                </p>
              }
            })
          }}
        </div>

        <div class="divide-y border-y-2 border-ink">
          {ARIA_ROWS
            .iter()
            .map(|(slug, attribute)| {
              view! {
                <div
                  class=move || {
                    if state.get() == *slug {
                      "flex flex-wrap items-center justify-between gap-2 bg-accent/40 px-3 py-2"
                    } else {
                      "flex flex-wrap items-center justify-between gap-2 px-3 py-2"
                    }
                  }
                >
                  <code class="text-xs text-foreground">{*slug}</code>
                  <code class="text-xs text-muted-foreground">{*attribute}</code>
                </div>
              }
            })
            .collect_view()}
        </div>
        <p class="text-xs leading-relaxed text-muted-foreground">
          {move || t("design.states-aria-hint")}
        </p>
      </div>
    </SectionCard>
  }
}
