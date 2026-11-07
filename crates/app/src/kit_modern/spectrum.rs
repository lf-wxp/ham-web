//! 首页「实时频谱」：频点柱状 + 信号轮廓 + 峰值保持 + 扫描线 + 余辉（Canvas 2D）。
//!
//! 为什么不用 CSS 竖条：CSS 只能做周期动画，一眼看出在循环。这里把「时间 → 每个频点的
//! 幅度」交给 `ham_web_core::spectrum_scope`（纯逻辑、有单测），本文件只管三件事：
//! 画布的尺寸与配色、一个自限速的渲染循环、卸载时的全部清理。
//!
//! 性能口径：
//! - 30fps 上限（扫频这个速度用不上 60fps），rAF 驱动，切到后台自动停；
//! - 滚出视口后由 `IntersectionObserver` 挂起，首屏滚走就不再绘制；
//! - 主题色每 90 帧（≈3 s）读一次 CSS 变量，逐帧路径上不碰 `getComputedStyle`。

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use ham_web_core::spectrum_scope::ScopeModel;
use leptos::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{
  CanvasRenderingContext2d, HtmlCanvasElement, IntersectionObserver, IntersectionObserverInit,
  ResizeObserver,
};

use super::spectrum_paint::{self, Palette};

/// 目标柱宽（CSS px，含缝）：8px 还读得出「柱」，再窄就糊成一片色块。
const BIN_PITCH: f64 = 8.0;
/// 柱数上下限：320px 的窄屏也要有足够多的柱子撑出「频段」的形状。
///
/// 与 `ham_web_core::spectrum_scope::MIN_BINS`（24）不是同一个数：那边是模型的**物理**
/// 下限，这里是「柱子还读得出是柱」的**视觉**下限，故意抬高一档。
const MIN_VIEW_BINS: usize = 28;
const MAX_BINS: usize = 132;
/// 帧间隔（毫秒）：30fps。
const FRAME_MS: f64 = 33.0;
/// 多少帧读一次主题色（≈3 s）。切明暗后最多晚一帧发现，肉眼看不出来。
const PALETTE_REFRESH: u32 = 90;

/// 一块画布的全部状态。
struct Scene {
  canvas: HtmlCanvasElement,
  ctx: CanvasRenderingContext2d,
  model: ScopeModel,
  palette: Palette,
  width: f64,
  height: f64,
  dpr: f64,
  frames: u32,
}

impl Scene {
  fn new(canvas: HtmlCanvasElement, ctx: CanvasRenderingContext2d) -> Self {
    let palette = Palette::read(&canvas);
    Self {
      canvas,
      ctx,
      // 先按最小值建，`sync_size` 会立刻按真实宽度重排。
      model: ScopeModel::new(MIN_VIEW_BINS),
      palette,
      width: 0.0,
      height: 0.0,
      dpr: 0.0,
      frames: 0,
    }
  }

  /// 按 CSS 尺寸重建后备缓冲（dpr 变化、断点换高度都在这里）。尺寸没变返回 `false`。
  fn sync_size(&mut self) -> bool {
    let width = f64::from(self.canvas.client_width()).max(1.0);
    let height = f64::from(self.canvas.client_height()).max(1.0);
    let dpr = web_sys::window()
      .map_or(1.0, |w| w.device_pixel_ratio())
      .max(1.0);
    if width == self.width && height == self.height && dpr == self.dpr {
      return false;
    }
    self.width = width;
    self.height = height;
    self.dpr = dpr;
    // 顺序不能反：设后备缓冲尺寸会重置绘制变换，缩放要放在它之后。
    self.canvas.set_width((width * dpr).round() as u32);
    self.canvas.set_height((height * dpr).round() as u32);
    let _ = self.ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0);
    // 柱数跟着宽度走：窄屏少几根，宽屏多几根，柱宽始终落在 6–9 CSS px。
    self.model.resize(bins_for(width));
    true
  }

  /// 推进 `dt_ms` 毫秒并重画一帧。
  fn advance(&mut self, dt_ms: f64) {
    self.model.step((dt_ms / 1000.0) as f32);
    self.frames = self.frames.wrapping_add(1);
    if self.frames.is_multiple_of(PALETTE_REFRESH) {
      self.palette = Palette::read(&self.canvas);
    }
    self.paint();
  }

  fn paint(&self) {
    spectrum_paint::paint(
      &self.ctx,
      &self.model,
      &self.palette,
      self.width,
      self.height,
    );
  }
}

