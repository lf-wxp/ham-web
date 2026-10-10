//! 通知中心：集中管理浏览器通知权限与三类提醒（倒计时 / DX 热点 / 卫星过境）开关。

use ham_web_core::dx_watch::AlertSettings;
use ham_web_core::sat_watch::SatWatch;
use leptos::prelude::*;
use web_sys::{Notification, NotificationPermission};

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::{t, tf, tp};
use crate::ui::{Button, Size, Variant};
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
  set_title("settings.notifications");

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
    NotificationPermission::Granted => t("settings.granted"),
    NotificationPermission::Denied => t("settings.denied"),
    _ => t("settings.not-requested"),
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
      <PageHeader
        title=move || t("settings.notifications")
        subtitle=move || t("settings.manage-browser-notification-permission")
      />

      <PageContainer class="space-y-4">
        <section class="rounded-xl border bg-card">
          <div class="flex flex-wrap items-center gap-2 border-b px-4 py-3">
            <h2 class="mr-auto text-sm font-semibold">{move || t("settings.browser-notification-permission")}</h2>
            <span class="text-xs text-muted-foreground">{perm_label}</span>
            {move || {
              (!matches!(permission.get(), NotificationPermission::Granted)).then(|| {
                view! {
                  <Button
                    variant=Variant::Default
                    size=Size::Sm
                    on_click=Callback::new(move |_| request(()))
                  >
                    {move || t("settings.request-permission")}
                  </Button>
                }
              })
            }}
          </div>
          <p class="px-4 py-3 text-xs text-muted-foreground">
            {move || t("settings.alerts-only-fire-while")}
          </p>
        </section>

        <section class="rounded-xl border bg-card">
          <div class="flex flex-wrap items-center gap-2 border-b px-4 py-3">
            <h2 class="mr-auto text-sm font-semibold">{move || t("settings.countdown-reminders")}</h2>
            <span class="text-xs text-muted-foreground">
              {tp("settings.countdowns", count.get() as u32, &[&count.get().to_string()])}
            </span>
          </div>
          <div class="px-4 py-3 text-xs text-muted-foreground">
            {move || t("settings.add-targets-like-exam")}
            <a href="/countdown" class="ml-1 text-primary underline underline-offset-4 hover:underline">
              {move || t("settings.manage-countdowns")}
            </a>
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <div class="flex flex-wrap items-center gap-2 border-b px-4 py-3">
            <h2 class="mr-auto text-sm font-semibold">{move || t("tools.dx-spot-alerts")}</h2>
          </div>
          <div class="space-y-3 px-4 py-3">
            <div class="flex items-center justify-between gap-2">
              <span class="text-sm">{move || t("settings.new-dxcc-entity")}</span>
              <button type="button" class=toggle_class(dx.get_untracked().new_dxcc) on:click=toggle_dxcc>
                {move || if dx.get().new_dxcc { t("settings.on") } else { t("settings.off") }}
              </button>
            </div>
            <div class="flex items-center justify-between gap-2">
              <span class="text-sm">{move || t("settings.new-band-on-worked")}</span>
              <button type="button" class=toggle_class(dx.get_untracked().new_band) on:click=toggle_band>
                {move || if dx.get().new_band { t("settings.on") } else { t("settings.off") }}
              </button>
            </div>
            <div class="text-xs text-muted-foreground">
              {tp(
                "settings.watched-callsigns",
                dx.get_untracked().calls.len() as u32,
                &[&dx.get_untracked().calls.len().to_string()],
              )}
              <a href="/dx-spots" class="ml-1 text-primary underline underline-offset-4 hover:underline">
                {move || t("settings.configure-on-dx-spots")}
              </a>
            </div>
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <div class="flex flex-wrap items-center gap-2 border-b px-4 py-3">
            <h2 class="mr-auto text-sm font-semibold">{move || t("settings.satellite-pass-alerts")}</h2>
          </div>
          <div class="space-y-3 px-4 py-3">
            <div class="flex items-center justify-between gap-2">
              <span class="text-sm">{move || t("settings.favorite-satellite-pass-alerts")}</span>
              <button type="button" class=toggle_class(sat.get_untracked().alerts) on:click=toggle_sat>
                {move || if sat.get().alerts { t("settings.on") } else { t("settings.off") }}
              </button>
            </div>
            <div class="flex items-center justify-between gap-2">
              <span class="text-sm">{move || t("settings.apt-weather-satellite-recording")}</span>
              <button type="button" class=toggle_class(sat.get_untracked().apt_alert) on:click=toggle_apt>
                {move || if sat.get().apt_alert { t("settings.on") } else { t("settings.off") }}
              </button>
            </div>
            <div class="text-xs text-muted-foreground">
              {move || t("settings.favorite-satellites-and-lead")}
              <a href="/satellites" class="ml-1 text-primary underline underline-offset-4 hover:underline">
                {move || t("settings.configure-on-satellites-page")}
              </a>
            </div>
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <div class="flex flex-wrap items-center gap-2 border-b px-4 py-3">
            <h2 class="mr-auto text-sm font-semibold">{move || t("settings.background-push")}</h2>
            <span class="text-xs text-muted-foreground">
              {move || if subscribed.get() { t("settings.subscribed") } else { t("settings.not-subscribed") }}
            </span>
          </div>
          <div class="space-y-3 px-4 py-3">
            <p class="text-xs text-muted-foreground">
              {move || t("settings.receive-daily-study-reminders")}
            </p>
            {if !supported {
              view! {
                <p class="text-xs text-muted-foreground">{move || t("settings.this-browser-doesn-t")}</p>
              }
              .into_any()
            } else {
              view! {
                <div class="flex flex-wrap items-center gap-2">
                  <Button
                    variant=Variant::Default
                    size=Size::Sm
                    disabled=Signal::derive(move || subscribed.get())
                    on_click=Callback::new(move |_| {
                                        leptos::task::spawn_local(async move {
                                          match crate::push::fetch_public_key().await {
                                            None => status.set(Some(t("settings.push-service-not-configured"))),
                                            Some(key) => match crate::push::subscribe(&key).await {
                                              Ok(_) => {
                                                subscribed.set(true);
                                                status.set(None);
                                              }
                                              Err(e) => status.set(Some(tf("settings.subscribe-failed", &[&e]))),
                                            },
                                          }
                                        });
                                      })
                  >
                    {move || t("settings.subscribe-to-push")}
                  </Button>
                  <Button
                    variant=Variant::Outline
                    size=Size::Sm
                    disabled=Signal::derive(move || !subscribed.get())
                    on_click=Callback::new(move |_| {
                                        leptos::task::spawn_local(async move {
                                          crate::push::unsubscribe().await;
                                          subscribed.set(false);
                                          status.set(None);
                                        });
                                      })
                  >
                    {move || t("settings.unsubscribe")}
                  </Button>
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
      </PageContainer>
    </div>
  }
}
