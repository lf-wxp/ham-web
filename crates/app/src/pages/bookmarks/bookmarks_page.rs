use ham_web_core::{Bank, QuestionItem};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::common::{EmptyState, PageContainer, PageHeader, Skeleton};
use crate::data;
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
            if questions.get().is_empty() {
              return view! {
                <div class="md-panel md-empty">
                  <h2 class="md-panel-title">{move || t("exam.no-bookmarks-yet")}</h2>
                  <p class="md-muted md-sm">{move || t("exam.tap-the-bookmark-icon")}</p>
                </div>
              }
                .into_any();
            }
            let list: Vec<QuestionItem> = filtered().collect();
            if list.is_empty() {
              return view! {
                <div class="md-panel md-empty">
                  <h2 class="md-panel-title">{move || t("exam.no-questions-in-this")}</h2>
                  <p class="md-muted md-sm">{move || t("exam.add-bookmarks-to-this")}</p>
                </div>
              }
                .into_any();
            }
            let gs = groups.get();
            let all_groups: Vec<String> = gs.groups.iter().map(|g| g.name.clone()).collect();
            view! {
              <div class="md-bookmark-list">
                {list
                  .into_iter()
                  .map(|q| {
                    let id = q.stable_id().unwrap_or_default();
                    let groups_of = gs
                      .groups
                      .iter()
                      .filter(|g| g.ids.iter().any(|x| x == &id))
                      .map(|g| g.name.clone())
                      .collect::<Vec<_>>();
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
              </div>
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
          } else if questions.get().is_empty() {
            view! {
              <EmptyState
                title=Signal::derive(move || t("exam.no-bookmarks-yet"))
                description=Signal::derive(move || t("exam.tap-the-bookmark-icon"))
              />
            }
            .into_any()
          } else {
            let list: Vec<QuestionItem> = filtered().collect();
            if list.is_empty() {
              return view! {
                <EmptyState title=Signal::derive(move || t("exam.no-questions-in-this")) description=t("exam.add-bookmarks-to-this") />
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
      </PageContainer>
    </div>
  }
  .into_any()
}
