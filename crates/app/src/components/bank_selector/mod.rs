//! 首页题库选择器：题库版本下拉（含可用状态）、刷新配置（失败自动重试一次）、题库类别单选。

use std::time::Duration;

mod status_icon;
use status_icon::status_icon;

use ham_web_core::{Bank, QuestionVersion};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::cn::cn;
use crate::data::{self, VersionStatus};
use crate::i18n::{bank_class, t, tf};
use crate::icons::{Icon, IconKind};
use crate::ui::{
  BadgeVariant, Button, Label, RadioGroup, RadioGroupItem, Select, SelectItem, Size, Variant,
  badge_class, button_class,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefreshState {
  Idle,
  Loading,
  Success,
  Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ErrorKind {
  Network,
  Config,
  Timeout,
  Unknown,
}

impl ErrorKind {
  const fn as_str(self) -> &'static str {
    match self {
      Self::Network => "network",
      Self::Config => "config",
      Self::Timeout => "timeout",
      Self::Unknown => "unknown",
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
struct VersionWithStatus {
  version: QuestionVersion,
  status: Option<VersionStatus>,
}

fn classify(message: &str) -> (ErrorKind, String) {
  if ["Failed to fetch", "NetworkError", "Load failed"]
    .iter()
    .any(|p| message.contains(p))
  {
    (ErrorKind::Network, t("exam.network-error-u-2014"))
  } else if message.contains("timeout") {
    (ErrorKind::Timeout, t("exam.request-timed-out-u"))
  } else if message.contains("config") {
    (ErrorKind::Config, t("exam.failed-to-load-config"))
  } else if message.is_empty() {
    (ErrorKind::Unknown, t("exam.unknown-error-u-2014"))
  } else {
    (ErrorKind::Unknown, message.to_owned())
  }
}

fn status_text(v: &VersionWithStatus) -> String {
  match &v.status {
    None => t("exam.status-unknown"),
    Some(s) if s.is_available => tf(
      "exam.includes",
      &[&s
        .available_banks
        .iter()
        .map(|b| b.as_str())
        .collect::<Vec<_>>()
        .join(", ")],
    ),
    Some(s) => s
      .error
      .clone()
      .unwrap_or_else(|| t("exam.no-banks-available")),
  }
}

fn locale_date(iso: &str) -> String {
  let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_str(iso));
  String::from(d.to_locale_date_string("zh-CN", &wasm_bindgen::JsValue::UNDEFINED))
}

#[component]
pub fn QuestionBankSelector(
  selected_version: RwSignal<Option<String>>,
  selected_bank: RwSignal<Bank>,
  #[prop(into)] disabled: Signal<bool>,
) -> impl IntoView {
  let versions = RwSignal::new(Vec::<VersionWithStatus>::new());
  let loading = RwSignal::new(true);
  let refresh_state = RwSignal::new(RefreshState::Idle);
  let error_message = RwSignal::new(String::new());
  let error_kind = RwSignal::new(None::<ErrorKind>);
  let retry_count = RwSignal::new(0u32);
  let auto_retrying = RwSignal::new(false);

  let reload = move || async move {
    loading.set(true);
    match data::all_versions(false).await {
      Ok(list) => {
        let mut out = Vec::with_capacity(list.len());
        for v in list {
          let status = data::version_status(&v.id, false).await;
          out.push(VersionWithStatus {
            version: v,
            status: Some(status),
          });
        }
        if selected_version.get_untracked().is_none() {
          let pick = out
            .iter()
            .find(|v| v.version.is_latest && v.status.as_ref().is_some_and(|s| s.is_available))
            .or_else(|| {
              out
                .iter()
                .find(|v| v.status.as_ref().is_some_and(|s| s.is_available))
            })
            .or_else(|| out.first());
          if let Some(p) = pick {
            selected_version.set(Some(p.version.id.clone()));
          }
        }
        versions.set(out);
      }
      Err(e) => web_sys::console::error_1(
        &format!("[ERROR] Failed to load question bank versions {e}").into(),
      ),
    }
    loading.set(false);
  };

  spawn_local(reload());

  let selected_data = Memo::new(move |_| {
    let id = selected_version.get()?;
    versions.with(|vs| vs.iter().find(|v| v.version.id == id).cloned())
  });
  let available_banks = Memo::new(move |_| {
    selected_data.with(|d| {
      d.as_ref()
        .and_then(|d| d.status.as_ref())
        .map(|s| s.available_banks.clone())
        .unwrap_or_default()
    })
  });

  // 当前版本不含所选题库时，自动切换到第一个可用题库
  Effect::new(move |_| {
    let banks = available_banks.get();
    if let Some(first) = banks.first()
      && !banks.contains(&selected_bank.get())
    {
      selected_bank.set(*first);
    }
  });

  let handle_refresh = StoredValue::new(None::<Callback<bool>>);
  let refresh = Callback::new(move |is_retry: bool| {
    if refresh_state.get_untracked() == RefreshState::Loading {
      return;
    }
    let was_auto = auto_retrying.get_untracked();
    let count = retry_count.get_untracked();
    refresh_state.set(RefreshState::Loading);
    error_message.set(String::new());
    error_kind.set(None);
    if is_retry {
      retry_count.update(|c| *c += 1);
      auto_retrying.set(true);
    } else {
      retry_count.set(0);
      auto_retrying.set(false);
    }
    spawn_local(async move {
      match data::refresh_all().await {
        Ok(()) => {
          reload().await;
          refresh_state.set(RefreshState::Success);
          retry_count.set(0);
          auto_retrying.set(false);
          set_timeout(
            move || refresh_state.set(RefreshState::Idle),
            Duration::from_secs(3),
          );
        }
        Err(e) => {
          web_sys::console::error_1(
            &tf("common.error-failed-to-refresh", &[&(e).to_string()]).into(),
          );
          let (kind, msg) = classify(&e.to_string());
          error_kind.set(Some(kind));
          error_message.set(msg);
          refresh_state.set(RefreshState::Error);
          if !is_retry && !was_auto && count < 2 {
            let delay = 1000u64 << count;
            set_timeout(
              move || {
                if let Some(cb) = handle_refresh.get_value() {
                  cb.run(true);
                }
              },
              Duration::from_millis(delay),
            );
          } else {
            auto_retrying.set(false);
            set_timeout(
              move || refresh_state.set(RefreshState::Idle),
              Duration::from_secs(3),
            );
          }
        }
      }
    });
  });
  handle_refresh.set_value(Some(refresh));

  // 刻意的例外：按钮底色随刷新状态（成功 / 失败 / 加载）变化，`Button` 的 `variant` 是
  // 静态的、`class` 也不接受按状态重算，因此这里直接用 `button_class` 合并类名
  // （见 `docs/ui-components.md` 的「常见坑」第 4 条）。
  let refresh_class = move || {
    let extra = match refresh_state.get() {
      RefreshState::Success => {
        "outline border-green-200 bg-green-50 hover:bg-green-100 dark:border-green-800 dark:bg-green-950/40 dark:hover:bg-green-900/40"
      }
      RefreshState::Error => {
        "outline border-red-200 bg-red-50 hover:bg-red-100 border-orange-300 bg-orange-50 hover:bg-orange-100 dark:border-orange-800 dark:bg-orange-950/40 dark:hover:bg-orange-900/40"
      }
      RefreshState::Loading | RefreshState::Idle => "outline",
    };
    button_class(
      Variant::Outline,
      Size::Sm,
      &cn(&["h-7 px-2 transition-all duration-300", extra]),
    )
  };

  let refresh_content = move || {
    match refresh_state.get() {
    RefreshState::Loading => {
      let text = if auto_retrying.get() && retry_count.get() > 0 {
        tf("exam.auto-retry-2-u", &[&retry_count.get().to_string()])
      } else {
        t("exam.refreshing-u-2026")
      };
      view! {
        <div class="flex items-center gap-1">
          <Icon kind=IconKind::Loader2 class="h-3 w-3 animate-spin" />
          <span class="text-xs">{text}</span>
        </div>
      }
      .into_any()
    }
    RefreshState::Success => view! {
      <div class="flex items-center gap-1">
        <Icon kind=IconKind::CheckCircle class="h-3 w-3 text-green-500 animate-in fade-in slide-in-from-left-1 duration-300" />
        <span class="text-xs text-green-600 dark:text-green-400 animate-in fade-in slide-in-from-right-1 duration-300">{move || t("exam.done")}</span>
      </div>
    }
    .into_any(),
    RefreshState::Error => {
      let label = if retry_count.get() >= 2 || auto_retrying.get() { t("exam.retry") } else { t("exam.failed") };
      view! {
        <div class="flex items-center gap-1">
          <Icon kind=IconKind::XCircle class="h-3 w-3 text-red-500 animate-in fade-in slide-in-from-left-1 duration-300" />
          <span class="text-xs text-red-600 dark:text-red-400 animate-in fade-in slide-in-from-right-1 duration-300">{label}</span>
        </div>
      }
      .into_any()
    }
    RefreshState::Idle => view! {
      <div class="flex items-center gap-1">
        <Icon kind=IconKind::RefreshCw class="h-3 w-3 transition-transform hover:rotate-12" />
        <span class="text-xs">{move || t("exam.refresh")}</span>
      </div>
    }
    .into_any(),
  }
  };

  let trigger = move || {
    selected_data.get().map(|v| {
      view! {
        <div class="flex items-center gap-2">
          {status_icon(&v)}
          <span>{v.version.name.clone()}</span>
          {v.version.is_latest.then(|| view! { <span class=badge_class(BadgeVariant::Secondary, "text-xs")>{move || t("exam.latest")}</span> })}
        </div>
      }
    })
  };

  let items = move || {
    versions
      .get()
      .into_iter()
      .map(|v| {
        view! {
          <SelectItem value=v.version.id.clone()>
            <div class="flex items-center justify-between w-full">
              <div class="flex items-center gap-2">
                {status_icon(&v)}
                <div class="flex flex-col">
                  <div class="flex items-center gap-2">
                    <span class="font-medium">{v.version.name.clone()}</span>
                    {v
                      .version
                      .is_latest
                      .then(|| view! { <span class=badge_class(BadgeVariant::Secondary, "text-xs")>{move || t("exam.latest")}</span> })}
                  </div>
                  <div class="text-xs text-muted-foreground">{status_text(&v)}</div>
                </div>
              </div>
            </div>
          </SelectItem>
        }
      })
      .collect_view()
  };

  let error_block = move || {
    (refresh_state.get() == RefreshState::Error
      && !error_message.get().is_empty()
      && (retry_count.get() >= 2 || !auto_retrying.get()))
    .then(|| {
      view! {
        <div class="mt-2 p-3 bg-orange-50 border border-orange-200 rounded-md dark:bg-orange-950/40 dark:border-orange-900">
          <div class="flex items-start gap-2">
            <Icon kind=IconKind::AlertCircle class="h-4 w-4 text-orange-500 mt-0.5 flex-shrink-0" />
            <div class="flex-1">
              <div class="text-sm font-medium text-orange-800 dark:text-orange-200">
                {move || t("exam.refresh-failed")} {move || error_kind.get().map(|k| format!("({})", k.as_str()))}
              </div>
              <div class="text-xs text-orange-700 dark:text-orange-300 mt-1">{move || error_message.get()}</div>
              <div class="mt-2">
                <Button
                  variant=Variant::Outline
                  size=Size::Sm
                  class="h-6 px-2 text-xs border-orange-300 text-orange-700 hover:bg-orange-100 dark:border-orange-800 dark:text-orange-300 dark:hover:bg-orange-900/40"
                  on_click=Callback::new(move |_| refresh.run(false))
                >
                  {move || t("exam.retry-manually")}
                </Button>
              </div>
            </div>
          </div>
        </div>
      }
    })
  };

  let bank_value = Signal::derive(move || selected_bank.get().as_str().to_owned());
  let on_bank = Callback::new(move |v: String| {
    if let Ok(b) = v.parse() {
      selected_bank.set(b);
    }
  });

  view! {
    {move || {
      if loading.get() && versions.with(Vec::is_empty) {
        return view! {
          <div class="space-y-4">
            <div class="flex items-center gap-2 text-sm text-muted-foreground">
              <Icon kind=IconKind::Loader2 class="h-4 w-4 animate-spin" />
              {move || t("exam.loading-bank-versions-u")}
            </div>
          </div>
        }
        .into_any();
      }
      view! {
        <div class="space-y-4">
          <div class="space-y-2">
            <div class="flex items-baseline justify-between">
              <div class="text-sm text-muted-foreground">{move || t("exam.bank-version")}</div>
              <div class="flex items-center gap-2">
                {move || {
                  selected_data
                    .get()
                    .map(|v| {
                      view! {
                        <div class="text-xs text-muted-foreground">{move || t("exam.updated")} {locale_date(&v.version.updated_at)}</div>
                      }
                    })
                }}
                <button
                  class=refresh_class
                  disabled=move || refresh_state.get() == RefreshState::Loading || disabled.get()
                  title=move || {
                    let m = error_message.get();
                    if m.is_empty() { t("exam.refresh-bank-config") } else { m }
                  }
                  on:click=move |_| refresh.run(false)
                >
                  {refresh_content}
                  <span class="sr-only">{move || t("exam.refresh-config")}</span>
                </button>
              </div>
            </div>
            <Select
              value=selected_version
              on_change=Callback::new(move |v| selected_version.set(Some(v)))
              disabled=Signal::derive(move || disabled.get() || versions.with(Vec::is_empty))
              placeholder=Signal::derive(move || t("exam.select-a-bank-version"))
              trigger=trigger
            >
              {items}
            </Select>
            {error_block}
          </div>

          <div class="space-y-2">
            <div class="text-sm text-muted-foreground">{move || t("exam.select-a-bank")}</div>
            <RadioGroup
              class="flex flex-wrap gap-x-6 gap-y-2"
              value=bank_value
              on_change=on_bank
              disabled=Signal::derive(move || disabled.get() || available_banks.with(Vec::is_empty))
            >
              {move || {
                let info = selected_data.get();
                available_banks
                  .get()
                  .into_iter()
                  .map(|bank| {
                    let id = format!("bank-{bank}");
                    let desc = info.as_ref().map(|v| v.version.banks.get(bank).description.clone());
                    view! {
                      <div class="flex items-center gap-2">
                        <RadioGroupItem id=id.clone() value=bank.as_str() />
                        // flex-col 覆盖 Label 默认 flex-row（cn 冲突组），题号与描述纵向排列且不换行
                        <Label r#for=id class=Signal::derive(|| "flex-col items-start gap-0.5".to_owned())>
                          <span class="whitespace-nowrap">{move || bank_class(bank.as_str())}</span>
                          {desc.map(|d| view! { <div class="text-xs text-muted-foreground">{d}</div> })}
                        </Label>
                      </div>
                    }
                  })
                  .collect_view()
              }}
            </RadioGroup>
            {move || {
              (available_banks.with(Vec::is_empty) && selected_version.with(Option::is_some))
                .then(|| {
                  view! {
                    <div class="text-sm text-muted-foreground flex items-center gap-2">
                      <Icon kind=IconKind::XCircle class="h-4 w-4 text-red-500" />
                      {move || t("exam.no-banks-available-for")}
                    </div>
                  }
                })
            }}
          </div>
        </div>
      }
      .into_any()
    }}
  }
}
