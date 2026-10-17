//! 多普勒联动的电台一侧：通过 Web Serial 连接电台，**可选地**把每秒算出的下行修正频率写进去。
//!
//! 安全约定（每一条都有对应的单测或 e2e）：
//! - 连接后只**读**；写入必须用户手动勾选「自动调谐」，重新连接、断开、手动改频都会把它关掉；
//! - 只写一个频率（接收修正频率），不改模式、不碰 VFO、不写发射频率；
//! - 读到电台自己回报的 `FA` 位数之前不写（写错位数电台会忽略，见 `ham_web_core::cat`）；
//! - 只在卫星高于地平线时写；写入有节流（`MIN_WRITE_DELTA_HZ`），避免静噪反复闪；
//! - 发现电台回读的频率连续几次偏离我们写的值，就当作有人转了旋钮，立刻停写。
//!
//! 写**上行（发射）频率**是另一个独立的、默认关闭的开关，规则更严（见 `ham_web_core::sat_transponder`）：
//! - 只写「另一个 VFO」（Kenwood 系 `FB`，Icom `25 01`），**不开分频、不切 VFO、没有任何 PTT 命令**；
//!   电台实际会不会往那个频率发射，取决于用户在电台上自己设的分频（页面上写明）；
//! - 必须先开着下行自动调谐才有效；下行因任何原因停下（手动改频、断开、换卫星、重连），上行**同时**停下；
//! - 每次写之前过 `guard_uplink`：落在业余波段内、卫星业余业务段内、且离标称上行不超过多普勒的物理上限；
//!   任何一条不满足就不写，并把原因显示给用户；
//! - 与下行同样的节流与「读到位数才写」。
//!
//! 判定逻辑（取整、节流、手动改频检测）都在 `ham_web_core::sat_doppler`，这里只负责串口读写与界面。

// `Update` 改名：不然它会压住 prelude 里的 `Update` trait，`RwSignal::update` 就找不到了。
use ham_web_core::cat::{
  BAUD_RATES, Parser, Protocol, Update as CatUpdate, format_mhz, poll_commands,
  set_frequency_command, set_other_vfo_frequency_command,
};
use ham_web_core::sat_doppler::{OverrideDetector, round_for_rig, should_write};
use ham_web_core::sat_transponder::{TxRefusal, guard_uplink};
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

/// 此刻该写进「另一个 VFO」的上行（发射）频率，连同护栏需要的标称值。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct TxPlan {
  /// 已做多普勒修正的发射频率（Hz）。
  pub hz: f64,
  /// 该信道 / 转发器的标称上行（Hz）。
  pub nominal_hz: u64,
  /// 标称之外允许的余量（Hz）：FM 为 0，线性转发器为通带半宽。
  pub slack_hz: u64,
}

