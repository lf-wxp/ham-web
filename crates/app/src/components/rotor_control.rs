//! 天线旋转器控制：通过 Web Serial 连接 GS-232 协议的旋转器，读取方位 / 仰角，遥控转向，
//! 并可**选择性地**让它自动对准多普勒面板里正在跟踪的卫星。
//!
//! 安全约定（旋转器驱动真实的机械运动，比写电台频率更保守；每一条都有对应的单测或 e2e）：
//! - 连接后只**读**；自动跟踪必须手动勾选，重新连接、断开、手动转向、按「停止」都会把它关掉；
//! - 只在卫星高于地平线时转；方位回绕到 0–359°、仰角夹到 0–90°（`ham_web_core::rotor`）；
//! - 目标变化小于死区（2°）不重发，且与**上次写入的目标**比较（回读有延迟，拿它比较会反复重发）；
//! - 离我们最后写入的目标的距离**连续约 8 拍没变近**（被人手动转走、卡住、按了控制器上的停止），立刻停止自动跟踪；
//!   旋转器转得慢，刚写完目标时离目标很远是正常的，所以看「有没有在靠近」而不是「离得远不远」；
//! - 协议对 Hamlib 的 `gs232a.c` / `gs232b.c` 核对过：`W<方位> <仰角>\r`、`C2\r` 查询、`S\r` 停止。
//!
//! **不处理线缆缠绕与行程限位**：本站不知道你的旋转器是 360° 还是 450°，也不知道它的限位在哪。
//! 过顶时方位会快速从一侧扫到另一侧，旋转器走哪条路由控制器自己决定。**没有任何一次真机验证**。
//!
//! 共用 `crate::web_serial`（含更完善的 `close_serial`：先 `abort` 写流再释放锁）。

use ham_web_core::rotor::{self, StallDetector, Target};
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;

use crate::i18n::{t, tf};
use crate::ui::{Button, Checkbox, Input, Label, Size, Variant};
use crate::util::{js_error_message, sleep, unique_id};
use crate::web_serial::{
  Conn, call, close_serial, is_user_cancel, open_serial, prop, serial, write_bytes,
};

/// 轮询 / 写入间隔（毫秒）。
const POLL_MS: u32 = 1000;
/// GS-232 控制器默认 9600 baud。
const BAUD: u32 = 9600;
/// 多普勒面板此刻要对准的方向（度）。卫星在地平线下或轨迹未加载时为 `None`。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aim {
  pub az: f64,
  pub el: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Status {
  Idle,
  Connecting,
  Connected,
  Error(String),
}

