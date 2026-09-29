//! 模拟考试相关组件：成绩、恢复、设置、交卷确认对话框与答题卡。

use std::collections::HashMap;

use ham_exam_core::ExamScore;
use leptos::prelude::*;

use crate::components::common::PreviewableImage;
use crate::data::Questions;
use crate::ui::{
  BadgeVariant, Checkbox, Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle,
  Label, Separator, Sheet, Size, Variant, badge_class, button_class,
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
      </div>
    </Dialog>
  }
}

#[component]
pub fn ExamResumeDialog(
  open: RwSignal<bool>,
  #[prop(into)] expires_in_ms: Signal<i64>,
  #[prop(into)] answered: Signal<usize>,
  #[prop(into)] total: Signal<usize>,
  on_resume: Callback<()>,
  on_restart: Callback<()>,
) -> impl IntoView {
  let remaining = move || {
    let ms = expires_in_ms.get().max(0);
    format!("{:02}:{:02}", ms / 60_000, (ms % 60_000) / 1000)
  };
  view! {
    <Dialog open=open class="sm:max-w-[520px]">
      <DialogHeader>
        <DialogTitle>"恢复考试"</DialogTitle>
        <DialogDescription>
          "检测到未完成的考试。已答 " {move || answered.get()} " / " {move || total.get()} "，剩余时间约 " {remaining}
          "。"
        </DialogDescription>
      </DialogHeader>
      <div class="flex items-center justify-end gap-2 pt-2">
        <button class=button_class(Variant::Outline, Size::Default, "") on:click=move |_| on_restart.run(())>
          "重新开始"
        </button>
        <button class=button_class(Variant::Default, Size::Default, "") on:click=move |_| on_resume.run(())>
          "继续考试"
        </button>
      </div>
    </Dialog>
  }
}

/// 快捷键说明行。
#[component]
pub fn ShortcutRow(label: &'static str, keys: &'static str) -> impl IntoView {
  view! {
    <div class="flex items-center justify-between">
      <span>{label}</span>
      <code class="px-2 py-0.5 rounded border bg-muted">{keys}</code>
    </div>
  }
}

#[component]
pub fn ExamSettingsDialog(
  open: RwSignal<bool>,
  #[prop(into)] show_explanation: Signal<bool>,
  on_change_show_explanation: Callback<bool>,
) -> impl IntoView {
  view! {
    <Dialog open=open class="sm:max-w-[520px]">
      <DialogHeader>
        <DialogTitle>"设置"</DialogTitle>
        <DialogDescription>"快捷键与考试说明"</DialogDescription>
      </DialogHeader>
      <div class="space-y-5">
        <div class="flex items-center gap-2">
          <Checkbox id="exam-show-expl" checked=show_explanation on_change=on_change_show_explanation />
          <Label r#for="exam-show-expl">"显示答案解析（仅交卷后）"</Label>
        </div>
        <div class="space-y-2 text-sm">
          <div class="text-muted-foreground">"快捷键"</div>
          <ShortcutRow label="上一题 / 下一题" keys="← / →" />
          <ShortcutRow label="选择 / 切换选项（单选/多选）" keys="1-9" />
          <ShortcutRow label="严格选择（多选，仅该项）" keys="Shift 或 Cmd（macOS） + 1-9" />
        </div>
        <Separator />
        <div class="text-xs text-muted-foreground">
          "考试规则：A 类 40 题（单选 32，多选 8），40 分钟，30 题合格；B 类 60 题（单选 45，多选 15），60 分钟，45 题合格；C 类 90 题（单选 70，多选 20），90 分钟，70 题合格。多选题需与标准答案完全一致，否则不得分。"
        </div>
      </div>
    </Dialog>
  }
}

