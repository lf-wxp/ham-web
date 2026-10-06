//! 开放 API `/api/v1/*`：版本化、向后兼容、可跨域、独立配额。
//!
//! 与前端专用的同源代理 `/api/*` 物理隔离：内部接口随时可变，这里一经发布即稳定。
//! 首批只开放**纯计算与静态参考数据**（零上游成本）；引用型接口（太阳 / 中继台 /
//! POTA / SOTA）待复用服务端缓存后再开放。
//!
//! 契约：
//! - 成功：`{ "data": …, "meta": { "source": … } }`，带 `ETag`，支持 `If-None-Match` → 304；
//! - 失败：`{ "error": { "code": …, "message": … } }`；
//! - 配额：匿名与携带 `Authorization: Bearer <key>` 两档（key 取自 `API_KEYS`，逗号分隔，
//!   只做配额区分，不做敏感鉴权）；
//! - CORS：对 `/api/v1/*` 开放任意来源，内部 `/api/*` 保持同源收紧。

use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};

use axum::extract::{Query, Request};
use axum::http::header::{
  ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN,
  ACCESS_CONTROL_EXPOSE_HEADERS, ACCESS_CONTROL_MAX_AGE, AUTHORIZATION, CACHE_CONTROL,
  CONTENT_TYPE, ETAG, IF_NONE_MATCH, RETRY_AFTER,
};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use ham_web_core::api_v1::{CACHE_MAX_AGE_SECS, ENDPOINTS, openapi};
use ham_web_core::{bands, callsign, dxcc, grid, voacap};
use serde_json::{Value, json};

use crate::util::is_safe_token;

type Params = Query<HashMap<String, String>>;

/// 单页条数上限与默认值。
const MAX_LIMIT: usize = 200;
const DEFAULT_LIMIT: usize = 50;

// ───────────────────────── 错误 ─────────────────────────

/// API 错误：统一渲染为 `{ "error": { "code", "message" } }`。
pub struct ApiError {
  status: StatusCode,
  code: &'static str,
  message: String,
}

impl ApiError {
  fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
    Self {
      status,
      code,
      message: message.into(),
    }
  }
}

fn invalid(message: impl Into<String>) -> ApiError {
  ApiError::new(StatusCode::BAD_REQUEST, "invalid_parameter", message)
}

impl IntoResponse for ApiError {
  fn into_response(self) -> Response {
    let body = json!({ "error": { "code": self.code, "message": self.message } });
    (
      self.status,
      [
        (CONTENT_TYPE, "application/json"),
        (CACHE_CONTROL, "no-store"),
      ],
      body.to_string(),
    )
      .into_response()
  }
}

/// 超出配额时的响应（由限流中间件使用）。
pub fn rate_limited() -> Response {
  let mut res = ApiError::new(
    StatusCode::TOO_MANY_REQUESTS,
    "rate_limited",
    "rate limit exceeded, retry later or use an API key",
  )
  .into_response();
  res
    .headers_mut()
    .insert(RETRY_AFTER, HeaderValue::from_static("60"));
  res
}

// ───────────────────────── 响应 ─────────────────────────

/// `If-None-Match` 是否命中（容忍 `W/` 弱标记与多值）。
fn etag_matches(headers: &HeaderMap, etag: &str) -> bool {
  headers
    .get(IF_NONE_MATCH)
    .and_then(|v| v.to_str().ok())
    .is_some_and(|v| {
      v.split(',')
        .map(str::trim)
        .any(|t| t == "*" || t.trim_start_matches("W/") == etag)
    })
}

/// 发送 JSON 正文：附 `ETag` 与 `Cache-Control`，命中条件请求时返回 304。
fn send(headers: &HeaderMap, body: String) -> Response {
  let mut hasher = DefaultHasher::new();
  body.hash(&mut hasher);
  let etag = format!("\"{:016x}\"", hasher.finish());
  let cache = format!("public, max-age={CACHE_MAX_AGE_SECS}");
  if etag_matches(headers, &etag) {
    return (
      StatusCode::NOT_MODIFIED,
      [(ETAG, etag), (CACHE_CONTROL, cache)],
    )
      .into_response();
  }
  (
    StatusCode::OK,
    [
      (CONTENT_TYPE, "application/json".to_owned()),
      (ETAG, etag),
      (CACHE_CONTROL, cache),
    ],
    body,
  )
    .into_response()
}

