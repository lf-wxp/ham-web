//! Web Push 订阅：用 VAPID 公钥订阅推送服务，供后端在页面关闭时也能发送提醒。
//!
//! 完整链路：本模块向浏览器 `PushManager` 订阅 → 得到订阅对象（含 endpoint 与密钥）
//! → 交给后端推送服务保存 → 后端用 VAPID 私钥签名向推送服务发消息 → Service Worker
//! （`sw.js`）收到 `push` 事件后显示系统通知。
//!
//! VAPID 公钥从站点根路径 `/push-config.json` 读取（内容形如 `{"publicKey":"…"}`），
//! 该文件与后端推送服务需由部署方自行生成并托管，未配置时本功能保持关闭状态。

use serde::Deserialize;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{PushManager, PushSubscription, ServiceWorkerRegistration};

use crate::i18n::t;
use crate::util::{storage, window};

const SUB_KEY: &str = "push-subscription";

#[derive(Deserialize)]
struct PushConfig {
  #[serde(default)]
  public_key: String,
}

/// 浏览器是否支持 Web Push。
#[must_use]
pub fn is_supported() -> bool {
  js_sys::eval("'PushManager' in window && 'pushManager' in ServiceWorkerRegistration.prototype")
    .map(|v| v.as_bool().unwrap_or(false))
    .unwrap_or(false)
}

/// 获取 VAPID 公钥：优先后端 `/api/push/vapid-public-key`，回退到静态 `/push-config.json`。
pub async fn fetch_public_key() -> Option<String> {
  if let Ok(cfg) =
    crate::data::fetch_external_json::<PushConfig>("/api/push/vapid-public-key").await
    && !cfg.public_key.is_empty()
  {
    return Some(cfg.public_key);
  }
  let cfg: PushConfig = crate::data::fetch_external_json("/push-config.json")
    .await
    .ok()?;
  if cfg.public_key.is_empty() {
    None
  } else {
    Some(cfg.public_key)
  }
}

/// 由用户设置的每日提醒时间（本地 `HH:MM`）换算为 UTC 分钟数（0–1439）。
fn reminder_utc_minutes() -> Option<u32> {
  let plan = crate::study::load_plan();
  let (h, m) = plan.reminder_time.split_once(':')?;
  let h: i64 = h.parse().ok()?;
  let m: i64 = m.parse().ok()?;
  let local = h * 60 + m;
  // Date.getTimezoneOffset()：本地时间与 UTC 的差（分钟），UTC = local + offset。
  let tz = js_sys::Date::new_0().get_timezone_offset() as i64;
  Some((local + tz).rem_euclid(1440) as u32)
}

/// POST JSON 到同源后端（静默失败）。
async fn post_json(url: &str, body: &str) -> Result<(), ()> {
  use wasm_bindgen::JsValue;
  use web_sys::{Headers, Request, RequestInit, Response};
  let headers = Headers::new().map_err(|_| ())?;
  headers
    .set("Content-Type", "application/json")
    .map_err(|_| ())?;
  let init = RequestInit::new();
  init.set_method("POST");
  init.set_headers(&headers);
  init.set_body(&JsValue::from_str(body));
  let request = Request::new_with_str_and_init(url, &init).map_err(|_| ())?;
  let resp: Response = JsFuture::from(window().fetch_with_request(&request))
    .await
    .map_err(|_| ())?
    .unchecked_into();
  if resp.ok() { Ok(()) } else { Err(()) }
}

/// 把订阅上报给后端推送服务（携带每日提醒时间）。
async fn report_subscription(json: &str) {
  let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
    return;
  };
  let endpoint = v["endpoint"].as_str().unwrap_or_default();
  let p256dh = v["keys"]["p256dh"].as_str().unwrap_or_default();
  let auth = v["keys"]["auth"].as_str().unwrap_or_default();
  let body = serde_json::json!({
    "endpoint": endpoint,
    "keys": { "p256dh": p256dh, "auth": auth },
    "reminder_utc_minutes": reminder_utc_minutes(),
  });
  let _ = post_json("/api/push/subscribe", &body.to_string()).await;
}

