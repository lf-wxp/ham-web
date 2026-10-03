//! 静态站点服务器：托管 Trunk 构建产物（`dist/`）。
//!
//! - 前端路由（无扩展名的路径）回退到 `index.html`；
//! - `sw.js` / `manifest.json` / `index.html` 禁止缓存，保证 PWA 能及时发现新版本；
//! - 带内容哈希的构建产物长期缓存；
//! - gzip / brotli 压缩；
//! - `GET /healthz` 健康检查。
//!
//! 环境变量：`HOST`（默认 `0.0.0.0`）、`PORT`（默认 `3000`）、`DIST_DIR`（默认 `dist`）。

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use axum::Router;
use axum::body::Body;
use axum::extract::Request;
use axum::http::header::{CACHE_CONTROL, EXPIRES, HeaderValue, PRAGMA};
use axum::http::{StatusCode, Uri};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use tower::ServiceExt;
use tower_http::compression::CompressionLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

mod alerts;
mod api_v1;
mod cache;
mod callsign;
mod geocode;
mod iss;
mod most_wanted;
mod passes;
mod portable;
mod psk_reporter;
mod push;
mod rate_limit;
mod rbn;
mod repeaters;
mod solar;
mod spots;
mod util;
mod voacap;
mod xray;

const NO_STORE: &str = "no-cache, no-store, must-revalidate";
const IMMUTABLE: &str = "public, max-age=31536000, immutable";

#[derive(Clone)]
struct Config {
  addr: SocketAddr,
  dist: PathBuf,
}

impl Config {
  fn from_env() -> anyhow::Result<Self> {
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_owned());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_owned());
    let addr = format!("{host}:{port}")
      .parse()
      .with_context(|| format!("invalid listen address {host}:{port}"))?;
    let dist = PathBuf::from(std::env::var("DIST_DIR").unwrap_or_else(|_| "dist".to_owned()));
    Ok(Self { addr, dist })
  }
}

/// 判断文件名是否带 Trunk 生成的内容哈希，如 `ham-web-app-1a2b3c4d5e6f7a8b_bg.wasm`。
fn is_hashed_asset(path: &str) -> bool {
  let file = path.rsplit('/').next().unwrap_or_default();
  let stem = file.split('.').next().unwrap_or_default();
  stem
    .split(['-', '_'])
    .any(|seg| seg.len() >= 12 && seg.chars().all(|c| c.is_ascii_hexdigit()))
}

fn cache_policy(path: &str) -> Option<&'static str> {
  match path {
    "/" | "/index.html" | "/sw.js" | "/manifest.json" | "/changelog.json" => Some(NO_STORE),
    p if p.starts_with("/questions/") && p.ends_with(".json") => Some("no-cache"),
    // DXCC 几何二进制：无内容哈希，用协商缓存保证重新生成后能拿到新版本。
    "/dxcc-entities.bin" => Some("no-cache"),
    p if p.starts_with("/questions/images/") || p.starts_with("/fonts/") => {
      Some("public, max-age=86400")
    }
    // 开放 API 自带 ETag 与 Cache-Control（见 `api_v1::send`），这里不能覆盖成 no-store。
    p if p.starts_with("/api/v1/") => None,
    p if p.starts_with("/api/") => Some(NO_STORE),
    p if is_hashed_asset(p) => Some(IMMUTABLE),
    _ => None,
  }
}

async fn cache_headers(req: Request, next: Next) -> Response {
  let path = req.uri().path().to_owned();
  let mut res = next.run(req).await;
  let is_html = res
    .headers()
    .get(axum::http::header::CONTENT_TYPE)
    .and_then(|v| v.to_str().ok())
    .is_some_and(|v| v.starts_with("text/html"));
  let policy = if is_html {
    Some(NO_STORE)
  } else {
    cache_policy(&path)
  };
  if let Some(policy) = policy {
    let headers = res.headers_mut();
    headers.insert(CACHE_CONTROL, HeaderValue::from_static(policy));
    if policy == NO_STORE {
      headers.insert(PRAGMA, HeaderValue::from_static("no-cache"));
      headers.insert(EXPIRES, HeaderValue::from_static("0"));
    }
  }
  res
}

