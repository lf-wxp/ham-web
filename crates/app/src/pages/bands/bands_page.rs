use std::time::Duration;

use ham_web_core::bands::{self, BANDS, FOOTNOTES, Usage};
use leptos::prelude::*;

use crate::components::common::{Deferred, PageContainer, PageHeader, ProvenanceNote};
use crate::data::kt_tracked;
use crate::icons::{Icon, IconKind};
use crate::ui::Stat;
use crate::util::{document, scroll_to_element, set_title};

use super::band_card::BandCard;
use super::band_quiz::BandQuiz;
use super::usage_badge::UsageBadge;
use super::{SAT_ICON, footnote_id, table_rows};
use crate::i18n::t;

#[component]
pub fn BandsPage() -> impl IntoView {
  set_title("shell.band-chart-title");
  // 本页渲染 `crates/core` 的中文原文，译文词典要自己挂一次拉取（本页不是 `KnowledgePage`）。
  crate::data::use_knowledge_i18n();
  let active = RwSignal::new(None::<&'static str>);

  let jump = Callback::new(move |code: &'static str| {
    if let Some(el) = document().get_element_by_id(&footnote_id(code)) {
      // 让出吸顶页头：`scroll_into_view` 会把脚注开头滚到视口顶部，被页头盖住。
      scroll_to_element(&el);
    }
    active.set(Some(code));
    set_timeout(
      move || {
        if active.try_get_untracked().flatten() == Some(code) {
          active.try_set(None);
        }
      },
      Duration::from_millis(1600),
    );
  });

  let footnotes = move || {
    FOOTNOTES
      .iter()
      .map(|f| {
        let code = f.code;
        view! {
          <div
            id=footnote_id(code)
            class=move || {
              format!(
                "grid scroll-mt-24 grid-cols-[4.5rem_1fr] gap-3 px-4 py-3 text-sm transition-colors duration-(--motion-slow) {}",
                if active.get() == Some(code) { "bg-accent" } else { "" },
              )
            }
          >
            <dt class="font-mono">{code}</dt>
            <dd class="leading-6 text-muted-foreground">{move || kt_tracked(f.text)}</dd>
          </div>
        }
      })
      .collect_view()
  };
  // 桌面表格 / 手机卡片二选一（见下方视图）。
  let wide = crate::media::use_media_query(crate::media::MD_UP);

  let legend = Usage::ALL
    .into_iter()
    .map(|u| view! { <UsageBadge usage=u /> })
    .collect_view();

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.band-chart-title")
        subtitle=move || t("radio.upper-limits-included-lower")
      />

      <PageContainer class="space-y-4">
        <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <Stat label=Signal::derive(move || t("radio.band")) value=BANDS.len() />
          <Stat label=Signal::derive(move || t("knowledge.amateur-bands")) value=bands::allocation_count() />
          <Stat label=Signal::derive(move || t("knowledge.also-available-to-the")) value=bands::satellite_count() />
          <Stat label=Signal::derive(move || t("radio.footnote")) value=FOOTNOTES.len() />
        </div>

        <div class="flex flex-wrap items-center gap-x-5 gap-y-2 text-xs text-muted-foreground">
          <span class="inline-flex items-center gap-1">
            <Icon kind=IconKind::Satellite class=SAT_ICON />
            {move || t("radio.means-the-band-is")}
          </span>
          <span class="inline-flex flex-wrap items-center gap-1.5">{move || t("knowledge.usage-status")} {legend}</span>
        </div>

        // 桌面表格与手机卡片只建当前宽度用得到的那一套（`md` 断点，跨断点时切换）：
        // 此前两套都建、再用 CSS 藏一套，进这一页就白建几百个节点。
        // `hidden md:block` / `md:hidden` 仍保留：信号与 CSS 是同一条断点，CSS 兜住切换前的那一帧。
        {move || {
          if wide.get() {
            // 刻意的例外：波段表的 tbody 有大量动态 `rowspan` / `colspan`
            // （`table_rows.rs` 按波段跨行合并单元格），`TableCell` 的静态 `colspan: u32`
            // 表达不了，整表保留原生 `<table>`。
            view! {
              <div class=crate::kit::card_class_with("hidden overflow-x-auto md:block")>
                <table class="w-full min-w-[960px] border-collapse border-hidden text-sm">
                  <thead class="bg-inset text-xs">
                    <tr>
                      <th class=crate::kit::table_cell_class("")>{move || t("radio.reference")}</th>
                      <th class=crate::kit::table_cell_class("") colspan="2">{move || t("radio.band-name")}</th>
                      <th class=crate::kit::table_cell_class("")>{move || t("radio.wavelength-range")}</th>
                      <th class=crate::kit::table_cell_class("") colspan="2">{move || t("radio.frequency-band-name")}</th>
                      <th class=crate::kit::table_cell_class("")>{move || t("radio.band-range")}</th>
                      <th class=crate::kit::table_cell_class("")>{move || t("radio.amateur-amateur-satellite-bands")}</th>
                      <th class=crate::kit::table_cell_class("")>{move || t("radio.status")}</th>
                      <th class=crate::kit::table_cell_class("")>{move || t("radio.footnote-notes")}</th>
                    </tr>
                  </thead>
                  <tbody>{table_rows(jump)}</tbody>
                </table>
              </div>
            }
            .into_any()
          } else {
            view! {
              <div class="space-y-3 md:hidden">
                {BANDS.iter().map(|band| view! { <BandCard band=band jump=jump /> }).collect_view()}
              </div>
            }
            .into_any()
          }
        }}

        // 脚注与小测在波段表下面（首屏以下）：首帧之后再挂（`Deferred` 会补上 `#脚注` 锚点的滚动定位）。
        // 出处说明也在里面：它在页面最底部，若留在外面，延后挂载时会先于脚注出现、再被整块往下推。
        <Deferred>
          <section class=crate::kit::card_class()>
            <h2 class=crate::kit::card_title_class()>{move || t("radio.footnote")}</h2>
            <dl class="divide-y">{footnotes()}</dl>
          </section>
          <BandQuiz />
          <ProvenanceNote domain="bands" />
        </Deferred>
      </PageContainer>
    </div>
  }
}
