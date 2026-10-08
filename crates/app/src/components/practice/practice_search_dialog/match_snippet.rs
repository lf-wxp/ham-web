//! `match_snippet`：从 `practice_search_dialog.rs` 拆出的视图构造函数（一个组件一个文件）。

use ham_web_core::text::upper_chars;
use leptos::prelude::*;

use super::{SNIPPET_CONTEXT, SNIPPET_MAX};

/// 搜索结果摘要，高亮命中片段。
pub(super) fn match_snippet(text: &str, query: &str) -> AnyView {
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
