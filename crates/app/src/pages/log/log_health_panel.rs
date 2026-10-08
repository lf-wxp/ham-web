//! 日志体检面板：把体检结果摊在日志页上，并提供「不会错」的那几个自动修正。
//!
//! 判定逻辑全在 `ham_web_core::log_health`（可脱离浏览器单测），这里只做三件事：
//! 把「现在几点、本机时区」喂进去、把种类翻成文案、把修正按钮接到日志 store 上。
//!
//! 「一键批量修正」只做三件事：按频率回填波段、把误填的本地时改成 UTC、合并重复通联。
//! 缺网格这类**不给自动修** —— 拿实体中心猜一个网格写进日志等于掺假（统计面板的估算
//! 提示里也一直强调那是估算），所以面板只把它指出来。

use ham_web_core::log_health::{HealthContext, Issue, IssueKind, apply_fix, check};
use leptos::prelude::*;

use crate::data::kt;
use crate::i18n::{t, tf};
use crate::ui::{Button, Size, Variant};

use super::log_helpers::confirm;
use super::use_log_store;

/// 每类问题最多列几条明细（再多只报总数）。
const MAX_ROWS: usize = 5;

/// 本机时区相对 UTC 的偏移（分钟，东八区 = 480）。
///
/// `getTimezoneOffset()` 返回的是「本地比 UTC 晚多少分钟」（东八区是 -480），取负才是偏移。
fn local_offset_minutes() -> i32 {
  -js_sys::Date::new_0().get_timezone_offset() as i32
}

/// 当前 UTC 时间（自 1970-01-01 起经过的分钟数）。
fn now_utc_minutes() -> i64 {
  (js_sys::Date::now() / 60_000.0).floor() as i64
}

/// 种类标题（带条数）。
fn kind_title(kind: IssueKind, count: usize) -> String {
  let n = count.to_string();
  let args: &[&str] = &[&n];
  match kind {
    IssueKind::Duplicate => tf("log.health-duplicate", args),
    IssueKind::FrequencyOutOfBand => tf("log.health-frequency-out-of-band", args),
    IssueKind::BandMismatch => tf("log.health-band-mismatch", args),
    IssueKind::ModeOutOfSegment => tf("log.health-mode-out-of-segment", args),
    IssueKind::TimeInFuture => tf("log.health-time-in-future", args),
    IssueKind::InvalidDateTime => tf("log.health-invalid-datetime", args),
    IssueKind::InvalidGrid => tf("log.health-invalid-grid", args),
    IssueKind::MissingGrid => tf("log.health-missing-grid", args),
    IssueKind::SuspiciousPrefix => tf("log.health-suspicious-prefix", args),
  }
}

/// 补充值：模式子段那一条带的是知识库正文（中文），要先按当前语言翻一遍。
///
/// 形态是 `频率区间 · 子段文案`（见 `log_health::check_entry`，用中点分隔是为了避免
/// 子段文案自带的括号被套两层）。整串查不到译文 —— 词典按中文原文存，而区间是行内
/// 数据 —— 所以拆开只译后半段；其余种类的补充值（原值、推出来的波段、条数）保持原样。
fn localized_extra(extra: &str) -> String {
  match extra.split_once(" · ") {
    Some((range, text)) => format!("{range} · {}", kt(text)),
    None => kt(extra).to_owned(),
  }
}

/// 一条明细：`呼号 · 日期 时间 · 波段 模式`，后面按需接补充值。
fn issue_line(issue: &Issue) -> String {
  let line = tf(
    "log.health-line",
    &[
      &issue.callsign,
      &issue.date,
      &issue.time,
      &issue.band,
      &issue.mode,
    ],
  );
  match issue.extra.as_deref() {
    Some(extra) if issue.kind == IssueKind::Duplicate => tf(
      "log.health-duplicate-detail",
      &[&line, &localized_extra(extra)],
    ),
    Some(extra) => tf("log.health-detail", &[&line, &localized_extra(extra)]),
    None => line,
  }
}

