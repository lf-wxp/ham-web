//! 答题快捷键：`←/→` 切题，`1-9` 选择选项（Shift/Cmd 为多选严格选择），`Enter` 打开搜索。

use leptos::ev;
use leptos::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;

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
}

fn is_typing_target(target: Option<web_sys::EventTarget>) -> bool {
  let Some(el) = target.and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok()) else {
    return false;
  };
  let tag = el.tag_name().to_lowercase();
  tag == "input" || tag == "textarea" || el.is_content_editable()
}

/// 注册全局快捷键，组件卸载时自动移除。
pub fn use_question_shortcuts(opts: Shortcuts) {
  let handle = window_event_listener(ev::keydown, move |e| {
    if is_typing_target(e.target()) {
      return;
    }
    let key = e.key();
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