/// 成功响应：`{ "data", "meta" }` 包装。
fn ok(headers: &HeaderMap, data: Value, meta: Value) -> Response {
  send(headers, json!({ "data": data, "meta": meta }).to_string())
}

fn source(name: &str) -> Value {
  json!({ "source": name })
}

// ───────────────────────── 参数 ─────────────────────────

fn text<'a>(q: &'a HashMap<String, String>, key: &str) -> Option<&'a str> {
  q.get(key).map(|s| s.trim()).filter(|s| !s.is_empty())
}

fn required<'a>(q: &'a HashMap<String, String>, key: &str) -> Result<&'a str, ApiError> {
  text(q, key).ok_or_else(|| invalid(format!("missing parameter `{key}`")))
}

fn number(q: &HashMap<String, String>, key: &str) -> Result<Option<f64>, ApiError> {
  match text(q, key) {
    None => Ok(None),
    Some(s) => s
      .parse::<f64>()
      .ok()
      .filter(|v| v.is_finite())
      .map(Some)
      .ok_or_else(|| invalid(format!("parameter `{key}` is not a valid number"))),
  }
}

fn required_number(q: &HashMap<String, String>, key: &str) -> Result<f64, ApiError> {
  number(q, key)?.ok_or_else(|| invalid(format!("missing parameter `{key}`")))
}

fn round(v: f64, places: i32) -> f64 {
  let p = 10f64.powi(places);
  (v * p).round() / p
}

fn entity_json(e: &dxcc::Entity) -> Value {
  json!({
    "dxcc": e.dxcc,
    "prefix": e.prefix,
    "name_zh": e.name,
    "name_en": e.name_en,
    "continent": e.continent,
    "cq_zone": e.cq,
    "itu_zone": e.itu,
    "lat": e.lat,
    "lon": e.lon,
  })
}

// ───────────────────────── 参考数据 ─────────────────────────

/// `GET /api/v1/dxcc`
pub async fn dxcc_list(headers: HeaderMap, Query(q): Params) -> Result<Response, ApiError> {
  let limit = match number(&q, "limit")? {
    None => DEFAULT_LIMIT,
    Some(v) if v.fract() == 0.0 && (1.0..=MAX_LIMIT as f64).contains(&v) => v as usize,
    Some(_) => {
      return Err(invalid(format!(
        "`limit` must be an integer in 1..={MAX_LIMIT}"
      )));
    }
  };
  let start = match text(&q, "cursor") {
    None => 0,
    Some(c) => c
      .parse::<usize>()
      .map_err(|_| invalid("`cursor` is invalid"))?,
  };
  let all = dxcc::entities();
  if start > all.len() {
    return Err(invalid("`cursor` is out of range"));
  }
  let end = (start + limit).min(all.len());
  let items: Vec<Value> = all[start..end].iter().map(entity_json).collect();
  let next = (end < all.len()).then(|| end.to_string());
  Ok(ok(
    &headers,
    Value::Array(items),
    json!({
      "source": "static",
      "total": all.len(),
      "limit": limit,
      "next_cursor": next,
    }),
  ))
}

/// `GET /api/v1/dxcc/lookup`
pub async fn dxcc_lookup(headers: HeaderMap, Query(q): Params) -> Result<Response, ApiError> {
  let call = required(&q, "callsign")?.to_ascii_uppercase();
  if !(3..=20).contains(&call.len()) || !is_safe_token(&call, &['/']) {
    return Err(invalid("`callsign` must be 3-20 letters, digits or `/`"));
  }
  let Some((matched, entity)) = dxcc::lookup_with_prefix(&call) else {
    return Err(ApiError::new(
      StatusCode::NOT_FOUND,
      "not_found",
      "callsign does not belong to any DXCC entity",
    ));
  };
  let parts = callsign::parse_callsign(&call);
  Ok(ok(
    &headers,
    json!({
      "callsign": call,
      "matched_prefix": matched,
      "entity": entity_json(entity),
      "parts": {
        "prefix": parts.prefix,
        "entity": parts.entity,
        "station_type": parts.station_type,
        "area": parts.area,
        "area_regions": parts.area_regions,
        "suffix": parts.suffix,
        "slash": parts.slash,
      },
    }),
    source("static"),
  ))
}

