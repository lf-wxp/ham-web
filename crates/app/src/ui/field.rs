//! 表单行：标签 + 控件 + 说明 + 错误提示的统一排版。
//!
//! 页面上反复出现「`<div class="space-y-1.5"><label>…</label><input/></div>`」，
//! 抽出后标签字号、间距、必填标记与错误文案只有一处定义。
//!
//! 除了排版，`Field` 还是**表单语义的唯一下发点**：它把「说明 / 错误节点的 id、
//! 是否必填、现在是否该显示错误」放进 context，控件自己取用。这样调用点既不用
//! 记得给每个 `<p>` 编 id，也不用写 `aria-describedby` ——
//! 漏写一次，读屏就少一条信息（WCAG 1.3.1 / 3.3.1 的常见失分点）。
//!
//! context 用 [`Provider`] 给每个 `Field` 自己的作用域，**不能**在组件体里 `provide_context`：
//! Leptos 的组件不开独立作用域，写在体里会让后一个 `Field` 覆盖前一个，
//! 且排在某个 `Field` 之后、却不在任何 `Field` 里的独立控件会读到前一行的说明 id、必填与「已触碰」
//! （与 `Select` 的 `SelectCtx` 同一个坑，见 `select/select_root.rs`）。
//!
//! 校验时机（[`ValidateOn`]）：默认 `Immediate`，与引入 context 之前的行为一致 ——
//! 站内现有的 `invalid` 都是「边输入边出结果」的计算器 / 实时预警（LoRa、测向、竞赛呼号查重），
//! 红框必须和旁边的结果同步，不能等失焦。表单类字段想要「输入过程中不判错」时显式传
//! `validate_on=ValidateOn::Blur`：控件被触碰过（失焦）或调用点已给了 `error` 文案后才透传。
//! 判定在控件侧做（[`FieldA11y::invalid_for`]），因为 `invalid` 是控件的 prop。

use leptos::context::Provider;
use leptos::prelude::*;

use crate::cn::cn;

use super::control::TextValue;
use super::label_class;

/// 错误提示的出现时机。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ValidateOn {
  /// 每次输入都判定（默认）：红框与旁边的实时结果同步，计算器、查重预警都是这种。
  #[default]
  Immediate,
  /// 控件失焦之后。输入过程中不给红框（避免用户敲第一个字符就被判错）；
  /// 调用点给了 `error` 文案时不受此限，立即显示。
  Blur,
}

/// `Field` 下发给子控件的无障碍信息。
///
/// 控件用 [`FieldA11y::describedby`] 拼 `aria-describedby`、用 [`FieldA11y::required`]
/// 输出 `aria-required`、用 [`FieldA11y::invalid_for`] 决定要不要透传 `invalid`。
#[derive(Clone, Copy)]
pub struct FieldA11y {
  /// 本行「说明 + 错误」节点的 id（空格分隔，可能为空）。
  pub describedby: Signal<String>,
  pub required: Signal<bool>,
  /// 现在是否应该显示错误态。
  show_invalid: Signal<bool>,
  /// 控件失焦时调用，把本行标记为「已触碰」。
  pub mark_touched: Callback<()>,
}

impl FieldA11y {
  /// 把控件自己的 `invalid` 与本行的「该不该显示」合并。
  ///
  /// 没有 `Field` 包裹时（控件单独使用）原样透传 —— 不能因为引入 context
  /// 就让独立控件的错误态失效。
  #[must_use]
  pub fn invalid_for(self, invalid: Signal<bool>) -> Signal<bool> {
    let show = self.show_invalid;
    Signal::derive(move || show.get() && invalid.get())
  }

  /// `aria-describedby` 的取值：空串返回 `None`，让属性整个省略（空属性会被 axe 当成无效引用）。
  #[must_use]
  pub fn describedby_attr(ctx: Option<Self>, own: Option<&str>) -> Option<String> {
    let mut ids: Vec<String> = Vec::new();
    if let Some(a) = ctx {
      let v = a.describedby.get();
      if !v.is_empty() {
        ids.push(v);
      }
    }
    if let Some(own) = own.filter(|s| !s.is_empty()) {
      ids.push(own.to_owned());
    }
    (!ids.is_empty()).then(|| ids.join(" "))
  }
}