#[component]
pub fn ExamSubmitConfirmDialog(
  open: RwSignal<bool>,
  on_confirm: Callback<()>,
  #[prop(into)] total: Signal<usize>,
  #[prop(into)] answered: Signal<usize>,
  #[prop(into)] flagged: Signal<usize>,
) -> impl IntoView {
  let unanswered = move || total.get().saturating_sub(answered.get());
  view! {
    <Dialog open=open>
      <DialogHeader>
        <DialogTitle>"确认交卷？"</DialogTitle>
        <DialogDescription>"交卷后将停止计时，答案将不可修改，但可以浏览查看正确答案与成绩。"</DialogDescription>
      </DialogHeader>
      <div class="space-y-2 text-sm">
        <div class="flex items-center justify-between">
          <span>"已作答"</span>
          <span>{move || answered.get()} " / " {move || total.get()}</span>
        </div>
        <div class="flex items-center justify-between">
          <span>"未作答"</span>
          <span class=move || (unanswered() > 0).then_some("text-red-600 dark:text-red-400")>{unanswered}</span>
        </div>
        <div class="flex items-center justify-between">
          <span>"已标记"</span>
          <span>{move || flagged.get()}</span>
        </div>
      </div>
      <DialogFooter>
        <button class=button_class(Variant::Outline, Size::Default, "") on:click=move |_| open.set(false)>
          "取消"
        </button>
        <button class=button_class(Variant::Destructive, Size::Default, "") on:click=move |_| on_confirm.run(())>
          "确认交卷"
        </button>
      </DialogFooter>
    </Dialog>
  }
}

/// 答题卡筛选。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AnswerCardFilter {
  #[default]
  All,
  Unanswered,
  Flagged,
}

impl AnswerCardFilter {
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::All => "all",
      Self::Unanswered => "unanswered",
      Self::Flagged => "flagged",
    }
  }

  pub fn parse(s: &str) -> Option<Self> {
    match s {
      "all" => Some(Self::All),
      "unanswered" => Some(Self::Unanswered),
      "flagged" => Some(Self::Flagged),
      _ => None,
    }
  }
}

