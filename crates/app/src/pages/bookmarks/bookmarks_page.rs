use std::collections::HashSet;

use ham_web_core::{Bank, QuestionItem};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::common::{EmptyState, LoadMore, PageContainer, PageHeader, Skeleton};
use crate::data;
use crate::pages::browse::PAGE;
use crate::store;
use crate::ui::{Button, ButtonLink, Input, Size, Variant};
use crate::util::set_title;

use super::bookmark_card::BookmarkCard;
use crate::i18n::{t, tf, tp};

/// 收藏分组过滤方式。
#[derive(Clone, PartialEq)]
enum Filter {
  All,
  Ungrouped,
  Group(String),
}

#[component]
pub fn BookmarksPage() -> impl IntoView {
  set_title("shell.bookmarks");
  // 题目卡与分类浏览共用 `QuestionRow`，它渲染 `crates/core` 的中文原文（`kt_tracked`），
  // 译文词典要在本页挂一次拉取，否则以英文 / 西语直接打开本页时标签停在中文。
  data::use_knowledge_i18n();

  let questions = RwSignal::new(Vec::<QuestionItem>::new());
  let loading = RwSignal::new(true);
  let groups = RwSignal::new(store::load_groups());
  let group_filter = RwSignal::new(Filter::All);
  let new_group = RwSignal::new(String::new());
  // 列表一次显示多少条：卡片带完整解析，一次铺开几百张会很重；筛选条件变了回到第一页。
  let visible = RwSignal::new(PAGE);
  // 收藏快照：每张题目卡从里面读自己的收藏状态，切换时由卡片就地改它；
  // 其它标签页改了收藏时由 `store` 的修订号触发重读。
  let bookmarks = store::use_bookmark_snapshot();

  // 只在挂载和「别的标签页改了收藏」时拉一次库。本页自己的取消收藏走下面的就地删除，
  // 不再整页重载 —— 重载会重建所有卡片，别的卡里展开着的笔记和未保存的草稿都会丢。
  let external_change = store::bookmarks_revision();
  Effect::new(move |_| {
    external_change.get();
    loading.set(true);
    spawn_local(async move {
      let ids = store::load_bookmarks();
      // 列表按题号键控渲染，同一题号只留一份（收藏本来就以题号为键）。
      let mut seen = HashSet::new();
      let mut result = Vec::new();
      for bank in Bank::ALL {
        if let Ok(qs) = data::load_bank(None, bank, false).await {
          result.extend(
            qs.iter()
              .filter(|q| {
                q.stable_id()
                  .is_some_and(|id| ids.contains(&id) && seen.insert(id))
              })
              .cloned(),
          );
        }
      }
      questions.set(result);
      loading.set(false);
    });
  });

  // 题目卡自己切换了存储与快照；取消收藏后这一题不再属于收藏集，就地拿掉它。
  // 延后一拍：这个回调是在那张卡自己的点击处理里被调用的，当场卸载它等于在处理函数
  // 执行到一半时销毁它的响应式所有者。
  let on_unbookmark = Callback::new(move |id: String| {
    spawn_local(async move {
      questions.update(|qs| qs.retain(|q| q.stable_id().as_deref() != Some(id.as_str())));
      // `toggle_bookmark` 已把存储里各分组的残留清掉了，页面持有的副本要跟着重读，
      // 否则分组计数过期，之后任何分组操作还会把残留条目写回存储。
      groups.set(store::load_groups());
    });
  });

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

  // 分组连同它的成员关系一起删，误点一次就得重建：先问一句。
  let delete_group = move |name: String| {
    crate::ui::confirm(
      tf("exam.delete-group-confirm", &[&name]),
      t("exam.delete-group-hint"),
      t("exam.delete-this-group"),
      crate::ui::ConfirmTone::Danger,
      Callback::new(move |_| {
        let mut gs = groups.get_untracked();
        gs.delete(&name);
        store::save_groups(&gs);
        groups.set(gs);
        if matches!(group_filter.get_untracked(), Filter::Group(n) if n == name) {
          group_filter.set(Filter::All);
        }
        crate::ui::push_success(t("exam.group-deleted"));
      }),
    );
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

  // 外层渲染闭包只订阅这几个「会改变分支」的 Memo（有没有收藏 / 筛选后是否为空）：
  // 列表里少一张卡、多一个分组都不会让它重跑，也就不会重建整个列表。
  // 列表本身交给 `<For>` 按题号键控，只增删真正变化的那几张卡。
  let has_any = Memo::new(move |_| questions.with(|qs| !qs.is_empty()));
  let total = Memo::new(move |_| filtered().count());
  let list_empty = Memo::new(move |_| total.get() == 0);
  let page_items = move || {
    filtered()
      .take(visible.get())
      .collect::<Vec<QuestionItem>>()
  };
  Effect::new(move |_| {
    group_filter.with(|_| ());
    crate::debounced::set_if_changed(visible, PAGE);
  });

  let filter_tab = |on: bool| {
    if on {
      "border bg-primary px-3 py-1 text-xs text-primary-foreground"
    } else {
      "border px-3 py-1 text-xs text-muted-foreground transition-colors hover:bg-accent"
    }
  };

  if crate::kit::is_modern_at_mount() {
    return view! {
      <div class="md-root">
        <div class="md-page md-article-page">
          <crate::kit_modern::ModernPageHead
            title=Signal::derive(move || t("shell.bookmarks"))
            subtitle=Signal::derive(move || t("exam.key-questions-bookmarked-during"))
          />

          <div class="md-glossary-bar">
            <crate::kit_modern::ModernInput
              value=new_group
              on_input=Callback::new(move |v: String| new_group.set(v))
              placeholder=Signal::derive(move || t("exam.new-group-name"))
              class="md-glossary-search"
            />
            <button type="button" class="md-btn md-btn-quiet" on:click=move |_| create_group()>
              {move || t("exam.new-group")}
            </button>
            <a class="md-btn md-btn-ghost" href="/print?src=bookmarks">
              {move || t("learning.print")}
            </a>
          </div>

          <div class="md-chip-row mb-4">
            <button
              type="button"
              class="md-chip"
              aria-pressed=move || (group_filter.get() == Filter::All).to_string()
              on:click=move |_| group_filter.set(Filter::All)
            >
              {move || t("exam.all")}
            </button>
            <button
              type="button"
              class="md-chip"
              aria-pressed=move || (group_filter.get() == Filter::Ungrouped).to_string()
              on:click=move |_| group_filter.set(Filter::Ungrouped)
            >
              {move || t("exam.ungrouped")}
            </button>
            {move || {
              groups
                .get()
                .groups
                .clone()
                .into_iter()
                .map(|g| {
                  let label = g.name.clone();
                  let click = g.name.clone();
                  let click_aria = g.name.clone();
                  let del = g.name.clone();
                  let count = g.ids.len();
                  view! {
                    <span class="inline-flex items-center gap-1">
                      <button
                        type="button"
                        class="md-chip"
                        aria-pressed=move || (group_filter.get() == Filter::Group(click_aria.clone())).to_string()
                        on:click=move |_| group_filter.set(Filter::Group(click.clone()))
                      >
                        {label}
                        <span class="md-menu-count">{count}</span>
                      </button>
                      <button
                        type="button"
                        class="md-icon-btn md-icon-btn-xs"
                        aria-label=move || t("exam.delete-this-group")
                        on:click=move |_| delete_group(del.clone())
                      >
                        <crate::kit_modern::ModernIcon kind=crate::kit_modern::MIcon::Close />
                      </button>
                    </span>
                  }
                })
                .collect_view()
            }}
            <span class="md-muted md-sm ml-auto">
              {move || tp("common.questions-total", questions.get().len(), &[&(questions.get().len()).to_string()])}
            </span>
          </div>

          {move || {
            if loading.get() {
              return view! {
                <div class="flex flex-col gap-3">
                  <span class="sr-only" role="status">{move || t("exam.loading")}</span>
                  {(0..4)
                    .map(|_| {
                      view! {
                        <div class="md-skel" style="height: 96px; border-radius: var(--md-r-lg)"></div>
                      }
                    })
                    .collect_view()}
                </div>
              }
                .into_any();
            }
            if !has_any.get() {
              return view! {
                <EmptyState
                  title=Signal::derive(move || t("exam.no-bookmarks-yet"))
                  description=Signal::derive(move || t("exam.tap-the-bookmark-icon"))
                  action_text=Signal::derive(move || t("exam.practice"))
                  action_href="/practice".to_owned()
                />
              }
                .into_any();
            }
            if list_empty.get() {
              return view! {
                <EmptyState
                  title=Signal::derive(move || t("exam.no-questions-in-this"))
                  description=Signal::derive(move || t("exam.add-bookmarks-to-this"))
                />
              }
                .into_any();
            }
            view! {
              <div class="md-bookmark-list">
                <For
                  each=page_items
                  key=|q| q.stable_id().unwrap_or_default()
                  children=move |q| {
                    view! {
                      <BookmarkCard
                        question=q
                        groups=groups
                        bookmarks=bookmarks
                        on_unbookmark=on_unbookmark
                        on_add_to_group=Callback::new(add_to_group)
                        on_remove_from_group=Callback::new(remove_from_group)
                      />
                    }
                  }
                />
              </div>
              <LoadMore visible=visible total=total step=PAGE />
            }
              .into_any()
          }}
        </div>
      </div>
    }
    .into_any();
  }

  view! {
    <div class="min-h-svh animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.bookmarks")
        subtitle=move || t("exam.key-questions-bookmarked-during")
        actions=ViewFn::from(move || {
          view! {
            <span class="border px-3 py-1 text-xs text-muted-foreground">
              {move || tp("common.questions-total", questions.get().len(), &[&(questions.get().len()).to_string()])}
            </span>
            <ButtonLink
              href="/print?src=bookmarks"
              variant=Variant::Outline
              size=Size::Sm
            >{move || t("learning.print")}</ButtonLink>
          }
        })
      >
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-2 px-4 pb-3">
          <Input
            value=new_group
            on_change=Callback::new(move |v: String| new_group.set(v))
            placeholder=Signal::derive(move || t("exam.new-group-name"))
            class="w-40"
          />
          <Button
            variant=Variant::Outline
            size=Size::Sm
            on_click=Callback::new(move |_| create_group())
          >
            {move || t("exam.new-group")}
          </Button>
          <div class="flex flex-wrap items-center gap-1.5">
            <button type="button" class=move || filter_tab(group_filter.get() == Filter::All) on:click=move |_| group_filter.set(Filter::All)>
              {move || t("exam.all")}
            </button>
            <button type="button" class=move || filter_tab(group_filter.get() == Filter::Ungrouped) on:click=move |_| group_filter.set(Filter::Ungrouped)>
              {move || t("exam.ungrouped")}
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
                      class="text-muted-foreground/60 transition-colors hover:text-hp-text"
                      aria-label=move || t("exam.delete-this-group")
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
      </PageHeader>

      <PageContainer class="space-y-4">
        {move || {
          if loading.get() {
            view! {
              <div class="space-y-3">
                <span class="sr-only" role="status">{move || t("exam.loading")}</span>
                {(0..4)
                  .map(|_| {
                    view! {
                      <div class=crate::kit::card_pad_class()>
                        <Skeleton class="h-3 w-32" />
                        <Skeleton class="mt-3 h-4 w-3/4" />
                      </div>
                    }
                  })
                  .collect_view()}
              </div>
            }
              .into_any()
          } else if !has_any.get() {
            view! {
              <EmptyState
                title=Signal::derive(move || t("exam.no-bookmarks-yet"))
                description=Signal::derive(move || t("exam.tap-the-bookmark-icon"))
              />
            }
            .into_any()
          } else if list_empty.get() {
            view! {
              <EmptyState title=Signal::derive(move || t("exam.no-questions-in-this")) description=t("exam.add-bookmarks-to-this") />
            }
            .into_any()
          } else {
            view! {
              <For
                each=page_items
                key=|q| q.stable_id().unwrap_or_default()
                children=move |q| {
                  view! {
                    <BookmarkCard
                      question=q
                      groups=groups
                      bookmarks=bookmarks
                      on_unbookmark=on_unbookmark
                      on_add_to_group=Callback::new(add_to_group)
                      on_remove_from_group=Callback::new(remove_from_group)
                    />
                  }
                }
              />
              <LoadMore visible=visible total=total step=PAGE class="w-full" />
            }
            .into_any()
          }
        }}
      </PageContainer>
    </div>
  }
  .into_any()
}
