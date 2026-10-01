use leptos::prelude::*;

/// 右下角更新提示卡片：标题、可选说明、条目列表与底部操作按钮。
#[component]
pub(crate) fn Notice(
  #[prop(into)] title: String,
  items: Vec<String>,
  #[prop(optional, into)] detail: Option<String>,
  children: Children,
) -> impl IntoView {
  view! {
    <div role="status" class="pointer-events-auto rounded-xl border bg-card p-4 text-sm shadow-lg animate-in fade-in slide-in-from-bottom-2 duration-300">
      <p class="font-semibold">{title}</p>
      {detail.map(|d| view! { <p class="mt-1 text-muted-foreground">{d}</p> })}
      {(!items.is_empty()).then(|| view! {
        <ul class="mt-2 list-disc space-y-1 pl-5 text-muted-foreground">
          {items.into_iter().map(|i| view! { <li>{i}</li> }).collect_view()}
        </ul>
      })}
      <div class="mt-3 flex justify-end gap-2">{children()}</div>
    </div>
  }
}
