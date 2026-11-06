//! 窄屏「分类 / 目录」菜单：右下角悬浮按钮 + 右侧抽屉。
//!
//! 带侧栏 / 目录 / 分类行的页面（题库分类、术语分类、工具目录）在 `< 768` 共用**一种**
//! 交互 —— 动作与像素风同一套（悬浮按钮 → 右侧抽屉），长的换成现代风的语言：
//! 56px 圆形行动色按钮、`md-sheet` 抽屉、44px 行式列表（选中态用行动色，不新造记号）。
//! `>= 768` 由桌面侧栏接管，按钮由 CSS 整颗隐藏，页面不需要判断断点。
//!
//! 按钮可以按住拖动改位置（与像素风同一种操作）：位置收拢在视口内、持久化到同一个
//! `localStorage` key，拖过一次之后才从「CSS 默认位置」切换成像素定位 —— 默认位置带
//! `safe-area-inset`，重复算一遍只会两边对不上。拖动的数学在 `ham_web_core::fab`，
//! 收拢与持久化在 `crate::fab`。
//!
//! 按钮与抽屉都走 `<Portal>`：`.md-page` 的 `container-type` 是 `fixed` 后代的包含块，
//! 留在页面里会被裁在版心内（见 `docs/ui/modern.md` §3「Portal 出来的表面」）。
//! 因此它们也不在 `.md-root` 子树里 —— 焦点环与减动效名单都要单独登记。

use ham_web_core::fab::FabDrag;
use leptos::ev;
use leptos::portal::Portal;
use leptos::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;

use super::backdrop::Backdrop;
use super::{MIcon, ModernIcon};
use crate::focus::use_escape_close;
use crate::i18n::t;
use crate::presence::use_presence;

/// 按钮边长（与 `.md-drawer-fab` 的 56px 一致）。
const FAB_SIZE: f64 = 56.0;
/// 累计位移超过该值视为拖动（否则是轻点，由随后的 `click` 打开抽屉）。
const DRAG_THRESHOLD: f64 = 6.0;

