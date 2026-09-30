use leptos::prelude::*;
use serde::Deserialize;

use crate::ui::{Size, Variant, button_class, input_class};
use crate::util::{download_text, storage};

/// 可选竞赛（CONTEST 字段值 → 展示名）。
const CONTESTS: &[(&str, &str)] = &[
  ("CQ-WW-SSB", "CQ WW SSB"),
  ("CQ-WW-CW", "CQ WW CW"),
  ("CQ-WPX-SSB", "CQ WPX SSB"),
  ("ARRL-DX-SSB", "ARRL DX SSB"),
  ("ARRL-DX-CW", "ARRL DX CW"),
];

/// 日志精简结构（读取 localStorage 的通联日志）。
#[derive(Deserialize, Default)]
struct LogbookLite {
  #[serde(default)]
  entries: Vec<EntryLite>,
}

#[derive(Deserialize, Default)]
struct EntryLite {
  #[serde(default)]
  date: String,
  #[serde(default)]
  time: String,
  #[serde(default)]
  freq: String,
  #[serde(default)]
  mode: String,
  #[serde(default)]
  callsign: String,
  #[serde(default)]
  rst_sent: String,
  #[serde(default)]
  rst_rcvd: String,
}

/// 本台信息精简结构。
#[derive(Deserialize, Default)]
struct StationLite {
  #[serde(default)]
  callsign: String,
  #[serde(default)]
  operator: String,
  #[serde(default)]
  gridsquare: String,
}

/// 频率 MHz → kHz（Cabrillo 要求整数 kHz）。
fn freq_to_khz(freq: &str) -> String {
  freq
    .trim()
    .parse::<f64>()
    .map(|f| format!("{:.0}", f * 1000.0))
    .unwrap_or_else(|_| freq.trim().to_owned())
}

/// 模式 → Cabrillo 模式代码。
fn mode_to_cabrillo(mode: &str) -> &'static str {
  match mode {
    "CW" => "CW",
    "SSB" | "AM" | "FM" => "PH",
    "RTTY" => "RY",
    _ => "DG",
  }
}

/// 生成 Cabrillo 文本（交换信息留空，需按竞赛规则补充）。
fn build_cabrillo(
  contest: &str,
  callsign: &str,
  operator: &str,
  gridsquare: &str,
  entries: &[EntryLite],
) -> String {
  let mut s = String::new();
  s.push_str("START-OF-LOG: 3.0\n");
  s.push_str(&format!("CALLSIGN: {}\n", callsign.trim()));
  s.push_str(&format!("CONTEST: {}\n", contest));
  s.push_str("CATEGORY-OPERATOR: SINGLE-OP\n");
  s.push_str("CATEGORY-ASSISTED: NON-ASSISTED\n");
  s.push_str("CATEGORY-BAND: ALL\n");
  s.push_str(&format!(
    "CATEGORY-MODE: {}\n",
    if contest.contains("CW") { "CW" } else { "SSB" }
  ));
  s.push_str("CATEGORY-TRANSMITTER: ONE\n");
  s.push_str("CLAIMED-SCORE: 0\n");
  s.push_str(&format!("NAME: {}\n", operator.trim()));
  s.push_str(&format!("LOCATION: {}\n", gridsquare.trim()));
  s.push_str(
    "ADDRESS:\nADDRESS-CITY:\nADDRESS-STATE-PROVINCE:\nADDRESS-POSTALCODE:\nADDRESS-COUNTRY:\n",
  );
  s.push_str("OPERATORS:\nSOAPBOX:\n");
  for e in entries {
    let date = e.date.replace('-', "");
    let time = e.time.replace(':', "");
    let rst_sent = if e.rst_sent.is_empty() {
      "599"
    } else {
      e.rst_sent.as_str()
    };
    let rst_rcvd = if e.rst_rcvd.is_empty() {
      "599"
    } else {
      e.rst_rcvd.as_str()
    };
    s.push_str(&format!(
      "QSO: {} {} {} {} {} {}  {} {} {}  0\n",
      freq_to_khz(&e.freq),
      mode_to_cabrillo(&e.mode),
      date,
      time,
      callsign.trim(),
      rst_sent,
      e.callsign,
      rst_rcvd,
      "",
    ));
  }
  s.push_str("END-OF-LOG:\n");
  s
}

