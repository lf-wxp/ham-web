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

use anyhow::Context;
use axum::Router;
use axum::body::Body;
use axum::extract::Request;
use axum::http::header::{CACHE_CONTROL, EXPIRES, HeaderValue, PRAGMA};
use axum::http::{StatusCode, Uri};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use tower::ServiceExt;
use tower_http::compression::CompressionLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

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
    "/" | "/index.html" | "/sw.js" | "/manifest.json" => Some(NO_STORE),
    p if p.starts_with("/questions/") && p.ends_with(".json") => Some("no-cache"),
    p if p.starts_with("/questions/images/") || p.starts_with("/fonts/") => {
      Some("public, max-age=86400")
    }
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
  match ServeFile::new(index).oneshot(req).await {
    Ok(res) => res.into_response(),
    Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
  }
}

fn app(cfg: &Config) -> Router {
  let index = cfg.dist.join("index.html");
  let fallback = tower::service_fn(move |req: Request<Body>| {
    let index = index.clone();
    async move {
      let uri = req.uri().clone();
      Ok::<_, std::convert::Infallible>(spa_fallback(index, uri, req).await)
    }
  });
  let static_files = ServeDir::new(&cfg.dist)
    .append_index_html_on_directories(true)
    .fallback(fallback);

  Router::new()
    .route("/healthz", get(|| async { "ok" }))
    .fallback_service(static_files)
    .layer(middleware::from_fn(cache_headers))
    .layer(CompressionLayer::new())
    .layer(TraceLayer::new_for_http())
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
  anyhow::ensure!(
    cfg.dist.join("index.html").is_file(),
    "{} 不存在，请先执行 `cargo make build-web`",
    cfg.dist.join("index.html").display()
  );

  let listener = tokio::net::TcpListener::bind(cfg.addr)
    .await
    .with_context(|| format!("failed to bind {}", cfg.addr))?;
  tracing::info!("serving {} on http://{}", cfg.dist.display(), cfg.addr);
  axum::serve(listener, app(&cfg))
    .with_graceful_shutdown(shutdown_signal())
    .await?;
  Ok(())
}

#[cfg(test)]
mod tests {
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
    assert_eq!(cache_policy("/pwa-icon.svg"), None);
  }
}
