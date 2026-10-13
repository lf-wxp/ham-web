//! 导线表：逐格可编辑的几何（起点 / 终点 / 半径 / 段数）。
//!
//! 表与画布（[`super::canvas::WireCanvas`]）共用同一份状态：在这里改一格，画布立刻跟着动。

use leptos::prelude::*;

use crate::i18n::t;
use crate::ui::{Button, ControlSize, Input, Size, Variant};

use super::WireRow;

/// 导线表。增删由页面提供：新增要沿「垂直于最后一根导线」的方向平移，逻辑属于页面。
#[component]
pub(super) fn WireTable(
  wires: RwSignal<Vec<WireRow>>,
  on_add: Callback<(), ()>,
  on_remove: Callback<usize, ()>,
) -> impl IntoView {
  // 表格单元：一个受控输入框，按键值定位要改的行，避免整表重建（保住光标）。
  macro_rules! cell {
    ($rows:expr, $id:expr, $field:ident, $label:expr) => {
      view! {
        <Input
          value=Signal::derive(move || {
            $rows.with(|rs| {
              rs.iter()
                .find(|r| r.id == $id)
                .map_or_else(String::new, |r| r.$field.clone())
            })
          })
          on_change=Callback::new(move |v: String| {
            $rows.update(|rs| {
              if let Some(r) = rs.iter_mut().find(|r| r.id == $id) {
                r.$field = v;
              }
            });
          })
          size=ControlSize::Sm
          aria_label=$label
          class="w-full"
        />
      }
    };
  }

  view! {
    <div class="overflow-x-auto pxl-card border bg-card">
      <div class="flex items-center justify-between border-b px-4 py-3">
        <span class="text-sm font-semibold">{move || t("tools.nec-wire-geometry")}</span>
        <Button
          variant=Variant::Outline
          size=Size::Xs
          on_click=Callback::new(move |_| on_add.run(()))
        >
          {move || t("tools.nec-add-wire")}
        </Button>
      </div>
      <table data-slot="nec-wires" class="w-full border-collapse text-xs">
        <thead class="bg-muted/50">
          <tr>
            <th class="border px-2 py-1 text-left">"#"</th>
            <th class="border px-2 py-1 text-left">{move || t("tools.nec-start-point")}</th>
            <th class="border px-2 py-1 text-left">{move || t("tools.nec-end-point")}</th>
            <th class="border px-2 py-1 text-left">{move || t("tools.nec-radius-mm")}</th>
            <th class="border px-2 py-1 text-left">{move || t("tools.nec-segments-count")}</th>
            <th class="border px-2 py-1 text-left">{move || t("log.actions")}</th>
          </tr>
        </thead>
        <tbody>
          <For each=move || wires.get() key=|r| r.id let:row>
            {
              let id = row.id;
              // 标签走 `t()` 字面量：既翻译，也能被静态扫描采集到。
              let label = |f: String, i: usize| format!("{f} {}", i + 1);
              let idx = move || {
                wires.with(|rs| rs.iter().position(|r| r.id == id).unwrap_or(0))
              };
              view! {
                <tr>
                  <td class="border px-2 py-1 tabular-nums">{move || idx() + 1}</td>
                  <td class="border p-1">
                    <div class="flex gap-1">
                      {cell!(wires, id, ax, label(t("tools.nec-wire-start-x"), idx()))}
                      {cell!(wires, id, ay, label(t("tools.nec-wire-start-y"), idx()))}
                      {cell!(wires, id, az, label(t("tools.nec-wire-start-z"), idx()))}
                    </div>
                  </td>
                  <td class="border p-1">
                    <div class="flex gap-1">
                      {cell!(wires, id, bx, label(t("tools.nec-wire-end-x"), idx()))}
                      {cell!(wires, id, by, label(t("tools.nec-wire-end-y"), idx()))}
                      {cell!(wires, id, bz, label(t("tools.nec-wire-end-z"), idx()))}
                    </div>
                  </td>
                  <td class="border p-1">{cell!(wires, id, radius_mm, label(t("tools.nec-wire-radius-mm"), idx()))}</td>
                  <td class="border p-1">{cell!(wires, id, segments, label(t("tools.nec-segments-count"), idx()))}</td>
                  <td class="border px-2 py-1">
                    <Button
                      variant=Variant::Outline
                      size=Size::IconSm
                      aria_label=Signal::derive(move || format!("{} {}", t("log.delete"), idx() + 1))
                      on_click=Callback::new(move |_| on_remove.run(id))
                    >
                      "×"
                    </Button>
                  </td>
                </tr>
              }
            }
          </For>
        </tbody>
      </table>
    </div>
  }
}
