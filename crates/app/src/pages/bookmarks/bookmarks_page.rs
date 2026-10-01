use ham_web_core::{Bank, QuestionItem};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::common::{EmptyState, Loading};
use crate::data;
use crate::store;
use crate::ui::{Size, Variant, button_class};
use crate::util::set_title;

use super::bookmark_card::BookmarkCard;

#[component]
pub fn BookmarksPage() -> impl IntoView {
  set_title("收藏集");

  let questions = RwSignal::new(Vec::<QuestionItem>::new());
  let loading = RwSignal::new(true);
  let refresh = RwSignal::new(0usize);

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

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-3xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">"收藏集"</h1>
            <div class="text-xs text-muted-foreground">"练习中手动收藏的重点题目"</div>
          </div>
          <span class="rounded-full border px-3 py-1 text-xs text-muted-foreground">
            {move || format!("共 {} 题", questions.get().len())}
          </span>
          <a href="/print?src=bookmarks" class=button_class(Variant::Outline, Size::Sm, "")>"打印"</a>
        </div>
      </header>

      <div class="mx-auto max-w-3xl space-y-4 px-4 py-5">
        {move || {
          if loading.get() {
            view! { <Loading label="加载中..." class="py-10" /> }
              .into_any()
          } else if questions.get().is_empty() {
            view! {
              <EmptyState
                title="暂无收藏"
                description="在「练习」页面点击右上角书签图标即可收藏当前题。"
              />
            }
            .into_any()
          } else {
            view! {
              {questions
                .get()
                .into_iter()
                .map(|q| {
                  view! { <BookmarkCard question=q on_remove=Callback::new(remove) /> }
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
