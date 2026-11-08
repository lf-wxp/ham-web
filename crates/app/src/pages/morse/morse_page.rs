//! 摩尔斯电码页的外壳。
//!
//! 只做三件事：设标题、提供 `morse_settings` 上下文、按皮肤选外壳
//! （像素风 `PageHeader` + `PageContainer`，现代风 `ModernPageHead` + `md-page`）。
//! 正文只有一份，在 [`super::morse_body::MorseBody`]。

use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::morse_settings::provide_morse_settings;
use crate::util::set_title;

use super::morse_body::MorseBody;
use super::signal_bars::SignalBars;
use crate::data;
use crate::i18n::t;

#[component]
pub fn MorsePage() -> impl IntoView {
  set_title("shell.morse-code");
  // 本页渲染 `crates/core` 的中文原文（`kt_tracked`），译文词典要自己挂一次拉取，
  // 否则以英文 / 西语直接打开本页时词典永远不来、这些文案停在中文。
  data::use_knowledge_i18n();
  provide_morse_settings();

  if crate::kit::is_modern_at_mount() {
    return view! {
      <div class="md-page">
        <crate::kit_modern::ModernPageHead
          title=Signal::derive(move || t("shell.morse-code"))
          subtitle=Signal::derive(move || t("morse.international-morse-code-itu"))
          actions=ViewFn::from(move || view! { <SignalBars /> })
        />
        // 正文自己排纵向节奏，且必须**再包一层**：`.md-page > section` 会挂上「章节编号 +
        // 刻度尺」那套文章式标头（`kit.css` 的 `#route-content .md-page > section`），
        // 而这里的 `<section>` 是面板而不是文章章节 —— 直接做 `.md-page` 的子节点时，
        // 面板左上会多出一个「01」和一条虚线刻度。与 `/log` 的现代分支同一写法。
        <div class="flex flex-col gap-5">
          <MorseBody />
        </div>
      </div>
    }
    .into_any();
  }

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.morse-code")
        subtitle=move || t("morse.international-morse-code-itu")
        actions=ViewFn::from(move || {
          view! {
            <SignalBars />
          }
        })
      />
      <PageContainer>
        <MorseBody />
      </PageContainer>
    </div>
  }
  .into_any()
}
