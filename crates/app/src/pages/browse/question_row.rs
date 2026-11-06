use std::collections::HashSet;

use ham_web_core::QuestionItem;
use ham_web_core::categories::{self, RefKind};
use leptos::prelude::*;

use crate::cn::cn;
use crate::i18n::{t, tf, tp};
use crate::icons::{Icon, IconKind};
use crate::store;
use crate::tone;
use crate::ui::ChipToggle;

/// 一道题的完整展示卡（题号 / 分类标签、题干、正确答案、解析、知识点、参考依据）。
///
/// 分类浏览与收藏集共用这一张卡：同一道题在两处长得一样，收藏集只通过 `children`
/// 在卡片末尾追加自己的分组 / 笔记操作区，不另画一套题目版式。
#[component]
pub(crate) fn QuestionRow(
  q: QuestionItem,
  /// 列表页持有的收藏快照：本行从里面读自己的状态，切换时就地改它。
  /// 不直接读 `store::is_bookmarked()`：那是普通函数、不参与响应式，点了不会变色。
  bookmarks: RwSignal<HashSet<String>>,
  /// 收藏状态切换后的回调，参数是切换后的真实状态（`true` = 已收藏）。
  /// 收藏集靠它在取消收藏后把这一题从列表里拿掉。
  #[prop(optional)]
  on_bookmark_change: Option<Callback<bool>>,
  /// 解析文本的来源；不传就用题目自带的 `explanation`。
  /// 错题本里的题是答错当时的快照，解析可能已被订正，调用方传入「读题库现在的解析」
  /// 的信号（`data::snapshot_explanation`），索引到达后只换这一块文字，整张卡不重建。
  #[prop(optional)]
  explanation: Option<Signal<Option<String>>>,
  /// 用户当时选的选项字母（错题本用）：命中的选项标上「我的选择」，选错的另加红色描边，
  /// 与正确项并排对照。文字标记是必须的，颜色之外读屏与灰度打印也能分辨。
  #[prop(optional)]
  chosen: Vec<String>,
  /// 卡片末尾的追加内容（分隔线以下的操作区）。
  #[prop(optional)]
  children: Option<Children>,
) -> impl IntoView {
  let explanation = explanation.unwrap_or_else(|| Signal::stored(q.explanation.clone()));
  // 没有稳定编号（`id` 与 `J` 都缺）的题不给收藏入口：位置兜底键 `pos:N` 随列表顺序变，
  // 写进存储后练习页对不上、收藏集也列不出来，用户还取消不了。
  let bookmark_toggle = q.stable_id().map(|id| {
    // 每行一个 Memo：快照整体变了，只有本题那一位翻转的行才继续通知下游绑定。
    let marked = Memo::new({
      let id = id.clone();
      move |_| bookmarks.with(|set| set.contains(&id))
    });
    // 名称恒定、带题号：切换状态只交给 `aria-pressed`，读屏也分得清是哪一题。
    let label = Signal::derive({
      let id = id.clone();
      move || tf("exam.bookmark-question", &[&id])
    });
    let title = Signal::derive(move || {
      if marked.get() {
        t("exam.remove-bookmark")
      } else {
        t("exam.bookmark-this-question")
      }
    });
    // 以存储为准：`toggle_bookmark` 返回切换后的真实状态，不信按钮传来的目标值。
    let on_change = Callback::new(move |_: bool| {
      let added = store::toggle_bookmark(&id);
      bookmarks.update(|set| {
        if added {
          set.insert(id.clone());
        } else {
          set.remove(&id);
        }
      });
      if let Some(cb) = on_bookmark_change {
        cb.run(added);
      }
    });
    view! {
      <ChipToggle active=marked on_change=on_change aria_label=label title=title>
        {view! {
          {move || if marked.get() { t("exam.bookmarked") } else { t("exam.bookmark") }}
        }
          .into_any()}
      </ChipToggle>
    }
  });
  let sub = q.p_code().and_then(categories::sub_category);
  let top_cat = sub.and_then(|s| categories::top_category(s.top));
  // 正确选项的字母（按选项顺序）：标题里要写出来，不能只靠选项的颜色 / 边框区分对错 ——
  // 读屏、色觉障碍与灰度打印都看不到颜色。
  let answer_letters: Vec<&str> = q
    .options
    .iter()
    .filter(|o| q.answer_keys.contains(&o.key))
    .map(|o| o.key.as_str())
    .collect();
  let answer_count = answer_letters.len();
  let answer_letters = answer_letters.join("、");
  let note = q.p_code().and_then(categories::sub_note);
  let refs = top_cat.map_or(&[][..], |c| categories::top_refs(c.key));
  let is_multiple = q.is_multiple();
  let answer_title = move || {
    if is_multiple {
      tp(
        "common.correct-answers-items",
        answer_count,
        &[&answer_count.to_string()],
      )
    } else {
      t("exam.correct-answer")
    }
  };

  if crate::kit::is_modern_at_mount() {
    let modern_id = q.stable_id();
    // 与像素风同一个无障碍名：带题号，收藏集里一屏有多张卡时读屏分得清是哪一题。
    let bookmark_label_id = modern_id.clone().unwrap_or_default();
    // 标签行与像素风取同一组字段（`id` 退化到 `J`、一级 / 二级分类、题型、官方分类码 `P`），
    // 两套皮肤对「这一题是什么」给出同一份答案。`id` 形如 `A-1`，本身就带题库类字母。
    let row_id = q.id_str().or(q.j_code()).map(ToOwned::to_owned);
    let p_code = q.p_code().map(ToOwned::to_owned);
    // 与像素风同一种写法：状态从列表持有的快照派生（`Memo` 自带观察者，组件体里读不会报
    // 「跟踪上下文之外读信号」）。不能只在挂载时取一次初值再靠本地信号同步 —— 那样
    // 其它标签页改了收藏、快照重读后，这张卡仍停在旧状态。
    let marked = Memo::new({
      let id = modern_id.clone();
      move |_| {
        id.as_ref()
          .is_some_and(|id| bookmarks.with(|s| s.contains(id)))
      }
    });
    let toggle_bookmark = {
      let id = modern_id.clone();
      move |_| {
        let Some(id) = id.clone() else { return };
        let added = store::toggle_bookmark(&id);
        bookmarks.update(|set| {
          if added {
            set.insert(id.clone());
          } else {
            set.remove(&id);
          }
        });
        if let Some(cb) = on_bookmark_change {
          cb.run(added);
        }
      }
    };

    return view! {
      <article class="md-q-row">
        // 标签行与题干分成两个块级行：题号 / 分类这些不定宽的胶囊若与题干同处一个 flex 行，
        // 题干首行会被推到胶囊右侧、换行后又回到最左边，左边缘就参差不齐。标签独占一行后，
        // 题干在自己的块里从卡片内距起排，窄屏与桌面宽度下都是同一条竖线。
        <div class="md-q-row-head">
          <div class="md-q-row-tags">
            {row_id
              .map(|id| view! { <span class="md-q-row-id">{id}</span> }.into_any())}
            {top_cat
              .map(|c| {
                // 分类标识色只做图形锚点（7px 圆点），与像素风用同一个 `tone::dot` 映射 ——
                // 分类名本身仍是中性文字色，颜色只是索引。
                view! {
                  <span class="md-q-row-tag">
                    <span class=format!("md-q-row-dot {}", tone::dot(c.tone)) aria-hidden="true"></span>
                    {move || crate::data::core_text_tracked(c.name)}
                  </span>
                }
              })}
            {sub
              .map(|s| {
                view! {
                  <span class="md-q-row-tag">{move || crate::data::kt_tracked(s.name)}</span>
                }
              })}
            {is_multiple
              .then(|| {
                view! {
                  <span class="md-q-row-tag md-q-row-tag-multi">{move || t("exam.multiple")}</span>
                }
              })}
            {p_code
              .map(|p| view! { <span class="md-q-row-code md-mono">{p}</span> })}
          </div>
          {modern_id
            .is_some()
            .then(|| {
              view! {
                // 不用 `md-icon-btn-xs`：那是「贴着输入框右缘」的尺寸（32，抬档也只到 32），
                // 这里是列表行里的独立目标，用整档的 `md-icon-btn`（coarse 下 44）。
                <button
                  type="button"
                  class="md-icon-btn"
                  aria-pressed=move || marked.get().to_string()
                  aria-label=move || tf("exam.bookmark-question", &[&bookmark_label_id])
                  title=move || {
                    if marked.get() { t("exam.remove-bookmark") } else { t("exam.bookmark-this-question") }
                  }
                  on:click=toggle_bookmark
                >
                  <crate::kit_modern::ModernIcon kind=crate::kit_modern::MIcon::Book />
                </button>
              }
            })}
        </div>

        <p class="md-q-row-text">{q.question.clone()}</p>
        {q
          .image()
          .map(|src| {
            view! {
              <img
                class="md-q-row-img"
                src=src.to_owned()
                alt=move || t("exam.question-image")
                loading="lazy"
              />
            }
          })}

        <ul class="md-q-row-opts">
          {q
            .options
            .iter()
            .map(|o| {
              let right = q.answer_keys.contains(&o.key);
              let picked = chosen.contains(&o.key);
              view! {
                <li data-right=right.to_string() data-chosen=picked.to_string()>
                  <span class="md-mono">{o.key.clone()}</span>
                  <span>{o.text.clone()}</span>
                  {picked
                    .then(|| view! { <span class="md-q-row-chosen">{move || t("learning.my-choice")}</span> })}
                </li>
              }
            })
            .collect_view()}
        </ul>

        <p class="md-q-row-answer">
          "✓ " {answer_title} " " <span class="md-mono">{answer_letters.clone()}</span>
        </p>

        <div class="md-q-row-meta">
          {move || {
            explanation
              .get()
              .filter(|e| !e.is_empty())
              .map(|e| {
                view! {
                  <div class="md-q-row-block">
                    <span class="md-stat-label">{move || t("exam.explanation")}</span>
                    <p class="md-muted md-sm whitespace-pre-line">{e}</p>
                  </div>
                }
              })
          }}
          {note
            .map(|n| {
              view! {
                <div class="md-q-row-block">
                  <span class="md-stat-label">
                    {move || t("knowledge.topic")} " "
                    {move || sub.map(|s| crate::data::kt_tracked(s.name))}
                  </span>
                  <p class="md-muted md-sm">{move || crate::data::kt_tracked(n)}</p>
                </div>
              }
            })}
          {(!refs.is_empty())
            .then(|| {
              view! {
                <div class="md-q-row-block">
                  <span class="md-stat-label">{move || t("exam.references")}</span>
                  <ul class="md-q-row-refs">
                    {refs
                      .iter()
                      .map(|r| {
                        view! {
                          <li>
                            <span class="md-ref-badge">{move || crate::data::kt_tracked(r.kind.label())}</span>
                            {match r.url {
                              Some(url) => view! {
                                <a
                                  href=url
                                  target="_blank"
                                  rel="noopener noreferrer"
                                  class="inline-flex items-center pointer-coarse:min-h-11"
                                >
                                  {move || crate::data::kt_tracked(r.text)}
                                </a>
                              }
                                .into_any(),
                              None => view! { <span>{move || crate::data::kt_tracked(r.text)}</span> }
                                .into_any(),
                            }}
                          </li>
                        }
                      })
                      .collect_view()}
                  </ul>
                </div>
              }
            })}
        </div>
        {children.map(|c| c())}
      </article>
    }
    .into_any();
  }

  view! {
    <article class=crate::kit::card_class_with("p-4 sm:p-5")>
      <div class="mb-3 flex flex-wrap items-center gap-2">
        <span class="bg-muted px-2 py-0.5 font-mono text-xs text-muted-foreground">
          {q.id_str().or(q.j_code()).map(ToOwned::to_owned)}
        </span>
        {top_cat
          .map(|c| {
            let chip = tone::chip(c.tone);
            view! {
              <span class=format!("border-2 px-2 py-0.5 text-xs text-foreground {chip}")>
                {move || crate::data::core_text_tracked(c.name)}
              </span>
            }
          })}
        {sub.map(|s| view! { <span class="bg-accent px-2.5 py-0.5 text-xs text-muted-foreground">{move || crate::data::kt_tracked(s.name)}</span> })}
        {is_multiple
          .then(|| {
            view! {
              <span class="bg-gold/10 px-2 py-0.5 text-xs text-gold-text">
                {move || t("exam.multiple")}
              </span>
            }
          })}
        // 编号与收藏同组：窄屏换行时两者一起靠右，不会让收藏单独落在一行的最左边。
        <span class="ml-auto flex items-center gap-2">
          <span class="font-mono text-xs text-muted-foreground">{q.p_code().map(ToOwned::to_owned)}</span>
          {bookmark_toggle}
        </span>
      </div>

      <div class="mb-3 leading-relaxed">{q.question.clone()}</div>
      {q.image().map(|src| view! { <img src=src.to_owned() alt=move || t("exam.question-image") loading="lazy" class="mb-3 max-h-64 border" /> })}

      <div class="mb-3 border border-win/40 bg-win/5 p-3">
        <div class="mb-1.5 flex items-center gap-1.5 text-xs text-win-text">
          <Icon kind=IconKind::CheckCircle2 class="size-6 w-3.5" />
          {answer_title} <span class="font-mono">{answer_letters}</span>
        </div>
        // 列出全部选项，正确项用绿色实心标出：只给正确项的话，读者看不到干扰项长什么样。
        {q
          .options
          .iter()
          .map(|o| {
            let right = q.answer_keys.contains(&o.key);
            let picked = chosen.contains(&o.key);
            let key_class = if right {
              "mt-0.5 flex size-5 shrink-0 items-center justify-center border-2 border-win bg-win/15 text-xs text-win-text"
            } else if picked {
              "mt-0.5 flex size-5 shrink-0 items-center justify-center border-2 border-hp bg-hp/15 text-xs text-hp-text"
            } else {
              "mt-0.5 flex size-5 shrink-0 items-center justify-center border-2 bg-card text-xs text-muted-foreground"
            };
            let text_class = if right || picked { "flex-1" } else { "flex-1 text-muted-foreground" };
            view! {
              <div
                class="flex items-start gap-2 py-0.5 text-sm"
                data-right=right.to_string()
                data-chosen=picked.to_string()
              >
                <span class=key_class>{o.key.clone()}</span>
                <span class=text_class>{o.text.clone()}</span>
                {picked
                  .then(|| view! { <span class="shrink-0 text-xs text-hp-text">{move || t("learning.my-choice")}</span> })}
              </div>
            }
          })
          .collect_view()}
      </div>

      {move || {
        explanation
          .get()
          .filter(|e| !e.is_empty())
          .map_or_else(
            || {
              view! {
                <div class="mb-3 border-t border-dashed pt-3">
                  <div class="mb-1 text-xs text-muted-foreground">{move || t("exam.explanation")}</div>
                  <div class="text-sm leading-6 text-muted-foreground">{move || t("knowledge.no-explanation-yet-for")}</div>
                </div>
              }
              .into_any()
            },
            |e| {
              view! {
                <div class="mb-3 border-t border-dashed pt-3">
                  <div class="mb-1 text-xs text-muted-foreground">{move || t("exam.explanation")}</div>
                  <div class="whitespace-pre-line text-sm leading-6 text-muted-foreground">{e}</div>
                </div>
              }
              .into_any()
            },
          )
      }}

      {note
        .map(|n| {
          view! {
            <div class="mb-3 border-t border-dashed pt-3">
              <div class="mb-1 text-xs text-muted-foreground">
                {move || t("knowledge.topic")} {move || sub.map(|s| crate::data::kt_tracked(s.name))}
              </div>
              <div class="text-sm leading-6 text-muted-foreground">{move || crate::data::kt_tracked(n)}</div>
            </div>
          }
        })}

      {(!refs.is_empty())
        .then(|| {
          view! {
            <div class="border-t border-dashed pt-3">
              <div class="mb-1 text-xs text-muted-foreground">{move || t("exam.references")}</div>
              <ul class="space-y-1">
                {refs
                  .iter()
                  .map(|r| {
                    let badge = cn(&[
                      "mt-0.5 shrink-0 px-1.5 py-0.5 text-xs",
                      if r.kind == RefKind::Law {
                        "bg-xp/10 text-xp-text"
                      } else {
                        "bg-win/10 text-win-text"
                      },
                    ]);
                    let body = match r.url {
                      Some(url) => view! {
                        <a
                          href=url
                          target="_blank"
                          rel="noopener noreferrer"
                          class=format!("{} {}", crate::kit::link_class(), "inline-flex items-center gap-1 pointer-coarse:min-h-11")
                        >
                          {move || crate::data::kt_tracked(r.text)}
                          <Icon kind=IconKind::ExternalLink class="size-6 w-3" />
                        </a>
                      }
                      .into_any(),
                      None => view! { <span>{move || crate::data::kt_tracked(r.text)}</span> }.into_any(),
                    };
                    view! {
                      <li class="flex items-start gap-2 text-sm text-muted-foreground">
                        <span class=badge>{move || crate::data::kt_tracked(r.kind.label())}</span>
                        {body}
                      </li>
                    }
                  })
                  .collect_view()}
              </ul>
            </div>
          }
        })}
      {children.map(|c| c())}
    </article>
  }
  .into_any()
}
