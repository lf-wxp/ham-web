//! 移动端「悬浮导航抽屉」：小屏下右下角一个悬浮按钮，点开右侧抽屉，替代被
//! `hidden` 隐藏的侧栏（分类 / 工具目录）。桌面端侧栏照旧显示，悬浮按钮由
//! `class` 里的断点类（`md:hidden` / `lg:hidden`）在小屏隐藏。
//!
//! 悬浮按钮可以按住拖动改位置：位置收拢在视口内、持久化到 `localStorage`，
//! 三个页面共享同一个位置。轻点（位移小于阈值）打开抽屉，拖动不打开 ——
//! 判定与收拢在 `ham_web_core::fab`，这里只做事件监听与持久化。

use std::time::Duration;

use ham_web_core::fab::FabDrag;
use leptos::ev;
use leptos::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;

use crate::cn::cn;
use crate::icons::{Icon, IconKind};
use crate::util::{storage, window};

use super::{Button, Sheet, Size, Variant};

/// 按钮边长（对应 [`Size::Icon`] 的 40px）。
const FAB_SIZE: f64 = 40.0;
/// 累计位移超过该值视为拖动（否则是轻点，打开抽屉）。
const DRAG_THRESHOLD: f64 = 6.0;
/// 位置持久化的 key（浏览 / 术语表 / 工具目录共享）。
const POS_KEY: &str = "fab-position";
/// 默认位置：距右 16px（原 `right-4`）。
const DEFAULT_GAP: f64 = 16.0;
/// 默认位置：距底 96px（原 `bottom-24`，避开底部 Tab 与其 safe-area）。
const DEFAULT_BOTTOM: f64 = 96.0;

/// 视口尺寸（读不到时为 0，收拢逻辑会退化成「钉在原点」）。
fn viewport() -> (f64, f64) {
  let w = window();
  let width = w.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
  let height = w
    .inner_height()
    .ok()
    .and_then(|v| v.as_f64())
    .unwrap_or(0.0);
  (width, height)
}

/// 默认位置（右下角）。
fn default_pos() -> (f64, f64) {
  let (vw, vh) = viewport();
  (
    (vw - FAB_SIZE - DEFAULT_GAP).max(0.0),
    (vh - FAB_SIZE - DEFAULT_BOTTOM).max(0.0),
  )
}

/// 把位置收拢进当前视口（旋转 / 换设备后旧位置可能出界）。
fn clamp_to_viewport(pos: (f64, f64)) -> (f64, f64) {
  let (vw, vh) = viewport();
  let max_x = (vw - FAB_SIZE).max(0.0);
  let max_y = (vh - FAB_SIZE).max(0.0);
  (pos.0.clamp(0.0, max_x), pos.1.clamp(0.0, max_y))
}

fn load_pos() -> (f64, f64) {
  let Some(raw) = storage::get(POS_KEY) else {
    return default_pos();
  };
  let Some((x, y)) = raw.split_once(',') else {
    return default_pos();
  };
  match (x.trim().parse::<f64>(), y.trim().parse::<f64>()) {
    (Ok(x), Ok(y)) => clamp_to_viewport((x, y)),
    _ => default_pos(),
  }
}

