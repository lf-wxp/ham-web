//! 模拟考试成绩历史：本地存储与趋势展示。

use ham_web_core::{Bank, ExamScore};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::util::{now_ms, storage};

/// 一条考试成绩记录。
#[derive(Serialize, Deserialize, Clone)]
struct ExamRecord {
  bank: String,
  correct: usize,
  total: usize,
  timestamp: i64,
}

const KEY: &str = "exam-history";
const MAX: usize = 50;

/// 保存一次考试成绩（最多保留最近 50 次）。
pub fn save(bank: Bank, score: ExamScore) {
  let mut history: Vec<ExamRecord> = storage::get_json(KEY).unwrap_or_default();
  history.push(ExamRecord {
    bank: bank.as_str().to_owned(),
    correct: score.correct,
    total: score.total,
    timestamp: now_ms(),
  });
  if history.len() > MAX {
    let drain = history.len() - MAX;
    history.drain(..drain);
  }
  storage::set_json(KEY, &history);
}

fn load() -> Vec<ExamRecord> {
  storage::get_json(KEY).unwrap_or_default()
}

/// 历史成绩趋势（最近 10 次的正确率柱状图），不足 2 次不展示。
#[component]
pub fn HistoryChart() -> impl IntoView {
  let history = load();
  let recent: Vec<ExamRecord> = history.into_iter().rev().take(10).rev().collect();
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
            let pct = if r.total == 0 {
              0.0
            } else {
              r.correct as f64 / r.total as f64 * 100.0
            };
            let h = (pct / 100.0 * 88.0).max(6.0);
            view! {
              <div
                class="flex flex-1 flex-col items-center gap-1"
                title=format!("{} 类：{}/{}（{}%）", r.bank, r.correct, r.total, pct.round() as i64)
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
