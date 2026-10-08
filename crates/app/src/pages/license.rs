//! 执照 / 操作证申办流程指南。

use leptos::prelude::*;
use wasm_bindgen::JsValue;

use super::countdown::add_countdown;
use crate::i18n::t;
use crate::ui::{Button, DatePicker, Size, Variant};
use crate::util::set_title;

/// 由发照日期推算到期日期（有效期 5 年，按公历加 5 年）。返回毫秒时间戳。
fn expiry_from_issue(issue: &str) -> Option<i64> {
  let ms = js_sys::Date::parse(issue);
  if ms.is_nan() {
    return None;
  }
  let d = js_sys::Date::new(&JsValue::from_f64(ms));
  d.set_full_year(d.get_full_year() + 5);
  Some(d.get_time() as i64)
}

/// 申办步骤。
const STEPS: &[(&str, &str)] = &[
  (
    "报名验证",
    "通过当地无线电管理机构或其指定平台报名操作技术能力验证（A / B / C 类）。",
  ),
  (
    "参加考试",
    "参加理论考试，合格后取得《业余无线电台操作证书》（对应 A / B / C 类）。",
  ),
  (
    "申请设台",
    "凭操作证书提交设台申请：个人需申请表、身份证明复印件、设备说明材料等。",
  ),
  (
    "核发执照与呼号",
    "批准后颁发《业余无线电台执照》并同时核发呼号，即可依法使用。",
  ),
];

/// 操作能力类别与权限。
const CLASSES: &[(&str, &str)] = &[
  ("A 类", "30–3000 MHz，发射功率 ≤ 25W。"),
  ("B 类", "30 MHz 以下 ≤15W 或 30 MHz 以上 ≤25W。"),
  ("C 类", "30 MHz 以下 ≤1000W 或 30 MHz 以上 ≤25W。"),
];

/// 关键须知。
const NOTES: &[&str] = &[
  "执照有效期不超过 5 年，届满 30 个工作日前申请更换。",
  "取得操作证书前，可在他人现场监督指导下实习操作。",
  "呼号停止使用应办理注销，注销 1 年后可重新投入分配。",
  "通信建立及结束时应发送呼号，过程中间隔不超过 10 分钟。",
  "设台须满足三个条件：熟悉无线电管理规定、通过操作技术能力验证、使用符合规定的设备。",
];

#[component]
pub fn LicensePage() -> impl IntoView {
  set_title("shell.licensing");
  let issue = RwSignal::new(String::new());
  let added = RwSignal::new(false);
  let expiry_label = move || {
    expiry_from_issue(&issue.get()).map(|ms| {
      let d = js_sys::Date::new(&JsValue::from_f64(ms as f64));
      format!(
        "{:04}-{:02}-{:02}",
        d.get_full_year(),
        d.get_month() + 1,
        d.get_date()
      )
    })
  };
  let add_reminder = move || {
    let Some(ms) = expiry_from_issue(&issue.get_untracked()) else {
      return;
    };
    add_countdown(&t("settings.license-expires"), ms);
    added.set(true);
  };
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("knowledge.licence-operator-permit-application")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("knowledge.register-exam-set-up")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.application-process")}</h2>
          <ol class="divide-y">
            {STEPS
              .iter()
              .enumerate()
              .map(|(i, &(k, v))| {
                view! {
                  <li class="flex gap-3 px-4 py-3">
                    <span class="flex size-6 shrink-0 items-center justify-center rounded-full bg-primary text-xs font-semibold text-primary-foreground">
                      {i + 1}
                    </span>
                    <div>
                      <div class="font-medium">{move || t(k)}</div>
                      <div class="mt-0.5 text-sm text-muted-foreground">{move || t(v)}</div>
                    </div>
                  </li>
                }
              })
              .collect_view()}
          </ol>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.operating-classes-and-privileges")}</h2>
          <dl class="divide-y">
            {CLASSES
              .iter()
              .map(|&(k, v)| {
                view! {
                  <div class="flex gap-3 px-4 py-3">
                    <dt class="w-14 shrink-0 font-mono font-semibold text-primary">{move || t(k)}</dt>
                    <dd class="text-sm text-muted-foreground">{move || t(v)}</dd>
                  </div>
                }
              })
              .collect_view()}
          </dl>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.key-notes")}</h2>
          <ul class="space-y-2 p-4">
            {NOTES
              .iter()
              .map(|note| {
                view! {
                  <li class="flex gap-2 text-sm text-muted-foreground">
                    <span class="mt-0.5 shrink-0 text-primary">"•"</span>
                    <span>{move || t(note)}</span>
                  </li>
                }
              })
              .collect_view()}
          </ul>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("settings.license-expiry-reminder")}</h2>
          <div class="space-y-3 p-4">
            <p class="text-xs text-muted-foreground">
              {move || t("settings.enter-the-issue-date")}
            </p>
            <div class="grid gap-3 sm:grid-cols-[auto_1fr_auto] sm:items-end">
              <label class="flex flex-col gap-1.5 text-sm">
                <span class="text-xs text-muted-foreground">{move || t("settings.issue-date")}</span>
                <DatePicker
                  value=issue
                  on_change=Callback::new(move |v: String| {
                    issue.set(v);
                    added.set(false);
                  })
                  aria_label=Signal::derive(move || t("settings.issue-date"))
                  class="w-44"
                />
              </label>
              <div class="flex flex-col gap-1.5 text-sm">
                <span class="text-xs text-muted-foreground">{move || t("settings.expiry-date")}</span>
                <span class="font-mono tabular-nums">
                  {move || expiry_label().unwrap_or_else(|| "—".to_owned())}
                </span>
              </div>
              <Button
                variant=Variant::Default
                size=Size::Default
                disabled=Signal::derive(move || issue.get().is_empty())
                on_click=Callback::new(move |_| add_reminder())
              >
                {move || t("settings.add-expiry-reminder")}
              </Button>
            </div>
            {move || {
              added.get().then(|| {
                view! {
                  <p class="text-xs font-medium text-emerald-700 dark:text-emerald-400">
                    {move || t("settings.added-to-countdown-the")}
                    <a href="/countdown" class="ml-1 text-primary underline underline-offset-4 hover:underline">
                      {move || t("settings.view-countdown")}
                    </a>
                  </p>
                }
              })
            }}
          </div>
        </section>
      </div>
    </div>
  }
}
