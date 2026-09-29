//! PWA：注册 Service Worker（仅 release 构建），发现新版本时提示刷新。

use std::time::Duration;

use leptos::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use wasm_bindgen_futures::JsFuture;
use web_sys::{ServiceWorker, ServiceWorkerRegistration, ServiceWorkerState};

use crate::ui::{
  Dialog, DialogDescription, DialogHeader, DialogTitle, Size, Variant, button_class,
};
use crate::util::{document, window};

/// 开发构建不注册 Service Worker，并注销同源下残留的旧 SW（例如之前 release 版留下的），
/// 避免缓存与“发现新版本”弹窗干扰调试。
pub fn register_service_worker() {
  let container = window().navigator().service_worker();
  if cfg!(debug_assertions) {
    // 在 Leptos 挂载之前调用，此时 Leptos 执行器尚未初始化，直接使用 wasm-bindgen-futures
    wasm_bindgen_futures::spawn_local(async move {
      let Ok(list) = JsFuture::from(container.get_registrations()).await else {
        return;
      };
      for reg in js_sys::Array::from(&list).iter() {
        let reg: ServiceWorkerRegistration = reg.unchecked_into();
        if let Ok(p) = reg.unregister() {
          let _ = JsFuture::from(p).await;
        }
      }
    });
    return;
  }
  let _ = container.register("/sw.js");
}

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

#[component]
pub fn PwaUpdatePrompt() -> impl IntoView {
  let open = RwSignal::new(false);
  let waiting: StoredValue<Option<ServiceWorker>, LocalStorage> = StoredValue::new_local(None);

  let has_controller = || window().navigator().service_worker().controller().is_some();
  let show = move |worker: ServiceWorker| {
    waiting.set_value(Some(worker));
    open.set(true);
  };

  let setup = move |reg: ServiceWorkerRegistration| {
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
    let container = window().navigator().service_worker();
    let on_change = Closure::once_into_js(reload);
    let _ =
      container.add_event_listener_with_callback("controllerchange", on_change.unchecked_ref());
    post_skip_waiting(&worker);
    set_timeout(reload, Duration::from_millis(1500));
  };

  view! {
    <Dialog open=open>
      <DialogHeader>
        <DialogTitle>"发现新版本"</DialogTitle>
        <DialogDescription>"有可用更新，刷新以加载最新题库与功能。"</DialogDescription>
      </DialogHeader>
      <div class="flex items-center gap-2 justify-end">
        <button class=button_class(Variant::Outline, Size::Default, "") on:click=move |_| open.set(false)>
          "稍后"
        </button>
        <button class=button_class(Variant::Default, Size::Default, "") on:click=update_now>
          "立即更新"
        </button>
      </div>
    </Dialog>
  }
}