/// 悬浮按钮 + 右侧抽屉的组合。
///
/// - `open`：抽屉开关，由调用方持有 —— 点分类项后要能主动 `open.set(false)` 收起；
/// - `title`：抽屉标题（响应式文案）；
/// - `button_label`：悬浮按钮的无障碍名（图标按钮必填，见 `docs/ui-components.md` §6）；
/// - `class`：悬浮按钮容器的断点类（浏览 / 术语表侧栏是 `md:block` 用 `md:hidden`，
///   工具目录侧栏是 `lg:block` 用 `lg:hidden`）；
/// - `children`：抽屉内容（与桌面侧栏共用同一份列表）。
#[component]
pub fn MobileNavDrawer(
  open: RwSignal<bool>,
  #[prop(into)] title: Signal<String>,
  #[prop(into)] button_label: Signal<String>,
  icon: IconKind,
  #[prop(optional, into)] class: String,
  children: ChildrenFn,
) -> impl IntoView {
  let pos = RwSignal::new(load_pos());
  // 进行中的手势（None = 没有按下）。
  let drag = StoredValue::new(None::<FabDrag>);
  // 拖动后抑制随后可能派发的合成 click（不同浏览器对 pointerdown
  // `prevent_default` 与 click 的关系不一致，这里双保险）。
  let suppress_click = StoredValue::new(false);
  let container_class = cn(&["fixed z-40 touch-none", &class]);

  let on_down = move |ev: web_sys::PointerEvent| {
    // 阻止合成 click / mouse 事件：打开改由 pointerup 判定，拖动后不会误开抽屉；
    // 键盘 Enter / Space 仍走 Button 的 `on_click`。
    ev.prevent_default();
    let origin = pos.get_untracked();
    let pointer = (f64::from(ev.client_x()), f64::from(ev.client_y()));
    drag.set_value(Some(FabDrag::new(pointer, origin)));
    // 捕获指针：手指移出按钮后 move / up 仍派发到这里。
    if let Some(el) = ev
      .current_target()
      .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
    {
      let _ = el.set_pointer_capture(ev.pointer_id());
    }
  };
  let on_move = move |ev: web_sys::PointerEvent| {
    let Some(mut d) = drag.get_value() else {
      return;
    };
    let (vw, vh) = viewport();
    let next = d.drag_to(
      (f64::from(ev.client_x()), f64::from(ev.client_y())),
      ((vw - FAB_SIZE).max(0.0), (vh - FAB_SIZE).max(0.0)),
    );
    drag.set_value(Some(d));
    pos.set(next);
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
    if d.is_drag(DRAG_THRESHOLD) {
      // 已拖动：只落点持久化，不打开抽屉。
      suppress_click.set_value(true);
      // 兜底：若浏览器根本没派发 click，稍后归位，避免误吞下一次轻点 / 键盘激活。
      set_timeout(
        move || suppress_click.set_value(false),
        Duration::from_millis(0),
      );
      let p = pos.get_untracked();
      storage::set(POS_KEY, &format!("{},{}", p.0, p.1));
    } else {
      // 轻点：打开抽屉。
      open.set(true);
    }
  };
  let on_cancel = move |ev: web_sys::PointerEvent| {
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
    pos.update(|p| *p = clamp_to_viewport(*p));
  });
  let resize_handle = SendWrapper::new(Some(resize_handle));
  on_cleanup(move || {
    if let Some(h) = resize_handle.take() {
      h.remove();
    }
  });

  // 键盘激活（Enter / Space）或未拖动时的 click 打开抽屉；拖动后的合成 click 被吞掉。
  let on_click = Callback::new(move |_| {
    if suppress_click.get_value() {
      suppress_click.set_value(false);
      return;
    }
    open.set(true);
  });

  // 与 `Sheet` / `Dialog` 同款：顶层显式 `move` 闭包，非 `Copy` 的 `children` /
  // `container_class` 在闭包内 clone 后再用，避免整个闭包退化成 `FnOnce`。
  move || {
    let container_class = container_class.clone();
    let children = children.clone();
    view! {
      <div
        class=container_class
        style=move || format!("left: {}px; top: {}px", pos.get().0, pos.get().1)
        on:pointerdown=on_down
        on:pointermove=on_move
        on:pointerup=on_up
        on:pointercancel=on_cancel
      >
        <Button
          variant=Variant::Default
          size=Size::Icon
          class="shadow-lg"
          aria_label=button_label
          on_click=on_click
        >
          <Icon kind=icon class="size-6" />
        </Button>
      </div>
      <Sheet open=open>
        <div data-slot="sheet-header" class="flex flex-col gap-1.5 p-4">
          <h2 data-slot="sheet-title" class="text-foreground">{move || title.get()}</h2>
        </div>
        // `pt-1`：打开时焦点自动落在第一个列表项（`focus_first`），它的 focus-visible
        // 聚焦环画在元素外框之外 —— 顶部不留 4px 余量会被 overflow 容器裁掉
        // （`ui/README.md` §3「状态反馈」的坑）。
        <div class="overflow-y-auto overscroll-contain px-4 pb-4 pt-1">{children()}</div>
      </Sheet>
    }
  }
}
