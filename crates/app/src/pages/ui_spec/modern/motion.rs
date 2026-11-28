//! 动效一览：4 个时长 token、6 个可重播演示、减少动态效果开关。
//!
//! 每个演示引用现代风 CSS 里真实存在的 keyframes，效果与名称一一对应：
//! 扩散脉冲 / 声呐波纹 / 波形流动 / 同心电波 / 指针高光 / 骨架扫光。
//! 时长与缓动走 `--md-dur*` / `--md-ease`，不写死毫秒，也不新增 `md-*` 类名。

use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::components::common::SectionCard;
use crate::i18n::t;
use crate::ui::{Button, Switch, Variant};

use super::super::spec_data::computed;
use super::data::{MD_MOTION_DEMOS, MD_MOTION_TOKENS};

/// 减少动态效果时用 `animation:none`，其余按数据表里的声明播放。
fn anim(reduced: bool, value: &str) -> String {
  if reduced {
    String::from("animation:none;")
  } else {
    format!("animation:{value};")
  }
}

/// 重播：把舞台内所有动画元素摘掉 `animation`、强制布局、再写回。
///
/// 不能靠「信号先关再开」——Leptos 的响应式 style 不同步落 DOM，两次写入会被合并；
/// CSS animation 也只有声明重新出现时才会从头播放。
fn replay(stage: NodeRef<leptos::html::Div>, reduced: bool) {
  let Some(el) = stage.get() else {
    return;
  };
  let Ok(list) = el.query_selector_all("[data-animate]") else {
    return;
  };
  for i in 0..list.length() {
    let Some(node) = list.get(i) else {
      continue;
    };
    // 波形演示用的是 `<path>`：SVGElement 不是 HtmlElement，两条路径都要能拿到 style。
    let style = if let Ok(html) = node.clone().dyn_into::<web_sys::HtmlElement>() {
      Some(web_sys::HtmlElement::style(&html))
    } else if let Ok(svg) = node.clone().dyn_into::<web_sys::SvgElement>() {
      Some(web_sys::SvgElement::style(&svg))
    } else {
      None
    };
    let Some(style) = style else {
      continue;
    };
    let Some(element) = node.dyn_ref::<web_sys::Element>() else {
      continue;
    };
    let Some(value) = element.get_attribute("data-animation") else {
      continue;
    };
    let _ = style.set_property("animation", "none");
    let _ = element.get_bounding_client_rect().width();
    let _ = style.set_property("animation", if reduced { "none" } else { &value });
  }
}

