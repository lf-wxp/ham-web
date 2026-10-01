use std::collections::HashMap;

use ham_web_core::qsl_labels::Label;
use leptos::prelude::*;

use crate::pages::log::LogEntry;

/// 单张 QSL 标签（一个呼号一行或数行通联）。
#[component]
pub(crate) fn LabelView(
  label: Label,
  entries: HashMap<u64, LogEntry>,
  my_call: String,
  compact: bool,
) -> impl IntoView {
  let rows: Vec<LogEntry> = label
    .ids
    .iter()
    .filter_map(|id| entries.get(id).cloned())
    .collect();
  let (call_size, text_size) = if compact {
    ("text-[10pt]", "text-[6.5pt]")
  } else {
    ("text-[13pt]", "text-[7.5pt]")
  };
  view! {
    <div class=format!("flex h-full flex-col overflow-hidden px-[2.5mm] py-[1.8mm] leading-tight {text_size}")>
      <div class="flex items-baseline justify-between gap-2">
        <span>
          <span class="mr-1 text-zinc-500">"To Radio"</span>
          <span class=format!("font-mono font-bold {call_size}")>{label.callsign.clone()}</span>
        </span>
        <span class="text-zinc-500">"Confirming QSO"</span>
      </div>
      <table class="mt-[1mm] w-full border-collapse tabular-nums">
        <thead>
          <tr class="border-b border-zinc-400 text-left text-zinc-600">
            <th class="font-medium">"Date"</th>
            <th class="font-medium">"UTC"</th>
            <th class="font-medium">"MHz"</th>
            <th class="font-medium">"Mode"</th>
            <th class="font-medium">"RST"</th>
          </tr>
        </thead>
        <tbody>
          {rows.into_iter().map(|e| {
            let freq = if e.freq.trim().is_empty() { e.band_label() } else { e.freq.trim().to_owned() };
            let rst = if e.rst_sent.is_empty() { e.rst.clone() } else { e.rst_sent.clone() };
            view! {
              <tr>
                <td>{e.date.clone()}</td>
                <td>{e.time.clone()}</td>
                <td>{freq}</td>
                <td>{e.mode.clone()}</td>
                <td>{rst}</td>
              </tr>
            }
          }).collect_view()}
        </tbody>
      </table>
      <div class="mt-auto flex justify-between text-zinc-500">
        <span>{if my_call.is_empty() { String::new() } else { format!("From {my_call}") }}</span>
        <span>"PSE QSL TNX 73"</span>
      </div>
    </div>
  }
}
