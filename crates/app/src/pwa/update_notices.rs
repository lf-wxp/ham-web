use std::time::Duration;

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use wasm_bindgen_futures::JsFuture;
use web_sys::{ServiceWorker, ServiceWorkerRegistration, ServiceWorkerState};

use ham_web_core::changelog::{self, Release};
use ham_web_core::update_prompt::{self, StateChange};

use crate::ui::{Button, Size, Variant};
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
/// 点「刷新以更新」后，等新版接管页面（`controllerchange`）的最长时间，超时才强制刷新。
///
/// 不能取得太短：waiting 的 worker 常被浏览器回收，收到 `SKIP_WAITING` 前要先冷启动、
/// 执行完整个脚本（含几百项的预缓存清单），慢设备上轻易超过一两秒。在它激活之前就刷新，
/// 新页面会在 `reg.waiting()` 里再次看到它，于是又弹出同一个提示。
const ACTIVATE_WAIT: Duration = Duration::from_secs(8);

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

/// 提示所指的 waiting worker 一旦不再是 `installed`，提示就过时了，该关。
/// 什么时候不该关（提示已指向更新的 worker、用户正等新版接管）由
/// [`update_prompt::should_dismiss`] 判定，这里只把浏览器侧的三个事实喂给它。
fn dismiss_when_settled(
  worker: &ServiceWorker,
  shown: StoredValue<Option<ServiceWorker>, LocalStorage>,
  open: RwSignal<bool>,
  applying: RwSignal<bool>,
) {
  let w = worker.clone();
  let on_state = Closure::<dyn Fn()>::new(move || {
    let change = StateChange {
      is_current_target: shown.with_value(|cur| cur.as_ref() == Some(&w)),
      still_installed: w.state() == ServiceWorkerState::Installed,
      user_applying: applying.get_untracked(),
    };
    if update_prompt::should_dismiss(change) {
      open.set(false);
    }
  });
  let _ = worker.add_event_listener_with_callback("statechange", on_state.as_ref().unchecked_ref());
  on_state.forget();
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
  // 更新日志条目是 `crates/core` 的中文原文，经 `core_text_tracked` 查知识库词典；
  // 词典不会自己到，组件体里挂一次拉取（切语言后由 Effect 重新判断）。
  data::use_knowledge_i18n();
  let open = RwSignal::new(false);
  // 用户已点「刷新以更新」、正在等新版接管页面：按钮转圈，提示不因 worker 激活而被关掉。
  let applying = RwSignal::new(false);
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
  // 启动 3 秒后：先检查题库更新，再在空闲时把常用题库预热进内存（见 `bank_updates::prewarm`）。
  set_timeout(
    || {
      leptos::task::spawn_local(async {
        bank_updates::check_on_start().await;
        bank_updates::prewarm().await;
      });
    },
    Duration::from_secs(3),
  );

  let has_controller = || window().navigator().service_worker().controller().is_some();
  let show = move |worker: ServiceWorker| {
    dismiss_when_settled(&worker, waiting, open, applying);
    waiting.set_value(Some(worker));
    // 出现了新的 waiting worker（含更新途中又部署了一版）：重新开始一轮，旧的「更新中」作废。
    applying.set(false);
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
    // `setup` 在 `.await` 之后才执行，此时没有 Owner：`on_cleanup` 会被直接丢弃、什么也清不掉
    // （这里曾经写了 `on_cleanup(handle.clear())`，是空操作）。「检查更新」本来就该活到文档结束，
    // 所以句柄不保留、监听器用 `forget()` 显式常驻，不假装它们会被清理。
    let _ = set_interval_with_handle(
      move || {
        let _ = reg_poll.update();
      },
      UPDATE_CHECK_INTERVAL,
    );

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

    // 常驻：闭包若在这里被 drop，`document` 上的监听还在，之后每次切标签页都会抛
    // “closure invoked after being dropped”，「回前台检查新版本」就失效了。
    on_visibility.forget();
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
    // 已经在等新版接管：再点一次只会重复发 `SKIP_WAITING` 并叠一个兜底定时器。
    if applying.get_untracked() {
      return;
    }
    let Some(worker) = waiting.get_value() else {
      return;
    };
    applying.set(true);
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
    // 正常由 `controllerchange` 触发刷新；这里只是兜底（个别浏览器不派发该事件）。
    set_timeout(reload, ACTIVATE_WAIT);
  };

  view! {
    <div
      class="pointer-events-none fixed inset-x-3 bottom-20 z-[var(--z-top)] ml-auto flex max-w-sm flex-col gap-2 print:hidden sm:inset-x-auto sm:right-4"
      aria-live="polite"
    >
      {move || open.get().then(|| view! {
        <Notice
          title=t("settings.new-version-available")
          detail=t("settings.refresh-to-use-the")
          items=new_items.get()
        >
          <Button
            variant=Variant::Ghost
            size=Size::Sm
            disabled=applying
            on_click=Callback::new(move |_| open.set(false))
          >
            {move || t("settings.later")}
          </Button>
          <Button
            variant=Variant::Default
            size=Size::Sm
            loading=applying
            on_click=Callback::new(move |_| update_now(()))
          >
            {move || t("settings.refresh-to-update")}
          </Button>
        </Notice>
      })}
      {move || (!whats_new.with(Vec::is_empty)).then(|| view! {
        <Notice title=t("settings.updated-to-the-new") items=whats_new.get()>
          <Button
            variant=Variant::Outline
            size=Size::Sm
            on_click=Callback::new(move |_| whats_new.set(Vec::new()))
          >
            {move || t("exam.ok")}
          </Button>
        </Notice>
      })}
      {move || (!bank_notes.with(Vec::is_empty)).then(|| view! {
        <Notice title=t("settings.question-bank-updated") items=bank_notes.get()>
          <Button
            variant=Variant::Outline
            size=Size::Sm
            on_click=Callback::new(move |_| bank_notes.set(Vec::new()))
          >
            {move || t("exam.ok")}
          </Button>
        </Notice>
      })}
    </div>
  }
}
