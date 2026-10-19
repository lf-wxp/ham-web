//! 地图上的一个关卡节点。

use ham_web_core::rpg::{Stage, StageState};
use leptos::prelude::*;

use crate::cn::cn;
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
            <span class="flex shrink-0 items-center" aria-hidden="true">
              {(1..=3u8)
                .map(|n| {
                  view! {
                    <PixelSprite
                      name="badge_star"
                      scale=2
                      class=if n <= stars { "" } else { "opacity-25 grayscale" }
                    />
                  }
                })
                .collect_view()}
            </span>
            // 星星是贴图，读屏听不到；补一句文本，名字里也就带上了自己的星级。
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
}
