//! 数据式下拉：传一个 `Vec<SelectOption>` 就得到完整的弹层下拉。
//!
//! 项目里有大量横向排布、选项固定的筛选器（波段 / 模式 / DXCC / 年份…）。它们不需要组合式
//! [`crate::ui::Select`] 的灵活性，只想「给一组选项，拿到一个选中值」。
//!
//! **这里没有任何自己的交互逻辑**：本组件把数据拼成 `Select` + `SelectItem` + `SelectGroup` 交给
//! [`crate::ui::Select`]，键盘、焦点、ARIA、箭头动画、两套皮肤的外观全部来自那一个引擎。
//! 因此语言切换、日志「模式」、页面里的筛选器与题库版本选择器（组合式）行为天然一致，不是靠人工对齐。
//!
//! 历史说明：这个组件早期叫 `SimpleSelect`，最早确实是原生 `<select>` 的样式封装；后来为了弹层外观
//! 与语言切换一致被重写成弹层，名字却一直没改 —— 它从来不是原生控件（Playwright 里要用
//! `getByRole("combobox")`，不能用 `selectOption()`）。现在的名字描述的是接口形态，不是实现。

use leptos::prelude::*;

use super::control::{ControlSize, TextValue};
use super::select::{Select, SelectGroup, SelectItem};

/// 一个下拉选项。
///
/// `label` 用 [`TextValue`]，因此可以传静态文案 `t("log.all-bands")`，也可以在需要跟随语言
/// 切换时传 `Signal::derive(move || t("log.all-bands"))`。
#[derive(Clone, Debug)]
pub struct SelectOption {
  /// 提交值。
  pub value: String,
  /// 展示文案。
  pub label: TextValue,
  /// 所属分组（可空）：同一分组名的相邻选项会归到一个组标题下，标题本身不可选。
  pub group: Option<String>,
}

impl SelectOption {
  /// 构造一个选项。
  pub fn new(value: impl Into<String>, label: impl Into<TextValue>) -> Self {
    Self {
      value: value.into(),
      label: label.into(),
      group: None,
    }
  }

  /// 构造一个带分组的选项（对应原生 `<select>` 的 `optgroup`：同一 `group` 的选项
  /// 归成一组，组名就是 `group`）。
  pub fn grouped(
    group: impl Into<String>,
    value: impl Into<String>,
    label: impl Into<TextValue>,
  ) -> Self {
    Self {
      value: value.into(),
      label: label.into(),
      group: Some(group.into()),
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

/// 数据式下拉框（与语言切换同一个引擎，见模块说明）。
#[component]
pub fn SimpleSelect(
  /// 当前选中的值（受控；空串表示「未选择」，此时显示 `placeholder`）。
  #[prop(into)]
  value: Signal<String>,
  /// 选中变化后的新值。
  on_change: Callback<String>,
  /// 选项列表：直接传 `Vec<SelectOption>`（固定的筛选器），或传 `Signal::derive(…)`
  /// （选项会变，如把表单里的当前值补进列表）。信号变化时，打开着的列表也会跟着更新。
  #[prop(into)]
  options: Signal<Vec<SelectOption>>,
  #[prop(optional)] size: ControlSize,
  /// 未选择时的占位文案（通常传「全部 ××」）。默认情况下有占位文案，弹层首行就是「全部 / 清空」，
  /// 筛选器靠它回到「全部」—— 原生 `<select>` 正是把 `placeholder` 渲染成一个 `value=""` 的 `<option>`。
  #[prop(optional, into)]
  placeholder: Option<TextValue>,
  /// 弹层首行是否放「清空」项（`value=""`）。默认跟随 `placeholder`：有占位文案就放。
  /// 必填项要「空值时显示占位、但不允许选回空」，就传 `false`。
  #[prop(optional)]
  clearable: Option<bool>,
  /// 无可见标签时的无障碍名称（工具栏筛选器必填）。
  #[prop(optional, into)]
  aria_label: Option<TextValue>,
  /// 鼠标悬停提示（原生 `title`）。不传就没有 —— 占位文案不会被当成提示。
  #[prop(optional, into)]
  title: Option<TextValue>,
  #[prop(optional, into)] id: Option<String>,
  #[prop(optional, into)] disabled: Signal<bool>,
  #[prop(optional, into)] invalid: Signal<bool>,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let placeholder = placeholder.unwrap_or_default();
  let has_clear_row = clearable.unwrap_or_else(|| !placeholder.is_empty());
  // 放进 `StoredValue`（`Copy`）：嵌套的响应式闭包不必「把文案移进去」再移出来
  // （那会让闭包退化成 `FnOnce`，而弹层每次打开都要重新构建一遍子节点）。
  let clear_label = StoredValue::new(placeholder.clone());
  let empty_label = placeholder;

  // 引擎的 `value` 是 `Option`（`None` = 没有选中项，显示占位）。这里永远传 `Some`：
  // 空串就是「清空行」的值，让它能像普通选项一样被标成选中（`aria-selected`），
  // 而占位文案由下面的 `trigger` 在值为空时自己渲染。
  let selected = Signal::derive(move || Some(value.get()));
  let trigger = move || {
    let cur = value.get();
    if cur.is_empty() {
      return view! { <span class="text-muted-foreground">{empty_label.get()}</span> }.into_any();
    }
    let text = options.with(|list| {
      list
        .iter()
        .find(|o| o.value == cur)
        .map_or_else(String::new, |o| o.label.get())
    });
    view! { <span class="block truncate text-left">{text}</span> }.into_any()
  };

  let items = move || {
    let clear_row = has_clear_row.then(|| {
      let label = clear_label.get_value();
      view! {
        <SelectItem value=String::new()>
          <span class="text-muted-foreground">{move || label.get()}</span>
        </SelectItem>
      }
    });
    let rows = options.with(|list| {
      // 相邻的同组选项先切成若干段，再逐段渲染；没有组名的选项直接平铺。
      let mut groups: Vec<(Option<&str>, Vec<&SelectOption>)> = Vec::new();
      for o in list {
        match groups.last_mut() {
          Some((name, rows)) if *name == o.group.as_deref() => rows.push(o),
          _ => groups.push((o.group.as_deref(), vec![o])),
        }
      }
      groups
        .into_iter()
        .map(|(name, rows)| {
          let rows = rows
            .into_iter()
            .map(|o| {
              let label = o.label.clone();
              view! { <SelectItem value=o.value.clone()>{move || label.get()}</SelectItem> }
            })
            .collect_view();
          match name {
            None => rows.into_any(),
            Some(name) => {
              view! { <SelectGroup label=name.to_owned()>{rows}</SelectGroup> }.into_any()
            }
          }
        })
        .collect_view()
    });
    view! {
      {clear_row}
      {rows}
    }
  };

  view! {
    <Select
      value=selected
      on_change=on_change
      // 引擎的 `placeholder` 只用于「没有选中项时的占位」和 `title` 的回退 ——
      // 这里值永远是 `Some`、`title` 又由调用方显式给，所以传空。
      placeholder=TextValue::default()
      trigger=trigger
      size=size
      // 未设置透传成空值：引擎把空串一律当作「未设置」（属性不渲染），不必在这里分叉。
      aria_label=aria_label.unwrap_or_default()
      title=title.unwrap_or_default()
      id=id.unwrap_or_default()
      disabled=disabled
      invalid=invalid
      class=class
    >
      // 套一层闭包：选项信号变化时只重算列表里的行，不会把整个弹层（含进场动画）重建一遍。
      {move || items()}
    </Select>
  }
}
