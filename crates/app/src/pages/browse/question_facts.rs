//! 题目卡末尾的「知识点 / 参考依据」折叠区：默认收起，首次展开前不构建正文。
//!
//! 这两块是解题的**补充资料**，不是答题必需：分类浏览一次铺 40 张卡、错题本能到 1000 条，
//! 每个考点都带一段说明与几条法条链接的话，卡片会被撑到一屏一张，而多数时候用户只读题干、
//! 选项与解析。分类浏览 / 错题集 / 收藏集共用 `QuestionRow`，收起状态在三处一致。
//!
//! 收起时读屏与页面内搜索都不该命中正文：两套皮肤各自沿用「折叠面板」的既有实现
//! （像素 `.ui-collapse`、现代 `.md-x-panel`），它们都在收起动画结束后把内容设为
//! `visibility: hidden`（`components/common/explanation_card.rs` 与 `kit_modern/explanation.rs`
//! 用的是同一手法）。

use ham_web_core::categories::{RefItem, RefKind};
use leptos::prelude::*;

use crate::cn::cn;
use crate::data::kt_tracked;
use crate::i18n::t;
use crate::icons::{Icon, IconKind};

#[component]
pub(crate) fn QuestionFacts(
  /// 知识点所属的二级分类名（`crates/core` 的中文常量），标签行里跟在「知识点 · 」后面。
  sub_name: Option<&'static str>,
  /// 知识点正文；没有就不出这一块。
  note: Option<&'static str>,
  /// 一级分类的参考依据；空切片就不出这一块。
  refs: &'static [RefItem],
) -> impl IntoView {
  // 两样都没有时整块不出现（不是渲染一个空的可展开区域）。
  if note.is_none() && refs.is_empty() {
    return ().into_any();
  }

  let open = RwSignal::new(false);
  // 只增不减：展开过一次后正文留在 DOM 里，收起时的高度过渡才有内容可裹。
  let opened_once = RwSignal::new(false);
  // 一屏几十张卡：`aria-controls` 要指向各自的面板，id 必须唯一；两套皮肤的按钮与面板
  // 都要把这个 id 用上（`aria-controls` 指向不存在的 id，axe 会报 `aria-valid-attr-value`）。
  let panel_id = crate::util::unique_id("q-facts");

  if crate::kit::is_modern_at_mount() {
    return view! {
      <div class="md-q-row-block">
        <button
          type="button"
          class="md-x-toggle"
          aria-expanded=move || open.get().to_string()
          aria-controls=panel_id.clone()
          on:click=move |_| {
            open.update(|v| *v = !*v);
            opened_once.set(true);
          }
        >
          <span class="md-x-title">{move || t("exam.topic-and-references")}</span>
          <crate::kit_modern::ModernIcon
            kind=crate::kit_modern::MIcon::ChevronDown
            class="md-chev"
          />
        </button>
        <div id=panel_id class="md-x-panel" data-open=move || open.get().to_string()>
          <div class="md-x-inner">
            {move || {
              opened_once
                .get()
                .then(|| {
                  view! {
                    <div class="flex flex-col gap-2.5 pt-2.5">
                      {note
                        .map(|n| {
                          view! {
                            <div class="flex flex-col gap-1">
                              <span class="md-stat-label">
                                {move || t("knowledge.topic")}
                                {move || sub_name.map(kt_tracked)}
                              </span>
                              <p class="md-muted md-sm whitespace-pre-line">{move || kt_tracked(n)}</p>
                            </div>
                          }
                        })}
                      {(!refs.is_empty())
                        .then(|| {
                          view! {
                            <div class="flex flex-col gap-1">
                              <span class="md-stat-label">{move || t("exam.references")}</span>
                              <ul class="md-q-row-refs">
                                {refs
                                  .iter()
                                  .map(|r| {
                                    view! {
                                      <li>
                                        <span class="md-ref-badge">{move || kt_tracked(r.kind.label())}</span>
                                        {match r.url {
                                          Some(url) => view! {
                                            <a
                                              href=url
                                              target="_blank"
                                              rel="noopener noreferrer"
                                              class="inline-flex items-center pointer-coarse:min-h-11"
                                            >
                                              {move || kt_tracked(r.text)}
                                            </a>
                                          }
                                            .into_any(),
                                          None => view! { <span>{move || kt_tracked(r.text)}</span> }
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
                  }
                })
            }}
          </div>
        </div>
      </div>
    }
    .into_any();
  }

  // 像素风：行高 0fr → 1fr 做高度过渡（`.ui-collapse` 负责裁剪与收起后隐藏）。
  let panel = Signal::derive(move || {
    cn(&[
      "ui-collapse",
      if open.get() {
        "grid-rows-[1fr]"
      } else {
        "grid-rows-[0fr]"
      },
    ])
  });
  let chevron = Signal::derive(move || {
    cn(&[
      // Tailwind v4 的 `rotate-*` 走 `rotate` 属性，过渡属性要写 `rotate` 才动得了。
      "h-4 w-4 shrink-0 transition-[rotate] duration-[var(--motion-normal)] ease-[var(--ease-out-expo)]",
      if open.get() { "rotate-180" } else { "" },
    ])
  });

  view! {
    <div class="mb-3 border-t border-dashed pt-3">
      <button
        type="button"
        class="flex w-full items-center justify-between gap-2 text-xs text-muted-foreground transition-colors hover:text-foreground pointer-coarse:min-h-11"
        aria-expanded=move || open.get().to_string()
        aria-controls=panel_id.clone()
        on:click=move |_| {
          open.update(|v| *v = !*v);
          opened_once.set(true);
        }
      >
        <span>{move || t("exam.topic-and-references")}</span>
        <Icon kind=IconKind::ChevronDown class=chevron />
      </button>
      <div id=panel_id class=panel data-open=move || open.get().to_string()>
        <div>
          <div class="space-y-2 pt-2">
            {move || {
              opened_once
                .get()
                .then(|| {
                  view! {
                    {note
                      .map(|n| {
                        view! {
                          <div>
                            <div class="mb-1 text-xs text-muted-foreground">
                              {move || t("knowledge.topic")}
                              {move || sub_name.map(kt_tracked)}
                            </div>
                            <div class="whitespace-pre-line text-sm leading-6 text-muted-foreground">
                              {move || kt_tracked(n)}
                            </div>
                          </div>
                        }
                      })}
                    {(!refs.is_empty())
                      .then(|| {
                        view! {
                          <div>
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
                                        class=format!(
                                          "{} {}",
                                          crate::kit::link_class(),
                                          "inline-flex items-center gap-1 pointer-coarse:min-h-11",
                                        )
                                      >
                                        {move || kt_tracked(r.text)}
                                        <Icon kind=IconKind::ExternalLink class="size-6 w-3" />
                                      </a>
                                    }
                                      .into_any(),
                                    None => view! { <span>{move || kt_tracked(r.text)}</span> }.into_any(),
                                  };
                                  view! {
                                    <li class="flex items-start gap-2 text-sm text-muted-foreground">
                                      <span class=badge>{move || kt_tracked(r.kind.label())}</span>
                                      {body}
                                    </li>
                                  }
                                })
                                .collect_view()}
                            </ul>
                          </div>
                        }
                      })}
                  }
                })
            }}
          </div>
        </div>
      </div>
    </div>
  }
  .into_any()
}
