//! 数字模式时隙钟：FT8 / FT4 / WSPR 的当前时隙、离下一个时隙还有多久（UTC 对齐）。
//!
//! 时间计算全在 `ham_web_core::slot_clock`（有单测）；本组件只负责每 100 ms 取一次本机时钟并画出来。
//! 用 100 ms 而不是 1 s：用户要的是「在时隙起点按下播放」，剩余时间显示到 0.1 秒才对得上。
//! 倒计时用 `role="timer"`（隐含 `aria-live="off"`）：每 0.1 秒变一次的数字不该被读屏播报。
//!
//! 时间取自浏览器（本机系统时钟），不知道它准不准；这些模式都要求时钟误差在 1 秒左右以内，
//! 所以组件里写明让用户先对时。

use std::time::Duration;

use ham_web_core::slot_clock::{self, SlotSpec};
use leptos::prelude::*;

use crate::i18n::{t, tf};

/// 当前时间（自 Unix 纪元的毫秒）。
fn now_ms() -> f64 {
  js_sys::Date::now()
}

/// 模式名（含周期）：词条按模式键取。
fn mode_label(key: &str) -> String {
  t(match key {
    "ft8" => "tools.slot-clock-ft8",
    "ft4" => "tools.slot-clock-ft4",
    _ => "tools.slot-clock-wspr",
  })
}

/// 「剩余 X.X 秒」里的数字：以 0.1 秒为单位，整数运算拼出一位小数。
fn tenths_text(tenths: u64) -> String {
  format!("{}.{}", tenths / 10, tenths % 10)
}

/// 时隙钟：`specs` 里每个模式一行。
#[component]
pub fn SlotClock(specs: Vec<SlotSpec>) -> impl IntoView {
  let now = RwSignal::new(now_ms());
  if let Ok(handle) =
    set_interval_with_handle(move || now.set(now_ms()), Duration::from_millis(100))
  {
    on_cleanup(move || handle.clear());
  }

  let utc = move || {
    slot_clock::utc_clock(now.get())
      .map_or_else(String::new, |(h, m, s)| format!("{h:02}:{m:02}:{s:02} UTC"))
  };

  let rows = specs
    .into_iter()
    .map(|spec| {
      let position = move || slot_clock::position(&spec, now.get());
      view! {
        <div class="space-y-1" data-slot=format!("slot-{}", spec.key)>
          <div class="flex flex-wrap items-baseline justify-between gap-x-3 gap-y-1">
            <span class="text-sm">{mode_label(spec.key)}</span>
            <span class="tabular-nums text-sm" role="timer">
              {move || {
                position()
                  .map(|p| tf("tools.slot-clock-remaining", &[&tenths_text(p.remaining_tenths())]))
              }}
            </span>
          </div>
          <div class="flex flex-wrap items-baseline justify-between gap-x-3 text-xs text-muted-foreground">
            <span class="tabular-nums">
              {move || {
                position()
                  .map(|p| {
                    let (h, m, s, tenth) = p.start_clock();
                    // 起点只有 FT4 会落在半秒上：整秒就不写小数，免得 FT8 / WSPR 满屏 `.0`。
                    let at = if tenth == 0 {
                      format!("{h:02}:{m:02}:{s:02}")
                    } else {
                      format!("{h:02}:{m:02}:{s:02}.{tenth}")
                    };
                    tf("tools.slot-clock-slot-start", &[&at])
                  })
              }}
            </span>
            // WSPR 的 120 秒周期没有「奇偶」的通联约定，不显示。
            {(spec.key != "wspr")
              .then(|| {
                view! {
                  <span>
                    {move || {
                      position()
                        .map(|p| {
                          t(
                            if p.is_even() {
                              "tools.slot-clock-even"
                            } else {
                              "tools.slot-clock-odd"
                            },
                          )
                        })
                    }}
                  </span>
                }
              })}
          </div>
          <div class="h-2 w-full overflow-hidden bg-muted">
            <div
              class="h-full bg-primary"
              style=move || {
                format!("width: {:.1}%", position().map_or(0.0, |p| p.progress() * 100.0))
              }
            ></div>
          </div>
          {(spec.tx_offset_ms > 0)
            .then(|| {
              view! {
                <p class="text-xs text-muted-foreground">
                  {move || {
                    tf(
                      "tools.slot-clock-tx-offset",
                      &[&(spec.tx_offset_ms / 1000).to_string()],
                    )
                  }}
                </p>
              }
            })}
        </div>
      }
    })
    .collect_view();

  view! {
    <div class="space-y-3" data-slot="slot-clock">
      <div class="flex flex-wrap items-baseline justify-between gap-x-3">
        <span class="text-xs text-foreground/70">{move || t("tools.slot-clock-heading")}</span>
        <span class="tabular-nums text-sm" role="timer" data-slot="slot-utc">
          {utc}
        </span>
      </div>
      {rows}
      <p class="text-xs text-muted-foreground">{move || t("tools.slot-clock-note")}</p>
    </div>
  }
}
