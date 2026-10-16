//! 多普勒联动的电台一侧：通过 Web Serial 连接电台，**可选地**把每秒算出的下行修正频率写进去。
//!
//! 安全约定（每一条都有对应的单测或 e2e）：
//! - 连接后只**读**；写入必须用户手动勾选「自动调谐」，重新连接、断开、手动改频都会把它关掉；
//! - 只写一个频率（接收修正频率），不改模式、不碰 VFO、不写发射频率；
//! - 读到电台自己回报的 `FA` 位数之前不写（写错位数电台会忽略，见 `ham_web_core::cat`）；
//! - 只在卫星高于地平线时写；写入有节流（`MIN_WRITE_DELTA_HZ`），避免静噪反复闪；
//! - 发现电台回读的频率连续几次偏离我们写的值，就当作有人转了旋钮，立刻停写。
//!
//! 判定逻辑（取整、节流、手动改频检测）都在 `ham_web_core::sat_doppler`，这里只负责串口读写与界面。

// `Update` 改名：不然它会压住 prelude 里的 `Update` trait，`RwSignal::update` 就找不到了。
use ham_web_core::cat::{
  BAUD_RATES, Parser, Protocol, Update as CatUpdate, format_mhz, poll_commands,
  set_frequency_command,
};
use ham_web_core::sat_doppler::{OverrideDetector, round_for_rig, should_write};
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;

use crate::components::cat_control::{load_settings, save_settings};
use crate::i18n::{t, tf, tp};
use crate::ui::{
  Button, Checkbox, ControlSize, Input, Label, NativeSelect, SelectOption, Size, Variant,
};
use crate::util::{js_error_message, sleep, unique_id};
use crate::web_serial::{
  Conn, call, close_serial, is_user_cancel, open_serial, prop, serial, write_bytes,
};

/// 轮询 / 写入间隔（毫秒）。
const POLL_MS: u32 = 1000;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Status {
  Idle,
  Connecting,
  Connected,
  Error(String),
}

