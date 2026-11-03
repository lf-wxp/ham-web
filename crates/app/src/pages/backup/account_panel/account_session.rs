use ham_web_core::sync_channel::SyncChannel;
use ham_web_core::text::format_units;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::account_sync::{self, AccountState};
use crate::i18n::{t, tf};
use crate::ui::{Button, Checkbox, ConfirmTone, Variant};

use super::super::auto_sync_switch::AutoSyncSwitch;
use super::super::sync_runner::{SyncAction, use_sync_runner};
use super::super::sync_status_line::SyncStatusLine;
use super::delete_dialog::AccountDeleteDialog;
use super::password_dialog::AccountPasswordDialog;

/// Unix 秒 → 本地时间 `YYYY-MM-DD HH:MM`。
///
/// 用浏览器自己的本地时区（备份是用户在哪台设备上传的，就该按他现在看表的时区读）；
/// 纯数字格式不随语言变，不需要词条。
fn local_datetime(unix_secs: i64) -> String {
  let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(unix_secs as f64 * 1000.0));
  format!(
    "{:04}-{:02}-{:02} {:02}:{:02}",
    d.get_full_year(),
    d.get_month() + 1,
    d.get_date(),
    d.get_hours(),
    d.get_minutes()
  )
}

/// 已登录的账号视图：云端备份概况、自动同步开关、三个手动同步动作与账号管理。
///
/// 手动动作与自定义远端面板是同一个执行器（`use_sync_runner`），只是通道不同。
#[component]
pub(super) fn AccountSession(
  /// 导入 / 导出正在进行。
  busy: Signal<bool>,
  /// 同步进行中；由父组件持有，好让导入 / 导出按钮一并禁用。
  syncing: RwSignal<bool>,
  /// 动作结束（无论成败）后的回调，父组件用它刷新存储占用。
  on_done: Callback<()>,
) -> impl IntoView {
  let state = account_sync::state();
  // 用户信息随刷新变化（云端备份刚更新）：只读响应式信号，不让外壳因此重建本视图。
  let user = Signal::derive(move || match state.get() {
    AccountState::SignedIn(u) => Some(u),
    _ => None,
  });
  let options = RwSignal::new(account_sync::options());
  let password_open = RwSignal::new(false);
  let delete_open = RwSignal::new(false);
  let leaving = RwSignal::new(false);
  let alive = StoredValue::new(crate::util::mount_guard());

  // 动作结束：除了父组件要刷新存储占用，云端概况（大小 / 更新时间）也要重新问一次。
  let refresh_then = Callback::new(move |()| {
    spawn_local(async {
      account_sync::refresh().await;
    });
    on_done.run(());
  });
  let run = use_sync_runner(SyncChannel::Account, busy, syncing, refresh_then);

  let locked = Signal::derive(move || busy.get() || syncing.get());

  // 覆盖会丢掉本机尚未同步出去的改动：先确认一次（与自定义远端一致）。
  let ask_overwrite = move |_| {
    crate::ui::confirm(
      t("tools.account-pull-overwrite-title"),
      t("tools.account-pull-overwrite-desc"),
      t("tools.account-pull-overwrite"),
      ConfirmTone::Danger,
      Callback::new(move |()| run.run(SyncAction::PullOverwrite)),
    );
  };

  let sign_out = move |_| {
    if leaving.get_untracked() {
      return;
    }
    leaving.set(true);
    let alive = alive.get_value();
    spawn_local(async move {
      let result = account_sync::logout().await;
      if result.is_ok() {
        crate::auto_sync::account_signed_out();
      }
      // 提示要照常弹：退出成功时登录态一变，外壳就把本视图卸载了（`alive` 为假），
      // 而 Toast 是全局的，不依赖本视图还在不在。
      match result {
        Ok(()) => crate::ui::push_info(t("tools.account-signed-out")),
        Err(e) => crate::ui::push_error(e),
      }
      if alive() {
        leaving.set(false);
      }
    });
  };

  view! {
    <div class="space-y-3">
      <div class="space-y-1">
        <p class="text-sm" data-testid="account-signed-in-as">
          {move || {
            user
              .get()
              .map(|u| tf("tools.account-signed-in-as", &[&u.username]))
              .unwrap_or_default()
          }}
        </p>
        <p class="text-xs text-muted-foreground" data-testid="account-cloud-summary">
          {move || {
            match user.get().and_then(|u| u.backup) {
              Some(b) => {
                tf(
                  "tools.account-cloud-summary",
                  &[&format_units(b.size as usize), &local_datetime(b.updated_at)],
                )
              }
              None => t("tools.account-cloud-empty"),
            }
          }}
        </p>
      </div>

      <AutoSyncSwitch channel=SyncChannel::Account disabled=Signal::derive(|| false) />

      <div class="space-y-1">
        <div class="flex items-center gap-2">
          <Checkbox
            id="account-exclude-qsl"
            checked=Signal::derive(move || options.get().exclude_qsl_images)
            on_change=Callback::new(move |v: bool| {
              options.update(|o| o.exclude_qsl_images = v);
              account_sync::save_options(&options.get_untracked());
              // 同步范围变了：云端内容的形状也会变，让下一次自动写入按新范围重新比一次。
              crate::auto_sync::mark_dirty();
            })
            aria_label=Signal::derive(move || t("tools.remote-exclude-qsl"))
          />
          <span class="text-sm">{move || t("tools.remote-exclude-qsl")}</span>
        </div>
        <p class="text-xs text-muted-foreground">{move || t("tools.remote-exclude-qsl-hint")}</p>
      </div>

      <SyncStatusLine channel=SyncChannel::Account />

      <div class="flex flex-wrap items-center gap-2">
        <Button
          variant=Variant::Outline
          class=crate::kit::btn_wrap_class()
          disabled=locked
          on_click=Callback::new(ask_overwrite)
        >
          {move || t("tools.account-pull-overwrite")}
        </Button>
        <Button
          variant=Variant::Outline
          class=crate::kit::btn_wrap_class()
          disabled=locked
          on_click=Callback::new(move |()| run.run(SyncAction::PullMerge))
        >
          {move || t("tools.account-pull-merge")}
        </Button>
        <Button
          variant=Variant::Default
          class=crate::kit::btn_wrap_class()
          disabled=locked
          on_click=Callback::new(move |()| run.run(SyncAction::Push))
        >
          {move || t("tools.account-push")}
        </Button>
      </div>

      <div class="flex flex-wrap items-center gap-2 border-t pt-3">
        <Button
          variant=Variant::Outline
          class=crate::kit::btn_wrap_class()
          loading=leaving
          on_click=Callback::new(sign_out)
        >
          {move || t("tools.account-logout")}
        </Button>
        <Button
          variant=Variant::Outline
          class=crate::kit::btn_wrap_class()
          on_click=Callback::new(move |()| password_open.set(true))
        >
          {move || t("tools.account-change-password")}
        </Button>
        <Button
          variant=Variant::Destructive
          class=crate::kit::btn_wrap_class()
          on_click=Callback::new(move |()| delete_open.set(true))
        >
          {move || t("tools.account-delete")}
        </Button>
      </div>

      <AccountPasswordDialog open=password_open />
      <AccountDeleteDialog open=delete_open />
    </div>
  }
}
