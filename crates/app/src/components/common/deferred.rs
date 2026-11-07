//! 首帧之后再挂载的内容块。

use leptos::prelude::*;

/// 把首屏以下的重块（统计面板、地图、奖项、体检报告……）推到新页面第一帧画出来之后再挂载。
///
/// 站内点一下链接，路由器会在同一个任务里把整张新页面同步建完，这期间屏幕一帧都画不出来。
/// 首屏以下的块用户本来就看不见，先让首屏出来、下一帧再建它们，点按到画面变化的时间就只剩首屏那一截
/// （做法与工具页的分批挂载同一个道理，见 `docs/ui/motion.md` §7）。
///
/// `min_height`：占位高度（任意 CSS 长度，如 `"24rem"`；空串 = 不占位）。块还没挂上时先占住这么高，
/// 挂上之后它下面的内容少往下跳一截；浏览器返回时恢复的滚动位置也不会落在「还没长出来」的区域。
/// 取值要取**各皮肤、各宽度下实测高度的下限**：占位比真身高，挂上时下面的内容会反过来往上缩。
/// 块里的内容可能整个不出现（如日志为空时的统计明细）时，传会随状态变空的 `Signal`，别让空页面留白。
/// 不传就不占位（块本身很矮、或它已经在页面最底部、下面没有内容可推时）。
///
/// 地址带锚点、而锚点目标就在这块里（如 `/log#awards`）时：浏览器 / 路由器在挂载前找不到目标、
/// 不会滚动，这里挂完之后补一次定位。
#[component]
pub fn Deferred(
  #[prop(optional, into)] min_height: Signal<String>,
  children: ChildrenFn,
) -> impl IntoView {
  let ready = RwSignal::new(false);
  // 挂载前目标就已经在页面上（在首屏那部分里）时，路由器自己会滚过去，这里不再插手。
  // `location.hash` 是编码形式，元素 `id` 是原文：先还原再查（中文锚点才找得到）。
  let need_scroll = crate::util::window()
    .location()
    .hash()
    .ok()
    .map(|h| crate::util::decode_uri_component(h.trim_start_matches('#')))
    .filter(|id| !id.is_empty() && crate::util::document().get_element_by_id(id).is_none());
  crate::after_paint::after_next_paint(move || {
    // `try_set` 返回 `Some` 说明信号已释放（页面在这一帧里就被换走了）。
    if ready.try_set(true).is_some() {
      return;
    }
    if let Some(id) = need_scroll {
      request_animation_frame(move || {
        if let Some(el) = crate::util::document().get_element_by_id(&id) {
          crate::util::scroll_to_element(&el);
        }
      });
    }
  });
  move || {
    if ready.get() {
      children().into_any()
    } else {
      // 空串 = 不占位：渲染一个空 `<div>` 也会在 `space-y-*` / `gap-*` 容器里多出一道间距。
      Some(min_height.get())
        .filter(|h| !h.is_empty())
        .map(|h| view! { <div aria-hidden="true" style=format!("min-height:{h}")></div> })
        .into_any()
    }
  }
}
