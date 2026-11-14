//! 像素风首页「值机台」：主显示屏 → 快速开始 → 知识频谱 → 今日值机 → 模块入口（顺序与面板形态见
//! `docs/ui/pixel.md` §3.4）。

use ham_web_core::Bank;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::bank_selector::QuestionBankSelector;
use crate::components::bubble::Bubble;
use crate::components::common::{PageContainer, SectionCard};
use crate::components::study_plan_card::StudyPlanCard;
use crate::data;
use crate::icons::{Icon, IconKind};
use crate::pages::{DEFAULT_TITLE, bank_href};
use crate::ui::{ButtonLink, Size, Variant};
use crate::util::set_title;

use super::base_hero::BaseHero;
use super::cards_card::CardsCard;
use super::daily_question::DailyQuestion;
use super::knowledge_spectrum::KnowledgeSpectrum;
use super::modules::MODULES;
use super::propagation_widget::PropagationWidget;
use super::quest_board::QuestBoard;
use super::review_card::ReviewCard;
use super::wanted_card::WantedCard;
use crate::i18n::{t, tf};

#[component]
pub fn PixelHomePage() -> impl IntoView {
  set_title(DEFAULT_TITLE);
  // 本页渲染 `crates/core` 的中文原文，译文词典要自己挂一次拉取（本页不是 `KnowledgePage`）。
  crate::data::use_knowledge_i18n();
  let version = RwSignal::new(None::<String>);
  let bank = RwSignal::new(Bank::A);
  let checking = RwSignal::new(false);
  let available = RwSignal::new(false);
  let warn_open = RwSignal::new(false);
  let warn_text = RwSignal::new(String::new());
  let generation = StoredValue::new(0u32);

  let stack = NodeRef::<leptos::html::Div>::new();
  stack.on_load(|el| crate::motion::reveal_children(&el));

  Effect::new(move |_| {
    let (Some(v), b) = (version.get(), bank.get()) else {
      return;
    };
    generation.update_value(|g| *g += 1);
    let current = generation.get_value();
    checking.set(true);
    spawn_local(async move {
      let ok = data::bank_available(Some(&v), b).await;
      if generation.try_get_value() == Some(current) {
        available.set(ok);
        checking.set(false);
      }
    });
  });

  let guard = move |e: web_sys::MouseEvent| {
    if !available.get_untracked() || version.with_untracked(Option::is_none) {
      e.prevent_default();
      warn_text.set(tf(
        "home.question-bank-is-unavailable",
        &[&bank.get_untracked().to_string()],
      ));
      warn_open.set(true);
    }
  };
  let practice_href = move || bank_href("/practice", version.get().as_deref(), bank.get());
  let exam_href = move || bank_href("/exam", version.get().as_deref(), bank.get());
  let custom_href = move || {
    format!(
      "{}&mode=custom",
      bank_href("/exam", version.get().as_deref(), bank.get())
    )
  };

  view! {
    <PageContainer class="space-y-0 relative pb-12 pt-8 sm:pt-12">
      <BaseHero />

      // 快速开始：面板（L1）+ 标题栏；题库类别是一排波段键（在 `QuestionBankSelector` 里）。
      <SectionCard title=Signal::derive(move || t("home.quick-start")) class="mt-6">
        <div class="space-y-4 p-4">
          <QuestionBankSelector selected_version=version selected_bank=bank disabled=checking />
          <div class="flex flex-wrap gap-3 relative">
            <ButtonLink
              href=Signal::derive(practice_href)
              variant=Variant::Default
              size=Size::Default
              on_click=Callback::new(guard)
            >
              {move || t("home.start-practice")}
            </ButtonLink>
            <ButtonLink
              href=Signal::derive(exam_href)
              variant=Variant::Secondary
              size=Size::Default
              on_click=Callback::new(guard)
            >
              {move || t("home.start-mock-exam")}
            </ButtonLink>
            <ButtonLink
              href=Signal::derive(custom_href)
              variant=Variant::Outline
              size=Size::Default
              on_click=Callback::new(guard)
            >
              {move || t("exam.custom-paper")}
            </ButtonLink>
            <Bubble open=warn_open text=warn_text />
          </div>
        </div>
      </SectionCard>

      // 知识频谱：每根柱是一个分类的掌握度，点进去就是专项练习。
      <div class="mt-6">
        <KnowledgeSpectrum />
      </div>

      <div class="mt-6">
        <QuestBoard />
      </div>

      // 模块入口：三列不等宽（首列更宽）+ 阶梯式下沉，打破三等分的呆板网格。
      // 断点放在 lg：md（768px）下三列各只剩 ~220px，18px 的区块标题在英文里撑不下；
      // 卡片上还带 `min-w-0`，窄到极限时让文字换行而不是把整张卡顶出容器。
      //
      // 卡片本身不再是一整个链接：标题行去「模块总览」，下面每个条目都是各自的真链接
      // （数据里一直带着 href，只是以前被丢弃，七个条目只能全跳到同一处，想进「错题集」
      // 得先进卡再找，多一跳）。嵌套 `<a>` 不合法，所以拆成「标题链接 + 条目链接」两层。
      <div class="mt-16 grid grid-cols-1 gap-4 lg:grid-cols-[1.3fr_1fr_1fr] lg:items-start">
        {MODULES
          .iter()
          .enumerate()
          .map(|(i, m)| {
            view! {
              // 用 `<div>` 而不是 `<section>`：卡片里没有标题元素可作 `aria-labelledby`
              // （标题是 `pxl-label` 的 `<span>`），`<section>` 会变成三个没有名字的 landmark。
              // 页面标题层级还是 h1（Hero）→ h2（下方 `SectionCard`），不在这里插 h3。
              <div
                class=format!(
                  "reveal pxl-card border bg-card relative flex min-w-0 flex-col gap-3 p-4 {}",
                  m.offset,
                )
                style=format!("animation-delay: {}ms", 300 + i * 90)
              >
                <a
                  href=m.href
                  // `motion-press` 只提供两帧的过渡（`motion.css`），并不自己位移；悬停反馈是
                  // `hover:bg-accent` 换底色。整卡可点、需要上抬 + 硬投影的是 `SectionCard href`（`motion-lift`）。
                  class="motion-press group relative flex items-center gap-3 hover:bg-accent"
                >
                  <div class="flex size-12 shrink-0 items-center justify-center border-2 border-ink bg-primary text-primary-foreground">
                    <Icon kind=m.icon class="size-6" />
                  </div>
                  <div class="min-w-0">
                    <div class="flex flex-wrap items-baseline gap-x-2">
                      // 序号从角落的大水印搬进标题行：水印会压在斜向的链接清单下面，读不清。
                      <span aria-hidden="true" class="pxl-label text-xs text-muted-foreground">
                        {m.index}
                      </span>
                      <span class="pxl-label text-base">{move || crate::data::core_text_tracked(m.title)}</span>
                    </div>
                    <div class="mt-1 text-xs text-muted-foreground">{move || crate::data::core_text_tracked(m.desc)}</div>
                  </div>
                  <span aria-hidden="true" class="ml-auto shrink-0 text-muted-foreground">
                    <Icon kind=IconKind::ChevronRight class="size-6" />
                  </span>
                </a>
                <ul class="mt-auto grid grid-cols-1 gap-x-4 border-t-2 border-dashed border-ink/30 pt-2 sm:grid-cols-2">
                  {m
                    .links
                    .iter()
                    .map(|&(label, href)| {
                      view! {
                        <li>
                          <a
                            href=href
                            // 条目是 12px 小字，点击区靠 `min-h-9`（36px）撑开，手机上不会点错行。
                            class="flex min-h-9 items-center gap-2 border-2 border-transparent px-1 text-xs text-muted-foreground hover:border-ink hover:bg-accent hover:text-accent-foreground"
                          >
                            <span aria-hidden="true" class="size-1 shrink-0 bg-primary"></span>
                            {move || crate::data::core_text_tracked(label)}
                          </a>
                        </li>
                      }
                    })
                    .collect_view()}
                </ul>
              </div>
            }
          })
          .collect_view()}
      </div>

      // 首屏以下的功能卡片随滚动逐个浮现，避免一进页面七张卡片同时刷出来。
      // `mt-6`：页面关掉了 `PageContainer` 的 `space-y-6`（各块自己写 `mt-6` 分段），这里必须补 ——
      // 漏了的话三列模块卡片与下方「今日学习队列」的上下墨框会直接贴在一起（2026-11-14 修）。
      <div node_ref=stack class="mt-6 space-y-6">
        <StudyPlanCard />
        <ReviewCard />
        <CardsCard />
        <WantedCard />
        <DailyQuestion />
        <PropagationWidget />
      </div>
    </PageContainer>
  }
}
