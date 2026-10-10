//! 成就墙：集中展示全部里程碑成就，已解锁高亮、未解锁灰显，并给出解锁进度。

use std::collections::HashSet;

use ham_web_core::achievements::{ACHIEVEMENTS, AchStats, unlocked};
use ham_web_core::most_wanted::{WANTED_ENTITIES, wanted_prefix};
use ham_web_core::{Bank, ExamRule};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::components::common::{PageContainer, PageHeader};
use crate::exam_history;
use crate::i18n::{t, tf};
use crate::icons::{PixelSprite, badge_sprite};
use crate::pages::log::use_log_store;
use crate::ui::Progress;
use crate::util::{set_title, storage};
use crate::{data, store, study};

/// 打卡状态（精简）。
#[derive(Deserialize, Clone, Default)]
struct CheckinLite {
  #[serde(default)]
  streak: usize,
}

#[component]
pub fn AchievementsPage() -> impl IntoView {
  set_title("learning.achievement-wall");

  let checkin: CheckinLite = storage::get_json("daily-checkin").unwrap_or_default();
  let bookmarks = store::load_bookmarks().len();
  let entries = use_log_store().logbook.get_untracked().entries;
  let log_count = entries.len();
  let logged_dxcc: HashSet<String> = entries
    .iter()
    .filter_map(|e| wanted_prefix(&e.callsign).map(str::to_owned))
    .collect();
  let done: Vec<String> = storage::get_json("dxcc_wanted_done").unwrap_or_default();
  let dxcc_done = WANTED_ENTITIES
    .iter()
    .filter(|(p, _, _)| done.iter().any(|x| x.as_str() == *p) || logged_dxcc.contains(*p))
    .count();
  let grids: HashSet<String> = entries
    .iter()
    .filter_map(|e| {
      if e.gridsquare.is_empty() {
        None
      } else {
        Some(e.gridsquare.clone())
      }
    })
    .collect();
  let contest_count = entries.iter().filter(|e| !e.contest_id.is_empty()).count();

  let study_stats = StoredValue::new(study::load_stats());
  let exam_records = StoredValue::new(exam_history::load());

  let coverage = RwSignal::new(Vec::<(Bank, usize, usize)>::new());
  spawn_local(async move {
    let mut out = Vec::new();
    for b in Bank::ALL {
      if let Ok(qs) = data::load_bank(None, b, false).await {
        let covered = study_stats
          .try_with_value(|s| s.bank(b).map_or(0, |bs| bs.covered(qs.iter())))
          .unwrap_or(0);
        out.push((b, covered, qs.len()));
      }
    }
    coverage.try_set(out);
  });

  let unlocked_ids = Memo::new(move |_| {
    let cov = coverage.get();
    let covs: Vec<f64> = cov
      .iter()
      .map(|(_, c, t)| *c as f64 / (*t).max(1) as f64)
      .collect();
    let passed = exam_records.with_value(|h| {
      h.iter()
        .any(|r| !r.weak && r.correct >= ExamRule::of(Bank::from_param(Some(&r.bank))).pass)
    });
    let perfect = exam_records.with_value(|h| {
      h.iter()
        .any(|r| !r.weak && r.total > 0 && r.correct == r.total)
    });
    let challenge_days =
      storage::get_json::<ham_web_core::daily_challenge::DailyResults>("daily-challenge")
        .map_or(0, |r| r.days.len());
    let s = AchStats {
      total_correct: study_stats.with_value(|s| s.total.correct),
      coverages: &covs,
      streak: checkin.streak,
      exam_passed: passed,
      exam_perfect: perfect,
      challenge_count: challenge_days,
      dxcc_count: dxcc_done,
      log_count,
      grid_count: grids.len(),
      contest_count,
      bookmarks,
    };
    unlocked(&s)
  });

  let total = ACHIEVEMENTS.len();

  // 40+ 枚成就一次性铺满视口时没有节奏可言：让格子随滚动逐个浮现，
  // 滚动本身就是「翻成就墙」的动作。
  let grid = NodeRef::<leptos::html::Div>::new();
  grid.on_load(|el| crate::motion::reveal_children(&el));

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("learning.achievement-wall")
        subtitle=move || t("learning.unlock-milestones-as-you")
        actions=ViewFn::from(move || {
          view! {
            <a href="/progress" class="text-xs text-primary underline-offset-4 hover:underline">
              {move || t("learning.progress")}
            </a>
          }
        })
      />

      <PageContainer class="space-y-5">
        <section class="rounded-xl border bg-card p-4">
          <div class="mb-1 flex items-center justify-between text-sm">
            <span class="font-medium">{move || t("learning.achievements-unlocked")}</span>
            <span class="tabular-nums text-muted-foreground">
              {move || tf("common.ratio", &[&unlocked_ids.with(Vec::len).to_string(), &total.to_string()])}
            </span>
          </div>
          <Progress
            value=Signal::derive(move || {
              i64::try_from(unlocked_ids.with(Vec::len) * 100 / total.max(1)).unwrap_or(100)
            })
            label=Signal::derive(move || t("learning.achievements-unlocked"))
            class="pxl-bar-gold"
          />
        </section>

        <div node_ref=grid class="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4">
          {ACHIEVEMENTS
            .iter()
            .map(|a| {
              let is_unlocked = move || unlocked_ids.with(|ids| ids.contains(&a.id));
              view! {
                <div class=move || if is_unlocked() {
                  "pxl-window p-4 text-center"
                } else {
                  "border-2 border-dashed border-border p-4 text-center text-muted-foreground"
                }>
                  // 未解锁显示锁：与地图、图鉴里「还没到」的语言一致，而不是把彩色徽章灰掉。
                  <div class="flex justify-center">
                    <PixelSprite
                      name=Signal::derive(move || {
                        if is_unlocked() { badge_sprite(a.id) } else { "badge_lock" }
                      })
                      scale=4
                      class=Signal::derive(move || {
                        if is_unlocked() { "pxl-bob".to_owned() } else { "opacity-60".to_owned() }
                      })
                    />
                  </div>
                  <div class="mt-2 text-sm">{move || t(a.name)}</div>
                  <div class="mt-1 text-xs text-muted-foreground">{move || t(a.desc)}</div>
                  <div class="mt-1 text-xs">
                    {move || if is_unlocked() {
                      view! { <span class="text-win-text">{t("learning.unlocked")}</span> }.into_any()
                    } else {
                      view! { <span>{t("learning.locked")}</span> }.into_any()
                    }}
                  </div>
                </div>
              }
            })
            .collect_view()}
        </div>

        <p class="text-xs text-muted-foreground">
          {move || t("learning.achievements-are-determined-from")}
        </p>
      </PageContainer>
    </div>
  }
}
