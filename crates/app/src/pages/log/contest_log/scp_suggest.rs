use ham_web_core::scp::{Index, Lookup};
use leptos::prelude::*;

use crate::i18n::tf;

use super::scp_group::ScpGroup;

/// 每组最多列几个候选：再多也看不过来，总数另外写在标题里。
const SHOWN: usize = 12;

/// 呼号补全候选：随着对方呼号的输入，列出库里包含它的呼号、只差一位的相近呼号。
///
/// 候选来自本机（自己的通联日志 + 自己载入的 SCP 文件），见 `ham_web_core::scp`。
/// 点候选就把它填进呼号框。
#[component]
pub(super) fn ScpSuggest(
  call: RwSignal<String>,
  index: Memo<Index>,
  on_pick: Callback<String>,
) -> impl IntoView {
  let lookup = Memo::new(move |_| -> Option<(String, Lookup)> {
    let typed = call.get().trim().to_ascii_uppercase();
    index
      .with(|ix| ix.lookup(&typed, SHOWN))
      .map(|l| (typed, l))
  });

  view! {
    {move || {
      lookup.get().and_then(|(typed, l)| {
        let nothing = !l.exact && l.partial.is_empty() && l.similar.is_empty();
        (!index.with(Index::is_empty)).then(|| {
          view! {
            <div class="mt-2 space-y-2 text-sm" data-slot="scp-suggest">
              <div aria-live="polite">
                {if l.exact {
                  view! {
                    <span class="text-win-text">
                      {tf("contest.scp-exact", &[&typed])}
                    </span>
                  }
                  .into_any()
                } else if nothing {
                  view! {
                    <span class="text-muted-foreground">{tf("contest.scp-not-found", &[&typed])}</span>
                  }
                  .into_any()
                } else {
                  view! { <span class="text-muted-foreground">{tf("contest.scp-not-in-list", &[&typed])}</span> }
                    .into_any()
                }}
              </div>
              <ScpGroup
                heading=tf("contest.scp-partial", &[&typed, &l.partial_total.to_string()])
                calls=l.partial.clone()
                total=l.partial_total
                on_pick=on_pick
              />
              <ScpGroup
                heading=tf("contest.scp-similar", &[&l.similar_total.to_string()])
                calls=l.similar.clone()
                total=l.similar_total
                on_pick=on_pick
              />
            </div>
          }
        })
      })
    }}
  }
}