/// `GET /api/v1/bands`
pub async fn bands_list(headers: HeaderMap) -> Response {
  let items: Vec<Value> = bands::BANDS
    .iter()
    .map(|b| {
      let allocations: Vec<Value> = b
        .allocations
        .iter()
        .map(|a| {
          json!({
            "range": a.range,
            "satellite": a.is_satellite(),
            "usage": a.usage.label(),
          })
        })
        .collect();
      json!({
        "number": b.number,
        "name": b.name,
        "wavelength": b.wavelength,
        "frequency_name": b.freq_name,
        "frequency_abbr": b.freq_abbr,
        "frequency_range": b.freq_range,
        "microwave": b.microwave,
        "allocations": allocations,
      })
    })
    .collect();
  ok(&headers, Value::Array(items), source("static"))
}

// ───────────────────────── 网格计算 ─────────────────────────

/// `GET /api/v1/grid/to-latlon`
pub async fn grid_to_latlon(headers: HeaderMap, Query(q): Params) -> Result<Response, ApiError> {
  let g = required(&q, "grid")?;
  let (lat, lon) = grid::lat_lon_from_grid(g)
    .ok_or_else(|| invalid("`grid` must be a 4- or 6-character Maidenhead locator"))?;
  Ok(ok(
    &headers,
    json!({ "grid": g.to_ascii_uppercase(), "lat": round(lat, 5), "lon": round(lon, 5) }),
    source("computed"),
  ))
}

/// `GET /api/v1/grid/from-latlon`
pub async fn grid_from_latlon(headers: HeaderMap, Query(q): Params) -> Result<Response, ApiError> {
  let lat = required_number(&q, "lat")?;
  let lon = required_number(&q, "lon")?;
  let g = grid::grid_from_lat_lon(lat, lon)
    .ok_or_else(|| invalid("`lat` must be in -90..=90 and `lon` in -180..=180"))?;
  Ok(ok(
    &headers,
    json!({ "grid": g, "lat": lat, "lon": lon }),
    source("computed"),
  ))
}

/// `GET /api/v1/grid/distance`
pub async fn grid_distance(headers: HeaderMap, Query(q): Params) -> Result<Response, ApiError> {
  let from = required(&q, "from")?;
  let to = required(&q, "to")?;
  let bad = || invalid("`from` and `to` must be 4- or 6-character Maidenhead locators");
  let (lat1, lon1) = grid::lat_lon_from_grid(from).ok_or_else(bad)?;
  let (lat2, lon2) = grid::lat_lon_from_grid(to).ok_or_else(bad)?;
  let (km, bearing) = grid::distance_bearing(lat1, lon1, lat2, lon2);
  let (_, reverse) = grid::distance_bearing(lat2, lon2, lat1, lon1);
  Ok(ok(
    &headers,
    json!({
      "from": from.to_ascii_uppercase(),
      "to": to.to_ascii_uppercase(),
      "distance_km": round(km, 1),
      "bearing_deg": round(bearing, 1),
      "reverse_bearing_deg": round(reverse, 1),
    }),
    source("computed"),
  ))
}

// ───────────────────────── 传播预测 ─────────────────────────

/// `GET /api/v1/propagation/muf`
pub async fn propagation_muf(headers: HeaderMap, Query(q): Params) -> Result<Response, ApiError> {
  let tx = required(&q, "tx")?;
  let rx = required(&q, "rx")?;
  let month = match number(&q, "month")? {
    None => 10,
    Some(v) if v.fract() == 0.0 && (1.0..=12.0).contains(&v) => v as u32,
    Some(_) => return Err(invalid("`month` must be an integer in 1..=12")),
  };
  let ssn = number(&q, "ssn")?.unwrap_or(100.0);
  if !(0.0..=400.0).contains(&ssn) {
    return Err(invalid("`ssn` must be in 0..=400"));
  }
  let hour = number(&q, "hour")?;
  if hour.is_some_and(|h| !(0.0..=24.0).contains(&h)) {
    return Err(invalid("`hour` must be in 0..=24"));
  }
  let prediction = voacap::predict_at(tx, rx, month, ssn, hour)
    .ok_or_else(|| invalid("`tx` and `rx` must be 4- or 6-character Maidenhead locators"))?;
  let data = serde_json::to_value(&prediction).map_err(|_| {
    ApiError::new(
      StatusCode::INTERNAL_SERVER_ERROR,
      "internal",
      "failed to serialize prediction",
    )
  })?;
  Ok(ok(&headers, data, source("computed")))
}

