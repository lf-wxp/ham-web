//! 集总负载（`LD` 卡）表：在导线的指定位置串入 R / L / C。

use leptos::prelude::*;

use crate::i18n::t;
use crate::ui::{ControlSize, Input};

use super::LoadRow;

/// 负载表。空表时显示一句提示，避免「什么都没有」被当成加载失败。
#[component]
pub(super) fn LoadTable(
  loads: RwSignal<Vec<LoadRow>>,
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
    <div class="overflow-x-auto rounded-xl border bg-card">
      <div class="flex items-center justify-between border-b px-4 py-3">
        <span class="text-sm font-semibold">{move || t("tools.nec-loads")}</span>
        <button
          type="button"
          class="cursor-pointer rounded-md border bg-background px-2 py-1 text-xs font-medium hover:bg-accent"
          on:click=move |_| on_add.run(())
        >
          {move || t("tools.nec-add-load")}
        </button>
      </div>
      {move || {
        if loads.get().is_empty() {
          view! {
            <p class="px-4 py-3 text-xs text-muted-foreground">
              {move || t("tools.nec-no-loads")}
            </p>
          }
          .into_any()
        } else {
          view! {
            <table data-slot="nec-loads" class="w-full border-collapse text-xs">
              <thead class="bg-muted/50">
                <tr>
                  <th class="border px-2 py-1 text-left">{move || t("tools.nec-load-wire-index")}</th>
                  <th class="border px-2 py-1 text-left">{move || t("tools.nec-load-position")}</th>
                  <th class="border px-2 py-1 text-left">"R (Ω)"</th>
                  <th class="border px-2 py-1 text-left">"L (µH)"</th>
                  <th class="border px-2 py-1 text-left">"C (pF)"</th>
                  <th class="border px-2 py-1 text-left">{move || t("log.actions")}</th>
                </tr>
              </thead>
              <tbody>
                <For each=move || loads.get() key=|r| r.id let:row>
                  {
                    let id = row.id;
                    let idx = move || {
                      loads.with(|rs| rs.iter().position(|r| r.id == id).unwrap_or(0))
                    };
                    let label = |f: String| format!("{f} {}", idx() + 1);
                    view! {
                      <tr>
                        <td class="border p-1">{cell!(loads, id, wire, label(t("tools.nec-load-wire-index")))}</td>
                        <td class="border p-1">{cell!(loads, id, at, label(t("tools.nec-load-position")))}</td>
                        <td class="border p-1">{cell!(loads, id, r_ohm, label(t("tools.nec-load-r")))}</td>
                        <td class="border p-1">{cell!(loads, id, l_uh, label(t("tools.nec-load-l")))}</td>
                        <td class="border p-1">{cell!(loads, id, c_pf, label(t("tools.nec-load-c")))}</td>
                        <td class="border px-2 py-1">
                          <button
                            type="button"
                            class="cursor-pointer rounded border px-1.5 py-0.5 text-xs hover:bg-accent"
                            aria-label=move || format!("{} {}", t("log.delete"), idx() + 1)
                            on:click=move |_| on_remove.run(id)
                          >
                            "×"
                          </button>
                        </td>
                      </tr>
                    }
                  }
                </For>
              </tbody>
            </table>
          }
          .into_any()
        }
      }}
    </div>
  }
}
