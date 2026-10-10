//! 文本类输入框：替代散落在各页面的原生 `<input>`。
//!
//! 覆盖 `text / search / password / email / tel / url / number / date / time /
//! datetime-local`，另提供前缀 / 后缀插槽与一键清除。

use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::cn::cn;
use crate::i18n::t;
use crate::icons::{Icon, IconKind};

use super::control::{ControlSize, TextValue, control_class, invalid_attr};

/// `<input type>`。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum InputType {
  #[default]
  Text,
  Search,
  Password,
  Email,
  Tel,
  Url,
  Number,
  Date,
  Time,
  DatetimeLocal,
}

impl InputType {
  /// 对应的 `type` 属性值。
  #[must_use]
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Text => "text",
      Self::Search => "search",
      Self::Password => "password",
      Self::Email => "email",
      Self::Tel => "tel",
      Self::Url => "url",
      Self::Number => "number",
      Self::Date => "date",
      Self::Time => "time",
      Self::DatetimeLocal => "datetime-local",
    }
  }

  /// 抹掉浏览器原生外观所需的类名。
  ///
  /// `type="search"` 在 Safari / Chrome 上自带圆角、内阴影和一个原生「×」清除按钮，
  /// 后者会与组件自带的清除按钮叠在一起，必须显式去掉；
  /// 其余类型要么本来就无装饰（`text` / `password` …），要么靠原生装饰才可用
  /// （`date` / `time` 的日历选择器图标），不能一刀切地 `appearance-none`。
  #[must_use]
  pub const fn appearance_class(self) -> &'static str {
    match self {
      Self::Search => {
        "appearance-none [&::-webkit-search-cancel-button]:appearance-none [&::-webkit-search-decoration]:appearance-none"
      }
      _ => "",
    }
  }
}

/// 把空串视为「未设置」，避免渲染出 `min=""` 这类无效属性。
fn non_empty(v: Option<String>) -> Option<String> {
  v.filter(|s| !s.is_empty())
}

/// `type="number"` 的内容是否处在「还没敲完」的状态（`-`、`1.`、`1e`）。
///
/// 这种状态下 `value` IDL 按规范返回**空串**，调用方会把它当成「用户清空了输入框」——
/// 既可能误删已存的值，也可能把用户正在敲的文本抹掉。浏览器只给出 `validity.badInput`
/// 这一个信号，所以半截输入一律不回调（等敲完整了自然会再来一次）。
fn is_incomplete_number(e: &web_sys::Event) -> bool {
  e.target()
    .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
    .is_some_and(|el| el.validity().bad_input())
}