/// 资源不存在时：带扩展名的路径返回 404，其余视为前端路由返回 `index.html`。
async fn spa_fallback(index: PathBuf, uri: Uri, req: Request<Body>) -> Response {
  let last = uri.path().rsplit('/').next().unwrap_or_default();
  if last.contains('.') {
    return (StatusCode::NOT_FOUND, "Not Found").into_response();
  }
  match ServeFile::new(index)
    .precompressed_br()
    .precompressed_gzip()
    .oneshot(req)
    .await
  {
    Ok(res) => res.into_response(),
    Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
  }
}

fn app(cfg: &Config, push_service: push::PushService) -> Router {
  let index = cfg.dist.join("index.html");
  let fallback = tower::service_fn(move |req: Request<Body>| {
    let index = index.clone();
    async move {
      let uri = req.uri().clone();
      Ok::<_, std::convert::Infallible>(spa_fallback(index, uri, req).await)
    }
  });
  let static_files = ServeDir::new(&cfg.dist)
    .precompressed_br()
    .precompressed_gzip()
    .append_index_html_on_directories(true)
    .fallback(fallback);

  // /api 限流：默认每 IP 每分钟 120 次，可通过 `RATE_LIMIT_PER_MIN` 覆盖。
  let rl_limit = std::env::var("RATE_LIMIT_PER_MIN")
    .ok()
    .and_then(|s| s.parse::<usize>().ok())
    .unwrap_or(120);
  let rate_limiter = Arc::new(rate_limit::RateLimiter::new(
    rl_limit,
    std::time::Duration::from_secs(60),
  ));
  // 开放 API（/api/v1）使用独立配额，避免第三方流量挤占前端：
  // 匿名一档，携带 `Authorization: Bearer <key>`（`API_KEYS`）另一档。
  let anon_limit = std::env::var("API_V1_RATE_LIMIT_PER_MIN")
    .ok()
    .and_then(|s| s.parse::<usize>().ok())
    .unwrap_or(ham_web_core::api_v1::ANON_LIMIT_PER_MIN);
  let v1_anon = Arc::new(rate_limit::RateLimiter::new(
    anon_limit,
    Duration::from_secs(60),
  ));
  let v1_keyed = Arc::new(rate_limit::RateLimiter::new(
    ham_web_core::api_v1::KEYED_LIMIT_PER_MIN,
    Duration::from_secs(60),
  ));
  let api_keys = Arc::new(api_v1::load_keys());
  let limit_mw = {
    let limiter = rate_limiter.clone();
    move |req: Request, next: Next| {
      let limiter = limiter.clone();
      let v1_anon = v1_anon.clone();
      let v1_keyed = v1_keyed.clone();
      let api_keys = api_keys.clone();
      async move {
        let path = req.uri().path();
        if !path.starts_with("/api/") {
          return next.run(req).await;
        }
        let is_v1 = path.starts_with("/api/v1/");
        let active = if !is_v1 {
          &limiter
        } else if api_v1::is_authorized(req.headers(), &api_keys) {
          &v1_keyed
        } else {
          &v1_anon
        };
        // 连接信息由 `into_make_service_with_connect_info` 注入到 extensions。
        let conn = req
          .extensions()
          .get::<axum::extract::ConnectInfo<SocketAddr>>()
          .cloned();
        if rate_limit::allow(active, conn, &req) {
          next.run(req).await
        } else if is_v1 {
          api_v1::rate_limited()
        } else {
          (StatusCode::TOO_MANY_REQUESTS, "too many requests").into_response()
        }
      }
    }
  };

  let solar_cache = Arc::new(solar::Cache::default());
  let solar_route = {
    let cache = solar_cache.clone();
    move || solar::handler(cache.clone())
  };

  let passes_cache = Arc::new(passes::Cache::default());
  let passes_route = {
    let cache = passes_cache.clone();
    move |query: axum::extract::Query<passes::PassQuery>| passes::handler(query, cache.clone())
  };

  let wanted_cache = Arc::new(most_wanted::Cache::default());
  let wanted_route = {
    let cache = wanted_cache.clone();
    move || most_wanted::handler(cache.clone())
  };

  let spots_cache = Arc::new(spots::Cache::default());
  let spots_route = {
    let cache = spots_cache.clone();
    move || spots::handler(cache.clone())
  };

  let alerts_cache = Arc::new(alerts::Cache::default());
  let alerts_route = {
    let cache = alerts_cache.clone();
    move || alerts::handler(cache.clone())
  };

  let iss_cache = Arc::new(iss::Cache::default());
  let iss_route = {
    let cache = iss_cache.clone();
    move || iss::handler(cache.clone())
  };

  let geocode_cache = Arc::new(geocode::Cache::default());
  let geocode_route = {
    let cache = geocode_cache.clone();
    move |query: axum::extract::Query<geocode::GeocodeQuery>| geocode::handler(query, cache.clone())
  };

  let voacap_cache = Arc::new(voacap::Cache::default());
  let voacap_route = {
    let cache = voacap_cache.clone();
    move |query: axum::extract::Query<voacap::VoacapQuery>| voacap::handler(query, cache.clone())
  };

  let psk_cache = Arc::new(psk_reporter::Cache::default());
  let psk_route = {
    let cache = psk_cache.clone();
    move |query: axum::extract::Query<psk_reporter::PskQuery>| {
      psk_reporter::handler(query, cache.clone())
    }
  };

  let rbn_cache = Arc::new(rbn::Cache::default());
  let rbn_route = {
    let cache = rbn_cache.clone();
    move |query: axum::extract::Query<rbn::RbnQuery>| rbn::handler(query, cache.clone())
  };

  let callsign_cache = Arc::new(callsign::Cache::default());
  let callsign_route = {
    let cache = callsign_cache.clone();
    move |query: axum::extract::Query<callsign::CallsignQuery>| {
      callsign::handler(query, cache.clone())
    }
  };

  let portable_cache = Arc::new(portable::Cache::default());
  let sota_route = {
    let cache = portable_cache.clone();
    move |query: axum::extract::Query<portable::SotaQuery>| {
      portable::sota_handler(query, cache.clone())
    }
  };
  let pota_route = {
    let cache = portable_cache.clone();
    move |query: axum::extract::Query<portable::PotaQuery>| {
      portable::pota_handler(query, cache.clone())
    }
  };

  let repeaters_cache = Arc::new(repeaters::Cache::default());
  let repeaters_route = {
    let cache = repeaters_cache.clone();
    move |query: axum::extract::Query<repeaters::RepeaterQuery>| {
      repeaters::handler(query, cache.clone())
    }
  };

  let xray_cache = Arc::new(xray::Cache::default());
  let xray_route = {
    let cache = xray_cache.clone();
    move || xray::handler(cache.clone())
  };

  let push_public = {
    let s = push_service.clone();
    move || push::public_key_handler(s.clone())
  };
  let push_subscribe = {
    let s = push_service.clone();
    move |headers: axum::http::HeaderMap, body: String| {
      push::subscribe_handler(s.clone(), headers, body)
    }
  };
  let push_unsubscribe = {
    let s = push_service.clone();
    move |headers: axum::http::HeaderMap, body: String| {
      push::unsubscribe_handler(s.clone(), headers, body)
    }
  };
  let push_review_count = {
    let s = push_service.clone();
    move |headers: axum::http::HeaderMap, body: String| {
      push::review_count_handler(s.clone(), headers, body)
    }
  };

  Router::new()
    .route("/healthz", get(|| async { "ok" }))
    .route("/api/solar", get(solar_route))
    .route("/api/passes", get(passes_route))
    .route("/api/most-wanted", get(wanted_route))
    .route("/api/spots", get(spots_route))
    .route("/api/alerts", get(alerts_route))
    .route("/api/iss", get(iss_route))
    .route("/api/geocode", get(geocode_route))
    .route("/api/voacap", get(voacap_route))
    .route("/api/psk-reporter", get(psk_route))
    .route("/api/rbn", get(rbn_route))
    .route("/api/callsign", get(callsign_route))
    .route("/api/sota", get(sota_route))
    .route("/api/pota", get(pota_route))
    .route("/api/repeaters", get(repeaters_route))
    .route("/api/xray", get(xray_route))
    .route("/api/push/vapid-public-key", get(push_public))
    .route("/api/push/subscribe", post(push_subscribe))
    .route("/api/push/unsubscribe", post(push_unsubscribe))
    .route("/api/push/review-count", post(push_review_count))
    // 开放 API v1：纯计算 / 静态参考数据，契约见 `ham_web_core::api_v1`。
    .route("/api/v1/dxcc", get(api_v1::dxcc_list))
    .route("/api/v1/dxcc/lookup", get(api_v1::dxcc_lookup))
    .route("/api/v1/bands", get(api_v1::bands_list))
    .route("/api/v1/grid/to-latlon", get(api_v1::grid_to_latlon))
    .route("/api/v1/grid/from-latlon", get(api_v1::grid_from_latlon))
    .route("/api/v1/grid/distance", get(api_v1::grid_distance))
    .route("/api/v1/propagation/muf", get(api_v1::propagation_muf))
    .route("/api/v1/status", get(api_v1::status))
    .route("/api/v1/openapi.json", get(api_v1::openapi_json))
    .route("/api/v1/{*rest}", get(api_v1::not_found))
    .fallback_service(static_files)
    .layer(middleware::from_fn(cache_headers))
    .layer(middleware::from_fn(limit_mw))
    // CORS 放在限流外层：429 响应与预检请求同样带跨域头。
    .layer(middleware::from_fn(api_v1::cors))
    .layer(CompressionLayer::new())
    // 兜底超时：上游已各自配置 ureq 超时（见 `util::http_agent`），这一层防止
    // handler 内部因单飞等待、telnet 读取或本地计算异常而无限挂起占住连接。
    .layer(TimeoutLayer::with_status_code(
      StatusCode::GATEWAY_TIMEOUT,
      request_timeout(),
    ))
    .layer(TraceLayer::new_for_http())
}

