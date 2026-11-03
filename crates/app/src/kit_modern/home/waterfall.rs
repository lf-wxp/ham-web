//! 最近 7 天的「信号瀑布」：每一列一天，颜色深浅表示当天作答量。
//!
//! 用 FT-8 瀑布图的读法（时间横向、强度纵向）承载真实的 `study-daily` 数据，
//! 而不是再加一组柱状图：像素风的学习计划卡已经有一张 14 天柱状图，
//! 现代风要的是同一件事的另一种版面语言。
//!
//! 纯 CSS 着色（不上 Canvas）：7 个格子不值得开一个渲染循环，
//! 也不需要在低端机上为一张静态快照付出一帧一画的代价。

use ham_web_core::study_plan::intensity_levels;
use leptos::prelude::*;

use crate::i18n::{t, tf, tp};
use crate::study;
use crate::util::local_today;

#[component]
pub(super) fn Waterfall() -> impl IntoView {
  let today = local_today();
  let days = study::load_daily().recent(&today, 7);
  let total: usize = days.iter().map(|(_, tally)| tally.answered as usize).sum();
  // 首页不为了给 7 个格子分档去异步加载题库算「今日目标」：
  // 这里按最近 7 日的最大值归一，表达的是「哪天亮、哪天暗」，不是完成率。
  let levels = intensity_levels(days.iter().map(|(_, tally)| tally.answered), None);

  view! {
    <div class="md-wf-wrap">
      <div class="md-wf-head">
        <span class="md-wf-title">{move || t("home.waterfall-title")}</span>
        <span class="md-wf-total">
          {move || tp("home.waterfall-total", total, &[&total.to_string()])}
        </span>
      </div>

      {(total == 0)
        .then(|| view! { <p class="md-wf-empty">{move || t("home.waterfall-empty")}</p> })}

      <div class="md-wf" role="img" aria-label=move || t("home.waterfall-summary")>
        {days
          .into_iter()
          .zip(levels)
          .map(|((day, tally), level)| {
            let answered = tally.answered;
            let is_today = day == today;
            // `YYYY-MM-DD` 只取 MM-DD：一年之内足够定位，也不会在 320px 上被截断。
            let stamp = day.get(5..).unwrap_or(&day).to_owned();
            let label = tf(
              "home.waterfall-cell",
              &[&stamp, &answered.to_string()],
            );
            view! {
              // 格子里不写字：深档的橙色底上放数字会掉到 4.5:1 以下，
              // 明细走 `title`，总量在右侧，读屏靠整图的 `aria-label`。
              <div class="md-wf-col" data-today=is_today.to_string()>
                <span class="md-wf-cell" data-level=level.to_string() title=label></span>
                <span class="md-wf-day">{stamp}</span>
              </div>
            }
          })
          .collect_view()}
      </div>
    </div>
  }
}
