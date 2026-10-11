//! 专题链：按顺序排成一行的专题胶囊，相邻之间用「→」连接。

use ham_web_core::learning_path::TopicNode;
use leptos::prelude::*;

use crate::data;
use crate::i18n::tp;

/// 专题链。
///
/// - 默认（依赖图）：每个胶囊带「N 个前置」角标。
/// - `numbered = true`（答错回推）：胶囊带序号并高亮，表示建议的复习顺序，不显示前置数。
///
/// 专题名是知识库正文，走 [`data::kt_tracked`]，译文晚到或切语言后会重算。
#[component]
pub fn TopicChips(
  nodes: Vec<&'static TopicNode>,
  #[prop(optional)] numbered: bool,
) -> impl IntoView {
  let total = nodes.len();
  let chip_class = if numbered {
    "rounded-full bg-primary/10 px-3 py-1.5 text-xs font-medium text-primary transition-colors hover:bg-primary/20"
  } else {
    "rounded-full border px-3 py-1.5 text-xs transition-colors hover:bg-accent"
  };
  view! {
    <div class="flex flex-wrap items-center gap-2 p-4">
      {nodes
        .into_iter()
        .enumerate()
        .map(|(i, node)| {
          view! {
            <span class="flex items-center gap-2">
              <a href=node.route class=chip_class>
                {numbered.then(|| format!("{}. ", i + 1))}
                {move || data::kt_tracked(node.name)}
                {(!numbered && !node.prerequisites.is_empty()).then(|| {
                  view! {
                    <span class="ml-1 text-[10px] text-muted-foreground">
                      {move || {
                        tp(
                          "knowledge.n-prerequisites",
                          node.prerequisites.len(),
                          &[&node.prerequisites.len().to_string()],
                        )
                      }}
                    </span>
                  }
                })}
              </a>
              {(i + 1 < total).then(|| {
                view! {
                  <span class="text-xs text-muted-foreground" aria-hidden="true">"→"</span>
                }
              })}
            </span>
          }
        })
        .collect_view()}
    </div>
  }
}
