use ham_web_core::eme_plan::utc_stamp;
use ham_web_core::moon::Window;
use leptos::prelude::*;

use crate::i18n::{t, tf};

/// 一组可用窗口；没有就说一句，而不是留一块空白。`id` 写在 `data-slot` 上供 e2e 定位。
///
/// 24 小时的扫描里一端最多出现两三个窗口，所以不截断列表 —— 静默丢条目比多显示一行更糟。
/// 起点 / 终点被查询边界截断的窗口要如实标出来，否则「现在开始、24 小时后结束」会被读成月球真的那时升落。
#[component]
pub(super) fn WindowList(list: Vec<Window>, id: &'static str) -> impl IntoView {
  if list.is_empty() {
    return view! {
      <p class="text-sm text-muted-foreground" data-slot=id>
        {move || t("radio.eme-windows-none")}
      </p>
    }
    .into_any();
  }
  let items = list
    .iter()
    .map(|w| {
      let mut text = tf(
        "radio.eme-window-item",
        &[
          &utc_stamp(w.start_unix),
          &utc_stamp(w.end_unix),
          &format!("{:.0}", w.peak_el_deg),
        ],
      );
      if w.clipped_start {
        text.push_str(&t("radio.eme-window-clip-start"));
      }
      if w.clipped_end {
        text.push_str(&t("radio.eme-window-clip-end"));
      }
      view! { <li>{text}</li> }
    })
    .collect_view();
  view! { <ul class="space-y-1 text-sm tabular-nums" data-slot=id>{items}</ul> }.into_any()
}
