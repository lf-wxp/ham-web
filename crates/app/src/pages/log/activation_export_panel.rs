//! 野外激活导出面板：把日志里某个山峰 / 公园的通联导出成 SOTA CSV V2 / POTA ADIF 提交文件。
//!
//! 文件内容、编号校验与跳过规则都在 `ham_web_core::activation_export`（含语义约定与已知边界）；
//! 这里只负责选平台、选编号、把结果摆出来，以及触发下载。
//!
//! 默认折叠：绝大多数用户只是记日志，不上传野外激活。

use ham_web_core::activation_export::{
  Export, Platform, RefError, Skipped, candidates, check_ref, export,
};
use leptos::prelude::*;

use crate::i18n::{t, tf, tp};
use crate::ui::{Button, Chip, ChipGroup, Field, Input, Size, Variant};
use crate::util::{download_text, unique_id};

use super::use_log_store;

/// 导出面板的计算结果：编号不可用的原因，或导出的文件与跳过计数。
#[derive(Clone, PartialEq, Eq)]
enum Outcome {
  Ref(RefError),
  Done(Export),
}

fn platform_of(v: &str) -> Platform {
  if v == "sota" {
    Platform::Sota
  } else {
    Platform::Pota
  }
}

/// 跳过原因的明细行（只列非零的）。
fn skipped_lines(s: &Skipped) -> Vec<String> {
  let mut v = Vec::new();
  if s.bad_datetime > 0 {
    v.push(tp(
      "log.activation-skipped-datetime",
      s.bad_datetime as u32,
      &[&s.bad_datetime.to_string()],
    ));
  }
  if s.missing_data > 0 {
    v.push(tp(
      "log.activation-skipped-missing",
      s.missing_data as u32,
      &[&s.missing_data.to_string()],
    ));
  }
  if s.unsupported_mode > 0 {
    v.push(tp(
      "log.activation-skipped-mode",
      s.unsupported_mode as u32,
      &[&s.unsupported_mode.to_string()],
    ));
  }
  v
}

