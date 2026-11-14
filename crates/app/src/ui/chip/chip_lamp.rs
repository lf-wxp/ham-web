//! [`ChipLamp`]：像素风波段键里的指示灯。

use leptos::prelude::*;

use super::super::led::{Led, LedState, LedTone};

/// 选中 / 打开时亮绿灯：与「按进去 + 顶部灯条」是同一个状态，灯只是电台面板上的又一个读法
/// （`docs/ui/pixel.md` §3.4）。现代风没有指示灯，什么也不渲染。
#[component]
pub(super) fn ChipLamp(#[prop(into)] on: Signal<bool>) -> impl IntoView {
  (!crate::kit::is_modern_at_mount())
    .then(|| view! { <Led tone=LedTone::Rx state=Signal::derive(move || LedState::lit(on.get())) small=true /> })
}
