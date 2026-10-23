//! 全局 Toast：轻量操作反馈（成功 / 失败 / 信息），右下角弹出、自动消失。
//!
//! 页面里调 [`push`] 入队即可，不用自己管理开合与计时；队列播放（入场 → 定时退场 →
//! 动画播完出队）与 [`crate::achievements::AchievementToast`] 同一套。需要用户确认的
//! 失败仍走 [`crate::components::common::MessageDialog`]，Toast 只做「看一眼就够」的
//! 轻反馈（保存成功、已复制、导入完成这类）。

use std::cell::Cell;
use std::collections::VecDeque;
use std::time::Duration;

use ham_web_core::toast_queue::{ToastEntry, enqueue};
use leptos::portal::Portal;
use leptos::prelude::*;

use crate::i18n::t;
use crate::icons::{Icon, IconKind};

/// Toast 类别（决定图标、强调色与停留时长）。定义与入队规则在 core（带单测）。
pub use ham_web_core::toast_queue::ToastKind;

/// 类别对应的（图标色, 左侧色带）。
const fn kind_accent(kind: ToastKind) -> (&'static str, &'static str) {
  match kind {
    ToastKind::Success => ("text-win-text", "border-l-win"),
    ToastKind::Error => ("text-destructive", "border-l-destructive"),
    ToastKind::Info => ("text-primary", "border-l-primary"),
  }
}

thread_local! {
  static QUEUE: ArcRwSignal<VecDeque<ToastEntry>> = ArcRwSignal::new(VecDeque::new());
  /// 队首是否正在播放退场动画。
  ///
  /// `push` 是个不在组件树里的自由函数，读不到 `ToastRoot` 里的 `leaving` 信号；入队规则
  /// 需要它（退场中的队首不接受合并），所以在 `ToastRoot` 改 `leaving` 时同步写一份到这里。
  /// 不需要响应式：它只在 `push` 的那一刻被读一次。
  static FRONT_LEAVING: Cell<bool> = const { Cell::new(false) };
}

/// 入队一条 Toast（同类别同文案会合并成 `×N`，规则见 `ham_web_core::toast_queue`）。
pub fn push(kind: ToastKind, text: impl Into<String>) {
  let text = text.into();
  let leaving = FRONT_LEAVING.with(Cell::get);
  QUEUE.with(|q| {
    q.update(|v| {
      static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
      enqueue(v, kind, text, leaving, || {
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
      });
    })
  });
}

/// 成功反馈。
pub fn push_success(text: impl Into<String>) {
  push(ToastKind::Success, text);
}

/// 失败反馈。
pub fn push_error(text: impl Into<String>) {
  push(ToastKind::Error, text);
}

/// 模态内公告读取当前播放的消息，避免 aria-modal 排除根部 live region。
pub(crate) fn current_message() -> String {
  QUEUE.with(|q| q.with(|v| v.front().map_or_else(String::new, |m| m.text.clone())))
}

/// 退场时长：与 `--motion-fast`（150ms）对齐并留余量，保证退出动画播完再卸载。
const TOAST_EXIT_MS: u64 = 180;

