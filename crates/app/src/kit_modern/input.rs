//! 现代风文本输入框。
//!
//! 为什么不用像素的 `ui::Input`：那套控件的边框、聚焦环、清除按钮都是像素语言。
//! 这里用原生 `<input>` + `md-input` 类，聚焦环与描边由 `.md-input` 统一定义；
//! 页面里因此不需要手写原生控件（`check-ui` 的 §11 ③ 也会拦）。

use leptos::prelude::*;

use crate::i18n::t;

use super::{MIcon, ModernIcon};

/// 空串视为「未设置」：`id=""` / `min=""` 这类空属性会让无障碍树与校验都变味。
fn non_empty(v: String) -> Option<String> {
  (!v.is_empty()).then_some(v)
}

#[component]
pub fn ModernInput(
  /// 受控值的**只读视图**：外部改它（换筛选条件、由数字信号派生）会同步到控件。
  ///
  /// 用 `Signal` 而不是 `RwSignal`：数字框的源信号是 `u32`，写回要通过 `on_input` 解析，
  /// 组件不该替调用方决定「怎么解析」（`2.` 这种半截输入在 `u32` 上会变成 0）。
  #[prop(into)]
  value: Signal<String>,
  /// 每次输入都回传完整值；清除按钮也走这里。
  #[prop(into)]
  on_input: Callback<String>,
  /// 占位文字；`aria_label` 为空时它兼任无障碍名。
  #[prop(optional, into)]
  placeholder: Signal<String>,
  #[prop(optional, into)] aria_label: Signal<String>,
  /// `search` / `text` / `number` …，默认 `text`。
  #[prop(optional, into)]
  kind: Signal<String>,
  /// 与 `ui::Field` 的 `r#for` 配对。没有可见标签时必须传 `aria_label`，否则控件无名。
  #[prop(optional, into)]
  id: Signal<String>,
  /// 数字框的上下限（原生属性；非 `number` 类型下无副作用）。
  #[prop(optional, into)]
  min: Signal<String>,
  #[prop(optional, into)] max: Signal<String>,
  /// 回车提交（搜索框「按回车跳转」一类场景）。不传就只做普通输入。
  #[prop(optional, into)]
  on_enter: Option<Callback<()>>,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let kind = Signal::derive(move || {
    let k = kind.get();
    if k.is_empty() { "text".to_owned() } else { k }
  });
  // 搜索框把输入法组字的中间串挡在外面（与 `ui::Input` 同一套规则，见 `crate::composition`）；
  // 其余类型逐字回传。事件里读的是当时的 `kind`，所以只在事件处理时判断，不订阅。
  let compose = crate::composition::Composition::new();
  let is_search = move || kind.with_untracked(|k| k == "search");

  view! {
    <div class=format!("md-input-wrap {class}")>
      <input
        class="md-input"
        id=move || non_empty(id.get())
        type=move || kind.get()
        min=move || non_empty(min.get())
        max=move || non_empty(max.get())
        placeholder=move || placeholder.get()
        // 没传标签时退回占位文字：渲染成 `aria-label=""` 会让读屏读出一个空名称
        // （占位文字也被覆盖掉，输入框就成了「无名控件」）。
        aria-label=move || {
          // 标签与占位文字都空时不写这个属性（返回 `None`）：`aria-label=""` 会被读屏当成空名称。
          let l = aria_label.get();
          let name = if l.is_empty() { placeholder.get() } else { l };
          (!name.is_empty()).then_some(name)
        }
        prop:value=move || value.get()
        on:input=move |e| {
          if is_search() {
            if let Some(v) = compose.on_input(&e) {
              on_input.run(v);
            }
          } else {
            on_input.run(event_target_value(&e));
          }
        }
        on:compositionend=move |e| {
          if is_search()
            && let Some(v) = compose.on_end(&e)
          {
            on_input.run(v);
          }
        }
        on:keydown=move |e: web_sys::KeyboardEvent| {
          if e.key() != "Enter" {
            return;
          }
          let Some(submit) = on_enter else {
            return;
          };
          e.prevent_default();
          submit.run(());
        }
      />
      {move || {
        (!value.get().is_empty())
          .then(|| {
            view! {
              <button
                type="button"
                class="md-input-clear"
                aria-label=move || t("exam.clear")
                on:click=move |_| on_input.run(String::new())
              >
                <ModernIcon kind=MIcon::Close />
              </button>
            }
          })
      }}
    </div>
  }
}