/// 电台联动条。`target` 是此刻该写进电台的下行修正频率（Hz）；卫星在地平线下时为 `None`。
#[component]
pub(super) fn DopplerRig(
  #[prop(into)] target: Signal<Option<f64>>,
  #[prop(into)] selection: Signal<u32>,
) -> impl IntoView {
  let supported = serial().is_some();
  let settings = RwSignal::new(load_settings());
  let status = RwSignal::new(Status::Idle);
  let rig_hz = RwSignal::new(None::<u64>);
  let digits = RwSignal::new(None::<usize>);
  let auto = RwSignal::new(false);
  let paused = RwSignal::new(false);
  let writes = RwSignal::new(0u32);
  let last_hz = RwSignal::new(None::<u64>);
  let conn = StoredValue::new_local(None::<Conn>);
  let generation = StoredValue::new(0u32);
  let detector = StoredValue::new(OverrideDetector::default());

  Effect::new(move |_| {
    let _ = selection.get();
    auto.set(false);
    paused.set(false);
    last_hz.set(None);
    detector.update_value(OverrideDetector::reset);
  });

  let auto_id = unique_id("doppler-auto-tune");

  let save = move || settings.with_untracked(save_settings);

  // 只动 `StoredValue`、不碰信号：它还会被 `on_cleanup` 调用，那时信号可能已随组件释放。
  let teardown = move || {
    generation.try_update_value(|g| *g = g.wrapping_add(1));
    if let Some(c) = conn.try_update_value(Option::take).flatten() {
      spawn_local(close_serial(c));
    }
  };
  let reset_ui = move || {
    auto.set(false);
    paused.set(false);
    rig_hz.set(None);
    digits.set(None);
    last_hz.set(None);
    writes.set(0);
    detector.update_value(OverrideDetector::reset);
  };
  let disconnect = move || {
    teardown();
    reset_ui();
    status.set(Status::Idle);
  };
  let fail = move |my: u32, msg: String| {
    if generation.try_get_value() == Some(my) {
      teardown();
      reset_ui();
      status.set(Status::Error(msg));
    }
  };

  let connect = move || {
    let s = settings.get_untracked();
    generation.update_value(|g| *g = g.wrapping_add(1));
    let my = generation.get_value();
    reset_ui();
    status.set(Status::Connecting);
    spawn_local(async move {
      let outcome = open_serial(s.baud).await;
      // 端口选择框可能开着很久：期间用户已断开或离开页面，就别再写信号，端口直接关掉。
      if generation.try_get_value() != Some(my) {
        if let Ok(c) = outcome {
          spawn_local(close_serial(c));
        }
        return;
      }
      let c = match outcome {
        Ok(c) => c,
        Err(e) => {
          status.set(if is_user_cancel(&e) {
            Status::Idle
          } else {
            Status::Error(js_error_message(&e))
          });
          return;
        }
      };
      let (reader, writer) = (c.reader.clone(), c.writer.clone());
      conn.set_value(Some(c));
      status.set(Status::Connected);
      let alive = move || generation.try_get_value() == Some(my);

      // 读循环：解析电台回报的频率，顺带学到它的 FA 位数，并检测手动改频。
      spawn_local(async move {
        let mut parser = Parser::new(s.protocol);
        while alive() {
          let chunk = match call(&reader, "read", &[]).await {
            Ok(r) => r,
            Err(e) => {
              if alive() {
                fail(my, js_error_message(&e));
              }
              return;
            }
          };
          if !alive() {
            return;
          }
          if prop(&chunk, "done").ok().is_some_and(|d| d.is_truthy()) {
            fail(my, t("radio.doppler-rig-lost"));
            return;
          }
          let Ok(v) = prop(&chunk, "value").and_then(JsCast::dyn_into::<js_sys::Uint8Array>) else {
            continue;
          };
          let updates = parser.feed(&v.to_vec());
          digits.set(parser.freq_digits());
          for u in updates {
            let CatUpdate::Freq(hz) = u else { continue };
            rig_hz.set(Some(hz));
            let last = last_hz.get_untracked();
            let moved = detector
              .try_update_value(|d| d.observe(last, hz))
              .unwrap_or(false);
            if moved && auto.get_untracked() {
              auto.set(false);
              paused.set(true);
            }
          }
        }
      });

      // 写循环：每秒查一次电台频率；勾选了自动调谐且目标有变化才写。
      let commands = poll_commands(s.protocol, s.civ_addr);
      while alive() {
        for cmd in &commands {
          if let Err(e) = write_bytes(&writer, cmd).await {
            return fail(my, js_error_message(&e));
          }
          if !alive() {
            return;
          }
          sleep(60).await;
          if !alive() {
            return;
          }
        }
        if auto.get_untracked()
          && detector.with_value(OverrideDetector::write_allowed)
          && let Some(hz) = target.get_untracked().and_then(round_for_rig)
          && should_write(last_hz.get_untracked(), hz)
          && let Some(cmd) =
            set_frequency_command(s.protocol, s.civ_addr, hz, digits.get_untracked())
        {
          if let Err(e) = write_bytes(&writer, &cmd).await {
            return fail(my, js_error_message(&e));
          }
          if !alive() {
            return;
          }
          last_hz.set(Some(hz));
          writes.update(|n| *n = n.saturating_add(1));
        }
        sleep(POLL_MS).await;
      }
    });
  };

  on_cleanup(teardown);

  let busy = move || matches!(status.get(), Status::Connected | Status::Connecting);
  let connected = move || matches!(status.get(), Status::Connected);

  // 标签走词条：`Protocol::label()` 是中文常量，直接用会在英文 / 西语界面里漏出中文。
  let protocol_options: Vec<SelectOption> = vec![
    SelectOption::new(
      "kenwood",
      Signal::derive(move || t("common.cat-protocol-kenwood")),
    ),
    SelectOption::new("icom", "Icom CI-V"),
  ];
  let baud_options: Vec<SelectOption> = BAUD_RATES
    .into_iter()
    .map(|b| SelectOption::new(b.to_string(), format!("{b} bps")))
    .collect();

  view! {
    <div class="space-y-2 border border-dashed px-3 py-2 text-xs text-muted-foreground">
      <div class="flex flex-wrap items-center gap-2">
        <span class="font-medium text-foreground">{move || t("radio.doppler-rig-title")}</span>
        {if supported {
          view! {
            <NativeSelect
              value=Signal::derive(move || {
                if settings.with(|s| s.protocol) == Protocol::Icom {
                  "icom".to_owned()
                } else {
                  "kenwood".to_owned()
                }
              })
              on_change=Callback::new(move |v: String| {
                settings
                  .update(|s| {
                    s.protocol = if v == "icom" { Protocol::Icom } else { Protocol::Kenwood };
                  });
                save();
              })
              options=protocol_options
              disabled=Signal::derive(busy)
              size=ControlSize::Sm
              aria_label=Signal::derive(move || t("common.cat-protocol"))
              class="w-auto"
            />
            <NativeSelect
              value=Signal::derive(move || settings.with(|s| s.baud).to_string())
              on_change=Callback::new(move |v: String| {
                settings.update(|s| s.baud = v.parse().unwrap_or(9600));
                save();
              })
              options=baud_options
              disabled=Signal::derive(busy)
              size=ControlSize::Sm
              aria_label=Signal::derive(move || t("common.baud-rate"))
              class="w-auto"
            />
            <Show when=move || settings.with(|s| s.protocol == Protocol::Icom)>
              <Input
                aria_label=Signal::derive(move || t("common.ci-v-address-hex"))
                title=Signal::derive(move || t("common.ci-v-address-hex-examples"))
                class="h-7 w-14 py-0 font-mono text-xs uppercase"
                maxlength=Signal::derive(|| "2".to_owned())
                disabled=Signal::derive(busy)
                value=Signal::derive(move || format!("{:02X}", settings.with(|s| s.civ_addr)))
                on_change=Callback::new(move |v: String| {
                  if let Ok(v) = u8::from_str_radix(v.trim(), 16) {
                    settings.update(|s| s.civ_addr = v);
                    save();
                  }
                })
              />
            </Show>
            <Button
              variant=Variant::Outline
              size=Size::Sm
              class="h-7"
              disabled=Signal::derive(move || matches!(status.get(), Status::Connecting))
              on_click=Callback::new(move |_| if busy() { disconnect() } else { connect() })
            >
              {move || match status.get() {
                Status::Connecting => t("common.connecting"),
                Status::Connected => t("common.disconnect"),
                Status::Idle | Status::Error(_) => t("common.connect-rig"),
              }}
            </Button>
          }
            .into_any()
        } else {
          view! { <span>{move || t("radio.doppler-rig-unsupported")}</span> }.into_any()
        }}
      </div>

      {move || match status.get() {
        Status::Error(msg) => {
          Some(view! {
            <p class="text-destructive" role="alert">
              {tf("common.connection-failed", &[&msg])}
            </p>
          })
        }
        _ => None,
      }}

      <Show when=connected>
        <div class="flex items-center gap-2">
          <Checkbox
            id=auto_id.clone()
            checked=auto
            on_change=Callback::new(move |on: bool| {
              auto.set(on);
              if on {
                // 重新开启：忘掉上次的写入记录，下一秒就按当前目标写一次。
                paused.set(false);
                last_hz.set(None);
                detector.update_value(OverrideDetector::reset);
              }
            })
          />
          <Label r#for=auto_id.clone()>{move || t("radio.doppler-rig-auto")}</Label>
        </div>
        <div class="space-y-0.5 tabular-nums" aria-live="polite">
          <div>
            {move || match rig_hz.get() {
              Some(hz) => tf("radio.doppler-rig-current", &[&format_mhz(hz)]),
              None => t("radio.doppler-rig-waiting"),
            }}
          </div>
          {move || {
            let n = writes.get();
            last_hz
              .get()
              .map(|hz| {
                view! {
                  <div>
                    {tp("radio.doppler-rig-written", n, &[&n.to_string(), &format_mhz(hz)])}
                  </div>
                }
              })
          }}
          {move || {
            let waiting = auto.get()
              && digits.get().is_none()
              && settings.with(|s| s.protocol) == Protocol::Kenwood;
            waiting.then(|| view! { <div>{move || t("radio.doppler-rig-waiting-digits")}</div> })
          }}
          {move || {
            paused
              .get()
              .then(|| {
                view! {
                  <div class="text-destructive" role="alert">
                    {move || t("radio.doppler-rig-paused")}
                  </div>
                }
              })
          }}
        </div>
      </Show>
      <p>{move || t("radio.doppler-rig-note")}</p>
    </div>
  }
}
