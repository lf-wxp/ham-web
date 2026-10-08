use ham_web_core::practice::SearchMatch;
use leptos::prelude::*;

mod match_snippet;
use match_snippet::match_snippet;

use crate::i18n::{t, tf, tp};
use crate::ui::{
  Button, ControlSize, Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Input,
  Size, Variant,
};

const SNIPPET_MAX: usize = 120;
const SNIPPET_CONTEXT: usize = 40;

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
        <DialogTitle>{move || t("exam.search-questions")}</DialogTitle>
        <DialogDescription>{move || t("exam.type-a-question-number")}</DialogDescription>
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
          placeholder=Signal::derive(move || t("exam.question-number-or-keyword"))
          clearable=true
          on_enter=Callback::new(move |_| jump())
        />
        {move || {
          has_input()
            .then(|| {
              view! { <div class="text-xs text-muted-foreground">{ move || { let n = matches.with(Vec::len); tp("exam.matches", n, &[&n.to_string()]) } }</div> }
            })
        }}
        <div class="rounded-md border">
          {move || {
            let list = matches.get();
            if list.is_empty() {
              let msg = if has_input() { t("exam.no-matches") } else { t("exam.type-to-search") };
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
                      <span class="ml-auto text-xs text-muted-foreground">{tf("exam.question-2", &[&(pos + 1).to_string()])}</span>
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
        <Button
          variant=Variant::Default
          size=Size::Default
          on_click=Callback::new(move |_| jump())
        >
          {move || t("exam.go")}
        </Button>
      </DialogFooter>
    </Dialog>
  }
}
