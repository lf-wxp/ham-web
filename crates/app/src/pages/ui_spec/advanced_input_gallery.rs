//! 进阶表单控件：数字 / 日期 / 时间 / 单选 / 富文本下拉 / 双态标签 / 标签关联 / 尺寸档。

use leptos::prelude::*;

use crate::components::common::SectionCard;
use crate::i18n::t;
use crate::icons::{Icon, IconKind};
use crate::ui::{
  Checkbox, ChipToggle, ControlSize, DatePicker, Field, Input, Label, NumberField, NumberKind,
  RadioGroup, RadioGroupItem, Select, SelectGroup, SelectItem, Separator, TimePicker, ValidateOn,
};

const MODES: &[(&str, &str)] = &[("cw", "CW"), ("ssb", "SSB"), ("fm", "FM")];

#[component]
pub fn AdvancedInputGallery() -> impl IntoView {
  let integer = RwSignal::new(String::from("14"));
  let decimal = RwSignal::new(String::from("3.5"));
  let date = RwSignal::new(String::new());
  let time = RwSignal::new(String::new());
  let mode = RwSignal::new(String::from("cw"));
  let version = RwSignal::new(None::<String>);
  let narrow = RwSignal::new(false);
  let marked = RwSignal::new(true);
  let agreed = RwSignal::new(true);
  let sized = RwSignal::new(String::new());
  let late = RwSignal::new(String::new());
  let late_invalid = Signal::derive(move || late.get().chars().count() < 2);

  let trigger = move || {
    let text = version
      .get()
      .unwrap_or_else(|| t("design.comp-placeholder-select"));
    view! {
      <span class="flex items-center gap-2">
        <Icon kind=IconKind::Radio />
        {text}
      </span>
    }
    .into_any()
  };

  view! {
    <SectionCard title=Signal::derive(move || t("design.comp-advanced-input"))>
      <div class="space-y-4 p-4">
        <p class="text-xs leading-relaxed text-muted-foreground">
          {move || t("design.comp-advanced-input-hint")}
        </p>
        <div class="grid gap-4 md:grid-cols-2">
          <Field
            label=Signal::derive(move || t("design.comp-number"))
            r#for=String::from("design-number")
          >
            <NumberField
              id=String::from("design-number")
              value=integer
              on_change=Callback::new(move |v: String| integer.set(v))
              min=0.0
              max=30.0
              step=1.0
            />
          </Field>
          <Field
            label=Signal::derive(move || t("design.comp-decimal"))
            r#for=String::from("design-decimal")
          >
            <NumberField
              id=String::from("design-decimal")
              value=decimal
              on_change=Callback::new(move |v: String| decimal.set(v))
              kind=NumberKind::Decimal
              step=0.5
            />
          </Field>
          <Field
            label=Signal::derive(move || t("design.comp-date"))
            r#for=String::from("design-date")
          >
            <DatePicker
              id=String::from("design-date")
              value=date
              on_change=Callback::new(move |v: String| date.set(v))
              aria_label=Signal::derive(move || t("design.comp-date"))
            />
          </Field>
          <Field
            label=Signal::derive(move || t("design.comp-time"))
            r#for=String::from("design-time")
          >
            <TimePicker
              id=String::from("design-time")
              value=time
              on_change=Callback::new(move |v: String| time.set(v))
              aria_label=Signal::derive(move || t("design.comp-time"))
            />
          </Field>
          <div class="space-y-2">
            <span class=super::spec_data::pixel_only("pxl-label", "", "text-xs text-muted-foreground")>
              {move || t("design.comp-radio-mode")}
            </span>
            <RadioGroup
              value=mode
              on_change=Callback::new(move |v: String| mode.set(v))
              aria_label=Signal::derive(move || t("design.comp-radio-mode"))
              class="flex flex-wrap gap-3"
            >
              {MODES
                .iter()
                .map(|(slug, name)| {
                  view! {
                    <label class="flex cursor-pointer items-center gap-2 text-xs text-muted-foreground">
                      <RadioGroupItem value=*slug />
                      {*name}
                    </label>
                  }
                })
                .collect_view()}
            </RadioGroup>
          </div>
          <Field
            label=Signal::derive(move || t("design.comp-select-rich"))
            r#for=String::from("design-select-rich")
          >
            <Select
              id=String::from("design-select-rich")
              value=version
              on_change=Callback::new(move |v: String| version.set(Some(v)))
              placeholder=Signal::derive(move || t("design.comp-placeholder-select"))
              trigger=trigger
              aria_label=Signal::derive(move || t("design.comp-select-rich"))
            >
              <SelectGroup label=t("design.comp-select-group")>
                <SelectItem value="v2">
                  <span class="flex items-center gap-2">
                    <Icon kind=IconKind::CheckCircle />
                    "V2"
                  </span>
                </SelectItem>
                <SelectItem value="v3">
                  <span class="flex items-center gap-2">
                    <Icon kind=IconKind::Sparkles />
                    "V3"
                  </span>
                </SelectItem>
              </SelectGroup>
            </Select>
          </Field>
        </div>

        <Separator />

        <div class="flex flex-wrap items-center gap-3">
          <ChipToggle
            active=narrow
            on_change=Callback::new(move |v: bool| narrow.set(v))
            aria_label=Signal::derive(move || t("design.comp-toggle"))
          >
            {move || t("design.comp-toggle")}
          </ChipToggle>
          <ChipToggle
            active=marked
            on_change=Callback::new(move |v: bool| marked.set(v))
            aria_label=Signal::derive(move || t("design.comp-toggle-marked"))
          >
            {move || t("design.comp-toggle-marked")}
          </ChipToggle>
          <span class="flex items-center gap-2 text-xs text-muted-foreground">
            <Checkbox
              id=String::from("design-agree")
              checked=agreed
              on_change=Callback::new(move |v: bool| agreed.set(v))
            />
            <Label r#for=String::from("design-agree")>
              {move || t("design.comp-label")}
            </Label>
          </span>
        </div>

        <div class="grid gap-3 sm:grid-cols-3">
          <Field
            label=Signal::derive(move || t("design.comp-size-sm"))
            r#for=String::from("design-size-sm")
          >
            <Input
              id=String::from("design-size-sm")
              value=sized
              on_change=Callback::new(move |v: String| sized.set(v))
              size=ControlSize::Sm
              aria_label=Signal::derive(move || t("design.comp-size-sm"))
            />
          </Field>
          <Field
            label=Signal::derive(move || t("design.comp-size-default"))
            r#for=String::from("design-size-default")
          >
            <Input
              id=String::from("design-size-default")
              value=sized
              on_change=Callback::new(move |v: String| sized.set(v))
              size=ControlSize::Default
              aria_label=Signal::derive(move || t("design.comp-size-default"))
            />
          </Field>
          <Field
            label=Signal::derive(move || t("design.comp-size-lg"))
            r#for=String::from("design-size-lg")
          >
            <Input
              id=String::from("design-size-lg")
              value=sized
              on_change=Callback::new(move |v: String| sized.set(v))
              size=ControlSize::Lg
              aria_label=Signal::derive(move || t("design.comp-size-lg"))
            />
          </Field>
        </div>

        <Field
          label=Signal::derive(move || t("design.comp-validate-late"))
          r#for=String::from("design-late")
          validate_on=ValidateOn::Blur
          invalid=late_invalid
        >
          <Input
            id=String::from("design-late")
            value=late
            on_change=Callback::new(move |v: String| late.set(v))
            invalid=late_invalid
          />
        </Field>

        <div class=crate::kit::tile_class("p-3")>
          <code class="block text-xs break-all text-foreground">
            {move || {
              format!(
                "number={:?} decimal={:?} date={:?} time={:?} mode={:?} version={:?} narrow={} marked={} agreed={}",
                integer.get(),
                decimal.get(),
                date.get(),
                time.get(),
                mode.get(),
                version.get(),
                narrow.get(),
                marked.get(),
                agreed.get(),
              )
            }}
          </code>
        </div>
      </div>
    </SectionCard>
  }
}
