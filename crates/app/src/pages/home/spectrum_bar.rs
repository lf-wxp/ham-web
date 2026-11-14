//! 知识频谱里的一根柱：去该分类专项练习的链接。

use ham_web_core::Bank;
use ham_web_core::rig_panel::{S_SEGMENTS, S_WEAK, S9, s_reading};
use leptos::prelude::*;

use crate::data::core_text_tracked;
use crate::i18n::{Locale, locale, t, tf};

use super::knowledge_spectrum::MIN_ANSWERED;

#[component]
pub(super) fn SpectrumBar(
  /// 第几根（0 起），也是屏幕顶部读数的选择键。
  index: usize,
  /// 一级分类 key（专项练习的 `topic` 参数）。
  topic: &'static str,
  /// 一级分类的中文全名（经知识库译文显示）。
  name: &'static str,
  /// 点亮的格数；`None` = 作答太少，没有读数。
  units: Option<u8>,
  bank: Bank,
  active: RwSignal<usize>,
) -> impl IntoView {
  let href = format!(
    "/practice?bank={bank}&topic={}",
    js_sys::encode_uri_component(topic)
  );
  let lit = units.unwrap_or(0);
  view! {
    <li>
      <a
        href=href
        class="pxl-spectrum-bar"
        data-active=move || (active.get() == index).then_some("")
        data-weak=units.is_some_and(|u| u < S_WEAK).then_some("")
        on:mouseenter=move |_| active.set(index)
        on:focus=move |_| active.set(index)
      >
        <span class="pxl-spectrum-cells" aria-hidden="true">
          {(0..S_SEGMENTS)
            .map(|c| {
              view! {
                <span data-lit=(c < lit).then_some("") data-plus=(c >= S9).then_some("")></span>
              }
            })
            .collect_view()}
        </span>
        // 柱底的分类简称：中文就是两字的分类键；英文 / 西语的分类名放不进 1/10 的宽度，写三字母代码。
        // 完整名字在屏幕顶部读数与读屏文本里，这里 `aria-hidden`。
        <span class="pxl-spectrum-index" aria-hidden="true">
          {move || if locale().get() == Locale::Zh { topic.to_owned() } else { spectrum_code(index) }}
        </span>
        <span class="sr-only">
          {move || {
            let reading = units.map_or_else(
              || tf("home.spectrum-few", &[&MIN_ANSWERED.to_string()]),
              s_reading,
            );
            format!("{} · {reading}", core_text_tracked(name))
          }}
        </span>
      </a>
    </li>
  }
}

/// 英文 / 西语柱底的三字母代码（按 `TOP_CATEGORIES` 的顺序）；字面量写在调用点（`check-i18n` 只认直接实参）。
fn spectrum_code(index: usize) -> String {
  match index {
    0 => t("home.spectrum-code-1"),
    1 => t("home.spectrum-code-2"),
    2 => t("home.spectrum-code-3"),
    3 => t("home.spectrum-code-4"),
    4 => t("home.spectrum-code-5"),
    5 => t("home.spectrum-code-6"),
    6 => t("home.spectrum-code-7"),
    7 => t("home.spectrum-code-8"),
    8 => t("home.spectrum-code-9"),
    _ => t("home.spectrum-code-10"),
  }
}
