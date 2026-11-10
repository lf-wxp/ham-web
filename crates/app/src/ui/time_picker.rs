//! 时间选择器：原生 `<input type="time">` 的弹层式替代。
//!
//! 原生时间输入的面板由浏览器绘制，样式无法跟随站点。这里改为「触发器 + 弹层双列列表」
//! （时 / 分各一列），弹层类名与开合行为取自 [`super::popover`] —— 与语言切换完全一致。
//!
//! 对外值与原生输入保持一致：`HH:MM`（24 小时制），空串表示未选择。

use leptos::html;
use leptos::prelude::*;

use crate::cn::cn;
use crate::i18n::t;
use crate::icons::{Icon, IconKind};

use super::control::{ControlSize, TextValue, invalid_attr};
use super::popover;

/// 解析 `HH:MM`。
fn parse_time(s: &str) -> Option<(u32, u32)> {
  let mut parts = s.split(':');
  let h = parts.next()?.parse::<u32>().ok()?;
  let min = parts.next()?.parse::<u32>().ok()?;
  (parts.next().is_none() && h < 24 && min < 60).then_some((h, min))
}

/// 单个可选项类名。
fn opt_class(selected: bool) -> String {
  cn(&[
    popover::item_base(),
    "cursor-pointer justify-center pl-2 font-mono tabular-nums",
    if selected {
      "bg-primary text-primary-foreground border-ink hover:text-primary-foreground"
    } else {
      ""
    },
  ])
}

const COL: &str = "flex max-h-56 flex-1 flex-col gap-0.5 overflow-y-auto p-0.5";

