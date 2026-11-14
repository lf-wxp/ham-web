use leptos::prelude::*;

use crate::i18n::tf;
use crate::ui::{Checkbox, Label};
use crate::util::unique_id;

/// 分享成绩图前的「显示我的呼号」开关（默认关）。
///
/// 呼号能反查到姓名与地址，而导出的成绩图是要发给别人的：不能悄悄印上去，要让用户在分享前明确勾选。
/// 状态不落存储（每次打开弹窗都回到「不显示」）：隐私选项宁可多问一次，也不要记住一次「同意」。
/// 日志里没设呼号时整个开关不渲染（没有东西可显示）。两套皮肤的成绩弹层共用。
#[component]
pub fn ShareCallToggle(show: RwSignal<bool>) -> impl IntoView {
  crate::share_score::own_callsign().map(|call| {
    let id = unique_id("share-call");
    view! {
      <div class="flex items-center gap-2 text-xs text-muted-foreground">
        <Checkbox id=id.clone() checked=show on_change=Callback::new(move |v| show.set(v)) />
        <Label r#for=id>{move || tf("exam.qsl-share-call", &[&call])}</Label>
      </div>
    }
  })
}