/// 护栏拒绝原因的显示文案（`t()` 的字面量写在调用点，静态扫描才认得）。
fn refusal_text(why: TxRefusal) -> String {
  match why {
    TxRefusal::NotValid => t("radio.doppler-rig-uplink-refused-invalid"),
    TxRefusal::OutsideAmateurBand => t("radio.doppler-rig-uplink-refused-band"),
    TxRefusal::OutsideSatelliteService => t("radio.doppler-rig-uplink-refused-service"),
    TxRefusal::FarFromNominal => t("radio.doppler-rig-uplink-refused-far"),
  }
}

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
  /// 上行（发射）频率计划；卫星在地平线下或参数不全时为 `None`。
  #[prop(into)]
  tx_plan: Signal<Option<TxPlan>>,
  #[prop(into)] selection: Signal<u32>,
) -> impl IntoView {
  let supported = serial().is_some();
  let settings = RwSignal::new(load_settings());
  let status = RwSignal::new(Status::Idle);
  let rig_hz = RwSignal::new(None::<u64>);
  let digits = RwSignal::new(None::<usize>);
  let auto = RwSignal::new(false);
  // 上行开关：默认关；只有 `auto` 开着才会真的写。任何让 `auto` 关掉的路径都经 `stop_auto` 一并关掉它。
  let tx_follow = RwSignal::new(false);
  let last_tx = RwSignal::new(None::<u64>);
  let tx_writes = RwSignal::new(0u32);
  let tx_refusal = RwSignal::new(None::<TxRefusal>);
  let paused = RwSignal::new(false);
  let writes = RwSignal::new(0u32);
  let last_hz = RwSignal::new(None::<u64>);
  let conn = StoredValue::new_local(None::<Conn>);
  let generation = StoredValue::new(0u32);
  let detector = StoredValue::new(OverrideDetector::default());

  // 下行自动调谐停下时，上行必须同时停下并忘掉自己的写入记录：上行绝不比下行活得更久。
  let stop_auto = move || {
    auto.set(false);
    tx_follow.set(false);
    last_tx.set(None);
    tx_refusal.set(None);
    // 上行被强制停下，重新开启时从头计数（与上行复选框的开关行为一致）。
    tx_writes.set(0);
  };

  Effect::new(move |_| {
    let _ = selection.get();
    stop_auto();
    paused.set(false);
    last_hz.set(None);
    detector.update_value(OverrideDetector::reset);
  });

  let auto_id = unique_id("doppler-auto-tune");
  let tx_id = unique_id("doppler-write-uplink");

  let save = move || settings.with_untracked(save_settings);

  // 只动 `StoredValue`、不碰信号：它还会被 `on_cleanup` 调用，那时信号可能已随组件释放。
  let teardown = move || {
    generation.try_update_value(|g| *g = g.wrapping_add(1));
    if let Some(c) = conn.try_update_value(Option::take).flatten() {
      spawn_local(close_serial(c));
    }
  };
  let reset_ui = move || {
    stop_auto();
    tx_writes.set(0);
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
              stop_auto();
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

        // 卫星在地平线下（`tx_plan` 为 None）时，清掉上一次残留的「上行被拒」提示，
        // 免得它和「地平线以下不写」同时挂着一条过期告警。
        if tx_plan.get_untracked().is_none() {
          tx_refusal.set(None);
        }

        // 上行（发射）：必须 `auto` 与 `tx_follow` 都开着；没有手动改频嫌疑；卫星在地平线上（`tx_plan` 为 Some）。
        if auto.get_untracked()
          && tx_follow.get_untracked()
          && detector.with_value(OverrideDetector::write_allowed)
          && let Some(plan) = tx_plan.get_untracked()
          && let Some(hz) = round_for_rig(plan.hz)
        {
          match guard_uplink(hz, plan.nominal_hz, plan.slack_hz) {
            Err(why) => {
              // 护栏拒绝：一个字节也不写，把原因给用户看；目标回到允许范围后下一秒自动恢复。
              tx_refusal.set(Some(why));
            }
            Ok(hz) => {
              tx_refusal.set(None);
              if should_write(last_tx.get_untracked(), hz)
                && let Some(cmd) = set_other_vfo_frequency_command(
                  s.protocol,
                  s.civ_addr,
                  hz,
                  digits.get_untracked(),
                )
              {
                if let Err(e) = write_bytes(&writer, &cmd).await {
                  return fail(my, js_error_message(&e));
                }
                if !alive() {
                  return;
                }
                last_tx.set(Some(hz));
                tx_writes.update(|n| *n = n.saturating_add(1));
              }
            }
          }
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
              if on {
                auto.set(true);
                // 重新开启：忘掉上次的写入记录，下一秒就按当前目标写一次。
                paused.set(false);
                last_hz.set(None);
                detector.update_value(OverrideDetector::reset);
              } else {
                stop_auto();
              }
            })
          />
          <Label r#for=auto_id.clone()>{move || t("radio.doppler-rig-auto")}</Label>
        </div>
        <div class="space-y-1 border-l-2 border-destructive/60 pl-3">
          <div class="flex items-center gap-2">
            <Checkbox
              id=tx_id.clone()
              checked=tx_follow
              disabled=Signal::derive(move || !auto.get())
              on_change=Callback::new(move |on: bool| {
                // 只有下行自动调谐开着才能开；关上行时忘掉自己的写入记录，再开就按当前目标重写一次。
                tx_follow.set(on && auto.get_untracked());
                last_tx.set(None);
                tx_refusal.set(None);
                tx_writes.set(0);
              })
            />
            <Label r#for=tx_id.clone()>{move || t("radio.doppler-rig-uplink")}</Label>
          </div>
          <p class="text-destructive">{move || t("radio.doppler-rig-uplink-warning")}</p>
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
            let n = tx_writes.get();
            last_tx
              .get()
              .map(|hz| {
                view! {
                  <div>
                    {tp("radio.doppler-rig-uplink-written", n, &[&n.to_string(), &format_mhz(hz)])}
                  </div>
                }
              })
          }}
          {move || {
            tx_refusal
              .get()
              .map(|why| {
                view! {
                  <div class="text-destructive" role="alert">
                    {refusal_text(why)}
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
