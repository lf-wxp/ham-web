//! 真机测量面板：在任意地址后加 `?perf=1` 打开，`?perf=0` 关闭（只对当前标签页的会话生效）。
//!
//! 为什么需要：模拟器（降速的 Chromium / 桌面 WebKit）测出来的数字和真机对不上，
//! 「点了要等一会」只能在手机上量。面板固定在左上角，每次点按后显示三项：
//!
//! - **first（首帧）**：按下 → 点击处理完之后的第一帧（`click` 里排一个 rAF，它要等这次点击的同步工作、
//!   包括路由器同步渲染新页面全部做完才轮得到）。这就是「点了多久屏幕才动」。
//! - **route（换页）**：按下 → 地址变化后的第一帧（没换页显示 `-`）。
//! - **worst gap（最长卡帧）**：按下后 1.5 秒内相邻两帧的最大间隔（掉帧 / 长任务的直观读数）。
//!
//! 面板与 `console` 日志的文字是**英文标签**：它只给开发者在真机上看，不是产品文案，不走 `t()`、
//! 也不值得养三语词条；写英文是为了不给「界面文案必须走词条」这条规则开例外
//! （`tools/src/i18n.rs` 的中文字面量白名单里不放它）。
//!
//! 测量期间不更新面板（自己改 DOM 会干扰读数），1.5 秒窗口结束后一次性写入。
//! 存在 `sessionStorage` 而不是 `localStorage`：调试开关不该进备份与跨设备同步。

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

const STORAGE_KEY: &str = "debug:perfHud";
/// 每次点按后的观察窗口。
const WINDOW_MS: f64 = 1500.0;
/// 面板保留的最近记录条数。
const KEEP: usize = 6;

/// 逐帧回调：自己给自己排下一帧，所以要能从里面拿到自己。
type FrameCallback = Closure<dyn FnMut()>;

fn now() -> f64 {
  crate::util::window().performance().map_or(0.0, |p| p.now())
}

/// 读地址里的 `perf=1 / 0`，写进会话；返回最终是否开启。
fn enabled() -> bool {
  let window = crate::util::window();
  let session = window.session_storage().ok().flatten();
  let search = window.location().search().unwrap_or_default();
  let param = search
    .trim_start_matches('?')
    .split('&')
    .find_map(|kv| kv.strip_prefix("perf="));
  if let (Some(v), Some(s)) = (param, session.as_ref()) {
    if v == "1" {
      let _ = s.set_item(STORAGE_KEY, "1");
    } else {
      let _ = s.remove_item(STORAGE_KEY);
    }
  }
  session
    .and_then(|s| s.get_item(STORAGE_KEY).ok().flatten())
    .is_some_and(|v| v == "1")
}

/// 一次点按的测量。
#[derive(Default)]
struct Probe {
  down: f64,
  path: String,
  clicked: bool,
  first_frame: Option<f64>,
  route_frame: Option<f64>,
  last_frame: f64,
  max_gap: f64,
  done: bool,
}

#[component]
pub fn PerfHud() -> impl IntoView {
  if !enabled() {
    return ().into_any();
  }
  let lines = RwSignal::new(Vec::<String>::new());
  let current: Rc<RefCell<Option<Probe>>> = Rc::new(RefCell::new(None));

  // 逐帧：记录帧间隔、检测地址变化；窗口结束后写面板。
  let tick: Rc<RefCell<Option<FrameCallback>>> = Rc::new(RefCell::new(None));
  {
    let current = current.clone();
    let tick_inner = tick.clone();
    *tick.borrow_mut() = Some(Closure::new(move || {
      let t = now();
      let mut finished = None;
      if let Some(p) = current.borrow_mut().as_mut().filter(|p| !p.done) {
        p.max_gap = p.max_gap.max(t - p.last_frame);
        p.last_frame = t;
        if p.route_frame.is_none()
          && crate::util::window()
            .location()
            .pathname()
            .unwrap_or_default()
            != p.path
        {
          p.route_frame = Some(t - p.down);
        }
        if t - p.down >= WINDOW_MS {
          p.done = true;
          let fmt = |v: Option<f64>| v.map_or_else(|| "-".to_owned(), |v| format!("{v:.0}"));
          let to = crate::util::window()
            .location()
            .pathname()
            .unwrap_or_default();
          finished = Some(format!(
            "first {} · route {} · worst gap {:.0} ms  {}",
            fmt(p.first_frame),
            fmt(p.route_frame),
            p.max_gap,
            if to == p.path {
              p.path.clone()
            } else {
              format!("{} → {to}", p.path)
            },
          ));
        }
      }
      if let Some(line) = finished {
        web_sys::console::info_1(&format!("[perf] {line}").into());
        lines.update(|l| {
          l.insert(0, line);
          l.truncate(KEEP);
        });
        return;
      }
      if current.borrow().as_ref().is_some_and(|p| !p.done)
        && let Some(cb) = tick_inner.borrow().as_ref()
      {
        let _ = crate::util::window().request_animation_frame(cb.as_ref().unchecked_ref());
      }
    }));
  }

  let down = {
    let current = current.clone();
    let tick = tick.clone();
    window_event_listener(leptos::ev::pointerdown, move |e| {
      // 鼠标的非主键、多指的后续手指不算一次点按。
      if !e.is_primary() {
        return;
      }
      let running = current.borrow().as_ref().is_some_and(|p| !p.done);
      let t = e.time_stamp();
      *current.borrow_mut() = Some(Probe {
        down: t,
        path: crate::util::window()
          .location()
          .pathname()
          .unwrap_or_default(),
        last_frame: now(),
        ..Probe::default()
      });
      if !running && let Some(cb) = tick.borrow().as_ref() {
        let _ = crate::util::window().request_animation_frame(cb.as_ref().unchecked_ref());
      }
    })
  };

  // 冒泡到 window 的 `click`：所有处理函数都跑完了，这里排的 rAF 就是「这次点击之后的第一帧」。
  let click = {
    let current = current.clone();
    window_event_listener(leptos::ev::click, move |_| {
      let mut slot = current.borrow_mut();
      let Some(p) = slot.as_mut().filter(|p| !p.done && !p.clicked) else {
        return;
      };
      p.clicked = true;
      let current = current.clone();
      let cb = Closure::once_into_js(move || {
        if let Some(p) = current
          .borrow_mut()
          .as_mut()
          .filter(|p| p.first_frame.is_none())
        {
          p.first_frame = Some(now() - p.down);
        }
      });
      let _ = crate::util::window().request_animation_frame(cb.unchecked_ref());
    })
  };

  let handles = send_wrapper::SendWrapper::new(RefCell::new(Some((down, click, tick))));
  on_cleanup(move || {
    if let Some((down, click, tick)) = handles.borrow_mut().take() {
      down.remove();
      click.remove();
      tick.borrow_mut().take();
    }
  });

  view! {
    <div
      aria-hidden="true"
      data-perf-hud=""
      style="position:fixed;left:4px;top:4px;z-index:2147483647;pointer-events:none;max-width:calc(100vw - 8px);padding:4px 6px;border-radius:6px;background:rgb(0 0 0 / 0.72);color:#fff;font:11px/1.35 ui-monospace,Menlo,monospace;white-space:pre"
    >
      {move || {
        let l = lines.get();
        if l.is_empty() { "perf: tap anywhere to measure".to_owned() } else { l.join("\n") }
      }}
    </div>
  }
  .into_any()
}
