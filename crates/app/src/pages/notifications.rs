//! 通知中心：集中管理浏览器通知权限与三类提醒（倒计时 / DX 热点 / 卫星过境）开关。

use ham_web_core::dx_watch::AlertSettings;
use ham_web_core::sat_watch::SatWatch;
use leptos::prelude::*;
use web_sys::{Notification, NotificationPermission};

use crate::i18n::{t, tf};
use crate::ui::{Size, Variant, button_class};
use crate::util::{set_title, storage};

const DX_KEY: &str = "dx-alerts";
const SAT_KEY: &str = "sat-watch";

/// 读取倒计时条数（`countdowns` 里的 `events` 长度）。
fn countdown_count() -> usize {
  storage::get_json::<serde_json::Value>("countdowns")
    .and_then(|v| v.get("events").and_then(|e| e.as_array()).map(Vec::len))
    .unwrap_or(0)
}

/// 开关按钮样式。
fn toggle_class(on: bool) -> &'static str {
  if on {
    "rounded-md bg-primary px-2.5 py-1 text-xs font-medium text-primary-foreground"
  } else {
    "rounded-md border px-2.5 py-1 text-xs text-muted-foreground transition-colors hover:bg-accent"
  }
}

/// 通知中心页面。
#[component]
pub fn NotificationsPage() -> impl IntoView {
  set_title(&t("通知中心"));

  let permission = RwSignal::new(Notification::permission());
  let dx = RwSignal::new(storage::get_json::<AlertSettings>(DX_KEY).unwrap_or_default());
  let sat = RwSignal::new(storage::get_json::<SatWatch>(SAT_KEY).unwrap_or_default());
  let count = RwSignal::new(countdown_count());
  let supported = crate::push::is_supported();
  let subscribed = RwSignal::new(crate::push::has_subscription());
  let status = RwSignal::new(None::<String>);

  let request = move |_| {
    leptos::task::spawn_local(async move {
      if let Ok(promise) = Notification::request_permission() {
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
      }
      permission.set(Notification::permission());
    });
  };

  let perm_label = move || match permission.get() {
    NotificationPermission::Granted => t("已授权"),
    NotificationPermission::Denied => t("已拒绝"),
    _ => t("尚未请求"),
  };

  let toggle_dxcc = move |_| {
    dx.update(|s| s.new_dxcc = !s.new_dxcc);
    storage::set_json(DX_KEY, &dx.get_untracked());
  };
  let toggle_band = move |_| {
    dx.update(|s| s.new_band = !s.new_band);
    storage::set_json(DX_KEY, &dx.get_untracked());
  };
  let toggle_sat = move |_| {
    sat.update(|s| s.alerts = !s.alerts);
    storage::set_json(SAT_KEY, &sat.get_untracked());
  };
  let toggle_apt = move |_| {
    sat.update(|s| s.apt_alert = !s.apt_alert);
    storage::set_json(SAT_KEY, &sat.get_untracked());
  };

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-3xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("通知中心")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("集中管理浏览器通知权限与提醒开关")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-3xl space-y-4 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <div class="flex flex-wrap items-center gap-2 border-b px-4 py-3">
            <h2 class="mr-auto text-sm font-semibold">{move || t("浏览器通知权限")}</h2>
            <span class="text-xs text-muted-foreground">{perm_label}</span>
            {move || {
              (!matches!(permission.get(), NotificationPermission::Granted)).then(|| {
                view! {
                  <button
                    type="button"
                    class=button_class(Variant::Default, Size::Sm, "")
                    on:click=request
                  >
                    {move || t("请求权限")}
                  </button>
                }
              })
            }}
          </div>
          <p class="px-4 py-3 text-xs text-muted-foreground">
            {move || t("提醒仅在页面打开期间生效；被系统拒绝后需在浏览器设置里手动开启。")}
          </p>
        </section>

        <section class="rounded-xl border bg-card">
          <div class="flex flex-wrap items-center gap-2 border-b px-4 py-3">
            <h2 class="mr-auto text-sm font-semibold">{move || t("倒计时提醒")}</h2>
            <span class="text-xs text-muted-foreground">
              {tf("{} 个倒计时", &[&count.get().to_string()])}
            </span>
          </div>
          <div class="px-4 py-3 text-xs text-muted-foreground">
            {move || t("在倒计时页添加考试日、执照到期等目标，到期时浏览器会弹出通知。")}
            <a href="/countdown" class="ml-1 text-primary underline underline-offset-4 hover:underline">
              {move || t("管理倒计时 →")}
            </a>
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <div class="flex flex-wrap items-center gap-2 border-b px-4 py-3">
            <h2 class="mr-auto text-sm font-semibold">{move || t("DX 热点提醒")}</h2>
          </div>
          <div class="space-y-3 px-4 py-3">
            <div class="flex items-center justify-between gap-2">
              <span class="text-sm">{move || t("新 DXCC 实体")}</span>
              <button type="button" class=toggle_class(dx.get_untracked().new_dxcc) on:click=toggle_dxcc>
                {move || if dx.get().new_dxcc { t("已开启") } else { t("已关闭") }}
              </button>
            </div>
            <div class="flex items-center justify-between gap-2">
              <span class="text-sm">{move || t("已通联实体的新波段")}</span>
              <button type="button" class=toggle_class(dx.get_untracked().new_band) on:click=toggle_band>
                {move || if dx.get().new_band { t("已开启") } else { t("已关闭") }}
              </button>
            </div>
            <div class="text-xs text-muted-foreground">
              {tf("关注呼号 {} 个", &[&dx.get_untracked().calls.len().to_string()])}
              <a href="/dx-spots" class="ml-1 text-primary underline underline-offset-4 hover:underline">
                {move || t("到 DX 热点页设置 →")}
              </a>
            </div>
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <div class="flex flex-wrap items-center gap-2 border-b px-4 py-3">
            <h2 class="mr-auto text-sm font-semibold">{move || t("卫星过境提醒")}</h2>
          </div>
          <div class="space-y-3 px-4 py-3">
            <div class="flex items-center justify-between gap-2">
              <span class="text-sm">{move || t("收藏卫星过境提醒")}</span>
              <button type="button" class=toggle_class(sat.get_untracked().alerts) on:click=toggle_sat>
                {move || if sat.get().alerts { t("已开启") } else { t("已关闭") }}
              </button>
            </div>
            <div class="flex items-center justify-between gap-2">
              <span class="text-sm">{move || t("APT 气象卫星录制提醒")}</span>
              <button type="button" class=toggle_class(sat.get_untracked().apt_alert) on:click=toggle_apt>
                {move || if sat.get().apt_alert { t("已开启") } else { t("已关闭") }}
              </button>
            </div>
            <div class="text-xs text-muted-foreground">
              {move || t("收藏卫星与提前量在卫星页设置。")}
              <a href="/satellites" class="ml-1 text-primary underline underline-offset-4 hover:underline">
                {move || t("到卫星页设置 →")}
              </a>
            </div>
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <div class="flex flex-wrap items-center gap-2 border-b px-4 py-3">
            <h2 class="mr-auto text-sm font-semibold">{move || t("后台推送")}</h2>
            <span class="text-xs text-muted-foreground">
              {move || if subscribed.get() { t("已订阅") } else { t("未订阅") }}
            </span>
          </div>
          <div class="space-y-3 px-4 py-3">
            <p class="text-xs text-muted-foreground">
              {move || t("订阅后即使页面关闭也能收到每日学习提醒（由后端推送服务定时发送）。")}
            </p>
            {if !supported {
              view! {
                <p class="text-xs text-muted-foreground">{move || t("当前浏览器不支持 Web Push。")}</p>
              }
              .into_any()
            } else {
              view! {
                <div class="flex flex-wrap items-center gap-2">
                  <button
                    type="button"
                    class=button_class(Variant::Default, Size::Sm, "")
                    disabled=move || subscribed.get()
                    on:click=move |_| {
                      leptos::task::spawn_local(async move {
                        match crate::push::fetch_public_key().await {
                          None => status.set(Some(t("未配置推送服务"))),
                          Some(key) => match crate::push::subscribe(&key).await {
                            Ok(_) => {
                              subscribed.set(true);
                              status.set(None);
                            }
                            Err(e) => status.set(Some(tf("订阅失败：{}", &[&e]))),
                          },
                        }
                      });
                    }
                  >
                    {move || t("订阅后台推送")}
                  </button>
                  <button
                    type="button"
                    class=button_class(Variant::Outline, Size::Sm, "")
                    disabled=move || !subscribed.get()
                    on:click=move |_| {
                      leptos::task::spawn_local(async move {
                        crate::push::unsubscribe().await;
                        subscribed.set(false);
                        status.set(None);
                      });
                    }
                  >
                    {move || t("退订")}
                  </button>
                </div>
              }
              .into_any()
            }}
            {move || {
              status.get().map(|s| {
                view! {
                  <p class="text-xs font-medium text-red-700 dark:text-red-400">{s}</p>
                }
              })
            }}
          </div>
        </section>
      </div>
    </div>
  }
}
