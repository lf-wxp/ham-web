//! 本台信息面板：管理**台站档案**（固定台 / 车载 / 野外 / POTA），并选定当前台站。
//!
//! 字段写入 ADIF 的 `STATION_CALLSIGN` / `MY_*`，但**逐条**写：切换当前台站只影响之后
//! 新记的通联，历史记录各自仍归属它当时的台站（见 `ham_web_core::station` 的模块文档）。
//!
//! 面板直接改 store 里的档案册、点「保存」才落盘；新增与删除也走同一个保存 ——
//! 一屏里的改动要么一起生效、要么一起丢掉，不搞「有的马上生效、有的要按保存」。

use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::ui::{Button, Chip, ChipGroup, Field, Input, Size, Variant};

use super::log_helpers::station_title;
use super::{Logbook, Removal, StationBook, StationProfile, use_log_store};
use crate::util::{alert, unique_id, window};

/// 当前台站名下有多少条通联（面板上显示，也是删除前的提示）。
fn entry_count(book: &StationBook, logbook: &Logbook) -> usize {
  let active = book.active().id;
  logbook
    .entries
    .iter()
    .filter(|e| book.of_entry(e).id == active)
    .count()
}

#[component]
pub(super) fn StationPanel(book: RwSignal<StationBook>, on_save: Callback<()>) -> impl IntoView {
  let show_station = RwSignal::new(false);
  let store = use_log_store();
  let logbook = store.logbook;

  // 编辑的永远是**当前台站**：要改别的台站，先在列表里切过去。
  let active = Memo::new(move |_| book.get().active().clone());
  // 台站列表与当前选中项各自成 memo：`Memo` 只在值真的变了才通知，所以在字段里打字
  // 不会把整排台站按钮重建（重建会丢掉原生控件上的交互状态）。
  let rows = Memo::new(move |_| {
    book
      .get()
      .profiles
      .iter()
      .map(|p| (p.id, station_title(p)))
      .collect::<Vec<_>>()
  });
  let count = Memo::new(move |_| book.with(|b| logbook.with(|lb| entry_count(b, lb))));

  let add = Callback::new(move |()| {
    book.update(|b| {
      let id = b.add("");
      b.set_active(id);
    });
  });

  let remove = Callback::new(move |()| {
    let (id, title) = {
      let b = book.get_untracked();
      (b.active().id, station_title(b.active()))
    };
    let msg = tf(
      "log.station-remove-confirm",
      &[&title, &count.get_untracked().to_string()],
    );
    if !window().confirm_with_message(&msg).unwrap_or(false) {
      return;
    }
    let mut outcome = Removal::Removed;
    book.update(|b| outcome = b.remove(id));
    if outcome == Removal::LastStation {
      alert(&t("log.station-keep-at-least-one"));
    }
  });

  // 每个字段一个入口：改的是当前档案的同名字段（大小写规整由各字段自己的闭包负责，
  // 呼号与网格转大写、其余原样）。
  let edit = move |set: fn(&mut StationProfile, String)| {
    Callback::new(move |v: String| {
      book.update(|b| {
        let id = b.active_id;
        if let Some(p) = b.profiles.iter_mut().find(|p| p.id == id) {
          set(p, v);
        }
      });
    })
  };

  view! {
    <section class="rounded-xl border bg-card">
      // 区块标题式的展开 / 收起开关：保留原生元素是因为 `Button` 没有 `aria-expanded`
      // 通道，而展开状态是读屏与 e2e 的契约（它视觉上是卡片标题，不是按钮）。
      <button
        type="button"
        class="flex w-full items-center justify-between px-4 py-3 text-left text-sm font-semibold"
        aria-expanded=move || show_station.get().to_string()
        on:click=move |_| show_station.update(|v| *v = !*v)
      >
        <span>{move || t("log.station-info")}</span>
        <span class="text-xs font-normal text-muted-foreground">
          {move || {
            let p = active.get();
            let call = p.callsign.trim().to_owned();
            let name = station_title(&p);
            if call.is_empty() {
              format!("{name} · {}", t("log.no-callsign-set-click"))
            } else if p.label.trim().is_empty() {
              // 没填档案名时 `name` 已经是呼号 / 默认名，再拼呼号会成「BG4XXX · BG4XXX」。
              name
            } else {
              format!("{name} · {call}")
            }
          }}
        </span>
      </button>
      {move || {
        show_station.get().then(|| {
          // `Field` 的标签与控件是兄弟节点，`r#for` / `id` 必须配对才能点击标签聚焦输入框。
          // id 在这里现生成：`then` 的闭包是 `FnOnce`，可以自由移出，而外层 `move ||`
          // 必须保持 `Fn`（可重复重渲染），不能捕获这些 id。
          let label_id = unique_id("station-label");
          let callsign_id = unique_id("station-callsign");
          let operator_id = unique_id("station-operator");
          let grid_id = unique_id("station-grid");
          let rig_id = unique_id("station-rig");
          let antenna_id = unique_id("station-antenna");
          view! {
            <div class="border-t p-4">
              <p class="mb-3 text-xs text-muted-foreground">
                {move || t("log.station-info-is-written")}
              </p>
              <div class="mb-4 flex flex-col gap-1.5">
                <span class="text-xs text-muted-foreground">{move || t("log.station-active")}</span>
                <div class="flex flex-wrap items-center gap-2">
                  <ChipGroup
                    value=Signal::derive(move || book.with(|b| b.active_id).to_string())
                    on_change=Callback::new(move |v: String| {
                      if let Ok(id) = v.parse::<u64>() {
                        book.update(|b| {
                          b.set_active(id);
                        });
                      }
                    })
                    aria_label=Signal::derive(move || t("log.station-active"))
                  >
                    {move || {
                      rows.get()
                        .into_iter()
                        .map(|(id, title)| view! { <Chip value=id.to_string()>{title}</Chip> })
                        .collect_view()
                    }}
                  </ChipGroup>
                  <Button variant=Variant::Outline size=Size::Sm on_click=add>
                    {move || t("log.station-add")}
                  </Button>
                  <Button
                    variant=Variant::Ghost
                    size=Size::Sm
                    disabled=Signal::derive(move || book.with(|b| b.profiles.len() <= 1))
                    on_click=remove
                  >
                    {move || t("log.station-remove")}
                  </Button>
                  <span class="text-xs text-muted-foreground">
                    {move || tf("log.station-entry-count", &[&count.get().to_string()])}
                  </span>
                </div>
              </div>
              <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
                <Field label=Signal::derive(move || t("log.station-name")) r#for=label_id.clone()>
                  <Input
                    id=label_id.clone()
                    value=Signal::derive(move || active.get().label.clone())
                    on_change=edit(|p, v| p.label = v)
                    placeholder=Signal::derive(move || t("log.station-name-placeholder"))
                  />
                </Field>
                <Field label=Signal::derive(move || t("log.station-callsign")) r#for=callsign_id.clone()>
                  <Input
                    id=callsign_id.clone()
                    value=Signal::derive(move || active.get().callsign.clone())
                    on_change=edit(|p, v| p.callsign = v.to_uppercase())
                    placeholder="BG4XXX"
                    class="uppercase"
                  />
                </Field>
                <Field label=Signal::derive(move || t("log.operator")) r#for=operator_id.clone()>
                  <Input
                    id=operator_id.clone()
                    value=Signal::derive(move || active.get().operator.clone())
                    on_change=edit(|p, v| p.operator = v)
                    placeholder=Signal::derive(move || t("log.same-as-callsign"))
                  />
                </Field>
                <Field label=Signal::derive(move || t("log.station-grid")) r#for=grid_id.clone()>
                  <Input
                    id=grid_id.clone()
                    value=Signal::derive(move || active.get().gridsquare.clone())
                    on_change=edit(|p, v| p.gridsquare = v.to_uppercase())
                    placeholder="OM89EW"
                    class="uppercase"
                  />
                </Field>
                <Field label=Signal::derive(move || t("log.rig")) r#for=rig_id.clone()>
                  <Input
                    id=rig_id.clone()
                    value=Signal::derive(move || active.get().rig.clone())
                    on_change=edit(|p, v| p.rig = v)
                    placeholder="FT-710"
                  />
                </Field>
                <Field label=Signal::derive(move || t("log.antenna")) r#for=antenna_id.clone()>
                  <Input
                    id=antenna_id.clone()
                    value=Signal::derive(move || active.get().antenna.clone())
                    on_change=edit(|p, v| p.antenna = v)
                    placeholder="DP 20m"
                  />
                </Field>
              </div>
              <div class="mt-3">
                <Button
                  variant=Variant::Default
                  size=Size::Sm
                  on_click=Callback::new(move |_| on_save.run(()))
                >
                  {move || t("log.save-station-info")}
                </Button>
              </div>
            </div>
          }
        })
      }}
    </section>
  }
}
