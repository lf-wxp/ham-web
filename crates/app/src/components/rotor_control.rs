//! 天线旋转器控制：通过 Web Serial 连接 GS-232 协议的旋转器，读取方位/仰角并遥控转向。
//!
//! Web Serial 仅 Chromium 系桌面浏览器支持；与电台 CAT 相同，这里用 `Reflect` 直接调用
//! `web-sys` 中的不稳定 API，避免引入 `--cfg web_sys_unstable_apis`。

use ham_web_core::rotor;
use js_sys::{Array, Function, Object, Promise, Reflect, Uint8Array};
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

use crate::i18n::{t, tf};
use crate::ui::{Button, Input, Size, Variant};
use crate::util::{js_error_message, sleep, window};

/// 轮询间隔（毫秒）。
const POLL_MS: u32 = 1000;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Status {
  Idle,
  Connecting,
  Connected,
  Error(String),
}

/// 下一个应答是方位还是仰角。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
  Azimuth,
  Elevation,
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

async fn open() -> Result<Conn, JsValue> {
  let serial = serial().ok_or_else(|| JsValue::from_str(&t("common.this-browser-does-not")))?;
  let port = call(&serial, "requestPort", &[]).await?;
  let opts = Object::new();
  // GS-232 控制器默认 9600 baud。
  Reflect::set(&opts, &"baudRate".into(), &9600u32.into())?;
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

/// 天线旋转器控制条。
#[component]
pub fn RotorControl() -> impl IntoView {
  let supported = serial().is_some();
  let status = RwSignal::new(Status::Idle);
  let azimuth = RwSignal::new(None::<u16>);
  let elevation = RwSignal::new(None::<u16>);
  let target_az = RwSignal::new(String::new());
  let conn = StoredValue::new_local(None::<Conn>);
  let writer_slot = StoredValue::new_local(None::<JsValue>);
  let generation = StoredValue::new(0u32);

  let disconnect = move || {
    generation.update_value(|g| *g = g.wrapping_add(1));
    writer_slot.try_update_value(Option::take);
    if let Some(c) = conn.try_update_value(Option::take).flatten() {
      spawn_local(close(c));
    }
  };

  let connect = move || {
    generation.update_value(|g| *g = g.wrapping_add(1));
    let my = generation.get_value();
    status.set(Status::Connecting);
    azimuth.set(None);
    elevation.set(None);
    spawn_local(async move {
      let c = match open().await {
        Ok(c) => c,
        Err(e) => {
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
      writer_slot.set_value(Some(writer.clone()));
      status.set(Status::Connected);
      let alive = move || generation.try_get_value() == Some(my);

      // 读循环：按分隔符切帧，根据当前期望字段解析角度。
      let expected = RwSignal::new(Field::Azimuth);
      let exp_reader = expected;
      spawn_local(async move {
        let mut buf: Vec<u8> = Vec::new();
        while alive() {
          let chunk = match call(&reader, "read", &[]).await {
            Ok(r) => r,
            Err(_) => break,
          };
          if prop(&chunk, "done").ok().is_some_and(|d| d.is_truthy()) {
            break;
          }
          if let Ok(v) = prop(&chunk, "value").and_then(JsCast::dyn_into::<Uint8Array>) {
            buf.extend_from_slice(&v.to_vec());
            while let Some(pos) = buf
              .iter()
              .position(|&b| b == b'\r' || b == b'\n' || b == b';')
            {
              let frame: Vec<u8> = buf.drain(..=pos).collect();
              if let Some(angle) = rotor::parse_angle(&String::from_utf8_lossy(&frame)) {
                match exp_reader.get_untracked() {
                  Field::Azimuth => azimuth.set(Some(angle)),
                  Field::Elevation => elevation.set(Some(angle)),
                }
              }
            }
            if buf.len() > 64 {
              buf.clear();
            }
          }
        }
      });

      // 写循环：交替轮询方位与仰角。
      let expected_write = expected;
      spawn_local(async move {
        let mut az_turn = true;
        while alive() {
          let cmd = if az_turn {
            rotor::QUERY_AZ
          } else {
            rotor::QUERY_EL
          };
          let bytes = Uint8Array::from(cmd);
          if call(&writer, "write", &[bytes.into()]).await.is_err() {
            break;
          }
          expected_write.set(if az_turn {
            Field::Azimuth
          } else {
            Field::Elevation
          });
          az_turn = !az_turn;
          sleep(POLL_MS).await;
        }
      });
    });
  };

  let send = move |cmd: Vec<u8>| {
    if let Some(writer) = writer_slot.try_get_value().flatten() {
      spawn_local(async move {
        let bytes = Uint8Array::from(cmd.as_slice());
        let _ = call(&writer, "write", &[bytes.into()]).await;
      });
    }
  };

  let rotate_az = move || {
    if let Ok(v) = target_az.get_untracked().parse::<u16>() {
      send(rotor::set_azimuth(v));
    }
  };

  on_cleanup(disconnect);

  let connected = move || matches!(status.get(), Status::Connected);
  let busy = move || matches!(status.get(), Status::Connected | Status::Connecting);

  view! {
    <div class="flex flex-wrap items-center gap-2 rounded-lg border border-dashed px-3 py-2 text-xs text-muted-foreground">
      <span class="font-medium text-foreground">{t("radio.antenna-rotator")}</span>
      {if supported {
        view! {
          <Button
            variant=Variant::Outline
            size=Size::Sm
            class="h-7"
            disabled=Signal::derive(move || matches!(status.get(), Status::Connecting))
            on_click=Callback::new(move |_| if busy() { disconnect(); status.set(Status::Idle); } else { connect(); })
          >
            {move || match status.get() {
              Status::Connecting => t("common.connecting"),
              Status::Connected => t("common.disconnect"),
              Status::Idle | Status::Error(_) => t("common.connect-rotator"),
            }}
          </Button>
          <Show when=connected>
            <span aria-live="polite" class="tabular-nums">
              {move || match (azimuth.get(), elevation.get()) {
                (Some(az), Some(el)) => tf("common.azimuth-elevation", &[&az.to_string(), &el.to_string()]),
                (Some(az), None) => tf("common.azimuth", &[&az.to_string()]),
                (None, Some(el)) => tf("common.elevation", &[&el.to_string()]),
                (None, None) => t("common.waiting-for-reply"),
              }}
            </span>
            <Input
              aria_label=t("common.target-azimuth-deg")
              placeholder=Signal::derive(move || t("knowledge.target-azimuth"))
              class="h-7 w-24 py-0 text-xs"
              value=target_az
              on_change=Callback::new(move |v: String| target_az.set(v))
            />
            <Button
              variant=Variant::Default
              size=Size::Sm
              class="h-7"
              on_click=Callback::new(move |_| rotate_az())
            >
              {t("common.rotate")}
            </Button>
            <Button
              variant=Variant::Ghost
              size=Size::Sm
              class="h-7"
              on_click=Callback::new(move |_| send(rotor::STOP.to_vec()))
            >
              {t("common.stop")}
            </Button>
          </Show>
          {move || match status.get() {
            Status::Error(msg) => view! { <span class="text-destructive">{tf("common.connection-failed", &[&msg.to_string()])}</span> }.into_any(),
            _ => view! { <span>{(!connected()).then_some(t("common.after-connecting-read-azimuth"))}</span> }.into_any(),
          }}
        }
        .into_any()
      } else {
        view! { <span>{t("common.a-desktop-chrome-edge")}</span> }.into_any()
      }}
    </div>
  }
}
