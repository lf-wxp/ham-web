//! 数字输入：替代原生 `<input type="number">`，可选加减步进按钮。
//!
//! 项目里有上百处数值录入（工具页计算器、通联日志、考试设置），统一由此组件承担：
//! 值与回调仍是 `String`，方便调用方「空串 = 未填写」的既有习惯。

use leptos::html;
use leptos::prelude::*;

use crate::cn::cn;
use crate::i18n::t;
use crate::icons::{Icon, IconKind};

use super::control::{ControlSize, TextValue};
use super::input::{Input, InputType};

/// 按 `step` 的小数位格式化，避免 `0.30000000000000004` 这类浮点尾数。
fn fmt_number(v: f64, step: f64) -> String {
  let decimals = decimals_of(step);
  let p = 10f64.powi(i32::try_from(decimals).unwrap_or(6));
  let v = (v * p).round() / p;
  format!("{v:.decimals$}")
}

fn decimals_of(step: f64) -> usize {
  let s = step.abs();
  // 用容差而不是 `fract() == 0.0`：精确比较在 `0.1 + 0.2` 这类值上会给出意外结果。
  let nearest = s.round();
  if s < f64::EPSILON || (s - nearest).abs() < f64::EPSILON * nearest.max(1.0) {
    return 0;
  }
  let text = format!("{s}");
  text.split('.').nth(1).map_or(0, |f| f.len().clamp(1, 6))
}

/// 数值转原生属性字符串（`300.0` → `300`，不补多余小数）。
fn num_attr(v: f64) -> String {
  format!("{v}")
}

/// 数字输入。
#[component]
pub fn NumberField(
  /// 当前值（受控，空串表示未填写）。
  #[prop(into)]
  value: Signal<String>,
  /// 每次输入 / 步进后的新值。
  on_change: Callback<String>,
  /// 加减步进的步长。
  #[prop(optional, default = 1.0)]
  step: f64,
  #[prop(optional)] min: Option<f64>,
  #[prop(optional)] max: Option<f64>,
  /// 是否显示加减按钮（触屏场景建议开启）。
  #[prop(optional, default = true)]
  controls: bool,
  #[prop(optional)] size: ControlSize,
  #[prop(optional, into)] placeholder: Option<TextValue>,
  /// 无可见标签时的无障碍名称。
  #[prop(optional, into)]
  aria_label: Option<TextValue>,
  #[prop(optional, into)] id: Option<String>,
  #[prop(optional, into)] disabled: Signal<bool>,
  #[prop(optional, into)] invalid: Signal<bool>,
  #[prop(optional, into)] class: String,
  #[prop(optional)] node_ref: NodeRef<html::Input>,
) -> impl IntoView {
  let step_size = if step.abs() < f64::EPSILON { 1.0 } else { step };
  let clamp = move |v: f64| -> f64 {
    let v = min.map_or(v, |m| v.max(m));
    max.map_or(v, |m| v.min(m))
  };
  let bump = Callback::new(move |delta: f64| {
    let cur = value.get_untracked().trim().parse::<f64>().unwrap_or(0.0);
    on_change.run(fmt_number(clamp(cur + delta), step_size));
  });
  // 手输钳制**上限**：只钳 +/− 的话，用户直接敲 999 就能越过 `max`，与「点加号会夹紧」
  // 自相矛盾。下限**不**逐键钳：`min = 5` 时用户想敲 12，钳一下会把 "1" 立刻改成 "5"，
  // 反而拼不出想要的数（下限交给 `bump` 与调用方自己的校验）。
  //
  // 没超限时原样透出，不重排用户正在敲的文本（`1.`、`-` 这类半截输入都能继续敲）。
  let typed = Callback::new(move |raw: String| match raw.trim().parse::<f64>() {
    Ok(v) if v.is_finite() && max.is_some_and(|m| v > m) => {
      on_change.run(fmt_number(max.unwrap_or(v), step_size));
    }
    _ => on_change.run(raw),
  });

  let stepper = move |icon: IconKind, delta: f64, label: Signal<String>| -> ViewFn {
    let bump = bump;
    let disabled = disabled;
    ViewFn::from(move || {
      view! {
        <button
          type="button"
          tabindex="-1"
          aria-label=move || label.get()
          disabled=move || disabled.get()
          class="flex size-6 items-center justify-center rounded-sm text-muted-foreground outline-none transition-colors hover:bg-accent hover:text-accent-foreground focus-visible:ring-ring/50 focus-visible:ring-[3px] disabled:pointer-events-none disabled:opacity-50"
          on:click=move |_| bump.run(delta)
        >
          <Icon kind=icon class="size-3.5" />
        </button>
      }
      .into_any()
    })
  };

  // 有自绘步进按钮时隐藏浏览器原生 spinner，避免两套控件并存。
  let native_hidden = if controls {
    "[&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none"
  } else {
    ""
  };
  let class = cn(&["tabular-nums", native_hidden, &class]);
  // `Input` 的可选属性走「可选 + strip_option」签名，只能接具体值；空串 / 默认文案等价于
  // 不设置（`Input` 内部会把空的 `id` / `aria_label` / `min` / `max` 过滤掉）。
  let min = min.map(num_attr).unwrap_or_default();
  let max = max.map(num_attr).unwrap_or_default();
  let step_attr = fmt_number(step_size, step_size);
  let placeholder = placeholder.unwrap_or_default();
  let aria_label = aria_label.unwrap_or_default();
  let id = id.unwrap_or_default();

  // 步进按钮是可选的：`Input` 的前缀 / 后缀插槽只在传入时预留内边距，
  // 因此两组属性分开渲染，而不是传一个「空插槽」把内边距白白占掉。
  if controls {
    view! {
      <Input
        value=value
        on_change=typed
        kind=InputType::Number
        size=size
        placeholder=placeholder
        aria_label=aria_label
        id=id
        disabled=disabled
        invalid=invalid
        node_ref=node_ref
        min=min
        max=max
        step=step_attr
        prefix=stepper(IconKind::Minus, -step_size, Signal::derive(move || t("common.decrease")))
        suffix=stepper(IconKind::Plus, step_size, Signal::derive(move || t("common.increase")))
        class=class
      />
    }
    .into_any()
  } else {
    view! {
      <Input
        value=value
        on_change=typed
        kind=InputType::Number
        size=size
        placeholder=placeholder
        aria_label=aria_label
        id=id
        disabled=disabled
        invalid=invalid
        node_ref=node_ref
        min=min
        max=max
        step=step_attr
        class=class
      />
    }
    .into_any()
  }
}