/// 弹层式时间选择器（外观与语言切换一致）。
#[component]
pub fn TimePicker(
  /// 当前值（`HH:MM`，受控；空串表示未选择）。
  #[prop(into)]
  value: Signal<String>,
  /// 选中时间后的新值。
  on_change: Callback<String>,
  #[prop(optional)] size: ControlSize,
  /// 未选择时的占位文案（默认 `HH:MM`）。
  #[prop(optional, into)]
  placeholder: Option<TextValue>,
  /// 无可见标签时的无障碍名称。
  #[prop(optional, into)]
  aria_label: Option<TextValue>,
  #[prop(optional, into)] id: Option<String>,
  #[prop(optional, into)] disabled: Signal<bool>,
  #[prop(optional, into)] invalid: Signal<bool>,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let open = RwSignal::new(false);
  let mounted = crate::presence::use_presence(open).visible;
  crate::focus::use_escape_close(open);
  // 面板引用：既作 `node_ref`，也给「焦点移出容器就关闭」的判定用
  // （见 `popover::close_on_focus_out`）。
  let panel_ref = NodeRef::<html::Div>::new();
  // 落位：面板 `fixed` 定位，放不下时翻到触发器上方（面板自带宽度，不跟触发器）。
  let trigger_ref = NodeRef::<html::Button>::new();
  let panel_style = popover::use_anchored_panel(open, trigger_ref, panel_ref, false);

  let placeholder = placeholder.unwrap_or_else(|| TextValue::from("HH:MM"));
  // 无障碍名称：优先用调用方给的 `aria_label`，否则回退到占位文案。触发器与弹层各持一份，
  // 供各自的响应式闭包读取（`TextValue` 不是 `Copy`，被两个闭包捕获要先克隆）。
  let label = aria_label.clone().unwrap_or_else(|| placeholder.clone());
  let trigger_label = label.clone();
  let trigger_class = popover::trigger_class(size, &class);
  let state = crate::presence::state_attr(open);
  let inert = crate::presence::inert_attr(open);
  let hidden = crate::presence::aria_hidden_attr(open);

  let hour_col = NodeRef::<html::Div>::new();
  let min_col = NodeRef::<html::Div>::new();
  // 弹层出现后把选中项滚进可视区：24 / 60 项时默认停在顶部会让人找不到当前值。
  Effect::new(move |_| {
    if !mounted.get() {
      return;
    }
    // 也要订阅 `value`：弹层开着时列表会随 `value` 重建（DOM 换成新节点），只盯
    // `mounted` 的话新节点不会被滚进可视区（`simple_select` 的同类 Effect 同时依赖
    // `mounted` 与当前激活项）。`track()` 只建立依赖、不取值。
    value.track();
    for col in [hour_col, min_col] {
      if let Some(el) = col.get() {
        popover::scroll_into_view_local(&el, "[aria-selected=\"true\"]");
      }
    }
  });

  let panel = move || {
    mounted.get().then(|| {
      let panel_label = label.get();
      let sel = parse_time(&value.get());

      let hour_opts = (0..24u32)
        .map(|h| {
          let is_sel = sel.is_some_and(|(sh, _)| sh == h);
          view! {
            <button
              type="button"
              role="option"
              aria-selected=is_sel.to_string()
              class=opt_class(is_sel)
              on:click=move |_| {
                let min = parse_time(&value.get_untracked()).map_or(0, |(_, m)| m);
                on_change.run(format!("{h:02}:{min:02}"));
              }
            >
              {format!("{h:02}")}
            </button>
          }
        })
        .collect_view();

      let min_opts = (0..60u32)
        .map(|m| {
          let is_sel = sel.is_some_and(|(_, sm)| sm == m);
          view! {
            <button
              type="button"
              role="option"
              aria-selected=is_sel.to_string()
              class=opt_class(is_sel)
              on:click=move |_| {
                let hour = parse_time(&value.get_untracked()).map_or(0, |(h, _)| h);
                on_change.run(format!("{hour:02}:{m:02}"));
                open.set(false);
              }
            >
              {format!("{m:02}")}
            </button>
          }
        })
        .collect_view();

      view! {
        <div class=popover::overlay_class() data-state=state inert=inert aria-hidden=hidden on:click=move |e| { popover::swallow(&e); open.set(false); }></div>
        <div
          role="dialog"
          aria-label=panel_label
          data-state=state
          inert=inert
          aria-hidden=hidden
          data-slot="time-picker"
          class=popover::panel_class("w-40 p-1")
          style=move || panel_style.get()
          node_ref=panel_ref
          on:focusout=popover::close_on_focus_out(open, panel_ref)
        >
          <div class="flex gap-1">
            // 两列各自给出可访问名称：`role="listbox"` 没有名字时，屏幕阅读器只会念
            // 「列表」，读不出这一列是时还是分（`date_picker` 的每个格子都带 aria-label）。
            <div node_ref=hour_col role="listbox" aria-label=move || t("common.hour") class=COL>
              {hour_opts}
            </div>
            <div node_ref=min_col role="listbox" aria-label=move || t("common.minute") class=COL>
              {min_opts}
            </div>
          </div>
        </div>
      }
    })
  };

  // `Field` 的下发：说明 / 错误 id、必填、错误显示时机（与 `Input` 同一套契约）。
  let field_a11y = use_context::<super::field::FieldA11y>();
  let invalid = match field_a11y {
    Some(a) => a.invalid_for(invalid),
    None => invalid,
  };
  let describedby = move || super::field::FieldA11y::describedby_attr(field_a11y, None);

  view! {
    <div class="relative">
      <button
        node_ref=trigger_ref
        type="button"
        role="combobox"
        id=id
        aria-haspopup="dialog"
        aria-expanded=move || open.get().to_string()
        aria-label=move || trigger_label.get()
        aria-describedby=describedby
        aria-required=move || {
          field_a11y.map(|a| a.required.get()).unwrap_or(false).then_some("true")
        }
        aria-invalid=invalid_attr(invalid)
        data-state=state
        disabled=move || disabled.get()
        class=trigger_class
        on:click=move |_| open.update(|o| *o = !*o)
        on:blur=move |_| {
          if let Some(a) = field_a11y {
            a.mark_touched.run(());
          }
        }
      >
        <span style="pointer-events: none;">
          {move || {
            let v = value.get();
            if v.is_empty() {
              view! { <span class="text-muted-foreground">{placeholder.get()}</span> }.into_any()
            } else {
              view! { <span class="font-mono tabular-nums">{v}</span> }.into_any()
            }
          }}
        </span>
        <Icon kind=IconKind::Clock class="size-6 shrink-0 opacity-70" />
      </button>
      {panel}
    </div>
  }
}
