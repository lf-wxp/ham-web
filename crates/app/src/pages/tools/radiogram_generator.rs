//! NTS Radiogram / ICS-213 报文生成器：填表 → 生成格式化报文 → 复制。

use ham_web_core::radiogram::{Hx, Ics213, NtsRadiogram, Precedence};
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::ui::{Button, Field, Input, NativeSelect, SelectOption, Size, Textarea, Variant};
use crate::util::copy_text;

#[component]
pub fn RadiogramGenerator() -> impl IntoView {
  let format = RwSignal::new("nts".to_owned());
  // NTS 字段。
  let number = RwSignal::new(String::new());
  let precedence = RwSignal::new("R".to_owned());
  let hx = RwSignal::new("none".to_owned());
  let station = RwSignal::new(String::new());
  let place = RwSignal::new(String::new());
  let time = RwSignal::new(String::new());
  let date = RwSignal::new(String::new());
  let addressee = RwSignal::new(String::new());
  let address = RwSignal::new(String::new());
  let phone = RwSignal::new(String::new());
  let body = RwSignal::new(String::new());
  let signature = RwSignal::new(String::new());
  // ICS-213 字段。
  let incident_name = RwSignal::new(String::new());
  let to = RwSignal::new(String::new());
  let from = RwSignal::new(String::new());
  let subject = RwSignal::new(String::new());
  let ics_date = RwSignal::new(String::new());
  let ics_time = RwSignal::new(String::new());
  let message = RwSignal::new(String::new());
  let approved_by = RwSignal::new(String::new());
  let reply = RwSignal::new(String::new());
  let replied_by = RwSignal::new(String::new());

  let output = Memo::new(move |_| {
    if format.get() == "ics213" {
      Ics213 {
        incident_name: incident_name.get(),
        to: to.get(),
        from: from.get(),
        subject: subject.get(),
        date: ics_date.get(),
        time: ics_time.get(),
        message: message.get(),
        approved_by: approved_by.get(),
        reply: reply.get(),
        replied_by: replied_by.get(),
      }
      .format()
    } else {
      // 先各读一次再判空：`get()` 两次之间信号若变了，会拿到「判空时非空、取值时已空」的值。
      let time_text = time.get();
      let phone_text = phone.get();
      NtsRadiogram {
        number: number.get(),
        precedence: match precedence.get().as_str() {
          "P" => Precedence::Priority,
          "W" => Precedence::Welfare,
          "E" => Precedence::Emergency,
          _ => Precedence::Routine,
        },
        hx: match hx.get().as_str() {
          "HXA" => Some(Hx::Hxa),
          "HXB" => Some(Hx::Hxb),
          "HXC" => Some(Hx::Hxc),
          "HXD" => Some(Hx::Hxd),
          "HXE" => Some(Hx::Hxe),
          "HXF" => Some(Hx::Hxf),
          "HXG" => Some(Hx::Hxg),
          _ => None,
        },
        station: station.get(),
        place: place.get(),
        time: (!time_text.is_empty()).then_some(time_text),
        date: date.get(),
        addressee: addressee.get(),
        address: address.get(),
        phone: (!phone_text.is_empty()).then_some(phone_text),
        body: body.get(),
        signature: signature.get(),
      }
      .format()
    }
  });

  let check = Memo::new(move |_| ham_web_core::radiogram::count_groups(&body.get()));
  // 必填字段清单：空表单也「能生成」的话，出来的是一份以空格开头、缺字段的错格式报文，
  // 用户复制走就等于发了一份坏电报。这里按格式给出缺哪几项，缺字段时不给复制。
  let missing = Memo::new(move |_| {
    let mut miss: Vec<Missing> = Vec::new();
    if format.get() == "ics213" {
      if to.get().trim().is_empty() {
        miss.push(Missing::To);
      }
      if from.get().trim().is_empty() {
        miss.push(Missing::From);
      }
      if subject.get().trim().is_empty() {
        miss.push(Missing::Subject);
      }
      if ics_date.get().trim().is_empty() {
        miss.push(Missing::Date);
      }
      if message.get().trim().is_empty() {
        miss.push(Missing::Message);
      }
    } else {
      if number.get().trim().is_empty() {
        miss.push(Missing::Number);
      }
      if station.get().trim().is_empty() {
        miss.push(Missing::Station);
      }
      if place.get().trim().is_empty() {
        miss.push(Missing::Place);
      }
      if date.get().trim().is_empty() {
        miss.push(Missing::Date);
      }
      if addressee.get().trim().is_empty() {
        miss.push(Missing::Addressee);
      }
      if body.get().trim().is_empty() {
        miss.push(Missing::Body);
      }
      if signature.get().trim().is_empty() {
        miss.push(Missing::Signature);
      }
    }
    miss
  });
  let copy = move |_| {
    if missing.get_untracked().is_empty() {
      // 事件回调里读值：不属于响应式上下文，用 untracked 避免多余的订阅告警。
      copy_text(&output.get_untracked());
    }
  };

  let format_options: Vec<SelectOption> = ["nts", "ics213"]
    .into_iter()
    .map(|f| {
      SelectOption::new(
        f.to_owned(),
        if f == "nts" {
          "NTS Radiogram"
        } else {
          "ICS-213"
        }
        .to_owned(),
      )
    })
    .collect();
  let precedence_options = StoredValue::new(
    ["R", "P", "W", "E"]
      .into_iter()
      .map(|p| {
        SelectOption::new(
          p.to_owned(),
          match p {
            "P" => t("tools.radiogram-precedence-priority"),
            "W" => t("tools.radiogram-precedence-welfare"),
            "E" => t("tools.radiogram-precedence-emergency"),
            _ => t("tools.radiogram-precedence-routine"),
          },
        )
      })
      .collect::<Vec<SelectOption>>(),
  );
  let hx_options = StoredValue::new(
    ["none", "HXA", "HXB", "HXC", "HXD", "HXE", "HXF", "HXG"]
      .into_iter()
      .map(|h| {
        SelectOption::new(
          h.to_owned(),
          if h == "none" {
            t("radio.none")
          } else {
            h.to_owned()
          },
        )
      })
      .collect::<Vec<SelectOption>>(),
  );

  view! {
    <div class="space-y-4">
      <Field label=Signal::derive(move || t("tools.radiogram-format")) r#for="radiogram-format" class="sm:max-w-xs">
        <NativeSelect
          id="radiogram-format"
          value=format
          on_change=Callback::new(move |v: String| format.set(v))
          options=format_options
          aria_label=Signal::derive(move || t("tools.radiogram-format"))
        />
      </Field>

      {move || {
        if format.get() == "ics213" {
          view! {
            <div class="grid gap-4 sm:grid-cols-2">
              <Field label=Signal::derive(move || t("tools.radiogram-incident-name")) r#for="rg-incident">
                <Input id="rg-incident" value=incident_name on_change=Callback::new(move |v: String| incident_name.set(v)) />
              </Field>
              <Field label=Signal::derive(move || t("tools.radiogram-to")) r#for="rg-to">
                <Input id="rg-to" value=to on_change=Callback::new(move |v: String| to.set(v)) />
              </Field>
              <Field label=Signal::derive(move || t("tools.radiogram-from")) r#for="rg-from">
                <Input id="rg-from" value=from on_change=Callback::new(move |v: String| from.set(v)) />
              </Field>
              <Field label=Signal::derive(move || t("tools.theme")) r#for="rg-subject">
                <Input id="rg-subject" value=subject on_change=Callback::new(move |v: String| subject.set(v)) />
              </Field>
              <Field label=Signal::derive(move || t("log.date")) r#for="rg-date">
                <Input id="rg-date" value=ics_date on_change=Callback::new(move |v: String| ics_date.set(v)) />
              </Field>
              <Field label=Signal::derive(move || t("log.time")) r#for="rg-time">
                <Input id="rg-time" value=ics_time on_change=Callback::new(move |v: String| ics_time.set(v)) />
              </Field>
            </div>
            <Field label=Signal::derive(move || t("tools.radiogram-message")) r#for="rg-message" class="mt-4">
              <Textarea id="rg-message" value=message on_change=Callback::new(move |v: String| message.set(v)) rows=4 />
            </Field>
            <div class="mt-4 grid gap-4 sm:grid-cols-2">
              <Field label=Signal::derive(move || t("tools.radiogram-approved-by")) r#for="rg-approved">
                <Input id="rg-approved" value=approved_by on_change=Callback::new(move |v: String| approved_by.set(v)) />
              </Field>
              <Field label=Signal::derive(move || t("tools.radiogram-replied-by")) r#for="rg-replied">
                <Input id="rg-replied" value=replied_by on_change=Callback::new(move |v: String| replied_by.set(v)) />
              </Field>
            </div>
            <Field label=Signal::derive(move || t("tools.radiogram-reply")) r#for="rg-reply" class="mt-4">
              <Textarea id="rg-reply" value=reply on_change=Callback::new(move |v: String| reply.set(v)) rows=3 />
            </Field>
          }
          .into_any()
        } else {
          view! {
            <div class="grid gap-4 sm:grid-cols-2">
              <Field label=Signal::derive(move || t("radio.reference-label")) r#for="rg-number">
                <Input id="rg-number" value=number on_change=Callback::new(move |v: String| number.set(v)) />
              </Field>
              <Field label=Signal::derive(move || t("tools.radiogram-precedence")) r#for="rg-precedence">
                <NativeSelect
                  id="rg-precedence"
                  value=precedence
                  on_change=Callback::new(move |v: String| precedence.set(v))
                  options=precedence_options.get_value()
                  aria_label=Signal::derive(move || t("tools.radiogram-precedence"))
                />
              </Field>
              <Field label=Signal::derive(move || t("tools.radiogram-hx")) r#for="rg-hx">
                <NativeSelect
                  id="rg-hx"
                  value=hx
                  on_change=Callback::new(move |v: String| hx.set(v))
                  options=hx_options.get_value()
                  aria_label=Signal::derive(move || t("tools.radiogram-hx"))
                />
              </Field>
              <Field label=Signal::derive(move || t("tools.radiogram-station")) r#for="rg-station">
                <Input id="rg-station" value=station on_change=Callback::new(move |v: String| station.set(v)) />
              </Field>
              <Field label=Signal::derive(move || t("tools.radiogram-place")) r#for="rg-place">
                <Input id="rg-place" value=place on_change=Callback::new(move |v: String| place.set(v)) />
              </Field>
              <Field label=Signal::derive(move || t("log.date")) r#for="rg-date2">
                <Input id="rg-date2" value=date on_change=Callback::new(move |v: String| date.set(v)) />
              </Field>
              <Field label=Signal::derive(move || t("log.time")) r#for="rg-time2">
                <Input id="rg-time2" value=time on_change=Callback::new(move |v: String| time.set(v)) />
              </Field>
            </div>
            <div class="mt-4 grid gap-4 sm:grid-cols-2">
              <Field label=Signal::derive(move || t("tools.radiogram-addressee")) r#for="rg-addressee">
                <Input id="rg-addressee" value=addressee on_change=Callback::new(move |v: String| addressee.set(v)) />
              </Field>
              <Field label=Signal::derive(move || t("tools.radiogram-phone")) r#for="rg-phone">
                <Input id="rg-phone" value=phone on_change=Callback::new(move |v: String| phone.set(v)) />
              </Field>
            </div>
            <Field label=Signal::derive(move || t("tools.radiogram-address")) r#for="rg-address" class="mt-4">
              <Input id="rg-address" value=address on_change=Callback::new(move |v: String| address.set(v)) />
            </Field>
            <Field label=Signal::derive(move || t("tools.radiogram-body")) r#for="rg-body" class="mt-4">
              <Textarea id="rg-body" value=body on_change=Callback::new(move |v: String| body.set(v)) rows=4 />
              <span class="text-xs text-muted-foreground">
                {move || crate::i18n::tf("tools.radiogram-check-hint", &[&check.get().to_string()])}
              </span>
            </Field>
            <Field label=Signal::derive(move || t("tools.radiogram-signature")) r#for="rg-signature" class="mt-4">
              <Input id="rg-signature" value=signature on_change=Callback::new(move |v: String| signature.set(v)) />
            </Field>
          }
          .into_any()
        }
      }}

      <div class="mt-4">
        <div class="mb-2 flex items-center justify-between">
          <span class="text-sm font-medium">{move || t("tools.radiogram-output")}</span>
          <Button
            variant=Variant::Outline
            size=Size::Sm
            on_click=copy
            disabled=Signal::derive(move || !missing.get().is_empty())
          >
            {move || t("morse.copy")}
          </Button>
        </div>
        {move || {
          let miss = missing.get();
          if miss.is_empty() {
            view! {
              <pre class="overflow-x-auto border bg-muted/40 p-4 font-mono text-sm leading-relaxed">
                {output.get()}
              </pre>
            }
              .into_any()
          } else {
            let names = miss.iter().map(|k| k.label()).collect::<Vec<_>>().join(", ");
            view! {
              <div class="border bg-muted/40 p-4 text-sm text-muted-foreground">
                {tf("tools.radiogram-missing-fields", &[&names])}
              </div>
            }
              .into_any()
          }
        }}
      </div>
    </div>
  }
}

/// 必填项缺失时，提示里列出的字段。
///
/// 用枚举而不是 `Vec<&'static str>` 存词条 key：`t()` 的实参必须是调用点的字面量，
/// `check-i18n` 才采集得到；经变量转手的 key 会被判成死条目。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Missing {
  To,
  From,
  Subject,
  Date,
  Message,
  Number,
  Station,
  Place,
  Addressee,
  Body,
  Signature,
}

impl Missing {
  fn label(self) -> String {
    match self {
      Self::To => t("tools.radiogram-to"),
      Self::From => t("tools.radiogram-from"),
      Self::Subject => t("tools.theme"),
      Self::Date => t("log.date"),
      Self::Message => t("tools.radiogram-message"),
      Self::Number => t("radio.reference-label"),
      Self::Station => t("tools.radiogram-station"),
      Self::Place => t("tools.radiogram-place"),
      Self::Addressee => t("tools.radiogram-addressee"),
      Self::Body => t("tools.radiogram-body"),
      Self::Signature => t("tools.radiogram-signature"),
    }
  }
}