/// base64url（无 padding）→ `Uint8Array`，作为 `applicationServerKey`。
fn b64url_to_bytes(s: &str) -> Option<js_sys::Uint8Array> {
  let b64 = s.replace('-', "+").replace('_', "/");
  let padded = format!("{b64}{}", "=".repeat((4 - b64.len() % 4) % 4));
  let decoded = window().atob(&padded).ok()?;
  let bytes = decoded.into_bytes();
  let arr = js_sys::Uint8Array::new_with_length(bytes.len() as u32);
  arr.copy_from(&bytes);
  Some(arr)
}

/// Service Worker 就绪后的 registration。
async fn registration() -> Option<ServiceWorkerRegistration> {
  let ready = window().navigator().service_worker().ready().ok()?;
  let value = JsFuture::from(ready).await.ok()?;
  Some(value.unchecked_into())
}

/// 订阅推送，返回订阅 JSON 字符串（供后端保存），并缓存到本地。
pub async fn subscribe(public_key: &str) -> Result<String, String> {
  let reg = registration()
    .await
    .ok_or_else(|| t("service worker 未就绪"))?;
  let pm: PushManager = reg.push_manager().map_err(|e| format!("{e:?}"))?;
  let key = b64url_to_bytes(public_key).ok_or_else(|| t("VAPID 公钥无效"))?;
  let options = web_sys::PushSubscriptionOptionsInit::new();
  options.set_user_visible_only(true);
  options.set_application_server_key_opt_u8_array(Some(&key));
  let promise = pm
    .subscribe_with_options(&options)
    .map_err(|e| format!("{e:?}"))?;
  let value = JsFuture::from(promise)
    .await
    .map_err(|e| format!("{e:?}"))?;
  let sub: PushSubscription = value.unchecked_into();
  let json = js_sys::JSON::stringify(&wasm_bindgen::JsValue::from(&sub))
    .map_err(|e| format!("{e:?}"))?
    .as_string()
    .unwrap_or_default();
  storage::set(SUB_KEY, &json);
  report_subscription(&json).await;
  // 订阅后立即同步一次待复习数，避免后端因缺少数据而在提醒时回退为通用文案。
  sync_review_counts();
  Ok(json)
}

/// 退订并清除本地缓存的订阅。
pub async fn unsubscribe() {
  let Some(reg) = registration().await else {
    return;
  };
  if let Ok(pm) = reg.push_manager()
    && let Ok(promise) = pm.get_subscription()
    && let Ok(value) = JsFuture::from(promise).await
    && let Some(sub) = value.dyn_ref::<PushSubscription>()
    && let Ok(p) = sub.unsubscribe()
  {
    let _ = JsFuture::from(p).await;
  }
  // 上报后端退订（静默失败）。
  if let Some(json) = storage::get(SUB_KEY)
    && let Ok(v) = serde_json::from_str::<serde_json::Value>(&json)
    && let Some(endpoint) = v["endpoint"].as_str()
  {
    let body = serde_json::json!({ "endpoint": endpoint }).to_string();
    let _ = post_json("/api/push/unsubscribe", &body).await;
  }
  storage::remove(SUB_KEY);
}

/// 是否已存在本地订阅（仅表示曾订阅过，未必仍有效）。
#[must_use]
pub fn has_subscription() -> bool {
  storage::get(SUB_KEY).is_some()
}

/// 上报「今日待复习数」给后端（静默失败），供每日提醒推送时附带具体数量。
async fn report_review_counts(mistakes: u32, cards: u32) {
  let Some(json) = storage::get(SUB_KEY) else {
    return;
  };
  let Ok(v) = serde_json::from_str::<serde_json::Value>(&json) else {
    return;
  };
  let Some(endpoint) = v["endpoint"].as_str() else {
    return;
  };
  let body = serde_json::json!({
    "endpoint": endpoint,
    "due_mistakes": mistakes,
    "due_cards": cards,
  });
  let _ = post_json("/api/push/review-count", &body.to_string()).await;
}

/// 同步本地「今日待复习数」到后端（页面打开 / 复习完成时调用；未订阅时为空操作）。
pub fn sync_review_counts() {
  if !has_subscription() {
    return;
  }
  let now = crate::util::now_ms();
  let mistakes = crate::study::load_book().due_count(now) as u32;
  let cards = crate::pages::load_card_schedule().due_total(now) as u32;
  leptos::task::spawn_local(async move {
    report_review_counts(mistakes, cards).await;
  });
}
