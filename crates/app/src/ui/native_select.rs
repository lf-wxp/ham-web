//! 下拉选择：原生 `<select>` 的**弹层式**替代。
//!
//! 项目里有大量横向排布、选项固定的筛选器（波段 / 模式 / DXCC / 年份…）。这些位置过去用
//! 原生 `<select>`，但它的下拉面板由浏览器绘制（尤其是移动端），配色、圆角、阴影都与站点
//! 割裂 —— 与导航栏语言切换并排时风格不统一。
//!
//! 因此这里统一改为「触发器 + 弹层列表」，弹层的类名与开合行为全部取自 [`super::popover`]，
//! 与语言切换（[`crate::ui::Select`]）**共用同一份定义**：语言切换长什么样，筛选器就长什么样。
//!
//! 与 [`crate::ui::Select`] 的区别只是接口形态：`Select` 用 `trigger` / `SelectItem` 子节点
//! 描述内容（适合富文本选项），`NativeSelect` 用 `Vec<SelectOption>` 描述数据（适合固定选项）。

use std::sync::atomic::{AtomicU32, Ordering};

use leptos::html;
use leptos::prelude::*;

use crate::cn::cn;
use crate::icons::{Icon, IconKind};

use super::control::{ControlSize, TextValue, invalid_attr};
use super::popover;

/// 键盘高亮行的额外类名。
///
/// `popover::ITEM` 只有 `hover:` / `focus:`，而键盘导航时焦点始终留在触发器上，
/// 这两种状态都不会命中 —— 没有它，方向键移动是「看不见」的。
const ITEM_ACTIVE: &str = "data-[active=true]:bg-accent data-[active=true]:text-accent-foreground";

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

