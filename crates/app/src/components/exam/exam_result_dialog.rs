use ham_web_core::weak_exam::CategoryDelta;
use ham_web_core::{Bank, ExamScore};
use leptos::prelude::*;

use crate::exam_history::HistoryChart;
use crate::share_score;
use crate::ui::{
  Button, ButtonLink, Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Size,
  Variant,
};

use super::boss_verdict::BossVerdict;
use super::category_compare::CategoryCompare;
use crate::i18n::{t, tf, tp};

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
          Ok(()) => crate::util::alert(&t("learning.copied-to-clipboard")),
          Err(e) => crate::util::alert(&tf("learning.copy-failed", &[&e])),
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
      let title = t("exam.amateur-radio-mock-exam");
      leptos::task::spawn_local(async move {
        if let Err(e) = share_score::share_image(&data_url, &filename, &title).await {
          crate::util::alert(&tf("learning.share-failed", &[&e]));
        }
      });
    }
  };
  view! {
    <Dialog open=open>
      <DialogHeader>
        <DialogTitle>{move || t("exam.score-2")}</DialogTitle>
        <DialogDescription class="sr-only">{move || t("exam.exam-result-details-and")}</DialogDescription>
      </DialogHeader>
      // 内容超过 60svh 时这里会滚动：滚动区必须能被键盘聚焦（`tabindex=0`），否则没有鼠标的用户
        // 滚不动它（axe 的 `scrollable-region-focusable`）。Boss 判词加进来之后这个区域才开始溢出。
        <div class="max-h-[60svh] space-y-2 overflow-y-auto" tabindex="0">
        <BossVerdict score=score pass_line=pass_line weak=weak />
        <div>{move || t("exam.score")} {move || score.get().correct} " / " {move || score.get().total}</div>
        <div class="text-sm text-muted-foreground">{move || t("exam.accuracy")} {move || score.get().percent()} "%"</div>
        {move || {
          let pass_line = pass_line.get();
          let passed = score.get().is_passed(pass_line);
          let class = if passed { "text-sm text-green-700 dark:text-green-400" } else { "text-sm text-red-700 dark:text-red-400" };
          view! {
            <div class=class>{if passed { t("exam.passed") } else { t("exam.not-passed") }} {tp("exam.pass-line-questions", pass_line, &[&pass_line.to_string()])}</div>
          }
        }}
        {move || weak.get().then(|| view! {
          <div class="text-xs text-muted-foreground">{move || t("exam.a-weak-area-exam")}</div>
        })}
        <div class="text-xs text-muted-foreground">{move || t("exam.after-submitting-you-can")}</div>
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
              <ButtonLink
                href=wrong_href.get()
                variant=Variant::Secondary
                size=Size::Default
              >
                {tp("exam.retry-mistakes", wrong_count.get(), &[&wrong_count.get().to_string()])}
              </ButtonLink>
            }
          })
        }}
        // 交卷弹窗关掉后，逐题对错原本就再也看不到了，这里给出复盘页入口。
        <ButtonLink
          href="/exam-review"
          variant=Variant::Secondary
          size=Size::Default
        >
          {move || t("exam.review-question-by-question")}
        </ButtonLink>
        <Button
          variant=Variant::Outline
          size=Size::Default
          on_click=Callback::new(move |_| share(()))
        >
          {move || t("learning.download-card")}
        </Button>
        <Button
          variant=Variant::Outline
          size=Size::Default
          on_click=Callback::new(move |_| copy(()))
        >
          {move || t("learning.copy-image")}
        </Button>
        <Button
          variant=Variant::Outline
          size=Size::Default
          on_click=Callback::new(move |_| sys_share(()))
        >
          {move || t("learning.system-share")}
        </Button>
        <Button
          variant=Variant::Default
          size=Size::Default
          on_click=Callback::new(move |_| open.set(false))
        >
          {move || t("exam.keep-browsing")}
        </Button>
      </DialogFooter>
    </Dialog>
  }
}
