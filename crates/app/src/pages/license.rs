//! 执照 / 操作证申办流程指南。

use leptos::prelude::*;

use crate::util::set_title;

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
  ("B 类", "30 MHz 以下 <15W 或 30 MHz 以上 ≤25W。"),
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
  set_title("执照申办");
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">"执照 / 操作证申办"</h1>
            <div class="text-xs text-muted-foreground">"报名 → 考试 → 设台 → 领证"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"申办流程"</h2>
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
                      <div class="font-medium">{k}</div>
                      <div class="mt-0.5 text-sm text-muted-foreground">{v}</div>
                    </div>
                  </li>
                }
              })
              .collect_view()}
          </ol>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"操作能力类别与权限"</h2>
          <dl class="divide-y">
            {CLASSES
              .iter()
              .map(|&(k, v)| {
                view! {
                  <div class="flex gap-3 px-4 py-3">
                    <dt class="w-14 shrink-0 font-mono font-semibold text-primary">{k}</dt>
                    <dd class="text-sm text-muted-foreground">{v}</dd>
                  </div>
                }
              })
              .collect_view()}
          </dl>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"关键须知"</h2>
          <ul class="space-y-2 p-4">
            {NOTES
              .iter()
              .map(|note| {
                view! {
                  <li class="flex gap-2 text-sm text-muted-foreground">
                    <span class="mt-0.5 shrink-0 text-primary">"•"</span>
                    <span>{*note}</span>
                  </li>
                }
              })
              .collect_view()}
          </ul>
        </section>
      </div>
    </div>
  }
}
