//! 设计存档面板：给当前模型起名保存、列出已存的设计、一键载回或删除。
//!
//! 数据模型与「存什么」在核心 [`ham_web_core::nec_designs`]；本组件只负责
//! 读写存储门面（`kv`）与界面。默认折叠：大多数时候用户只是在调参数。

use ham_web_core::nec::{Ground, NecInput, NecResult, TransmissionLine, solve_with_lines};
use ham_web_core::nec_designs::{CORRUPT_KEY, DESIGNS_KEY, DesignBook, SaveError, SavedModel};
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::ui::{
  Button, Field, Input, Size, Table, TableBody, TableCell, TableHead, TableHeader, TableRow,
  Variant,
};

use super::NOTE;

/// 对比视图：两份存档的求解结果与「能不能直接比」的判据。
#[derive(Debug, Clone, PartialEq)]
struct CompareView {
  a_name: String,
  a: NecResult,
  b_name: String,
  b: NecResult,
  a_freq: String,
  a_ground: String,
  b_freq: String,
  b_ground: String,
  /// 频率与地面都相同才可直接比。
  comparable: bool,
}

/// 地面模型的显示名（与求解页同一组词条）。
fn ground_label(ground: Ground) -> String {
  match ground {
    Ground::Free => t("tools.nec-ground-free"),
    Ground::Perfect => t("tools.nec-ground-perfect"),
    Ground::Lossy { .. } => t("tools.nec-ground-lossy"),
  }
}

/// 把存档册写进存储门面（localStorage 快照 + IndexedDB 权威）；序列化失败返回 `false`，
/// 此时什么都没写，调用方不能让界面显示「已保存」。
fn persist(book: &DesignBook) -> bool {
  let Ok(json) = serde_json::to_string(book) else {
    return false;
  };
  crate::kv::save_large(DESIGNS_KEY, &json);
  true
}

/// 读出持久化的存档册。解析失败时把原文另存到 [`CORRUPT_KEY`]（已有备份则不覆盖更早的那份）
/// 并提示 —— 直接当空册的话，用户下一次保存就把坏数据永久覆盖了。
fn read_book(json: &str, notice: RwSignal<String>) -> DesignBook {
  if json.trim().is_empty() {
    return DesignBook::default();
  }
  DesignBook::parse(json).unwrap_or_else(|| {
    if crate::kv::load_snapshot(CORRUPT_KEY).is_none() {
      crate::kv::save_large(CORRUPT_KEY, json);
    }
    let _ = notice.try_set(t("tools.nec-archive-corrupt"));
    DesignBook::default()
  })
}

