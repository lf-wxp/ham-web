//! VNA 读数页面：两条入口（`.s1p` 文件 / NanoVNA 串口）落成同一份 [`Sweep`]，
//! 再给出最低驻波、该点阻抗、驻波 ≤ 2 的频段与两张图。

use ham_web_core::touchstone::{
  Sweep, min_swr_index, nanovna_sweep, parse_s1p, shell_done, swr_band, to_s1p,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsValue;

use super::locus::S11Locus;
use super::swr_plot::SwrPlot;
use super::tdr_panel::TdrPanel;
use super::{NOTE, fmt_hz, parse_error_text};
use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::{t, tf};
use crate::share_score::download;
use crate::ui::{Button, ButtonLink, ControlSize, Field, FileInput, NumberField, Size, Variant};
use crate::util::{js_error_message, read_file_text, set_title};
use crate::web_serial::{
  Conn, close_serial, is_timeout, is_user_cancel, open_serial, read_chunk_within, serial,
  write_bytes,
};

/// NanoVNA 固件 shell 的扫频点数：101 是所有型号（含最早的 NanoVNA）都支持的固件默认值，
/// 较新固件虽可更多点，但读数页要的是「能在任何型号上读出来」，所以不去探测型号。
const POINTS: u32 = 101;
/// 驻波可用带宽的门限。
const SWR_LIMIT: f64 = 2.0;
/// 串口两次数据之间最长等多久（毫秒）：设备不回提示符（选错端口、固件卡住）时据此放弃，
/// 否则按钮会一直转圈、端口一直占着，用户没有任何办法退出。
const IDLE_TIMEOUT_MS: i32 = 5_000;
/// 单条命令回显的字节上限：`data 0` 在 101 点时不到 4 KiB，1 MiB 已远超任何正常回显，
/// 只用来挡住「对着一个不停吐数据的设备读」把内存吃光。
const MAX_ECHO_BYTES: usize = 1 << 20;

/// 发一条 shell 命令，读到提示符为止，返回整段回显。
///
/// NanoVNA 是 USB CDC，波特率形同虚设；读到流结束（拔线）、空闲超时、回显超限都按错误处理。
async fn shell(conn: &Conn, command: &str) -> Result<String, JsValue> {
  write_bytes(&conn.writer, format!("{command}\r").as_bytes()).await?;
  let mut buf = Vec::new();
  loop {
    match read_chunk_within(&conn.reader, IDLE_TIMEOUT_MS).await? {
      Some(bytes) => buf.extend_from_slice(&bytes),
      None => return Err(JsValue::from_str("serial stream closed")),
    }
    if buf.len() > MAX_ECHO_BYTES {
      return Err(JsValue::from_str("serial echo too long"));
    }
    let text = String::from_utf8_lossy(&buf);
    if shell_done(&text) {
      return Ok(text.into_owned());
    }
  }
}

/// 一次完整的串口读数：设扫频范围 → 读频点 → 读 S11。
async fn read_nanovna(start_hz: u64, stop_hz: u64) -> Result<(String, String), JsValue> {
  let conn = open_serial(115_200).await?;
  // 先清掉上电横幅 / 残留提示符：发一个空行，读到提示符为止。
  let outcome = async {
    shell(&conn, "").await?;
    shell(&conn, &format!("sweep {start_hz} {stop_hz} {POINTS}")).await?;
    let freqs = shell(&conn, "frequencies").await?;
    let data = shell(&conn, "data 0").await?;
    Ok((freqs, data))
  }
  .await;
  close_serial(conn).await;
  outcome
}

#[component]
pub fn VnaReaderPage() -> impl IntoView {
  set_title("tools.vna-reader");
  let supported = serial().is_some();

  let sweep = RwSignal::new(None::<Sweep>);
  let sweep_memo = Memo::new(move |_| sweep.get());
  let error = RwSignal::new(None::<String>);
  let source = RwSignal::new(String::new());
  let busy = RwSignal::new(false);
  let start_mhz = RwSignal::new(String::from("6.9"));
  let stop_mhz = RwSignal::new(String::from("7.4"));
  // TDR 面板的速率因子必须由页面持有：面板住在下面的结果区里，而结果区整体在一个响应式闭包
  // 内（切语言会重跑它、整段重建），状态放面板里会被重置成默认值。
  let tdr_vf = RwSignal::new(String::from("0.66"));

  let best = Signal::derive(move || sweep.with(|s| s.as_ref().and_then(min_swr_index)));

  let on_files = Callback::new(move |files: Vec<web_sys::File>| {
    let Some(file) = files.into_iter().next() else {
      return;
    };
    let name = file.name();
    spawn_local(async move {
      // 读不出来（超过大小上限 / 读取失败）与「读出来但没有数据」是两回事，要分开说。
      let Some(text) = read_file_text(&file).await else {
        error.set(Some(t("tools.vna-file-unreadable")));
        return;
      };
      match parse_s1p(&text) {
        Ok(s) => {
          sweep.set(Some(s));
          source.set(name);
          error.set(None);
        }
        Err(e) => error.set(Some(tf("tools.vna-parse-failed", &[&parse_error_text(&e)]))),
      }
    });
  });

  let read_serial = move |_| {
    let start = start_mhz
      .get_untracked()
      .trim()
      .parse::<f64>()
      .unwrap_or(0.0);
    let stop = stop_mhz
      .get_untracked()
      .trim()
      .parse::<f64>()
      .unwrap_or(0.0);
    if !(start > 0.0 && stop > start) {
      error.set(Some(t("tools.vna-range-invalid")));
      return;
    }
    error.set(None);
    busy.set(true);
    spawn_local(async move {
      let outcome = read_nanovna((start * 1e6) as u64, (stop * 1e6) as u64).await;
      // 读数期间离开了页面：信号已释放，什么都不写。
      if busy.try_set(false).is_some() {
        return;
      }
      match outcome {
        Ok((freqs, data)) => match nanovna_sweep(&freqs, &data, 50.0) {
          Ok(s) => {
            sweep.set(Some(s));
            source.set("NanoVNA".to_owned());
          }
          Err(e) => error.set(Some(tf("tools.vna-parse-failed", &[&parse_error_text(&e)]))),
        },
        Err(e) if is_user_cancel(&e) => {}
        Err(e) if is_timeout(&e) => error.set(Some(t("tools.vna-serial-timeout"))),
        Err(e) => error.set(Some(tf(
          "tools.vna-serial-failed",
          &[&js_error_message(&e)],
        ))),
      }
    });
  };

  let export = move |_| {
    let Some(s) = sweep.get_untracked() else {
      return;
    };
    let body = js_sys::encode_uri_component(&to_s1p(&s));
    download(&format!("data:text/plain;charset=utf-8,{body}"), "s11.s1p");
  };

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=Signal::derive(move || t("tools.vna-reader"))
        subtitle=Signal::derive(move || t("tools.vna-reader-subtitle"))
      />
      <PageContainer class="space-y-4">
        <div class="grid gap-4 md:grid-cols-2">
          <section class="pxl-card space-y-2 border bg-card p-4">
            <h2 class="text-sm font-semibold">{move || t("tools.vna-import-s1p")}</h2>
            <p class=NOTE>{move || t("tools.vna-import-note")}</p>
            <FileInput
              accept=".s1p,.S1P,.txt"
              label=t("tools.vna-import-s1p")
              variant=Variant::Outline
              on_files=on_files
            />
          </section>
          <section class="pxl-card space-y-2 border bg-card p-4">
            <h2 class="text-sm font-semibold">{move || t("tools.vna-serial")}</h2>
            {if supported {
              view! {
                <p class=NOTE>{move || t("tools.vna-serial-note")}</p>
                <div class="grid grid-cols-2 gap-2">
                  <Field label=Signal::derive(move || t("tools.vna-start-mhz")) r#for="vna-start">
                    <NumberField
                      id="vna-start"
                      value=Signal::derive(move || start_mhz.get())
                      on_change=Callback::new(move |v: String| start_mhz.set(v))
                      step=0.1
                      min=0.05
                      max=6000.0
                      size=ControlSize::Sm
                    />
                  </Field>
                  <Field label=Signal::derive(move || t("tools.vna-stop-mhz")) r#for="vna-stop">
                    <NumberField
                      id="vna-stop"
                      value=Signal::derive(move || stop_mhz.get())
                      on_change=Callback::new(move |v: String| stop_mhz.set(v))
                      step=0.1
                      min=0.05
                      max=6000.0
                      size=ControlSize::Sm
                    />
                  </Field>
                </div>
                <Button
                  variant=Variant::Default
                  size=Size::Sm
                  loading=Signal::derive(move || busy.get())
                  disabled=Signal::derive(move || busy.get())
                  on_click=Callback::new(read_serial)
                >
                  {move || if busy.get() { t("tools.vna-reading") } else { t("tools.vna-read") }}
                </Button>
              }
                .into_any()
            } else {
              view! { <p class=NOTE role="note">{move || t("tools.vna-serial-unsupported")}</p> }
                .into_any()
            }}
          </section>
        </div>

        {move || error.get().map(|e| view! { <p class="text-sm text-destructive" role="alert">{e}</p> })}

        {move || {
          let Some(s) = sweep.get() else {
            return view! { <p class=NOTE>{move || t("tools.vna-empty")}</p> }.into_any();
          };
          let n = s.points.len();
          let (f0, f1) = (s.points[0].f_hz, s.points[n - 1].f_hz);
          let best_line = best
            .get()
            .map(|i| {
              let p = s.points[i];
              let z = p
                .impedance(s.z0)
                .map_or_else(
                  || "∞".to_owned(),
                  |(r, x)| format!("{r:.1} {} j{:.1}", if x < 0.0 { "−" } else { "+" }, x.abs()),
                );
              (
                tf("tools.vna-min-swr", &[&format!("{:.2}", p.swr()), &fmt_hz(p.f_hz)]),
                tf("tools.vna-impedance", &[&z, &format!("{:.1}", p.return_loss_db())]),
              )
            });
          let band = swr_band(&s, SWR_LIMIT).map(|(lo, hi)| {
            let at_edge = (lo - f0).abs() < 1e-6 || (hi - f1).abs() < 1e-6;
            (
              tf("tools.vna-band", &[&fmt_hz(lo), &fmt_hz(hi), &fmt_hz(hi - lo)]),
              at_edge,
            )
          });
          view! {
            <section class="pxl-card space-y-3 border bg-card p-4" data-slot="vna-result">
              <p class="text-sm" data-slot="vna-summary">
                {source.get()} " · " {tf("tools.vna-summary", &[&n.to_string(), &fmt_hz(f0), &fmt_hz(f1)])}
              </p>
              {best_line
                .map(|(a, b)| {
                  view! {
                    <p class="text-sm font-medium" data-slot="vna-best">{a}</p>
                    <p class="text-sm">{b}</p>
                  }
                })}
              {match band {
                Some((text, at_edge)) => {
                  view! {
                    <p class="text-sm" data-slot="vna-band">{text}</p>
                    {at_edge.then(|| view! { <p class=NOTE>{move || t("tools.vna-band-edge-note")}</p> })}
                  }
                    .into_any()
                }
                None => view! { <p class="text-sm" data-slot="vna-band">{move || t("tools.vna-band-none")}</p> }.into_any(),
              }}
              <div class="grid gap-4 md:grid-cols-2">
                <S11Locus sweep=sweep_memo best=best />
                <SwrPlot sweep=sweep_memo />
              </div>
              <TdrPanel sweep=sweep_memo vf_text=tdr_vf />
              <div class="flex flex-wrap gap-2">
                <Button variant=Variant::Outline size=Size::Sm on_click=Callback::new(export)>
                  {move || t("tools.vna-export")}
                </Button>
                <ButtonLink href="/smith" variant=Variant::Ghost size=Size::Sm>
                  {move || t("tools.vna-to-smith")}
                </ButtonLink>
              </div>
            </section>
          }
            .into_any()
        }}
      </PageContainer>
    </div>
  }
}
