//! 「今日开台清单」的一行：编号 + 任务 + 收益 + 状态灯，整行是一个链接。
//!
//! 行而不是卡片：三条任务并排成三张卡就会与下方的「快捷入口」撞成同一层，
//! 首页要的是「一份清单」（一次性读完、逐条点开），不是两组卡片。

use leptos::prelude::*;

use crate::kit_modern::{MIcon, ModernIcon};

/// 一行任务的状态。颜色由 `data-state` 交给样式层，组件里不写颜色。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OnAirState {
  /// 今日已完成。
  Done,
  /// 今天还没做，但可以立刻做。
  Todo,
  /// 没有次数限制的常驻入口。
  Open,
}

impl OnAirState {
  fn attr(self) -> &'static str {
    match self {
      Self::Done => "done",
      Self::Todo => "todo",
      Self::Open => "open",
    }
  }
}

#[component]
pub(super) fn OnAirRow(
  /// 行首两位编号（与章节尺同一套等宽记号）。
  channel: &'static str,
  href: &'static str,
  #[prop(into)] title: Signal<String>,
  #[prop(into)] hint: Signal<String>,
  #[prop(into)] meta: Signal<String>,
  #[prop(into)] state_label: Signal<String>,
  state: OnAirState,
) -> impl IntoView {
  view! {
    <li>
      <a class="md-onair-row" href=href data-state=state.attr()>
        <span class="md-onair-ch" aria-hidden="true">
          {channel}
        </span>
        <span class="md-onair-main">
          <span class="md-onair-title">{move || title.get()}</span>
          <span class="md-onair-hint">{move || hint.get()}</span>
        </span>
        <span class="md-onair-meta">{move || meta.get()}</span>
        <span class="md-onair-state">
          <span class="md-led" aria-hidden="true"></span>
          {move || state_label.get()}
        </span>
        <ModernIcon kind=MIcon::ArrowRight class="size-4 md-onair-go" />
      </a>
    </li>
  }
}
