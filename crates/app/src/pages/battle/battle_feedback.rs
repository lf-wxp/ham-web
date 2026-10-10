//! 每回合的反馈条：命中 / 受伤、经验、怪物还剩几击。

use ham_web_core::mistake_book::RecordOutcome;
use leptos::prelude::*;

use crate::i18n::{t, tp};

use super::run::Feedback;

/// 反馈条。始终渲染外层 `status` 区域（空时不占高度），内容出现时读屏会自动播报。
#[component]
pub(super) fn BattleFeedback(
  #[prop(into)] feedback: Signal<Option<Feedback>>,
  /// 复仇模式才讲「怪物还剩几击」：闯关里错题本的进度对玩家是噪音。
  revenge: bool,
) -> impl IntoView {
  view! {
    <div role="status" aria-live="polite">
      {move || {
        feedback
          .get()
          .map(|fb| {
            if fb.correct {
              view! {
                <div class="pxl-window motion-pop flex flex-wrap items-center gap-x-4 gap-y-1 p-3 text-sm">
                  <span class="pxl-title text-win-text">{t("rpg.hit-correct")}</span>
                  {(fb.turn.damage > 1)
                    .then(|| view! { <span class="pxl-label text-xs">{t("rpg.crit")}</span> })}
                  <span class="pxl-label text-xs text-gold-text">
                    {format!("+{} XP", fb.turn.xp)}
                  </span>
                  {revenge
                    .then(|| match fb.outcome {
                      Some(RecordOutcome::Mastered) => {
                        Some(view! { <span>{t("rpg.monster-defeated")}</span> }.into_any())
                      }
                      Some(RecordOutcome::Progressed { streak, target, .. }) => {
                        let left = target.saturating_sub(streak);
                        Some(
                          view! {
                            <span>
                              {tp("rpg.monster-left", u32::from(left), &[&left.to_string()])}
                            </span>
                          }
                            .into_any(),
                        )
                      }
                      _ => None,
                    })
                    .flatten()}
                </div>
              }
              .into_any()
            } else {
              view! {
                <div class="pxl-window motion-pop flex flex-wrap items-center gap-x-4 gap-y-1 p-3 text-sm">
                  <span class="pxl-title text-hp-text">{t("rpg.hit-wrong")}</span>
                  <span>{t("rpg.in-bestiary")}</span>
                  {(!fb.finished)
                    .then(|| view! { <span class="text-muted-foreground">{t("rpg.requeued")}</span> })}
                </div>
              }
              .into_any()
            }
          })
      }}
    </div>
  }
}