/// 输入框。
///
/// 典型用法见 [`crate::ui`] 模块文档的「使用示例」与 `src/ui/README.md`。
#[component]
pub fn Input(
  /// 当前值（受控）。
  #[prop(into)]
  value: Signal<String>,
  /// 每次输入后的新值。
  on_change: Callback<String>,
  /// 原生 `type`。
  #[prop(optional)]
  kind: InputType,
  /// 尺寸。
  #[prop(optional)]
  size: ControlSize,
  #[prop(optional, into)] placeholder: Option<TextValue>,
  /// 无可见标签时的无障碍名称。
  #[prop(optional, into)]
  aria_label: Option<TextValue>,
  /// 原生 `title`（鼠标悬停提示）。
  #[prop(optional, into)]
  title: Option<TextValue>,
  #[prop(optional, into)] id: Option<String>,
  #[prop(optional, into)] name: Option<String>,
  #[prop(optional, into)] autocomplete: Option<String>,
  #[prop(optional, into)] inputmode: Option<String>,
  #[prop(optional, into)] autocapitalize: Option<String>,
  #[prop(optional, into)] spellcheck: Option<String>,
  /// 关联的外部说明元素 id（例如实时提示区 `aria-live`）。
  #[prop(optional, into)]
  aria_describedby: Option<String>,
  #[prop(optional, into)] min: Option<String>,
  #[prop(optional, into)] max: Option<String>,
  #[prop(optional, into)] step: Option<String>,
  /// `maxlength`（受控：随目标文案长度变化的场合用 `Signal::derive`）。
  #[prop(optional, into)]
  maxlength: Option<Signal<String>>,
  #[prop(optional, into)] disabled: Signal<bool>,
  #[prop(optional, into)] readonly: Signal<bool>,
  /// 错误态：描边转 `destructive`，并输出 `aria-invalid`。
  #[prop(optional, into)]
  invalid: Signal<bool>,
  #[prop(optional)] autofocus: bool,
  /// 有值时显示清除按钮。
  #[prop(optional)]
  clearable: bool,
  /// 左侧插槽（图标、单位等）。
  #[prop(optional, into)]
  prefix: Option<ViewFn>,
  /// 右侧插槽（图标、单位、附加按钮等）。
  #[prop(optional, into)]
  suffix: Option<ViewFn>,
  #[prop(optional, into)] class: String,
  /// 外层定位容器的类名。
  ///
  /// 有前缀 / 后缀 / 清除按钮时，控件外面会多包一层 `relative` 容器用于绝对定位。
  /// `flex-1`、`min-w-*` 这类需要作用在**外层 flex 子项**上的布局类要写在这里，
  /// 写在 `class` 上只会作用于 `<input>` 本身（此时它已不是外层 flex 的直接子项）。
  #[prop(optional, into)]
  wrapper_class: String,
  /// 回车提交回调（搜索框常用）。
  #[prop(optional, into)]
  on_enter: Option<Callback<()>>,
  /// 其余按键（方向键、`Esc`、回车+空格组合的焦点转移等）。比 [`Self::on_enter`]
  /// 更底层：两者都传时，本回调先跑，回车再由 `on_enter` 处理一次。
  #[prop(optional, into)]
  on_keydown: Option<Callback<web_sys::KeyboardEvent>>,
  #[prop(optional)] node_ref: NodeRef<html::Input>,
) -> impl IntoView {
  // 空串一律当作「未设置」：`aria-label=""` 会被读屏当成空名称，`min=""` / `max=""`
  // 会让浏览器把约束判为无效 —— 都比属性缺失更糟。
  let aria_label = aria_label.filter(|v| !v.is_empty());
  let id = non_empty(id);
  let min = non_empty(min);
  let max = non_empty(max);
  let step = non_empty(step);
  let maxlength = maxlength.filter(|v| !v.get().is_empty());

  let pad_l = if prefix.is_some() { "pl-9" } else { "" };
  let pad_r = if suffix.is_some() || clearable {
    "pr-9"
  } else {
    ""
  };
  let input_class = control_class(size, &cn(&[kind.appearance_class(), pad_l, pad_r, &class]));

  let field = view! {
    <input
      node_ref=node_ref
      data-slot="input"
      type=kind.as_str()
      id=id
      name=name
      autocomplete=autocomplete
      inputmode=inputmode
      autocapitalize=autocapitalize
      spellcheck=spellcheck
      aria-describedby=aria_describedby
      min=min
      max=max
      step=step
      maxlength=move || maxlength.map(|v| v.get())
      placeholder=move || placeholder.as_ref().map(TextValue::get)
      title=move || title.as_ref().map(TextValue::get)
      aria-label=move || aria_label.as_ref().map(TextValue::get)
      aria-invalid=invalid_attr(invalid)
      autofocus=autofocus
      disabled=move || disabled.get()
      readonly=move || readonly.get()
      class=input_class
      prop:value=move || value.get()
      on:input=move |e| {
        // 数字框的半截输入（`-`、`1.`）`value` 是空串，别当成「清空」（见 helper 的注释）。
        if kind == InputType::Number && is_incomplete_number(&e) {
          return;
        }
        on_change.run(event_target_value(&e))
      }
      on:keydown=move |e| {
        if let Some(cb) = on_keydown {
          cb.run(e.clone());
        }
        if e.key() == "Enter" && let Some(cb) = on_enter {
          cb.run(());
        }
      }
    />
  }
  .into_any();

  if prefix.is_none() && suffix.is_none() && !clearable {
    return field;
  }

  // 清除按钮默认贴右侧；有后缀插槽时再让出一个后缀位，避免两者叠在一起。
  let clear_pos = if suffix.is_some() {
    "right-8"
  } else {
    "right-2"
  };

  view! {
    <div class=cn(&["relative", &wrapper_class])>
      {field}
      {prefix
        .map(|p| {
          view! {
            <span class="pointer-events-none absolute left-3 top-1/2 flex -translate-y-1/2 items-center text-muted-foreground [&_svg]:size-4">
              {p.run()}
            </span>
          }
        })}
      {suffix
        .map(|s| {
          view! {
            <span class="absolute right-3 top-1/2 flex -translate-y-1/2 items-center text-muted-foreground [&_svg]:size-4">
              {s.run()}
            </span>
          }
        })}
      {clearable
        .then(|| {
          let clear = move |_| on_change.run(String::new());
          view! {
            <Show when=move || !value.get().is_empty()>
              <button
                type="button"
                tabindex="-1"
                aria-label=move || t("exam.clear")
                data-slot="input-clear"
                class=cn(&[
                  "absolute top-1/2 flex size-6 -translate-y-1/2 items-center justify-center rounded-sm text-muted-foreground outline-none transition-colors hover:bg-accent hover:text-accent-foreground focus-visible:ring-ring/50 focus-visible:ring-[3px]",
                  clear_pos,
                ])
                on:click=clear
              >
                <Icon kind=IconKind::X class="size-3.5" />
              </button>
            </Show>
          }
        })}
    </div>
  }
  .into_any()
}
