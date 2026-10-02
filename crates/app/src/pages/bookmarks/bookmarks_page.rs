use ham_web_core::{Bank, QuestionItem};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::common::{EmptyState, Loading};
use crate::data;
use crate::store;
use crate::ui::{Size, Variant, button_class, input_class};
use crate::util::set_title;

use super::bookmark_card::BookmarkCard;
use crate::i18n::{t, tf};

/// 收藏分组过滤方式。
#[derive(Clone, PartialEq)]
enum Filter {
  All,
  Ungrouped,
  Group(String),
}

#[component]
pub fn BookmarksPage() -> impl IntoView {
  set_title(&t("收藏集"));

  let questions = RwSignal::new(Vec::<QuestionItem>::new());
  let loading = RwSignal::new(true);
  let refresh = RwSignal::new(0usize);
  let groups = RwSignal::new(store::load_groups());
  let group_filter = RwSignal::new(Filter::All);
  let new_group = RwSignal::new(String::new());

  Effect::new(move |_| {
    refresh.get();
    loading.set(true);
    spawn_local(async move {
      let ids = store::load_bookmarks();
      let mut result = Vec::new();
      for bank in Bank::ALL {
        if let Ok(qs) = data::load_bank(None, bank, false).await {
          result.extend(
            qs.iter()
              .filter(|q| q.stable_id().is_some_and(|id| ids.contains(&id)))
              .cloned(),
          );
        }
      }
      questions.set(result);
      loading.set(false);
    });
  });

  let remove = move |q: QuestionItem| {
    if let Some(id) = q.stable_id() {
      store::toggle_bookmark(&id);
    }
    refresh.update(|r| *r += 1);
  };

  let create_group = move || {
    let name = new_group.get_untracked().trim().to_owned();
    if name.is_empty() {
      return;
    }
    let mut gs = groups.get_untracked();
    if gs.create(&name) {
      store::save_groups(&gs);
      groups.set(gs);
      new_group.set(String::new());
    }
  };

  let delete_group = move |name: String| {
    let mut gs = groups.get_untracked();
    gs.delete(&name);
    store::save_groups(&gs);
    groups.set(gs);
    if matches!(group_filter.get_untracked(), Filter::Group(n) if n == name) {
      group_filter.set(Filter::All);
    }
  };

  let add_to_group = move |(q, name): (QuestionItem, String)| {
    let Some(id) = q.stable_id() else {
      return;
    };
    let mut gs = groups.get_untracked();
    gs.add(&name, &id);
    store::save_groups(&gs);
    groups.set(gs);
  };

  let remove_from_group = move |(q, name): (QuestionItem, String)| {
    let Some(id) = q.stable_id() else {
      return;
    };
    let mut gs = groups.get_untracked();
    gs.remove(&name, &id);
    store::save_groups(&gs);
    groups.set(gs);
  };

  // 过滤后的题目列表。
  let filtered = move || {
    let gs = groups.get();
    let flt = group_filter.get();
    questions.get().into_iter().filter(move |q| {
      let id = q.stable_id().unwrap_or_default();
      let owned = gs.groups_of(&id);
      match &flt {
        Filter::All => true,
        Filter::Ungrouped => owned.is_empty(),
        Filter::Group(name) => owned.iter().any(|g| g == name),
      }
    })
  };

  let filter_tab = |on: bool| {
    if on {
      "rounded-full border bg-primary px-3 py-1 text-xs font-medium text-primary-foreground"
    } else {
      "rounded-full border px-3 py-1 text-xs text-muted-foreground transition-colors hover:bg-accent"
    }
  };

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-3xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("收藏集")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("练习中手动收藏的重点题目，可按分组整理")}</div>
          </div>
          <span class="rounded-full border px-3 py-1 text-xs text-muted-foreground">
            {move || tf("共 {} 题", &[&(questions.get().len()).to_string()])}
          </span>
          <a href="/print?src=bookmarks" class=button_class(Variant::Outline, Size::Sm, "")>{move || t("打印")}</a>
        </div>
        <div class="mx-auto flex max-w-3xl flex-wrap items-center gap-2 px-4 pb-3">
          <input
            type="text"
            prop:value=move || new_group.get()
            on:input=move |e| new_group.set(event_target_value(&e))
            placeholder=move || t("新建分组名称…")
            class=format!("{} w-40", input_class(""))
          />
          <button type="button" class=button_class(Variant::Outline, Size::Sm, "") on:click=move |_| create_group()>
            {move || t("新建分组")}
          </button>
          <div class="flex flex-wrap items-center gap-1.5">
            <button type="button" class=move || filter_tab(group_filter.get() == Filter::All) on:click=move |_| group_filter.set(Filter::All)>
              {move || t("全部")}
            </button>
            <button type="button" class=move || filter_tab(group_filter.get() == Filter::Ungrouped) on:click=move |_| group_filter.set(Filter::Ungrouped)>
              {move || t("未分组")}
            </button>
            {move || {
              groups.get().groups.clone().into_iter().map(|g| {
                let label_name = g.name.clone();
                let class_name = g.name.clone();
                let click_name = g.name.clone();
                let del_name = g.name.clone();
                view! {
                  <span class="inline-flex items-center gap-1">
                    <button
                      type="button"
                      class=move || filter_tab(group_filter.get() == Filter::Group(class_name.clone()))
                      on:click=move |_| group_filter.set(Filter::Group(click_name.clone()))
                    >
                      {label_name.clone()} " " <span class="tabular-nums">{g.ids.len()}</span>
                    </button>
                    <button
                      type="button"
                      class="text-muted-foreground/60 transition-colors hover:text-destructive"
                      title=move || t("删除该分组")
                      aria-label=move || t("删除该分组")
                      on:click=move |_| delete_group(del_name.clone())
                    >
                      "×"
                    </button>
                  </span>
                }
              }).collect_view()
            }}
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-3xl space-y-4 px-4 py-5">
        {move || {
          if loading.get() {
            view! { <Loading label=t("加载中...") class="py-10" /> }
              .into_any()
          } else if questions.get().is_empty() {
            view! {
              <EmptyState
                title=t("暂无收藏")
                description=t("在「练习」页面点击右上角书签图标即可收藏当前题。")
              />
            }
            .into_any()
          } else {
            let list: Vec<QuestionItem> = filtered().collect();
            if list.is_empty() {
              return view! {
                <EmptyState title=t("该分组暂无题目") description=t("在题目卡片的「加入分组」下拉里把收藏归入此分组。") />
              }
              .into_any();
            }
            let gs = groups.get();
            let all_groups: Vec<String> = gs.groups.iter().map(|g| g.name.clone()).collect();
            view! {
              {list
                .into_iter()
                .map(|q| {
                  let id = q.stable_id().unwrap_or_default();
                  let groups_of = gs.groups_of(&id);
                  view! {
                    <BookmarkCard
                      question=q
                      groups_of=groups_of
                      all_groups=all_groups.clone()
                      on_remove=Callback::new(remove)
                      on_add_to_group=Callback::new(add_to_group)
                      on_remove_from_group=Callback::new(remove_from_group)
                    />
                  }
                })
                .collect_view()}
            }
            .into_any()
          }
        }}
      </div>
    </div>
  }
}
