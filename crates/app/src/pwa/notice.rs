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
    <div role="status" class=crate::kit::card_class_with("pointer-events-auto p-4 text-sm shadow-lg animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal)")>
      <p>{title}</p>
      {detail.map(|d| view! { <p class="mt-1 text-muted-foreground">{d}</p> })}
      {(!items.is_empty()).then(|| view! {
        <ul class="mt-2 list-disc space-y-1 pl-5 text-muted-foreground">
          // 条目正文来自 `crates/core`（更新日志 / 题库差异说明），是中文原文：
          // 直接 `<li>{i}</li>` 会让 en / es 界面漏出中文，必须走词典。
          {items
            .into_iter()
            .map(|i| {
              view! { <li>{move || crate::data::core_text_tracked(&i)}</li> }
            })
            .collect_view()}
        </ul>
      })}
      <div class="mt-3 flex justify-end gap-2">{children()}</div>
    </div>
  }
}
