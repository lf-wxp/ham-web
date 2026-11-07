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

/// 长尾小数的显示收敛：只处理「纯数值且小数位过长」的字符串，其余一律原样返回。
///
/// 不改调用方的值，只影响 `<input>` 的显示。量级越大保留的小数越少：
/// `>=1000` 两位、`>=1` 四位、其余六位，末尾零去掉。
///
/// 三条硬约束（都是踩过的坑，别再放开）：
/// 1. **只给 `type="number"` 用**（调用点判断，见 `Input` 的 `display`）：文本框里
///    的值可能恰好是纯数字串（序号、长整数），收敛会把 `1234567890123456789` 显示成
///    `1234567890123456800` —— 看见的值不是真值；
/// 2. **小数位本来就不超档的原样返回**：整数与短小数没有可收敛的地方，动它就是改值；
/// 3. **数量级太小不收敛**：`1e-7` 会被 6 位小数抹成 `"0"`，看起来像被清零。
fn tidy_number(raw: &str) -> String {
  /// 小于这个数量级的数值不收敛：再往下保留 6 位小数只会得到 `"0"` / `"-0"`。
  const MIN_ABS: f64 = 1e-4;

  let t = raw.trim();
  if t.is_empty() {
    return raw.to_owned();
  }
  if !t
    .chars()
    .all(|c| c.is_ascii_digit() || c == '.' || c == '-' || c == '+')
  {
    return raw.to_owned();
  }
  let Ok(v) = t.parse::<f64>() else {
    return raw.to_owned();
  };
  if !v.is_finite() || v.abs() < MIN_ABS {
    return raw.to_owned();
  }
  let decimals = if v.abs() >= 1000.0 {
    2
  } else if v.abs() >= 1.0 {
    4
  } else {
    6
  };
  // 小数位本来就不超档：没有需要收敛的东西（整数尤其不能碰）。
  if t
    .split('.')
    .nth(1)
    .is_none_or(|frac| frac.len() <= decimals)
  {
    return raw.to_owned();
  }
  let mut s = format!("{v:.decimals$}");
  if s.contains('.') {
    while s.ends_with('0') {
      s.pop();
    }
    if s.ends_with('.') {
      s.pop();
    }
  }
  // 收敛只允许「看起来更整齐」，不允许改动有效数字。
  match s.parse::<f64>() {
    Ok(tidy) if (tidy - v).abs() <= v.abs() * 1e-3 => s,
    _ => raw.to_owned(),
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
///
/// **输入法组字**：`kind=InputType::Search` 的搜索框在组字期间（拼音还没选字）不回调 `on_change`，
/// 选字确认时补回调一次（见 `crate::composition`）；其余类型逐字回调，表单字段的字数计数、
/// `maxlength` 与逐字校验不受影响。
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
  let maxlength = maxlength.filter(|v| !v.get_untracked().is_empty());

  let pad_l = if prefix.is_some() { "pl-9" } else { "" };
  let pad_r = if suffix.is_some() || clearable {
    "pr-9"
  } else {
    ""
  };
  // 现代风只换类名（`.md-input`），DOM 结构与输入事件处理完全共用 —— 这样 46 个计算器、
  // 通联日志、各处搜索框一次性换装，不需要逐个页面改。
  let modern = crate::kit::is_modern_at_mount();
  // 只有搜索框把组字中间串挡在外面（理由见上面的组件文档）。
  let compose = (kind == InputType::Search).then(crate::composition::Composition::new);
  // 计算器的结果经常是 `2.0689655172413794` 这种长尾。聚焦时保持原文（不能动光标），
  // 失焦后按量级取整显示；值本身不变，只是「看起来」干净。
  //
  // 只对 `type="number"` 生效：文本框中纯数字串（序号、长整数）不该被重写，
  // 那会让「看到的值」与「提交的值」不一致。
  // 聚焦期间冻结显示：否则一点进去就从 `2.069` 跳成 `2.0689655172413794`，
  // 用户看到的是一个「爆炸」的数字。失焦后重新按量级收敛。
  let frozen = RwSignal::new(None::<String>);
  let display = Memo::new(move |_| {
    let raw = value.get();
    if !modern || kind != InputType::Number {
      return raw;
    }
    if let Some(f) = frozen.get() {
      return f;
    }
    tidy_number(&raw)
  });
  let input_class = if modern {
    cn(&["md-input", &class])
  } else {
    control_class(size, &cn(&[kind.appearance_class(), pad_l, pad_r, &class]))
  };

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
      prop:value=move || display.get()
      on:focus=move |_| frozen.set(Some(display.get_untracked()))
      on:blur=move |_| frozen.set(None)
      on:input=move |e| {
        // 数字框的半截输入（`-`、`1.`）`value` 是空串，别当成「清空」（见 helper 的注释）。
        if kind == InputType::Number && is_incomplete_number(&e) {
          return;
        }
        // 搜索框：输入法组字中（拼音还没选字）不派发 —— 打「天线」要先派发 t / ti / tia / tian …
        // 每个字母都触发一次整份搜索重算，而这些中间串本来也搜不出东西。组字结束时补派一次。
        match compose {
          Some(c) => {
            if let Some(v) = c.on_input(&e) {
              on_change.run(v);
            }
          }
          None => on_change.run(event_target_value(&e)),
        }
      }
      on:compositionend=move |e| {
        if let Some(c) = compose
          && let Some(v) = c.on_end(&e)
        {
          on_change.run(v);
        }
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

  let (wrap_class, prefix_class, suffix_class) = if modern {
    (
      "md-input-wrap",
      "md-adorn md-adorn-left",
      "md-adorn md-adorn-right",
    )
  } else {
    (
      "relative",
      "pointer-events-none absolute left-3 top-1/2 flex -translate-y-1/2 items-center text-muted-foreground [&_svg]:size-4",
      "absolute right-3 top-1/2 flex -translate-y-1/2 items-center text-muted-foreground [&_svg]:size-4",
    )
  };

  // 有图标时输入框要让出内边距。以前靠 `.md-input-wrap:has(.md-adorn-left)`，Firefox 120 及以下
  // 不认 `:has()`，图标就压在输入文字上；现在由这里写属性，CSS 用普通属性选择器。
  let adorn_left = (modern && prefix.is_some()).then_some("");
  let adorn_right = (modern && suffix.is_some()).then_some("");

  view! {
    <div
      class=cn(&[wrap_class, &wrapper_class])
      data-adorn-left=adorn_left
      data-adorn-right=adorn_right
    >
      {field}
      {prefix
        .map(|p| {
          view! { <span class=prefix_class>{p.run()}</span> }
        })}
      {suffix
        .map(|s| {
          view! { <span class=suffix_class>{s.run()}</span> }
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
                class=if modern {
                  "md-input-clear".to_owned()
                } else {
                  cn(&[
                    "absolute top-1/2 flex size-6 -translate-y-1/2 items-center justify-center border-2 border-transparent text-muted-foreground outline-none hover:border-ink hover:bg-accent hover:text-accent-foreground focus-visible:outline-2 focus-visible:outline-dashed focus-visible:outline-ring",
                    clear_pos,
                  ])
                }
                on:click=clear
              >
                <Icon kind=IconKind::X class="size-6" />
              </button>
            </Show>
          }
        })}
    </div>
  }
  .into_any()
}

#[cfg(test)]
mod tests {
  use super::tidy_number;

  #[test]
  fn tidy_number_shortens_only_long_tails() {
    // 计算器结果的长尾：`>=1` 保留四位小数，末尾零去掉。
    assert_eq!(tidy_number("2.0689655172413794"), "2.069");
    assert_eq!(tidy_number("1234.5678"), "1234.57");
    assert_eq!(tidy_number("14.074000"), "14.074");
  }

  #[test]
  fn tidy_number_leaves_integers_and_short_decimals_alone() {
    // 整数没有可收敛的小数位：动它就是改值（长整数经 f64 往返会丢低位）。
    assert_eq!(tidy_number("1234567890123456789"), "1234567890123456789");
    assert_eq!(tidy_number("12"), "12");
    assert_eq!(tidy_number("12.5"), "12.5");
    assert_eq!(tidy_number("0"), "0");
  }

  #[test]
  fn tidy_number_never_shows_a_tiny_value_as_zero() {
    // 6 位小数会把 1e-7 抹成 "0"：那样用户读到的是「值被清零了」。
    assert_eq!(tidy_number("0.0000001"), "0.0000001");
    assert_eq!(tidy_number("-0.0000001"), "-0.0000001");
  }

  #[test]
  fn tidy_number_ignores_non_numeric_text() {
    assert_eq!(tidy_number(""), "");
    assert_eq!(tidy_number("14.074 MHz"), "14.074 MHz");
    assert_eq!(tidy_number("尚在输入"), "尚在输入");
    assert_eq!(tidy_number("1.2.3"), "1.2.3");
  }
}
