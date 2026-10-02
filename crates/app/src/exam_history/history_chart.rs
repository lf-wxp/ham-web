use ham_web_core::exam_history::ExamRecord;
use leptos::prelude::*;

use super::load;
use crate::i18n::tf;

/// 历史成绩趋势（最近 10 次常规模考的正确率柱状图），不足 2 次不展示。
#[component]
pub fn HistoryChart() -> impl IntoView {
  let history = load();
  let mut recent: Vec<ExamRecord> = history.into_iter().filter(|r| !r.weak).collect();
  let skip = recent.len().saturating_sub(10);
  recent.drain(..skip);
  if recent.len() < 2 {
    return view! { <div></div> }.into_any();
  }

  view! {
    <div class="mt-4 border-t pt-3">
      <div class="text-sm font-medium">"历史成绩（最近 " {recent.len()} " 次正确率）"</div>
      <div class="mt-3 flex items-end gap-1.5">
        {recent
          .iter()
          .map(|r| {
            let pct = r.percent();
            let h = (pct / 100.0 * 88.0).max(6.0);
            view! {
              <div
                class="flex flex-1 flex-col items-center gap-1"
                title=tf("{} 类：{}/{}（{}%）", &[&(r.bank).to_string(), &(r.correct).to_string(), &(r.total).to_string(), &(pct.round() as i64).to_string()])
              >
                <div class="w-full rounded-t bg-primary" style=format!("height: {h:.1}px")></div>
                <span class="text-[10px] tabular-nums text-muted-foreground">{pct.round() as i64}</span>
              </div>
            }
          })
          .collect_view()}
      </div>
    </div>
  }
  .into_any()
}
