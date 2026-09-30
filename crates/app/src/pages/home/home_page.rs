use ham_web_core::Bank;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::bank_selector::QuestionBankSelector;
use crate::components::bubble::Bubble;
use crate::data;
use crate::icons::{Icon, IconKind};
use crate::pages::{DEFAULT_TITLE, bank_href};
use crate::ui::{
  CARD_HEADER, Size, Variant, button_class, card_class, card_content_class, card_title_class,
};
use crate::util::set_title;

use super::daily_question::DailyQuestion;
use super::propagation_widget::PropagationWidget;

/// 首页模块入口。
struct ModuleCard {
  title: &'static str,
  desc: &'static str,
  href: &'static str,
  icon: IconKind,
  links: &'static [(&'static str, &'static str)],
}

const MODULES: &[ModuleCard] = &[
  ModuleCard {
    title: "考试中心",
    desc: "A/B/C 类题库 · 练习 · 模拟考试",
    href: "/practice",
    icon: IconKind::Timer,
    links: &[
      ("练习", "/practice"),
      ("模拟考试", "/exam"),
      ("分类浏览", "/browse"),
      ("闪卡刷题", "/flashcards"),
      ("错题集", "/mistakes"),
    ],
  },
  ModuleCard {
    title: "知识库",
    desc: "30+ 专题 · 呼号 · 术语 · 模式 · 传播 · 天线 · 通联",
    href: "/reference",
    icon: IconKind::BookOpen,
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
    links: &[
      ("小工具", "/tools"),
      ("通联日志", "/log"),
      ("太阳活动", "/solar"),
      ("业余卫星", "/satellites"),
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
      warn_text.set(format!(
        "题库 {} 暂不可用或为空，请先构建数据集",
        bank.get_untracked()
      ));
      warn_open.set(true);
    }
  };
  let practice_href = move || bank_href("/practice", version.get().as_deref(), bank.get());
  let exam_href = move || bank_href("/exam", version.get().as_deref(), bank.get());

  view! {
    <main class="container mx-auto max-w-5xl space-y-6 px-4 py-10 animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      // Hero
      <div class="py-8 text-center">
        <h1 class="text-3xl font-bold tracking-tight sm:text-4xl">"业余无线电"</h1>
        <p class="mt-3 text-sm text-muted-foreground sm:text-base">
          "一站式题库练习、知识速查与通联工具平台"
        </p>
        <div class="mt-6 flex flex-wrap justify-center gap-3">
          <a href="/practice" class=button_class(Variant::Default, Size::Default, "")>
            "开始练习"
          </a>
          <a href="/exam" class=button_class(Variant::Secondary, Size::Default, "")>
            "模拟考试"
          </a>
          <a href="/reference" class=button_class(Variant::Outline, Size::Default, "")>
            "浏览知识库"
          </a>
        </div>
      </div>

      // 模块入口
      <div class="grid grid-cols-1 gap-4 sm:grid-cols-3">
        {MODULES
          .iter()
          .map(|m| {
            view! {
              <a
                href=m.href
                class="group flex flex-col gap-4 rounded-xl border bg-card p-5 shadow-sm transition-colors hover:bg-accent/40"
              >
                <div class="flex items-center gap-3">
                  <div class="flex size-10 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                    <Icon kind=m.icon class="h-5 w-5" />
                  </div>
                  <div class="min-w-0">
                    <div class="font-semibold">{m.title}</div>
                    <div class="text-xs text-muted-foreground">{m.desc}</div>
                  </div>
                </div>
                <div class="flex flex-wrap gap-1.5">
                  {m
                    .links
                    .iter()
                    .map(|&(label, _)| {
                      view! {
                        <span class="rounded-full border bg-muted/40 px-2.5 py-0.5 text-xs text-muted-foreground">
                          {label}
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
      <div data-slot="card" class=card_class("")>
        <div data-slot="card-header" class=CARD_HEADER>
          <div data-slot="card-title" class=card_title_class("")>"快速开始练习"</div>
          <div class="text-sm text-muted-foreground">"选择题库版本与类别，进入练习或模拟考试"</div>
        </div>
        <div data-slot="card-content" class=card_content_class("space-y-4")>
          <QuestionBankSelector selected_version=version selected_bank=bank disabled=checking />
          <div class="flex flex-wrap gap-3 relative">
            <a
              href=practice_href
              data-slot="button"
              class=button_class(Variant::Default, Size::Default, "")
              on:click=guard
            >
              "开始练习"
            </a>
            <a
              href=exam_href
              data-slot="button"
              class=button_class(Variant::Secondary, Size::Default, "")
              on:click=guard
            >
              "开始模拟考试"
            </a>
            <Bubble open=warn_open text=warn_text />
          </div>
        </div>
      </div>

      <DailyQuestion />
      <PropagationWidget />
    </main>
  }
}
