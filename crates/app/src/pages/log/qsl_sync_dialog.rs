//! 同步 QSL 确认：把 LoTW / eQSL / Club Log / QRZ 的确认报告与本地日志做**差异分析**，
//! 由人逐条裁决后再落库（差异怎么算、为什么不能照单全收，见 `ham_web_core::qsl_sync`
//! 的模块文档）。
//!
//! 界面上只有两个决定：**来源**（决定报告在说哪些渠道的事）与**每行的两个勾**
//! （补上远端新增 / 以远端为准）。默认只勾「补上」，因为「以远端为准」会清掉本地已有的
//! 确认，是这里唯一的破坏性操作，必须由人逐条确认。

use std::collections::BTreeSet;

use ham_web_core::qsl_sync::{
  Channel, QslDiff, QslFlag, QslSource, QslSyncResult, QsoDecision, apply_diff, diff_report,
};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::i18n::{t, tf};
use crate::ui::{
  Button, Checkbox, Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle, FileInput,
  RadioGroup, RadioGroupItem, Size, Textarea, Variant,
};

use super::use_log_store;

const HINT: &str = "text-xs text-muted-foreground";
const TH: &str = "border-b px-2 py-1.5 text-left font-medium text-muted-foreground";

/// 差异表里显示的本地缺失行数上限（再多就只报总数）。
const MISSING_PREVIEW: usize = 8;

/// 渠道名。
///
/// 与来源名共用同一批 key：中文释义全库唯一（反向索引按中文原文找 key），两个 key 用
/// 同一个词会直接让 `cargo make check` 变红。
fn channel_label(channel: Channel) -> String {
  match channel {
    Channel::Paper => t("log.qsl-channel-paper"),
    Channel::Lotw => t("log.qsl-channel-lotw"),
    Channel::Eqsl => t("log.qsl-channel-eqsl"),
    Channel::Qrz => t("log.qsl-channel-qrz"),
  }
}

/// 来源名。
fn source_label(source: QslSource) -> String {
  match source {
    QslSource::Lotw => t("log.qsl-channel-lotw"),
    QslSource::Eqsl => t("log.qsl-channel-eqsl"),
    QslSource::ClubLog => t("log.qsl-channel-clublog"),
    QslSource::Qrz => t("log.qsl-channel-qrz"),
    QslSource::Other => t("log.qsl-channel-other"),
  }
}

/// 标志位文案（全部复用日志页已有的那批 key，不另起一套）。
fn flag_label(flag: QslFlag) -> String {
  match flag {
    QslFlag::PaperSent => t("log.sent"),
    QslFlag::PaperRcvd => t("log.qsl-received"),
    QslFlag::LotwSent => t("log.lotw-uploaded"),
    QslFlag::LotwRcvd => t("log.lotw-confirmed"),
    QslFlag::EqslSent => t("log.eqsl-sent"),
    QslFlag::EqslRcvd => t("log.eqsl-confirmed"),
    QslFlag::QrzRcvd => t("log.qrz-confirmed"),
  }
}

/// 一组标志位的可读文案（`LoTW 已确认 · eQSL 已寄出`）。
fn flags_text(flags: &[QslFlag]) -> String {
  flags
    .iter()
    .map(|f| flag_label(*f))
    .collect::<Vec<_>>()
    .join(" · ")
}

/// 翻一个勾选集合。
fn toggle(set: RwSignal<BTreeSet<u64>>, id: u64, on: bool) {
  set.update(|s| {
    if on {
      s.insert(id);
    } else {
      s.remove(&id);
    }
  });
}

