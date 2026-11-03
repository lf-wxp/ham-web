use ham_web_core::account::{ErrorCode, validate_password};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::account_sync::{self, AccountState};
use crate::i18n::t;
use crate::ui::{
  Button, Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Field, Input,
  InputType, Variant,
};

/// 修改口令对话框。成功后服务器让**其他设备**的登录失效，当前设备保持登录。
#[component]
pub(super) fn AccountPasswordDialog(open: RwSignal<bool>) -> impl IntoView {
  let current = RwSignal::new(String::new());
  let next = RwSignal::new(String::new());
  let again = RwSignal::new(String::new());
  let error = RwSignal::new(String::new());
  let working = RwSignal::new(false);
  let alive = StoredValue::new(crate::util::mount_guard());

  // 关掉对话框就清空输入：口令不该在页面状态里多留一会儿。
  Effect::new(move |_| {
    if !open.get() {
      current.set(String::new());
      next.set(String::new());
      again.set(String::new());
      error.set(String::new());
    }
  });

  let submit = move || {
    if working.get_untracked() {
      return;
    }
    let username = match account_sync::state().get_untracked() {
      AccountState::SignedIn(u) => u.username,
      _ => String::new(),
    };
    let (cur, new) = (current.get_untracked(), next.get_untracked());
    if let Err(e) = validate_password(&new, &username) {
      error.set(account_sync::error_message(ErrorCode::from_credential(e)));
      return;
    }
    if new != again.get_untracked() {
      error.set(t("tools.account-password-mismatch"));
      return;
    }
    error.set(String::new());
    working.set(true);
    let alive = alive.get_value();
    spawn_local(async move {
      let result = account_sync::change_password(&cur, &new).await;
      // 提示是全局的，不依赖本对话框还在不在；本组件自己的信号才要先看。
      let failure = match result {
        Ok(()) => {
          crate::ui::push_success(t("tools.account-password-changed"));
          None
        }
        Err(e) => Some(e),
      };
      if alive() {
        working.set(false);
        match failure {
          Some(e) => error.set(e),
          None => open.set(false),
        }
      }
    });
  };

  view! {
    <Dialog open=open>
      <DialogHeader>
        <DialogTitle>{move || t("tools.account-change-password")}</DialogTitle>
        <DialogDescription>{move || t("tools.account-change-password-desc")}</DialogDescription>
      </DialogHeader>
      <div class="space-y-3">
        <Field label=Signal::derive(move || t("tools.account-current-password")) r#for="account-current-password">
          <Input
            id="account-current-password"
            kind=InputType::Password
            value=current
            on_change=Callback::new(move |v: String| current.set(v))
            autocomplete="current-password"
          />
        </Field>
        <Field
          label=Signal::derive(move || t("tools.account-new-password"))
          r#for="account-new-password"
          hint=Signal::derive(move || t("tools.account-password-rules-hint"))
        >
          <Input
            id="account-new-password"
            kind=InputType::Password
            value=next
            on_change=Callback::new(move |v: String| next.set(v))
            autocomplete="new-password"
          />
        </Field>
        <Field
          label=Signal::derive(move || t("tools.account-repeat-password"))
          r#for="account-repeat-password"
          error=error
        >
          <Input
            id="account-repeat-password"
            kind=InputType::Password
            value=again
            on_change=Callback::new(move |v: String| again.set(v))
            on_enter=Callback::new(move |()| submit())
            autocomplete="new-password"
            invalid=Signal::derive(move || !error.get().is_empty())
          />
        </Field>
      </div>
      <DialogFooter>
        <Button variant=Variant::Outline on_click=Callback::new(move |()| open.set(false))>
          {move || t("exam.cancel")}
        </Button>
        <Button variant=Variant::Default loading=working on_click=Callback::new(move |()| submit())>
          {move || t("tools.account-change-password")}
        </Button>
      </DialogFooter>
    </Dialog>
  }
}
