use std::collections::HashSet;

use ham_web_core::QuestionItem;
use ham_web_core::bookmark_groups::BookmarkGroups;
use leptos::prelude::*;

use crate::components::common::NoteEditor;
use crate::i18n::t;
use crate::pages::browse::QuestionRow;
use crate::store;
use crate::ui::{Button, ControlSize, SelectOption, SimpleSelect, Size, Tooltip, Variant};

/// 一道收藏题卡片：题目展示直接用分类浏览的 `QuestionRow`（两处同一张卡），
/// 本组件只在卡片末尾追加收藏集独有的分组与笔记操作区。
///
/// `groups` 是页面持有的分组信号：卡片自己读它，分组增删只更新这块操作区，
/// 不会重建整张卡（否则别的卡片里展开着的笔记和没保存的草稿会跟着丢）。
/// `on_unbookmark` 在卡片上的收藏按钮被取消后触发，参数是这一题的编号（列表据此就地拿掉它）。
#[component]
pub(super) fn BookmarkCard(
  question: QuestionItem,
  groups: RwSignal<BookmarkGroups>,
  bookmarks: RwSignal<HashSet<String>>,
  on_unbookmark: Callback<String>,
  on_add_to_group: Callback<(QuestionItem, String)>,
  on_remove_from_group: Callback<(QuestionItem, String)>,
) -> impl IntoView {
  let modern = crate::kit::is_modern_at_mount();
  let q_add = question.clone();
  let q_remove = question.clone();
  let pick = RwSignal::new(String::new());
  // 笔记按需展开：收藏列表可能很长，每题都铺开一个输入框会把页面撑得没法浏览。
  let id = question.stable_id().unwrap_or_default();
  let note_open = RwSignal::new(false);
  // 已有笔记的题目标出来，省得逐条展开去找（笔记保存后展开状态会重渲染，此处够用）。
  let has_note = store::load_note(&id).is_some();
  let on_bookmark_change = Callback::new({
    let id = id.clone();
    move |added: bool| {
      if !added {
        on_unbookmark.run(id.clone());
      }
    }
  });
  let note_id = Signal::stored(id.clone());

  // 本题当前所属分组 / 全部分组：都从分组信号里读，随增删实时更新。
  let groups_of = {
    let id = id.clone();
    move || groups.with(|gs| gs.groups_of(&id))
  };
  let all_groups = move || {
    groups.with(|gs| {
      gs.groups
        .iter()
        .map(|g| g.name.clone())
        .collect::<Vec<String>>()
    })
  };

  let footer = if modern {
    let opts = Signal::derive(move || {
      all_groups()
        .into_iter()
        .map(|g| (g.clone(), g))
        .collect::<Vec<(String, String)>>()
    });
    view! {
      <div class="md-bookmark-foot">
        <span class="md-stat-label">{move || t("exam.group")}</span>
        {move || {
          groups_of()
            .into_iter()
            .map(|name| {
              let n = name.clone();
              let qr = q_remove.clone();
              view! {
                <button
                  type="button"
                  class="md-chip md-chip-sm"
                  on:click=move |_| on_remove_from_group.run((qr.clone(), n.clone()))
                >
                  {name} " ×"
                </button>
              }
            })
            .collect_view()
        }}
        {move || {
          (!all_groups().is_empty())
            .then(|| {
              let q = q_add.clone();
              view! {
                <crate::kit_modern::ModernSelect
                  value=pick
                  on_change=Callback::new(move |v: String| {
                    on_add_to_group.run((q.clone(), v));
                  })
                  options=opts
                  placeholder=Signal::derive(move || t("exam.add-to-group"))
                  aria_label=Signal::derive(move || t("exam.add-to-group"))
                  class="md-select-sm"
                />
              }
            })
        }}
        <button
          type="button"
          class="md-btn md-btn-ghost md-btn-sm ml-auto"
          aria-expanded=move || note_open.get().to_string()
          on:click=move |_| note_open.update(|v| *v = !*v)
        >
          {move || t("exam.notes")}
          {has_note.then(|| view! { <span class="md-dot-badge" aria-hidden="true"></span> })}
        </button>
      </div>

      {move || {
        note_open
          .get()
          .then(|| {
            view! {
              <div class="mt-3">
                <NoteEditor question_id=note_id />
              </div>
            }
          })
      }}
    }
    .into_any()
  } else {
    view! {
      <div class="mt-3 flex flex-wrap items-center gap-1.5 border-t pt-2">
        <span class="text-xs text-muted-foreground">{move || t("exam.group")}</span>
        {move || {
          groups_of()
            .into_iter()
            .map(|name| {
              let n = name.clone();
              let qr = q_remove.clone();
              view! {
                <Tooltip label=Signal::derive(move || t("exam.remove-from-this-group"))>
                  <button
                    type="button"
                    class="inline-flex items-center gap-1 border bg-accent px-2 py-0.5 text-xs transition-colors hover:bg-destructive/10"
                    on:click=move |_| on_remove_from_group.run((qr.clone(), n.clone()))
                  >
                    {name} " ×"
                  </button>
                </Tooltip>
              }
            })
            .collect_view()
        }}
        {move || {
          (!all_groups().is_empty())
            .then(|| {
              let q = q_add.clone();
              let group_options: Vec<SelectOption> = all_groups()
                .into_iter()
                .map(|g| SelectOption::new(g.clone(), g))
                .collect();
              view! {
                <SimpleSelect
                  value=pick
                  on_change=Callback::new(move |v: String| {
                    if !v.is_empty() {
                      on_add_to_group.run((q.clone(), v));
                    }
                    pick.set(String::new());
                  })
                  options=group_options
                  placeholder=Signal::derive(move || t("exam.add-to-group"))
                  size=ControlSize::Sm
                  aria_label=Signal::derive(move || t("exam.add-to-group"))
                  class="h-7 w-auto px-2 py-0 text-xs md:text-xs"
                />
              }
            })
        }}
        <Button
          variant=Variant::Text
          size=Size::Xs
          class="ml-auto gap-1 px-1 hover:bg-accent"
          aria_expanded=Signal::derive(move || note_open.get())
          on_click=Callback::new(move |_| note_open.update(|v| *v = !*v))
        >
          {move || t("exam.notes")}
          {has_note.then(|| {
            view! { <span class="size-1.5 bg-primary"></span> }
          })}
        </Button>
      </div>
      {move || {
        note_open
          .get()
          .then(|| {
            view! {
              <div class="mt-3">
                <NoteEditor question_id=note_id />
              </div>
            }
          })
      }}
    }
    .into_any()
  };

  view! {
    <QuestionRow q=question bookmarks=bookmarks on_bookmark_change=on_bookmark_change>
      {footer}
    </QuestionRow>
  }
}
