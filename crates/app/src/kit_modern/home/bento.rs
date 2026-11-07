//! 现代风首页的三张行动卡：大小与底色各不相同，只有第一张是实心品牌色。
//!
//! 刻意不用「六宫格等大磁贴」：等大 + 同色是后台仪表盘的特征。这里三张卡各自对应一个
//! 明确的动作（练习 / 模拟考试 / 错题），数字只出现在「今日待复习」这种页面上没有重复的地方 ——
//! 上方数据条已经给过已答数与正确率，卡片再抄一遍就变成报表了。

use leptos::prelude::*;

use crate::i18n::{t, tf};

#[component]
pub fn Bento() -> impl IntoView {
  // 光标跟随的高光：指针位置写进 `--mx / --my`，由 `.md-spotlight::after` 画柔光。
  use crate::kit_modern::motion::spotlight;
  let due = crate::study::load_book().due_count(crate::util::now_ms());
  let due_text = due.to_string();
  // 文案在 `move ||` 里现算：组件体里算一次会停在首次渲染的语言。
  let due_foot_text = due_text.clone();

  view! {
    // 焦点卡的圆锥描边是逐帧主线程重绘的动画：滚出视口就暂停（`motion::pause_offscreen_in`）。
    <section class="mt-14" data-pause-offscreen="">
      <div class="mb-4 flex items-baseline justify-between gap-4">
        <h2 class="md-h2">{move || t("home.quick-actions")}</h2>
      </div>

      <div class="md-bento">
        <a class="md-feature md-feature-accent md-spotlight" href="/practice" on:pointermove=spotlight>
          <div class="flex flex-col gap-2">
            <span class="md-feature-kicker">{move || t("home.continue-learning")}</span>
            <span class="md-feature-title">{move || t("home.start-practice")}</span>
          </div>
          <span class="md-feature-foot">
            <span>{move || t("home.a-b-c-question")}</span>
            <crate::kit_modern::ModernIcon kind=crate::kit_modern::MIcon::ArrowRight />
          </span>
        </a>

        <a class="md-feature md-feature-quiet md-spotlight" href="/exam" on:pointermove=spotlight>
          <span class="md-feature-kicker">{move || t("shell.mock-exam")}</span>
          <div class="flex flex-col gap-2">
            <span class="md-feature-title">{move || t("home.start-mock-exam")}</span>
            <span class="md-feature-foot">
              <span>{move || t("exam.take-a-mock-exam")}</span>
            </span>
          </div>
        </a>

        <a class="md-feature md-feature-ink md-spotlight" href="/mistakes" on:pointermove=spotlight>
          <span class="md-feature-kicker">{move || t("shell.mistakes")}</span>
          <div class="flex flex-col gap-2">
            <span class="md-feature-value">{due_text}</span>
            <span class="md-feature-foot">
              <span>{move || tf("common.review-mistakes", &[&due_foot_text])}</span>
            </span>
          </div>
        </a>
      </div>
    </section>
  }
}
