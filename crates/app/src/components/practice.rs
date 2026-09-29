//! 练习模式相关组件：恢复进度、设置、搜索对话框。

use ham_web_core::practice::{PracticeOrder, SearchMatch};
use ham_web_core::text::upper_chars;
use leptos::prelude::*;

use crate::components::exam::ShortcutRow;
use crate::icons::{Icon, IconKind};
use crate::ui::{
  Checkbox, Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Label, RadioGroup,
  RadioGroupItem, Separator, Size, Variant, button_class, input_class,
};

#[component]
pub fn PracticeResumeDialog(
  open: RwSignal<bool>,
  no_prompt: RwSignal<bool>,
  on_restart: Callback<()>,
  on_resume: Callback<()>,
) -> impl IntoView {
  view! {
    <Dialog open=open>
      <DialogHeader>
        <DialogTitle>"发现上次练习记录"</DialogTitle>
        <DialogDescription>"是否加载到上次练习的位置，还是重新开始？"</DialogDescription>
      </DialogHeader>
      <div class="py-2">
        <div class="flex items-center gap-2">
          <Checkbox
            id="no-prompt-this-bank"
            checked=no_prompt
            on_change=Callback::new(move |v| no_prompt.set(v))
          />
          <Label r#for="no-prompt-this-bank">"本题库不再提示"</Label>
        </div>
      </div>
      <DialogFooter>
        <button class=button_class(Variant::Outline, Size::Default, "") on:click=move |_| on_restart.run(())>
          "重新开始"
        </button>
        <button class=button_class(Variant::Default, Size::Default, "") on:click=move |_| on_resume.run(())>
          "继续上次"
        </button>
      </DialogFooter>
    </Dialog>
  }
}

#[component]
pub fn PracticeSettingsDialog(
  open: RwSignal<bool>,
  #[prop(into)] order: Signal<PracticeOrder>,
  on_change_order: Callback<PracticeOrder>,
  #[prop(into)] show_answer: Signal<bool>,
  on_toggle_show_answer: Callback<bool>,
  #[prop(into)] show_explanation: Signal<bool>,
  on_toggle_show_explanation: Callback<bool>,
) -> impl IntoView {
  let order_value = Signal::derive(move || order.get().as_str().to_owned());
  let on_order = Callback::new(move |v: String| {
    if let Some(o) = PracticeOrder::parse(&v) {
      on_change_order.run(o);
    }
  });
  view! {
    <Dialog open=open class="sm:max-w-[520px]">
      <DialogHeader>
        <DialogTitle>"设置"</DialogTitle>
        <DialogDescription>"题序、显示答案与快捷键说明"</DialogDescription>
      </DialogHeader>
      <div class="space-y-5">
        <div class="space-y-2">
          <div class="text-sm text-muted-foreground">"顺序/随机"</div>
          <RadioGroup class="flex items-center gap-4" value=order_value on_change=on_order>
            <div class="flex items-center space-x-2">
              <RadioGroupItem value="sequential" id="order-seq" />
              <Label r#for="order-seq">"顺序"</Label>
            </div>
            <div class="flex items-center space-x-2">
              <RadioGroupItem value="random" id="order-rand" />
              <Label r#for="order-rand">"随机"</Label>
            </div>
          </RadioGroup>
        </div>
        <div class="flex items-center gap-2">
          <Checkbox id="show-ans" checked=show_answer on_change=on_toggle_show_answer />
          <Label r#for="show-ans">"显示正确答案"</Label>
        </div>
        <div class="flex items-center gap-2">
          <Checkbox id="show-expl" checked=show_explanation on_change=on_toggle_show_explanation />
          <Label r#for="show-expl">"显示答案解析"</Label>
        </div>
        <Separator />
        <div class="space-y-2 text-sm">
          <div class="text-muted-foreground">"快捷键"</div>
          <ShortcutRow label="上一题 / 下一题" keys="← / →" />
          <ShortcutRow label="选择 / 切换选项（单选/多选）" keys="1-9" />
          <ShortcutRow label="严格选择（多选，仅该项）" keys="Shift 或 Cmd（macOS） + 1-9" />
          <ShortcutRow label="打开搜索（仅顺序模式）" keys="Enter" />
        </div>
      </div>
    </Dialog>
  }
}

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
        <DialogTitle>"搜索题目"</DialogTitle>
        <DialogDescription>"输入题号或关键词（如 LK0501 / 天线），回车或点击跳转"</DialogDescription>
      </DialogHeader>
      <div class="space-y-3 overflow-auto pr-1 min-h-0">
        <div class="relative">
          <input
            id="jump"
            data-slot="input"
            class=input_class("h-11 text-base md:h-9 md:text-sm pr-10")
            placeholder="题号或关键词，如 LK0501 / 天线"
            prop:value=move || input.get()
            on:input=move |e| input.set(event_target_value(&e))
            on:keydown=move |e| {
              if e.key() == "Enter" {
                jump();
              }
            }
          />
          {move || {
            has_input()
              .then(|| {
                view! {
                  <button
                    type="button"
                    aria-label="清除"
                    class="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                    on:click=move |_| input.set(String::new())
                  >
                    <Icon kind=IconKind::X class="h-4 w-4" />
                  </button>
                }
              })
          }}
        </div>
        {move || {
          has_input()
            .then(|| {
              view! { <div class="text-xs text-muted-foreground">"匹配 " {move || matches.with(Vec::len)} " 条"</div> }
            })
        }}
        <div class="rounded-md border">
          {move || {
            let list = matches.get();
            if list.is_empty() {
              let msg = if has_input() { "未找到匹配" } else { "输入以开始搜索" };
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
                      <span class="ml-auto text-xs text-muted-foreground">"第 " {pos + 1} " 题"</span>
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
          "跳转"
        </button>
      </DialogFooter>
    </Dialog>
  }
}
