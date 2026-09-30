use ham_web_core::{Bank, QuestionItem};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::data;
use crate::store;
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
            <div class="text-base font-semibold leading-tight">"收藏集"</div>
            <div class="text-xs text-muted-foreground">"练习中手动收藏的重点题目"</div>
          </div>
          <span class="rounded-full border px-3 py-1 text-xs text-muted-foreground">
            {move || format!("共 {} 题", questions.get().len())}
          </span>
        </div>
      </header>

      <div class="mx-auto max-w-3xl space-y-4 px-4 py-5">
        {move || {
          if loading.get() {
            view! { <div class="px-4 py-10 text-center text-sm text-muted-foreground">"加载中..."</div> }
              .into_any()
          } else if questions.get().is_empty() {
            view! {
              <div class="rounded-xl border bg-card px-4 py-12 text-center">
                <div class="text-sm font-medium">"暂无收藏"</div>
                <div class="mt-1 text-xs text-muted-foreground">
                  "在「练习」页面点击右上角书签图标即可收藏当前题。"
                </div>
              </div>
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
