//! 页面形态标记：在 `<body>` 上写 `data-page-mode="practice" | "exam"`。
//!
//! 为什么不用 CSS 的 `body:has(.md-practice)`：它决定的是**版式**（练习 / 考试页收起底部 dock、
//! 底部留白换成底栏高度）。不支持 `:has()` 的浏览器（Firefox 120 及以下）会整条丢掉，结果是手机上
//! dock 压在考试底栏上，点「交卷」跳到导航页 —— 这种失败不能靠「降级」糊弄。
//! 页面自己知道自己是什么形态，由 Rust 写一个属性，CSS 用普通属性选择器，既没有兼容缺口，
//! 也省掉 `body` 级 `:has()` 在每次 DOM 变化时的样式失效开销。
//!
//! 与页面根节点的 `md-practice` / `md-exam` 类名同一口径：只在现代风下生效，随风格切换而增删。

use leptos::prelude::*;

use crate::util::document;

const ATTR: &str = "data-page-mode";

fn set(mode: &str) {
  if let Some(body) = document().body() {
    let _ = body.set_attribute(ATTR, mode);
  }
}

/// 只清掉**自己**写的值：练习 → 考试这类跳转里，新页面的 `set` 可能早于旧页面的清理，
/// 无条件清除会把新页面刚写好的标记抹掉。
fn clear_if(mode: &str) {
  if let Some(body) = document().body()
    && body.get_attribute(ATTR).as_deref() == Some(mode)
  {
    let _ = body.remove_attribute(ATTR);
  }
}

/// 声明「当前页面是 `mode` 形态」。必须在页面组件体（或与根节点同生命周期的闭包）里调用一次。
pub fn use_page_mode(mode: &'static str) {
  Effect::new(move |_| {
    if crate::kit::is_modern() {
      set(mode);
    } else {
      clear_if(mode);
    }
  });
  on_cleanup(move || clear_if(mode));
}
