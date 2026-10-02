//! 电台 CAT 联动：通过 Web Serial 连接电台，定时读取频率与模式并回填到录入表单。
//!
//! Web Serial 仅 Chromium 系桌面浏览器支持；`web-sys` 中该 API 仍属不稳定特性，
//! 这里用 `Reflect` 直接调用，避免引入 `--cfg web_sys_unstable_apis`。

use ham_web_core::cat::{
  BAUD_RATES, DEFAULT_CIV_ADDR, Parser, Protocol, Update as CatUpdate, format_mhz, poll_commands,
};
use js_sys::{Array, Function, Object, Promise, Reflect, Uint8Array};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

use crate::i18n::{t, tf};
use crate::ui::{Size, Variant, button_class, input_class};
use crate::util::{js_error_message, sleep, storage, window};

const KEY: &str = "cat-settings";
/// 轮询间隔（毫秒）。
const POLL_MS: u32 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
  protocol: Protocol,
  baud: u32,
  civ_addr: u8,
}

impl Default for Settings {
  fn default() -> Self {
    Self {
      protocol: Protocol::Kenwood,
      baud: 9600,
      civ_addr: DEFAULT_CIV_ADDR,
    }
  }
}

/// 从电台读到的变化：只在数值变化时回调，避免覆盖用户手动修改。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatReading {
  /// 频率（MHz 字符串，如 `14.074`）。
  pub freq: Option<String>,
  pub mode: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Status {
  Idle,
  Connecting,
  Connected,
  Error(String),
}

/// 已打开的串口及其读写器。
struct Conn {
  port: JsValue,
  reader: JsValue,
  writer: JsValue,
}

fn serial() -> Option<JsValue> {
  Reflect::get(&window().navigator(), &"serial".into())
    .ok()
    .filter(|v| !v.is_undefined())
}

fn prop(obj: &JsValue, key: &str) -> Result<JsValue, JsValue> {
  Reflect::get(obj, &key.into())
}

/// 调用返回 Promise 的方法并等待结果。
async fn call(obj: &JsValue, method: &str, args: &[JsValue]) -> Result<JsValue, JsValue> {
  let f: Function = prop(obj, method)?.dyn_into()?;
  let p: Promise = Reflect::apply(&f, obj, &args.iter().collect::<Array>())?.dyn_into()?;
  JsFuture::from(p).await
}

fn call_sync(obj: &JsValue, method: &str) -> Result<JsValue, JsValue> {
  let f: Function = prop(obj, method)?.dyn_into()?;
  f.call0(obj)
}

async fn open(settings: Settings) -> Result<Conn, JsValue> {
  let serial = serial().ok_or_else(|| JsValue::from_str(&t("当前浏览器不支持 Web Serial")))?;
  let port = call(&serial, "requestPort", &[]).await?;
  let opts = Object::new();
  Reflect::set(&opts, &"baudRate".into(), &settings.baud.into())?;
  call(&port, "open", &[opts.into()]).await?;
  let reader = call_sync(&prop(&port, "readable")?, "getReader")?;
  let writer = call_sync(&prop(&port, "writable")?, "getWriter")?;
  Ok(Conn {
    port,
    reader,
    writer,
  })
}

async fn close(conn: Conn) {
  let _ = call(&conn.reader, "cancel", &[]).await;
  let _ = call_sync(&conn.reader, "releaseLock");
  let _ = call_sync(&conn.writer, "releaseLock");
  let _ = call(&conn.port, "close", &[]).await;
}

