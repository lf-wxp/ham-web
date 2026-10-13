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

use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

use axum::extract::{FromRequestParts, Query, Request};
use axum::http::header::{
  ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN,
  ACCESS_CONTROL_EXPOSE_HEADERS, ACCESS_CONTROL_MAX_AGE, AUTHORIZATION, CACHE_CONTROL,
  CONTENT_TYPE, ETAG, IF_NONE_MATCH, RETRY_AFTER,
};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use ham_web_core::api_v1::{CACHE_MAX_AGE_SECS, ENDPOINTS, openapi};
use ham_web_core::filter_design::{self, FilterKind, ResonatorTopology};
use ham_web_core::link_budget;
use ham_web_core::unit_math;
use ham_web_core::{bands, callsign, dxcc, grid, voacap};
use serde_json::{Value, json};

use crate::util::is_safe_token;

/// 查询串参数表。
///
/// 故意不直接用 `Query<HashMap<..>>`：axum 对它的默认拒绝是**纯文本 400**，会绕过本
/// 模块的统一错误结构（`ham_web_core::api_v1` 的契约：失败一律
/// `{ error: { code, message } }`）。查询串不可解码（如非法 UTF-8 百分号转义）时正好
/// 走那条路，所以这里自己接一层。
pub struct Params(pub HashMap<String, String>);

impl<S: Send + Sync> FromRequestParts<S> for Params {
  type Rejection = ApiError;

  async fn from_request_parts(
    parts: &mut axum::http::request::Parts,
    state: &S,
  ) -> Result<Self, Self::Rejection> {
    match Query::<HashMap<String, String>>::from_request_parts(parts, state).await {
      Ok(Query(q)) => Ok(Params(q)),
      Err(_) => Err(invalid("query string is not valid UTF-8")),
    }
  }
}

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
  // 用 SHA-256 前 8 字节而不是 `DefaultHasher`：后者的算法不受 std API 稳定性承诺，
  // 标准库换实现会让所有 ETag 一次性全变（缓存穿透）。64 位截断对条件请求的碰撞
  // 概率足够低，且 SHA-256 不依赖进程内随机种子（多副本部署下 ETag 一致）。
  let digest = Sha256::digest(body.as_bytes());
  let etag = format!(
    "\"{}\"",
    digest[..8]
      .iter()
      .map(|b| format!("{b:02x}"))
      .collect::<String>()
  );
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
  // `+ 0.0`：把 `-0.0`（如 `ratio_to_db(0.99999)` 取整后）规范成 `0.0`，不让 JSON 里出现 `-0.0`。
  (v * p).round() / p + 0.0
}

