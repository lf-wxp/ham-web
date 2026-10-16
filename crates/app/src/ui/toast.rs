//! 全局 Toast：轻量操作反馈（成功 / 失败 / 信息），右下角弹出、自动消失。
//!
//! 页面里调 [`push`] 入队即可，不用自己管理开合与计时；队列播放（入场 → 定时退场 →
//! 动画播完出队）与 [`crate::achievements::AchievementToast`] 同一套。需要用户确认的
//! 失败仍走 [`crate::components::common::MessageDialog`]，Toast 只做「看一眼就够」的
//! 轻反馈（保存成功、已复制、导入完成这类）。

use std::collections::VecDeque;
use std::time::Duration;

use leptos::portal::Portal;
use leptos::prelude::*;

use crate::i18n::t;
use crate::icons::{Icon, IconKind};

/// Toast 类别（决定图标与强调色）。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
  Success,
  Error,
  Info,
}

/// 一条待展示的 Toast。
#[derive(Clone, PartialEq, Eq)]
struct ToastMsg {
  id: u64,
  kind: ToastKind,
  /// 已翻译的文案（入队时由调用点用 `t(…)` 取好，组件不再持有词条 key）。
  text: String,
}

thread_local! {
  static QUEUE: ArcRwSignal<VecDeque<ToastMsg>> = ArcRwSignal::new(VecDeque::new());
}

/// 入队一条 Toast。
pub fn push(kind: ToastKind, text: impl Into<String>) {
  QUEUE.with(|q| {
    q.update(|v| {
      static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
      v.push_back(ToastMsg {
        id: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        kind,
        text: text.into(),
      })
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

/// Toast 展示时长。
const TOAST_DISPLAY_MS: u64 = 3500;
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
        leaving.set(true);
        let h = set_timeout_with_handle(pop.clone(), Duration::from_millis(TOAST_EXIT_MS)).ok();
        pop_timer.set_value(h);
      }
    }
  };

  Effect::new({
    let dismiss = dismiss.clone();
    move |_| {
      // 队首变化（新消息入场或旧消息出队）时重置退场态，让下一条从入场态重新开始。
      leaving.set(false);
      if front.get().is_some() {
        if let Some(h) = timer.get_value() {
          h.clear();
        }
        if let Some(h) = pop_timer.get_value() {
          h.clear();
        }
        let h = set_timeout_with_handle(
          {
            let dismiss = dismiss.clone();
            move || dismiss()
          },
          Duration::from_millis(TOAST_DISPLAY_MS),
        )
        .ok();
        timer.set_value(h);
      }
    }
  });

  on_cleanup(move || {
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
      let (icon, accent) = match m.kind {
        ToastKind::Success => (IconKind::CheckCircle, "text-win-text"),
        ToastKind::Error => (IconKind::XCircle, "text-destructive"),
        ToastKind::Info => (IconKind::AlertCircle, "text-primary"),
      };
      view! {
        <Portal>
        <div
          role="status"
          aria-live="polite"
          data-slot="toast"
          data-toast-id=m.id.to_string()
          data-leaving=move || leaving.get().to_string()
          class="motion-toast pxl-popover fixed bottom-20 right-4 z-[60] flex w-72 items-center gap-3 p-3 xl:bottom-4"
        >
          <Icon kind=icon class=format!("size-6 shrink-0 {accent}") />
          <div class="min-w-0 flex-1 text-sm">{m.text.clone()}</div>
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