// ───────────────────────── 元信息 ─────────────────────────

/// `GET /api/v1/status`
pub async fn status(headers: HeaderMap) -> Response {
  let endpoints: Vec<&str> = ENDPOINTS.iter().map(|e| e.path).collect();
  ok(
    &headers,
    json!({ "status": "ok", "version": "v1", "endpoints": endpoints }),
    source("static"),
  )
}

/// `GET /api/v1/openapi.json`
pub async fn openapi_json(headers: HeaderMap) -> Response {
  send(&headers, openapi().to_string())
}

/// `/api/v1/*` 下未定义的路径：返回 JSON 404，而不是落入 SPA 回退的 `index.html`。
pub async fn not_found() -> ApiError {
  ApiError::new(StatusCode::NOT_FOUND, "not_found", "unknown endpoint")
}

// ───────────────────────── 鉴权与跨域 ─────────────────────────

/// 从 `API_KEYS`（逗号分隔）读取 API key 集合。
pub fn load_keys() -> HashSet<String> {
  std::env::var("API_KEYS")
    .unwrap_or_default()
    .split(',')
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .map(str::to_owned)
    .collect()
}

/// 请求是否携带有效 key（`Authorization: Bearer <key>`）。key 只用于区分配额档位。
pub fn is_authorized(headers: &HeaderMap, keys: &HashSet<String>) -> bool {
  headers
    .get(AUTHORIZATION)
    .and_then(|v| v.to_str().ok())
    .and_then(|v| v.strip_prefix("Bearer "))
    .map(str::trim)
    .is_some_and(|k| keys.contains(k))
}

/// `/api/v1/*` 的 CORS：开放任意来源（只读 GET），并直接答复预检请求。
pub async fn cors(req: Request, next: Next) -> Response {
  if !req.uri().path().starts_with("/api/v1/") {
    return next.run(req).await;
  }
  if req.method() == Method::OPTIONS {
    return (
      StatusCode::NO_CONTENT,
      [
        (ACCESS_CONTROL_ALLOW_ORIGIN, "*"),
        (ACCESS_CONTROL_ALLOW_METHODS, "GET, OPTIONS"),
        (ACCESS_CONTROL_ALLOW_HEADERS, "Authorization, If-None-Match"),
        (ACCESS_CONTROL_MAX_AGE, "86400"),
      ],
    )
      .into_response();
  }
  let mut res = next.run(req).await;
  let h = res.headers_mut();
  h.insert(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
  h.insert(
    ACCESS_CONTROL_EXPOSE_HEADERS,
    HeaderValue::from_static("ETag, Retry-After"),
  );
  res
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn etag_matching_handles_weak_and_lists() {
    let mut h = HeaderMap::new();
    h.insert(IF_NONE_MATCH, HeaderValue::from_static("\"a\", W/\"b\""));
    assert!(etag_matches(&h, "\"a\""));
    assert!(etag_matches(&h, "\"b\""));
    assert!(!etag_matches(&h, "\"c\""));
    h.insert(IF_NONE_MATCH, HeaderValue::from_static("*"));
    assert!(etag_matches(&h, "\"anything\""));
    assert!(!etag_matches(&HeaderMap::new(), "\"a\""));
  }

  #[test]
  fn bearer_key_selects_quota_tier() {
    let keys: HashSet<String> = ["k1".to_owned()].into();
    let mut h = HeaderMap::new();
    assert!(!is_authorized(&h, &keys));
    h.insert(AUTHORIZATION, HeaderValue::from_static("Bearer k1"));
    assert!(is_authorized(&h, &keys));
    h.insert(AUTHORIZATION, HeaderValue::from_static("Bearer nope"));
    assert!(!is_authorized(&h, &keys));
    h.insert(AUTHORIZATION, HeaderValue::from_static("Basic k1"));
    assert!(!is_authorized(&h, &keys));
  }

  #[test]
  fn rounding_is_stable() {
    assert_eq!(round(1.23456, 2), 1.23);
    assert_eq!(round(9014.96, 1), 9015.0);
  }
}