/// 保留 `digits` 位有效数字。
///
/// 线性量（mW、W、元件值）跨越十几个数量级：固定小数位会把 −100 dBm（1e-10 mW）这类
/// 常见灵敏度输入整个抹成 `0.0`。借 `{:e}` 格式化取整，避免手算 `10^k` 在极端量级上溢出。
fn round_sig(v: f64, digits: usize) -> f64 {
  if v == 0.0 || !v.is_finite() {
    return v;
  }
  format!("{v:.digits$e}", digits = digits.saturating_sub(1))
    .parse::<f64>()
    .map_or(v, |r| r + 0.0)
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
pub async fn dxcc_list(headers: HeaderMap, params: Params) -> Result<Response, ApiError> {
  let limit = match number(&params.0, "limit")? {
    None => DEFAULT_LIMIT,
    Some(v) if v.fract() == 0.0 && (1.0..=MAX_LIMIT as f64).contains(&v) => v as usize,
    Some(_) => {
      return Err(invalid(format!(
        "`limit` must be an integer in 1..={MAX_LIMIT}"
      )));
    }
  };
  let start = match text(&params.0, "cursor") {
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
pub async fn dxcc_lookup(headers: HeaderMap, params: Params) -> Result<Response, ApiError> {
  let call = required(&params.0, "callsign")?.to_ascii_uppercase();
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

// ───────────────────────── 单位换算 ─────────────────────────

/// `GET /api/v1/units/convert`
pub async fn units_convert(headers: HeaderMap, params: Params) -> Result<Response, ApiError> {
  // 单位名大小写不敏感（`dBm` / `mW` 是最自然的写法）。
  let from = required(&params.0, "from")?.to_ascii_lowercase();
  let to = required(&params.0, "to")?.to_ascii_lowercase();
  let value = required_number(&params.0, "value")?;
  let result = match (from.as_str(), to.as_str()) {
    ("mw", "w") => Some(round_sig(value / 1000.0, 6)),
    ("w", "mw") => Some(round_sig(value * 1000.0, 6)),
    ("mw", "dbm") => unit_math::mw_to_dbm(value).map(|v| round(v, 3)),
    ("w", "dbm") => unit_math::watts_to_dbm(value).map(|v| round(v, 3)),
    ("dbm", "mw") => unit_math::dbm_to_mw(value).map(|v| round_sig(v, 6)),
    ("dbm", "w") => unit_math::dbm_to_mw(value).map(|v| round_sig(v / 1000.0, 6)),
    ("db", "ratio") => unit_math::db_to_ratio(value).map(|v| round_sig(v, 6)),
    ("ratio", "db") => unit_math::ratio_to_db(value).map(|v| round(v, 3)),
    ("mhz", "m") => unit_math::mhz_to_wavelength(value).map(|v| round(v, 3)),
    ("m", "mhz") => unit_math::wavelength_to_mhz(value).map(|v| round(v, 4)),
    _ => {
      return Err(invalid(
        "unsupported unit pair; supported: mw<->dbm, w<->dbm, mw<->w, db<->ratio, mhz<->m",
      ));
    }
  };
  // 定义域外（如 mW ≤ 0）与结果溢出都落到这里：`round` 的放大步骤本身也可能溢出，
  // 所以有限性要在取整之后再查 —— 否则 `inf` 会被序列化成 `null`。
  let result = result
    .filter(|v| v.is_finite())
    .ok_or_else(|| invalid("`value` is out of range for this conversion"))?;
  Ok(ok(
    &headers,
    json!({ "from": from, "to": to, "value": value, "result": result }),
    source("computed"),
  ))
}

// ───────────────────────── 滤波器设计 ─────────────────────────

/// `GET /api/v1/filter/design`
pub async fn filter_design(headers: HeaderMap, params: Params) -> Result<Response, ApiError> {
  let kind = required(&params.0, "kind")?;
  let n = required_order(&params.0)?;
  let fc_hz = number(&params.0, "fc_hz")?;
  let f0_hz = number(&params.0, "f0_hz")?;
  let bw_hz = number(&params.0, "bw_hz")?;
  let z0 = number(&params.0, "z0")?.unwrap_or(50.0);
  // 有限性一并查：`z0=inf` / `NaN` 会穿过 `<= 0` 进入设计函数。
  if !z0.is_finite() || z0 <= 0.0 {
    return Err(invalid("`z0` must be positive"));
  }
  let stages = match kind {
    "lowpass" => {
      let fc = fc_hz.ok_or_else(|| invalid("`fc_hz` is required for lowpass"))?;
      design(n, fc, 0.0, z0, FilterKind::LowPass)?
    }
    "highpass" => {
      let fc = fc_hz.ok_or_else(|| invalid("`fc_hz` is required for highpass"))?;
      design(n, fc, 0.0, z0, FilterKind::HighPass)?
    }
    "bandpass" => {
      let (f0, bw) = center_and_bw(f0_hz, bw_hz)?;
      design(n, f0, bw, z0, FilterKind::BandPass)?
    }
    "bandstop" => {
      let (f0, bw) = center_and_bw(f0_hz, bw_hz)?;
      design(n, f0, bw, z0, FilterKind::BandStop)?
    }
    _ => {
      return Err(invalid(
        "`kind` must be lowpass / highpass / bandpass / bandstop",
      ));
    }
  };
  let items = stages
    .iter()
    .map(|st| {
      Ok(json!({
        "series": st.series,
        "l_uh": st.l_uh.map(round3_finite).transpose()?,
        "c_pf": st.c_pf.map(round3_finite).transpose()?,
        "topology": topology_label(st.topology),
      }))
    })
    .collect::<Result<Vec<Value>, ApiError>>()?;
  Ok(ok(
    &headers,
    json!({ "kind": kind, "n": n, "z0": z0, "stages": items }),
    source("computed"),
  ))
}

/// 滤波器阶数：必须是 `1..=MAX_ORDER` 的整数（`2.7` 不能被静默截成 2 再回显）。
///
/// 上限取自 g 值表实际覆盖的阶数：此前放行到 9 而表只有 1–5，6–9 阶会 200 + 空 `stages`。
fn required_order(q: &HashMap<String, String>) -> Result<usize, ApiError> {
  let n = required_number(q, "n")?;
  if n.fract() != 0.0 || !(1.0..=filter_design::MAX_ORDER as f64).contains(&n) {
    return Err(invalid(format!(
      "`n` must be an integer in 1..={}",
      filter_design::MAX_ORDER
    )));
  }
  Ok(n as usize)
}

fn design(
  n: usize,
  fc_hz: f64,
  bw_hz: f64,
  z0: f64,
  kind: FilterKind,
) -> Result<Vec<filter_design::FilterStage>, ApiError> {
  if fc_hz <= 0.0 || bw_hz < 0.0 {
    return Err(invalid("frequencies must be positive"));
  }
  let stages = match kind {
    FilterKind::LowPass => filter_design::design_lowpass(n, fc_hz, z0),
    FilterKind::HighPass => filter_design::design_highpass(n, fc_hz, z0),
    FilterKind::BandPass => filter_design::design_bandpass(n, fc_hz, bw_hz, z0),
    FilterKind::BandStop => filter_design::design_bandstop(n, fc_hz, bw_hz, z0),
  };
  // 防御：任何阶数没有 g 值时不能以 200 + 空数组冒充成功。
  if stages.is_empty() {
    return Err(invalid("this filter order is not supported"));
  }
  Ok(stages)
}

fn center_and_bw(f0: Option<f64>, bw: Option<f64>) -> Result<(f64, f64), ApiError> {
  let f0 = f0.ok_or_else(|| invalid("`f0_hz` is required for bandpass / bandstop"))?;
  let bw = bw.ok_or_else(|| invalid("`bw_hz` is required for bandpass / bandstop"))?;
  if f0 <= 0.0 || bw <= 0.0 {
    return Err(invalid("`f0_hz` and `bw_hz` must be positive"));
  }
  Ok((f0, bw))
}

fn topology_label(t: ResonatorTopology) -> &'static str {
  match t {
    ResonatorTopology::Single => "single",
    ResonatorTopology::SeriesLc => "series_lc",
    ResonatorTopology::ParallelLc => "parallel_lc",
  }
}

/// 元件值取 6 位有效数字（高频下 L 只有 1e-5 μH 量级，固定 3 位小数会归零）；
/// 极端频率会让元件值溢出为 `inf`（序列化成 `null`），按越界拒绝。
fn round3_finite(v: f64) -> Result<f64, ApiError> {
  Some(round_sig(v, 6))
    .filter(|r| r.is_finite())
    .ok_or_else(|| invalid("frequency is out of range for a component value"))
}

// ───────────────────────── 链路预算 ─────────────────────────

/// `GET /api/v1/link-budget`
pub async fn link_budget(headers: HeaderMap, params: Params) -> Result<Response, ApiError> {
  let tx_dbm = required_number(&params.0, "tx_dbm")?;
  let tx_gain = required_number(&params.0, "tx_gain_dbi")?;
  let rx_gain = required_number(&params.0, "rx_gain_dbi")?;
  let freq = required_number(&params.0, "freq_mhz")?;
  let dist = required_number(&params.0, "dist_km")?;
  let extra = number(&params.0, "extra_loss_db")?.unwrap_or(0.0);
  let fspl = link_budget::free_space_path_loss_db(freq, dist)
    .ok_or_else(|| invalid("`freq_mhz` and `dist_km` must be positive"))?;
  let rx = link_budget::received_dbm(tx_dbm, tx_gain, rx_gain, freq, dist, extra)
    .map(|v| round(v, 3))
    .filter(|v| v.is_finite())
    .ok_or_else(|| invalid("power, gain and loss values are out of range"))?;
  Ok(ok(
    &headers,
    json!({
      "fspl_db": round(fspl, 3),
      "rx_dbm": rx,
      "tx_dbm": tx_dbm,
      "freq_mhz": freq,
      "dist_km": dist,
    }),
    source("computed"),
  ))
}

// ───────────────────────── 网格计算 ─────────────────────────

/// `GET /api/v1/grid/to-latlon`
pub async fn grid_to_latlon(headers: HeaderMap, params: Params) -> Result<Response, ApiError> {
  let g = required(&params.0, "grid")?;
  let (lat, lon) = grid::lat_lon_from_grid(g)
    .ok_or_else(|| invalid("`grid` must be a 4- or 6-character Maidenhead locator"))?;
  Ok(ok(
    &headers,
    json!({ "grid": g.to_ascii_uppercase(), "lat": round(lat, 5), "lon": round(lon, 5) }),
    source("computed"),
  ))
}

/// `GET /api/v1/grid/from-latlon`
pub async fn grid_from_latlon(headers: HeaderMap, params: Params) -> Result<Response, ApiError> {
  let lat = required_number(&params.0, "lat")?;
  let lon = required_number(&params.0, "lon")?;
  let g = grid::grid_from_lat_lon(lat, lon)
    .ok_or_else(|| invalid("`lat` must be in -90..=90 and `lon` in -180..=180"))?;
  Ok(ok(
    &headers,
    json!({ "grid": g, "lat": lat, "lon": lon }),
    source("computed"),
  ))
}

/// `GET /api/v1/grid/distance`
pub async fn grid_distance(headers: HeaderMap, params: Params) -> Result<Response, ApiError> {
  let from = required(&params.0, "from")?;
  let to = required(&params.0, "to")?;
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
pub async fn propagation_muf(headers: HeaderMap, params: Params) -> Result<Response, ApiError> {
  let tx = required(&params.0, "tx")?;
  let rx = required(&params.0, "rx")?;
  let month = match number(&params.0, "month")? {
    None => 10,
    Some(v) if v.fract() == 0.0 && (1.0..=12.0).contains(&v) => v as u32,
    Some(_) => return Err(invalid("`month` must be an integer in 1..=12")),
  };
  let ssn = number(&params.0, "ssn")?.unwrap_or(100.0);
  if !(0.0..=400.0).contains(&ssn) {
    return Err(invalid("`ssn` must be in 0..=400"));
  }
  let hour = number(&params.0, "hour")?;
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

/// 路径是否属于开放 API v1（含裸前缀 `/api/v1`）。
///
/// CORS / 限流 / 缓存策略三处必须用同一个判断：只匹配 `/api/v1/` 时，裸 `/api/v1`
/// 会漏掉 CORS 头、走内部限流（超限返回纯文本 429）、并被 `/api/` 的 no-store 覆盖缓存头。
#[must_use]
pub fn is_v1_path(path: &str) -> bool {
  path == "/api/v1" || path.starts_with("/api/v1/")
}

/// 提取请求携带的、在 `keys` 集合中的 API key（`Bearer` 方案）。key 只用于区分配额档位。
///
/// 限流中间件用它把「有效 key 的请求」按 **key 值**分桶：不同 key 互不挤占配额，
/// 也不能靠换来源 IP 绕过自己的额度。无效 key 返回 `None`，落回匿名桶。
#[must_use]
pub fn bearer_key(headers: &HeaderMap, keys: &HashSet<String>) -> Option<String> {
  headers
    .get(AUTHORIZATION)
    .and_then(|v| v.to_str().ok())
    // RFC 7235：认证方案名大小写不敏感。
    .and_then(|v| v.split_once(' '))
    .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
    .map(|(_, key)| key.trim().to_owned())
    .filter(|k| keys.contains(k))
}

/// `/api/v1/*` 的 CORS：开放任意来源（只读 GET），并直接答复预检请求。
pub async fn cors(req: Request, next: Next) -> Response {
  if !is_v1_path(req.uri().path()) {
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

  async fn body(res: Response) -> Value {
    let (_, body) = res.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX).await.expect("body");
    serde_json::from_slice(&bytes).expect("json")
  }

  fn params(q: &[(&str, &str)]) -> Params {
    Params(
      q.iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect(),
    )
  }

  async fn call(
    fut: impl std::future::Future<Output = Result<Response, ApiError>>,
  ) -> Result<Value, Value> {
    match fut.await {
      Ok(res) => Ok(body(res).await),
      Err(e) => Err(body(e.into_response()).await),
    }
  }

  #[tokio::test]
  async fn units_convert_computes_known_values() {
    let p = |q: &[(&str, &str)]| params(q);
    let ok = call(units_convert(
      HeaderMap::new(),
      p(&[("from", "dbm"), ("to", "mw"), ("value", "30")]),
    ))
    .await
    .expect("ok");
    assert_eq!(ok["data"]["result"], 1000.0);
    let ok = call(units_convert(
      HeaderMap::new(),
      p(&[("from", "mhz"), ("to", "m"), ("value", "146")]),
    ))
    .await
    .expect("ok");
    assert_eq!(ok["data"]["result"], 2.055);
    let ok = call(units_convert(
      HeaderMap::new(),
      p(&[("from", "ratio"), ("to", "db"), ("value", "2")]),
    ))
    .await
    .expect("ok");
    assert_eq!(ok["data"]["result"], 3.01);
    let err = call(units_convert(
      HeaderMap::new(),
      p(&[("from", "dbm"), ("to", "feet"), ("value", "1")]),
    ))
    .await
    .expect_err("err");
    assert_eq!(err["error"]["code"], "invalid_parameter");
  }

  #[tokio::test]
  async fn filter_design_returns_stages() {
    let p = params(&[("kind", "lowpass"), ("n", "3"), ("fc_hz", "7100000")]);
    let ok = call(filter_design(HeaderMap::new(), p)).await.expect("ok");
    assert_eq!(ok["data"]["stages"].as_array().map(Vec::len), Some(3));
    // 巴特沃斯 3 阶低通：串-并-串，g 值 1/2/1。
    let stages = ok["data"]["stages"].as_array().expect("array");
    assert_eq!(stages[0]["topology"], "single");
    assert_eq!(stages[1]["topology"], "single");
    assert_eq!(stages[2]["topology"], "single");
    assert_eq!(stages[0]["series"], true);
    assert_eq!(stages[1]["series"], false);
    assert_eq!(stages[2]["series"], true);
    // 阶数越界要报错而不是返回空表。
    let err = call(filter_design(
      HeaderMap::new(),
      params(&[("kind", "lowpass"), ("n", "10"), ("fc_hz", "1000")]),
    ))
    .await
    .expect_err("err");
    assert_eq!(err["error"]["code"], "invalid_parameter");
  }

  #[tokio::test]
  async fn link_budget_matches_the_classic_example() {
    let p = params(&[
      ("tx_dbm", "20"),
      ("tx_gain_dbi", "2"),
      ("rx_gain_dbi", "2"),
      ("freq_mhz", "146"),
      ("dist_km", "100"),
    ]);
    let ok = call(link_budget(HeaderMap::new(), p)).await.expect("ok");
    assert_eq!(ok["data"]["fspl_db"], 115.737);
    assert_eq!(ok["data"]["rx_dbm"], -91.737);
  }

  #[tokio::test]
  async fn numeric_edges_are_rejected_instead_of_serialising_null() {
    let code = |e: Value| e["error"]["code"].clone();
    // `dbm=3100` → 10^310 mW 溢出成 inf（序列化会变 null）。有效数字取整不再有「放大步骤」，
    // 所以 3050（10^305，仍有限）现在应当正常返回，只有真正溢出的才拒绝。
    let finite = call(units_convert(
      HeaderMap::new(),
      params(&[("from", "dbm"), ("to", "mw"), ("value", "3050")]),
    ))
    .await
    .expect("10^305 仍是有限值");
    assert_eq!(finite["data"]["result"], 1e305);
    let err = call(units_convert(
      HeaderMap::new(),
      params(&[("from", "dbm"), ("to", "mw"), ("value", "3100")]),
    ))
    .await
    .expect_err("err");
    assert_eq!(code(err), "invalid_parameter");
    // 阶数必须是整数：2.7 不能被截成 2。
    let err = call(filter_design(
      HeaderMap::new(),
      params(&[("kind", "lowpass"), ("n", "2.7"), ("fc_hz", "1000")]),
    ))
    .await
    .expect_err("err");
    assert_eq!(code(err), "invalid_parameter");
    // 极小频率让元件值溢出。
    let err = call(filter_design(
      HeaderMap::new(),
      params(&[("kind", "lowpass"), ("n", "3"), ("fc_hz", "1e-300")]),
    ))
    .await
    .expect_err("err");
    assert_eq!(code(err), "invalid_parameter");
    // 各项有限、求和溢出：返回 400，而不是 expect 崩溃或 null。
    let err = call(link_budget(
      HeaderMap::new(),
      params(&[
        ("tx_dbm", "1e308"),
        ("tx_gain_dbi", "1e308"),
        ("rx_gain_dbi", "1e308"),
        ("freq_mhz", "146"),
        ("dist_km", "100"),
      ]),
    ))
    .await
    .expect_err("err");
    assert_eq!(code(err), "invalid_parameter");
  }

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
  fn bearer_scheme_is_case_insensitive() {
    let keys: HashSet<String> = ["k1".to_owned()].into();
    for v in ["bearer k1", "BEARER k1", "Bearer  k1 "] {
      let mut h = HeaderMap::new();
      h.insert(AUTHORIZATION, HeaderValue::from_str(v).expect("ascii"));
      assert!(bearer_key(&h, &keys).is_some(), "{v}");
    }
  }

  #[test]
  fn rounding_keeps_small_magnitudes_and_never_emits_negative_zero() {
    // −100 dBm = 1e-10 mW：固定 4 位小数会得到 0.0。
    assert_eq!(round_sig(1e-10, 6), 1e-10);
    assert_eq!(round_sig(1.234_567_89e-7, 6), 1.234_57e-7);
    assert_eq!(round_sig(0.0, 6), 0.0);
    assert!(round_sig(f64::INFINITY, 6).is_infinite());
    assert_eq!(round_sig(123_456_789.0, 6), 123_457_000.0);
    assert!(round(-0.000_01, 3).is_sign_positive(), "不能输出 -0.0");
    assert!(round_sig(-0.0, 6).is_sign_positive() || round_sig(-0.0, 6) == 0.0);
  }

  #[tokio::test]
  async fn units_convert_does_not_flatten_sensitivity_levels_to_zero() {
    let v = call(units_convert(
      HeaderMap::new(),
      params(&[("from", "dbm"), ("to", "mw"), ("value", "-100")]),
    ))
    .await
    .expect("ok");
    assert_eq!(v["data"]["result"], 1e-10);
  }

  #[tokio::test]
  async fn filter_orders_above_the_g_table_are_rejected() {
    for n in ["6", "9", "0", "10"] {
      let err = call(filter_design(
        HeaderMap::new(),
        params(&[("kind", "lowpass"), ("n", n), ("fc_hz", "1e6")]),
      ))
      .await
      .expect_err("err");
      assert_eq!(err["error"]["code"], "invalid_parameter", "n={n}");
    }
    let ok = call(filter_design(
      HeaderMap::new(),
      params(&[("kind", "lowpass"), ("n", "5"), ("fc_hz", "1e6")]),
    ))
    .await
    .expect("ok");
    assert_eq!(ok["data"]["stages"].as_array().map(Vec::len), Some(5));
  }

  #[test]
  fn bearer_key_selects_quota_tier() {
    let keys: HashSet<String> = ["k1".to_owned()].into();
    let mut h = HeaderMap::new();
    assert!(bearer_key(&h, &keys).is_none());
    h.insert(AUTHORIZATION, HeaderValue::from_static("Bearer k1"));
    assert_eq!(bearer_key(&h, &keys).as_deref(), Some("k1"));
    h.insert(AUTHORIZATION, HeaderValue::from_static("Bearer nope"));
    assert!(bearer_key(&h, &keys).is_none());
    h.insert(AUTHORIZATION, HeaderValue::from_static("Basic k1"));
    assert!(bearer_key(&h, &keys).is_none());
  }

  #[test]
  fn rounding_is_stable() {
    assert_eq!(round(1.23456, 2), 1.23);
    assert_eq!(round(9014.96, 1), 9015.0);
  }
}
