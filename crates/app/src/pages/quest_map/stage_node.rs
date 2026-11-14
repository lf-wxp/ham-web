//! 地图上的一个关卡节点。

use ham_web_core::rpg::{Stage, StageState};
use leptos::prelude::*;

use crate::cn::cn;
use crate::components::radio::SignalBars;
use crate::i18n::{t, tf, tp};
use crate::icons::PixelSprite;

/// 桌面端的横向错位：让关卡沿一条蜿蜒的小路排开，而不是整齐的一列。
///
/// 写成完整类名字面量：Tailwind 只扫描源码里的完整类名，运行时拼 `sm:ml-{n}` 不会生成样式。
const OFFSETS: [&str; 4] = ["sm:ml-0", "sm:ml-12", "sm:ml-24", "sm:ml-12"];

/// 关卡节点：可挑战 / 已通关是链接，未解锁是一块置灰的牌子。
#[component]
pub(super) fn StageNode(
  index: usize,
  stage: Stage,
  state: StageState,
  /// 进入战斗的链接。
  #[prop(into)]
  href: String,
) -> impl IntoView {
  let offset = OFFSETS[index % OFFSETS.len()];
  let stars = match state {
    StageState::Cleared(n) => n,
    _ => 0,
  };
  let sprite = match state {
    StageState::Locked => "badge_lock",
    StageState::Open => "node_flag",
    StageState::Cleared(_) => "node_coin",
  };
  let inner = move || {
    view! {
      <PixelSprite name=sprite scale=3 class=if state == StageState::Open { "pxl-bob" } else { "" } />
      <span class="min-w-0 flex-1">
        <span class="pxl-label block text-xs text-muted-foreground">
          {format!("{:02}", index + 1)}
        </span>
        <span class="block truncate">{t(stage.name)}</span>
        <span class="block text-xs text-muted-foreground">
          {tp("rpg.stage-questions", stage.count, &[&stage.count.to_string()])}
        </span>
      </span>
      {match state {
        StageState::Locked => {
          view! { <span class="pxl-label text-xs">{t("learning.locked")}</span> }.into_any()
        }
        _ => {
          view! {
            // 1–3 格信号（原来的星星）：电平格是装饰，读屏听不到；补一句文本，名字里也就带上了自己的格数。
            <SignalBars lit=stars />
            {(stars > 0)
              .then(|| {
                view! {
                  <span class="sr-only">
                    {tf("rpg.stars-value", &[&stars.to_string()])}
                  </span>
                }
              })}
          }
            .into_any()
        }
      }}
    }
  };

  if crate::kit::is_modern_at_mount() {
    let state_key = match state {
      StageState::Locked => "locked",
      StageState::Open => "open",
      StageState::Cleared(_) => "cleared",
    };
    let mark_icon = match state {
      StageState::Locked => crate::kit_modern::MIcon::Shield,
      StageState::Open => crate::kit_modern::MIcon::Play,
      StageState::Cleared(_) => crate::kit_modern::MIcon::Check,
    };
    let star_views = (1..=3u8)
      .map(|n| {
        view! {
          <crate::kit_modern::ModernIcon
            kind=crate::kit_modern::MIcon::Star
            class=if n <= stars { "md-star-on" } else { "md-star-off" }
          />
        }
      })
      .collect_view();

    // 记号必须是卡片的**兄弟**节点：带 `transform` 的元素会成为绝对定位后代的包含块。
    // 早先卡片 hover 会 `translateX`，记号放在卡片内部就被一起拖走，表现为
    // 「鼠标一上去，圆形图标跳到文字上」。现在 hover 只换描边与底色、不再位移，但兄弟结构要保留 ——
    // 以后给卡片加任何 transform（按压、入场）都不会再把记号带走。
    let mark = view! {
      <span class="md-stage-mark" data-state=state_key aria-hidden="true">
        <crate::kit_modern::ModernIcon kind=mark_icon />
      </span>
    };

    let body = view! {
      <span class="md-stage-body">
        <span class="md-stage-num">{format!("{:02}", index + 1)}</span>
        <span class="md-stage-name">{t(stage.name)}</span>
        <span class="md-stage-meta">
          {tp("rpg.stage-questions", stage.count, &[&stage.count.to_string()])}
        </span>
      </span>
      {if state == StageState::Locked {
        view! { <span class="md-stage-lock">{move || t("learning.locked")}</span> }.into_any()
      } else {
        view! {
          <span class="md-stage-stars" aria-hidden="true">{star_views}</span>
          {(stars > 0)
            .then(|| {
              view! {
                <span class="sr-only">{tf("rpg.stars-value", &[&stars.to_string()])}</span>
              }
            })}
        }
          .into_any()
      }}
    };

    return view! {
      <li class="md-stage-item">
        {mark}
        {if state == StageState::Locked {
          view! {
            <div class="md-stage" data-state="locked" aria-disabled="true">
              {body}
            </div>
          }
            .into_any()
        } else {
          view! {
            <a class="md-stage" data-state=state_key href=href.clone()>
              {body}
            </a>
          }
            .into_any()
        }}
      </li>
    }
    .into_any();
  }

  view! {
    <li class=cn(&["list-none", offset])>
      {match state {
        StageState::Locked => {
          view! {
            <div
              aria-disabled="true"
              class="pxl-window flex items-center gap-3 p-3 opacity-60"
            >
              {inner()}
            </div>
          }
            .into_any()
        }
        _ => {
          // 不再挂 `aria-label`：可见文本（序号、关卡名、题数、星级）已经是一个完整名字，
          // 手工拼一个更短的名字只会与它对不上（axe 的 label-content-name-mismatch）。
          view! {
            <a href=href class="pxl-window motion-press flex items-center gap-3 p-3 hover:bg-accent">
              {inner()}
            </a>
          }
            .into_any()
        }
      }}
    </li>
  }
  .into_any()
}
