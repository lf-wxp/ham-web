//! 传输线（`TL`）表格：把一段馈线接在天线的两个断口之间。
//!
//! 数值用 `String` 承载（与页面上其它表格一致），解析失败的行会被跳过；
//! 越界的值交给核心的 `solve_with_lines` 拒绝，页面显示「参数无法求解」。

use ham_web_core::nec::TransmissionLine;
use leptos::prelude::*;

mod field;
use field::field;

use crate::i18n::{t, tf};

const NOTE: &str = "text-xs text-muted-foreground";
const CELL: &str = "border px-2 py-1";

/// 一行传输线（表格里的字符串状态）。
#[derive(Debug, Clone, PartialEq)]
pub(super) struct LineRow {
  pub id: usize,
  pub wire_a: String,
  pub at_a: String,
  pub wire_b: String,
  pub at_b: String,
  pub z0: String,
  pub length_m: String,
  pub vf: String,
}

impl LineRow {
  fn new(id: usize) -> Self {
    Self {
      id,
      wire_a: String::from("1"),
      at_a: String::from("1"),
      wire_b: String::from("1"),
      at_b: String::from("0"),
      z0: String::from("50"),
      length_m: String::from("5.3"),
      vf: String::from("1"),
    }
  }

  /// 转成核心类型；任一字段解析失败返回 `None`（该行被跳过）。
  pub(super) fn to_line(&self) -> Option<TransmissionLine> {
    Some(TransmissionLine {
      wire_a: self.wire_a.trim().parse::<usize>().ok()?,
      at_a: self.at_a.trim().parse::<f64>().ok()?,
      wire_b: self.wire_b.trim().parse::<usize>().ok()?,
      at_b: self.at_b.trim().parse::<f64>().ok()?,
      z0: self.z0.trim().parse::<f64>().ok()?,
      length_m: self.length_m.trim().parse::<f64>().ok()?,
      velocity_factor: self.vf.trim().parse::<f64>().ok()?,
    })
  }
}

/// 把表格里的行转成核心类型（解析不了的行直接跳过）。
pub(super) fn parsed(rows: &[LineRow]) -> Vec<TransmissionLine> {
  rows.iter().filter_map(LineRow::to_line).collect()
}

#[component]
pub(super) fn LinesSection(
  lines: RwSignal<Vec<LineRow>>,
  /// 增加一行时用的 id（与导线表共用同一个生成器）。
  new_id: Callback<(), usize>,
) -> impl IntoView {
  let add = move |_| {
    let id = new_id.run(());
    lines.update(|v| v.push(LineRow::new(id)));
  };

  view! {
    <div class="space-y-2 rounded-xl border bg-card p-4">
      <div class="flex flex-wrap items-center justify-between gap-2">
        <span class="text-sm font-semibold">{move || t("tools.nec-lines")}</span>
        <button
          type="button"
          class="cursor-pointer rounded-md border bg-background px-3 py-1 text-xs font-medium hover:bg-accent"
          on:click=add
        >
          {move || t("tools.nec-add-line")}
        </button>
      </div>
      <p class=NOTE>{move || t("tools.nec-lines-note")}</p>

      {move || {
        let rows = lines.get();
        if rows.is_empty() {
          return view! { <p class=NOTE>{move || t("tools.nec-no-lines")}</p> }.into_any();
        }
        let count = tf("tools.nec-line-count", &[&rows.len().to_string()]);
        let body = rows
          .into_iter()
          .enumerate()
          .map(|(idx, row)| {
            let id = row.id;
            let set = move |f: fn(&mut LineRow) -> &mut String, v: String| {
              lines.update(|rows| {
                if let Some(r) = rows.iter_mut().find(|r| r.id == id) {
                  *f(r) = v;
                }
              });
            };
            let text = |f: fn(&LineRow) -> &String| {
              let r = row.clone();
              Signal::derive(move || f(&r).clone())
            };
            // 注意：`t()` 的字面量必须写在**调用点**，走变量传进去静态扫描认不出来
            // （词条会被判成死条目，CI 直接红）。
            let label = move |base: String, idx: usize| {
              Signal::derive(move || format!("{base} {}", idx + 1))
            };
            view! {
              <tr>
                <td class=CELL>{idx + 1}</td>
                {field(
                  label(t("tools.nec-line-wire-a"), idx),
                  text(|r| &r.wire_a),
                  Callback::new(move |v: String| set(|r| &mut r.wire_a, v)),
                  "w-12",
                )}
                {field(
                  label(t("tools.nec-line-at-a"), idx),
                  text(|r| &r.at_a),
                  Callback::new(move |v: String| set(|r| &mut r.at_a, v)),
                  "w-16",
                )}
                {field(
                  label(t("tools.nec-line-wire-b"), idx),
                  text(|r| &r.wire_b),
                  Callback::new(move |v: String| set(|r| &mut r.wire_b, v)),
                  "w-12",
                )}
                {field(
                  label(t("tools.nec-line-at-b"), idx),
                  text(|r| &r.at_b),
                  Callback::new(move |v: String| set(|r| &mut r.at_b, v)),
                  "w-16",
                )}
                {field(
                  label(t("tools.nec-line-z0"), idx),
                  text(|r| &r.z0),
                  Callback::new(move |v: String| set(|r| &mut r.z0, v)),
                  "w-16",
                )}
                {field(
                  label(t("tools.nec-line-length"), idx),
                  text(|r| &r.length_m),
                  Callback::new(move |v: String| set(|r| &mut r.length_m, v)),
                  "w-16",
                )}
                {field(
                  label(t("tools.nec-line-vf"), idx),
                  text(|r| &r.vf),
                  Callback::new(move |v: String| set(|r| &mut r.vf, v)),
                  "w-14",
                )}
                <td class=CELL>
                  <button
                    type="button"
                    class="cursor-pointer rounded border px-1.5 py-0.5 text-xs hover:bg-accent"
                    aria-label=move || format!("{} {}", t("log.delete"), idx + 1)
                    on:click=move |_| lines.update(|v| v.retain(|r| r.id != id))
                  >
                    "×"
                  </button>
                </td>
              </tr>
            }
            .into_any()
          })
          .collect_view();
        view! {
          <div class="overflow-x-auto">
            <table data-slot="nec-lines" class="w-full min-w-[760px] border-collapse text-xs">
              <thead class="bg-muted/50">
                <tr>
                  <th class=format!("{CELL} text-left")>"#"</th>
                  <th class=format!("{CELL} text-left")>
                    {move || t("tools.nec-line-port-a")}
                  </th>
                  <th class=format!("{CELL} text-left")>
                    {move || t("tools.nec-line-port-b")}
                  </th>
                  <th class=format!("{CELL} text-left")>"Z0 (Ω)"</th>
                  <th class=format!("{CELL} text-left")>
                    {move || t("tools.nec-line-length")}
                  </th>
                  <th class=format!("{CELL} text-left")>
                    {move || t("tools.nec-line-vf")}
                  </th>
                  <th class=format!("{CELL} text-left")>{move || t("log.actions")}</th>
                </tr>
              </thead>
              <tbody>{body}</tbody>
            </table>
          </div>
          <p class=NOTE>{count}</p>
        }
        .into_any()
      }}
    </div>
  }
}
