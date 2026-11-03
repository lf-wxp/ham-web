use std::sync::atomic::{AtomicU32, Ordering};

use ham_web_core::listbox::{Nav, navigate};
use leptos::context::Provider;
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::cn::cn;
use crate::icons::{Icon, IconKind};
use crate::util::document;

use super::super::control::{ControlSize, TextValue, invalid_attr};
use super::super::popover;

/// 可被键盘选中的行。`aria-disabled` 的行不参与方向键导航（目前没有组件会产生它，先把口子留对）。
const OPTION_SELECTOR: &str = r#"[role="option"]:not([aria-disabled="true"])"#;

#[derive(Clone, Copy)]
pub(super) struct SelectCtx {
  pub(super) value: Signal<Option<String>>,
  pub(super) on_change: Callback<String>,
  pub(super) open: RwSignal<bool>,
  /// 键盘高亮行的 DOM id。
  ///
  /// 存 id 而不是值或下标：`SelectItem` 是组合式子节点，根组件事先并不知道有哪些行、什么顺序；
  /// 导航时直接读 DOM 里的 `[role="option"]` 顺序，行自己拿 id 和这里比一下就知道该不该亮。
  pub(super) active: RwSignal<Option<String>>,
}

/// 下拉选择：全站**唯一**的弹层式下拉实现（键盘、焦点、ARIA、箭头动画都只在这里写一遍）。
///
/// 两种用法共用这一个引擎：
/// - 本组件 + [`super::SelectItem`] / [`super::SelectGroup`]：组合式，选项可以是任意内容（图标、徽章、两行文字）；
/// - [`crate::ui::SimpleSelect`]：数据式，传一个 `Vec<SelectOption>`，内部就是用本组件拼出来的。
///
/// `trigger` 为选中项在触发器中的展示内容；`children` 为若干 [`super::SelectItem`]。
///
/// 键盘（焦点始终留在触发器上，用 `aria-activedescendant` 指当前行）：
/// `↓` / `↑` 打开并移动，`Home` / `End` 跳首尾，`Enter` / `Space` 选中，`Esc` 收起，`Tab` 离开即收起。
#[component]
pub fn Select(
  #[prop(into)] value: Signal<Option<String>>,
  on_change: Callback<String>,
  #[prop(optional, into)] disabled: Signal<bool>,
  #[prop(into)] placeholder: TextValue,
  #[prop(into)] trigger: ViewFn,
  /// 触发按钮尺寸（默认与输入框对齐）。
  #[prop(optional)]
  size: ControlSize,
  /// 错误态。
  #[prop(optional, into)]
  invalid: Signal<bool>,
  /// 无可见标签时的无障碍名称。
  #[prop(optional, into)]
  aria_label: Option<TextValue>,
  /// 鼠标悬停提示（原生 `title`）。不传则退回 `placeholder`（语言切换这类无可见标签的控件靠它提示）；
  /// 两者都为空就不写这个属性。
  #[prop(optional, into)]
  title: Option<TextValue>,
  #[prop(optional, into)] id: Option<String>,
  #[prop(optional, into)] class: String,
  children: ChildrenFn,
) -> impl IntoView {
  // 空串一律当作「未设置」：`aria-label=""` 会被读屏当成空名称，`id=""` 也是无效值 ——
  // 都比属性缺失更糟。转发自 [`crate::ui::SimpleSelect`] 的未设置值因此能安全透传。
  let aria_label = aria_label.filter(|v| !v.is_empty());
  let title = title
    .filter(|v| !v.is_empty())
    .or_else(|| (!placeholder.is_empty()).then(|| placeholder.clone()));
  // 列表框也要有名字（axe 的 `aria-input-field-name`），并且名字要**对**：
  // 优先用 `aria_label`，其次用 `title`（没有可见标签的控件，触发器的名字就取自它），
  // 这两者都没有才指回触发器。不能一上来就指回触发器 —— 触发器的内容是**当前选中项**，
  // 读屏念出来会是「2025 年 10 月版本 列表框」而不是「选择题库版本 列表框」；
  // axe 只查「有没有名字」，查不出名字对不对。
  let list_label = aria_label.clone().or_else(|| title.clone());
  let list_labelled_by = list_label.is_none();
  let base_id = StoredValue::new(id.filter(|s| !s.is_empty()).unwrap_or_else(|| {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    format!("select-{}", SEQ.fetch_add(1, Ordering::Relaxed))
  }));
  let listbox_id = move || format!("{}-listbox", base_id.get_value());

  let open = RwSignal::new(false);
  // 关闭后延迟卸载（退场动画），开合行为见 `popover` 模块。
  let mounted = crate::presence::use_presence(open).visible;
  popover::close_on_escape(open);
  let active = RwSignal::new(None::<String>);
  let state = popover::state(open);
  let trigger_class = popover::trigger_class(size, &class);
  // 外层容器：焦点离开它才收起（见 `popover::close_on_focus_leave`）。
  let root_ref = NodeRef::<html::Div>::new();
  // 选项列表的滚动容器：键盘高亮要在它里面滚动，页面不能跟着动。
  let list_ref = NodeRef::<html::Div>::new();
  // 上下文**不能**在组件体里 `provide_context`：Leptos 的组件不会各自开一个作用域，同一个父级下
  // 并排的两个 `Select`，后创建的那个会覆盖先创建的 —— 先创建那个的选项（打开面板时才构建）
  // 读到的是兄弟的值，`aria-selected` / `data-active` 全部判错。下面用 `Provider` 给每个
  // `Select` 自己的作用域（见 e2e `select.spec.ts` 里「同页多个下拉」的用例）。
  let ctx = SelectCtx {
    value,
    on_change,
    open,
    active,
  };

  // ── 键盘高亮 ────────────────────────────────────────────────
  // 关闭时清掉高亮：下次打开重新放到当前选中项上，而不是停在上次关闭前的位置。
  let primed = StoredValue::new(false);
  Effect::new(move |_| {
    if !open.get() {
      primed.set_value(false);
      active.set(None);
    }
  });
  // 面板挂上之后选项才进 DOM：此时把高亮放到当前选中项（没有选中项就不高亮，第一次 `↓` 落到第一行）。
  Effect::new(move |_| {
    if !open.get() || !mounted.get() {
      return;
    }
    // 订阅 `list_ref`：节点挂上时重跑一次，而不是在它还是 `None` 的那一拍就放弃。
    let Some(list) = list_ref.get() else {
      return;
    };
    if primed.get_value() {
      return;
    }
    primed.set_value(true);
    if let Ok(Some(selected)) = list.query_selector(r#"[role="option"][aria-selected="true"]"#) {
      active.set(Some(selected.id()));
    }
  });
  // 高亮行必须跟着键盘一起滚进可视区：面板 `max-h-96`、面板内又有一层 `overflow-y-auto`，
  // 选项多（DXCC / 年份…）时方向键移到了看不见的行上，用户看到的就是「按了没反应」。
  // 按 id 取元素，不依赖 `data-active` 此刻是否已经写到 DOM 上；
  // 只滚列表容器自身，别用 `scroll_into_view`（会把页面一起滚到顶部）。
  Effect::new(move |_| {
    let Some(id) = active.get() else {
      return;
    };
    let Some(list) = list_ref.get() else {
      return;
    };
    if let Some(node) = document().get_element_by_id(&id) {
      popover::scroll_node_into_view_local(&list, &node);
    }
  });

  // 当前 DOM 里按显示顺序排好的可选行。
  let rows = move || -> Vec<web_sys::Element> {
    let Some(list) = list_ref.get_untracked() else {
      return Vec::new();
    };
    let Ok(nodes) = list.query_selector_all(OPTION_SELECTOR) else {
      return Vec::new();
    };
    (0..nodes.length())
      .filter_map(|i| nodes.item(i))
      .filter_map(|n| n.dyn_into::<web_sys::Element>().ok())
      .collect()
  };
  let step = move |nav: Nav| {
    let rows = rows();
    let current = active
      .get_untracked()
      .and_then(|id| rows.iter().position(|r| r.id() == id));
    active.set(navigate(current, rows.len(), nav).map(|i| rows[i].id()));
  };
  let on_keydown = move |e: web_sys::KeyboardEvent| {
    let is_open = open.get_untracked();
    match e.key().as_str() {
      key @ ("ArrowDown" | "ArrowUp") => {
        e.prevent_default();
        if is_open {
          step(if key == "ArrowDown" {
            Nav::Next
          } else {
            Nav::Prev
          });
        } else {
          open.set(true);
        }
      }
      "Home" if is_open => {
        e.prevent_default();
        step(Nav::First);
      }
      "End" if is_open => {
        e.prevent_default();
        step(Nav::Last);
      }
      // Enter / 空格在 `<button>` 上会另外合成一次 click（把弹层又关掉），这里先 `prevent_default`
      // 掐掉它，选中由本分支负责：对高亮行触发一次 click，与鼠标点选走同一条路径（`SelectItem` 的处理）。
      "Enter" | " " if is_open => {
        e.prevent_default();
        let picked = active
          .get_untracked()
          .and_then(|id| document().get_element_by_id(&id))
          .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok());
        match picked {
          Some(row) => row.click(),
          None => open.set(false),
        }
      }
      _ => {}
    }
  };

  let title_label = title;
  let title = move || title_label.as_ref().map(TextValue::get);
  let placeholder_shown = placeholder.clone();
  view! {
    // `min-w-0`：包一层 `relative` 是为了给弹层定位，但它同时也是父级 flex 行里的一个子项 ——
    // 不收缩的话，触发器里的长文案（见 `popover::trigger_class`）会连同这层一起把整行撑宽。
    <div
      class="relative min-w-0"
      node_ref=root_ref
      on:focusout=popover::close_on_focus_leave(open, root_ref)
    >
      <button
        type="button"
        role="combobox"
        id=move || base_id.get_value()
        aria-haspopup="listbox"
        aria-expanded=move || open.get().to_string()
        aria-controls=move || mounted.get().then(listbox_id)
        aria-activedescendant=move || open.get().then(|| active.get()).flatten()
        aria-label=move || aria_label.as_ref().map(TextValue::get)
        aria-invalid=invalid_attr(invalid)
        title=title
        data-state=state
        disabled=move || disabled.get()
        class=trigger_class
        on:click=move |_| open.update(|o| *o = !*o)
        on:keydown=on_keydown
      >
        <span style="pointer-events: none;">
          {move || {
            if value.with(Option::is_some) {
              trigger.run()
            } else {
              view! { <span class="text-muted-foreground">{placeholder_shown.get()}</span> }.into_any()
            }
          }}
        </span>
        <Icon
          kind=IconKind::ChevronDown
          class=Signal::derive(move || {
            cn(&[
              "size-6 shrink-0 opacity-70 transition-transform duration-(--motion-normal)",
              if open.get() { "rotate-180" } else { "" },
            ])
          })
        />
      </button>
      {move || {
        mounted
          .get()
          .then(|| {
            let list_label = list_label.clone();
            // `Provider` 的子节点是一次性闭包，会把它捕获的东西移走：每次挂载面板各用一份克隆。
            let children = children.clone();
            view! {
              <div class=popover::OVERLAY data-state=state inert=popover::inert(open) aria-hidden=popover::aria_hidden(open) on:click=move |e| { popover::swallow(&e); open.set(false); }></div>
              // 外层只管外观与进出场动画（`data-state`）；**可滚动的这一层自己就是 listbox**，
              // 选项成为它的直接子节点 —— 这是 ARIA 对 listbox 的标准形态。
              // 若把 `role="listbox"` 放在外层、里面再套一层带 `tabindex` 的滚动容器，axe 的
              // `aria-required-children` 会认为列表框里没有合格的子节点（可聚焦的中间层不算透明）。
              //
              // `tabindex="-1"`：Chrome 会让「没有可聚焦子元素的滚动容器」进入 Tab 序，
              // 选项行本身都是 `tabindex="-1"`，列表就会变成一个多余的 Tab 停靠点。
              <div data-state=state inert=popover::inert(open) aria-hidden=popover::aria_hidden(open) class=popover::panel_base()>
                <div
                  role="listbox"
                  id=listbox_id()
                  aria-label=move || list_label.as_ref().map(TextValue::get)
                  aria-labelledby=list_labelled_by.then(|| base_id.get_value())
                  node_ref=list_ref
                  tabindex="-1"
                  class="p-1 max-h-96 w-full overflow-y-auto"
                >
                  <Provider value=ctx>{children()}</Provider>
                </div>
              </div>
            }
          })
      }}
    </div>
  }
}