#[component]
pub fn ModernMotion() -> impl IntoView {
  let reduced = RwSignal::new(false);

  view! {
    <SectionCard title=Signal::derive(move || t("design.motion-title"))>
      <div class="space-y-4 p-4">
        <p class="md-muted md-sm">{move || t("design.md-motion-hint")}</p>

        <section class="space-y-2">
          <h3 class="md-section-title">{move || t("design.md-motion-tokens")}</h3>
          <div data-spec-list="motion-tokens" class="divide-y">
            <For each=move || MD_MOTION_TOKENS key=|row| row.0 let:row>
              <div class="flex flex-wrap items-center justify-between gap-2 py-2">
                <code class="md-sm">{row.0}</code>
                <code class="md-muted md-sm">{row.1}</code>
                <span class="md-muted md-sm">{computed(row.0)}</span>
              </div>
            </For>
          </div>
        </section>

        <label class="flex cursor-pointer items-center gap-2 md-sm">
          <Switch
            checked=Signal::derive(move || reduced.get())
            on_change=Callback::new(move |on: bool| reduced.set(on))
            aria_label=Signal::derive(move || t("design.motion-reduced"))
          />
          <span>{move || t("design.motion-reduced")}</span>
        </label>

        <section class="space-y-2">
          <h3 class="md-section-title">{move || t("design.md-motion-demos-title")}</h3>
          <div data-spec-list="motion-demos" class="grid gap-3 sm:grid-cols-2">
            <For each=move || 0..MD_MOTION_DEMOS.len() key=|i| *i let:i>
              {
                let demo = MD_MOTION_DEMOS[i];
                let stage = NodeRef::<leptos::html::Div>::new();
                let click_stage = stage;
                view! {
                  <div class=crate::kit::tile_class("space-y-2 p-3")>
                    <div class="flex items-center justify-between gap-2">
                      <code class="md-muted md-sm">{demo.slug}</code>
                      <Button
                        variant=Variant::Outline
                        on_click=Callback::new(move |_| replay(click_stage, reduced.get()))
                      >
                        {move || t("design.md-motion-replay")}
                      </Button>
                    </div>
                    <div class="md-muted md-sm">{move || t(demo.label)}</div>
                    <div
                      node_ref=stage
                      data-motion-stage
                      class="relative flex items-center justify-center"
                      style="height:96px;padding:24px"
                      on:pointermove=crate::kit_modern::motion::spotlight
                    >
                      {move || {
                        let a = anim(reduced.get(), demo.animation);
                        match demo.slug {
                          // 扩散脉冲：常驻圆点 + 一圈向外扩散并淡出的信号环。
                          "ping" => view! {
                            <>
                              <span
                                data-motion-box
                                style="width:40px;height:40px;border-radius:var(--md-r-pill);background:var(--md-accent)"
                              ></span>
                              <span
                                data-animate
                                data-animation=demo.animation
                                style=format!("{a}position:absolute;width:40px;height:40px;border-radius:var(--md-r-pill);border:2px solid var(--md-accent)")
                              ></span>
                            </>
                          }
                            .into_any(),
                          // 声呐波纹：圆点向外发出一圈 box-shadow 波纹。
                          "sonar" => view! {
                            <span
                              data-motion-box
                              data-animate
                              data-animation=demo.animation
                              style=format!("{a}width:40px;height:40px;border-radius:var(--md-r-pill);background:var(--md-blue)")
                            ></span>
                          }
                            .into_any(),
                          // 波形流动：虚线沿正弦路径流动。
                          "wave" => view! {
                            <svg data-motion-box width="160" height="40" viewBox="0 0 160 40" aria-hidden="true">
                              <path
                                data-animate
                                data-animation=demo.animation
                                d="M0 20 Q 20 2 40 20 T 80 20 T 120 20 T 160 20"
                                style=format!("{a}fill:none;stroke:var(--md-accent);stroke-width:3;stroke-dasharray:14 10")
                              ></path>
                            </svg>
                          }
                            .into_any(),
                          // 同心电波：三圈依次向外扩散。
                          "ring-wave" => view! {
                            <>
                              <span
                                data-motion-box
                                style="width:16px;height:16px;border-radius:var(--md-r-pill);background:var(--md-accent)"
                              ></span>
                              <span
                                data-animate
                                data-animation=demo.animation
                                style=format!("{a}position:absolute;width:44px;height:44px;border-radius:var(--md-r-pill);border:2px solid var(--md-accent)")
                              ></span>
                              <span
                                data-animate
                                data-animation=demo.animation
                                style=format!("{a}position:absolute;width:44px;height:44px;border-radius:var(--md-r-pill);border:2px solid var(--md-accent);animation-delay:var(--md-dur-fast)")
                              ></span>
                              <span
                                data-animate
                                data-animation=demo.animation
                                style=format!("{a}position:absolute;width:44px;height:44px;border-radius:var(--md-r-pill);border:2px solid var(--md-accent);animation-delay:var(--md-dur)")
                              ></span>
                            </>
                          }
                            .into_any(),
                          // 指针高光：径向光斑跟随指针位置（--mx / --my 由 spotlight 写入）。
                          "spotlight" => view! {
                            <div
                              data-motion-box
                              data-animate
                              data-animation=demo.animation
                              style=format!("{a}width:140px;height:40px;border-radius:var(--md-r-sm);background:radial-gradient(60px circle at var(--mx,50%) var(--my,50%),var(--md-accent),transparent)")
                            ></div>
                          }
                            .into_any(),
                          // 骨架扫光：两条占位条上的渐变高光横向扫过。
                          _ => view! {
                            <div class="flex flex-col gap-2">
                              <div
                                data-motion-box
                                data-animate
                                data-animation=demo.animation
                                style=format!("{a}width:120px;height:12px;border-radius:var(--md-r-sm);background:linear-gradient(100deg,var(--md-surface-2) 30%,color-mix(in srgb,var(--md-surface-2) 55%,var(--md-surface-3)) 50%,var(--md-surface-2) 70%);background-size:260% 100%")
                              ></div>
                              <div
                                data-animate
                                data-animation=demo.animation
                                style=format!("{a}width:80px;height:12px;border-radius:var(--md-r-sm);background:linear-gradient(100deg,var(--md-surface-2) 30%,color-mix(in srgb,var(--md-surface-2) 55%,var(--md-surface-3)) 50%,var(--md-surface-2) 70%);background-size:260% 100%")
                              ></div>
                            </div>
                          }
                            .into_any(),
                        }
                      }}
                    </div>
                  </div>
                }
              }
            </For>
          </div>
        </section>
      </div>
    </SectionCard>
  }
}
