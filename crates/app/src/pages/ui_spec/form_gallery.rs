//! 表单控件示例：全部走 `crate::ui`，标签与控件 id 逐对配对，输入值实时回显。

use leptos::prelude::*;

use crate::components::common::SectionCard;
use crate::i18n::t;
use crate::ui::{
  Checkbox, Chip, ChipGroup, Field, Input, InputType, SelectOption, SimpleSelect, Slider, Switch,
  Textarea,
};

#[component]
pub fn FormGallery() -> impl IntoView {
  let text = RwSignal::new(String::new());
  let search = RwSignal::new(String::new());
  let password = RwSignal::new(String::new());
  let note = RwSignal::new(String::new());
  let band = RwSignal::new(String::new());
  let toggle = RwSignal::new(true);
  let checked = RwSignal::new(true);
  let level = RwSignal::new(60.0_f64);
  let chip = RwSignal::new(String::from("all"));
  let options = vec![
    SelectOption::new("hf", "HF"),
    SelectOption::new("vhf", "VHF"),
    SelectOption::new("uhf", "UHF"),
    SelectOption::new("shf", "SHF"),
  ];

  view! {
    <SectionCard title=Signal::derive(move || t("design.comp-forms"))>
      <div class="grid gap-4 p-4 md:grid-cols-2">
        <Field
          label=Signal::derive(move || t("design.comp-text-label"))
          r#for=String::from("design-text")
        >
          <Input
            id=String::from("design-text")
            value=text
            on_change=Callback::new(move |v: String| text.set(v))
            placeholder=Signal::derive(move || t("design.comp-placeholder-text"))
          />
        </Field>
        <Field
          label=Signal::derive(move || t("design.comp-search-label"))
          r#for=String::from("design-search")
        >
          <Input
            id=String::from("design-search")
            value=search
            on_change=Callback::new(move |v: String| search.set(v))
            kind=InputType::Search
            clearable=true
            placeholder=Signal::derive(move || t("design.comp-placeholder-search"))
          />
        </Field>
        <Field
          label=Signal::derive(move || t("design.comp-password-label"))
          r#for=String::from("design-password")
        >
          <Input
            id=String::from("design-password")
            value=password
            on_change=Callback::new(move |v: String| password.set(v))
            kind=InputType::Password
            placeholder=Signal::derive(move || t("design.comp-placeholder-text"))
          />
        </Field>
        <Field
          label=Signal::derive(move || t("design.comp-select-label"))
          r#for=String::from("design-select")
        >
          <SimpleSelect
            id=String::from("design-select")
            value=band
            on_change=Callback::new(move |v: String| band.set(v))
            options=options
            placeholder=Signal::derive(move || t("design.comp-placeholder-select"))
          />
        </Field>
        <Field
          label=Signal::derive(move || t("design.comp-textarea-label"))
          r#for=String::from("design-note")
        >
          <Textarea
            id=String::from("design-note")
            value=note
            on_change=Callback::new(move |v: String| note.set(v))
            placeholder=Signal::derive(move || t("design.comp-placeholder-textarea"))
          />
        </Field>
        <div class="space-y-3">
          <Field
            label=Signal::derive(move || t("design.comp-switch-label"))
            r#for=String::from("design-switch")
          >
            <Switch
              id=String::from("design-switch")
              checked=toggle
              on_change=Callback::new(move |v: bool| toggle.set(v))
            />
          </Field>
          <Field
            label=Signal::derive(move || t("design.comp-check-label"))
            r#for=String::from("design-check")
          >
            <Checkbox
              id=String::from("design-check")
              checked=checked
              on_change=Callback::new(move |v: bool| checked.set(v))
            />
          </Field>
          <Field
            label=Signal::derive(move || t("design.comp-slider-label"))
            r#for=String::from("design-slider")
          >
            <Slider
              id=String::from("design-slider")
              value=level
              on_change=Callback::new(move |v: f64| level.set(v))
              aria_label=Signal::derive(move || t("design.comp-slider-label"))
              aria_valuetext=Signal::derive(move || format!("{}%", level.get().round() as i64))
            />
          </Field>
        </div>
        <div class="md:col-span-2 space-y-2">
          <span class="pxl-label text-xs text-muted-foreground">
            {move || t("design.comp-chip-label")}
          </span>
          <ChipGroup
            value=chip
            on_change=Callback::new(move |v: String| chip.set(v))
            aria_label=Signal::derive(move || t("design.comp-chip-label"))
          >
            <Chip value="all">"ALL"</Chip>
            <Chip value="a">"A"</Chip>
            <Chip value="b">"B"</Chip>
            <Chip value="c">"C"</Chip>
          </ChipGroup>
        </div>
        <div class=crate::kit::tile_class("md:col-span-2 space-y-1 p-3")>
          <span class="pxl-label text-xs text-muted-foreground">
            {move || t("design.comp-echo")}
          </span>
          <code class="block text-xs break-all text-foreground">
            {move || {
              format!(
                "text={:?} search={:?} band={:?} note={:?} switch={} checkbox={} slider={} chip={:?}",
                text.get(),
                search.get(),
                band.get(),
                note.get(),
                toggle.get(),
                checked.get(),
                level.get().round() as i64,
                chip.get(),
              )
            }}
          </code>
        </div>
      </div>
    </SectionCard>
  }
}