/// 表单行。
#[component]
pub fn Field(
  /// 标签文案。
  #[prop(into)]
  label: TextValue,
  children: Children,
  /// 关联控件的 `id`（点击标签即可聚焦）。
  #[prop(optional, into)]
  r#for: Option<String>,
  /// 标签下方的补充说明。
  #[prop(optional, into)]
  hint: Option<TextValue>,
  /// 错误文案；非空即视为错误态。
  #[prop(optional, into)]
  error: Option<Signal<String>>,
  /// 仅「有 / 无错误」而没有具体文案时用这个：非空即渲染一行默认提示。
  ///
  /// 计算器一类的表单只做「能否解析」的判定（`NumberField` 的 `invalid`），
  /// 没有可读的错误句子；以前这种情况界面上只有一圈红边、没有解释。
  #[prop(optional, into)]
  invalid: Signal<bool>,
  #[prop(optional)] required: bool,
  /// 错误提示的出现时机，默认即时（见 [`ValidateOn`]）。
  #[prop(default = ValidateOn::Immediate)]
  validate_on: ValidateOn,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let modern = crate::kit::is_modern_at_mount();
  // 两个 id 都只在本行确实会渲染对应节点时才给出去，避免 `aria-describedby`
  // 指向不存在的元素（axe 会按 `aria-valid-attr-value` 报错）。
  let hint_id = hint.as_ref().map(|_| crate::util::unique_id("field-hint"));
  let hint_id = StoredValue::new(hint_id);
  let error_id = crate::util::unique_id("field-error");
  let has_error_prop = error.is_some();

  let error_is_set = Signal::derive(move || error.map(|e| !e.get().is_empty()).unwrap_or(false));
  let touched = RwSignal::new(false);
  let show_invalid = Signal::derive(move || {
    validate_on == ValidateOn::Immediate || touched.get() || error_is_set.get()
  });
  // 错误节点是否在渲染：与下面两个 `Show` 的条件是**同一个**信号，id 才不会悬空。
  // 给了 `error` 文案就只看文案（`invalid` 此时不渲染节点）；没给才看 `invalid`。
  let error_visible = Signal::derive(move || {
    if has_error_prop {
      error_is_set.get()
    } else {
      show_invalid.get() && invalid.get()
    }
  });

  let describedby = Signal::derive({
    let error_id = error_id.clone();
    move || {
      let mut ids: Vec<String> = Vec::new();
      if let Some(id) = hint_id.get_value() {
        ids.push(id);
      }
      if error_visible.get() {
        ids.push(error_id.clone());
      }
      ids.join(" ")
    }
  });

  let ctx = FieldA11y {
    describedby,
    required: Signal::derive(move || required),
    show_invalid,
    mark_touched: Callback::new(move |()| touched.set(true)),
  };

  let error_class = if modern {
    "md-field-error"
  } else {
    "text-xs text-hp-text"
  };

  view! {
    <div class=if modern { cn(&["md-field", &class]) } else { cn(&["space-y-1.5", &class]) }>
      <label for=r#for class=if modern { "md-field-label".to_owned() } else { label_class("") }>
        {move || label.get()}
        {required
          .then(|| {
            view! { <span class=if modern { "md-field-required" } else { "text-hp-text" } aria-hidden="true">"*"</span> }
          })}
      </label>
      <Provider value=ctx>{children()}</Provider>
      {hint
        .map(|h| {
          view! {
            <p
              id=hint_id.get_value()
              class=if modern { "md-field-hint".to_owned() } else { "text-xs text-muted-foreground".to_owned() }
            >
              {move || h.get()}
            </p>
          }
        })}
      <Show when=move || error_visible.get()>
        <p id=error_id.clone() class=error_class role="alert">
          {move || match error {
            Some(e) => e.get(),
            None => crate::i18n::t("common.check-this-field"),
          }}
        </p>
      </Show>
    </div>
  }
}
