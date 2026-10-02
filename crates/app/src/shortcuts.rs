//! 答题快捷键：`←/→` 切题，`1-9` 选择选项（Shift/Cmd 为多选严格选择），`Enter` 打开搜索，`?` 打开帮助。

use leptos::ev;
use leptos::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;

/// 快捷键说明条目：`(按键, 说明)`。
///
/// 这是键位的**唯一数据源**：帮助面板直接渲染它，避免文档与实现各写一份而逐渐脱节。
/// 均为中文原文，渲染时经 `t()` 翻译。
pub const SHORTCUT_HELP: &[(&str, &str)] = &[
  ("← / →", "上一题 / 下一题"),
  ("1 – 9", "选择对应选项"),
  ("Shift / Cmd + 数字", "多选题只选这一项"),
  ("Enter", "打开题目搜索（顺序练习）"),
  ("/", "打开全站搜索"),
  ("?", "显示快捷键帮助"),
  ("Esc", "关闭对话框"),
];

/// 数字键选择的附加信息。
#[derive(Debug, Clone, Copy)]
pub struct DigitDetail {
  pub strict: bool,
}

pub struct Shortcuts {
  pub on_prev: Callback<()>,
  pub on_next: Callback<()>,
  pub on_digit: Callback<(usize, DigitDetail)>,
  /// 返回 `true` 时 Enter 打开搜索。
  pub enter_search: Option<(Signal<bool>, Callback<()>)>,
  /// 按下 `?` 时的回调；为 `None` 时不响应。
  pub on_help: Option<Callback<()>>,
}

fn is_typing_target(target: Option<web_sys::EventTarget>) -> bool {
  let Some(el) = target.and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok()) else {
    return false;
  };
  let tag = el.tag_name().to_lowercase();
  tag == "input" || tag == "textarea" || el.is_content_editable()
}

/// 自身响应 Enter / 方向键的控件（按钮、链接、下拉、单选组等），交给浏览器默认行为。
fn handles_own_keys(target: Option<web_sys::EventTarget>, key: &str) -> bool {
  let Some(el) = target.and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else {
    return false;
  };
  let tag = el.tag_name().to_lowercase();
  let role = el.get_attribute("role").unwrap_or_default();
  match key {
    "Enter" => {
      matches!(tag.as_str(), "button" | "a" | "select" | "summary")
        || matches!(
          role.as_str(),
          "button" | "link" | "checkbox" | "radio" | "option" | "tab" | "menuitem" | "switch"
        )
    }
    _ => {
      tag == "select"
        || matches!(
          role.as_str(),
          "option" | "tab" | "menuitem" | "slider" | "listbox"
        )
    }
  }
}

/// 注册全局快捷键，组件卸载时自动移除。
pub fn use_question_shortcuts(opts: Shortcuts) {
  let handle = window_event_listener(ev::keydown, move |e| {
    if is_typing_target(e.target()) {
      return;
    }
    let key = e.key();
    if matches!(key.as_str(), "Enter" | "ArrowLeft" | "ArrowRight")
      && handles_own_keys(e.target(), &key)
    {
      return;
    }
    match key.as_str() {
      "ArrowLeft" => {
        e.prevent_default();
        opts.on_prev.run(());
      }
      "ArrowRight" => {
        e.prevent_default();
        opts.on_next.run(());
      }
      "Enter" => {
        if let Some((enabled, cb)) = opts.enter_search
          && enabled.get_untracked()
        {
          e.prevent_default();
          cb.run(());
        }
      }
      // `?` 唤起快捷键帮助；只有接入方提供了回调才拦截。
      "?" => {
        if let Some(cb) = opts.on_help {
          e.prevent_default();
          cb.run(());
        }
      }
      k if k.len() == 1 && matches!(k.as_bytes()[0], b'1'..=b'9') => {
        let n = usize::from(k.as_bytes()[0] - b'0');
        opts.on_digit.run((
          n,
          DigitDetail {
            strict: e.shift_key() || e.meta_key(),
          },
        ));
      }
      _ => {}
    }
  });
  let handle = SendWrapper::new(Some(handle));
  on_cleanup(move || {
    if let Some(h) = handle.take() {
      h.remove();
    }
  });
}

/// 根据数字键计算新的作答：单选直接替换；多选切换，严格模式仅选该项。
pub fn digit_answer(
  options: &[String],
  multiple: bool,
  current: &[String],
  n: usize,
  strict: bool,
) -> Option<Vec<String>> {
  let key = options.get(n.checked_sub(1)?)?.clone();
  if !multiple || strict {
    return Some(vec![key]);
  }
  let mut next: Vec<String> = current.to_vec();
  if let Some(pos) = next.iter().position(|k| *k == key) {
    next.remove(pos);
  } else {
    next.push(key);
  }
  Some(next)
}
