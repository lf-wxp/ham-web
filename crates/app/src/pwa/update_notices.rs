use std::time::Duration;

use leptos::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use wasm_bindgen_futures::JsFuture;
use web_sys::{ServiceWorker, ServiceWorkerRegistration, ServiceWorkerState};

use ham_web_core::changelog::{self, Release};

use crate::ui::{Size, Variant, button_class};
use crate::util::{document, now_ms, storage, window};
use crate::{bank_updates, data};

use super::notice::Notice;
use crate::i18n::t;

/// 已看过的更新日志版本（日期）。
const SEEN_KEY: &str = "app:changelog-seen";
/// 每次最多展示的更新条目数。
const MAX_ITEMS: usize = 6;
/// 定期检查新版本的间隔。
const UPDATE_CHECK_INTERVAL: Duration = Duration::from_secs(60 * 60);

fn post_skip_waiting(worker: &ServiceWorker) {
  for ty in ["SKIP_WAITING", "skip-waiting"] {
    let msg = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&msg, &"type".into(), &ty.into());
    let _ = worker.post_message(&msg);
  }
}

fn reload() {
  let _ = window().location().reload();
}

/// 新版本的更新内容：拉取新部署的 `/changelog.json`，取比当前运行版本新的条目。
async fn fetch_new_items() -> (String, Vec<String>) {
  let Ok(releases) =
    data::fetch_external_json::<Vec<Release>>(&format!("/changelog.json?t={}", now_ms())).await
  else {
    return (String::new(), Vec::new());
  };
  let latest = releases
    .iter()
    .map(|r| r.date.clone())
    .max()
    .unwrap_or_default();
  (
    latest,
    changelog::items_since(&releases, changelog::current(), MAX_ITEMS),
  )
}

/// 页面右下角的更新提示：发现新版本、本次更新内容、题库内容变化。不弹模态框，避免打断考试。
#[component]
pub fn UpdateNotices() -> impl IntoView {
  let open = RwSignal::new(false);
  let new_items = RwSignal::new(Vec::<String>::new());
  let new_date = RwSignal::new(String::new());
  let waiting: StoredValue<Option<ServiceWorker>, LocalStorage> = StoredValue::new_local(None);

  // 本次更新：首次使用只记录版本，之后版本变化时展示新增条目
  let seen = storage::get(SEEN_KEY);
  let whats_new = RwSignal::new(match seen.as_deref() {
    Some(s) if s < changelog::current() => {
      changelog::items_since(&changelog::releases(), s, MAX_ITEMS)
    }
    _ => Vec::new(),
  });
  if seen.as_deref() != Some(changelog::current()) {
    storage::set(SEEN_KEY, changelog::current());
  }

  let bank_notes = RwSignal::new(Vec::<String>::new());
  let handle = window_event_listener_untyped(bank_updates::EVENT, move |e| {
    if let Some(text) = e
      .dyn_ref::<web_sys::CustomEvent>()
      .and_then(|e| e.detail().as_string())
    {
      bank_notes.update(|n| {
        if !n.contains(&text) {
          n.push(text);
        }
      });
    }
  });
  on_cleanup(move || handle.remove());
  set_timeout(
    || leptos::task::spawn_local(bank_updates::check_on_start()),
    Duration::from_secs(3),
  );

  let has_controller = || window().navigator().service_worker().controller().is_some();
  let show = move |worker: ServiceWorker| {
    waiting.set_value(Some(worker));
    open.set(true);
    leptos::task::spawn_local(async move {
      let (date, items) = fetch_new_items().await;
      new_date.set(date);
      new_items.set(items);
    });
  };

  let setup = move |reg: ServiceWorkerRegistration| {
    // 长时间开着的页面（如挂机听题、竞赛录入）也能定期发现新版本
    let reg_poll = reg.clone();
    if let Ok(handle) = set_interval_with_handle(
      move || {
        let _ = reg_poll.update();
      },
      UPDATE_CHECK_INTERVAL,
    ) {
      on_cleanup(move || handle.clear());
    }

    let reg_vis = reg.clone();
    let on_visibility = Closure::<dyn Fn()>::new(move || {
      if document().visibility_state() == web_sys::VisibilityState::Visible {
        let _ = reg_vis.update();
      }
    });
    let _ = document()
      .add_event_listener_with_callback("visibilitychange", on_visibility.as_ref().unchecked_ref());

    if let Some(w) = reg.waiting() {
      show(w);
    }

    let reg_upd = reg.clone();
    let on_update = Closure::<dyn Fn()>::new(move || {
      if let Some(installing) = reg_upd.installing() {
        let worker = installing.clone();
        let on_state = Closure::<dyn Fn()>::new(move || {
          if worker.state() == ServiceWorkerState::Installed && has_controller() {
            show(worker.clone());
          }
        });
        installing.set_onstatechange(Some(on_state.as_ref().unchecked_ref()));
        on_state.forget();
      }
    });
    let _ = reg.add_event_listener_with_callback("updatefound", on_update.as_ref().unchecked_ref());
    on_update.forget();

    let guard = SendWrapper::new(on_visibility);
    on_cleanup(move || {
      let cb = guard.take();
      let _ = document()
        .remove_event_listener_with_callback("visibilitychange", cb.as_ref().unchecked_ref());
    });
  };

  if !cfg!(debug_assertions)
    && let Ok(ready) = window().navigator().service_worker().ready()
  {
    leptos::task::spawn_local(async move {
      if let Ok(reg) = JsFuture::from(ready).await {
        setup(reg.unchecked_into());
      }
    });
  }

  let update_now = move |_| {
    let Some(worker) = waiting.get_value() else {
      return;
    };
    // 更新内容已经在这里看过，刷新后不再重复展示
    new_date.with_untracked(|d| {
      if !d.is_empty() {
        storage::set(SEEN_KEY, d);
      }
    });
    let container = window().navigator().service_worker();
    let on_change = Closure::once_into_js(reload);
    let _ =
      container.add_event_listener_with_callback("controllerchange", on_change.unchecked_ref());
    post_skip_waiting(&worker);
    set_timeout(reload, Duration::from_millis(1500));
  };

  view! {
    <div
      class="pointer-events-none fixed inset-x-3 bottom-20 z-50 ml-auto flex max-w-sm flex-col gap-2 print:hidden sm:inset-x-auto sm:right-4"
      aria-live="polite"
    >
      {move || open.get().then(|| view! {
        <Notice
          title=t("发现新版本")
          detail=t("刷新后即可使用最新功能；正在进行的练习与考试进度会自动保存。")
          items=new_items.get()
        >
          <button type="button" class=button_class(Variant::Ghost, Size::Sm, "") on:click=move |_| open.set(false)>
            {move || t("稍后")}
          </button>
          <button type="button" class=button_class(Variant::Default, Size::Sm, "") on:click=update_now>
            {move || t("刷新以更新")}
          </button>
        </Notice>
      })}
      {move || (!whats_new.with(Vec::is_empty)).then(|| view! {
        <Notice title=t("已更新到新版本") items=whats_new.get()>
          <button type="button" class=button_class(Variant::Outline, Size::Sm, "") on:click=move |_| whats_new.set(Vec::new())>
            {move || t("知道了")}
          </button>
        </Notice>
      })}
      {move || (!bank_notes.with(Vec::is_empty)).then(|| view! {
        <Notice title=t("题库已更新") items=bank_notes.get()>
          <button type="button" class=button_class(Variant::Outline, Size::Sm, "") on:click=move |_| bank_notes.set(Vec::new())>
            {move || t("知道了")}
          </button>
        </Notice>
      })}
    </div>
  }
}
