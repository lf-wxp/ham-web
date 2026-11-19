//! 动效：时长 token、一次性重播演示、逐格位移与「减少动态效果」开关。

use leptos::prelude::*;

use crate::components::common::SectionCard;
use crate::i18n::t;
use crate::theme::use_display_prefs;
use crate::ui::{Button, Switch, Variant};

use super::spec_data::MOTION_TOKENS;

/// 摘掉动画类、下一帧再加回来：CSS 动画只在类名重新出现时重播。
fn replay(playing: RwSignal<bool>) {
  playing.set(false);
  // `try_set`：下一帧到来前用户已离开页面时信号已销毁，不能再写。
  request_animation_frame(move || {
    let _ = playing.try_set(true);
  });
}

#[component]
pub fn MotionSection() -> impl IntoView {
  let prefs = use_display_prefs();
  let enter_open = RwSignal::new(true);
  let fade_open = RwSignal::new(true);
  let shake = RwSignal::new(false);
  let flash = RwSignal::new(false);
  let float = RwSignal::new(false);
  let shifted = RwSignal::new(false);

  view! {
    <SectionCard title=Signal::derive(move || t("design.motion-title"))>
      <div class="space-y-5 p-4">
        <p class="text-xs leading-relaxed text-muted-foreground">
          {move || t("design.motion-hint")}
        </p>

        <section class="space-y-2">
          <h3 class="pxl-sub-head">{move || t("design.motion-tokens")}</h3>
          <div class="divide-y border-y-2 border-ink">
            {MOTION_TOKENS
              .iter()
              .map(|token| {
                view! {
                  <div class="flex flex-wrap items-center justify-between gap-2 py-2">
                    <span class="text-xs text-foreground">{move || t(token.key)}</span>
                    <code class="text-xs text-muted-foreground">{token.var}</code>
                    <code class=crate::kit::tag_class("text-xs")>{token.value}</code>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="space-y-3">
          <h3 class="pxl-sub-head">{move || t("design.motion-demos")}</h3>
          <div class="grid gap-3 sm:grid-cols-2">
            <div class=crate::kit::tile_class("space-y-3 p-3")>
              <div
                class="pxl-window pxl-enter p-4 text-xs text-foreground"
                data-state=move || if enter_open.get() { "open" } else { "closed" }
              >
                {move || t("design.motion-enter")}
              </div>
              <Button
                variant=Variant::Outline
                on_click=Callback::new(move |_| enter_open.update(|v| *v = !*v))
              >
                {move || t("design.motion-replay")}
              </Button>
            </div>

            <div class=crate::kit::tile_class("space-y-3 p-3")>
              <div
                class="pxl-window pxl-enter-fade p-4 text-xs text-foreground"
                data-state=move || if fade_open.get() { "open" } else { "closed" }
              >
                {move || t("design.motion-fade")}
              </div>
              <Button
                variant=Variant::Outline
                on_click=Callback::new(move |_| fade_open.update(|v| *v = !*v))
              >
                {move || t("design.motion-replay")}
              </Button>
            </div>

            <div class=crate::kit::tile_class("space-y-3 p-3")>
              <div
                class=move || {
                  if shake.get() {
                    "pxl-shake border-2 border-ink bg-card p-4 text-xs text-foreground"
                  } else {
                    "border-2 border-ink bg-card p-4 text-xs text-foreground"
                  }
                }
              >
                {move || t("design.motion-shake")}
              </div>
              <Button
                variant=Variant::Outline
                on_click=Callback::new(move |_| replay(shake))
              >
                {move || t("design.motion-replay")}
              </Button>
            </div>

            <div class=crate::kit::tile_class("space-y-3 p-3")>
              <div
                class=move || {
                  if flash.get() {
                    "pxl-flash border-2 border-ink bg-card p-4 text-xs text-foreground"
                  } else {
                    "border-2 border-ink bg-card p-4 text-xs text-foreground"
                  }
                }
              >
                {move || t("design.motion-flash")}
              </div>
              <Button
                variant=Variant::Outline
                on_click=Callback::new(move |_| replay(flash))
              >
                {move || t("design.motion-replay")}
              </Button>
            </div>

            <div class=crate::kit::tile_class("space-y-3 p-3")>
              <div
                class=move || {
                  if float.get() {
                    "pxl-float-up border-2 border-ink bg-card p-4 text-xs text-foreground"
                  } else {
                    "border-2 border-ink bg-card p-4 text-xs text-foreground"
                  }
                }
              >
                {move || t("design.motion-float")}
              </div>
              <Button
                variant=Variant::Outline
                on_click=Callback::new(move |_| replay(float))
              >
                {move || t("design.motion-replay")}
              </Button>
            </div>

            <div class=crate::kit::tile_class("space-y-3 p-3")>
              <div class=crate::kit::tile_class("h-12 overflow-hidden p-1") aria-hidden="true">
                <div
                  class="h-8 w-8 bg-primary"
                  style=move || {
                    format!(
                      "translate: {} 0; transition: translate var(--motion-normal) var(--ease-standard)",
                      if shifted.get() { "300%" } else { "0" }
                    )
                  }
                ></div>
              </div>
              <Button
                variant=Variant::Outline
                on_click=Callback::new(move |_| shifted.update(|v| *v = !*v))
              >
                {move || t("design.motion-steps-play")}
              </Button>
            </div>
          </div>
        </section>

        <section class=crate::kit::tile_class("space-y-2 p-3")>
          <label class="flex cursor-pointer items-center gap-2 text-xs text-foreground">
            <Switch
              checked=Signal::derive(move || !prefs.pixel_motion())
              on_change=Callback::new(move |on: bool| prefs.set_pixel_motion(!on))
              aria_label=Signal::derive(move || t("design.motion-reduced"))
            />
            {move || t("design.motion-reduced")}
          </label>
          <p class="text-xs leading-relaxed text-muted-foreground">
            {move || t("design.motion-reduced-hint")}
          </p>
        </section>
      </div>
    </SectionCard>
  }
}