/// 电台 CAT 连接条。
#[component]
pub fn CatControl(#[prop(into)] on_reading: Callback<CatReading>) -> impl IntoView {
  let supported = serial().is_some();
  let settings = RwSignal::new(storage::get_json::<Settings>(KEY).unwrap_or_default());
  let status = RwSignal::new(Status::Idle);
  let freq = RwSignal::new(None::<u64>);
  let mode = RwSignal::new(None::<&'static str>);
  let conn = StoredValue::new_local(None::<Conn>);
  let generation = StoredValue::new(0u32);

  let save = move || settings.with_untracked(|s| storage::set_json(KEY, s));

  let disconnect = move || {
    generation.update_value(|g| *g = g.wrapping_add(1));
    if let Some(c) = conn.try_update_value(Option::take).flatten() {
      spawn_local(close(c));
    }
  };

  let apply = move |updates: Vec<CatUpdate>| {
    let mut reading = CatReading {
      freq: None,
      mode: None,
    };
    for u in updates {
      match u {
        CatUpdate::Freq(hz) if freq.get_untracked() != Some(hz) => {
          freq.set(Some(hz));
          reading.freq = Some(format_mhz(hz));
        }
        CatUpdate::Mode(m) if mode.get_untracked() != Some(m) => {
          mode.set(Some(m));
          reading.mode = Some(m);
        }
        _ => {}
      }
    }
    if reading.freq.is_some() || reading.mode.is_some() {
      on_reading.run(reading);
    }
  };

  let fail = move |my: u32, err: &JsValue| {
    if generation.try_get_value() == Some(my) {
      disconnect();
      status.set(Status::Error(js_error_message(err)));
    }
  };

  let connect = move || {
    let s = settings.get_untracked();
    generation.update_value(|g| *g = g.wrapping_add(1));
    let my = generation.get_value();
    status.set(Status::Connecting);
    freq.set(None);
    mode.set(None);
    spawn_local(async move {
      let c = match open(s).await {
        Ok(c) => c,
        Err(e) => {
          // 用户在选择框里点了取消
          let canceled = prop(&e, "name")
            .ok()
            .and_then(|n| n.as_string())
            .is_some_and(|n| n == "NotFoundError");
          status.set(if canceled {
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

      spawn_local(async move {
        let mut parser = Parser::new(s.protocol);
        while alive() {
          let chunk = match call(&reader, "read", &[]).await {
            Ok(r) => r,
            Err(e) => return fail(my, &e),
          };
          if prop(&chunk, "done").ok().is_some_and(|d| d.is_truthy()) {
            break;
          }
          if let Ok(v) = prop(&chunk, "value").and_then(JsCast::dyn_into::<Uint8Array>) {
            apply(parser.feed(&v.to_vec()));
          }
        }
      });

      let commands = poll_commands(s.protocol, s.civ_addr);
      while alive() {
        for cmd in &commands {
          let bytes = Uint8Array::from(cmd.as_slice());
          if let Err(e) = call(&writer, "write", &[bytes.into()]).await {
            return fail(my, &e);
          }
          sleep(60).await;
        }
        sleep(POLL_MS).await;
      }
    });
  };

  on_cleanup(disconnect);

  let connected = move || matches!(status.get(), Status::Connected);
  let busy = move || matches!(status.get(), Status::Connected | Status::Connecting);

  view! {
    <div class="flex flex-wrap items-center gap-2 rounded-lg border border-dashed px-3 py-2 text-xs text-muted-foreground">
      <span class="font-medium text-foreground">{move || t("电台 CAT")}</span>
      {if supported {
        view! {
          <select
            aria-label=move || t("CAT 协议")
            class=input_class("h-7 w-auto py-0 text-xs")
            prop:disabled=busy
            on:change=move |e| {
              let v = event_target_value(&e);
              settings.update(|s| s.protocol = if v == "icom" { Protocol::Icom } else { Protocol::Kenwood });
              save();
            }
          >
            {Protocol::ALL.into_iter().map(|p| view! {
              <option
                value=if p == Protocol::Icom { "icom" } else { "kenwood" }
                selected=move || settings.with(|s| s.protocol == p)
              >
                {p.label()}
              </option>
            }).collect_view()}
          </select>
          <select
            aria-label=move || t("波特率")
            class=input_class("h-7 w-auto py-0 text-xs")
            prop:disabled=busy
            on:change=move |e| {
              let v = event_target_value(&e).parse().unwrap_or(9600);
              settings.update(|s| s.baud = v);
              save();
            }
          >
            {BAUD_RATES.into_iter().map(|b| view! {
              <option value=b.to_string() selected=move || settings.with(|s| s.baud == b)>{format!("{b} bps")}</option>
            }).collect_view()}
          </select>
          <Show when=move || settings.with(|s| s.protocol == Protocol::Icom)>
            <input
              aria-label=move || t("CI-V 地址（十六进制）")
              title=move || t("CI-V 地址（十六进制），IC-7300 为 94，IC-705 为 A4")
              class=input_class("h-7 w-14 py-0 font-mono text-xs uppercase")
              maxlength="2"
              prop:disabled=busy
              prop:value=move || format!("{:02X}", settings.with(|s| s.civ_addr))
              on:change=move |e| {
                if let Ok(v) = u8::from_str_radix(event_target_value(&e).trim(), 16) {
                  settings.update(|s| s.civ_addr = v);
                  save();
                }
              }
            />
          </Show>
          <button
            type="button"
            class=button_class(Variant::Outline, Size::Sm, "h-7")
            prop:disabled=move || matches!(status.get(), Status::Connecting)
            on:click=move |_| if busy() { disconnect(); status.set(Status::Idle); } else { connect(); }
          >
            {move || match status.get() {
              Status::Connecting => t("连接中…"),
              Status::Connected => t("断开"),
              Status::Idle | Status::Error(_) => t("连接电台"),
            }}
          </button>
          <span aria-live="polite" class="tabular-nums">
            {move || match status.get() {
              Status::Connected => {
                let f = freq.get().map_or_else(|| t("等待电台应答…"), |hz| format!("{} MHz", format_mhz(hz)));
                let m = mode.get().map(|m| format!(" · {m}")).unwrap_or_default();
                view! { <span class="text-emerald-700 dark:text-emerald-400">{tf("已连接 · {}{}", &[&(f).to_string(), &(m).to_string()])}</span> }.into_any()
              }
              Status::Error(msg) => view! { <span class="text-destructive">{tf("连接失败：{}", &[&(msg).to_string()])}</span> }.into_any(),
              _ => view! { <span>{(!connected()).then_some(t("连接后自动回填频率与模式"))}</span> }.into_any(),
            }}
          </span>
        }
        .into_any()
      } else {
        view! { <span>{move || t("需要桌面版 Chrome / Edge（Web Serial）才能连接电台。")}</span> }.into_any()
      }}
    </div>
  }
}
