//! [`Table`]：表格根（滚动容器 + `<table>`）。

use leptos::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast as _;
use wasm_bindgen::closure::Closure;

use crate::cn::cn;
use crate::ui::TextValue;

use super::shared::{TableDensity, table_class};

/// 表格根：外层 `overflow-x-auto` 滚动容器（移动端列宽保不住的兜底），内层 `<table>`。
///
/// - 列多了就在 `class` 里写 `min-w-[…]`（作用在 `<table>` 上，与滚动容器配套）；
///   列放得下就不写，滚动容器没有溢出时就是普通块。
/// - 没有可见标题的表格传 `aria_label`（读屏的表格名，e2e 也按它定位）。
/// - `borderless=true` 去掉单元格网格线（极简行列表，如训练记录）。
/// - 需要限制表格区域高度（如 `max-h-[28rem]`）时写在 `wrapper_class`（作用在滚动容器上；
///   `overflow-x-auto` 会让纵向溢出自动变为可滚动，无需再写 `overflow-auto`）。
/// - **键盘可达是默认行为**：容器溢出（`scrollWidth > clientWidth`）时自动加
///   `tabindex="0"` 与 `role="region"`（配 `aria_label` 命名该区域），不溢出时不进 Tab 序 ——
///   以前这两个属性是可选 prop，等于「让调用方记得」，实际几乎没人传，宽表在窄屏上
///   键盘用户就没法横向滚动（WCAG 2.1.1）。调用方仍可显式覆盖。
#[component]
pub fn Table(
  #[prop(optional, into)] aria_label: Option<TextValue>,
  #[prop(optional)] borderless: bool,
  #[prop(optional, into)] class: String,
  #[prop(optional, into)] wrapper_class: String,
  #[prop(optional)] tabindex: Option<&'static str>,
  #[prop(optional)] role: Option<&'static str>,
  /// 密度：默认标准（8/12 内距）；行数 ≥ 15 的表用 [`TableDensity::Compact`]（判据与例外见
  /// `docs/ui/modern.md` §4.3）。
  #[prop(optional)]
  density: TableDensity,
  children: Children,
) -> impl IntoView {
  let aria_label = aria_label.filter(|v| !v.is_empty());
  let region_label = aria_label.clone();
  // `aria-label` 在无 role 的 `div` 上不被支持（axe: aria-prohibited-attr）。
  // 只有确实有区域语义（溢出可滚，或调用方明确要 region）时才把名字挂到滚动容器上。
  let container = NodeRef::<leptos::html::Div>::new();
  let scrollable = RwSignal::new(false);

  // 溢出才可聚焦：既保住键盘可达，又不给每张短表加一个多余的 Tab 停靠点。
  // 观察容器与 `<table>` 两个盒子 —— 只观察容器的话，列宽变化（`min-w-[…]` 生效、
  // 文案折行）不会触发回调。
  // 观察器与它的闭包都要活到组件卸载：`on_load` 里注册的 `on_cleanup` 不在组件 Owner 下，
  // 所以句柄存在 `Rc` 里，由组件体的 `on_cleanup` 统一释放。
  // `on_cleanup` 要求 `Send + Sync`：句柄过不了线程边界（`ResizeObserver` 是 JS 对象），
  // 用 `SendWrapper` 声明「只在主线程用」，与 `tooltip` 里的共享气泡同一手法。
  let observer = SendWrapper::new(std::rc::Rc::new(std::cell::RefCell::new(
    None::<(web_sys::ResizeObserver, Closure<dyn FnMut()>)>,
  )));
  {
    let observer = observer.clone();
    container.on_load(move |el| {
      let target = el.clone();
      let check = Closure::<dyn FnMut()>::new(move || {
        // `clientWidth` 不含滚动条，正好与 `scrollWidth` 同口径。
        scrollable.set(target.scroll_width() > target.client_width());
      });
      let Ok(ro) = web_sys::ResizeObserver::new(check.as_ref().unchecked_ref()) else {
        return;
      };
      ro.observe(&el);
      if let Some(table) = el
        .query_selector("[data-slot=\"table\"]")
        .ok()
        .flatten()
        .and_then(|n| n.dyn_into::<web_sys::Element>().ok())
      {
        ro.observe(&table);
      }
      // 首帧先算一次：`ResizeObserver` 的回调是异步的，首屏不能等它。
      scrollable.set(el.scroll_width() > el.client_width());
      *observer.borrow_mut() = Some((ro, check));
    });
  }
  on_cleanup(move || {
    if let Some((ro, _)) = observer.borrow_mut().take() {
      ro.disconnect();
    }
  });

  let wants_region = role.is_some() || aria_label.is_some();
  let wrapper_label = wants_region.then(|| region_label.clone()).flatten();
  let wrapper_role = move || role.or(scrollable.get().then_some("region"));
  let wrapper_tabindex = move || tabindex.or(scrollable.get().then_some("0"));
  view! {
    <div
      node_ref=container
      class=cn(&["relative w-full overflow-x-auto", &wrapper_class])
      data-slot="table-container"
      tabindex=wrapper_tabindex
      role=wrapper_role
      aria-label=move || wrapper_label.as_ref().map(TextValue::get)
    >
      <table
        data-slot="table"
        aria-label=move || aria_label.as_ref().map(TextValue::get)
        class=table_class(borderless, density, &class)
      >
        {children()}
      </table>
    </div>
  }
}