#[component]
pub(super) fn DesignArchiveSection(
  /// 当前可求解的输入（导线表里有格子没填好时为 `None`）。
  current: Memo<Option<NecInput>>,
  /// 当前的传输线表（已解析）。
  lines: Signal<Vec<TransmissionLine>>,
  /// 载回一份设计：页面负责把它铺进各张表。
  on_load: Callback<(NecInput, Vec<TransmissionLine>, String), ()>,
) -> impl IntoView {
  let notice = RwSignal::new(String::new());
  // 首屏先用同步快照，随后按门面的版本仲裁校正（与日志 store 同一套读法）。
  let book = RwSignal::new(
    crate::kv::load_snapshot(DESIGNS_KEY)
      .map(|s| read_book(&s, notice))
      .unwrap_or_default(),
  );
  let dirty = StoredValue::new(false);
  wasm_bindgen_futures::spawn_local(async move {
    if let Some(json) = crate::kv::load_large(DESIGNS_KEY).await
      && !dirty.try_get_value().unwrap_or(true)
    {
      let _ = book.try_set(read_book(&json, notice));
    }
  });
  // 写入失败时不更新界面里的册子：界面与存储不一致，比一条错误提示更糟。
  let commit = move |next: DesignBook| -> bool {
    if !persist(&next) {
      notice.set(t("tools.nec-archive-save-failed"));
      return false;
    }
    dirty.set_value(true);
    book.set(next);
    true
  };

  let name = RwSignal::new(String::new());

  // 对比选中：最多两份，选满才求解（求解两次不便宜，不白跑）。
  let picked = RwSignal::new(Vec::<u64>::new());
  let toggle = move |id: u64| {
    picked.update(|list| {
      if let Some(pos) = list.iter().position(|x| *x == id) {
        list.remove(pos);
      } else {
        if list.len() == 2 {
          list.remove(0);
        }
        list.push(id);
      }
    });
  };

  let compare = Memo::new(move |_| {
    let ids = picked.get();
    let [a_id, b_id] = ids.as_slice() else {
      return None;
    };
    let book = book.get();
    let a = book.get(*a_id)?;
    let b = book.get(*b_id)?;
    let (a_input, a_lines) = a.model.to_input();
    let (b_input, b_lines) = b.model.to_input();
    let a_result = solve_with_lines(&a_input, &a_lines)?;
    let b_result = solve_with_lines(&b_input, &b_lines)?;
    // 频率或地面模型不同，增益 / 前后比就没有可比性：页面必须说出来，
    // 否则「八木 6.86 dBi < 偶极 7.96 dBi」会被读成八木更差（其实是地面模型不同）。
    let comparable = a_input.freq_hz == b_input.freq_hz && a_input.ground == b_input.ground;
    Some(CompareView {
      a_name: a.name.clone(),
      a: a_result,
      b_name: b.name.clone(),
      b: b_result,
      a_freq: format!("{:.3} MHz", a_input.freq_hz / 1_000_000.0),
      a_ground: ground_label(a_input.ground),
      b_freq: format!("{:.3} MHz", b_input.freq_hz / 1_000_000.0),
      b_ground: ground_label(b_input.ground),
      comparable,
    })
  });

  let save = move |_| {
    let Some(input) = current.get_untracked() else {
      notice.set(t("tools.nec-archive-unusable"));
      return;
    };
    let model = SavedModel::from_input(&input, &lines.get_untracked());
    let mut next = book.get_untracked();
    match next.save(&name.get_untracked(), model, &crate::util::local_today()) {
      Ok(_) => {
        let saved = next.designs[0].name.clone();
        if commit(next) {
          notice.set(tf("tools.nec-archive-saved", &[&saved]));
        }
      }
      Err(SaveError::EmptyName) => notice.set(t("tools.nec-archive-name-required")),
      Err(SaveError::Unusable) => notice.set(t("tools.nec-archive-unusable")),
      Err(SaveError::Full) => notice.set(t("tools.nec-archive-full")),
    }
  };

  view! {
    <details class="pxl-card border bg-card">
      <summary class="cursor-pointer px-4 py-3 text-sm font-semibold">
        {move || t("tools.nec-archive")}
      </summary>
      <div class="space-y-3 px-4 pb-4">
        <p class=NOTE>{move || t("tools.nec-archive-note")}</p>
        <div class="flex flex-wrap items-end gap-2">
          <Field label=Signal::derive(move || t("tools.nec-archive-name")) r#for="nec-archive-name">
            <Input
              id="nec-archive-name"
              value=name
              on_change=Callback::new(move |v: String| name.set(v))
              placeholder=Signal::derive(move || t("tools.nec-archive-name-placeholder"))
              class="w-56"
            />
          </Field>
          <Button variant=Variant::Default size=Size::Sm on_click=Callback::new(save)>
            {move || t("tools.nec-archive-save")}
          </Button>
        </div>
        {move || {
          let msg = notice.get();
          (!msg.is_empty()).then(|| view! { <p class=NOTE role="status">{msg}</p> })
        }}
        {move || {
          let designs = book.get().designs;
          if designs.is_empty() {
            return view! { <p class=NOTE>{move || t("tools.nec-archive-empty")}</p> }.into_any();
          }
          view! {
            <ul class="divide-y border" data-slot="nec-archive-list">
              {designs
                .into_iter()
                .map(|d| {
                  let id = d.id;
                  let title = d.name.clone();
                  let usable = d.model.is_usable();
                  let model = d.model.clone();
                  let load_name = d.name.clone();
                  let del_name = d.name.clone();
                  let pick_name = d.name.clone();
                  view! {
                    <li class="flex flex-wrap items-center gap-2 px-3 py-2 text-sm">
                      <span class="font-medium">{title}</span>
                      <span class="text-xs text-muted-foreground tabular-nums">{d.saved_at.clone()}</span>
                      <span class="ml-auto flex gap-2">
                        <Button
                          variant=Variant::Outline
                          size=Size::Xs
                          aria_label=Signal::derive({
                            let n = pick_name.clone();
                            move || {
                              if picked.get().contains(&id) {
                                tf("tools.nec-compare-unpick-named", &[&n])
                              } else {
                                tf("tools.nec-compare-pick-named", &[&n])
                              }
                            }
                          })
                          on_click=Callback::new(move |_| toggle(id))
                        >
                          {move || {
                            if picked.get().contains(&id) {
                              t("tools.nec-compare-unpick")
                            } else {
                              t("tools.nec-compare-pick")
                            }
                          }}
                        </Button>
                        <Button
                          variant=Variant::Outline
                          size=Size::Xs
                          disabled=Signal::derive(move || !usable)
                          aria_label=Signal::derive({
                            let n = load_name.clone();
                            move || tf("tools.nec-archive-load-named", &[&n])
                          })
                          on_click=Callback::new({
                            let n = load_name.clone();
                            move |_| {
                              let (input, lines) = model.to_input();
                              on_load.run((input, lines, n.clone()));
                            }
                          })
                        >
                          {move || t("tools.nec-archive-load")}
                        </Button>
                        <Button
                          variant=Variant::Outline
                          size=Size::Xs
                          aria_label=Signal::derive({
                            let n = del_name.clone();
                            move || tf("tools.nec-archive-delete-named", &[&n])
                          })
                          on_click=Callback::new({
                            let n = del_name.clone();
                            move |_| {
                              let ask = tf("tools.nec-archive-delete-confirm", &[&n]);
                              let label = tf("tools.nec-archive-delete-named", &[&n]);
                              crate::ui::confirm(
                                ask,
                                "",
                                label,
                                crate::ui::ConfirmTone::Danger,
                                Callback::new(move |_| {
                                  let mut next = book.get_untracked();
                                  if next.remove(id) {
                                    let _ = commit(next);
                                  }
                                }),
                              );
                            }
                          })
                        >
                          {move || t("log.delete")}
                        </Button>
                      </span>
                    </li>
                  }
                })
                .collect_view()}
            </ul>
            <div class="space-y-2" data-slot="nec-compare">
              {move || {
                let Some(view) = compare.get() else {
                  return view! {
                    <p class=NOTE>{move || t("tools.nec-compare-hint")}</p>
                  }
                    .into_any();
                };
                let rows = [
                  (
                    t("tools.nec-compare-freq-label"),
                    view.a_freq.clone(),
                    view.b_freq.clone(),
                    "—".to_owned(),
                  ),
                  (
                    t("tools.nec-ground"),
                    view.a_ground.clone(),
                    view.b_ground.clone(),
                    "—".to_owned(),
                  ),
                  (
                    t("tools.nec-compare-gain-label"),
                    format!("{:.2} dBi", view.a.gain_max_dbi),
                    format!("{:.2} dBi", view.b.gain_max_dbi),
                    format!("{:+.2}", view.b.gain_max_dbi - view.a.gain_max_dbi),
                  ),
                  (
                    t("tools.nec-compare-f2b-label"),
                    format!("{:.1} dB", view.a.front_to_back_db),
                    format!("{:.1} dB", view.b.front_to_back_db),
                    format!("{:+.1}", view.b.front_to_back_db - view.a.front_to_back_db),
                  ),
                  (
                    t("tools.nec-compare-swr-label"),
                    format!("{:.2}", view.a.swr_50),
                    format!("{:.2}", view.b.swr_50),
                    format!("{:+.2}", view.b.swr_50 - view.a.swr_50),
                  ),
                  (
                    t("tools.nec-compare-eff-label"),
                    format!("{:.1}%", view.a.efficiency * 100.0),
                    format!("{:.1}%", view.b.efficiency * 100.0),
                    format!("{:+.1}", (view.b.efficiency - view.a.efficiency) * 100.0),
                  ),
                  (
                    t("tools.nec-compare-elev-label"),
                    format!("{:.0}°", view.a.gain_max_elevation_deg),
                    format!("{:.0}°", view.b.gain_max_elevation_deg),
                    format!(
                      "{:+.0}",
                      view.b.gain_max_elevation_deg - view.a.gain_max_elevation_deg
                    ),
                  ),
                ];
                let body = rows
                  .into_iter()
                  .map(|(label, av, bv, dv)| {
                    view! {
                      <TableRow>
                        <TableCell class="border px-3 py-2 align-top">{label}</TableCell>
                        <TableCell class="border px-3 py-2 align-top whitespace-nowrap tabular-nums">
                          {av}
                        </TableCell>
                        <TableCell class="border px-3 py-2 align-top whitespace-nowrap tabular-nums">
                          {bv}
                        </TableCell>
                        <TableCell class="border px-3 py-2 align-top whitespace-nowrap tabular-nums text-muted-foreground">
                          {dv}
                        </TableCell>
                      </TableRow>
                    }
                  })
                  .collect_view();
                let a_head = view.a_name.clone();
                let b_head = view.b_name.clone();
                let mismatch = !view.comparable;
                view! {
                  <div class="space-y-2">
                    <p class=NOTE>{tf("tools.nec-compare-title", &[&view.a_name, &view.b_name])}</p>
                    {mismatch
                      .then(|| {
                        view! {
                          <p class="text-xs text-amber-700 dark:text-amber-300">
                            {move || t("tools.nec-compare-model-mismatch")}
                          </p>
                        }
                      })}
                    <Table class="min-w-[520px] text-sm">
                      <TableHead>
                        <TableRow>
                          <TableHeader class="border px-3 py-2">
                            {move || t("tools.nec-compare-metric")}
                          </TableHeader>
                          <TableHeader class="border px-3 py-2">{a_head}</TableHeader>
                          <TableHeader class="border px-3 py-2">{b_head}</TableHeader>
                          <TableHeader class="border px-3 py-2">
                            {move || t("tools.nec-compare-delta")}
                          </TableHeader>
                        </TableRow>
                      </TableHead>
                      <TableBody>{body}</TableBody>
                    </Table>
                  </div>
                }
                  .into_any()
              }}
            </div>
          }
          .into_any()
        }}
      </div>
    </details>
  }
}
