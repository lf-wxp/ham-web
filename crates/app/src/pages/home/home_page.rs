use ham_web_core::Bank;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::bank_selector::QuestionBankSelector;
use crate::components::bubble::Bubble;
use crate::components::study_plan_card::StudyPlanCard;
use crate::data;
use crate::icons::{Icon, IconKind};
use crate::pages::{DEFAULT_TITLE, bank_href};
use crate::ui::{
  ButtonLink, CARD_HEADER, Size, Variant, card_class, card_content_class, card_title_class,
};
use crate::util::set_title;

use super::cards_card::CardsCard;
use super::daily_challenge_card::DailyChallengeCard;
use super::daily_question::DailyQuestion;
use super::propagation_widget::PropagationWidget;
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
    desc: "30+ 专题 · 呼号 · 术语 · 模式 · 传播 · 天线 · 通联",
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
    <div class="container relative mx-auto max-w-5xl px-4 pb-12 pt-14 sm:pt-24">
      // Hero
      // `isolate`：自成一个层叠上下文，装饰层的 `z-index: -1` 只在 Hero 内部「垫底」，
      // 不会钻到页面级的极光 / Web Threads 背景层后面去。
      // `overflow-x-clip`：窄屏上光团比容器宽，横向裁掉；用 `clip` 而不是 `hidden`，
      // 因为 `hidden` 会把 y 轴也变成滚动容器，裁掉光团上探与频谱面板的远光。
      <section class="relative isolate flex flex-col items-center overflow-x-clip text-center">
        <div class="hero-orb" aria-hidden="true"></div>
        <div class="hero-grid" aria-hidden="true"></div>

        <span
          class="reveal inline-flex items-center gap-2 rounded-full border bg-card/70 px-3.5 py-1.5 font-mono text-[11px] font-medium tracking-[0.14em] text-muted-foreground shadow-sm backdrop-blur-md"
          style="animation-delay: 0ms"
        >
          <span class="relative flex size-1.5">
            <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-primary opacity-60"></span>
            <span class="relative inline-flex size-1.5 rounded-full bg-primary"></span>
          </span>
          {move || t("knowledge.amateur-radio")}
        </span>

        // 展示字体 + 随视口缩放的字号：拉丁文用 Unbounded，中文落到系统字体，
        // `clamp` 的下限按西语最长的标题（Radioafición）在 375px 屏上不溢出来定。
        <h1
          class="reveal mt-7 font-display text-[clamp(2rem,8.5vw,5.25rem)] font-bold leading-[1.05] tracking-tight"
          style="animation-delay: 60ms"
        >
          <span class="hero-gradient-text">{move || t("shell.amateur-radio")}</span>
        </h1>

        <p
          class="reveal mt-5 max-w-2xl text-balance text-base text-foreground/75 sm:text-lg"
          style="animation-delay: 120ms"
        >
          {move || t("home.one-platform-for-exam")}
        </p>

        <div class="reveal mt-9 flex flex-wrap justify-center gap-3" style="animation-delay: 180ms">
          <ButtonLink
            href="/practice"
            variant=Variant::Default
            size=Size::Lg
          >
            {move || t("home.start-practice")}
          </ButtonLink>
          <ButtonLink
            href="/exam"
            variant=Variant::Secondary
            size=Size::Lg
          >
            {move || t("shell.mock-exam")}
          </ButtonLink>
          <ButtonLink
            href="/reference"
            variant=Variant::Outline
            size=Size::Lg
          >
            {move || t("home.browse-knowledge")}
          </ButtonLink>
        </div>

        // 频谱瀑布视觉（SDR 意象）：整页唯一一块「信号玻璃」面板，
        // 强模糊 + 渐变描边（`glass` + `edge-glow`），底下的刻度读数取 20 米波段的起 / 中 / 止频率。
        <div class="reveal mt-14 w-full max-w-2xl" style="animation-delay: 240ms">
          <div class="glass edge-glow relative rounded-3xl border p-4 sm:p-5">
            <div class="mb-4 flex items-center justify-between text-xs font-medium text-muted-foreground">
              <span class="font-mono tracking-widest uppercase">"Spectrum · 14.000 MHz"</span>
              <span class="inline-flex items-center gap-1.5">
                <span class="relative flex size-1.5">
                  <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-primary opacity-60"></span>
                  <span class="relative inline-flex size-1.5 rounded-full bg-primary"></span>
                </span>
                {move || t("home.receiving")}
              </span>
            </div>
            <div class="spectrum-bar h-16 rounded-2xl border"></div>
            <div class="spectrum-ticks mt-2 h-3 rounded-sm opacity-60"></div>
            <div
              class="mt-1.5 flex justify-between font-mono text-[10px] tabular-nums text-muted-foreground/80"
              aria-hidden="true"
            >
              <span>"14.000"</span>
              <span>"14.175"</span>
              <span>"14.350"</span>
            </div>
          </div>
        </div>
      </section>

      // 模块入口：三列不等宽（首列更宽）+ 阶梯式下沉，打破三等分的呆板网格。
      // 卡片靠 `spotlight` 在鼠标下泛光、`edge-glow` 在悬停时点亮渐变描边。
      <div class="mt-16 grid grid-cols-1 gap-4 md:grid-cols-[1.3fr_1fr_1fr] md:items-start">
        {MODULES
          .iter()
          .enumerate()
          .map(|(i, m)| {
            view! {
              <a
                href=m.href
                // `motion-press` + `active:translate-y-0`：按下时卡片「落回原位」，
                // 和抬起方向相反，形成完整的按压手感（桌面 hover 抬起，触屏无 hover
                // 但有 active，两端都有反馈）。过渡属性由 `motion-press` 统一给出
                // （含 `translate`），这里不再写 `transition-*`。
                class=format!(
                  "reveal group spotlight edge-glow relative flex flex-col gap-5 overflow-hidden rounded-2xl border bg-card p-5 shadow-sm hover:-translate-y-1.5 hover:border-primary/40 hover:shadow-xl hover:shadow-primary/15 motion-press active:translate-y-0 {}",
                  m.offset,
                )
                style=format!("animation-delay: {}ms", 300 + i * 90)
              >
                // 角落的序号水印：纯装饰，悬停时染上主色。
                <span
                  aria-hidden="true"
                  class="pointer-events-none absolute -bottom-5 -right-1 select-none font-display text-8xl font-bold leading-none text-foreground/[0.05] transition-colors duration-500 group-hover:text-primary/15"
                >
                  {m.index}
                </span>
                <div class="relative flex items-center gap-3">
                  <div class="flex size-12 shrink-0 items-center justify-center rounded-2xl bg-primary/10 text-primary ring-1 ring-primary/15 transition-[background-color,color,box-shadow,scale] duration-300 group-hover:scale-105 group-hover:bg-primary group-hover:text-primary-foreground group-hover:shadow-lg group-hover:shadow-primary/40">
                    <Icon kind=m.icon class="h-5 w-5" />
                  </div>
                  <div class="min-w-0">
                    <div class="text-lg font-semibold tracking-tight">{move || t(m.title)}</div>
                    <div class="mt-0.5 text-xs text-muted-foreground">{move || t(m.desc)}</div>
                  </div>
                  <span
                    aria-hidden="true"
                    class="ml-auto grid size-8 shrink-0 place-items-center rounded-full border bg-background/60 text-muted-foreground transition-[translate,background-color,color,border-color] duration-300 group-hover:translate-x-0.5 group-hover:border-primary group-hover:bg-primary group-hover:text-primary-foreground"
                  >
                    <Icon kind=IconKind::ChevronRight class="size-4" />
                  </span>
                </div>
                <div class="relative flex flex-wrap gap-1.5">
                  {m
                    .links
                    .iter()
                    .map(|&(label, _)| {
                      view! {
                        <span class="rounded-full border bg-muted/50 px-2.5 py-0.5 text-xs text-muted-foreground transition-colors group-hover:bg-accent group-hover:text-accent-foreground">
                          {move || t(label)}
                        </span>
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
    </div>
  }
}
