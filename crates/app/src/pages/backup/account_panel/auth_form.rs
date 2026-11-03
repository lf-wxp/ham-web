use ham_web_core::account::{ErrorCode, validate_password, validate_username};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::account_sync;
use crate::i18n::t;
use crate::ui::{Button, Field, Input, InputType, Variant};

/// 登录 / 注册表单（没登录时显示）。
///
/// 一份表单两个按钮：账号名与口令是同一组输入，分成两个页签只会让用户在「我到底注册过没有」
/// 之间来回切。回车 = 登录；注册按钮只在服务器开放注册时出现。
/// 注册时先按 core 的规则本地校验，错误当场说清；登录不做本地规则校验（规则以后可能收紧，
/// 不能把以前合法的老账号挡在门外），一律交给服务器。
#[component]
pub(super) fn AccountAuthForm(registration_open: bool) -> impl IntoView {
  let username = RwSignal::new(String::new());
  let password = RwSignal::new(String::new());
  let working = RwSignal::new(false);
  // 账号名与口令各自的就地错误；服务器返回的错误按所属字段放。
  let name_error = RwSignal::new(String::new());
  let pass_error = RwSignal::new(String::new());
  let alive = StoredValue::new(crate::util::mount_guard());

  let submit = move |register: bool| {
    if working.get_untracked() {
      return;
    }
    let (name, pass) = (username.get_untracked(), password.get_untracked());
    name_error.set(String::new());
    pass_error.set(String::new());
    if register {
      let checked = validate_username(&name).and_then(|n| validate_password(&pass, &n).map(|()| n));
      if let Err(e) = checked {
        let msg = account_sync::error_message(ErrorCode::from_credential(e));
        match ErrorCode::from_credential(e) {
          ErrorCode::InvalidUsername => name_error.set(msg),
          _ => pass_error.set(msg),
        }
        return;
      }
    } else if name.trim().is_empty() || pass.is_empty() {
      // 空着就点登录：服务器只会回「账号名或口令不对」，没必要为它发一次请求。
      pass_error.set(account_sync::error_message(ErrorCode::BadCredentials));
      return;
    }
    working.set(true);
    let alive = alive.get_value();
    spawn_local(async move {
      let result = if register {
        account_sync::register(&name, &pass).await
      } else {
        account_sync::login(&name, &pass).await
      };
      // 登录 / 注册一成功，登录态就变了，外壳随即把本表单卸载（`alive` 为假）——
      // 但「登录成功」是服务端已发生的事实：自动同步要知道、欢迎提示也要弹（Toast 是全局的）。
      // 只有本组件自己的信号才需要先看还在不在。
      let failure = match result {
        Ok(user) => {
          crate::auto_sync::account_signed_in();
          crate::ui::push_success(crate::i18n::tf("tools.account-welcome", &[&user.username]));
          None
        }
        Err(msg) => Some(msg),
      };
      if alive() {
        working.set(false);
        match failure {
          Some(msg) => pass_error.set(msg),
          None => password.set(String::new()),
        }
      }
    });
  };

  view! {
    <div class="space-y-3">
      <Field
        label=Signal::derive(move || t("tools.account-username"))
        r#for="account-username"
        error=name_error
      >
        <Input
          id="account-username"
          value=username
          on_change=Callback::new(move |v: String| {
            username.set(v);
            name_error.set(String::new());
          })
          autocomplete="username"
          autocapitalize="off"
          spellcheck="false"
          invalid=Signal::derive(move || !name_error.get().is_empty())
        />
      </Field>
      <Field
        label=Signal::derive(move || t("tools.account-password"))
        r#for="account-password"
        hint=Signal::derive(move || t("tools.account-rules-hint"))
        error=pass_error
      >
        <Input
          id="account-password"
          kind=InputType::Password
          value=password
          on_change=Callback::new(move |v: String| {
            password.set(v);
            pass_error.set(String::new());
          })
          on_enter=Callback::new(move |()| submit(false))
          autocomplete="current-password"
          invalid=Signal::derive(move || !pass_error.get().is_empty())
        />
      </Field>
      <div class="flex flex-wrap items-center gap-2">
        <Button
          variant=Variant::Default
          class=crate::kit::btn_wrap_class()
          loading=working
          on_click=Callback::new(move |()| submit(false))
        >
          {move || t("tools.account-login")}
        </Button>
        {registration_open
          .then(|| {
            view! {
              <Button
                variant=Variant::Outline
                class=crate::kit::btn_wrap_class()
                disabled=working
                on_click=Callback::new(move |()| submit(true))
              >
                {move || t("tools.account-register")}
              </Button>
            }
          })}
      </div>
      {(!registration_open)
        .then(|| view! { <p class="text-xs text-muted-foreground">{move || t("tools.account-registration-closed")}</p> })}
    </div>
  }
}