#[component]
pub fn AnswerCardSheet(
  open: RwSignal<bool>,
  #[prop(into)] questions: Signal<Questions>,
  #[prop(into)] answers: Signal<HashMap<String, Vec<String>>>,
  #[prop(into)] flags: Signal<HashMap<String, bool>>,
  #[prop(into)] finished: Signal<bool>,
  #[prop(into)] filter: Signal<AnswerCardFilter>,
  on_change_filter: Callback<AnswerCardFilter>,
  on_jump: Callback<usize>,
  #[prop(into)] current_index: Signal<usize>,
) -> impl IntoView {
  let is_answered = move |key: &str| answers.with(|a| a.get(key).is_some_and(|v| !v.is_empty()));
  let is_flagged = move |key: &str| flags.with(|f| f.get(key).copied().unwrap_or(false));
  let answered_count = move || answers.with(|a| a.values().filter(|v| !v.is_empty()).count());
  let flagged_count = move || flags.with(|f| f.values().filter(|v| **v).count());

  let find = move |from: Option<usize>, pred: &dyn Fn(&str) -> bool| -> Option<usize> {
    let start = from.map_or(0, |f| f + 1);
    questions.with(|qs| (start..qs.len()).find(|&i| pred(&qs[i].answer_key(i))))
  };
  let jump = move |target: Option<usize>| {
    if let Some(i) = target {
      on_jump.run(i);
      open.set(false);
    }
  };

  let filter_button = move |f: AnswerCardFilter, label: &'static str| {
    view! {
      <button
        class=move || {
          button_class(if filter.get() == f { Variant::Default } else { Variant::Outline }, Size::Sm, "")
        }
        on:click=move |_| on_change_filter.run(f)
      >
        {label}
      </button>
    }
  };

  let grid = move || {
    let fin = finished.get();
    let filt = filter.get();
    questions.with(|qs| {
      qs.iter()
        .enumerate()
        .filter_map(|(i, q)| {
          let key = q.answer_key(i);
          let user = answers.with(|a| a.get(&key).cloned().unwrap_or_default());
          let answered = !user.is_empty();
          let flagged = is_flagged(&key);
          let visible = match filt {
            AnswerCardFilter::All => true,
            AnswerCardFilter::Unanswered => !answered,
            AnswerCardFilter::Flagged => flagged,
          };
          if !visible {
            return None;
          }
          let correct = q.is_answer_correct(&user);
          let btn_class = if answered {
            "relative h-9 w-full rounded-md border text-sm font-medium transition-colors bg-primary text-primary-foreground"
          } else {
            "relative h-9 w-full rounded-md border text-sm font-medium transition-colors bg-muted text-foreground"
          };
          let image = q.image().map(|src| {
            let label = q.j_code().map_or_else(|| "题目附图".to_owned(), |j| format!("题号 {j} 题图"));
            view! { <PreviewableImage src=src.to_owned() alt=label.clone() title=label small_trigger=true /> }
          });
          Some(view! {
            <div class="relative">
              <button
                class=btn_class
                on:click=move |_| {
                  on_jump.run(i);
                  open.set(false);
                }
              >
                <span>{i + 1}</span>
                {flagged
                  .then(|| {
                    view! { <span class="absolute -top-1 -right-1 inline-block size-3 rounded-full bg-yellow-400"></span> }
                  })}
                {fin
                  .then(|| {
                    let extra = if correct {
                      "absolute -bottom-1 -right-1 px-1 py-0 text-[10px] bg-green-600 text-white"
                    } else {
                      "absolute -bottom-1 -right-1 px-1 py-0 text-[10px] bg-red-600 text-white"
                    };
                    view! {
                      <span data-slot="badge" class=badge_class(BadgeVariant::Secondary, extra)>
                        {if correct { "✓" } else { "✗" }}
                      </span>
                    }
                  })}
              </button>
              {image}
            </div>
          })
        })
        .collect_view()
    })
  };

  view! {
    <Sheet open=open>
      <div data-slot="sheet-header" class="flex flex-col gap-1.5 p-4">
        <h2 data-slot="sheet-title" class="text-foreground font-semibold">"答题卡"</h2>
      </div>
      <div class="px-4 space-y-3 overflow-y-auto">
        <div class="flex items-center justify-between gap-3 text-sm">
          <div class="text-muted-foreground">
            "已答 " {answered_count} " / " {move || questions.with(|q| q.len())} "｜标记 " {flagged_count}
          </div>
          <div class="flex items-center gap-2">
            {filter_button(AnswerCardFilter::All, "全部")}
            {filter_button(AnswerCardFilter::Unanswered, "未答")}
            {filter_button(AnswerCardFilter::Flagged, "标记")}
          </div>
        </div>
        <Separator />
        <div class="flex items-center justify-between flex-wrap gap-2">
          <div class="text-sm text-muted-foreground">"点击题号跳转"</div>
          <div class="flex items-center gap-2">
            <button
              class=button_class(Variant::Outline, Size::Sm, "")
              on:click=move |_| jump(find(None, &|k| !is_answered(k)))
            >
              "首个未答"
            </button>
            <button
              class=button_class(Variant::Outline, Size::Sm, "")
              on:click=move |_| jump(find(Some(current_index.get_untracked()), &|k| !is_answered(k)))
            >
              "下一个未答"
            </button>
            <button
              class=button_class(Variant::Outline, Size::Sm, "")
              on:click=move |_| jump(find(Some(current_index.get_untracked()), &|k| is_flagged(k)))
            >
              "下一个标记"
            </button>
          </div>
        </div>
        <div class="grid grid-cols-6 gap-2 sm:grid-cols-8">{grid}</div>
        {move || {
          finished
            .get()
            .then(|| {
              view! {
                <div class="pt-2 text-xs text-muted-foreground flex items-center gap-3">
                  <span class="inline-flex items-center gap-1">
                    <span class="inline-block size-2 rounded-full bg-green-600"></span>
                    " 正确"
                  </span>
                  <span class="inline-flex items-center gap-1">
                    <span class="inline-block size-2 rounded-full bg-red-600"></span>
                    " 错误"
                  </span>
                  <span class="inline-flex items-center gap-1">
                    <span class="inline-block size-2 rounded-full bg-yellow-400"></span>
                    " 已标记"
                  </span>
                </div>
              }
            })
        }}
      </div>
    </Sheet>
  }
}
