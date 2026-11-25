//! 设台申请核对清单：逐项勾选，每项指到《业余无线电台管理办法》的条文，能用站内工具核对的再指到工具。
//!
//! 清单的内容与出处全在核心 `ham_web_core::license_checklist`（单测逐条核对出处原话真的在条文里）；
//! 这里只负责渲染与勾选。勾选状态存在本机的 `ui:license-checklist`（`ui:*` 是设备专属、不同步的 key）。

use std::collections::BTreeSet;

use ham_web_core::license_checklist::{GROUPS, ITEMS, items_in};
use leptos::prelude::*;

use super::checklist_row::ChecklistRow;
use super::texts::group_title;
use crate::i18n::{t, tf};
use crate::ui::{Button, Size, Variant, card_title_class};
use crate::util::storage;

const KEY: &str = "ui:license-checklist";

/// 读出已勾选的 id：只认清单里现有的项（清单改过之后，旧的 id 不该继续占着计数）。
fn load_checked() -> BTreeSet<String> {
  storage::get_json::<Vec<String>>(KEY)
    .unwrap_or_default()
    .into_iter()
    .filter(|id| ITEMS.iter().any(|i| i.id == id))
    .collect()
}

#[component]
pub fn LicenseChecklist() -> impl IntoView {
  let checked = RwSignal::new(load_checked());
  let persist = move || {
    let ids: Vec<String> = checked.get_untracked().into_iter().collect();
    storage::set_json(KEY, &ids);
  };
  let total = ITEMS.len();
  let done = Memo::new(move |_| checked.with(BTreeSet::len));

  view! {
    <section class=crate::kit::card_class() id="license-checklist" data-testid="license-checklist">
      <div class="flex flex-wrap items-center gap-3 border-b px-4 py-3">
        <h2 class=card_title_class("min-w-0 flex-1")>{move || t("knowledge.checklist-title")}</h2>
        <span class="text-xs text-muted-foreground" role="status" data-testid="checklist-progress">
          {move || tf("knowledge.checklist-progress", &[&done.get().to_string(), &total.to_string()])}
        </span>
        <Button
          variant=Variant::Outline
          size=Size::Sm
          disabled=Signal::derive(move || done.get() == 0)
          on_click=Callback::new(move |_| {
            checked.set(BTreeSet::new());
            persist();
          })
        >
          {move || t("knowledge.checklist-reset")}
        </Button>
      </div>
      <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("knowledge.checklist-note")}</p>
      {GROUPS
        .iter()
        .map(|&group| {
          view! {
            <div class="px-4 py-3">
              <h3 class="text-sm">{move || group_title(group)}</h3>
              <ul class="mt-2 divide-y">
                {items_in(group).map(|item| view! { <ChecklistRow item=item checked=checked persist=persist /> }).collect_view()}
              </ul>
            </div>
          }
        })
        .collect_view()}
    </section>
  }
}
