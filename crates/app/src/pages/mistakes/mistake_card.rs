use std::collections::HashSet;

use crate::i18n::{t, tf, tp};
use ham_web_core::Bank;
use ham_web_core::categories::{sub_category, top_pages};
use ham_web_core::explanation_index::ExplanationIndex;
use ham_web_core::mistake_book::{MistakeRecord, WRONG_CAUSES};
use leptos::prelude::*;

use crate::data;
use crate::pages::browse::QuestionRow;
use crate::ui::ChipToggle;

const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// 一道错题视图。
///
/// 题目本身（题号 / 分类标签、题干、选项、正确答案、解析、知识点、参考依据）直接用分类浏览的
/// `QuestionRow`，与收藏集同一张卡，解析默认铺开；本组件只在卡片末尾追加错题独有的信息：
/// 我的答案、考点直达、复习进度、错因自评与移出。
///
/// `fresh` 是题库当前版本的解析索引：快照里的解析是答错当时的，展示时经它换成最新的，
/// 索引到达只换卡片里的文字，整张卡不会重建。
#[component]
pub(super) fn MistakeCard(
  record: MistakeRecord,
  now_ms: i64,
  fresh: ReadSignal<ExplanationIndex>,
  bookmarks: RwSignal<HashSet<String>>,
  on_remove: Callback<String>,
) -> impl IntoView {
  let q = &record.question;
  let sub = q.p_code().and_then(sub_category);
  let topic = sub.map_or("未分类", |s| s.name);
  // 考点对应的一级分类专题页（取首个），用于点击直达补知识点。
  let topic_href = sub
    .and_then(|s| top_pages(s.top).first())
    .map(|&(href, _)| href);
  // 该一级分类的正确率（用于展示薄弱程度）。
  // 只读共享的那一份：每张卡都要读，克隆整份统计（含「做过的题」集合）×40 张卡不值得。
  let topic_rate = sub.and_then(|s| {
    crate::study::stats_shared()?
      .total
      .categories
      .get(s.top)
      .and_then(|t| t.rate())
      .map(|r| (r * 100.0).round() as u32)
  });
  // 同类题再练：按二级分类（考点）筛选该题库下的同类题目。
  let same_href = sub.map(|s| format!("/browse?bank={}&sub={}", Bank::of_id(q.id_str()), s.name));
  let is_due = record.is_due(now_ms);
  let due = if is_due {
    t("learning.due-for-review")
  } else {
    let days = ((record.due_ms - now_ms) as f64 / DAY_MS as f64).ceil() as i64;
    tp("learning.review-in-days", days, &[&days.to_string()])
  };
  // 掌握度：难度系数越小，间隔增长越慢、越难掌握。
  // 第二项是现代风按语义取色用的档位，第三项是像素风的状态色工具类。
  let difficulty = if record.ease < 1.8 {
    (t("exam.hard"), "hard", "text-hp-text")
  } else if record.ease < 2.3 {
    (t("exam.medium"), "medium", "text-gold-text")
  } else {
    (t("exam.easy"), "easy", "text-win-text")
  };
  let my_answer = if record.my_answer.is_empty() {
    t("learning.marked-don-t-know")
  } else {
    record.my_answer.join("、")
  };
  let key = record.key.clone();
  // 一屏有多张卡：「移出」的无障碍名带题号，读屏分得清是哪一题。
  let remove_label = q.stable_id().map_or_else(
    || t("learning.remove"),
    |id| tf("learning.remove-mistake-n", &[&id]),
  );
  let chosen = record.my_answer.clone();
  let explain = data::snapshot_explanation(fresh, record.question.clone());
  let cause = RwSignal::new(record.cause.clone());
  let set_cause = {
    let key = key.clone();
    move |c: &'static str| {
      cause.set(if c.is_empty() {
        None
      } else {
        Some(c.to_owned())
      });
      crate::study::set_mistake_cause(&key, c);
    }
  };
  // 错因胶囊两套皮肤共用一份；现代风取密集档：横向内边距与字号收一档，四枚胶囊在 390px
  // 下才排得紧凑。高度不在这里省 —— `.md-chip` 的 `min-height: 32px` 仍然生效（触屏抬到 44）。
  let modern = crate::kit::is_modern_at_mount();
  let chip_class = if modern { "md-chip-sm" } else { "" };
  let cause_chips = WRONG_CAUSES
    .iter()
    .map(|&(ck, cn)| {
      let sc = set_cause.clone();
      view! {
        <ChipToggle
          active=Signal::derive(move || cause.get().as_deref() == Some(ck))
          on_change=Callback::new(move |on: bool| sc(if on { ck } else { "" }))
          class=chip_class
        >
          {view! { {move || t(cn)} }.into_any()}
        </ChipToggle>
      }
    })
    .collect_view();
  let streak = tf(
    "learning.in-a-row",
    &[
      &record.streak.to_string(),
      &record.target_streak().to_string(),
    ],
  );
  let interval = tp(
    "exam.day-interval",
    record.interval_days.round() as i64,
    &[&format!("{:.0}", record.interval_days)],
  );
  let wrong_count = record.wrong_count;

  // 题目本身是 `QuestionRow`，这里只管卡片末尾：现代风用专属的 `md-mistake-*` 版式
  // （与收藏集的 `md-bookmark-foot` 同一条分隔线与节奏），像素风沿用工具类。
  let footer = if modern {
    view! {
      <div class="md-mistake-foot">
        // 复盘：我选了什么、这题属于哪个考点、去哪补
        <div class="md-mistake-row">
          <span class="md-mistake-mine">
            {move || t("learning.my-answer")} <span class="md-mono">{my_answer}</span>
          </span>
          <span class="md-mistake-topic">
            {move || t("learning.topic")}
            {match topic_href {
              Some(href) => view! {
                <a href=href class="md-link inline-flex items-center pointer-coarse:min-h-11">
                  {move || t(topic)}
                </a>
              }
              .into_any(),
              None => view! { <span>{move || t(topic)}</span> }.into_any(),
            }}
            {topic_rate
              .map(|r| view! { <span class="md-muted">{tf("learning.accuracy-in-parens", &[&r.to_string()])}</span> })}
          </span>
          {same_href
            .map(|href| view! {
              <a href=href class="md-link inline-flex items-center pointer-coarse:min-h-11">
                {move || t("exam.practise-similar-link")}
              </a>
            })}
        </div>
        // 进度：答错几次、连对进度、难度、间隔、何时复习
        <ul class="md-mistake-stats">
          <li>
            {move || t("learning.wrong")} <b>{wrong_count}</b> {move || t("learning.times")}
          </li>
          <li>{streak}</li>
          <li>
            {move || t("exam.difficulty")} <b class="md-mistake-level" data-level=difficulty.1>{difficulty.0}</b>
          </li>
          <li>{interval}</li>
          <li data-due=is_due.to_string()>{due}</li>
        </ul>
        // 错因自评 + 移出：移出放在行尾，与自评分开，不会被当成又一枚胶囊
        <div class="md-mistake-row md-mistake-cause">
          <span class="md-stat-label">{move || t("exam.why-wrong")}</span>
          {cause_chips}
          <button
            type="button"
            class="md-btn md-btn-ghost md-btn-sm md-mistake-remove"
            aria-label=remove_label
            on:click=move |_| on_remove.run(key.clone())
          >
            {move || t("learning.remove")}
          </button>
        </div>
      </div>
    }
    .into_any()
  } else {
    view! {
      <div class="mt-3 space-y-2 border-t pt-3">
        <div class="flex flex-wrap items-center gap-x-5 gap-y-1 text-xs">
          <span class="text-hp-text">
            {move || t("learning.my-answer")} <span class="font-mono">{my_answer}</span>
          </span>
          <span class="text-muted-foreground">
            {move || t("learning.topic")}
            {match topic_href {
              Some(href) => view! {
                <a href=href class="text-foreground">{move || t(topic)}</a>
              }
              .into_any(),
              None => view! { <span class="text-foreground">{move || t(topic)}</span> }.into_any(),
            }}
            {topic_rate.map(|r| view! { <span class="ml-1 tabular-nums">{tf("learning.accuracy-in-parens", &[&r.to_string()])}</span> })}
            {same_href.map(|href| view! {
              <a href=href class="ml-1 text-primary">
                {move || t("exam.practise-similar-link")}
              </a>
            })}
          </span>
        </div>
        <div class="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
          <span>{move || t("learning.wrong")} <span class="text-foreground tabular-nums">{wrong_count}</span> {move || t("learning.times")}</span>
          <span>{streak}</span>
          <span>
            {move || t("exam.difficulty")} <span class=difficulty.2>{difficulty.0}</span>
          </span>
          <span>{interval}</span>
          <span>{due}</span>
          <button
            type="button"
            class="ml-auto px-2 py-0.5 transition-colors hover:bg-accent hover:text-foreground pointer-coarse:min-h-11"
            aria-label=remove_label
            on:click=move |_| on_remove.run(key.clone())
          >
            {move || t("learning.remove")}
          </button>
        </div>
        <div class="flex flex-wrap items-center gap-1.5 text-xs">
          <span class="text-muted-foreground">{move || t("exam.why-wrong")}</span>
          {cause_chips}
        </div>
      </div>
    }
    .into_any()
  };

  view! {
    <QuestionRow q=record.question.clone() bookmarks=bookmarks explanation=explain chosen=chosen>
      {footer}
    </QuestionRow>
  }
}
