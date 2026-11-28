//! 色彩体系：6 组 / 45 个 token，明暗切换、运行时计算值、选中详情与复制。

use std::collections::BTreeMap;

use leptos::prelude::*;

use crate::components::common::SectionCard;
use crate::i18n::t;
use crate::theme::{Theme, use_theme};
use crate::ui::{Button, Switch, Variant, push_success};
use crate::util::copy_text;

use super::super::spec_data::computed;
use super::data::{MD_TOKEN_GROUPS, all_md_tokens};

#[component]
pub fn ModernColors() -> impl IntoView {
  let theme = use_theme();
  let values = RwSignal::new(BTreeMap::<String, String>::new());
  let selected = RwSignal::new(String::from("--md-bg"));

  // `Theme::set` 先改 DOM 属性再通知信号，所以这里读到的已经是新主题。
  Effect::new(move |_| {
    let _ = theme.is_dark();
    let snapshot = all_md_tokens()
      .into_iter()
      .map(|name| (String::from(name), computed(name)))
      .collect::<BTreeMap<String, String>>();
    let _ = values.try_set(snapshot);
  });

  let value_of = move |name: &str| values.with(|map| map.get(name).cloned().unwrap_or_default());

  view! {
    <SectionCard title=Signal::derive(move || t("design.colors-title"))>
      <div class="space-y-4 p-4">
        <p class="md-muted md-sm">{move || t("design.md-colors-hint")}</p>
        <label class="flex cursor-pointer items-center gap-2 md-sm">
          <Switch
            checked=Signal::derive(move || theme.is_dark())
            on_change=Callback::new(move |on: bool| {
              theme.set(if on { Theme::Dark } else { Theme::Light });
            })
          />
          <span>{move || t("design.md-dark")}</span>
        </label>

        <div class=crate::kit::tile_class("flex flex-wrap items-center gap-3 p-3")>
          <span
            aria-hidden="true"
            style=move || format!(
              "display:block;width:64px;height:64px;flex:none;background:var({});border:1px solid var(--md-border);border-radius:var(--md-r-sm)",
              selected.get()
            )
          ></span>
          <div class="min-w-0 flex-1 space-y-1">
            <div class="md-muted md-sm">{move || t("design.md-color-selected")}</div>
            <code class="md-sm">{move || selected.get()}</code>
            <div class="md-muted md-sm">{move || value_of(&selected.get())}</div>
          </div>
          <Button
            variant=Variant::Outline
            on_click=Callback::new(move |_| {
              copy_text(&selected.get());
              push_success(t("design.colors-copied"));
            })
          >
            {move || t("design.colors-copy")}
          </Button>
        </div>

        <For each=move || MD_TOKEN_GROUPS key=|group| group.key let:group>
          <section data-spec-list="token-group" class="space-y-2">
            <h3 class="md-section-title">{move || t(group.key)}</h3>
            <div data-spec-list="tokens" class="grid gap-2 sm:grid-cols-2">
              <For each=move || group.tokens key=|name| *name let:name>
                <Button
                  variant=Variant::Outline
                  class="md-tile h-auto flex items-center gap-3 p-2 text-left"
                  on_click=Callback::new(move |_| selected.set(String::from(*name)))
                >
                  <span
                    aria-hidden="true"
                    style=format!(
                      "display:block;width:24px;height:24px;flex:none;background:var({name});border:1px solid var(--md-border)"
                    )
                  ></span>
                  <code class="min-w-0 flex-1 md-muted md-sm">{*name}</code>
                  <span class="md-muted md-sm">{move || value_of(name)}</span>
                </Button>
              </For>
            </div>
          </section>
        </For>
      </div>
    </SectionCard>
  }
}
