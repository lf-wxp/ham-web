use ham_web_core::practice::SearchMatch;
use ham_web_core::text::upper_chars;
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::ui::{
  ControlSize, Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Input, Size,
  Variant, button_class,
};

const SNIPPET_MAX: usize = 120;
const SNIPPET_CONTEXT: usize = 40;

/// 搜索结果摘要，高亮命中片段。
fn match_snippet(text: &str, query: &str) -> AnyView {
  let chars: Vec<char> = text.chars().collect();
  let plain = || {
    let head: String = chars.iter().take(SNIPPET_MAX).collect();
    let tail = if chars.len() > SNIPPET_MAX { "…" } else { "" };
    view! { <span class="truncate">{head} {tail}</span> }.into_any()
  };
  let q: Vec<char> = query.chars().collect();
  if q.is_empty() {
    return plain();
  }
  let up = upper_chars(text);
  let Some(idx) = up.windows(q.len()).position(|w| w == q.as_slice()) else {
    return plain();
  };
  let start = idx.saturating_sub(SNIPPET_CONTEXT);
  let end = (idx + q.len() + SNIPPET_CONTEXT).min(chars.len());
  let slice = |a: usize, b: usize| chars[a..b].iter().collect::<String>();
  let prefix = if start > 0 { "…" } else { "" };
  let suffix = if end < chars.len() { "…" } else { "" };
  view! {
    <span class="inline-flex items-center gap-1 max-w-[46ch]">
      <span class="truncate">
        {prefix} {slice(start, idx)}
        <mark class="bg-yellow-200 text-yellow-900 dark:bg-yellow-500/30 dark:text-yellow-100 px-0.5 rounded-sm">{slice(idx, idx + q.len())}</mark>
        {slice(idx + q.len(), end)} {suffix}
      </span>
    </span>
  }
  .into_any()
}

#[component]
pub fn PracticeSearchDialog(
  open: RwSignal<bool>,
  input: RwSignal<String>,
  #[prop(into)] matches: Signal<Vec<SearchMatch>>,
  on_pick: Callback<usize>,
  on_jump: Callback<()>,
) -> impl IntoView {
  let query_upper = move || input.with(|s| s.trim().to_uppercase());
  let has_input = move || input.with(|s| !s.trim().is_empty());
  let jump = move || {
    on_jump.run(());
    open.set(false);
  };

  view! {
    <Dialog open=open class="sm:max-w-[640px]">
      <DialogHeader>
        <DialogTitle>{move || t("搜索题目")}</DialogTitle>
        <DialogDescription>{move || t("输入题号或关键词（如 LK0501 / 天线），回车或点击跳转")}</DialogDescription>
      </DialogHeader>
      // `p-1 -m-1`：滚动容器会裁掉子元素溢出的内容，输入框 3px 的聚焦环必须留出余量；
      // 负外边距把多出来的 4px 抵掉，视觉位置不变。
      <div class="-m-1 min-h-0 space-y-3 overflow-auto p-1">
        <Input
          id="jump"
          value=input
          on_change=Callback::new(move |v: String| input.set(v))
          size=ControlSize::Lg
          class="md:h-9 md:text-sm"
          placeholder=Signal::derive(move || t("题号或关键词，如 LK0501 / 天线"))
          clearable=true
          on_enter=Callback::new(move |_| jump())
        />
        {move || {
          has_input()
            .then(|| {
              view! { <div class="text-xs text-muted-foreground">{move || tf("匹配 {} 条", &[&matches.with(Vec::len).to_string()])}</div> }
            })
        }}
        <div class="rounded-md border">
          {move || {
            let list = matches.get();
            if list.is_empty() {
              let msg = if has_input() { t("未找到匹配") } else { t("输入以开始搜索") };
              return view! { <div class="p-3 text-sm text-muted-foreground">{msg}</div> }.into_any();
            }
            let q = query_upper();
            let items = list
              .into_iter()
              .take(10)
              .map(|m| {
                let pos = m.pos;
                view! {
                  <li
                    class="p-2 cursor-pointer transition-colors hover:bg-accent hover:text-accent-foreground"
                    on:click=move |_| {
                      on_pick.run(pos);
                      open.set(false);
                    }
                  >
                    <div class="flex items-center gap-2 text-sm">
                      <span class="inline-flex items-center px-2 py-0.5 rounded border text-xs bg-muted text-foreground">
                        {m.j}
                      </span>
                      {match_snippet(&m.text, &q)}
                      <span class="ml-auto text-xs text-muted-foreground">{tf("第 {} 题", &[&(pos + 1).to_string()])}</span>
                    </div>
                  </li>
                }
              })
              .collect_view();
            view! { <ul class="divide-y max-h-[40svh] overflow-auto">{items}</ul> }.into_any()
          }}
        </div>
      </div>
      <DialogFooter>
        <button class=button_class(Variant::Default, Size::Default, "") on:click=move |_| jump()>
          {move || t("跳转")}
        </button>
      </DialogFooter>
    </Dialog>
  }
}
