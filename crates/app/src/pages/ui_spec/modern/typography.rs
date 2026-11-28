//! 排版规范：7 阶字号 + 4 档字族 + 5 个排版类；预览文本 + 现代风字族分段选择。
//!
//! 这里不接 `readable_font`：那是像素风的偏好开关，现代风设置里没有这一项。
//! 对齐像素风「一个预览输入 + 一个切换控件」的项目结构，切换对象换成 `--md-font*` 三档。

use leptos::prelude::*;

use crate::components::common::SectionCard;
use crate::i18n::t;
use crate::ui::{Chip, ChipGroup, Field, Input};

use super::data::{MD_FONT_FAMILIES, MD_FONTS, MD_TYPE_CLASSES, MD_TYPE_STEPS};

#[component]
pub fn ModernTypography() -> impl IntoView {
  let sample = RwSignal::new(String::new());
  let family = RwSignal::new(String::from("--md-font"));
  let preview = move || {
    let typed = sample.get();
    if typed.is_empty() {
      t("design.md-sample")
    } else {
      typed
    }
  };

  view! {
    <SectionCard title=Signal::derive(move || t("design.type-title"))>
      <div class="space-y-4 p-4">
        <p class="md-muted md-sm">{move || t("design.md-typography-hint")}</p>
        <Field
          label=Signal::derive(move || t("design.type-sample"))
          r#for=String::from("md-design-preview")
        >
          <Input
            id=String::from("md-design-preview")
            value=sample
            on_change=Callback::new(move |v: String| sample.set(v))
            placeholder=Signal::derive(move || t("design.md-sample"))
          />
        </Field>

        <div class="space-y-2">
          <span class="md-muted md-sm">{move || t("design.md-type-preview-family")}</span>
          <div data-spec-list="font-families" class="flex flex-wrap items-center gap-2">
            <ChipGroup
              value=family
              on_change=Callback::new(move |v: String| family.set(v))
              aria_label=Signal::derive(move || t("design.md-type-preview-family"))
            >
              {MD_FONT_FAMILIES
                .iter()
                .map(|(value, name)| view! { <Chip value=*value>{*name}</Chip> })
                .collect_view()}
            </ChipGroup>
          </div>
          <div
            class=crate::kit::tile_class("min-w-0 break-words p-3")
            style=move || format!("font-family:var({})", family.get())
          >
            {move || preview()}
          </div>
        </div>

        <section data-spec-list="type-steps" class="space-y-2">
          <h3 class="md-section-title">{move || t("design.type-scale")}</h3>
          <For each=move || MD_TYPE_STEPS key=|step| step.name let:step>
            <div class=crate::kit::tile_class("flex flex-wrap items-baseline justify-between gap-2 px-3 py-2")>
              <code class="md-muted md-sm">{step.name}</code>
              <code class="md-muted md-sm">{step.var}</code>
              <code class="md-muted md-sm">{step.line}</code>
              <span
                class="min-w-0 flex-1"
                style=format!("font-size:var({});line-height:{}", step.var, step.line)
              >
                {move || preview()}
              </span>
            </div>
          </For>
        </section>

        <section data-spec-list="type-families" class="space-y-2">
          <h3 class="md-section-title">{move || t("design.md-type-families")}</h3>
          <For each=move || MD_FONTS key=|name| *name let:name>
            <div class=crate::kit::tile_class("flex flex-wrap items-baseline justify-between gap-2 px-3 py-2")>
              <code class="md-muted md-sm">{*name}</code>
              <span
                class=move || if name.starts_with("--") { String::new() } else { String::from(*name) }
                style=move || {
                  if name.starts_with("--") {
                    format!("font-family:var({name})")
                  } else {
                    String::new()
                  }
                }
              >
                {move || preview()}
              </span>
            </div>
          </For>
        </section>

        <section data-spec-list="type-classes" class="space-y-2">
          <h3 class="md-section-title">{move || t("design.type-classes")}</h3>
          <div data-spec-list="type-class-items" class="grid gap-2 sm:grid-cols-2">
            <For each=move || MD_TYPE_CLASSES key=|class| *class let:class>
              <div class=crate::kit::tile_class("space-y-1 p-3")>
                <code class="md-muted md-sm">{*class}</code>
                <span class=*class>{move || preview()}</span>
              </div>
            </For>
          </div>
        </section>
      </div>
    </SectionCard>
  }
}
