//! 现代风首屏：电波带 + 大字号主张 + 发射塔主视觉。
//!
//! 与像素版 `pages/home/base_hero.rs` 是两套完全不同的构图：那边是「角色铭牌 + 精灵 + 经验条」
//! 的掌机基地，这边是面向个人用户的落地页 —— 一句主张、两个动作、一座正在发报的塔。
//! 主视觉是纯 SVG（随主题变色、无位图资产、无网络请求），同心电波与底部曲线都由 CSS 驱动。

use std::cell::Cell;
use std::rc::Rc;

use leptos::ev;
use leptos::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

use crate::i18n::{t, tf};
use crate::rpg::{load_profile, rank_label};

#[component]
pub fn ModernHero() -> impl IntoView {
  let profile = load_profile();
  let level = profile.level;
  let percent = level.percent();
  // 只留标量：HUD 上的等级 / 称号 / 经验 / 连续天数都要跟着切语言重算，
  // 取值必须在 `move ||` 闭包里现算（组件体里算一次会停在首次渲染的语言）。
  let level_no = level.level;
  let is_max = level.is_max();
  let into_level = level.into_level;
  let needed = level.needed;
  let streak = profile.streak;

  // 视差：**JS 驱动**，不用 `animation-timeline`。
  // 原因：scroll-timeline 只在部分浏览器可用，而且被 `overflow:hidden` 的首屏裁掉后几乎看不见；
  // 这里直接按滚动量写 transform，三层不同速度，任何现代浏览器都能看到分层。
  let copy_ref = NodeRef::<leptos::html::Div>::new();
  let art_ref = NodeRef::<leptos::html::Div>::new();
  let eq_ref = NodeRef::<leptos::html::Div>::new();

  // 系统「减少动态效果」的查询对象只建一次：`match_media` 每次调用都会新建一个
  // `MediaQueryList`，放在 scroll 回调里（每秒几十次）是纯浪费。对象本身是活的，
  // 用户在系统设置里改了偏好，下一次 `matches()` 就读到新值。
  let reduced = web_sys::window().and_then(|w| {
    w.match_media("(prefers-reduced-motion: reduce)")
      .ok()
      .flatten()
  });

  // 下面三处读 `NodeRef` 一律用 `try_get_untracked()`：这里跑在滚动回调 / rAF 里（没有观察者，
  // 订阅既建立不起来也会在 dev 构建刷「outside a reactive tracking context」），
  // 元素挂载时机由 `Effect` 的首帧调用兜住。用 `try_*` 是因为已排队的 rAF 可能在组件卸载
  // 之后才执行，那时 `NodeRef` 已随 Owner 释放，直接读会 panic。
  let apply: Rc<dyn Fn()> = Rc::new(move || {
    if reduced.as_ref().is_some_and(|m| m.matches()) {
      return;
    }
    let y = web_sys::window()
      .and_then(|w| w.scroll_y().ok())
      .unwrap_or(0.0)
      .clamp(0.0, 900.0);

    // 写 `transform` / `opacity` 两个属性，而不是 `set_attribute("style", …)`：
    // 后者会整体替换内联样式，之后任何代码给这三层写的内联属性都会被抹掉。
    if let Some(el) = copy_ref.try_get_untracked().flatten() {
      let style = web_sys::HtmlElement::style(&el);
      let _ = style.set_property("transform", &format!("translateY({:.1}px)", -y * 0.18));
      let _ = style.set_property("opacity", &format!("{:.2}", (1.0 - y / 1100.0).max(0.45)));
    }
    if let Some(el) = art_ref.try_get_untracked().flatten() {
      let style = web_sys::HtmlElement::style(&el);
      let _ = style.set_property(
        "transform",
        &format!(
          "translateY({:.1}px) scale({:.3})",
          y * 0.30,
          1.0 + y * 0.0004
        ),
      );
    }
    if let Some(el) = eq_ref.try_get_untracked().flatten() {
      let style = web_sys::HtmlElement::style(&el);
      let _ = style.set_property("transform", &format!("translateY({:.1}px)", y * 0.15));
      let _ = style.set_property("opacity", &format!("{:.2}", (1.0 - y / 900.0).max(0.2)));
    }
  });

  // 滚动合帧：scroll 事件每帧可能来好几次，全都直接写样式等于每帧做两三遍重排。
  // 与 `kit_modern::motion::use_scroll_spy` 同一写法（一处改了另一处要跟上）。
  let ticking = Rc::new(Cell::new(false));
  let schedule: Rc<dyn Fn()> = {
    let apply = Rc::clone(&apply);
    let ticking = Rc::clone(&ticking);
    Rc::new(move || {
      if ticking.replace(true) {
        return;
      }
      let apply = Rc::clone(&apply);
      let ticking = Rc::clone(&ticking);
      let cb = Closure::once_into_js(move || {
        ticking.set(false);
        apply();
      });
      if let Some(w) = web_sys::window() {
        let _ = w.request_animation_frame(cb.unchecked_ref());
      }
    })
  };

  // 首帧同步执行一次：刷新后浏览器会恢复滚动位置，走 rAF 会先闪一帧「没有视差」。
  Effect::new({
    let apply = Rc::clone(&apply);
    move |_| apply()
  });

  let handle = {
    let schedule = Rc::clone(&schedule);
    window_event_listener(ev::scroll, move |_| schedule())
  };
  let handle = SendWrapper::new(Some(handle));
  on_cleanup(move || {
    if let Some(h) = handle.take() {
      h.remove();
    }
  });

  view! {
    // `data-pause-offscreen`：滚出视口时暂停极光、电波、卫星这些循环动画（`motion::pause_offscreen_in`）。
    <section class="md-hero-band" data-pause-offscreen="">
      <div class="md-aurora" aria-hidden="true"></div>
      <div class="md-shell md-hero-grid">
        <div class="md-hero-copy" node_ref=copy_ref>
          <span class="md-badge">
            <span class="md-badge-dot" aria-hidden="true"></span>
            {move || t("home.hero-badge")}
          </span>

          // 标题拆成两段只为让后半句上品牌色：中文断行在逗号处，英文/西语按语序各自成立。
          <h1 class="md-display">
            <span>{move || t("home.hero-title-a")}</span>
            <span class="md-display-gradient">{move || t("home.hero-title-b")}</span>
          </h1>

          <p class="md-lede">{move || t("home.one-platform-for-exam")}</p>

          <div class="md-cta-row flex flex-wrap items-center gap-3">
            <a class="md-btn md-btn-accent md-btn-lg" href="/practice">
              {move || t("home.start-practice")}
              <crate::kit_modern::ModernIcon kind=crate::kit_modern::MIcon::ArrowRight />
            </a>
            <a class="md-btn md-btn-quiet md-btn-lg" href="/exam">
              <crate::kit_modern::ModernIcon kind=crate::kit_modern::MIcon::Play />
              {move || t("shell.mock-exam")}
            </a>
          </div>

          <p class="md-trust">{move || t("home.hero-trust")}</p>
        </div>

        <div class="md-hero-art" node_ref=art_ref>
          // 几何底盘：一个实线大圆 + 一条虚线内圈 + 一条竖直轴线。
          // 它把塔、同心电波、VFO 表盘收进同一个圆心，首屏从「一张插画」变成「一张方向图」。
          <div class="md-hero-disc" aria-hidden="true"></div>

          // 发射塔 + 三圈同心电波 + 地平线网格。全部用 currentColor / 主题变量，暗色下自动变「电光」。
          <svg viewBox="0 0 420 360" aria-hidden="true">
            <defs>
              <linearGradient id="md-art-ring" x1="0" y1="0" x2="1" y2="1">
                <stop offset="0%" stop-color="var(--md-blue)" stop-opacity="0.85" />
                <stop offset="100%" stop-color="var(--md-violet)" stop-opacity="0.1" />
              </linearGradient>
              <linearGradient id="md-art-mast" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stop-color="var(--md-blue)" stop-opacity="0.95" />
                <stop offset="100%" stop-color="var(--md-blue)" stop-opacity="0.25" />
              </linearGradient>
              <linearGradient id="md-art-horizon" x1="0" y1="0" x2="1" y2="0">
                <stop offset="0%" stop-color="var(--md-violet)" stop-opacity="0" />
                <stop offset="50%" stop-color="var(--md-blue)" stop-opacity="0.55" />
                <stop offset="100%" stop-color="var(--md-violet)" stop-opacity="0" />
              </linearGradient>
            </defs>

            <defs>
              <radialGradient id="md-moon" cx="50%" cy="50%" r="50%">
                <stop offset="0%" stop-color="var(--md-orange)" stop-opacity="0.55" />
                <stop offset="55%" stop-color="var(--md-violet)" stop-opacity="0.18" />
                <stop offset="100%" stop-color="var(--md-violet)" stop-opacity="0" />
              </radialGradient>
            </defs>
            // 月亮 / 电离层辉光：塔背后的光源，让剪影有前后层次。
            <circle cx="210" cy="96" r="132" fill="url(#md-moon)" />

            // 星点：极低透明度的装饰
            <g fill="var(--md-fg)" opacity="0.35">
              <circle cx="52" cy="58" r="1.6" />
              <circle cx="96" cy="34" r="1.1" />
              <circle cx="352" cy="72" r="1.6" />
              <circle cx="392" cy="42" r="1.1" />
              <circle cx="330" cy="26" r="1" />
              <circle cx="24" cy="130" r="1" />
            </g>

            // 同心电波（CSS 错峰扩散）
            <g fill="none" stroke="url(#md-art-ring)" stroke-width="1.5">
              <circle class="md-ring-wave" cx="210" cy="96" r="34" />
              <circle class="md-ring-wave" cx="210" cy="96" r="62" />
              <circle class="md-ring-wave" cx="210" cy="96" r="92" />
            </g>

            // 塔身：两根斜梁 + 横撑 + 顶部航标灯
            <g class="md-tower-glow" fill="none" stroke="url(#md-art-mast)" stroke-width="2">
              <path d="M196 300 L210 100 L224 300" stroke-linejoin="round" />
              <path d="M199 258h22M202 216h16M205 174h10M207.5 136h5" stroke-width="1.4" />
              <path d="M196 300 L150 316M224 300 L270 316" stroke-width="1.2" opacity="0.5" />
            </g>
            <circle cx="210" cy="96" r="4.5" fill="var(--md-orange)" />
            <circle cx="210" cy="96" r="9" fill="none" stroke="var(--md-orange)" stroke-opacity="0.35" />

            // ---- 卫星通信：轨道 + 卫星 + 下行波束 ----
            <g class="md-sat" fill="none" stroke="var(--md-blue)" stroke-opacity="0.45" stroke-width="1">
              <path d="M58 172 Q150 78 248 114" stroke-dasharray="3 7" />
            </g>
            // 注意：CSS 的 transform 会覆盖 SVG 的 transform 属性，所以位移放在外层，
            // 动画放在内层（否则卫星会被动画拽到原点）。
            <g transform="translate(132 116)">
              <g class="md-sat">
              // 星体 + 两翼太阳能板
              <rect x="-7" y="-6" width="14" height="12" rx="2" fill="var(--md-surface-3)" stroke="var(--md-blue)" stroke-width="1.2" />
              <rect x="-26" y="-4" width="16" height="8" rx="1" fill="var(--md-blue)" fill-opacity="0.35" stroke="var(--md-blue)" stroke-width="0.8" />
              <rect x="10" y="-4" width="16" height="8" rx="1" fill="var(--md-blue)" fill-opacity="0.35" stroke="var(--md-blue)" stroke-width="0.8" />
              <line x1="0" y1="-6" x2="0" y2="-16" stroke="var(--md-blue)" stroke-width="1.2" />
              // 下行波束（指向地面）
              <path d="M-2 8 L-14 40 M2 8 L14 40" stroke="var(--md-orange)" stroke-opacity="0.35" stroke-dasharray="2 5" stroke-width="1" />
              </g>
            </g>

            // ---- EME（月面反射）：地面站 → 月球 → 回波 ----
            <g fill="none" stroke="var(--md-violet)" stroke-width="1.1">
              <path class="md-eme" d="M214 292 Q286 120 336 96" stroke-opacity="0.55" />
              <path class="md-eme md-eme-back" d="M330 110 Q262 168 196 296" stroke-opacity="0.3" />
            </g>

            // 月球本体（EME 的反射体）：比背景辉光更实的圆盘 + 两个环形山
            <g class="md-moon-disc">
              <circle cx="336" cy="96" r="17" fill="var(--md-surface-3)" fill-opacity="0.92" />
              <circle cx="331" cy="91" r="4" fill="var(--md-fg)" fill-opacity="0.1" />
              <circle cx="341" cy="101" r="2.6" fill="var(--md-fg)" fill-opacity="0.08" />
            </g>

            // 远山剪影：两层，越远越淡。
            <path d="M0 300 L60 268 L108 286 L150 262 L210 292 L268 258 L322 288 L368 270 L420 300 Z" fill="var(--md-blue)" opacity="0.10" />
            <path d="M0 300 L52 282 L96 296 L150 276 L210 298 L270 280 L330 296 L378 284 L420 300 Z" fill="var(--md-violet)" opacity="0.12" />

            // 地平线 + 透视网格
            <path d="M8 300h404" stroke="url(#md-art-horizon)" stroke-width="1.2" />
            <g stroke="var(--md-blue)" stroke-opacity="0.22" stroke-width="1">
              <path d="M120 300 L60 360M170 300 L130 360M210 300 L210 360M250 300 L290 360M300 300 L360 360" />
              <path d="M40 318h345M56 336h312" stroke-opacity="0.12" />
            </g>
          </svg>

          <div class="md-hero-stat">
            // VFO 刻度盘：外圈 60 格刻度 + 进度弧 + 指针，替代通用的百分比圆环。
            <div class="md-dial md-dial-sm" aria-hidden="true">
              <svg viewBox="0 0 200 200">
                <defs>
                  <linearGradient id="md-dial-g" x1="0" y1="0" x2="1" y2="1">
                    <stop offset="0%" stop-color="var(--md-orange)" />
                    <stop offset="100%" stop-color="var(--md-violet)" />
                  </linearGradient>
                </defs>
                {(0..60)
                  .map(|i| {
                    let major = i % 5 == 0;
                    view! {
                      <line
                        class="md-dial-tick"
                        data-major=major.to_string()
                        x1="100"
                        y1="10"
                        x2="100"
                        y2=if major { "22" } else { "16" }
                        transform=format!("rotate({} 100 100)", i * 6)
                      />
                    }
                  })
                  .collect_view()}
                <circle cx="100" cy="100" r="72" fill="none" stroke="var(--md-surface-3)" stroke-width="8" />
                <circle
                  class="md-dial-arc"
                  cx="100"
                  cy="100"
                  r="72"
                  fill="none"
                  stroke="url(#md-dial-g)"
                  stroke-width="8"
                  stroke-linecap="round"
                  stroke-dasharray="452"
                  stroke-dashoffset=format!("{}", 452 - 452 * percent / 100)
                  transform="rotate(-90 100 100)"
                />
                <g
                  class="md-dial-needle"
                  style=format!("--dial-deg: {}deg", -90 + 36 * i32::try_from(percent).unwrap_or(0) / 10)
                >
                  <line x1="100" y1="100" x2="100" y2="46" stroke="var(--md-orange)" stroke-width="2.5" stroke-linecap="round" />
                </g>
              </svg>
              <div class="md-dial-center">
                <span class="md-dial-value">{format!("{percent}%")}</span>
              </div>
            </div>
            <span class="md-hero-stat-divider" aria-hidden="true"></span>

            <div class="flex flex-col gap-0.5">
              <span class="md-stat-label">{move || rank_label(level_no)}</span>
              <span class="md-mono md-sm">
                {move || tf("shell.hud-level", &[&level_no.to_string()])}
              </span>
            </div>
            <span class="md-hero-stat-divider" aria-hidden="true"></span>
            <div class="flex flex-col gap-0.5">
              <span class="md-stat-label">{move || t("rpg.exp-label")}</span>
              <span class="md-mono md-sm">
                {move || {
                  if is_max {
                    t("shell.hud-max-level")
                  } else {
                    format!("{into_level} / {needed}")
                  }
                }}
              </span>
            </div>
            <span class="md-hero-stat-divider" aria-hidden="true"></span>
            <div class="flex flex-col gap-0.5">
              <span class="md-stat-label">{move || t("home.streak-days-label")}</span>
              <span class="md-mono md-sm">
                {move || tf("shell.hud-streak", &[&streak.to_string()])}
              </span>
            </div>
          </div>

          <span class="sr-only">{format!("{percent}%")}</span>
        </div>
      </div>

      // 实时频谱：频点柱 + 峰值保持 + 扫描线 + 余辉，模型在 `ham_web_core::spectrum_scope`。
      // CSS 竖条只能做周期动画，一眼看出在循环；这条带子是「接收机正在扫描」的读数。
      <div class="md-shell" node_ref=eq_ref>
        <crate::kit_modern::ModernSpectrum class="md-spectrum" />
      </div>

      // 电波母题：一条平滑曲线，两端淡出、虚线缓慢流动。纯装饰，不承载信息。
      <svg class="md-wave" viewBox="0 0 1200 96" preserveAspectRatio="none" aria-hidden="true">
        <defs>
          <linearGradient id="md-wave-grad" x1="0" y1="0" x2="1" y2="0">
            <stop offset="0%" stop-color="var(--md-orange)" stop-opacity="0" />
            <stop offset="28%" stop-color="var(--md-orange)" stop-opacity="0.75" />
            <stop offset="62%" stop-color="var(--md-blue)" stop-opacity="0.6" />
            <stop offset="100%" stop-color="var(--md-blue)" stop-opacity="0" />
          </linearGradient>
        </defs>
        <path
          d="M0 72 C 90 72 130 18 220 18 S 360 72 450 72 S 590 26 680 26 S 820 76 910 76 S 1050 34 1200 34"
          fill="none"
          stroke="url(#md-wave-grad)"
          stroke-width="2"
          stroke-linecap="round"
        />
        <path
          d="M0 86 C 110 86 150 44 250 44 S 390 88 490 88 S 630 50 730 50 S 870 92 970 92 S 1110 58 1200 58"
          fill="none"
          stroke="url(#md-wave-grad)"
          stroke-width="1"
          stroke-linecap="round"
          opacity="0.45"
        />
      </svg>
    </section>
  }
}