/// 宽度 → 柱数（换算与异常宽度的处理在 core 的 `bins_for_width`，有单测）。
fn bins_for(width: f64) -> usize {
  ham_web_core::spectrum_scope::bins_for_width(width, BIN_PITCH, MIN_VIEW_BINS, MAX_BINS)
}

/// 自调度用的一帧回调：闭包要先塞进槽里才能把自己挂到下一帧，所以套一层 `Rc<RefCell<…>>`。
type FrameTick = Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>;

/// 卸载时要撤掉的一批句柄。
///
/// `Closure` 与 observer 都不是 `Send`，而 `on_cleanup` 要求 `Send + 'static`：
/// 整包交给 `SendWrapper`（只在创建它的线程上访问，语义上安全）。
struct Watchers {
  resize: Option<ResizeObserver>,
  _resize_cb: Closure<dyn FnMut(js_sys::Array)>,
  intersect: Option<IntersectionObserver>,
  _intersect_cb: Closure<dyn FnMut(js_sys::Array)>,
  tick: FrameTick,
  handle: Rc<Cell<i32>>,
}

#[component]
pub fn ModernSpectrum(#[prop(optional, into)] class: String) -> impl IntoView {
  let canvas_ref = NodeRef::<leptos::html::Canvas>::new();
  canvas_ref.on_load(start);

  view! {
    <canvas node_ref=canvas_ref class=class aria-hidden="true"></canvas>
  }
}