#[component]
pub(super) fn ActivationExportPanel() -> impl IntoView {
  let store = use_log_store();
  let logbook = store.logbook;
  let book = store.station;

  let platform = RwSignal::new("pota".to_owned());
  let reference = RwSignal::new(String::new());
  let ref_id = unique_id("activation-ref");

  // 日志里出现过的该平台编号：点一下就填进输入框，不用自己再敲一遍。
  let found = Memo::new(move |_| {
    let p = platform_of(&platform.get());
    logbook.with(|lb| candidates(&lb.entries, p))
  });

  // 切换平台后编号格式就变了：输入框里旧平台的编号留着只会报「格式不对」，直接换成新平台里最常用的那个。
  Effect::new(move |_| {
    platform.track();
    let first = found.with_untracked(|f| f.first().map(|(r, _)| r.clone()));
    reference.set(first.unwrap_or_default());
  });

  let outcome = Memo::new(move |_| {
    let p = platform_of(&platform.get());
    let r = reference.get();
    match check_ref(p, &r) {
      Err(e) => Outcome::Ref(e),
      Ok(r) => {
        let result = logbook.with(|lb| book.with(|b| export(&lb.entries, b, p, &r)));
        match result {
          Ok(x) => Outcome::Done(x),
          Err(e) => Outcome::Ref(e),
        }
      }
    }
  });

  view! {
    <details class="pxl-card border bg-card">
      <summary class="cursor-pointer px-4 py-3 text-sm font-semibold">
        {move || t("log.activation-title")}
      </summary>
      <div class="space-y-3 border-t p-4">
        <p class="text-xs text-muted-foreground">{move || t("log.activation-hint")}</p>

        <ChipGroup
          value=platform
          on_change=Callback::new(move |v: String| platform.set(v))
          aria_label=Signal::derive(move || t("log.activation-platform"))
        >
          <Chip value="pota".to_owned()>
            {view! { {move || t("radio.pota-parks")} }.into_any()}
          </Chip>
          <Chip value="sota".to_owned()>
            {view! { {move || t("radio.sota-summits")} }.into_any()}
          </Chip>
        </ChipGroup>

        <Field
          label=Signal::derive(move || {
            if platform.get() == "sota" {
              t("log.activation-my-summit")
            } else {
              t("log.activation-my-park")
            }
          })
          r#for=ref_id.clone()
        >
          <Input
            id=ref_id
            value=reference
            on_change=Callback::new(move |v: String| reference.set(v))
            placeholder=Signal::derive(move || {
              if platform.get() == "sota" { "BY/BJ-001".to_owned() } else { "CN-0123".to_owned() }
            })
            class="max-w-xs uppercase"
          />
        </Field>

        {move || {
          let list = found.get();
          (!list.is_empty())
            .then(|| {
              view! {
                <div class="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                  <span>{move || t("log.activation-in-your-log")}</span>
                  {list
                    .into_iter()
                    .map(|(r, n)| {
                      let pick = r.clone();
                      view! {
                        <Button
                          variant=Variant::Ghost
                          size=Size::Sm
                          on_click=Callback::new(move |_| reference.set(pick.clone()))
                        >
                          {tp("common.records", n as u32, &[&r, &n.to_string()])}
                        </Button>
                      }
                    })
                    .collect_view()}
                </div>
              }
            })
        }}

        {move || match outcome.get() {
          Outcome::Ref(RefError::Empty) => {
            let key = if found.with(|f| f.is_empty()) {
              "log.activation-no-refs-in-log"
            } else {
              "log.activation-enter-ref"
            };
            view! { <p class="text-xs text-muted-foreground">{t(key)}</p> }.into_any()
          }
          Outcome::Ref(RefError::BadFormat) => {
            let key = if platform.get() == "sota" {
              "log.activation-bad-sota-ref"
            } else {
              "log.activation-bad-pota-ref"
            };
            view! { <p class="text-xs text-destructive" role="alert">{t(key)}</p> }.into_any()
          }
          Outcome::Done(x) => {
            let p = platform_of(&platform.get());
            let min = p.min_qsos();
            let lines = skipped_lines(&x.skipped);
            let empty = x.files.is_empty();
            view! {
              <div class="space-y-2">
                {empty
                  .then(|| {
                    view! {
                      <p class="text-xs text-muted-foreground" role="status">
                        {t("log.activation-nothing-to-export")}
                      </p>
                    }
                  })}
                {(!lines.is_empty())
                  .then(|| {
                    view! {
                      <ul class="space-y-0.5 text-xs text-muted-foreground">
                        {lines.into_iter().map(|l| view! { <li>{l}</li> }).collect_view()}
                      </ul>
                    }
                  })}
                <ul class="space-y-1.5">
                  {x
                    .files
                    .into_iter()
                    .map(|f| {
                      let (name, content) = (f.name.clone(), f.content.clone());
                      let short = f.qsos < min;
                      let mime = if p == Platform::Sota { "text/csv" } else { "text/plain" };
                      view! {
                        <li class="flex flex-wrap items-center gap-x-3 gap-y-1 border px-3 py-2 text-sm">
                          <span class="font-mono text-xs">{f.name.clone()}</span>
                          <span class="text-xs text-muted-foreground">
                            {tp("log.activation-file-qsos", f.qsos as u32, &[&f.date, &f.qsos.to_string()])}
                          </span>
                          {short
                            .then(|| {
                              view! {
                                <span class="text-xs text-destructive">
                                  {tf("log.activation-below-minimum", &[&min.to_string()])}
                                </span>
                              }
                            })}
                          <Button
                            variant=Variant::Outline
                            size=Size::Sm
                            class="ml-auto"
                            aria_label=Signal::derive({
                              let name = name.clone();
                              move || tf("log.activation-download-file", &[&name])
                            })
                            on_click=Callback::new(move |_| download_text(&name, &content, mime))
                          >
                            {move || t("log.activation-download")}
                          </Button>
                        </li>
                      }
                    })
                    .collect_view()}
                </ul>
              </div>
            }
              .into_any()
          }
        }}
      </div>
    </details>
  }
}
