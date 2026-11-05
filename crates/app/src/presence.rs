//! 弹层「在场」：关闭后先留在 DOM 里播完退场动画，再卸载。**全站唯一的一份实现。**
//!
//! 以前每类弹层各写一份（下拉 / 对话框 / 抽屉 / Toast / 成就 / 现代风六个表面），卸载延迟各自写死成
//! 200 / 200 / 240 / 180 / 180 / 180 毫秒，注释里都写着「要和 CSS 同步」，但没有任何东西保证同步 ——
//! 改了样式表里的时长，Rust 这边不会知道，退场就被截断或多留一帧空壳。
//!
//! 现在延迟**直接读样式表里的时长 token**（见 [`exit_ms`]），并且退场动画自己也用同一个 token：
//! 一个 token，两个使用者（CSS 退场动画、这里的卸载延迟），不可能再各说各话。
//!
//! 约定：
//! - 弹层的**退场是动画而不是过渡**。过渡在「入场动画与 `data-state` 同一次样式变更里被移除」时
//!   不会启动（Chrome 不从动画值起过渡），像素风的 `.pxl-enter[data-state="closed"]` 曾经就这样
//!   从来没动过 —— 面板在关闭后第一帧就是透明的。
//! - 退场动画的时长只能是 [`ExitSpeed::Normal`] 或 [`ExitSpeed::Fast`] 对应的 token，
//!   不能超过卸载延迟（`check-structure` 的「退场时长走 token」守着）。
//! - 退场期间元素还在 DOM 里，但 `data-state="closed"` 的遮罩 / 面板都是 `pointer-events: none`，
//!   不会再吞掉用户紧接着的那次点击。

use std::time::Duration;

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

/// 退场节奏的两档，各对应两套皮肤里一个时长 token。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitSpeed {
  /// 对话框 / 抽屉 / 下拉 / Toast：`--md-dur`（现代）、`--motion-normal`（像素）。
  Normal,
  /// 悬停气泡：来得快也要走得快，`--md-dur-fast`（现代）、`--motion-fast`（像素）。
  Fast,
}

impl ExitSpeed {
  /// `(token 名, 读不到 token 时的兜底毫秒)`。兜底值与 `tokens.css` 里的当前值一致，
  /// 只在样式表还没加载 / 被用户样式覆盖掉时才会用到。
  const fn token(self, modern: bool) -> (&'static str, u64) {
    match (modern, self) {
      (true, Self::Normal) => ("--md-dur", 180),
      (true, Self::Fast) => ("--md-dur-fast", 120),
      (false, Self::Normal) => ("--motion-normal", 240),
      (false, Self::Fast) => ("--motion-fast", 120),
    }
  }
}

/// 卸载比动画结束晚两帧。
///
/// 一帧给「动画真正开始」：`data-state="closed"` 的样式计算在帧内完成，动画从那一刻起算，
/// 而计时器可能早于它；一帧给「动画的最后一帧画出来」，不然会闪一下「还原成可见」。
const FRAME_MS: u64 = 32;

/// 在下一帧再起算卸载计时。
///
/// 退场动画要等浏览器把 `data-state="closed"` 的样式应用上去才开始播，而这里是在
/// `open` 信号刚变假时就跑 —— 两者之间隔了一次 Leptos 更新 + 样式计算。机器忙时这段
/// 间隔能到几十毫秒，计时器会先到点，动画在最后一截被截断（CI 上实测 135ms 卸载 /
/// 180ms 动画）。把起算推到下一帧，计时与动画就从同一起点开始，只剩 [`FRAME_MS`] 的余量。
///
/// 后台标签页里 `requestAnimationFrame` 整个暂停：那时没有「下一帧」可等（动画本身也不会推进），
/// 排进去只会让弹层卡在「退场中」（`inert` + `aria-hidden`）直到用户切回来，所以直接起算。
fn start_exit_timer(wait: Duration, fire: impl FnOnce() + 'static) {
  let window = web_sys::window();
  let hidden = window
    .as_ref()
    .and_then(web_sys::Window::document)
    .is_some_and(|d| d.hidden());
  let pending = std::rc::Rc::new(std::cell::RefCell::new(Some(fire)));
  let scheduled = !hidden
    && window.is_some_and(|w| {
      let queued = pending.clone();
      let start = Closure::once_into_js(move || {
        if let Some(fire) = queued.borrow_mut().take() {
          set_timeout(fire, wait);
        }
      });
      w.request_animation_frame(start.unchecked_ref()).is_ok()
    });
  if !scheduled {
    // 后台标签页 / 拿不到窗口 / 帧回调排不进去（文档正在销毁）：退回同步起算。
    // 宁可少几毫秒余量，也不能让弹层永远不卸载。
    if let Some(fire) = pending.borrow_mut().take() {
      set_timeout(fire, wait);
    }
  }
}

/// 读 `<html>` 上的自定义属性（token 都定义在根上，Portal 出来的弹层也读得到）。
fn root_var(name: &str) -> Option<String> {
  let root = crate::util::document().document_element()?;
  let style = crate::util::window().get_computed_style(&root).ok()??;
  let value = style.get_property_value(name).ok()?;
  (!value.trim().is_empty()).then_some(value)
}

/// 当前风格下，指定档位的「退场动画时长 + 两帧余量」，毫秒。
///
/// 每次关闭时现读（而不是缓存）：用户运行中切换风格，两套皮肤的 token 值并不相同。
#[must_use]
pub fn exit_ms(speed: ExitSpeed) -> u64 {
  let (name, fallback) = speed.token(crate::theme::DisplayPrefs::style_is_modern_dom());
  root_var(name)
    .and_then(|v| ham_web_core::css_time::parse_ms(&v))
    .unwrap_or(fallback)
    + FRAME_MS
}

/// 在场状态：`visible` 决定是否渲染，`closing` 表示正处于退场动画中。
#[derive(Clone, Copy)]
pub struct Presence {
  pub visible: Signal<bool>,
  pub closing: Signal<bool>,
}

/// `open` → 在场状态。关闭后延迟 [`exit_ms`]`(Normal)` 再把 `visible` 置假；期间重新打开会让旧计时器失效。
#[must_use]
pub fn use_presence(open: RwSignal<bool>) -> Presence {
  let visible = RwSignal::new(open.get_untracked());
  let closing = RwSignal::new(false);
  // 代际号：打开与关闭都自增，计时器只在代际号没变时才真正卸载。
  // 只比较 `open` 是不够的：关 → 开 → 关 的快速序列里，第一次关闭的旧计时器到点时 `open`
  // 恰好又是 `false`，会把第二次退场截断在半路。
  let generation = RwSignal::new(0u32);

  Effect::new(move |_| {
    if open.get() {
      generation.update(|g| *g = g.wrapping_add(1));
      closing.set(false);
      visible.set(true);
    } else if visible.get_untracked() && !closing.get_untracked() {
      closing.set(true);
      generation.update(|g| *g = g.wrapping_add(1));
      let mine = generation.get_untracked();
      start_exit_timer(
        Duration::from_millis(exit_ms(ExitSpeed::Normal)),
        move || {
          // 计时器回来时组件可能已经卸载（退场途中跳转页面）：信号随 Owner 释放，
          // `get_untracked` 会 panic 并带崩整个 wasm 实例；`try_*` 在已释放时返回 `None`。
          if generation.try_get_untracked() == Some(mine) {
            visible.try_set(false);
            closing.try_set(false);
          }
        },
      );
    }
  });

  Presence {
    visible: visible.into(),
    closing: closing.into(),
  }
}