/// 挂上渲染循环与两个观察器。
fn start(canvas: HtmlCanvasElement) {
  let Some(window) = web_sys::window() else {
    return;
  };
  let Some(ctx) = canvas
    .get_context("2d")
    .ok()
    .flatten()
    .and_then(|v| v.dyn_into::<CanvasRenderingContext2d>().ok())
  else {
    return;
  };

  let scene = Rc::new(RefCell::new(Scene::new(canvas.clone(), ctx)));
  scene.borrow_mut().sync_size();

  let reduced = window
    .match_media("(prefers-reduced-motion: reduce)")
    .ok()
    .flatten()
    .is_some_and(|m| m.matches());

  // 减少动态效果：定格一帧「有台在发报」的快照，而不是一排空柱子或一块空白。
  if reduced {
    let mut scene = scene.borrow_mut();
    for _ in 0..90 {
      scene.model.step(1.0 / 30.0);
    }
    scene.paint();
  }

  let visible = Rc::new(Cell::new(true));

  // 尺寸跟随：断点换高度（64 → 40）、窗口缩放、dpr 变化都走这里。
  // `ResizeObserver` 的首个回调不保证在首帧前到达，所以上面已经同步量过一次。
  let resize_cb = {
    let scene = Rc::clone(&scene);
    Closure::<dyn FnMut(js_sys::Array)>::new(move |_entries: js_sys::Array| {
      let mut scene = scene.borrow_mut();
      // 改后备缓冲尺寸会清空画布：立刻补一帧，否则会闪一下空白。
      if scene.sync_size() {
        scene.paint();
      }
    })
  };
  let resize_observer =
    ResizeObserver::new(resize_cb.as_ref().unchecked_ref::<js_sys::Function>()).ok();
  if let Some(observer) = resize_observer.as_ref() {
    observer.observe(&canvas);
  }

  let tick: FrameTick = Rc::new(RefCell::new(None));
  // 排队中的那一帧；`-1` 表示没有排队（还没开始，或滚出视口后已停下）。
  let handle = Rc::new(Cell::new(-1i32));

  // 滚出视口就停下：首屏一滚走，这条带子不该继续占主线程。此前只是「不画」，每帧仍然自调度一次；
  // 现在帧循环直接停掉，重新进入视口时由这里再挂上。
  let intersect_cb = {
    let visible = Rc::clone(&visible);
    let tick = Rc::clone(&tick);
    let handle = Rc::clone(&handle);
    Closure::<dyn FnMut(js_sys::Array)>::new(move |entries: js_sys::Array| {
      if let Ok(entry) = entries
        .get(0)
        .dyn_into::<web_sys::IntersectionObserverEntry>()
      {
        let now_visible = entry.is_intersecting();
        visible.set(now_visible);
        if now_visible
          && handle.get() < 0
          && let (Some(window), Some(cb)) = (web_sys::window(), tick.borrow().as_ref())
          && let Ok(id) = window.request_animation_frame(cb.as_ref().unchecked_ref())
        {
          handle.set(id);
        }
      }
    })
  };
  let init = IntersectionObserverInit::new();
  // 上下各多留 100px：快速滚回来时不会先看到「挂上还没画」的空帧。
  init.set_root_margin("100px 0px");
  let intersect_observer = IntersectionObserver::new_with_options(
    intersect_cb.as_ref().unchecked_ref::<js_sys::Function>(),
    &init,
  )
  .ok();
  if let Some(observer) = intersect_observer.as_ref() {
    observer.observe(&canvas);
  }

  let last = Rc::new(Cell::new(0.0f64));

  if !reduced {
    let frame = {
      let tick = Rc::clone(&tick);
      let handle = Rc::clone(&handle);
      let last = Rc::clone(&last);
      let visible = Rc::clone(&visible);
      let scene = Rc::clone(&scene);
      Closure::<dyn FnMut(f64)>::new(move |ts: f64| {
        if !visible.get() {
          // 停在这里，不再自调度；重新进入视口时由上面的观察器再挂上。
          last.set(0.0);
          handle.set(-1);
          return;
        }
        let prev = last.get();
        // `prev <= 0` 是「刚恢复」或首帧：给一个标准间隔，别把挂起时长算进动画。
        if prev <= 0.0 || ts - prev >= FRAME_MS {
          let dt = if prev <= 0.0 { FRAME_MS } else { ts - prev };
          last.set(ts);
          scene.borrow_mut().advance(dt);
        }

        // 自调度：自己把自己挂到下一帧。`tick` 持有当前闭包，所以这里能拿到它。
        if let (Some(window), Some(cb)) = (web_sys::window(), tick.borrow().as_ref())
          && let Ok(id) = window.request_animation_frame(cb.as_ref().unchecked_ref())
        {
          handle.set(id);
        }
      })
    };
    *tick.borrow_mut() = Some(frame);
    if let Some(cb) = tick.borrow().as_ref()
      && let Ok(id) = window.request_animation_frame(cb.as_ref().unchecked_ref())
    {
      handle.set(id);
    }
  }

  let watchers = SendWrapper::new(Some(Watchers {
    resize: resize_observer,
    _resize_cb: resize_cb,
    intersect: intersect_observer,
    _intersect_cb: intersect_cb,
    tick,
    handle,
  }));
  on_cleanup(move || {
    let Some(w) = watchers.take() else {
      return;
    };
    if let Some(observer) = w.resize {
      observer.disconnect();
    }
    if let Some(observer) = w.intersect {
      observer.disconnect();
    }
    // 顺序要紧：先把排队中的那一帧撤掉，再丢掉闭包。反过来会让回调在闭包释放后被调用。
    if let Some(window) = web_sys::window() {
      let id = w.handle.get();
      if id >= 0 {
        let _ = window.cancel_animation_frame(id);
      }
    }
    *w.tick.borrow_mut() = None;
  });
}
