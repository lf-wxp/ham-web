use std::time::Duration;

use leptos::html;
use leptos::prelude::*;

/// 数字滚动总时长（ms），与 `--motion-normal`（300ms）同档、稍加余量让滚动更从容。
const COUNT_DURATION_MS: u64 = 600;
/// 步进次数：12 步、ease-out 收尾。
const COUNT_STEPS: u32 = 12;
const COUNT_STEP_MS: u64 = COUNT_DURATION_MS / COUNT_STEPS as u64;

/// 统计卡片：大数字 + 标签。`value` 可为静态 `usize` 或响应式 `Signal<usize>`。
///
/// 带 `motion-lift`（悬停抬起）并参与滚动浮现：统计格通常是「整块可扫读」的信息，
/// 逐个浮现比整页一次性出现更容易抓住注意力。
///
/// 数值变化时做一次 ease-out 的数字滚动（尊重「减少动态效果」偏好），
/// 让「答对后计数 +1」这类变化有明确的视觉反馈，而不是干巴巴地跳数字。
#[component]
pub fn Stat(#[prop(into)] label: String, #[prop(into)] value: Signal<usize>) -> impl IntoView {
  let node = NodeRef::<html::Div>::new();
  node.on_load(|el| crate::motion::reveal_on_scroll(&el, 0));

  // 展示值：与 `value` 分离，滚动时逐帧逼近，避免直接跳变。
  let display = RwSignal::new(value.get_untracked());
  // 代际计数：值快速连续变化时，过期动画的定时器按代际失效，不会把数字往回写。
  let generation = StoredValue::new(0u32);

  Effect::new(move |_| {
    let target = value.get();
    let from = display.get_untracked();
    generation.update_value(|g| *g += 1);
    let epoch = generation.get_value();
    if from == target {
      return;
    }
    // 减少动态效果：直接落位，不做滚动。
    if !crate::motion::motion_allowed() {
      display.set(target);
      return;
    }
    for i in 1..=COUNT_STEPS {
      let t = i as f64 / COUNT_STEPS as f64;
      // ease-out cubic：先快后慢，与 token 的 expo 手感一致。
      let eased = 1.0 - (1.0 - t).powi(3);
      let val = (from as f64 + (target as f64 - from as f64) * eased).round() as usize;
      set_timeout(
        move || {
          if generation.try_get_value() == Some(epoch) {
            display.set(val);
          }
        },
        Duration::from_millis(COUNT_STEP_MS * i as u64),
      );
    }
  });

  view! {
    <div node_ref=node class="motion-lift rounded-xl border bg-card p-3">
      <div class="text-2xl font-semibold tabular-nums">{move || display.get()}</div>
      <div class="text-xs text-muted-foreground">{label}</div>
    </div>
  }
}
