//! 首页题库选择器：题库版本下拉（含可用状态）、刷新配置（失败自动重试一次）、题库类别单选。

use std::time::Duration;

use ham_web_core::{Bank, QuestionVersion};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::cn::cn;
use crate::data::{self, VersionStatus};
use crate::icons::{Icon, IconKind};
use crate::ui::{
  BadgeVariant, Label, RadioGroup, RadioGroupItem, Select, SelectItem, Size, Variant, badge_class,
  button_class,
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
    (
      ErrorKind::Network,
      "网络连接失败，请检查网络后重试".to_owned(),
    )
  } else if message.contains("timeout") {
    (ErrorKind::Timeout, "请求超时，请重试".to_owned())
  } else if message.contains("config") {
    (ErrorKind::Config, "配置文件加载失败，请重试".to_owned())
  } else if message.is_empty() {
    (ErrorKind::Unknown, "未知错误，请重试".to_owned())
  } else {
    (ErrorKind::Unknown, message.to_owned())
  }
}

fn status_icon(v: &VersionWithStatus) -> AnyView {
  match &v.status {
    None => view! { <Icon kind=IconKind::AlertCircle class="h-4 w-4 text-gray-400" /> }.into_any(),
    Some(s) if s.is_available => {
      view! { <Icon kind=IconKind::CheckCircle class="h-4 w-4 text-green-500" /> }.into_any()
    }
    Some(_) => view! { <Icon kind=IconKind::XCircle class="h-4 w-4 text-red-500" /> }.into_any(),
  }
}

fn status_text(v: &VersionWithStatus) -> String {
  match &v.status {
    None => "状态未知".to_owned(),
    Some(s) if s.is_available => {
      format!(
        "包含: {}",
        s.available_banks
          .iter()
          .map(|b| b.as_str())
          .collect::<Vec<_>>()
          .join(", ")
      )
    }
    Some(s) => s.error.clone().unwrap_or_else(|| "无可用题库".to_owned()),
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
          web_sys::console::error_1(&format!("[ERROR] 刷新配置失败 {e}").into());
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
        format!("自动重试({}/2)...", retry_count.get())
      } else {
        "刷新中...".to_owned()
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
        <span class="text-xs text-green-600 dark:text-green-400 animate-in fade-in slide-in-from-right-1 duration-300">"成功"</span>
      </div>
    }
    .into_any(),
    RefreshState::Error => {
      let label = if retry_count.get() >= 2 || auto_retrying.get() { "重试" } else { "失败" };
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
        <span class="text-xs">"刷新"</span>
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
          {v.version.is_latest.then(|| view! { <span class=badge_class(BadgeVariant::Secondary, "text-xs")>"最新"</span> })}
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
                      .then(|| view! { <span class=badge_class(BadgeVariant::Secondary, "text-xs")>"最新"</span> })}
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
                "刷新失败 " {move || error_kind.get().map(|k| format!("({})", k.as_str()))}
              </div>
              <div class="text-xs text-orange-700 dark:text-orange-300 mt-1">{move || error_message.get()}</div>
              <div class="mt-2">
                <button
                  class=button_class(
                    Variant::Outline,
                    Size::Sm,
                    "h-6 px-2 text-xs border-orange-300 text-orange-700 hover:bg-orange-100 dark:border-orange-800 dark:text-orange-300 dark:hover:bg-orange-900/40",
                  )
                  on:click=move |_| refresh.run(false)
                >
                  "手动重试"
                </button>
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
              "加载题库版本中..."
            </div>
          </div>
        }
        .into_any();
      }
      view! {
        <div class="space-y-4">
          <div class="space-y-2">
            <div class="flex items-baseline justify-between">
              <div class="text-sm text-muted-foreground">"题库版本"</div>
              <div class="flex items-center gap-2">
                {move || {
                  selected_data
                    .get()
                    .map(|v| {
                      view! {
                        <div class="text-xs text-muted-foreground">"更新时间: " {locale_date(&v.version.updated_at)}</div>
                      }
                    })
                }}
                <button
                  class=refresh_class
                  disabled=move || refresh_state.get() == RefreshState::Loading || disabled.get()
                  title=move || {
                    let m = error_message.get();
                    if m.is_empty() { "刷新题库配置".to_owned() } else { m }
                  }
                  on:click=move |_| refresh.run(false)
                >
                  {refresh_content}
                  <span class="sr-only">"刷新配置"</span>
                </button>
              </div>
            </div>
            <Select
              value=selected_version
              on_change=Callback::new(move |v| selected_version.set(Some(v)))
              disabled=Signal::derive(move || disabled.get() || versions.with(Vec::is_empty))
              placeholder="选择题库版本"
              trigger=trigger
            >
              {items}
            </Select>
            {error_block}
          </div>

          <div class="space-y-2">
            <div class="text-sm text-muted-foreground">"选择题库"</div>
            <RadioGroup
              class="flex gap-6"
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
                        <Label r#for=id>
                          {bank.as_str()} " 类"
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
                      "该版本暂无可用的题库"
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