/// 同步 QSL 确认：选来源 → 粘贴 / 上传报告 → 分析差异 → 逐条裁决后应用。
#[component]
pub fn QslSyncDialog(open: RwSignal<bool>) -> impl IntoView {
  let store = use_log_store();
  let logbook = store.logbook;
  let source = RwSignal::new(QslSource::Lotw.id().to_owned());
  let text = RwSignal::new(String::new());
  let diff = RwSignal::new(None::<QslDiff>);
  let applied = RwSignal::new(None::<QslSyncResult>);
  // 每行的两个勾：默认只勾「补上远端新增」。
  let pick_added = RwSignal::new(BTreeSet::<u64>::new());
  let pick_mirror = RwSignal::new(BTreeSet::<u64>::new());
  // 上传报告是异步读盘：`await` 期间对话框可能已被卸载，而信号随 Owner 一起释放，
  // 再 `set` 会 panic（见 `util::mount_guard`）。文本本来也要靠人再点一次「分析」，
  // 读完后发现已卸载就直接丢掉，不影响任何流程。
  let alive = crate::util::mount_guard();

  // 按当前来源与文本重新算一遍差异，并把勾选重置成默认值。
  let analyze = move || {
    let adif = text.get_untracked();
    if adif.trim().is_empty() {
      crate::util::alert(&t("log.paste-or-upload-a"));
      return false;
    }
    let src = QslSource::from_id(&source.get_untracked());
    let d = logbook.with_untracked(|lb| diff_report(&lb.entries, &adif, src));
    // 解析出 0 条**不是**「无差异」：选错文件（PDF、Excel、空报告）也会走到这里，
    // 而界面上那句「没有需要处理的差异」会让人以为同步已经成功了。
    if d.total == 0 {
      diff.set(None);
      pick_added.set(BTreeSet::new());
      pick_mirror.set(BTreeSet::new());
      crate::util::alert(&t("log.diff-report-empty"));
      return false;
    }
    // 文本被截断（有 `<` 没有 `>`）：条数可能不全。仍然让人继续（部分同步也有用），
    // 但必须说清楚，免得把「报告就这么长」当成「没有差异」。
    if d.truncated {
      crate::util::alert(&t("log.diff-report-truncated"));
    }
    pick_added.set(
      d.records
        .iter()
        .filter(|r| !r.added.is_empty())
        .map(|r| r.entry_id)
        .collect(),
    );
    pick_mirror.set(BTreeSet::new());
    diff.set(Some(d));
    true
  };

  let on_analyze = Callback::new(move |()| {
    applied.set(None);
    analyze();
  });

  let on_apply = Callback::new(move |()| {
    // 没分析过就点应用：与「没粘报告就点分析」同一条提示（老对话框也是这个行为）。
    if diff.get_untracked().is_none() && !analyze() {
      return;
    }
    let Some(d) = diff.get_untracked() else {
      return;
    };
    let (added, mirrored) = (pick_added.get_untracked(), pick_mirror.get_untracked());
    let mut result = QslSyncResult::default();
    logbook.update(|lb| {
      result = apply_diff(&mut lb.entries, &d, |q| QsoDecision {
        apply_added: added.contains(&q.entry_id),
        prefer_remote: mirrored.contains(&q.entry_id),
      });
    });
    store.persist();
    applied.set(Some(result));
    // 应用后立刻重算：表格只剩没勾的那些差异，用户能马上看到「已经一致了」。
    analyze();
  });

  let upload = Callback::new(move |files: Vec<web_sys::File>| {
    let Some(file) = files.into_iter().next() else {
      return;
    };
    let alive = alive.clone();
    spawn_local(async move {
      // 读失败必须出声：静默什么都不做，用户会以为文件已经读进去了。
      match crate::util::read_file_text(&file).await {
        Some(s) => {
          if !alive() {
            return;
          }
          text.set(s);
          applied.set(None);
          diff.set(None);
        }
        None => crate::util::alert(&t("common.file-read-failed")),
      }
    });
  });

  view! {
    <Dialog open=open class="max-w-4xl">
      <DialogHeader>
        <DialogTitle>{move || t("log.sync-qsl-confirmations")}</DialogTitle>
        <DialogDescription>{move || t("log.paste-an-adif-confirmation")}</DialogDescription>
      </DialogHeader>
      <div class="space-y-3">
        // ── 来源 ──────────────────────────────────────────────
        <div class="flex flex-col gap-1.5">
          <span class=HINT>{move || t("log.qsl-source")}</span>
          <RadioGroup
            value=source
            on_change=Callback::new(move |v: String| {
              source.set(v);
              // 换来源会改变「哪些字段算数」，已算出的差异作废，避免误勾。
              diff.set(None);
              applied.set(None);
            })
            aria_label=Signal::derive(move || t("log.qsl-source"))
            class="flex flex-wrap items-center gap-4"
          >
            {QslSource::ALL
              .into_iter()
              .map(|s| {
                view! {
                  <label class="flex cursor-pointer items-center gap-2 text-sm">
                    <RadioGroupItem value=s.id().to_owned() />
                    {move || source_label(s)}
                  </label>
                }
              })
              .collect_view()}
          </RadioGroup>
          <p class=HINT>
            {move || {
              let s = QslSource::from_id(&source.get());
              if s == QslSource::Qrz {
                // QRZ 的确认没有标准 ADIF 字段，走本工具的扩展字段 —— 这段必须说清楚，
                // 否则用户会奇怪「为什么这份报告什么也没改」。
                t("log.qsl-source-qrz")
              } else {
                let names = s
                  .channels()
                  .iter()
                  .map(|c| channel_label(*c))
                  .collect::<Vec<_>>()
                  .join(" · ");
                tf("log.qsl-source-channels", &[&names])
              }
            }}
          </p>
        </div>

        // ── 报告 ──────────────────────────────────────────────
        <Textarea
          value=text
          on_change=Callback::new(move |v: String| {
            text.set(v);
            applied.set(None);
            diff.set(None);
          })
          placeholder=Signal::derive(move || t("log.paste-adif-report-here"))
          aria_label=Signal::derive(move || t("log.paste-adif-report-here"))
          class="h-28 font-mono tabular-nums"
        />
        <div class="flex flex-wrap items-center gap-2">
          <FileInput
            accept=".adi,.adif,.txt"
            label=t("log.upload-report-file")
            variant=Variant::Outline
            size=Size::Sm
            on_files=upload
          />
          <Button variant=Variant::Default size=Size::Sm on_click=on_analyze>
            {move || t("log.analyze-report")}
          </Button>
          {move || {
            applied
              .get()
              .map(|r| {
                view! {
                  <span class=HINT>
                    {tf(
                      "log.diff-applied",
                      &[&r.flags_applied.to_string(), &r.flags_cleared.to_string()],
                    )}
                  </span>
                }
              })
          }}
        </div>

        // ── 还没有报告的人：把「签名上传」那一段接上 ──────────────
        <p class=HINT>
          {move || t("log.no-report-yet-sign")}
          " "
          <a href="/eqsl" class="underline underline-offset-2 hover:text-foreground">
            {move || t("log.lotw-upload-guide")}
          </a>
        </p>

        // ── 差异 ──────────────────────────────────────────────
        {move || {
          diff.get().map(|d| {
            let missing = d.missing.clone();
            let shown = missing.iter().take(MISSING_PREVIEW).map(|m| {
              format!("{} · {} {} · {} · {}", m.callsign, m.date, m.time, m.band, m.mode)
            }).collect::<Vec<_>>();
            view! {
              <div class="space-y-2">
                <p class=HINT>
                  {tf(
                    "log.diff-summary",
                    &[
                      &d.total.to_string(),
                      &d.matched.to_string(),
                      &d.records.len().to_string(),
                      &d.missing_count.to_string(),
                    ],
                  )}
                </p>
                <p class=HINT>{move || t("log.diff-conflict-hint")}</p>
                // 报告里没有任何可采纳的标记（例如 QRZ 官方导出不带扩展字段）：
                // 说明「为什么一条都没得改」，否则用户只会看到一张空表。
                {(d.analyze_only)
                  .then(|| view! { <p class="text-xs">{move || t("log.qsl-source-analysis-only")}</p> })}

                {(d.records.is_empty())
                  .then(|| {
                    view! {
                      <p class="text-xs">{move || t("log.diff-none")}</p>
                    }
                  })}

                {(!d.records.is_empty())
                  .then(|| {
                    view! {
                      <div class="max-h-80 overflow-y-auto rounded-lg border">
                        <table class="w-full border-collapse text-xs">
                          <thead>
                            <tr>
                              <th class=TH>{move || t("log.callsign")}</th>
                              <th class=TH>{move || t("log.diff-added")}</th>
                              <th class=TH>{move || t("log.diff-conflicts")}</th>
                              <th class=TH>{move || t("log.diff-apply-added")}</th>
                              <th class=TH>{move || t("log.diff-prefer-remote")}</th>
                            </tr>
                          </thead>
                          <tbody>
                            {d
                              .records
                              .iter()
                              .map(|q| {
                                let id = q.entry_id;
                                let can_add = !q.added.is_empty();
                                let has_conflict = !q.conflicts.is_empty();
                                let added_text = if can_add { flags_text(&q.added) } else { "—".to_owned() };
                                let conflict_text = if has_conflict { flags_text(&q.conflicts) } else { "—".to_owned() };
                                view! {
                                  <tr class="border-b last:border-b-0">
                                    <td class="px-2 py-1.5 align-top">
                                      <span class="font-mono font-medium">{q.callsign.clone()}</span>
                                      <span class="block text-muted-foreground">
                                        {format!("{} · {} · {}", q.date, q.band, q.mode)}
                                      </span>
                                    </td>
                                    <td class="px-2 py-1.5 align-top">{added_text}</td>
                                    <td class="px-2 py-1.5 align-top text-amber-600 dark:text-amber-400">
                                      {conflict_text}
                                    </td>
                                    <td class="px-2 py-1.5 align-top">
                                      <Checkbox
                                        checked=Signal::derive(move || pick_added.with(|s| s.contains(&id)))
                                        on_change=Callback::new(move |v: bool| toggle(pick_added, id, v))
                                        disabled=Signal::derive(move || !can_add)
                                        aria_label=Signal::derive(move || t("log.diff-apply-added"))
                                      />
                                    </td>
                                    <td class="px-2 py-1.5 align-top">
                                      <Checkbox
                                        checked=Signal::derive(move || pick_mirror.with(|s| s.contains(&id)))
                                        on_change=Callback::new(move |v: bool| toggle(pick_mirror, id, v))
                                        disabled=Signal::derive(move || !has_conflict)
                                        aria_label=Signal::derive(move || t("log.diff-prefer-remote"))
                                      />
                                    </td>
                                  </tr>
                                }
                              })
                              .collect_view()}
                          </tbody>
                        </table>
                      </div>
                    }
                  })}

                {(!shown.is_empty())
                  .then(|| {
                    view! {
                      <div class="space-y-1">
                        <p class=HINT>{move || t("log.diff-missing-hint")}</p>
                        <ul class="space-y-0.5 text-xs text-muted-foreground">
                          {shown
                            .into_iter()
                            .map(|line| view! { <li class="font-mono">{line}</li> })
                            .collect_view()}
                        </ul>
                      </div>
                    }
                  })}
              </div>
            }
          })
        }}
      </div>
      <DialogFooter>
        <Button variant=Variant::Default on_click=on_apply>
          {move || t("log.apply")}
        </Button>
      </DialogFooter>
    </Dialog>
  }
}