/// 全局 Toast 挂载点：挂在 App 根部，逐条展示、自动消失、可手动关闭。
#[component]
pub fn ToastRoot() -> impl IntoView {
  // `ArcRwSignal` 是 `Clone` 而非 `Copy`，被多个 `move` 闭包捕获时要显式 clone。
  let pending = QUEUE.with(|s| s.clone());
  let front = Memo::new({
    let pending = pending.clone();
    move |_| pending.with(|q| q.front().cloned())
  });
  let leaving = RwSignal::new(false);
  // 同时写一份给 `push` 看（它读不到这个信号），见 `FRONT_LEAVING`。
  let set_leaving = move |v: bool| {
    leaving.set(v);
    FRONT_LEAVING.with(|c| c.set(v));
  };
  let timer: StoredValue<Option<TimeoutHandle>> = StoredValue::new(None);
  // 退场动画播完后的「弹出队首」定时器：与 `timer` 一样需要能在队列变化时取消，
  // 否则退场期间新消息到达、`Effect` 重置 `leaving` 后，这个已排队的 pop 仍会提前出队。
  let pop_timer: StoredValue<Option<TimeoutHandle>> = StoredValue::new(None);

  let pop = {
    let pending = pending.clone();
    move || {
      pending.update(|v| {
        if !v.is_empty() {
          v.remove(0);
        }
      });
    }
  };

  // 触发退场：先置 leaving 播放退出动画，动画播完再 pop。加守卫避免退出期间重复触发。
  let dismiss = {
    let pop = pop.clone();
    move || {
      if !leaving.get_untracked() {
        set_leaving(true);
        let h = set_timeout_with_handle(pop.clone(), Duration::from_millis(TOAST_EXIT_MS)).ok();
        pop_timer.set_value(h);
      }
    }
  };

  // 只在「队首换成另一条」时重置计时：同一条消息被合并（`count` 变大）不该延长它的寿命，
  // 否则用户每多点一次，已经看到的那条就多留几秒。
  let front_id = Memo::new(move |_| front.get().map(|m| m.id));
  Effect::new({
    let dismiss = dismiss.clone();
    move |prev: Option<Option<u64>>| {
      let id = front_id.get();
      if prev == Some(id) {
        return id;
      }
      // 队首变化（新消息入场或旧消息出队）时重置退场态，让下一条从入场态重新开始。
      set_leaving(false);
      if let Some(h) = timer.get_value() {
        h.clear();
      }
      if let Some(h) = pop_timer.get_value() {
        h.clear();
      }
      if let Some(hold) = front.with(|f| f.as_ref().map(|m| m.kind.display_ms())) {
        let h = set_timeout_with_handle(
          {
            let dismiss = dismiss.clone();
            move || dismiss()
          },
          Duration::from_millis(hold),
        )
        .ok();
        timer.set_value(h);
      }
      id
    }
  });

  on_cleanup(move || {
    FRONT_LEAVING.with(|c| c.set(false));
    if let Some(Some(h)) = timer.try_get_value() {
      h.clear();
    }
    if let Some(Some(h)) = pop_timer.try_get_value() {
      h.clear();
    }
  });

  move || {
    // `dismiss` 捕获了非 `Copy` 的 `pending`，本身也不再 `Copy`；外层闭包是 `FnMut`，
    // 每次渲染都要重新 clone 一份交给事件处理器。
    let dismiss = dismiss.clone();
    front.get().map(|m| {
      let icon = match m.kind {
        ToastKind::Success => IconKind::CheckCircle,
        ToastKind::Error => IconKind::XCircle,
        ToastKind::Info => IconKind::AlertCircle,
      };
      let (accent, band) = kind_accent(m.kind);
      view! {
        <Portal>
        <div
          role="status"
          aria-live="polite"
          data-slot="toast"
          data-toast-id=m.id.to_string()
          data-leaving=move || leaving.get().to_string()
          class=format!("motion-toast pxl-popover fixed bottom-20 right-4 z-[60] flex w-72 items-center gap-3 border-l-4 p-3 xl:bottom-4 {band}")
        >
          <Icon kind=icon class=format!("size-6 shrink-0 {accent}") />
          <div class="min-w-0 flex-1 text-sm">
            {m.text.clone()}
            {(m.count > 1)
              .then(|| view! { <span class="ml-1 tabular-nums text-muted-foreground">{format!("×{}", m.count)}</span> })}
          </div>
          <button
            type="button"
            class="opacity-60 transition-opacity hover:opacity-100"
            aria-label=t("learning.dismiss")
            on:click={let dismiss = dismiss.clone(); move |_| dismiss()}
          >
            <Icon kind=IconKind::X class="size-6" />
          </button>
        </div>
        </Portal>
      }
      .into_any()
    })
  }
}
