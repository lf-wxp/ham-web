use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::account_sync::{self, AccountState};
use crate::i18n::t;
use crate::ui::{Button, Variant};

use super::account_session::AccountSession;
use super::auth_form::AccountAuthForm;

/// 面板该显示哪一块（不带数据）：账号信息变化（云端备份刚更新）时不应把整块重建，
/// 否则正在输入的对话框会被冲掉。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
  Checking,
  Unavailable,
  SignedOut { registration_open: bool },
  SignedIn,
}

/// 在线账号：服务器上的账号保存同一份备份，换设备登录即可取回。
///
/// 与「自定义远端同步」是并列的两条通道：不想自建接口的人用这个，已经有自己服务的人用那个；
/// 两者可以同时配置，自动同步一次只跑其中一条（见 `AutoSyncSwitch`）。
#[component]
pub(in crate::pages::backup) fn AccountPanel(
  /// 导入 / 导出正在进行。
  busy: Signal<bool>,
  /// 同步进行中；由父组件持有，好让导入 / 导出按钮一并禁用。
  syncing: RwSignal<bool>,
  /// 动作结束（无论成败）后的回调，父组件用它刷新存储占用。
  on_done: Callback<()>,
) -> impl IntoView {
  let state = account_sync::state();
  // 每次打开页面问一次：别的设备改过口令、会话过期都要在这里反映出来。
  spawn_local(async {
    account_sync::refresh().await;
  });

  let kind = Memo::new(move |_| match state.get() {
    AccountState::Unknown => Kind::Checking,
    AccountState::Unavailable => Kind::Unavailable,
    AccountState::SignedOut { registration_open } => Kind::SignedOut { registration_open },
    AccountState::SignedIn(_) => Kind::SignedIn,
  });
  let retry = Callback::new(move |()| {
    spawn_local(async {
      account_sync::refresh().await;
    });
  });

  view! {
    // 现代风是页面里的一张 `.md-panel`；像素风保持原来的 inset 块（卡片内嵌，版式冻结）。
    <div
      class=crate::kit::inset_panel_class("space-y-3")
      data-testid="account-panel"
    >
      // 与 `RemoteSyncPanel` 同一写法：像素风保持 `pxl-label`，现代风换区块标题。
      <h2 class=if crate::kit::is_modern_at_mount() { "md-section-title" } else { "pxl-label text-base" }>
        {move || t("tools.account-title")}
      </h2>
      <p class="text-xs text-muted-foreground">{move || t("tools.account-desc")}</p>
      {move || match kind.get() {
        Kind::Checking => {
          view! { <p role="status" class="text-xs">{move || t("tools.account-checking")}</p> }
            .into_any()
        }
        Kind::Unavailable => {
          view! {
            <div class="space-y-2">
              <p role="status" class="text-xs text-hp-text">{move || t("tools.account-err-unavailable")}</p>
              <Button variant=Variant::Outline on_click=retry>
                {move || t("exam.retry")}
              </Button>
            </div>
          }
            .into_any()
        }
        Kind::SignedOut { registration_open } => {
          view! { <AccountAuthForm registration_open=registration_open /> }.into_any()
        }
        Kind::SignedIn => {
          view! { <AccountSession busy=busy syncing=syncing on_done=on_done /> }.into_any()
        }
      }}
    </div>
  }
}