#[component]
pub(super) fn LogHealthPanel() -> impl IntoView {
  let store = use_log_store();
  let logbook = store.logbook;
  // 「话务落在不允许话务的子段」那条明细后半段是知识库正文（IARU 模式子段文案），
  // 因此这里也要确保译文在拉 —— 日志页其余部分都是词典文案，不带这个依赖。
  Effect::new(move |_| crate::data::ensure_knowledge_i18n());

  // 「现在」每次重算都取一次：面板只在日志变化时重算，不会自己定时刷新。
  let report = Memo::new(move |_| {
    let ctx = HealthContext {
      now_utc_minutes: now_utc_minutes(),
      local_offset_minutes: local_offset_minutes(),
    };
    logbook.with(|lb| check(&lb.entries, &ctx))
  });

  // 逐条应用，返回改动数；一条都没改就不落库。
  let run = move |issues: &[Issue]| -> usize {
    let mut changed = 0;
    logbook.update(|lb| {
      for issue in issues {
        changed += apply_fix(&mut lb.entries, issue);
      }
    });
    if changed > 0 {
      store.persist();
    }
    changed
  };

  let fix_kind = Callback::new(move |kind: IssueKind| {
    let issues: Vec<Issue> = report.with_untracked(|r| {
      r.issues
        .iter()
        .filter(|i| i.kind == kind && i.fix.is_some())
        .cloned()
        .collect()
    });
    if issues.is_empty() {
      return;
    }
    // 合并重复项是这里唯一的破坏性操作（会删记录），先问一句。
    if kind == IssueKind::Duplicate
      && !confirm(&tf(
        "log.health-merge-confirm",
        &[&issues.len().to_string()],
      ))
    {
      return;
    }
    let changed = run(&issues);
    crate::util::alert(&tf("log.health-fixed", &[&changed.to_string()]));
  });

  let fix_all = Callback::new(move |()| {
    let issues: Vec<Issue> = report.with_untracked(|r| {
      r.issues
        .iter()
        .filter(|i| i.fix.is_some())
        .cloned()
        .collect()
    });
    if issues.is_empty() {
      return;
    }
    if !confirm(&tf(
      "log.health-fix-all-confirm",
      &[&issues.len().to_string()],
    )) {
      return;
    }
    let changed = run(&issues);
    crate::util::alert(&tf("log.health-fixed", &[&changed.to_string()]));
  });

  view! {
    <section class="rounded-xl border bg-card">
      <div class="flex flex-wrap items-center gap-2 border-b px-4 py-3">
        <h2 class="mr-auto text-sm font-semibold">{move || t("log.health-title")}</h2>
        {move || {
          let r = report.get();
          (!r.is_clean())
            .then(|| {
              view! {
                <span class="text-xs text-muted-foreground">
                  {tf(
                    "log.health-summary",
                    &[&r.issues.len().to_string(), &r.fixable_count().to_string()],
                  )}
                </span>
                <Button
                  variant=Variant::Outline
                  size=Size::Sm
                  disabled=Signal::derive(move || report.get().fixable_count() == 0)
                  on_click=fix_all
                >
                  {move || {
                    tf("log.health-fix-all", &[&report.get().fixable_count().to_string()])
                  }}
                </Button>
              }
            })
        }}
      </div>
      <div class="space-y-3 p-4">
        {move || {
          // 订阅知识库译文的加载状态：明细里可能带 IARU 子段文案，译文到达后要重算
          // 这一列，否则体检先算出来、译文后到，页面就一直停在中文。
          crate::data::track_knowledge();
          let r = report.get();
          if r.is_clean() {
            return view! {
              <p class="text-xs text-muted-foreground">{move || t("log.health-ok")}</p>
            }
              .into_any();
          }
          IssueKind::ALL
            .into_iter()
            .filter(|kind| r.count(*kind) > 0)
            .map(|kind| {
              let issues: Vec<Issue> = r.issues.iter().filter(|i| i.kind == kind).cloned().collect();
              let fixable = r.fixable_of(kind);
              let shown: Vec<String> = issues.iter().take(MAX_ROWS).map(issue_line).collect();
              let rest = issues.len().saturating_sub(shown.len());
              view! {
                <div class="space-y-1">
                  <div class="flex flex-wrap items-center gap-2">
                    <span class="text-xs font-medium">{kind_title(kind, issues.len())}</span>
                    {(fixable > 0)
                      .then(|| {
                        view! {
                          <Button
                            variant=Variant::Ghost
                            size=Size::Sm
                            on_click=Callback::new(move |_| fix_kind.run(kind))
                          >
                            {tf("log.health-fix-kind", &[&fixable.to_string()])}
                          </Button>
                        }
                      })}
                  </div>
                  <ul class="space-y-0.5 text-xs text-muted-foreground">
                    {shown
                      .into_iter()
                      .map(|line| view! { <li>{line}</li> })
                      .collect_view()}
                  </ul>
                  {(rest > 0)
                    .then(|| {
                      view! {
                        <p class="text-xs text-muted-foreground">
                          {tf("log.health-more", &[&rest.to_string()])}
                        </p>
                      }
                    })}
                </div>
              }
            })
            .collect_view()
            .into_any()
        }}
      </div>
    </section>
  }
}
