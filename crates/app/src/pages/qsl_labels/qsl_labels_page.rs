use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use ham_web_core::qsl_labels::{LAYOUTS, Layout, build, layout, paginate};
use ham_web_core::qsl_status::{QslVia, mark_sent};
use leptos::prelude::*;

use crate::components::common::PageHeader;
use crate::pages::log::{LogEntry, use_log_store};
use crate::ui::{
  Button, ButtonLink, Chip, ChipGroup, ControlSize, DatePicker, NativeSelect, NumberField,
  SelectOption, Size, Variant,
};
use crate::util::{set_title, storage, window};

use super::label_view::LabelView;
use crate::i18n::{t, tf, tp};

const LAYOUT_KEY: &str = "qsl-label-layout";

/// 通联范围。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
  /// 未寄出的通联。
  Unsent,
  /// 全部通联。
  All,
}

impl Scope {
  /// 短键：`Chip` 的 `value` 是字符串，需要一个稳定的往返写法。
  fn key(self) -> &'static str {
    match self {
      Self::Unsent => "unsent",
      Self::All => "all",
    }
  }

  /// 由短键还原；认不出时按「未寄出」处理（与界面上默认那档一致）。
  fn from_key(key: &str) -> Self {
    if key == "all" {
      Self::All
    } else {
      Self::Unsent
    }
  }
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
  set_title("shell.qsl-label-printing");
  let store = use_log_store();
  let layout_id =
    RwSignal::new(storage::get(LAYOUT_KEY).unwrap_or_else(|| LAYOUTS[0].id.to_owned()));
  let scope = RwSignal::new(Scope::Unsent);
  let since = RwSignal::new(String::new());
  let skip = RwSignal::new(0usize);
  // 「标记为已寄出」时一并记下寄出方式（QSL_SENT_VIA）：卡片局与直寄的回收周期
  // 差一个数量级，追卡时要看的就是这个。默认卡片局（群里最常用的方式）。
  let via_sel = RwSignal::new(QslVia::Bureau.key().to_owned());

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
      .confirm_with_message(&tp(
        "log.mark-these-qsos-as",
        ids.len(),
        &[&ids.len().to_string()],
      ))
      .unwrap_or(false)
    {
      return;
    }
    let via = QslVia::from_key(&via_sel.get_untracked());
    let mut changed = false;
    store.logbook.update(|lb| {
      for e in lb.entries.iter_mut().filter(|e| ids.contains(&e.id)) {
        changed |= mark_sent(e, via);
      }
    });
    // 全部都已经寄出且方式相同 → 没有任何改动，不必落库。
    if changed {
      store.persist();
    }
  };

  let sheets = move || {
    let l = current.get();
    let list = labels.get();
    if list.is_empty() {
      return view! {
        <p class="py-16 text-center text-sm text-muted-foreground">
          {if scope.get() == Scope::Unsent { t("log.no-qsos-waiting-to") } else { t("log.no-qsos-in-the") }}
        </p>
      }
      .into_any();
    }
    // 每次渲染只建一份 `id → LogEntry` 表，用 `Arc` 共享给各标签槽；
    // 否则每个槽都要 clone 一遍整表（见 `LabelView` 的说明）。
    let entries: Arc<HashMap<u64, LogEntry>> =
      Arc::new(selected.with(|s| s.iter().map(|e| (e.id, e.clone())).collect()));
    let my_call = store.active_station().callsign.trim().to_ascii_uppercase();
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
            data-slot="label-page"
            aria-label=tf("log.page-2", &[&(p + 1).to_string()])
          >
            {slots.into_iter().enumerate().map(|(slot, idx)| {
              let (x, y) = l.origin(slot);
              let inner = idx.map(|i| view! {
                <LabelView label=list[i].clone() entries=Arc::clone(&entries) my_call=my_call.clone() compact=compact />
              });
              view! {
                <div
                  data-slot="label-slot"
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

  let via_options: Vec<SelectOption> = [QslVia::Bureau, QslVia::Direct]
    .into_iter()
    .map(|via| {
      let key = via.key();
      let label = Signal::derive(move || match via {
        QslVia::Bureau => t("log.qsl-via-bureau"),
        QslVia::Direct => t("log.qsl-via-direct"),
        QslVia::None => t("log.not-sent"),
      });
      SelectOption::new(key, label)
    })
    .collect();
  let layout_options: Vec<SelectOption> = LAYOUTS
    .iter()
    .map(|l| SelectOption::new(l.id, l.name))
    .collect();

  view! {
    <div class="min-h-screen bg-muted/40 pb-10 print:bg-white print:pb-0">
      <style>{move || page_css(&current.get())}</style>
      <PageHeader
        class="print-hide"
        title=move || t("shell.qsl-label-printing")
        subtitle=move || t("log.qsos-with-the-same")
        actions=ViewFn::from(move || {
          view! {
            <ButtonLink
              href="/log"
              variant=Variant::Ghost
              size=Size::Sm
            >{move || t("log.back-to-log")}</ButtonLink>
            <NativeSelect
              value=via_sel
              on_change=Callback::new(move |v: String| via_sel.set(v))
              options=via_options.clone()
              size=ControlSize::Sm
              aria_label=Signal::derive(move || t("log.qsl-sent-via"))
              class="w-auto"
            />
            <Button
              variant=Variant::Outline
              size=Size::Sm
              disabled=Signal::derive(move || selected.with(Vec::is_empty))
              on_click=Callback::new(move |_| mark_sent())
            >
              {move || tf("log.mark-as-sent", &[&selected.with(Vec::len).to_string()])}
            </Button>
            <Button
              variant=Variant::Default
              size=Size::Sm
              disabled=Signal::derive(move || labels.with(Vec::is_empty))
              on_click=Callback::new(move |_| { let _ = window().print(); })
            >
              {move || t("learning.print")}
            </Button>
          }
        })
      >
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-x-4 gap-y-2 px-4 pb-3 text-xs text-muted-foreground">
          // 互斥选择用 `ChipGroup` + `Chip`（`role="radiogroup"` / `aria-checked`）：
          // 手写的 `aria-pressed` 分段控件不在规范内，读屏与键盘行为也没有这层保证。
          <ChipGroup
            value=Signal::derive(move || scope.get().key().to_owned())
            on_change=Callback::new(move |v: String| scope.set(Scope::from_key(&v)))
            aria_label=Signal::derive(move || t("log.qso-scope"))
          >
            <Chip value=Scope::Unsent.key()>{move || t("未寄出")}</Chip>
            <Chip value=Scope::All.key()>{move || t("全部")}</Chip>
          </ChipGroup>
          <label class="inline-flex items-center gap-1.5">
            {move || t("log.start-date")}
            <DatePicker
              value=since
              on_change=Callback::new(move |v: String| since.set(v))
              size=ControlSize::Sm
              aria_label=Signal::derive(move || t("log.start-date"))
              class="w-auto"
            />
          </label>
          <label class="inline-flex items-center gap-1.5">
            {move || t("log.label-sheet")}
            <NativeSelect
              value=layout_id
              on_change=Callback::new(move |v: String| {
                storage::set(LAYOUT_KEY, &v);
                layout_id.set(v);
                skip.update(|s| *s = (*s).min(current.get_untracked().per_page() - 1));
              })
              options=layout_options
              size=ControlSize::Sm
              aria_label=Signal::derive(move || t("log.label-sheet"))
              class="w-auto"
            />
          </label>
          <label class="inline-flex items-center gap-1.5" title=move || t("log.partly-used-sheet-skip")>
            {move || t("log.skip-first")}
            <NumberField
              value=Signal::derive(move || skip.get().to_string())
              on_change=Callback::new(move |v: String| {
                let max = current.get_untracked().per_page() - 1;
                skip.set(v.trim().parse::<usize>().unwrap_or(0).min(max));
              })
              min=0.0
              size=ControlSize::Sm
              class="w-16 px-1.5"
              aria_label=Signal::derive(move || t("log.skip-first"))
              controls=false
            />
            {move || t("log.labels")}
          </label>
          <span class="tabular-nums" aria-live="polite">
            {move || {
              let l = current.get();
              let n = labels.with(Vec::len);
              let qsos = selected.with(Vec::len);
              let pages = paginate(n, &l, skip.get()).len();
              // 一句话里三个可数名词：`tp` 只吃一个 count，故外层模板只留 `{}` 承接，
              // 每段各自 `tp` 出「1 张标签 / 1 条通联 / 1 页」（见 docs 的 P3-C5）。
              tf(
                "log.labels-qsos-pages",
                &[
                  &tp("log.n-labels", n, &[&n.to_string()]),
                  &tp("log.n-qsos", qsos, &[&qsos.to_string()]),
                  &tp("log.n-pages", pages, &[&pages.to_string()]),
                ],
              )
            }}
          </span>
        </div>
        {move || store.active_station().callsign.trim().is_empty().then(|| view! {
          <p class="mx-auto max-w-5xl px-4 pb-3 text-xs text-amber-800 dark:text-amber-300">
            {move || t("log.station-callsign-not-set")}
            " "
            <a href="/log" class="font-medium underline underline-offset-4">{move || t("contest.qso-log-station-info")}</a>
            " "
            {move || t("contest.to-fill-it-in")}
          </p>
        })}
      </PageHeader>

      <div class="print-sheet mt-6 overflow-x-auto px-4 print:mt-0 print:overflow-visible print:px-0">{sheets}</div>
      <p class="print-hide mt-2 text-center text-xs text-muted-foreground">{move || t("log.when-printing-choose-actual")}</p>
    </div>
  }
}
