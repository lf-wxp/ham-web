//! 卫星过境提醒：设置持久化、过境拉取，以及全局后台检查（应用打开期间任意页面都会提醒）。

use std::cell::RefCell;
use std::time::Duration;

use ham_web_core::sat_watch::{Notified, Pass, SatWatch, compass, mark_notified};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use wasm_bindgen::JsValue;

use crate::util::{notify, storage};

const WATCH_KEY: &str = "sat-watch";
const NOTIFIED_KEY: &str = "sat-watch-notified";
/// 过境数据刷新间隔（毫秒）：服务端预报覆盖未来 24 小时，半小时刷新足够。
const REFRESH_MS: f64 = 30.0 * 60_000.0;
/// 后台检查间隔。
const TICK: Duration = Duration::from_secs(30);

#[derive(Deserialize)]
struct PassesApi {
  passes: Vec<Pass>,
}

pub fn load() -> SatWatch {
  storage::get_json(WATCH_KEY).unwrap_or_default()
}

pub fn save(w: &SatWatch) {
  storage::set_json(WATCH_KEY, w);
}

/// 查询未来 24 小时过境。
pub async fn fetch_passes(w: &SatWatch) -> Result<Vec<Pass>, crate::data::AppError> {
  let url = format!(
    "/api/passes?lat={}&lon={}&min_elev={}",
    w.lat, w.lon, w.min_elev
  );
  crate::data::fetch_external_json::<PassesApi>(&url)
    .await
    .map(|api| api.passes)
}

/// Unix 秒 → 本地时间 `HH:MM`。
pub fn local_hm(unix: i64) -> String {
  let d = js_sys::Date::new(&JsValue::from_f64(unix as f64 * 1000.0));
  format!("{:02}:{:02}", d.get_hours(), d.get_minutes())
}

fn query_key(w: &SatWatch) -> String {
  format!("{:.2},{:.2},{:.0}", w.lat, w.lon, w.min_elev)
}

#[derive(Default)]
struct Cache {
  key: String,
  fetched_ms: f64,
  passes: Vec<Pass>,
  inflight: bool,
}

thread_local! {
  static CACHE: RefCell<Cache> = RefCell::default();
}

fn notifications_granted() -> bool {
  matches!(
    web_sys::Notification::permission(),
    web_sys::NotificationPermission::Granted
  )
}

fn tick() {
  let w = load();
  if !w.alerts || w.favorites.is_empty() || !notifications_granted() {
    return;
  }
  let now_ms = js_sys::Date::now();
  let key = query_key(&w);
  let stale =
    CACHE.with_borrow(|c| !c.inflight && (c.key != key || now_ms - c.fetched_ms > REFRESH_MS));
  if stale {
    CACHE.with_borrow_mut(|c| c.inflight = true);
    spawn_local(async move {
      let result = fetch_passes(&w).await;
      CACHE.with_borrow_mut(|c| {
        c.inflight = false;
        if let Ok(passes) = result {
          c.key = key;
          c.fetched_ms = js_sys::Date::now();
          c.passes = passes;
        }
      });
      check(&load());
    });
    return;
  }
  check(&w);
}

fn check(w: &SatWatch) {
  let now = (js_sys::Date::now() / 1000.0) as i64;
  let mut notified: Notified = storage::get_json(NOTIFIED_KEY).unwrap_or_default();
  let hits: Vec<Pass> = CACHE.with_borrow(|c| {
    w.due(&c.passes, &notified, now)
      .into_iter()
      .cloned()
      .collect()
  });
  if hits.is_empty() {
    return;
  }
  for p in &hits {
    let mins = ((p.aos - now) as f64 / 60.0).ceil() as i64;
    notify(&format!(
      "{} {mins} 分钟后过境（{}）· 最高 {:.0}° · {}方",
      p.name,
      local_hm(p.aos),
      p.max_elev,
      compass(p.azimuth)
    ));
  }
  let refs: Vec<&Pass> = hits.iter().collect();
  mark_notified(&mut notified, &refs, now);
  storage::set_json(NOTIFIED_KEY, &notified);
}

/// 启动全局过境提醒（在根组件调用一次）。
pub fn start_watcher() {
  tick();
  set_interval(tick, TICK);
}
