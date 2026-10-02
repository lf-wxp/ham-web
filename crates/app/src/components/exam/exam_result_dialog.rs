use ham_web_core::weak_exam::CategoryDelta;
use ham_web_core::{Bank, ExamScore};
use leptos::prelude::*;

use crate::exam_history::HistoryChart;
use crate::share_score;
use crate::ui::{
  Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Size, Variant, button_class,
};

use super::category_compare::CategoryCompare;
use crate::i18n::{t, tf};

#[component]
pub fn ExamResultDialog(
  open: RwSignal<bool>,
  #[prop(into)] score: Signal<ExamScore>,
  #[prop(into)] pass_line: Signal<usize>,
  #[prop(into)] deltas: Signal<Vec<CategoryDelta>>,
  #[prop(into)] weak: Signal<bool>,
  #[prop(into)] wrong_count: Signal<usize>,
  #[prop(into)] wrong_href: Signal<String>,
  #[prop(into)] bank: Signal<Bank>,
) -> impl IntoView {
  let card = move || {
    let (s, b, pl) = (score.get(), bank.get(), pass_line.get());
    share_score::render_score_card(b, s, pl).ok()
  };
  let share = move |_| {
    if let Some(data_url) = card() {
      share_score::download(
        &data_url,
        &format!(
          "ham-exam-{}-{}.png",
          bank.get().as_str(),
          crate::util::local_today()
        ),
      );
    }
  };
  let copy = move |_| {
    if let Some(data_url) = card() {
      leptos::task::spawn_local(async move {
        match share_score::copy_image(&data_url).await {
          Ok(()) => crate::util::alert(&t("已复制到剪贴板")),
          Err(e) => crate::util::alert(&tf("复制失败：{}", &[&e])),
        }
      });
    }
  };
  let sys_share = move |_| {
    if let Some(data_url) = card() {
      let filename = format!(
        "ham-exam-{}-{}.png",
        bank.get().as_str(),
        crate::util::local_today()
      );
      let title = t("业余无线电模拟考试");
      leptos::task::spawn_local(async move {
        if let Err(e) = share_score::share_image(&data_url, &filename, &title).await {
          crate::util::alert(&tf("分享失败：{}", &[&e]));
        }
      });
    }
  };
  view! {
    <Dialog open=open>
      <DialogHeader>
        <DialogTitle>{move || t("成绩")}</DialogTitle>
        <DialogDescription class="sr-only">{move || t("考试成绩详情与是否通过")}</DialogDescription>
      </DialogHeader>
      <div class="max-h-[60svh] space-y-2 overflow-y-auto">
        <div>{move || t("得分：")} {move || score.get().correct} " / " {move || score.get().total}</div>
        <div class="text-sm text-muted-foreground">{move || t("正确率：")} {move || score.get().percent()} "%"</div>
        {move || {
          let pass_line = pass_line.get();
          let passed = score.get().is_passed(pass_line);
          let class = if passed { "text-sm text-green-700 dark:text-green-400" } else { "text-sm text-red-700 dark:text-red-400" };
          view! {
            <div class=class>{if passed { t("合格") } else { t("不合格") }} {tf("（合格线：{} 题）", &[&pass_line.to_string()])}</div>
          }
        }}
        {move || weak.get().then(|| view! {
          <div class="text-xs text-muted-foreground">{move || t("薄弱项组卷偏重你的弱项，成绩不计入备考状态与历史趋势。")}</div>
        })}
        <div class="text-xs text-muted-foreground">{move || t("交卷后可继续浏览题目查看答案。")}</div>
        {move || {
          let d = deltas.get();
          (!d.is_empty()).then(|| view! { <CategoryCompare deltas=d /> })
        }}
        {move || (open.get() && !weak.get()).then(|| view! { <HistoryChart /> })}
      </div>
      <DialogFooter>
        {move || {
          (wrong_count.get() > 0).then(|| {
            view! {
              <a
                class=button_class(Variant::Secondary, Size::Default, "")
                href=wrong_href.get()
              >
                {tf("重练 {} 道错题", &[&wrong_count.get().to_string()])}
              </a>
            }
          })
        }}
        // 交卷弹窗关掉后，逐题对错原本就再也看不到了，这里给出复盘页入口。
        <a class=button_class(Variant::Secondary, Size::Default, "") href="/exam-review">
          {move || t("逐题复盘")}
        </a>
        <button
          type="button"
          class=button_class(Variant::Outline, Size::Default, "")
          on:click=share
        >
          {move || t("下载卡片")}
        </button>
        <button
          type="button"
          class=button_class(Variant::Outline, Size::Default, "")
          on:click=copy
        >
          {move || t("复制图片")}
        </button>
        <button
          type="button"
          class=button_class(Variant::Outline, Size::Default, "")
          on:click=sys_share
        >
          {move || t("系统分享")}
        </button>
        <button
          class=button_class(Variant::Default, Size::Default, "")
          on:click=move |_| open.set(false)
        >
          {move || t("继续浏览题目")}
        </button>
      </DialogFooter>
    </Dialog>
  }
}
