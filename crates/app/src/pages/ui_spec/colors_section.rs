//! 色彩体系：色块与取值都从运行时计算样式读，切换方案 / 明暗即时生效。

use std::collections::BTreeMap;

use ham_web_core::color_scheme::SCHEMES;
use leptos::prelude::*;

use crate::components::common::SectionCard;
use crate::i18n::t;
use crate::theme::{Theme, use_display_prefs, use_theme};
use crate::ui::{Button, Chip, ChipGroup, Switch, Variant, push_success};
use crate::util::copy_text;

use super::spec_data::{TOKEN_GROUPS, all_tokens, computed, scheme_key};

#[component]
pub fn ColorsSection() -> impl IntoView {
  let prefs = use_display_prefs();
  let theme = use_theme();
  let values = RwSignal::new(BTreeMap::<String, String>::new());
  let selected = RwSignal::new(String::from("--primary"));

  // `set_scheme` / `Theme::set` 都是先改 DOM 属性再通知信号，所以这里读到的已经是新方案。
  Effect::new(move |_| {
    let _ = prefs.scheme();
    let _ = theme.is_dark();
    let snapshot = all_tokens()
      .into_iter()
      .map(|name| (String::from(name), computed(name)))
      .collect::<BTreeMap<String, String>>();
    let _ = values.try_set(snapshot);
  });

  // 用 `with` 借用：每个色块各有一个读它的闭包，`get()` 会让每次刷新克隆整张表。
  let value_of =
    move |name: &str| -> String { values.with(|map| map.get(name).cloned().unwrap_or_default()) };

  view! {
    <SectionCard title=Signal::derive(move || t("design.colors-title"))>
      <div class="space-y-4 p-4">
        <p class="text-xs leading-relaxed text-muted-foreground">
          {move || t("design.colors-hint")}
        </p>
        <div class=crate::kit::tile_class("space-y-3 p-3")>
          <div class="flex flex-wrap items-center justify-between gap-2">
            <span class="pxl-label text-xs text-muted-foreground">
              {move || t("design.colors-schemes")}
            </span>
            <label class="flex cursor-pointer items-center gap-2 text-xs text-muted-foreground">
              <Switch
                checked=Signal::derive(move || theme.is_dark())
                on_change=Callback::new(move |on: bool| {
                  theme.set(if on { Theme::Dark } else { Theme::Light });
                })
                aria_label=Signal::derive(move || t("design.colors-dark"))
              />
              {move || {
                if theme.is_dark() { t("design.colors-dark") } else { t("design.colors-light") }
              }}
            </label>
          </div>
          <ChipGroup
            value=Signal::derive(move || String::from(prefs.scheme()))
            on_change=Callback::new(move |v: String| prefs.set_scheme(&v))
            aria_label=Signal::derive(move || t("design.colors-schemes"))
          >
            {SCHEMES
              .iter()
              .map(|scheme| {
                let key = scheme_key(scheme.id);
                view! { <Chip value=scheme.id>{move || t(key)}</Chip> }
              })
              .collect_view()}
          </ChipGroup>
          <p class="text-xs text-muted-foreground">
            {move || t("design.colors-schemes-hint")}
          </p>
        </div>

        <div class=crate::kit::tile_class("flex flex-wrap items-center gap-4 p-3")>
          <div
            class="h-16 w-16 shrink-0 border-2 border-ink"
            style=move || format!("background: var({})", selected.get())
          ></div>
          <div class="min-w-0 flex-1 space-y-1">
            <span class="pxl-label text-xs text-muted-foreground">
              {move || t("design.colors-detail")}
            </span>
            <code class="block text-base break-all text-foreground">
              {move || selected.get()}
            </code>
            <code class="block text-xs break-all text-muted-foreground">
              {move || format!("{}: {}", t("design.colors-value"), value_of(&selected.get()))}
            </code>
          </div>
          <Button
            variant=Variant::Secondary
            on_click=Callback::new(move |_| {
              copy_text(&selected.get());
              push_success(t("design.colors-copied"));
            })
          >
            {move || t("design.colors-copy")}
          </Button>
        </div>

        <div class="space-y-4">
          {TOKEN_GROUPS
            .iter()
            .map(|group| {
              let key = group.key;
              view! {
                <section class="space-y-2">
                  <h3 class="pxl-sub-head">{move || t(key)}</h3>
                  <div class="grid grid-cols-2 gap-2 sm:grid-cols-3 md:grid-cols-4">
                    {group
                      .tokens
                      .iter()
                      .map(|token| {
                        let name = *token;
                        view! {
                          <Button
                            variant=Variant::Outline
                            class="h-auto flex flex-col items-stretch gap-1 p-2 min-w-0 text-left"
                            aria_label=Signal::derive(move || String::from(name))
                            on_click=Callback::new(move |_| selected.set(String::from(name)))
                          >
                            <span
                              class="h-10 w-full border-2 border-ink"
                              style=format!("background: var({name})")
                            ></span>
                            <code class="truncate text-xs text-foreground">{name}</code>
                            <code class="truncate text-xs text-muted-foreground">
                              {move || value_of(name)}
                            </code>
                          </Button>
                        }
                      })
                      .collect_view()}
                  </div>
                </section>
              }
            })
            .collect_view()}
        </div>
        <p class="text-xs text-muted-foreground">{move || t("design.colors-live")}</p>
      </div>
    </SectionCard>
  }
}
