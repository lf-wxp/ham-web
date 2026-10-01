use std::collections::{HashMap, HashSet};

use ham_web_core::qsl_labels::{LAYOUTS, Layout, build, layout, paginate};
use leptos::prelude::*;

use crate::pages::log::{LogEntry, use_log_store};
use crate::ui::{Size, Variant, button_class, input_class};
use crate::util::{set_title, storage, window};

use super::label_view::LabelView;

const LAYOUT_KEY: &str = "qsl-label-layout";

/// 通联范围。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
  /// 未寄出的通联。
  Unsent,
  /// 全部通联。
  All,
}

/// 打印时覆盖全局的页边距与纸张，标签坐标按纸张左上角计算。
fn page_css(l: &Layout) -> String {
  format!(
    "@media print {{ @page {{ size: {}; margin: 0; }} }}",
    l.paper.css()
  )
}

/// QSL 标签打印页。
#[component]
pub fn QslLabelsPage() -> impl IntoView {
  set_title("QSL 标签打印");
  let store = use_log_store();
  let layout_id =
    RwSignal::new(storage::get(LAYOUT_KEY).unwrap_or_else(|| LAYOUTS[0].id.to_owned()));
  let scope = RwSignal::new(Scope::Unsent);
  let since = RwSignal::new(String::new());
  let skip = RwSignal::new(0usize);

  let current = Memo::new(move |_| *layout(&layout_id.get()));

  let selected = Memo::new(move |_| {
    let (sc, from) = (scope.get(), since.get());
    store.logbook.with(|lb| {
      lb.entries
        .iter()
        .filter(|e| sc == Scope::All || !e.qsl_sent)
        .filter(|e| from.is_empty() || e.date >= from)
        .cloned()
        .collect::<Vec<_>>()
    })
  });

  let labels = Memo::new(move |_| {
    let l = current.get();
    selected.with(|list| build(&list.iter().collect::<Vec<_>>(), l.qsos_per_label))
  });

  let mark_sent = move || {
    let ids: HashSet<u64> = selected.with_untracked(|l| l.iter().map(|e| e.id).collect());
    if ids.is_empty() {
      return;
    }
    if !window()
      .confirm_with_message(&format!("把这 {} 条通联标记为「QSL 已寄出」？", ids.len()))
      .unwrap_or(false)
    {
      return;
    }
    store.logbook.update(|lb| {
      for e in lb.entries.iter_mut().filter(|e| ids.contains(&e.id)) {
        e.qsl_sent = true;
      }
    });
    store.persist();
  };

  let seg = |active: bool| {
    if active {
      "rounded-md bg-background px-2.5 py-1 text-xs font-medium shadow-sm"
    } else {
      "rounded-md px-2.5 py-1 text-xs text-muted-foreground hover:text-foreground"
    }
  };

  let sheets = move || {
    let l = current.get();
    let list = labels.get();
    if list.is_empty() {
      return view! {
        <p class="py-16 text-center text-sm text-muted-foreground">
          {if scope.get() == Scope::Unsent { "没有待寄出的通联。可切换到「全部」重新打印。" } else { "日志里还没有通联记录。" }}
        </p>
      }
      .into_any();
    }
    let entries: HashMap<u64, LogEntry> =
      selected.with(|s| s.iter().map(|e| (e.id, e.clone())).collect());
    let my_call = store
      .station
      .with(|s| s.callsign.trim().to_ascii_uppercase());
    let (w, h) = l.paper.size_mm();
    let compact = l.height < 30.0;
    paginate(list.len(), &l, skip.get())
      .into_iter()
      .enumerate()
      .map(|(p, slots)| {
        view! {
          <div
            class=if p == 0 { "relative mx-auto mb-6 overflow-hidden bg-white text-zinc-900 shadow-sm print:mb-0 print:shadow-none" } else { "print-break-before relative mx-auto mb-6 overflow-hidden bg-white text-zinc-900 shadow-sm print:mb-0 print:shadow-none" }
            style:width=format!("{w}mm")
            style:height=format!("{h}mm")
            aria-label=format!("第 {} 页", p + 1)
          >
            {slots.into_iter().enumerate().map(|(slot, idx)| {
              let (x, y) = l.origin(slot);
              let inner = idx.map(|i| view! {
                <LabelView label=list[i].clone() entries=entries.clone() my_call=my_call.clone() compact=compact />
              });
              view! {
                <div
                  class=if idx.is_some() { "absolute rounded-[2mm] border border-dashed border-zinc-300 print:border-transparent" } else { "absolute rounded-[2mm] border border-dashed border-zinc-200 bg-zinc-50 print:border-transparent print:bg-transparent" }
                  style:left=format!("{x}mm")
                  style:top=format!("{y}mm")
                  style:width=format!("{}mm", l.width)
                  style:height=format!("{}mm", l.height)
                  data-label=idx.map(|i| list[i].callsign.clone())
                >
                  {inner}
                </div>
              }
            }).collect_view()}
          </div>
        }
      })
      .collect_view()
      .into_any()
  };

  view! {
    <div class="min-h-screen bg-muted/40 pb-10 print:bg-white print:pb-0">
      <style>{move || page_css(&current.get())}</style>
      <div class="print-hide sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-x-4 gap-y-2 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">"QSL 标签打印"</h1>
            <div class="text-xs text-muted-foreground">"同一呼号的通联合并到一张标签，贴在 QSL 卡背面即可寄出"</div>
          </div>
          <a href="/log" class=button_class(Variant::Ghost, Size::Sm, "")>"返回日志"</a>
          <button
            type="button"
            class=button_class(Variant::Outline, Size::Sm, "")
            prop:disabled=move || selected.with(Vec::is_empty)
            on:click=move |_| mark_sent()
          >
            {move || format!("标记为已寄出（{}）", selected.with(Vec::len))}
          </button>
          <button
            type="button"
            class=button_class(Variant::Default, Size::Sm, "")
            prop:disabled=move || labels.with(Vec::is_empty)
            on:click=move |_| { let _ = window().print(); }
          >
            "打印"
          </button>
        </div>
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-x-4 gap-y-2 px-4 pb-3 text-xs text-muted-foreground">
          <div class="inline-flex rounded-lg bg-muted p-0.5" role="group" aria-label="通联范围">
            {[(Scope::Unsent, "未寄出"), (Scope::All, "全部")].into_iter().map(|(s, label)| view! {
              <button type="button" class=move || seg(scope.get() == s) aria-pressed=move || (scope.get() == s).to_string() on:click=move |_| scope.set(s)>
                {label}
              </button>
            }).collect_view()}
          </div>
          <label class="inline-flex items-center gap-1.5">
            "起始日期"
            <input type="date" class=input_class("h-7 w-auto py-0 text-xs") prop:value=move || since.get() on:input=move |e| since.set(event_target_value(&e)) />
          </label>
          <label class="inline-flex items-center gap-1.5">
            "标签纸"
            <select
              class=input_class("h-7 w-auto py-0 text-xs")
              on:change=move |e| {
                let v = event_target_value(&e);
                storage::set(LAYOUT_KEY, &v);
                layout_id.set(v);
                skip.update(|s| *s = (*s).min(current.get_untracked().per_page() - 1));
              }
            >
              {LAYOUTS.iter().map(|l| view! {
                <option value=l.id selected=move || layout_id.with(|id| id == l.id)>{l.name}</option>
              }).collect_view()}
            </select>
          </label>
          <label class="inline-flex items-center gap-1.5" title="用过一部分的标签纸：跳过前面已撕掉的标签">
            "跳过前"
            <input
              type="number"
              min="0"
              prop:max=move || (current.get().per_page() - 1).to_string()
              class=input_class("h-7 w-16 py-0 text-xs")
              prop:value=move || skip.get().to_string()
              on:input=move |e| {
                let max = current.get_untracked().per_page() - 1;
                skip.set(event_target_value(&e).parse::<usize>().unwrap_or(0).min(max));
              }
            />
            "张"
          </label>
          <span class="tabular-nums" aria-live="polite">
            {move || {
              let l = current.get();
              let n = labels.with(Vec::len);
              let pages = paginate(n, &l, skip.get()).len();
              format!("{n} 张标签 · {} 条通联 · {pages} 页", selected.with(Vec::len))
            }}
          </span>
        </div>
        {move || store.station.with(|s| s.callsign.trim().is_empty()).then(|| view! {
          <p class="mx-auto max-w-5xl px-4 pb-3 text-xs text-amber-800 dark:text-amber-300">
            "还没有设置本台呼号，标签上不会显示「From」。可到 "
            <a href="/log" class="font-medium underline underline-offset-4">"通联日志 → 本台信息"</a>
            " 填写。"
          </p>
        })}
      </div>

      <div class="print-sheet mt-6 overflow-x-auto px-4 print:mt-0 print:overflow-visible print:px-0">{sheets}</div>
      <p class="print-hide mt-2 text-center text-xs text-muted-foreground">"打印时请选择「实际大小 / 100%」并关闭页眉页脚，否则标签会错位。"</p>
    </div>
  }
}
