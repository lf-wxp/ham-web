//! 通联记录表单：新增 / 编辑共用，含 CAT 读取、呼号提示与更多字段折叠。

use leptos::prelude::*;
use leptos::task::spawn_local;

use ham_web_core::dxcc::lookup;
use ham_web_core::logbook::{MODES, call_hint, path_to};

use crate::components::cat_control::{CatControl, CatReading};
use crate::data;
use crate::ui::{Size, Variant, button_class, input_class};

use super::form_state::LogFormState;
use super::log_helpers::{PROP_MODES, compass};
use super::{LogEntry, Logbook, StationInfo};
use crate::i18n::{t, tf};

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
      crate::util::alert(&t("请输入完整呼号后再查询"));
      return;
    }
    lookup_loading.set(true);
    lookup_msg.set(None);
    let url = format!("/api/callsign?callsign={call}");
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
          lookup_msg.set(Some(t("已自动填入姓名 / 网格 / QTH")));
        }
        Err(_) => lookup_msg.set(Some(t("查询失败或暂无该呼号资料"))),
      }
      lookup_loading.set(false);
    });
  };

  let text_input =
    move |label: String, placeholder: &'static str, sig: RwSignal<String>, extra: &'static str| {
      view! {
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{label}</span>
          <input
            type="text"
            placeholder=placeholder
            prop:value=move || sig.get()
            on:input=move |e| sig.set(event_target_value(&e))
            class=input_class(extra)
          />
        </label>
      }
    };

  view! {
    // 新增记录表单
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">
        {move || if editing.get().is_some() { t("编辑通联记录") } else { t("添加通联记录") }}
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
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || t("日期（UTC）")}</span>
          <input
            type="date"
            prop:value=move || form.date.get()
            on:input=move |e| form.date.set(event_target_value(&e))
            class=input_class("")
          />
        </label>
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || t("时间（UTC）")}</span>
          <input
            type="time"
            prop:value=move || form.time.get()
            on:input=move |e| form.time.set(event_target_value(&e))
            class=input_class("")
          />
        </label>
        {text_input(t("频率（MHz）"), "14.074", form.freq, "")}
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || t("模式")}</span>
          <select
            prop:value=move || form.mode.get()
            on:change=move |e| form.mode.set(event_target_value(&e))
            class=input_class("")
          >
            {move || {
              let current = form.mode.get();
              let mut list: Vec<String> = MODES.iter().map(|m| (*m).to_owned()).collect();
              if !current.is_empty() && !list.contains(&current) {
                list.push(current);
              }
              list
                .into_iter()
                .map(|m| view! { <option value=m.clone()>{m.clone()}</option> })
                .collect_view()
            }}
          </select>
        </label>
        <div class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || t("对方呼号 *")}</span>
          <div class="flex gap-2">
            <input
              type="text"
              placeholder="BG4XXX"
              prop:value=move || form.callsign.get()
              on:input=move |e| form.callsign.set(event_target_value(&e))
              class=input_class("flex-1 uppercase")
            />
            <button
              type="button"
              class=button_class(Variant::Outline, Size::Default, "shrink-0")
              on:click=move |_| lookup_call()
            >
              {move || if lookup_loading.get() { t("查询中…") } else { t("查询") }}
            </button>
          </div>
          {move || lookup_msg.get().map(|m| view! {
            <span class="text-[11px] text-muted-foreground">{m}</span>
          })}
        </div>
        {text_input(t("信号报告（发 RST_SENT）"), "59 / 599", form.rst_sent, "")}
        {move || {
          hint.get().map(|(h, path)| {
            let entity = h.entity.map_or_else(
              || t("未识别 DXCC 实体"),
              |e| {
                let (cq, itu) = (e.cq.to_string(), e.itu.to_string());
                tf("{}（{} · CQ {} · ITU {}）", &[(e.name), (e.continent), &(cq).to_string(), &(itu).to_string()])
              },
            );
            view! {
              <div class="flex flex-wrap items-center gap-x-3 gap-y-1 rounded-lg bg-muted/40 px-3 py-2 text-xs sm:col-span-2 lg:col-span-3">
                <span class="text-muted-foreground">{entity}</span>
                {path.map(|p| view! {
                  <span class="tabular-nums text-muted-foreground" title=if p.approximate { t("未填对方网格，按 DXCC 实体中心估算") } else { t("按双方网格计算") }>
                    {tf(
                      "{}{} km · 方位 {}°（{}）",
                      &[
                        &if p.approximate { t("约 ") } else { String::new() },
                        &format!("{:.0}", p.km),
                        &format!("{:.0}", p.bearing),
                        &t(compass(p.bearing)),
                      ],
                    )}
                  </span>
                })}
                {h.new_dxcc.then(|| view! {
                  <span class="rounded-full bg-emerald-500/15 px-2 py-0.5 font-medium text-emerald-700 dark:text-emerald-300">{move || t("新 DXCC！")}</span>
                })}
                {h.new_band.then(|| view! {
                  <span class="rounded-full bg-sky-500/15 px-2 py-0.5 font-medium text-sky-700 dark:text-sky-300">{move || t("该实体新波段")}</span>
                })}
                {h.dupe.then(|| view! {
                  <span class="rounded-full bg-amber-500/15 px-2 py-0.5 font-medium text-amber-700 dark:text-amber-300">{move || t("重复：同波段同模式已联过")}</span>
                })}
                <span class="text-muted-foreground">
                  {match (h.worked, h.last.clone()) {
                    (0, _) | (_, None) => t("首次通联该呼号"),
                    (n, Some(last)) => tf("已通联 {} 次，最近 {}", &[&n.to_string(), &(last).to_string()]),
                  }}
                </span>
              </div>
            }
          })
        }}
        {text_input(t("信号报告（收 RST_RCVD）"), "59 / 599", form.rst_rcvd, "")}
        {text_input(t("对方网格"), "OM89EW", form.gridsquare, "uppercase")}
        {text_input(t("对方姓名"), "", form.name, "")}
        {text_input(t("对方 QTH"), "", form.qth, "")}
        <label class="flex flex-col gap-1.5 text-sm sm:col-span-2 lg:col-span-2">
          <span class="text-xs text-muted-foreground">{move || t("备注")}</span>
          <input
            type="text"
            prop:value=move || form.remark.get()
            on:input=move |e| form.remark.set(event_target_value(&e))
            class=input_class("")
          />
        </label>
        <label class="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            prop:checked=move || form.qsl_sent.get()
            on:change=move |e| form.qsl_sent.set(event_target_checked(&e))
            class="size-4 accent-primary"
          />
          <span class="text-xs text-muted-foreground">{move || t("QSL 已寄出")}</span>
        </label>
        <label class="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            prop:checked=move || form.qsl_rcvd.get()
            on:change=move |e| form.qsl_rcvd.set(event_target_checked(&e))
            class="size-4 accent-primary"
          />
          <span class="text-xs text-muted-foreground">{move || t("QSL 已收到")}</span>
        </label>
        <label class="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            prop:checked=move || form.lotw_sent.get()
            on:change=move |e| form.lotw_sent.set(event_target_checked(&e))
            class="size-4 accent-primary"
          />
          <span class="text-xs text-muted-foreground">{move || t("LoTW 已上传")}</span>
        </label>
        <label class="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            prop:checked=move || form.lotw_rcvd.get()
            on:change=move |e| form.lotw_rcvd.set(event_target_checked(&e))
            class="size-4 accent-primary"
          />
          <span class="text-xs text-muted-foreground">{move || t("LoTW 已确认")}</span>
        </label>
        <label class="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            prop:checked=move || form.eqsl_sent.get()
            on:change=move |e| form.eqsl_sent.set(event_target_checked(&e))
            class="size-4 accent-primary"
          />
          <span class="text-xs text-muted-foreground">{move || t("eQSL 已寄出")}</span>
        </label>
        <label class="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            prop:checked=move || form.eqsl_rcvd.get()
            on:change=move |e| form.eqsl_rcvd.set(event_target_checked(&e))
            class="size-4 accent-primary"
          />
          <span class="text-xs text-muted-foreground">{move || t("eQSL 已确认")}</span>
        </label>
        <button
          type="button"
          class="text-left text-xs text-primary underline-offset-4 hover:underline sm:col-span-2 lg:col-span-3"
          on:click=move |_| form.show_more.update(|v| *v = !*v)
        >
          {move || if form.show_more.get() { t("收起更多字段") } else { t("更多字段：功率、结束时间、卫星 / 传播方式、SOTA / POTA、CQ / ITU 分区") }}
        </button>
        {move || {
          form.show_more.get().then(|| {
            view! {
              {text_input(t("功率（W，TX_PWR）"), "100", form.tx_pwr, "")}
              <label class="flex flex-col gap-1.5 text-sm">
                <span class="text-xs text-muted-foreground">{move || t("结束时间（UTC）")}</span>
                <input
                  type="time"
                  prop:value=move || form.time_off.get()
                  on:input=move |e| form.time_off.set(event_target_value(&e))
                  class=input_class("")
                />
              </label>
              <label class="flex flex-col gap-1.5 text-sm">
                <span class="text-xs text-muted-foreground">{move || t("传播方式（PROP_MODE）")}</span>
                <select
                  prop:value=move || form.prop_mode.get()
                  on:change=move |e| form.prop_mode.set(event_target_value(&e))
                  class=input_class("")
                >
                  {PROP_MODES
                    .iter()
                    .map(|&(v, l)| view! { <option value=v>{move || t(l)}</option> })
                    .collect_view()}
                </select>
              </label>
              {text_input(t("卫星（SAT_NAME）"), "SO-50 / RS-44", form.sat_name, "uppercase")}
              {text_input(t("SOTA 编号"), "BY/BJ-001", form.sota_ref, "uppercase")}
              {text_input(t("POTA 编号"), "CN-0001", form.pota_ref, "uppercase")}
              <label class="flex flex-col gap-1.5 text-sm">
                <span class="text-xs text-muted-foreground">{move || t("CQ 分区（留空自动）")}</span>
                <input
                  type="number"
                  min="1"
                  max="40"
                  placeholder=move || auto_zones.get().map(|(cq, _)| cq.to_string()).unwrap_or_default()
                  prop:value=move || form.cqz.get()
                  on:input=move |e| form.cqz.set(event_target_value(&e))
                  class=input_class("")
                />
              </label>
              <label class="flex flex-col gap-1.5 text-sm">
                <span class="text-xs text-muted-foreground">{move || t("ITU 分区（留空自动）")}</span>
                <input
                  type="number"
                  min="1"
                  max="90"
                  placeholder=move || auto_zones.get().map(|(_, itu)| itu.to_string()).unwrap_or_default()
                  prop:value=move || form.ituz.get()
                  on:input=move |e| form.ituz.set(event_target_value(&e))
                  class=input_class("")
                />
              </label>
            }
          })
        }}
      </div>
      <div class="flex items-center gap-2 px-4 pb-4">
        <button type="button" class=button_class(Variant::Default, Size::Default, "") on:click=move |_| on_save.run(())>
          {move || if editing.get().is_some() { t("保存修改") } else { t("添加记录") }}
        </button>
        {move || {
          editing.get().is_some().then(|| {
            view! {
              <button
                type="button"
                class=button_class(Variant::Ghost, Size::Default, "")
                on:click=move |_| {
                  form.reset();
                  editing.set(None);
                }
              >
                {move || t("取消")}
              </button>
            }
          })
        }}
      </div>
    </section>
  }
}
