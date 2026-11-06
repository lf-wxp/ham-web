//! 现代风「答案解析」卡：圆角卡 + 一行折叠头 + 法规引用链接。
//!
//! 折叠语义、法规引用切分都复用像素版（`components/common/explanation_card.rs` 的
//! `segments`），这里只重画外观：不再有标题栏色带、`▾` 换成线性 chevron、朗读换成图标按钮；
//! 标题是小号灰色标签（`md-x-title`），层级低于题卡。

use std::sync::atomic::{AtomicU32, Ordering};

use leptos::prelude::*;

use crate::components::common::explanation_card::segments;
use crate::i18n::t;
use crate::speech;

use super::{MIcon, ModernIcon};

#[component]
pub fn ModernExplanation(#[prop(into)] explanation: Signal<Option<String>>) -> impl IntoView {
  let open = RwSignal::new(false);
  // 只增不减：展开过一次后正文留着，收起时高度过渡才有内容可裹（与像素版同一策略）。
  let opened_once = RwSignal::new(false);
  let text = Memo::new(move |_| {
    explanation
      .get()
      .map(|s| s.trim().to_owned())
      .filter(|s| !s.is_empty())
  });

  view! {
    {move || {
      let Some(body) = text.get() else {
        return view! {
          <div class="md-empty-note">
            <ModernIcon kind=MIcon::Book />
            <span>{move || t("knowledge.no-explanation-yet-for")}</span>
            <a
              class="md-link-inline"
              href="https://github.com/lf-wxp/ham-web"
              target="_blank"
              rel="noopener noreferrer"
            >
              {move || t("knowledge.contribute")}
            </a>
          </div>
        }
        .into_any();
      };
      let speak_text = body.clone();
      // 页面上同时有多张解析卡：按钮的 `aria-controls` 要指向各自的面板，id 得唯一。
      static NEXT_PANEL: AtomicU32 = AtomicU32::new(0);
      let panel_id = format!("md-x-panel-{}", NEXT_PANEL.fetch_add(1, Ordering::Relaxed));
      let controls = panel_id.clone();

      view! {
        <div class="md-x">
          <div class="md-x-head">
            <button
              type="button"
              class="md-x-toggle"
              aria-expanded=move || open.get().to_string()
              aria-controls=controls
              on:click=move |_| {
                open.update(|v| *v = !*v);
                opened_once.set(true);
              }
            >
              <span class="md-x-title">
                <ModernIcon kind=MIcon::Book />
                {move || t("learning.explanation")}
              </span>
              <ModernIcon kind=MIcon::ChevronDown class="md-chev" />
            </button>
            <button
              type="button"
              class="md-icon-btn"
              aria-label=move || t("learning.read-explanation-aloud")
              on:click=move |_| speech::speak_zh(&speak_text)
            >
              <ModernIcon kind=MIcon::Volume />
            </button>
          </div>

          <div id=panel_id class="md-x-panel" data-open=move || open.get().to_string()>
            <div class="md-x-inner">
              {move || {
                opened_once.get().then(|| {
                  view! {
                    <div class="md-x-body">
                      {segments(&body)
                        .into_iter()
                        .map(|seg| {
                          if let Some((href, _title)) = seg.link {
                            view! {
                              <a class="md-link-inline" href=href>
                                {seg.text}
                              </a>
                            }
                              .into_any()
                          } else {
                            view! { <span>{seg.text}</span> }.into_any()
                          }
                        })
                        .collect_view()}
                    </div>
                  }
                })
              }}
            </div>
          </div>
        </div>
      }
      .into_any()
    }}
  }
}