/// 悬浮按钮 + 右侧抽屉。
///
/// - `open`：抽屉开关，由调用方持有 —— 选中一项后要能主动 `open.set(false)` 收起；
/// - `title`：抽屉标题（也是对话框的无障碍名）；
/// - `button_label`：悬浮按钮的无障碍名（图标按钮必填，见 `docs/ui/modern.md` §6）；
/// - `children`：抽屉内容（与桌面侧栏同一份数据的行式列表）。
#[component]
pub fn ModernMenuDrawer(
  open: RwSignal<bool>,
  #[prop(into)] title: Signal<String>,
  #[prop(into)] button_label: Signal<String>,
  children: ChildrenFn,
) -> impl IntoView {
  let presence = use_presence(open);
  let backdrop = Backdrop::new(open);
  use_escape_close(open);
  crate::overlay::use_scroll_lock(open);
  crate::focus::use_modal_focus(open);

  // `None` = 还没拖过：位置交给 CSS（右下角、含 `safe-area-inset`）。
  let pos = RwSignal::new(crate::fab::load_pos(FAB_SIZE));
  // 进行中的手势（None = 没有按下）。
  let drag = StoredValue::new(None::<FabDrag>);
  // 刚拖完：吞掉随后补派的指针 click（不开抽屉）。**不用定时器复位** —— 浏览器补派
  // click 的时机在不同设备上从同一任务到几百毫秒都有，定时器赌不准；改成「下一次按下
  // 才复位」，晚到多久都吞得掉。
  let swallow_click = StoredValue::new(false);

  let on_down = move |ev: web_sys::PointerEvent| {
    swallow_click.set_value(false);
    // 防止长按选中文字 / 弹出系统菜单（拖动本身由 `touch-action: none` 拦住滚动）。
    ev.prevent_default();
    let Some(el) = ev
      .current_target()
      .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
    else {
      return;
    };
    // 起点取按钮当前的视觉位置：默认位置的 CSS 带着 `safe-area-inset`，这里读回来
    // 就不用把那份 `calc()` 在 Rust 里再算一遍。
    let rect = el.get_bounding_client_rect();
    let pointer = (f64::from(ev.client_x()), f64::from(ev.client_y()));
    drag.set_value(Some(FabDrag::new(pointer, (rect.left(), rect.top()))));
    // 捕获指针：手指移出按钮后 move / up 仍派发到这里。
    let _ = el.set_pointer_capture(ev.pointer_id());
  };
  let on_move = move |ev: web_sys::PointerEvent| {
    let Some(mut d) = drag.get_value() else {
      return;
    };
    let (vw, vh) = crate::fab::viewport();
    let next = d.drag_to(
      (f64::from(ev.client_x()), f64::from(ev.client_y())),
      ((vw - FAB_SIZE).max(0.0), (vh - FAB_SIZE).max(0.0)),
    );
    drag.set_value(Some(d));
    pos.set(Some(next));
  };
  let on_up = move |ev: web_sys::PointerEvent| {
    let Some(d) = drag.get_value() else {
      return;
    };
    drag.set_value(None);
    if let Some(el) = ev
      .current_target()
      .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
    {
      let _ = el.release_pointer_capture(ev.pointer_id());
    }
    // 这里**只收尾拖动，不开抽屉**。开抽屉统一放在 `click` 里：`click` 派发在按钮上，
    // 之后抽屉才挂载，不会再有补派的 click 落进抽屉。若在 `pointerup` 里就开，浏览器随后
    // 补派的 click 落点按「派发那一刻指针下方的元素」重算，会打在抽屉的行上 ——
    // 工具页上就是「点悬浮按钮，直接跳到某个计算器并收起抽屉」。
    if d.is_drag(DRAG_THRESHOLD) {
      swallow_click.set_value(true);
      if let Some(p) = pos.get_untracked() {
        crate::fab::save_pos(p);
      }
    }
  };
  let on_cancel = move |ev: web_sys::PointerEvent| {
    if drag.get_value().is_some_and(|d| d.is_drag(DRAG_THRESHOLD)) {
      swallow_click.set_value(true);
    }
    drag.set_value(None);
    if let Some(el) = ev
      .current_target()
      .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
    {
      let _ = el.release_pointer_capture(ev.pointer_id());
    }
  };
  // 视口变化（旋转 / 分屏）后把位置收拢回屏内。
  let resize_handle = window_event_listener(ev::resize, move |_| {
    if let Some(p) = pos.get_untracked() {
      pos.set(Some(crate::fab::clamp_to_viewport(p, FAB_SIZE)));
    }
  });
  let resize_handle = SendWrapper::new(Some(resize_handle));
  on_cleanup(move || {
    if let Some(h) = resize_handle.take() {
      h.remove();
    }
  });

  // 唯一打开抽屉的入口：鼠标 / 轻点 / 键盘 Enter·Space / 读屏激活都走 `click`。
  // 例外只有一个：刚拖完补派的指针 click（`detail > 0`）要吞掉；键盘激活
  // （`detail == 0`）不受影响，拖完紧接着按 Enter 仍然能开。
  let on_click = move |ev: web_sys::MouseEvent| {
    if ev.detail() > 0 && swallow_click.get_value() {
      return;
    }
    open.set(true);
  };

  // 拖过之后才写内联定位：同时把 CSS 的右下角定位清掉，两套坐标不能并存。
  let fab_style = move || {
    pos
      .get()
      .map(|(x, y)| format!("inset-block-end:auto; inset-inline-end:auto; left:{x}px; top:{y}px;"))
  };

  move || {
    let children = children.clone();
    view! {
      <Portal>
        <button
          type="button"
          class="md-drawer-fab"
          style=fab_style
          aria-label=move || button_label.get()
          aria-haspopup="dialog"
          aria-expanded=move || open.get().to_string()
          on:pointerdown=on_down
          on:pointermove=on_move
          on:pointerup=on_up
          on:pointercancel=on_cancel
          on:click=on_click
        >
          <ModernIcon kind=MIcon::Sliders />
        </button>
      </Portal>

      {move || {
        // `children` 是 `Arc<dyn Fn>`（非 Copy）：每次求值都克隆一份交给下面那个
        // `FnOnce`（`then` 的入参），外层闭包才能保持可重复调用。
        let children = children.clone();
        presence.visible.get().then(move || {
          view! {
            <Portal>
              <div
                class="md-sheet"
                data-state=presence.state()
                inert=presence.inert()
                aria-hidden=presence.aria_hidden()
                // 只在「点遮罩」时关闭，不在面板上 `stop_propagation`：后者会把路由的全局
                // click 拦截一起挡掉，面板里的链接就退化成整页跳转（见 `backdrop.rs`）。
                on:pointerdown=backdrop.on_pointerdown()
                on:click=backdrop.on_click()
              >
                <div
                  class="md-sheet-panel"
                  data-modal-root=""
                  tabindex="-1"
                  role="dialog"
                  aria-modal="true"
                  aria-label=move || title.get()
                >
                  <div class="md-sheet-head">
                    <h2 class="md-h2">{move || title.get()}</h2>
                    <button
                      type="button"
                      class="md-icon-btn"
                      aria-label=move || t("shell.close-menu")
                      on:click=move |_| open.set(false)
                    >
                      <ModernIcon kind=MIcon::Close />
                    </button>
                  </div>
                  <div class="md-sheet-body">{children()}</div>
                </div>
              </div>
            </Portal>
          }
        })
      }}
    }
  }
}
