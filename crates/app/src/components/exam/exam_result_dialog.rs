use ham_web_core::ExamScore;
use leptos::prelude::*;

use crate::exam_history::HistoryChart;
use crate::ui::{
  Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Size, Variant, button_class,
};

#[component]
pub fn ExamResultDialog(
  open: RwSignal<bool>,
  #[prop(into)] score: Signal<ExamScore>,
  #[prop(into)] pass_line: Signal<usize>,
) -> impl IntoView {
  view! {
    <Dialog open=open>
      <DialogHeader>
        <DialogTitle>"成绩"</DialogTitle>
        <DialogDescription class="sr-only">"考试成绩详情与是否通过"</DialogDescription>
      </DialogHeader>
      <div class="space-y-2">
        <div>"得分：" {move || score.get().correct} " / " {move || score.get().total}</div>
        <div class="text-sm text-muted-foreground">"正确率：" {move || score.get().percent()} "%"</div>
        {move || {
          let pass_line = pass_line.get();
          let passed = score.get().is_passed(pass_line);
          let class = if passed { "text-sm text-green-600 dark:text-green-400" } else { "text-sm text-red-600 dark:text-red-400" };
          view! {
            <div class=class>{if passed { "合格" } else { "不合格" }} "（合格线：" {pass_line} " 题）"</div>
          }
        }}
        <div class="text-xs text-muted-foreground">"交卷后可继续浏览题目查看答案。"</div>
        {move || open.get().then(|| view! { <HistoryChart /> })}
      </div>
      <DialogFooter>
        <button
          class=button_class(Variant::Default, Size::Default, "")
          on:click=move |_| open.set(false)
        >
          "继续浏览题目"
        </button>
      </DialogFooter>
    </Dialog>
  }
}
