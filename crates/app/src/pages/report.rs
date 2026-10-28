//! 学习报告：汇总学习、错题、成就与通联统计，可导出 PNG 卡片。

use leptos::prelude::*;
use serde::Deserialize;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::{t, tf, tp};
use crate::pages::log::{StatsScopeSelect, scoped_log_entries, use_stats_scope};
use crate::share_score::{ReportData, download, render_report_card};
use crate::ui::{Button, Size, Variant};
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
    range: "learning.all-time",
  }
}

#[component]
pub fn ReportPage() -> impl IntoView {
  set_title("learning.study-report");

  // 通联两项按统计口径算（与日志页 / `/stats` / `/dxcc-map` 共用同一个口径）；
  // 导出的卡片与页面上看到的是同一组数字。
  let scope = use_stats_scope();
  let scoped = scoped_log_entries(scope);
  let report = Memo::new(move |_| scoped.with(|e| build_report_data(e)));
  let data = report.get_untracked();

  let export = move || {
    // 卡片渲染是同步的（几毫秒的 canvas 绘制）：原来「生成中」状态在同一回调里
    // 置位又复位，按钮永远等不到它，是个死状态，这里直接去掉。
    match render_report_card(&report.get_untracked()) {
      Ok(data_url) => download(&data_url, "study-report.png"),
      Err(_) => crate::ui::push_error(t("log.rendering-failed-please-retry")),
    }
  };

  let stat = |label: &'static str, value: String| {
    view! {
      <div class="pxl-card border bg-card p-4 text-center">
        <div class="pxl-stat">{value}</div>
        <div class="mt-0.5 text-xs text-muted-foreground">{move || t(label)}</div>
      </div>
    }
  };

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=Signal::derive(move || t("learning.study-report"))
        subtitle=Signal::derive(move || t("learning.local-data-summary-export"))
      />
      <PageContainer>
        <div class="grid grid-cols-2 gap-3 sm:grid-cols-3">
          {stat("learning.answered-total", data.answered.to_string())}
          {move || stat("learning.accuracy", tf("common.percent", &[&data.correct_rate.to_string()]))}
          {move || stat("learning.day-streak", tp("common.days", data.streak, &[&data.streak.to_string()]))}
          {stat("exam.current-mistakes", data.mistakes.to_string())}
          {stat("learning.bookmarks", data.bookmarks.to_string())}
          {stat("exam.achievement-unlocked", data.achievements.to_string())}
        </div>

        <StatsScopeSelect scope=scope count=Signal::derive(move || scoped.with(Vec::len)) />
        <div class="grid grid-cols-2 gap-3">
          {move || stat("learning.qsos", report.get().log_count.to_string())}
          {move || stat("log.dxcc-entities", report.get().dxcc_count.to_string())}
        </div>

        <section class="pxl-card border bg-card p-4">
          <Button
            variant=Variant::Default
            size=Size::Default
            class="w-full"
            on_click=Callback::new(move |_| export())
          >
            {move || t("learning.export-report-png")}
          </Button>
          <p class="mt-3 text-xs text-muted-foreground">
            {move || t("learning.the-report-is-generated")}
          </p>
        </section>
      </PageContainer>
    </div>
  }
}