/// 天线旋转器控制条。`aim` 是自动跟踪的目标来源（由多普勒面板提供）。
#[component]
pub fn RotorControl(#[prop(into)] aim: Signal<Option<Aim>>) -> impl IntoView {
  let supported = serial().is_some();
  let status = RwSignal::new(Status::Idle);
  let azimuth = RwSignal::new(None::<u16>);
  let elevation = RwSignal::new(None::<u16>);
  let target_az = RwSignal::new(String::new());
  let track = RwSignal::new(false);
  let last_written = RwSignal::new(None::<Target>);
  let writes = RwSignal::new(0u32);
  // 自动跟踪被停下的原因：用户手动操作了控制器。
  let overridden = RwSignal::new(false);
  let conn = StoredValue::new_local(None::<Conn>);
  let writer_slot = StoredValue::new_local(None::<wasm_bindgen::JsValue>);
  let generation = StoredValue::new(0u32);
  let stall = StoredValue::new(StallDetector::default());
  let track_id = unique_id("rotor-auto-track");

  // 只动 `StoredValue`、不碰信号：它还会被 `on_cleanup` 调用，那时信号可能已随组件释放。
  let teardown = move || {
    generation.try_update_value(|g| *g = g.wrapping_add(1));
    writer_slot.try_update_value(Option::take);
    if let Some(c) = conn.try_update_value(Option::take).flatten() {
      spawn_local(close_serial(c));
    }
  };
  let reset_ui = move || {
    track.set(false);
    last_written.set(None);
    writes.set(0);
    overridden.set(false);
    stall.update_value(StallDetector::reset);
    azimuth.set(None);
    elevation.set(None);
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
    generation.update_value(|g| *g = g.wrapping_add(1));
    let my = generation.get_value();
    reset_ui();
    status.set(Status::Connecting);
    spawn_local(async move {
      let outcome = open_serial(BAUD).await;
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
      writer_slot.set_value(Some(writer.clone()));
      conn.set_value(Some(c));
      status.set(Status::Connected);
      let alive = move || generation.try_get_value() == Some(my);

      // 读循环：按行切帧（`\r` / `\n` / `;`），GS-232A 与 B 两种应答都认。
      spawn_local(async move {
        let mut buf: Vec<u8> = Vec::new();
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
            fail(my, t("radio.rotor-lost"));
            return;
          }
          let Ok(v) = prop(&chunk, "value").and_then(JsCast::dyn_into::<js_sys::Uint8Array>) else {
            continue;
          };
          buf.extend_from_slice(&v.to_vec());
          while let Some(pos) = buf
            .iter()
            .position(|&b| b == b'\r' || b == b'\n' || b == b';')
          {
            let frame: Vec<u8> = buf.drain(..=pos).collect();
            let text = String::from_utf8_lossy(&frame);
            let Some((az, el)) = rotor::parse_position(&text) else {
              continue;
            };
            azimuth.set(Some(az));
            elevation.set(el.or(elevation.get_untracked()));
            // 没在往目标走：离我们写的目标的距离连续几拍没变近（被人手动转走、卡住、按了停止）。
            if track.get_untracked()
              && let Some(want) = last_written.get_untracked()
              && stall
                .try_update_value(|d| d.observe(az, el, want))
                .unwrap_or(false)
            {
              track.set(false);
              overridden.set(true);
              last_written.set(None);
            }
          }
          if buf.len() > 64 {
            buf.clear();
          }
        }
      });

      // 写循环：每秒查一次位置；勾选了自动跟踪且目标有变化才写。
      while alive() {
        if let Err(e) = write_bytes(&writer, rotor::QUERY_BOTH).await {
          return fail(my, js_error_message(&e));
        }
        if !alive() {
          return;
        }
        if track.get_untracked()
          && let Some(a) = aim.get_untracked()
          && let Some(next) = rotor::next_target(a.az, a.el, last_written.get_untracked())
        {
          if let Err(e) = write_bytes(&writer, &rotor::target_command(next)).await {
            return fail(my, js_error_message(&e));
          }
          if !alive() {
            return;
          }
          last_written.set(Some(next));
          // 新目标：旧的「最近距离」不再可比。
          stall.update_value(StallDetector::reset);
          writes.update(|n| *n = n.saturating_add(1));
        }
        sleep(POLL_MS).await;
      }
    });
  };

  let send = move |cmd: Vec<u8>| {
    if let Some(writer) = writer_slot.try_get_value().flatten() {
      spawn_local(async move {
        let _ = write_bytes(&writer, &cmd).await;
      });
    }
  };

  // 手动转向 / 停止：都意味着用户接管了旋转器，自动跟踪立刻关掉。
  let take_over = move || {
    track.set(false);
    last_written.set(None);
    stall.update_value(StallDetector::reset);
  };
  let rotate_az = move || {
    let Ok(az) = target_az.get_untracked().trim().parse::<f64>() else {
      return;
    };
    // 保持当前仰角；还没读到仰角就用 0（只转方位，不抬升）。
    let keep = elevation.get_untracked().map_or(0.0, f64::from);
    if let Some(cmd) = rotor::set_azimuth_keep(az, keep) {
      take_over();
      send(cmd);
    }
  };

  on_cleanup(teardown);

  let connected = move || matches!(status.get(), Status::Connected);
  let busy = move || matches!(status.get(), Status::Connected | Status::Connecting);

  view! {
    <div class="space-y-2 border border-dashed px-3 py-2 text-xs text-muted-foreground">
      <div class="flex flex-wrap items-center gap-2">
        <span class="text-foreground">{move || t("radio.antenna-rotator")}</span>
        {if supported {
          view! {
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
                Status::Idle | Status::Error(_) => t("common.connect-rotator"),
              }}
            </Button>
            <Show when=connected>
              <span aria-live="polite" class="tabular-nums">
                {move || match (azimuth.get(), elevation.get()) {
                  (Some(az), Some(el)) => {
                    tf("common.azimuth-elevation", &[&az.to_string(), &el.to_string()])
                  }
                  (Some(az), None) => tf("common.azimuth", &[&az.to_string()]),
                  (None, Some(el)) => tf("common.elevation", &[&el.to_string()]),
                  (None, None) => t("common.waiting-for-reply"),
                }}
              </span>
              <Input
                aria_label=Signal::derive(move || t("common.target-azimuth-deg"))
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
                {move || t("common.rotate")}
              </Button>
              <Button
                variant=Variant::Ghost
                size=Size::Sm
                class="h-7"
                on_click=Callback::new(move |_| {
                  take_over();
                  send(rotor::STOP.to_vec());
                })
              >
                {move || t("common.stop")}
              </Button>
            </Show>
            {move || match status.get() {
              Status::Error(msg) => {
                view! {
                  <span class="text-hp-text" role="alert">
                    {tf("common.connection-failed", &[&msg])}
                  </span>
                }
                  .into_any()
              }
              _ => {
                view! { <span>{(!connected()).then_some(t("common.after-connecting-read-azimuth"))}</span> }
                  .into_any()
              }
            }}
          }
            .into_any()
        } else {
          view! { <span>{move || t("common.a-desktop-chrome-edge")}</span> }.into_any()
        }}
      </div>

      <Show when=connected>
        <div class="space-y-1 border-l-2 border-destructive/60 pl-3">
          <div class="flex items-center gap-2">
            <Checkbox
              id=track_id.clone()
              checked=track
              on_change=Callback::new(move |on: bool| {
                track.set(on);
                // 重新开启：忘掉上次的写入记录与偏离计数，下一秒按当前目标写一次。
                last_written.set(None);
                stall.update_value(StallDetector::reset);
                overridden.set(false);
                writes.set(0);
              })
            />
            <Label r#for=track_id.clone()>{move || t("radio.rotor-auto-track")}</Label>
          </div>
          <p class="text-hp-text">{move || t("radio.rotor-auto-track-warning")}</p>
          <div class="tabular-nums" aria-live="polite">
            {move || {
              let n = writes.get();
              last_written
                .get()
                .map(|w| {
                  crate::i18n::tp(
                    "radio.rotor-auto-written",
                    n,
                    &[&n.to_string(), &w.az.to_string(), &w.el.to_string()],
                  )
                })
            }}
          </div>
          {move || {
            (track.get() && aim.get().is_none())
              .then(|| view! { <div>{t("radio.rotor-auto-idle")}</div> })
          }}
          {move || {
            overridden
              .get()
              .then(|| {
                view! {
                  <div class="text-hp-text" role="alert">
                    {t("radio.rotor-auto-overridden")}
                  </div>
                }
              })
          }}
        </div>
      </Show>
    </div>
  }
}
