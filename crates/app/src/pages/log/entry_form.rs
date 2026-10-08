//! 通联记录表单：新增 / 编辑共用，含 CAT 读取、呼号提示与更多字段折叠。

use leptos::prelude::*;
use leptos::task::spawn_local;

use ham_web_core::dxcc::lookup;
use ham_web_core::logbook::{MODES, call_hint, path_to};
use ham_web_core::qsl_status::QslVia;

use crate::components::cat_control::{CatControl, CatReading};
use crate::data;
use crate::ui::{
  Button, Checkbox, DatePicker, Field, Input, NativeSelect, NumberField, Select, SelectItem,
  SelectOption, TimePicker, Variant,
};

use super::form_state::LogFormState;
use super::log_helpers::{PROP_MODES, compass};
use super::qsl_image::QslImage;
use super::{LogEntry, Logbook, StationInfo};
use crate::i18n::{t, tf, tp};

/// `/api/callsign` 返回的呼号资料（仅取需要字段）。
#[derive(serde::Deserialize, Clone)]
struct CallsignInfo {
  name: String,
  grid: String,
  qth: String,
}

#[component]
pub(super) fn EntryForm(
  form: LogFormState,
  logbook: RwSignal<Logbook>,
  station: RwSignal<StationInfo>,
  editing: RwSignal<Option<u64>>,
  on_save: Callback<()>,
) -> impl IntoView {
  let hint = Memo::new(move |_| {
    let call = form.callsign.get().trim().to_uppercase();
    if call.len() < 3 {
      return None;
    }
    let draft = LogEntry {
      freq: form.freq.get(),
      band: form.band.get(),
      ..Default::default()
    };
    let (m, skip) = (form.mode.get(), editing.get());
    let h = logbook.with(|lb| call_hint(&lb.entries, &call, &draft.band_label(), &m, skip));
    let path = station.with(|s| path_to(&s.gridsquare, &form.gridsquare.get(), &call));
    Some((h, path))
  });
  // 自动推断的分区，作为输入框占位提示。
  let auto_zones = Memo::new(move |_| lookup(form.callsign.get().trim()).map(|e| (e.cq, e.itu)));

  // 在线呼号查询：自动补全姓名 / 网格 / QTH。
  let lookup_loading = RwSignal::new(false);
  let lookup_msg = RwSignal::new(None::<String>);
  let lookup_call = move || {
    let call = form.callsign.get().trim().to_uppercase();
    if call.len() < 3 {
      crate::util::alert(&t("log.enter-a-full-callsign"));
      return;
    }
    lookup_loading.set(true);
    lookup_msg.set(None);
    // 呼号是用户输入，必须编码后再拼进查询串：含 `&` / `#` / `?` / 空格时会截断 URL
    // 或凭空多出查询参数（`encode_uri_component` 与 JS 的 `encodeURIComponent` 等价）。
    let url = format!(
      "/api/callsign?callsign={}",
      crate::util::encode_uri_component(&call)
    );
    spawn_local(async move {
      match data::fetch_external_json::<CallsignInfo>(&url).await {
        Ok(info) => {
          if form.name.get().trim().is_empty() {
            form.name.set(info.name);
          }
          if form.gridsquare.get().trim().is_empty() {
            form.gridsquare.set(info.grid);
          }
          if form.qth.get().trim().is_empty() {
            form.qth.set(info.qth);
          }
          lookup_msg.set(Some(t("log.name-grid-qth-filled")));
        }
        Err(_) => lookup_msg.set(Some(t("log.lookup-failed-or-no"))),
      }
      lookup_loading.set(false);
    });
  };

  // 标签取 `Signal<String>` 而不是现成的 `String`：静态串在切语言后不会更新，
  // 调用点统一写成 `Signal::derive(move || t("…"))`（见 `ui/control.rs` 的 `TextValue`）。
  let text_input = move |label: Signal<String>,
                         placeholder: &'static str,
                         sig: RwSignal<String>,
                         extra: &'static str| {
    let id = crate::util::unique_id("log-text");
    let label_for = id.clone();
    view! {
      <Field label=label r#for=label_for>
        <Input
          id=id
          value=sig
          on_change=Callback::new(move |v: String| sig.set(v))
          placeholder=placeholder
          class=extra
        />
      </Field>
    }
  };

  // 固定控件的 id：`Field` 的标签与控件是兄弟节点，`r#for` / `id` 必须配对才能点击标签
  // 聚焦输入框（e2e 与读屏都按这个关联定位）。
  let date_id = crate::util::unique_id("log-date");
  let time_id = crate::util::unique_id("log-time");
  let mode_id = crate::util::unique_id("log-mode");
  let callsign_id = crate::util::unique_id("log-callsign");
  let notes_id = crate::util::unique_id("log-notes");
  let qsl_via_id = crate::util::unique_id("log-qsl-via");

  view! {
    // 新增记录表单
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">
        {move || if editing.get().is_some() { t("log.edit-qso") } else { t("log.add-qso") }}
      </h2>
      <div class="px-4 pt-4">
        <CatControl on_reading=move |r: CatReading| {
          if let Some(f) = r.freq {
            form.freq.set(f);
          }
          if let Some(m) = r.mode {
            form.mode.set(m.to_owned());
          }
        } />
      </div>
      <div class="grid gap-3 p-4 sm:grid-cols-2 lg:grid-cols-3">
        <Field label=Signal::derive(move || t("log.date-utc")) r#for=date_id.clone()>
          <DatePicker
            id=date_id
            value=form.date
            on_change=Callback::new(move |v: String| form.date.set(v))
            aria_label=Signal::derive(move || t("log.date-utc"))
          />
        </Field>
        <Field label=Signal::derive(move || t("log.time-utc")) r#for=time_id.clone()>
          <TimePicker
            id=time_id
            value=form.time
            on_change=Callback::new(move |v: String| form.time.set(v))
            aria_label=Signal::derive(move || t("log.time-utc"))
          />
        </Field>
        {text_input(Signal::derive(move || t("log.frequency-mhz")), "14.074", form.freq, "")}
        <Field label=Signal::derive(move || t("log.mode")) r#for=mode_id.clone()>
          <Select
            id=mode_id
            value=Signal::derive(move || {
              let v = form.mode.get();
              (!v.is_empty()).then_some(v)
            })
            on_change=Callback::new(move |v: String| form.mode.set(v))
            placeholder=Signal::derive(move || t("log.mode"))
            trigger=move || view! { <span class="block truncate text-left">{form.mode.get()}</span> }.into_any()
            aria_label=Signal::derive(move || t("log.mode"))
          >
            {move || {
              let current = form.mode.get();
              let mut list: Vec<String> = MODES.iter().map(|m| (*m).to_owned()).collect();
              if !current.is_empty() && !list.contains(&current) {
                list.push(current);
              }
              list
                .into_iter()
                .map(|m| view! { <SelectItem value=m.clone()>{m.clone()}</SelectItem> })
                .collect_view()
            }}
          </Select>
        </Field>
        <Field label=Signal::derive(move || t("log.their-callsign")) r#for=callsign_id.clone()>
          <div class="flex gap-2">
            <Input
              id=callsign_id
              value=form.callsign
              on_change=Callback::new(move |v: String| form.callsign.set(v))
              placeholder="BG4XXX"
              class="flex-1 uppercase"
            />
            <Button
              variant=Variant::Outline
              class="shrink-0"
              on_click=Callback::new(move |_| lookup_call())
            >
              {move || if lookup_loading.get() { t("log.looking-up") } else { t("log.look-up") }}
            </Button>
          </div>
          {move || lookup_msg.get().map(|m| view! {
            <span class="text-[11px] text-muted-foreground">{m}</span>
          })}
        </Field>
        {text_input(Signal::derive(move || t("log.signal-report-sent-rst")), "59 / 599", form.rst_sent, "")}
        {move || {
          hint.get().map(|(h, path)| {
            let entity = h.entity.map_or_else(
              || t("log.unknown-dxcc-entity"),
              |e| {
                let (cq, itu) = (e.cq.to_string(), e.itu.to_string());
                tf("log.cq-itu", &[(e.name), (e.continent), &(cq).to_string(), &(itu).to_string()])
              },
            );
            view! {
              <div class="flex flex-wrap items-center gap-x-3 gap-y-1 rounded-lg bg-muted/40 px-3 py-2 text-xs sm:col-span-2 lg:col-span-3">
                <span class="text-muted-foreground">{entity}</span>
                {path.map(|p| view! {
                  <span class="tabular-nums text-muted-foreground" title=if p.approximate { t("log.no-grid-given-estimated") } else { t("log.computed-from-both-grids") }>
                    {tf(
                      "log.km-bearing",
                      &[
                        &if p.approximate { t("log.entry-2") } else { String::new() },
                        &format!("{:.0}", p.km),
                        &format!("{:.0}", p.bearing),
                        &t(compass(p.bearing)),
                      ],
                    )}
                  </span>
                })}
                {h.new_dxcc.then(|| view! {
                  <span class="rounded-full bg-emerald-500/15 px-2 py-0.5 font-medium text-emerald-700 dark:text-emerald-300">{move || t("log.new-dxcc")}</span>
                })}
                {h.new_band.then(|| view! {
                  <span class="rounded-full bg-sky-500/15 px-2 py-0.5 font-medium text-sky-700 dark:text-sky-300">{move || t("log.new-band-for-this")}</span>
                })}
                {h.dupe.then(|| view! {
                  <span class="rounded-full bg-amber-500/15 px-2 py-0.5 font-medium text-amber-700 dark:text-amber-300">{move || t("log.dupe-already-worked-on")}</span>
                })}
                <span class="text-muted-foreground">
                  {match (h.worked, h.last.clone()) {
                    (0, _) | (_, None) => t("log.first-qso-with-this"),
                    (n, Some(last)) => tp("log.worked-times-last", n, &[&n.to_string(), &(last).to_string()]),
                  }}
                </span>
              </div>
            }
          })
        }}
        {text_input(Signal::derive(move || t("log.signal-report-received-rst")), "59 / 599", form.rst_rcvd, "")}
        {text_input(Signal::derive(move || t("log.their-grid")), "OM89EW", form.gridsquare, "uppercase")}
        {text_input(Signal::derive(move || t("log.their-name")), "", form.name, "")}
        {text_input(Signal::derive(move || t("log.their-qth")), "", form.qth, "")}
        <Field
          label=Signal::derive(move || t("log.notes"))
          r#for=notes_id.clone()
          class="sm:col-span-2 lg:col-span-2"
        >
          <Input
            id=notes_id
            value=form.remark
            on_change=Callback::new(move |v: String| form.remark.set(v))
          />
        </Field>
        <Field label=Signal::derive(move || t("log.qsl-sent-via")) r#for=qsl_via_id.clone()>
          <NativeSelect
            id=qsl_via_id
            value=Signal::derive(move || form.qsl_via.get())
            on_change=Callback::new(move |v: String| form.qsl_via.set(v))
            options=vec![
              SelectOption::new(QslVia::None.key(), Signal::derive(move || t("log.not-sent"))),
              SelectOption::new(
                QslVia::Bureau.key(),
                Signal::derive(move || t("log.qsl-via-bureau")),
              ),
              SelectOption::new(
                QslVia::Direct.key(),
                Signal::derive(move || t("log.qsl-via-direct")),
              ),
            ]
            aria_label=Signal::derive(move || t("log.qsl-sent-via"))
            class="w-full"
          />
        </Field>
        <label class="flex cursor-pointer items-center gap-2">
          <Checkbox
            checked=form.qsl_rcvd
            on_change=Callback::new(move |v: bool| form.qsl_rcvd.set(v))
            aria_label=Signal::derive(move || t("log.qsl-received"))
          />
          <span class="text-xs text-muted-foreground">{move || t("log.qsl-received")}</span>
        </label>
        <label class="flex cursor-pointer items-center gap-2">
          <Checkbox
            checked=form.lotw_sent
            on_change=Callback::new(move |v: bool| form.lotw_sent.set(v))
            aria_label=Signal::derive(move || t("log.lotw-uploaded"))
          />
          <span class="text-xs text-muted-foreground">{move || t("log.lotw-uploaded")}</span>
        </label>
        <label class="flex cursor-pointer items-center gap-2">
          <Checkbox
            checked=form.lotw_rcvd
            on_change=Callback::new(move |v: bool| form.lotw_rcvd.set(v))
            aria_label=Signal::derive(move || t("log.lotw-confirmed"))
          />
          <span class="text-xs text-muted-foreground">{move || t("log.lotw-confirmed")}</span>
        </label>
        <label class="flex cursor-pointer items-center gap-2">
          <Checkbox
            checked=form.eqsl_sent
            on_change=Callback::new(move |v: bool| form.eqsl_sent.set(v))
            aria_label=Signal::derive(move || t("log.eqsl-sent"))
          />
          <span class="text-xs text-muted-foreground">{move || t("log.eqsl-sent")}</span>
        </label>
        <label class="flex cursor-pointer items-center gap-2">
          <Checkbox
            checked=form.eqsl_rcvd
            on_change=Callback::new(move |v: bool| form.eqsl_rcvd.set(v))
            aria_label=Signal::derive(move || t("log.eqsl-confirmed"))
          />
          <span class="text-xs text-muted-foreground">{move || t("log.eqsl-confirmed")}</span>
        </label>
        {move || match editing.get() {
          Some(id) => view! { <QslImage entry_id=id /> }.into_any(),
          // 新记录还没有 id，影像按 id 关联，所以先提示「保存后再说」。
          None => view! {
            <div class="flex flex-col gap-1.5 sm:col-span-2 lg:col-span-3">
              <span class="text-xs text-muted-foreground">{move || t("log.qsl-card-image")}</span>
              <span class="text-xs text-muted-foreground">
                {move || t("log.save-this-qso-then")}
              </span>
            </div>
          }.into_any(),
        }}
        <button
          type="button"
          class="text-left text-xs text-primary underline-offset-4 hover:underline sm:col-span-2 lg:col-span-3"
          on:click=move |_| form.show_more.update(|v| *v = !*v)
        >
          {move || if form.show_more.get() { t("log.hide-extra-fields") } else { t("log.more-fields-power-end") }}
        </button>
        {move || {
          form.show_more.get().then(|| {
            // 折叠区的 id 在这里现生成：`then` 的闭包是 `FnOnce`，可以自由移出，
            // 而外层 `move ||` 必须保持 `Fn`（可重复重渲染），不能捕获这些 id。
            let time_off_id = crate::util::unique_id("log-time-off");
            let prop_mode_id = crate::util::unique_id("log-prop-mode");
            let cqz_id = crate::util::unique_id("log-cqz");
            let ituz_id = crate::util::unique_id("log-ituz");
            view! {
              {text_input(Signal::derive(move || t("log.power-w-tx-pwr")), "100", form.tx_pwr, "")}
              <Field label=Signal::derive(move || t("log.end-time-utc")) r#for=time_off_id.clone()>
                <TimePicker
                  id=time_off_id.clone()
                  value=form.time_off
                  on_change=Callback::new(move |v: String| form.time_off.set(v))
                  aria_label=Signal::derive(move || t("log.end-time-utc"))
                />
              </Field>
              <Field
                label=Signal::derive(move || t("log.propagation-mode-prop-mode"))
                r#for=prop_mode_id.clone()
              >
                {{
                  let prop_mode_options: Vec<SelectOption> = PROP_MODES
                    .iter()
                    .filter(|(v, _)| !v.is_empty())
                    .map(|&(v, l)| SelectOption::new(v, Signal::derive(move || t(l))))
                    .collect();
                  view! {
                    <NativeSelect
                      id=prop_mode_id.clone()
                      value=form.prop_mode
                      on_change=Callback::new(move |v: String| form.prop_mode.set(v))
                      options=prop_mode_options
                      placeholder=PROP_MODES[0].1
                      aria_label=Signal::derive(move || t("log.propagation-mode-prop-mode"))
                    />
                  }
                }}
              </Field>
              {text_input(Signal::derive(move || t("log.satellite-sat-name")), "SO-50 / RS-44", form.sat_name, "uppercase")}
              {text_input(Signal::derive(move || t("log.sota-reference")), "BY/BJ-001", form.sota_ref, "uppercase")}
              {text_input(Signal::derive(move || t("log.pota-reference")), "CN-0001", form.pota_ref, "uppercase")}
              <Field label=Signal::derive(move || t("log.cq-zone-blank-auto")) r#for=cqz_id.clone()>
                <NumberField
                  id=cqz_id.clone()
                  step=1.0
                  min=1.0
                  max=40.0
                  value=form.cqz
                  on_change=Callback::new(move |v: String| form.cqz.set(v))
                  placeholder=Signal::derive(move || {
                    auto_zones.get().map(|(cq, _)| cq.to_string()).unwrap_or_default()
                  })
                  controls=false
                />
              </Field>
              <Field label=Signal::derive(move || t("log.itu-zone-blank-auto")) r#for=ituz_id.clone()>
                <NumberField
                  id=ituz_id.clone()
                  step=1.0
                  min=1.0
                  max=90.0
                  value=form.ituz
                  on_change=Callback::new(move |v: String| form.ituz.set(v))
                  placeholder=Signal::derive(move || {
                    auto_zones.get().map(|(_, itu)| itu.to_string()).unwrap_or_default()
                  })
                  controls=false
                />
              </Field>
            }
          })
        }}
      </div>
      <div class="flex items-center gap-2 px-4 pb-4">
        <Button variant=Variant::Default on_click=Callback::new(move |_| on_save.run(()))>
          {move || if editing.get().is_some() { t("log.save-changes") } else { t("log.add-entry") }}
        </Button>
        {move || {
          editing.get().is_some().then(|| {
            view! {
              <Button
                variant=Variant::Ghost
                on_click=Callback::new(move |_| {
                  form.reset();
                  editing.set(None);
                })
              >
                {move || t("exam.cancel")}
              </Button>
            }
          })
        }}
      </div>
    </section>
  }
}
