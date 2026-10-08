//! 错题诊断面板：聚合分类分布、错因与薄弱环节，给出可行动结论。

use ham_web_core::categories::{TOP_CATEGORIES, top_pages};
use ham_web_core::mistake_book::cause_stats;
use leptos::prelude::*;

use super::mistakes_page::top_of;
use crate::i18n::{Locale, locale, t, tf, tp};
use crate::study;

/// 一级分类 key → 中文名。
fn category_name(key: &str) -> &'static str {
  TOP_CATEGORIES
    .iter()
    .find(|c| c.key == key)
    .map_or("其他", |c| c.name)
}

/// 错题诊断：聚合分类分布、错因与薄弱环节，给出可行动结论。
#[component]
pub(super) fn MistakeDiagnosis() -> impl IntoView {
  let records = study::load_book().sorted();
  let stats = study::load_stats();

  // 错题分类分布（按错题数降序）。
  let mut cat_counts: Vec<(&'static str, usize)> = TOP_CATEGORIES
    .iter()
    .map(|c| {
      (
        c.key,
        records
          .iter()
          .filter(|m| top_of(&m.question) == c.key)
          .count(),
      )
    })
    .filter(|(_, n)| *n > 0)
    .collect();
  cat_counts.sort_by_key(|(_, n)| std::cmp::Reverse(*n));

  // 主要错因（已标注的最多）。
  let top_cause = cause_stats(&study::load_book())
    .into_iter()
    .filter(|(_, _, n)| *n > 0)
    .max_by_key(|(_, _, n)| *n)
    .map(|(_, name, n)| (name, n));

  // 最薄弱分类（答题 ≥ 5 次中正确率最低）。
  let weakest = stats.total.weakest(5);

  let top_cat = cat_counts.first().copied();

  // 生成结论。
  let summary = {
    let mut parts: Vec<String> = Vec::new();
    if let Some((key, n)) = top_cat {
      parts.push(tp(
        "common.questions-2",
        n as u32,
        &[
          &t("exam.most-mistakes-are-in"),
          &t(category_name(key)),
          &n.to_string(),
        ],
      ));
    }
    if let Some((name, n)) = top_cause {
      parts.push(tp(
        "common.questions-2",
        n as u32,
        &[&t("exam.main-cause-is"), &t(name), &n.to_string()],
      ));
    }
    if let Some((key, tally)) = weakest {
      let rate = (tally.rate().unwrap_or(0.0) * 100.0).round() as u32;
      parts.push(tf(
        "common.correct",
        &[
          &t("exam.weakest-area-is"),
          &t(category_name(key)),
          &rate.to_string(),
        ],
      ));
    }
    if parts.is_empty() {
      t("exam.few-mistakes-keep-it")
    } else {
      // 标点也随语言走：原先写死的中文标点会让整句在 en / es 下仍是中文句式
      let (sep, end) = match locale().get() {
        Locale::Zh => ("；", "。"),
        _ => ("; ", "."),
      };
      format!("{}{}", parts.join(sep), end)
    }
  };

  let has_signal = top_cat.is_some() || top_cause.is_some() || weakest.is_some();
  if !has_signal {
    return ().into_any();
  }

  let top_cat_href = top_cat.and_then(|(key, _)| top_pages(key).first().map(|&(href, _)| href));
  let weak_href = weakest.and_then(|(key, _)| top_pages(key).first().map(|&(href, _)| href));

  view! {
    <div class="rounded-xl border bg-card p-4">
      <h3 class="mb-2 text-sm font-semibold">{move || t("exam.mistake-diagnosis")}</h3>
      <p class="text-sm text-muted-foreground">{summary}</p>
      <div class="mt-3 flex flex-wrap gap-1.5">
        {top_cat_href.map(|href| view! {
          <a
            href=href
            class="rounded-full bg-primary/10 px-3 py-1 text-xs font-medium text-primary transition-colors hover:bg-primary/20"
          >
            {move || t("exam.review-weak-topic")}
          </a>
        })}
        {weak_href.map(|href| view! {
          <a
            href=href
            class="rounded-full border px-3 py-1 text-xs transition-colors hover:bg-accent"
          >
            {move || t("exam.targeted-practice")}
          </a>
        })}
      </div>
    </div>
  }
  .into_any()
}