/// 从通联日志按竞赛规则生成可提交的 Cabrillo 文件。
#[component]
pub(super) fn CabrilloGenerator() -> impl IntoView {
  let station: StationLite = storage::get_json("station-info").unwrap_or_default();
  let callsign = RwSignal::new(station.callsign.clone());
  let operator = RwSignal::new(station.operator.clone());
  let gridsquare = RwSignal::new(station.gridsquare.clone());
  let contest = RwSignal::new("CQ-WW-SSB".to_owned());
  let generated = RwSignal::new(String::new());

  let generate = move || {
    let lb: LogbookLite = storage::get_json("logbook").unwrap_or_default();
    let text = build_cabrillo(
      contest.get().as_str(),
      callsign.get().trim(),
      operator.get().trim(),
      gridsquare.get().trim(),
      &lb.entries,
    );
    generated.set(text);
  };

  let download = move || {
    let text = generated.get();
    if !text.is_empty() {
      download_text("contest.cbr", &text, "text/plain");
    }
  };

  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">"竞赛日志生成器"</h2>
      <div class="space-y-3 p-4">
        <div class="grid gap-3 sm:grid-cols-2">
          <label class="flex flex-col gap-1.5 text-sm">
            <span class="text-xs text-muted-foreground">"本台呼号"</span>
            <input
              type="text"
              prop:value=move || callsign.get()
              on:input=move |e| callsign.set(event_target_value(&e).to_uppercase())
              class=input_class("uppercase")
            />
          </label>
          <label class="flex flex-col gap-1.5 text-sm">
            <span class="text-xs text-muted-foreground">"竞赛"</span>
            <select
              prop:value=move || contest.get()
              on:change=move |e| contest.set(event_target_value(&e))
              class=input_class("")
            >
              {CONTESTS
                .iter()
                .map(|&(code, name)| {
                  view! { <option value=code>{name}</option> }
                })
                .collect_view()}
            </select>
          </label>
          <label class="flex flex-col gap-1.5 text-sm">
            <span class="text-xs text-muted-foreground">"操作员"</span>
            <input
              type="text"
              prop:value=move || operator.get()
              on:input=move |e| operator.set(event_target_value(&e))
              class=input_class("")
            />
          </label>
          <label class="flex flex-col gap-1.5 text-sm">
            <span class="text-xs text-muted-foreground">"本台网格"</span>
            <input
              type="text"
              prop:value=move || gridsquare.get()
              on:input=move |e| gridsquare.set(event_target_value(&e).to_uppercase())
              class=input_class("uppercase")
            />
          </label>
        </div>

        <div class="flex flex-wrap items-center gap-2">
          <button
            type="button"
            class=button_class(Variant::Default, Size::Default, "")
            on:click=move |_| generate()
          >
            "生成 Cabrillo"
          </button>
          <button
            type="button"
            class=button_class(Variant::Outline, Size::Default, "")
            on:click=move |_| download()
          >
            "下载 .cbr"
          </button>
        </div>

        {move || {
          let text = generated.get();
          if text.is_empty() {
            view! {
              <p class="text-xs text-muted-foreground">
                "从「通联日志」读取记录，选择竞赛后生成；交换信息（分区 / 序号等）留空，请按竞赛规则补充后提交。"
              </p>
            }
            .into_any()
          } else {
            view! {
              <pre class="max-h-64 overflow-auto rounded-lg border bg-muted/40 p-3 font-mono text-xs leading-5">
                {text}
              </pre>
            }
            .into_any()
          }
        }}
      </div>
    </section>
  }
}