/// 弹层式下拉框（与语言切换同款外观）。
#[component]
pub fn NativeSelect(
  /// 当前选中的值（受控；空串表示「未选择」，此时显示 `placeholder`）。
  #[prop(into)]
  value: Signal<String>,
  /// 选中变化后的新值。
  on_change: Callback<String>,
  /// 选项列表。
  #[prop(into)]
  options: Vec<SelectOption>,
  #[prop(optional)] size: ControlSize,
  /// 未选择时的占位文案（通常传「全部 ××」）。
  #[prop(optional, into)]
  placeholder: Option<TextValue>,
  /// 无可见标签时的无障碍名称（工具栏筛选器必填）。
  #[prop(optional, into)]
  aria_label: Option<TextValue>,
  #[prop(optional, into)] id: Option<String>,
  #[prop(optional, into)] disabled: Signal<bool>,
  #[prop(optional, into)] invalid: Signal<bool>,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  // 空串一律当作「未设置」：`aria-label=""` 会被读屏当成空名称，比属性缺失更糟。
  let aria_label = aria_label.filter(|v| !v.is_empty());
  let id = id.filter(|s| !s.is_empty());
  let placeholder = placeholder.unwrap_or_default();
  // 有占位文案时，弹层首行就是「全部 / 清空」——原生的 `<select>` 正是把 `placeholder`
  // 渲染成一个 `value=""` 的 `<option>`，筛选器靠它回到「全部」。触发器在空值时显示同一文案。
  let has_clear_row = !placeholder.is_empty();
  let clear_label = placeholder.clone();

  let open = RwSignal::new(false);
  let mounted = popover::mount_on_open(open);
  popover::close_on_escape(open);
  // 面板引用：既作 `node_ref`，也给「焦点移出容器就关闭」的判定用
  // （见 `popover::close_on_focus_out`）。
  let panel_ref = NodeRef::<html::Div>::new();
  let state = popover::state(open);
  let trigger_class = popover::trigger_class(size, &class);

  // 选项列表只在触发器与弹层两处按引用读取：放进 `StoredValue`（`Copy`）后，
  // 嵌套的响应式闭包不必「把列表移进去」再移出来（那会让闭包退化成 `FnOnce`）。
  let opts = StoredValue::new(options);

  // ── 键盘可达性 ──────────────────────────────────────────────
  // 焦点始终留在触发器上，用 `aria-activedescendant` 指当前高亮项：`role="option"` 的行
  // 本身不可聚焦（`tabindex="-1"`），没有这套通路的话键盘用户打不开、也走不了列表。
  //
  // 行号约定：第 0 行是「全部 / 清空」（有占位文案时才有），其后依次是数据项。
  let offset = usize::from(has_clear_row);
  let row_count = opts.with_value(|list| list.len()) + offset;
  let active = RwSignal::new(None::<usize>);
  // 选项行的 id 前缀：`aria-activedescendant` 需要一个稳定的 id 去指，`id` 属性缺省时现造一个。
  let base_id = StoredValue::new(id.clone().unwrap_or_else(|| {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    format!("native-select-{}", SEQ.fetch_add(1, Ordering::Relaxed))
  }));
  let opt_id = move |i: usize| format!("{}-opt-{i}", base_id.get_value());
  let value_at = move |i: usize| -> Option<String> {
    if has_clear_row && i == 0 {
      return Some(String::new());
    }
    opts.with_value(|list| list.get(i - offset).map(|o| o.value.clone()))
  };
  // 当前选中值所在的行（打开弹层时把高亮放上去）。
  let selected_row = move || {
    let cur = value.get_untracked();
    if cur.is_empty() {
      return has_clear_row.then_some(0);
    }
    opts.with_value(|list| list.iter().position(|o| o.value == cur).map(|i| i + offset))
  };
  let move_active = move |step: i32| {
    if row_count == 0 {
      return;
    }
    let n = row_count as i32;
    let next = match active.get_untracked() {
      Some(i) => (i as i32 + step).rem_euclid(n) as usize,
      None if step > 0 => 0,
      None => (n - 1) as usize,
    };
    active.set(Some(next));
  };

  // 高亮行必须跟着键盘一起滚进可视区：面板 `max-h-96`、面板内又有一层
  // `overflow-y-auto`，选项多（DXCC / 年份…）时方向键移到了看不见的行上，
  // 用户看到的就是「按了没反应」[`TimePicker`] 在挂载时有同样的处理，可对照。
  let list_ref = NodeRef::<leptos::html::Div>::new();
  Effect::new(move |_| {
    if !mounted.get() {
      return;
    }
    // 订阅高亮位置：`active` 一变就重新滚动。
    let _ = active.get();
    if let Some(el) = list_ref.get()
      && let Ok(Some(node)) = el.query_selector("[data-active=\"true\"]")
    {
      node.scroll_into_view();
    }
  });

  view! {
    <div class="relative">
      <button
        type="button"
        role="combobox"
        id=move || base_id.get_value()
        aria-haspopup="listbox"
        aria-expanded=move || open.get().to_string()
        aria-label=move || aria_label.as_ref().map(TextValue::get)
        aria-activedescendant=move || {
          open.get().then(|| active.get()).flatten().map(opt_id)
        }
        aria-invalid=invalid_attr(invalid)
        data-state=state
        disabled=move || disabled.get()
        class=trigger_class
        on:click=move |_| {
          let next = !open.get_untracked();
          open.set(next);
          if next {
            active.set(selected_row());
          }
        }
        on:keydown=move |e: web_sys::KeyboardEvent| match e.key().as_str() {
          // 打开时把高亮放到当前选中项，而不是永远从第一行开始。
          "ArrowDown" | "ArrowUp" => {
            e.prevent_default();
            if open.get_untracked() {
              move_active(if e.key() == "ArrowDown" { 1 } else { -1 });
            } else {
              open.set(true);
              active.set(selected_row());
            }
          }
          "Home" if open.get_untracked() => {
            e.prevent_default();
            active.set((row_count > 0).then_some(0));
          }
          "End" if open.get_untracked() => {
            e.prevent_default();
            active.set((row_count > 0).then_some(row_count - 1));
          }
          // Enter / 空格在 `<button>` 上会另外合成一次 click（把弹层又关掉），
          // 这里先 `prevent_default` 掐掉它，选中由本分支负责。
          "Enter" | " " if open.get_untracked() => {
            e.prevent_default();
            if let Some(v) = active.get_untracked().and_then(value_at) {
              on_change.run(v);
            }
            open.set(false);
          }
          _ => {}
        }
      >
        <span style="pointer-events: none;">
          {move || {
            let cur = value.get();
            if cur.is_empty() {
              view! { <span class="text-muted-foreground">{placeholder.get()}</span> }.into_any()
            } else {
              let text = opts.with_value(|list| {
                list
                  .iter()
                  .find(|o| o.value == cur)
                  .map_or_else(String::new, |o| o.label.get())
              });
              view! { <span class="block truncate text-left">{text}</span> }.into_any()
            }
          }}
        </span>
        <Icon
          kind=IconKind::ChevronDown
          class=Signal::derive(move || {
            cn(&[
              "h-4 w-4 shrink-0 opacity-50 transition-transform duration-200",
              if open.get() { "rotate-180" } else { "" },
            ])
          })
        />
      </button>
      {move || {
        mounted
          .get()
          .then(|| {
            view! {
              <div class=popover::OVERLAY on:click=move |e| { popover::swallow(&e); open.set(false); }></div>
              <div
                role="listbox"
                data-state=state
                class=popover::PANEL
                node_ref=panel_ref
                on:focusout=popover::close_on_focus_out(open, panel_ref)
              >
                <div node_ref=list_ref class="p-1 max-h-96 w-full overflow-y-auto">
                  {has_clear_row
                    .then(|| {
                      let label = clear_label.clone();
                      view! {
                        <div
                          role="option"
                          id=opt_id(0)
                          aria-selected=move || value.get().is_empty().to_string()
                          data-state=move || if value.get().is_empty() { "checked" } else { "unchecked" }
                          data-active=move || (active.get() == Some(0)).to_string()
                          tabindex="-1"
                          class=cn(&[popover::ITEM, ITEM_ACTIVE])
                          on:click=move |e| {
                            popover::swallow(&e);
                            on_change.run(String::new());
                            open.set(false);
                          }
                        >
                          <span class="absolute left-2 flex h-3.5 w-3.5 items-center justify-center">
                            {move || {
                              value
                                .get()
                                .is_empty()
                                .then(|| view! { <Icon kind=IconKind::Check class="h-4 w-4" /> })
                            }}
                          </span>
                          <span class="w-full text-muted-foreground">{move || label.get()}</span>
                        </div>
                      }
                    })}
                  {move || {
                    let cur = value.get();
                    opts
                      .with_value(|list| {
                        list
                          .iter()
                          .enumerate()
                          .map(|(i, o)| {
                            let v = o.value.clone();
                            let label = o.label.clone();
                            let selected = cur == v;
                            let tick = v.clone();
                            let row = i + offset;
                            view! {
                              <div
                                role="option"
                                id=opt_id(row)
                                aria-selected=selected.to_string()
                                data-state=if selected { "checked" } else { "unchecked" }
                                data-active=move || (active.get() == Some(row)).to_string()
                                tabindex="-1"
                                class=cn(&[popover::ITEM, ITEM_ACTIVE])
                                on:click=move |e| {
                                  popover::swallow(&e);
                                  on_change.run(tick.clone());
                                  open.set(false);
                                }
                              >
                                <span class="absolute left-2 flex h-3.5 w-3.5 items-center justify-center">
                                  {selected.then(|| view! { <Icon kind=IconKind::Check class="h-4 w-4" /> })}
                                </span>
                                <span class="w-full">{move || label.get()}</span>
                              </div>
                            }
                          })
                          .collect_view()
                      })
                  }}
                </div>
              </div>
            }
          })
      }}
    </div>
  }
}
