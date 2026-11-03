use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::account_sync;
use crate::i18n::t;
use crate::ui::{
  Button, Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Field, Input,
  InputType, Variant,
};

/// 注销账号对话框：再输一次口令确认。账号与云端备份一并删除，**本机数据不动**。
#[component]
pub(super) fn AccountDeleteDialog(open: RwSignal<bool>) -> impl IntoView {
  let password = RwSignal::new(String::new());
  let error = RwSignal::new(String::new());
  let working = RwSignal::new(false);
  let alive = StoredValue::new(crate::util::mount_guard());

  Effect::new(move |_| {
    if !open.get() {
      password.set(String::new());
      error.set(String::new());
    }
  });

  let submit = move || {
    if working.get_untracked() {
      return;
    }
    let pass = password.get_untracked();
    if pass.is_empty() {
      error.set(t("tools.account-err-bad-credentials"));
      return;
    }
    error.set(String::new());
    working.set(true);
    let alive = alive.get_value();
    spawn_local(async move {
      let result = account_sync::delete_account(&pass).await;
      if result.is_ok() {
        crate::auto_sync::account_signed_out();
      }
      // 注销成功时登录态一变，外壳会把整个已登录视图（连同本对话框）卸载：
      // 提示是全局的，要照常弹；只有本组件自己的信号才需要先看还在不在。
      let failure = match result {
        Ok(()) => {
          crate::ui::push_info(t("tools.account-deleted"));
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
        <DialogTitle>{move || t("tools.account-delete")}</DialogTitle>
        <DialogDescription>{move || t("tools.account-delete-desc")}</DialogDescription>
      </DialogHeader>
      <Field
        label=Signal::derive(move || t("tools.account-password"))
        r#for="account-delete-password"
        error=error
      >
        <Input
          id="account-delete-password"
          kind=InputType::Password
          value=password
          on_change=Callback::new(move |v: String| {
            password.set(v);
            error.set(String::new());
          })
          on_enter=Callback::new(move |()| submit())
          autocomplete="current-password"
          invalid=Signal::derive(move || !error.get().is_empty())
        />
      </Field>
      <DialogFooter>
        <Button variant=Variant::Outline on_click=Callback::new(move |()| open.set(false))>
          {move || t("exam.cancel")}
        </Button>
        <Button variant=Variant::Destructive loading=working on_click=Callback::new(move |()| submit())>
          {move || t("tools.account-delete")}
        </Button>
      </DialogFooter>
    </Dialog>
  }
}
