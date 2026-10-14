//! 设计存档面板：给当前模型起名保存、列出已存的设计、一键载回或删除。
//!
//! 数据模型与「存什么」在核心 [`ham_web_core::nec_designs`]；本组件只负责
//! 读写存储门面（`kv`）与界面。默认折叠：大多数时候用户只是在调参数。

use ham_web_core::nec::{NecInput, TransmissionLine};
use ham_web_core::nec_designs::{CORRUPT_KEY, DESIGNS_KEY, DesignBook, SaveError, SavedModel};
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::ui::{Button, Field, Input, Size, Variant};

use super::NOTE;

/// 把存档册写进存储门面（localStorage 快照 + IndexedDB 权威）；序列化失败返回 `false`，
/// 此时什么都没写，调用方不能让界面显示「已保存」。
fn persist(book: &DesignBook) -> bool {
  let Ok(json) = serde_json::to_string(book) else {
    return false;
  };
  crate::kv::save_large(DESIGNS_KEY, &json);
  true
}

/// 读出持久化的存档册。解析失败时把原文另存到 [`CORRUPT_KEY`]（已有备份则不覆盖更早的那份）
/// 并提示 —— 直接当空册的话，用户下一次保存就把坏数据永久覆盖了。
fn read_book(json: &str, notice: RwSignal<String>) -> DesignBook {
  if json.trim().is_empty() {
    return DesignBook::default();
  }
  DesignBook::parse(json).unwrap_or_else(|| {
    if crate::kv::load_snapshot(CORRUPT_KEY).is_none() {
      crate::kv::save_large(CORRUPT_KEY, json);
    }
    let _ = notice.try_set(t("tools.nec-archive-corrupt"));
    DesignBook::default()
  })
}

#[component]
pub(super) fn DesignArchiveSection(
  /// 当前可求解的输入（导线表里有格子没填好时为 `None`）。
  current: Memo<Option<NecInput>>,
  /// 当前的传输线表（已解析）。
  lines: Signal<Vec<TransmissionLine>>,
  /// 载回一份设计：页面负责把它铺进各张表。
  on_load: Callback<(NecInput, Vec<TransmissionLine>, String), ()>,
) -> impl IntoView {
  let notice = RwSignal::new(String::new());
  // 首屏先用同步快照，随后按门面的版本仲裁校正（与日志 store 同一套读法）。
  let book = RwSignal::new(
    crate::kv::load_snapshot(DESIGNS_KEY)
      .map(|s| read_book(&s, notice))
      .unwrap_or_default(),
  );
  let dirty = StoredValue::new(false);
  wasm_bindgen_futures::spawn_local(async move {
    if let Some(json) = crate::kv::load_large(DESIGNS_KEY).await
      && !dirty.try_get_value().unwrap_or(true)
    {
      let _ = book.try_set(read_book(&json, notice));
    }
  });
  // 写入失败时不更新界面里的册子：界面与存储不一致，比一条错误提示更糟。
  let commit = move |next: DesignBook| -> bool {
    if !persist(&next) {
      notice.set(t("tools.nec-archive-save-failed"));
      return false;
    }
    dirty.set_value(true);
    book.set(next);
    true
  };

  let name = RwSignal::new(String::new());

  let save = move |_| {
    let Some(input) = current.get_untracked() else {
      notice.set(t("tools.nec-archive-unusable"));
      return;
    };
    let model = SavedModel::from_input(&input, &lines.get_untracked());
    let mut next = book.get_untracked();
    match next.save(&name.get_untracked(), model, &crate::util::local_today()) {
      Ok(_) => {
        let saved = next.designs[0].name.clone();
        if commit(next) {
          notice.set(tf("tools.nec-archive-saved", &[&saved]));
        }
      }
      Err(SaveError::EmptyName) => notice.set(t("tools.nec-archive-name-required")),
      Err(SaveError::Unusable) => notice.set(t("tools.nec-archive-unusable")),
      Err(SaveError::Full) => notice.set(t("tools.nec-archive-full")),
    }
  };

  view! {
    <details class="pxl-card border bg-card">
      <summary class="cursor-pointer px-4 py-3 text-sm font-semibold">
        {move || t("tools.nec-archive")}
      </summary>
      <div class="space-y-3 px-4 pb-4">
        <p class=NOTE>{move || t("tools.nec-archive-note")}</p>
        <div class="flex flex-wrap items-end gap-2">
          <Field label=Signal::derive(move || t("tools.nec-archive-name")) r#for="nec-archive-name">
            <Input
              id="nec-archive-name"
              value=name
              on_change=Callback::new(move |v: String| name.set(v))
              placeholder=Signal::derive(move || t("tools.nec-archive-name-placeholder"))
              class="w-56"
            />
          </Field>
          <Button variant=Variant::Default size=Size::Sm on_click=Callback::new(save)>
            {move || t("tools.nec-archive-save")}
          </Button>
        </div>
        {move || {
          let msg = notice.get();
          (!msg.is_empty()).then(|| view! { <p class=NOTE role="status">{msg}</p> })
        }}
        {move || {
          let designs = book.get().designs;
          if designs.is_empty() {
            return view! { <p class=NOTE>{move || t("tools.nec-archive-empty")}</p> }.into_any();
          }
          view! {
            <ul class="divide-y border" data-slot="nec-archive-list">
              {designs
                .into_iter()
                .map(|d| {
                  let id = d.id;
                  let title = d.name.clone();
                  let usable = d.model.is_usable();
                  let model = d.model.clone();
                  let load_name = d.name.clone();
                  let del_name = d.name.clone();
                  view! {
                    <li class="flex flex-wrap items-center gap-2 px-3 py-2 text-sm">
                      <span class="font-medium">{title}</span>
                      <span class="text-xs text-muted-foreground tabular-nums">{d.saved_at.clone()}</span>
                      <span class="ml-auto flex gap-2">
                        <Button
                          variant=Variant::Outline
                          size=Size::Xs
                          disabled=Signal::derive(move || !usable)
                          aria_label=Signal::derive({
                            let n = load_name.clone();
                            move || tf("tools.nec-archive-load-named", &[&n])
                          })
                          on_click=Callback::new({
                            let n = load_name.clone();
                            move |_| {
                              let (input, lines) = model.to_input();
                              on_load.run((input, lines, n.clone()));
                            }
                          })
                        >
                          {move || t("tools.nec-archive-load")}
                        </Button>
                        <Button
                          variant=Variant::Outline
                          size=Size::Xs
                          aria_label=Signal::derive({
                            let n = del_name.clone();
                            move || tf("tools.nec-archive-delete-named", &[&n])
                          })
                          on_click=Callback::new({
                            let n = del_name.clone();
                            move |_| {
                              let ask = tf("tools.nec-archive-delete-confirm", &[&n]);
                              if !crate::util::window().confirm_with_message(&ask).unwrap_or(false) {
                                return;
                              }
                              let mut next = book.get_untracked();
                              if next.remove(id) {
                                let _ = commit(next);
                              }
                            }
                          })
                        >
                          {move || t("log.delete")}
                        </Button>
                      </span>
                    </li>
                  }
                })
                .collect_view()}
            </ul>
          }
          .into_any()
        }}
      </div>
    </details>
  }
}
