use leptos::prelude::*;

use crate::cn::cn;

/// 筛选 / 搜索结果的数量（`role="status"`，改变时读屏会播报一句「共 N 条」）。
///
/// 默认 `sr-only`：像素风的视觉计数留在页面原有的位置（侧栏项的角标、统计卡），
/// 这里只补一条播报 —— `ChipGroup` / `ChipToggle` / 搜索框改变结果集之后，
/// 读屏用户原本完全不知道「还剩几条、是不是筛空了」，只能逐个 Tab 进去数。
///
/// 现代风没有「当前筛选」统计格（默认筛选下它恒等于总数，并排两个一样的大数字只是占位），
/// 于是传 `class` 把它变成**可见**的一行小字，放在列表正上方：筛选改变时它是唯一直接
/// 说明「现在列表里有几条」的地方。可见与播报共用这一个节点，不会念两遍。
///
/// 为什么不直接给页面上那个可见计数加 `aria-live`：它在多数页面里是侧栏角标，
/// 每次筛选会连带播报十几个数字；这里只播一句「共 N 条」。
///
/// `pending`：数据还没到（首屏加载中）。此时 `count` 是 0 但不是「筛空了」，
/// 先不报，等真实数字到了再出文案；占位用不换行空格，可见时这一行的高度不会在数据到达时跳变。
///
/// 用法（放在结果列表附近）：
///
/// ```text
/// <ResultCount count=Signal::derive(move || rows.with(Vec::len)) pending=loading />
/// <ResultCount count=… pending=… class="md-muted md-sm mb-3" />   // 现代风：可见
/// ```
#[component]
pub fn ResultCount(
  #[prop(into)] count: Signal<usize>,
  #[prop(optional, into)] pending: Signal<bool>,
  /// 可见时的类名；缺省 `sr-only`（只播报，不占布局）。
  #[prop(optional, into)]
  class: Option<String>,
) -> impl IntoView {
  let class = class.map_or_else(|| "sr-only".to_owned(), |c| cn(&[&c]));
  view! {
    <p role="status" aria-live="polite" class=class>
      {move || {
        if pending.get() {
          "\u{a0}".to_owned()
        } else {
          let n = count.get();
          crate::i18n::tp("common.result-count", n, &[&n.to_string()])
        }
      }}
    </p>
  }
}
