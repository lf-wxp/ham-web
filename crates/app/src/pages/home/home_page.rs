use ham_web_core::Bank;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::bank_selector::QuestionBankSelector;
use crate::components::bubble::Bubble;
use crate::components::common::PageContainer;
use crate::components::study_plan_card::StudyPlanCard;
use crate::data;
use crate::icons::{Icon, IconKind};
use crate::pages::{DEFAULT_TITLE, bank_href};
use crate::ui::{
  ButtonLink, CARD_HEADER, Size, Variant, card_class, card_content_class, card_title_class,
};
use crate::util::set_title;

use super::base_hero::BaseHero;
use super::cards_card::CardsCard;
use super::daily_challenge_card::DailyChallengeCard;
use super::daily_question::DailyQuestion;
use super::propagation_widget::PropagationWidget;
use super::quest_board::QuestBoard;
use super::review_card::ReviewCard;
use super::wanted_card::WantedCard;
use crate::i18n::{t, tf};

/// 首页模块入口。
struct ModuleCard {
  title: &'static str,
  desc: &'static str,
  href: &'static str,
  icon: IconKind,
  /// 卡片角落的序号水印。
  index: &'static str,
  /// 桌面端的纵向错位（阶梯式下沉，形成一条斜向的视线）。
  ///
  /// 写成完整的类名字面量放在数据里：Tailwind 只扫描源码里的完整类名，
  /// 运行时拼接 `md:mt-{n}` 会被当成不存在而不生成样式。
  offset: &'static str,
  links: &'static [(&'static str, &'static str)],
}

const MODULES: &[ModuleCard] = &[
  ModuleCard {
    title: "考试中心",
    desc: "A/B/C 类题库 · 练习 · 模拟考试",
    href: "/practice",
    icon: IconKind::Timer,
    index: "01",
    offset: "md:mt-0",
    links: &[
      ("练习", "/practice"),
      ("模拟考试", "/exam"),
      ("每日挑战", "/daily-challenge"),
      ("分类浏览", "/browse"),
      ("闪卡刷题", "/flashcards"),
      ("错题集", "/mistakes"),
      ("学习周报", "/weekly"),
    ],
  },
  ModuleCard {
    title: "知识库",
    desc: "80+ 专题 · 呼号 · 术语 · 模式 · 传播 · 天线 · 通联",
    href: "/reference",
    icon: IconKind::BookOpen,
    index: "02",
    offset: "md:mt-6",
    links: &[
      ("考试速查", "/reference"),
      ("呼号前缀", "/prefixes"),
      ("术语表", "/glossary"),
      ("莫尔斯电码", "/morse"),
      ("数字模式", "/modes"),
      ("通联实务", "/operating"),
    ],
  },
  ModuleCard {
    title: "工具",
    desc: "计算器 · 通联日志 · 实时数据",
    href: "/tools",
    icon: IconKind::Calculator,
    index: "03",
    offset: "md:mt-12",
    links: &[
      ("小工具", "/tools"),
      ("通联日志", "/log"),
      ("呼号抄收", "/callsign-copy"),
      ("太阳活动", "/solar"),
      ("业余卫星", "/satellites"),
      ("通知中心", "/notifications"),
    ],
  },
];

#[component]
pub fn HomePage() -> impl IntoView {
  set_title(DEFAULT_TITLE);
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
      // 基地首屏：角色 + 等级 + 主入口，再接今日任务。
      <div class="space-y-8">
        <BaseHero />
        <QuestBoard />
      </div>

      // 频谱条（SDR 意象）：保留作为电台气质的点缀，放进一块普通的像素窗口里。
      <div class="mt-8 w-full">
        <div class="pxl-window p-4 sm:p-5">
          <div class="mb-3 flex items-center justify-between text-xs text-muted-foreground">
            <span class="pxl-label">"Spectrum · 14.000 MHz"</span>
            <span class="pxl-label pxl-blink">{move || t("home.receiving")}</span>
          </div>
          <div class="spectrum-bar h-12 border-2 border-ink"></div>
          <div class="spectrum-ticks mt-2 h-3 opacity-60"></div>
          <div
            class="mt-1.5 flex justify-between text-xs tabular-nums text-muted-foreground"
            aria-hidden="true"
          >
            <span>"14.000"</span>
            <span>"14.175"</span>
            <span>"14.350"</span>
          </div>
        </div>
      </div>

      // 模块入口：三列不等宽（首列更宽）+ 阶梯式下沉，打破三等分的呆板网格。
      <div class="mt-16 grid grid-cols-1 gap-4 md:grid-cols-[1.3fr_1fr_1fr] md:items-start">
        {MODULES
          .iter()
          .enumerate()
          .map(|(i, m)| {
            view! {
              <a
                href=m.href
                // `motion-press`：悬停上移 2px、按下落回（硬阴影随之消失），见 `motion.css`。
                class=format!(
                  "reveal group pxl-window motion-press relative flex flex-col gap-5 overflow-hidden p-5 hover:bg-accent {}",
                  m.offset,
                )
                style=format!("animation-delay: {}ms", 300 + i * 90)
              >
                // 角落的序号水印：纯装饰。
                <span
                  aria-hidden="true"
                  class="pxl-title pointer-events-none absolute -bottom-3 right-1 select-none text-6xl leading-none text-foreground/10"
                >
                  {m.index}
                </span>
                <div class="relative flex items-center gap-3">
                  <div class="flex size-12 shrink-0 items-center justify-center border-2 border-ink bg-primary text-primary-foreground">
                    <Icon kind=m.icon class="size-6" />
                  </div>
                  <div class="min-w-0">
                    <div class="pxl-title text-sm">{move || t(m.title)}</div>
                    <div class="mt-1 text-xs text-muted-foreground">{move || t(m.desc)}</div>
                  </div>
                  <span aria-hidden="true" class="ml-auto shrink-0 text-muted-foreground">
                    <Icon kind=IconKind::ChevronRight class="size-6" />
                  </span>
                </div>
                <div class="relative flex flex-wrap gap-2">
                  {m
                    .links
                    .iter()
                    .map(|&(label, _)| {
                      view! {
                        <span class="pxl-badge text-xs">{move || t(label)}</span>
                      }
                    })
                    .collect_view()}
                </div>
              </a>
            }
          })
          .collect_view()}
      </div>

      // 快速开始练习（选择题库版本与类别）
      <div data-slot="card" class=card_class("mt-12")>
        <div data-slot="card-header" class=CARD_HEADER>
          <div data-slot="card-title" class=card_title_class("")>{move || t("home.quick-start")}</div>
          <div class="text-sm text-muted-foreground">{move || t("home.choose-a-bank-version")}</div>
        </div>
        <div data-slot="card-content" class=card_content_class("space-y-4")>
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
      </div>

      // 首屏以下的功能卡片随滚动逐个浮现，避免一进页面七张卡片同时刷出来。
      <div node_ref=stack class="space-y-6">
        <StudyPlanCard />
        <ReviewCard />
        <CardsCard />
        <DailyChallengeCard />
        <WantedCard />
        <DailyQuestion />
        <PropagationWidget />
      </div>
    </PageContainer>
  }
}