/// 单个请求的兜底超时，可用 `REQUEST_TIMEOUT_SECS` 覆盖（默认 60 秒）。
fn request_timeout() -> Duration {
  std::env::var("REQUEST_TIMEOUT_SECS")
    .ok()
    .and_then(|s| s.parse::<u64>().ok())
    .filter(|&s| s > 0)
    .map_or(Duration::from_secs(60), Duration::from_secs)
}

async fn shutdown_signal() {
  let ctrl_c = async {
    let _ = tokio::signal::ctrl_c().await;
  };
  #[cfg(unix)]
  let terminate = async {
    if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
      s.recv().await;
    }
  };
  #[cfg(not(unix))]
  let terminate = std::future::pending::<()>();
  tokio::select! {
    () = ctrl_c => {},
    () = terminate => {},
  }
  tracing::info!("shutting down");
}

/// `ham-web-server healthcheck`：请求本机 `/healthz`，供容器 HEALTHCHECK 使用（运行镜像内无 curl）。
async fn healthcheck() -> anyhow::Result<()> {
  use tokio::io::{AsyncReadExt, AsyncWriteExt};
  let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_owned());
  let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}")).await?;
  stream
    .write_all(b"GET /healthz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
    .await?;
  let mut buf = Vec::new();
  stream.read_to_end(&mut buf).await?;
  anyhow::ensure!(buf.starts_with(b"HTTP/1.1 200"), "unhealthy");
  Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  if std::env::args().nth(1).as_deref() == Some("healthcheck") {
    return healthcheck().await;
  }

  tracing_subscriber::fmt()
    .with_env_filter(
      tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,tower_http=warn".into()),
    )
    .init();

  let cfg = Config::from_env()?;
  // 开发联调（仅提供 /api 代理）时允许跳过静态产物检查
  if std::env::var("SKIP_DIST_CHECK").is_err() {
    anyhow::ensure!(
      cfg.dist.join("index.html").is_file(),
      "{} 不存在，请先执行 `cargo make build-web`",
      cfg.dist.join("index.html").display()
    );
  }

  let push_service = push::PushService::default();
  push_service.init()?;
  push_service.clone().spawn_reminder_loop();
  if push::token_required() {
    tracing::warn!(
      "已设置 PUSH_API_TOKEN：/api/push/subscribe 与 /unsubscribe 需要 Bearer 令牌。\
       浏览器前端是共享的静态资源，无法携带该令牌，因此官方前端将无法订阅；\
       除非你自行构建前端并注入 Authorization 头，否则请勿设置此变量。"
    );
  }

  let listener = tokio::net::TcpListener::bind(cfg.addr)
    .await
    .with_context(|| format!("failed to bind {}", cfg.addr))?;
  tracing::info!("serving {} on http://{}", cfg.dist.display(), cfg.addr);
  axum::serve(
    listener,
    app(&cfg, push_service).into_make_service_with_connect_info::<SocketAddr>(),
  )
  .with_graceful_shutdown(shutdown_signal())
  .await?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use std::fs;
  use std::path::PathBuf;
  use std::sync::atomic::{AtomicU64, Ordering};

  use super::*;

  #[test]
  fn detects_hashed_assets() {
    assert!(is_hashed_asset("/ham-web-app-8a1f0c2d9e3b4a5c_bg.wasm"));
    assert!(is_hashed_asset("/input-0123456789abcdef.css"));
    assert!(!is_hashed_asset("/pwa-icon-192.png"));
    assert!(!is_hashed_asset("/manifest.json"));
  }

  #[test]
  fn cache_policies() {
    assert_eq!(cache_policy("/sw.js"), Some(NO_STORE));
    assert_eq!(cache_policy("/questions/A.json"), Some("no-cache"));
    assert_eq!(cache_policy("/dxcc-entities.bin"), Some("no-cache"));
    assert_eq!(cache_policy("/pwa-icon.svg"), None);
    // 内部代理接口 no-store；开放 API 自管缓存头。
    assert_eq!(cache_policy("/api/solar"), Some(NO_STORE));
    assert_eq!(cache_policy("/api/v1/dxcc"), None);
  }

  /// 带 `index.html` 的临时静态目录，测试结束自动清理（RAII）。
  struct TempDist {
    path: PathBuf,
  }

  impl TempDist {
    fn new() -> Self {
      static COUNTER: AtomicU64 = AtomicU64::new(0);
      let n = COUNTER.fetch_add(1, Ordering::Relaxed);
      let path =
        std::env::temp_dir().join(format!("ham-web-server-test-{}-{n}", std::process::id()));
      fs::create_dir_all(&path).expect("create temp dist");
      fs::write(path.join("index.html"), "<html>app</html>").expect("write index.html");
      Self { path }
    }
  }

  impl Drop for TempDist {
    fn drop(&mut self) {
      let _ = fs::remove_dir_all(&self.path);
    }
  }

  fn config(dist: &TempDist) -> Config {
    Config {
      addr: "127.0.0.1:0".parse().expect("valid addr"),
      dist: dist.path.clone(),
    }
  }

  fn get(uri: &str) -> axum::extract::Request {
    axum::extract::Request::builder()
      .uri(uri)
      .body(Body::empty())
      .expect("valid request")
  }

  #[tokio::test]
  async fn healthz_returns_ok() {
    let dist = TempDist::new();
    let res = app(&config(&dist), push::PushService::default())
      .oneshot(get("/healthz"))
      .await
      .expect("response");
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
      .await
      .expect("body");
    assert_eq!(&body[..], b"ok");
  }

  #[tokio::test]
  async fn spa_fallback_serves_index_html() {
    let dist = TempDist::new();
    let res = app(&config(&dist), push::PushService::default())
      .oneshot(get("/exam/A"))
      .await
      .expect("response");
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
      .await
      .expect("body");
    assert_eq!(&body[..], b"<html>app</html>");
  }

  #[tokio::test]
  async fn missing_asset_returns_404() {
    let dist = TempDist::new();
    let res = app(&config(&dist), push::PushService::default())
      .oneshot(get("/missing.js"))
      .await
      .expect("response");
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
  }

  /// 请求开放 API，返回 `(状态码, 响应头, JSON 正文)`。
  async fn v1(uri: &str) -> (StatusCode, axum::http::HeaderMap, serde_json::Value) {
    let dist = TempDist::new();
    let res = app(&config(&dist), push::PushService::default())
      .oneshot(get(uri))
      .await
      .expect("response");
    let status = res.status();
    let headers = res.headers().clone();
    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
      .await
      .expect("body");
    let json = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
    (status, headers, json)
  }

  /// 契约表里登记的每个端点都必须真实可路由（不带参数时应是 400 / 200，而不是 404）。
  #[tokio::test]
  async fn every_documented_endpoint_is_routed() {
    for e in ham_web_core::api_v1::ENDPOINTS {
      let (status, _, _) = v1(e.path).await;
      assert_ne!(status, StatusCode::NOT_FOUND, "{} 已登记但未挂路由", e.path);
    }
  }

  #[tokio::test]
  async fn v1_unknown_path_is_json_404_not_spa_html() {
    let (status, _, body) = v1("/api/v1/nope").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "not_found");
  }

  #[tokio::test]
  async fn v1_success_is_wrapped_cached_and_cors_enabled() {
    let (status, headers, body) = v1("/api/v1/dxcc/lookup?callsign=ba1xx").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["callsign"], "BA1XX");
    assert_eq!(body["data"]["entity"]["dxcc"], 318);
    assert_eq!(body["meta"]["source"], "static");
    assert_eq!(headers["access-control-allow-origin"], "*");
    assert!(
      headers["cache-control"]
        .to_str()
        .expect("ascii")
        .starts_with("public, max-age=")
    );
    assert!(headers.contains_key("etag"));
  }

  #[tokio::test]
  async fn v1_conditional_request_returns_304() {
    let dist = TempDist::new();
    let first = app(&config(&dist), push::PushService::default())
      .oneshot(get("/api/v1/bands"))
      .await
      .expect("response");
    let etag = first.headers()["etag"].clone();
    let req = axum::extract::Request::builder()
      .uri("/api/v1/bands")
      .header("if-none-match", etag)
      .body(Body::empty())
      .expect("valid request");
    let second = app(&config(&dist), push::PushService::default())
      .oneshot(req)
      .await
      .expect("response");
    assert_eq!(second.status(), StatusCode::NOT_MODIFIED);
  }

  #[tokio::test]
  async fn v1_validates_parameters_with_json_errors() {
    for uri in [
      "/api/v1/dxcc/lookup",
      "/api/v1/dxcc/lookup?callsign=a%20b",
      "/api/v1/dxcc?limit=0",
      "/api/v1/dxcc?limit=201",
      "/api/v1/grid/to-latlon?grid=ZZ",
      "/api/v1/grid/from-latlon?lat=91&lon=0",
      "/api/v1/grid/from-latlon?lat=abc&lon=0",
      "/api/v1/propagation/muf?tx=OM89&rx=IO91&month=13",
      "/api/v1/propagation/muf?tx=OM89&rx=IO91&hour=25",
    ] {
      let (status, _, body) = v1(uri).await;
      assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}");
      assert_eq!(body["error"]["code"], "invalid_parameter", "{uri}");
    }
  }

  #[tokio::test]
  async fn v1_unassignable_callsign_is_404() {
    let (status, _, body) = v1("/api/v1/dxcc/lookup?callsign=BG4XXX/MM").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "not_found");
  }

  #[tokio::test]
  async fn v1_dxcc_list_paginates_with_cursor() {
    let (_, _, first) = v1("/api/v1/dxcc?limit=5").await;
    assert_eq!(first["data"].as_array().map(Vec::len), Some(5));
    let cursor = first["meta"]["next_cursor"].as_str().expect("应有下一页");
    let (status, _, second) = v1(&format!("/api/v1/dxcc?limit=5&cursor={cursor}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_ne!(first["data"][0]["dxcc"], second["data"][0]["dxcc"]);
    // 翻到最后一页后不再给 next_cursor。
    let total = first["meta"]["total"].as_u64().expect("total");
    let (_, _, last) = v1(&format!("/api/v1/dxcc?limit=200&cursor={}", total - 1)).await;
    assert_eq!(last["data"].as_array().map(Vec::len), Some(1));
    assert!(last["meta"]["next_cursor"].is_null());
  }

  #[tokio::test]
  async fn v1_computations_match_core() {
    let (_, _, g) = v1("/api/v1/grid/from-latlon?lat=0&lon=0").await;
    assert_eq!(g["data"]["grid"], "JJ00AA");
    let (_, _, d) = v1("/api/v1/grid/distance?from=FN31&to=FN31").await;
    assert_eq!(d["data"]["distance_km"], 0.0);
    let (status, _, m) = v1("/api/v1/propagation/muf?tx=OM89&rx=IO91&hour=12").await;
    assert_eq!(status, StatusCode::OK);
    assert!(m["data"]["muf"].as_f64().is_some_and(|v| v > 0.0));
    assert_eq!(m["data"]["bands"].as_array().map(Vec::len), Some(10));
  }

  #[tokio::test]
  async fn v1_preflight_is_answered_with_cors_headers() {
    let dist = TempDist::new();
    let req = axum::extract::Request::builder()
      .method("OPTIONS")
      .uri("/api/v1/dxcc")
      .body(Body::empty())
      .expect("valid request");
    let res = app(&config(&dist), push::PushService::default())
      .oneshot(req)
      .await
      .expect("response");
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    assert_eq!(res.headers()["access-control-allow-origin"], "*");
  }

  #[tokio::test]
  async fn internal_api_stays_same_origin() {
    let dist = TempDist::new();
    let res = app(&config(&dist), push::PushService::default())
      .oneshot(get("/healthz"))
      .await
      .expect("response");
    assert!(!res.headers().contains_key("access-control-allow-origin"));
  }
}
