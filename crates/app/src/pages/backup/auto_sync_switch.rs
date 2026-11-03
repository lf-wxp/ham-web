use ham_web_core::sync_channel::{ChannelPref, SyncChannel};
use leptos::prelude::*;

use crate::i18n::t;
use crate::ui::Switch;

/// 「用这条通道自动同步」开关。
///
/// 两条通道各有一个，**同一时间至多一个是开的**：打开一个，另一个自然关掉；关掉正在跑的这个，
/// 就是都不自动同步（手动按钮仍可用）。开关反映的是 `auto_sync` 里正在跑的通道，
/// 而不是各自存一份布尔值，所以两个开关不会出现「都开着」的状态。
#[component]
pub(super) fn AutoSyncSwitch(
  channel: SyncChannel,
  /// 这条通道现在没法自动同步（没登录 / 没配接口）时禁用。
  #[prop(into)]
  disabled: Signal<bool>,
) -> impl IntoView {
  let active = crate::auto_sync::active_channel();
  // 每个通道的开关 id 不同：同一页里有两个，`id` 重复会让 e2e 定位都错。
  let id = format!("auto-sync-{}", channel.as_str());
  // 两个开关的无障碍名不能相同（同一页里读屏与 `getByRole('switch', { name })` 都要分得清）；
  // 词条字面量写在调用点。
  let label = Signal::derive(move || match channel {
    SyncChannel::Custom => t("tools.remote-auto-switch"),
    SyncChannel::Account => t("tools.account-auto-switch"),
  });
  view! {
    <div class="space-y-1">
      <div class="flex items-center gap-2">
        <Switch
          id=id
          checked=Signal::derive(move || active.get() == Some(channel))
          disabled=disabled
          on_change=Callback::new(move |on: bool| {
            crate::auto_sync::set_pref(
              Some(if on { ChannelPref::of(channel) } else { ChannelPref::Off }),
            );
          })
          aria_label=label
        />
        <span class="text-sm">{move || label.get()}</span>
      </div>
      <p class="text-xs text-muted-foreground">{move || t("tools.sync-auto-switch-hint")}</p>
    </div>
  }
}
