//! 「先把这一帧画出来，再做重活」，以及「重活拆成几批，每批之间让浏览器画一帧」。
//!
//! 两件事是同一个根因：Leptos 的渲染是同步的，一次做完一大块，浏览器这段时间一帧都画不出来。
//!
//! - 点击反馈：点底部 dock、目录里的链接时，路由器会在 `click` 之后的微任务里把整张新页面
//!   同步渲染完（手机上要几百毫秒），目标项的高亮、目录面板的收起都要等新页面一起出现 ——
//!   用户看到的是「点了没反应，过一会才动」。先改界面状态、等这一帧画出来，再导航或挂面板
//!   （[`after_next_paint`]）。
//! - 长页面：工具页 60 个计算器、术语表 60 张卡片的统计，一口气做完就是几百毫秒的长任务，
//!   页面要等它全部做完才出第一帧。首屏先出，其余每画完一帧推进一批（[`each_frame`]），
//!   单批控制在一帧的预算内。见 `docs/ui/motion.md`「长列表优先分页」。

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use leptos::prelude::*;

/// 等浏览器把当前状态画出来之后再执行 `f`。
///
/// `requestAnimationFrame` 的回调跑在这一帧绘制**之前**，在它里面排一个 0ms 定时器，
/// 定时器才会落在绘制**之后**。只用 rAF 的话，重活会赶在这一帧绘制前跑完，等于没让。
///
/// 页面已经隐藏时 rAF 整个暂停，排进去的活要等切回来才执行：点完 dock / 目录链接立刻切走
/// （手机上很常见）会让**导航**一直不执行，回来才跳。这种时候直接同步执行——反正也看不到
/// 「先画一帧」的好处，而目标项的高亮已经在这之前改好了。
pub fn after_next_paint(f: impl FnOnce() + 'static) {
  if crate::util::document().hidden() {
    f();
    return;
  }
  request_animation_frame(move || set_timeout(f, Duration::ZERO));
}

/// 只能执行一次的回调，放在共享槽里：谁先取走谁执行（见 [`when_idle`]）。
type OnceSlot = Rc<RefCell<Option<Box<dyn FnOnce()>>>>;

/// 等浏览器空闲时再执行 `f`（`requestIdleCallback`，最多等 `timeout`）。
///
/// 给「现在不急、但用户很可能马上要用」的预热活用（见 `bank_updates::prewarm`）：
/// 空闲回调只在没有输入、没有渲染要做的间隙触发，不会和点击、换页抢主线程。
/// 不支持的浏览器（Safari 直到 18 才有），或调用本身失败时，退回一个 200ms 的定时器 ——
/// 回调不能就此消失：[`idle`] 在等它，调用方 `.await` 会永远挂着。
pub fn when_idle(f: impl FnOnce() + 'static, timeout: Duration) {
  use wasm_bindgen::JsCast;
  use wasm_bindgen::closure::Closure;

  // 两条路径（空闲回调、定时器）共用一个槽：谁先到谁取走，`f` 只会执行一次。
  let slot: OnceSlot = Rc::new(RefCell::new(Some(Box::new(f))));
  let run = {
    let slot = slot.clone();
    move || {
      let f = slot.borrow_mut().take();
      if let Some(f) = f {
        f();
      }
    }
  };

  let window = crate::util::window();
  let ric = js_sys::Reflect::get(&window, &"requestIdleCallback".into())
    .ok()
    .and_then(|v| v.dyn_into::<js_sys::Function>().ok());
  let queued = ric.is_some_and(|ric| {
    let cb = Closure::once_into_js(run.clone());
    let opts = js_sys::Object::new();
    let ms = u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX);
    let _ = js_sys::Reflect::set(&opts, &"timeout".into(), &f64::from(ms).into());
    ric.call2(&window, &cb, &opts).is_ok()
  });
  if !queued {
    set_timeout(run, Duration::from_millis(200));
  }
}

/// [`when_idle`] 的异步版：`.await` 到浏览器空闲为止。
pub async fn idle(timeout: Duration) {
  let promise = js_sys::Promise::new(&mut |resolve, _reject| {
    when_idle(
      move || {
        let _ = resolve.call0(&wasm_bindgen::JsValue::NULL);
      },
      timeout,
    );
  });
  let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

/// 每画完一帧调用一次 `step`，直到它返回 `false`。
///
/// 第一次调用也在当前这一帧画完之后：调用方的首屏先出，分批的活排在后面。
/// `step` 里写信号请用 `try_*`：页面在分批途中被卸载时信号已释放，`try_*` 返回 `None`，
/// 顺势返回 `false` 结束循环。
pub fn each_frame(step: impl FnMut() -> bool + 'static) {
  fn tick(step: Rc<RefCell<dyn FnMut() -> bool>>) {
    after_next_paint(move || {
      let more = (step.borrow_mut())();
      if more {
        tick(step);
      }
    });
  }
  tick(Rc::new(RefCell::new(step)));
}

/// 这次点击是不是「普通左键点链接」：只有这种情况才接管导航。
///
/// 口径与 `leptos_router` 自己拦截链接时一致：已被拦下的、中键、带修饰键
/// （新标签 / 新窗口 / 下载）的点击原样交给浏览器与路由器。
#[must_use]
pub fn is_plain_click(ev: &web_sys::MouseEvent) -> bool {
  !ev.default_prevented()
    && ev.button() == 0
    && !ev.meta_key()
    && !ev.alt_key()
    && !ev.ctrl_key()
    && !ev.shift_key()
}
