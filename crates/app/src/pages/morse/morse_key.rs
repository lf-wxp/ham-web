use leptos::prelude::*;

use crate::ui::TextValue;

/// morse 练习页的大圆键外壳：试听键（点击）与长按发报键（按下 / 松开）共用。
///
/// 为什么抽出来：三处各手写了一份 `shadow-lg shadow-primary/25` 软阴影 + `hover:scale-105` /
/// `active:scale-95` 缩放 + `focus-visible:ring-2` 实线焦点环，全是像素规范明令不要的东西
/// （缩放的中间帧会让点阵图标变糊；焦点环与全站的 2px 虚线不是一套语言）。
/// 这里改用 `pxl-btn pxl-btn-primary`：硬阴影、悬停上抬、按下下沉都是现成的。
///
/// 「正在播放 / 正在按住」由 `active` 驱动，落在 `data-state="active"` 上，CSS 里
/// `.pxl-btn[data-state="active"]` 画成按下态（见 `controls.css`）—— 状态只信属性，不在 Rust
/// 里拼类名。扩散圈原来用 `animate-ping`（放大 + 淡出），同样是缩放动画；换成内框逐帧闪烁。
///
/// 仍是原生 `<button>`：长按发报要 `pointerdown` / `pointerup` / `pointerleave` 一组事件，
/// `Button` 没有这条通道（见 `docs/ui-components.md` §10.3）。
#[component]
pub(super) fn MorseKey(
  /// 尺寸类（`size-16` / `size-20` / `h-28 w-28`），都取 4 的倍数。
  class: &'static str,
  /// 正在播放 / 正在按住。
  #[prop(into)]
  active: Signal<bool>,
  #[prop(optional, into)] aria_label: Option<TextValue>,
  /// 点击（试听键用）。
  #[prop(optional, into)]
  on_click: Option<Callback<()>>,
  /// 按下（发报键用）；传了它，按钮同时禁掉浏览器的触摸手势，避免长按被当成滚动。
  #[prop(optional, into)]
  on_down: Option<Callback<()>>,
  /// 松开 / 离开 / 取消（发报键用）。
  #[prop(optional, into)]
  on_up: Option<Callback<()>>,
  children: Children,
) -> impl IntoView {
  let run = move |cb: Option<Callback<()>>| {
    if let Some(cb) = cb {
      cb.run(());
    }
  };
  view! {
    <button
      type="button"
      data-slot="morse-key"
      data-state=move || if active.get() { "active" } else { "idle" }
      aria-label=move || aria_label.as_ref().map(TextValue::get)
      // 只有长按发报键需要禁掉浏览器手势：`None` 时不输出 `style`，别留一个空属性。
      style=move || on_down.is_some().then_some("touch-action: none;")
      class=format!("pxl-btn pxl-btn-primary relative select-none {class}")
      on:click=move |_| run(on_click)
      on:pointerdown=move |_| run(on_down)
      on:pointerup=move |_| run(on_up)
      on:pointerleave=move |_| run(on_up)
      on:pointercancel=move |_| run(on_up)
    >
      {move || {
        active
          .get()
          .then(|| {
            view! { <span class="pxl-blink pointer-events-none absolute inset-1 border-2 border-primary-foreground"></span> }
          })
      }}
      {children()}
    </button>
  }
}
