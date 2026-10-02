//! 学习报告：汇总学习、错题、成就与通联统计，可导出 PNG 卡片。

use leptos::prelude::*;
use serde::Deserialize;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::{t, tf};
use crate::pages::log::use_log_store;
use crate::share_score::{ReportData, download, render_report_card};
use crate::ui::{Size, Variant, button_class};
use crate::util::{set_title, storage};
use crate::{store, study};

/// 打卡状态（精简）。
#[derive(Deserialize, Default)]
struct Checkin {
  #[serde(default)]
  streak: usize,
}

/// 汇总本地数据，构造成报告数据。
fn build_report_data(entries: &[crate::pages::log::LogEntry]) -> ReportData {
  let stats = study::load_stats();
  let answered = stats.total.answered;
  let correct = stats.total.correct;
  let correct_rate = if answered > 0 {
    (f64::from(correct) / f64::from(answered) * 100.0).round() as u32
  } else {
    0
  };
  let checkin: Checkin = storage::get_json("daily-checkin").unwrap_or_default();
  let mistakes = study::load_book().records.len();
  let bookmarks = store::load_bookmarks().len();
  let achievements = crate::achievements::unlocked_count();
  let log_count = entries.len();
  let dxcc_count = ham_web_core::award_progress::AwardProgress::from_entries(entries)
    .dxcc
    .worked
    .len();

  ReportData {
    answered,
    correct_rate,
    streak: checkin.streak,
    mistakes,
    bookmarks,
    achievements,
    log_count,
    dxcc_count,
    range: "累计至今",
  }
}

#[component]
pub fn ReportPage() -> impl IntoView {
  set_title(&t("学习报告"));

  let log_store = use_log_store();
  let entries = log_store.logbook.get_untracked().entries.clone();
  let data = build_report_data(&entries);
  let exporting = RwSignal::new(false);

  let export = move || {
    exporting.set(true);
    match render_report_card(&data) {
      Ok(data_url) => download(&data_url, "study-report.png"),
      Err(_) => crate::util::alert(&t("生成失败，请重试。")),
    }
    exporting.set(false);
  };

  let stat = |label: &'static str, value: String| {
    view! {
      <div class="rounded-xl border bg-card p-4 text-center">
        <div class="text-2xl font-semibold tabular-nums">{value}</div>
        <div class="mt-0.5 text-xs text-muted-foreground">{move || t(label)}</div>
      </div>
    }
  };

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=Signal::derive(move || t("学习报告"))
        subtitle=Signal::derive(move || t("本地数据汇总 · 可导出 PNG"))
      />
      <PageContainer>
        <div class="grid grid-cols-2 gap-3 sm:grid-cols-3">
          {stat("累计作答", data.answered.to_string())}
          {stat("正确率", tf("{}%", &[&data.correct_rate.to_string()]))}
          {stat("连续打卡", tf("{} 天", &[&data.streak.to_string()]))}
          {stat("当前错题", data.mistakes.to_string())}
          {stat("收藏题目", data.bookmarks.to_string())}
          {stat("解锁成就", data.achievements.to_string())}
        </div>

        <div class="grid grid-cols-2 gap-3">
          {stat("通联记录", data.log_count.to_string())}
          {stat("DXCC 实体", data.dxcc_count.to_string())}
        </div>

        <section class="rounded-xl border bg-card p-4">
          <button
            type="button"
            class=button_class(Variant::Default, Size::Default, "w-full")
            on:click=move |_| export()
          >
            {move || if exporting.get() { t("生成中…") } else { t("导出报告 PNG") }}
          </button>
          <p class="mt-3 text-xs text-muted-foreground">
            {move || t("报告基于本地数据生成，仅保存在浏览器中，不上传任何信息。")}
          </p>
        </section>
      </PageContainer>
    </div>
  }
}
