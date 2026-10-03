//! 原生 `<select>` 的样式化封装。
//!
//! 项目里有大量横向排布、选项固定的筛选器（波段 / 模式 / DXCC / 年份…）。这类场景用
//! 弹层式 [`crate::ui::Select`] 反而更重（多一层焦点管理与定位），而原生 select 在移动端
//! 由系统接管滚轮、键盘可达性天然完整。这里保留原生元素，只把外观与交互反馈统一到设计
//! 系统：`appearance-none` + 自绘箭头 + 与输入框一致的边框 / 聚焦环 / 禁用 / 错误态。

use leptos::html;
use leptos::prelude::*;

use crate::cn::cn;
use crate::icons::{Icon, IconKind};

use super::control::{ControlSize, TextValue, control_class, invalid_attr};

/// 一个下拉选项。
///
/// `label` 用 [`TextValue`]，因此可以传静态文案 `t("全部波段")`，也可以在需要跟随语言
/// 切换时传 `Signal::derive(move || t("全部波段"))`。
#[derive(Clone, Debug)]
pub struct SelectOption {
  /// 提交值（`<option value>`）。
  pub value: String,
  /// 展示文案。
  pub label: TextValue,
}

impl SelectOption {
  /// 构造一个选项。
  pub fn new(value: impl Into<String>, label: impl Into<TextValue>) -> Self {
    Self {
      value: value.into(),
      label: label.into(),
    }
  }
}

impl From<(&str, &str)> for SelectOption {
  fn from((value, label): (&str, &str)) -> Self {
    Self::new(value, label)
  }
}

impl From<(String, String)> for SelectOption {
  fn from((value, label): (String, String)) -> Self {
    Self::new(value, label)
  }
}

/// 样式化原生下拉框。
#[component]
pub fn NativeSelect(
  /// 当前选中的值（受控；空串通常表示「全部」）。
  #[prop(into)]
  value: Signal<String>,
  /// 选中变化后的新值。
  on_change: Callback<String>,
  /// 选项列表。
  #[prop(into)]
  options: Vec<SelectOption>,
  #[prop(optional)] size: ControlSize,
  /// 传入时会额外渲染一个「空值」首项（文案即本值）。
  #[prop(optional, into)]
  placeholder: Option<TextValue>,
  /// 无可见标签时的无障碍名称（工具栏筛选器必填）。
  #[prop(optional, into)]
  aria_label: Option<TextValue>,
  #[prop(optional, into)] id: Option<String>,
  #[prop(optional, into)] name: Option<String>,
  #[prop(optional, into)] disabled: Signal<bool>,
  #[prop(optional, into)] invalid: Signal<bool>,
  #[prop(optional, into)] class: String,
  #[prop(optional)] node_ref: NodeRef<html::Select>,
) -> impl IntoView {
  // 选中值必须在 `<option>` 挂载之后再写入：先设 value 再插 option 会被浏览器丢弃。
  // 因此不走 `prop:value`，而是「挂载后写一次 + 之后随信号同步」，且只在真的不一致时写回。
  let sync = move |el: web_sys::HtmlSelectElement| {
    let v = value.get_untracked();
    if el.value() != v {
      el.set_value(&v);
    }
  };
  Effect::new(move |_| {
    let v = value.get();
    if let Some(el) = node_ref.get()
      && el.value() != v
    {
      el.set_value(&v);
    }
  });
  node_ref.on_load(sync);

  let class = control_class(size, &cn(&["appearance-none pr-8", &class]));

  view! {
    <div class="relative">
      <select
        node_ref=node_ref
        data-slot="native-select"
        id=id
        name=name
        aria-label=move || aria_label.as_ref().map(TextValue::get)
        aria-invalid=invalid_attr(invalid)
        disabled=move || disabled.get()
        class=class
        on:change=move |e| on_change.run(event_target_value(&e))
      >
        {move || {
          placeholder
            .as_ref()
            .map(TextValue::get)
            .map(|text| view! { <option value="">{text}</option> })
        }}
        {options
          .into_iter()
          .map(|o| {
            view! { <option value=o.value>{move || o.label.get()}</option> }
          })
          .collect_view()}
      </select>
      <Icon
        kind=IconKind::ChevronDown
        class="pointer-events-none absolute right-3 top-1/2 size-4 -translate-y-1/2 opacity-50"
      />
    </div>
  }
}
