//! 战斗操作条：出招 / 继续 / 撤退。

use leptos::html;
use leptos::prelude::*;

use crate::i18n::t;
use crate::ui::{Button, ButtonLink, Size, Variant};

use super::run::Run;

/// 操作条。
///
/// 出招后立刻把焦点交给「继续」：键盘用户按 Enter 就能进下一回合，
/// 不必再 Tab 一路找回按钮（题目选项的 Enter 被单选组自己吃掉了，全局快捷键接不到）。
#[component]
pub(super) fn BattleActions(
  run: Run,
  on_attack: Callback<()>,
  on_continue: Callback<()>,
  #[prop(into)] retreat_href: Signal<String>,
) -> impl IntoView {
  let no_answer = Signal::derive(move || run.answer.with(Vec::is_empty));
  let primary = move || match run.feedback.get() {
    Some(fb) => {
      let node = NodeRef::<html::Button>::new();
      node.on_load(|b| {
        let _ = b.focus();
      });
      view! {
        <Button variant=Variant::Default size=Size::Lg node_ref=node on_click=on_continue>
          {if fb.finished { t("rpg.settle") } else { t("rpg.continue") }}
        </Button>
      }
      .into_any()
    }
    None => view! {
      <Button
        variant=Variant::Default
        size=Size::Lg
        disabled=no_answer
        title=Signal::derive(move || {
          if no_answer.get() { t("rpg.pick-answer-first") } else { t("rpg.attack") }
        })
        on_click=on_attack
      >
        {t("rpg.attack")}
      </Button>
    }
    .into_any(),
  };

  view! {
    <div class="flex flex-wrap items-center gap-3">
      {primary}
      <ButtonLink href=retreat_href variant=Variant::Ghost size=Size::Default>
        {move || t("rpg.retreat")}
      </ButtonLink>
    </div>
  }
}
