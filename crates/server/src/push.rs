//! Web Push 服务：VAPID 密钥、订阅存储、RFC 8291 内容加密与推送发送。
//!
//! 完整链路：前端 `PushManager.subscribe` 拿到订阅（endpoint + `p256dh`/`auth` 密钥）
//! → `POST /api/push/subscribe` 上报本服务 → 定时任务到点（每日提醒时间）用 VAPID
//! 私钥签发 JWT、按 RFC 8291 加密消息，POST 到推送服务的 endpoint。
//!
//! VAPID 密钥对通过环境变量注入：`VAPID_PRIVATE_KEY`（base64url，32 字节标量）、
//! `VAPID_PUBLIC_KEY`（base64url，65 字节未压缩点）、`VAPID_SUBJECT`（默认
//! `mailto:admin@example.com`）。未配置时启动会自动生成（重启后公钥变化，订阅需重订）。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use aes_gcm::aead::{Aead, Nonce, Payload};
use aes_gcm::{Aes128Gcm, KeyInit};
use anyhow::{Context, bail};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{Datelike, Timelike, Utc};
use hkdf::Hkdf;
use p256::ecdh::EphemeralSecret;
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use p256::elliptic_curve::Generate;
use p256::{PublicKey, SecretKey};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::cache::{json_err, json_ok};

/// 加密算法对应的记录大小字段（RFC 8188，实际以单条记录发送）。
const RECORD_SIZE: u32 = 4096;
/// 推送服务建议的存活时间（秒）。
const TTL_SECS: u32 = 60;
/// 定时检查提醒的间隔。
const TICK_SECS: u64 = 60;
/// 持久化文件路径的环境变量；未设置时退化为内存存储（重启丢失）。
const PUSH_STORE_ENV: &str = "PUSH_STORE";
/// 订阅数量上限，防止公开接口被刷导致 `PUSH_STORE` 无限膨胀。
const MAX_SUBSCRIPTIONS: usize = 10_000;

/// 受信任的浏览器推送服务域名后缀（白名单，防 SSRF）。
///
/// 订阅 `endpoint` 由客户端任意上报，若不校验，本服务会在定时提醒时向任意 URL
/// （含内网 / 云元数据地址）发送带 VAPID 签名的请求，构成 SSRF 风险。此处仅允许
/// `https` + 域名后缀白名单。
const TRUSTED_PUSH_HOSTS: &[&str] = &[
  "googleapis.com",     // fcm.googleapis.com
  "push.apple.com",     // web.push.apple.com
  "mozilla.com",        // updates.push.services.mozilla.com
  "notify.windows.com", // *.notify.windows.com
];

/// 浏览器上报的推送订阅。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
  pub endpoint: String,
  pub keys: SubscriptionKeys,
  /// 每日学习提醒的 UTC 分钟数（0–1439，来自用户本地时间 + 时区换算）；缺省则不调度。
  #[serde(default)]
  pub reminder_utc_minutes: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionKeys {
  pub p256dh: String,
  pub auth: String,
}

/// VAPID 密钥对（私钥只存内存，公钥对外分发）。
#[derive(Clone)]
struct VapidKeys {
  private: SecretKey,
  public_b64: String,
  subject: String,
}

/// 持久化到 `PUSH_STORE` 文件的数据：VAPID 私钥 + 订阅表 + 每日提醒去重状态。
#[derive(Default, Serialize, Deserialize)]
struct StoreData {
  /// VAPID 私钥标量（base64url，32 字节）。
  #[serde(default)]
  vapid_private_b64: String,
  #[serde(default)]
  subscriptions: HashMap<String, Subscription>,
  /// endpoint → 上次发送提醒的「纪元日」编号，保证重启后同一天不重复发送。
  #[serde(default)]
  last_reminded: HashMap<String, i32>,
}

/// 服务状态：VAPID 密钥 + 订阅表 + 每日提醒去重 + 持久化路径。
#[derive(Default)]
struct State {
  vapid: Option<VapidKeys>,
  subscriptions: HashMap<String, Subscription>,
  /// endpoint → 上次发送提醒的「纪元日」编号，保证每天只发一次。
  last_reminded: HashMap<String, i32>,
  /// 持久化文件路径；`None` 时退化为内存存储。
  store: Option<PathBuf>,
}

impl State {
  /// 把 VAPID 私钥与订阅表写回持久化文件（未配置路径时为空操作）。
  fn persist(&self) {
    let Some(path) = &self.store else {
      return;
    };
    let data = StoreData {
      vapid_private_b64: self
        .vapid
        .as_ref()
        .map_or_else(String::new, VapidKeys::private_b64),
      subscriptions: self.subscriptions.clone(),
      last_reminded: self.last_reminded.clone(),
    };
    persist_store(path, &data);
  }
}

/// 可跨线程共享的推送服务。
#[derive(Clone, Default)]
pub struct PushService {
  inner: Arc<Mutex<State>>,
}

/// base64url（无 padding）编码。
fn b64url(bytes: &[u8]) -> String {
  URL_SAFE_NO_PAD.encode(bytes)
}

/// base64url 解码。
fn unb64url(s: &str) -> anyhow::Result<Vec<u8>> {
  URL_SAFE_NO_PAD
    .decode(s.as_bytes())
    .with_context(|| "invalid base64url")
}

/// 读取持久化文件（不存在或损坏时返回空）。
fn load_store(path: &Path) -> StoreData {
  std::fs::read_to_string(path)
    .ok()
    .and_then(|s| serde_json::from_str(&s).ok())
    .unwrap_or_default()
}

/// 写入持久化文件（失败仅告警，不影响服务）。
///
/// VAPID 私钥以明文存储于此文件，写入时显式限制为仅属主可读写（0600），
/// 避免同机其他用户读取私钥。
fn persist_store(path: &Path, data: &StoreData) {
  if let Ok(json) = serde_json::to_string_pretty(data)
    && let Err(e) = write_private(path, json.as_bytes())
  {
    tracing::warn!("persist push store {} failed: {e}", path.display());
  }
}

/// 以仅属主可读写（0600）的权限写入文件；非 Unix 平台回退到普通写入。
#[cfg(unix)]
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
  use std::io::Write;
  use std::os::unix::fs::OpenOptionsExt;
  let mut f = std::fs::OpenOptions::new()
    .write(true)
    .create(true)
    .truncate(true)
    .mode(0o600)
    .open(path)?;
  f.write_all(bytes)
}

#[cfg(not(unix))]
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
  std::fs::write(path, bytes)
}

impl VapidKeys {
  /// 由私钥标量（base64url）构造。
  fn from_private_b64(b64: &str, subject: &str) -> anyhow::Result<Self> {
    let raw = unb64url(b64)?;
    let private = SecretKey::from_slice(&raw).context("VAPID 私钥无效")?;
    let public_b64 = b64url(&private.public_key().to_sec1_bytes());
    Ok(Self {
      private,
      public_b64,
      subject: subject.to_owned(),
    })
  }

  /// 随机生成一对。
  fn generate(subject: &str) -> Self {
    let private = SecretKey::generate();
    let public_b64 = b64url(&private.public_key().to_sec1_bytes());
    Self {
      private,
      public_b64,
      subject: subject.to_owned(),
    }
  }

  /// 私钥标量（base64url）。
  fn private_b64(&self) -> String {
    b64url(self.private.to_bytes().as_slice())
  }
}

/// 由订阅 endpoint 提取 origin（如 `https://push.example.com`），作为 VAPID JWT 的 `aud`。
fn endpoint_origin(endpoint: &str) -> Option<String> {
  let scheme_end = endpoint.find("://")? + 3;
  let rest = &endpoint[scheme_end..];
  let host_end = rest.find('/').unwrap_or(rest.len());
  Some(endpoint[..scheme_end + host_end].to_owned())
}

/// 校验 endpoint 是否指向受信任的推送服务（`https` + 域名后缀白名单）。
fn valid_endpoint(endpoint: &str) -> bool {
  let Some(rest) = endpoint.strip_prefix("https://") else {
    return false;
  };
  // host 取 `https://` 之后、首个 `/` 之前的部分。
  let host = rest.split('/').next().unwrap_or("");
  // 去掉可能的 userinfo（`user:pass@`）与端口（`:443`）。
  let host = host.rsplit('@').next().unwrap_or("");
  let host = host.split(':').next().unwrap_or("");
  if host.is_empty() {
    return false;
  }
  TRUSTED_PUSH_HOSTS.iter().any(|d| {
    host == *d
      || (host.len() > d.len()
        && host.ends_with(d)
        && host.as_bytes()[host.len() - d.len() - 1] == b'.')
  })
}

/// 签发 VAPID JWT（ES256，RFC 7515 的 raw R||S 签名）。
fn vapid_jwt(vapid: &VapidKeys, aud: &str) -> anyhow::Result<String> {
  let header = b64url(br#"{"typ":"JWT","alg":"ES256"}"#);
  let exp = (Utc::now().timestamp() + 12 * 3600) as u64;
  let claims = serde_json::json!({ "aud": aud, "exp": exp, "sub": vapid.subject });
  let payload = b64url(claims.to_string().as_bytes());
  let signing_input = format!("{header}.{payload}");

  let signing_key = SigningKey::from(&vapid.private);
  let sig: Signature = signing_key.sign(signing_input.as_bytes());
  let jwt = format!("{signing_input}.{}", b64url(sig.to_bytes().as_slice()));
  Ok(jwt)
}

/// RFC 8291 `aes128gcm` 内容加密：返回 (HTTP body, `Encryption` 头值, `Crypto-Key` 头值)。
fn encrypt_payload(
  plaintext: &[u8],
  client_p256dh_b64: &str,
) -> anyhow::Result<(Vec<u8>, String, String)> {
  let client_pub =
    PublicKey::from_sec1_bytes(&unb64url(client_p256dh_b64)?).context("订阅 p256dh 无效")?;
  let client_pub_bytes = client_pub.to_sec1_bytes().to_vec();

  let salt: [u8; 16] = Generate::generate();

  let server_secret = EphemeralSecret::generate();
  let server_pub_bytes = server_secret.public_key().to_sec1_bytes().to_vec();
  let shared = server_secret.diffie_hellman(&client_pub);
  let shared_bytes = shared.raw_secret_bytes();

  // HKDF-Extract(salt, ecdh_secret) 后 HKDF-Expand 一次得到 32 字节：CEK(16) + nonce(12)。
  let hk = Hkdf::<Sha256>::new(Some(&salt), shared_bytes);
  let mut okm = [0u8; 32];
  let mut info = Vec::with_capacity(14 + 65 + 65);
  info.extend_from_slice(b"WebPush: info\0");
  info.extend_from_slice(&client_pub_bytes);
  info.extend_from_slice(&server_pub_bytes);
  hk.expand(&info, &mut okm)
    .map_err(|_| anyhow::anyhow!("HKDF expand 失败"))?;
  let (cek, nonce_bytes) = (&okm[..16], &okm[16..28]);

  let cipher = Aes128Gcm::new_from_slice(cek).context("AES 密钥无效")?;
  let nonce =
    Nonce::<Aes128Gcm>::try_from(nonce_bytes).map_err(|_| anyhow::anyhow!("nonce 长度错误"))?;
  let ciphertext = cipher
    .encrypt(
      &nonce,
      Payload {
        msg: plaintext,
        aad: b"",
      },
    )
    .map_err(|_| anyhow::anyhow!("AES-GCM 加密失败"))?;

  // body = salt(16) || record-size(4) || idlen(1=0) || ciphertext
  let mut body = Vec::with_capacity(16 + 4 + 1 + ciphertext.len());
  body.extend_from_slice(&salt);
  body.extend_from_slice(&RECORD_SIZE.to_be_bytes());
  body.push(0);
  body.extend_from_slice(&ciphertext);

  let encryption = format!("salt={}", b64url(&salt));
  let crypto_key = format!("dh={}", b64url(&server_pub_bytes));
  Ok((body, encryption, crypto_key))
}

/// 发送一条推送；返回是否成功（`false` 表示订阅已失效，应删除）。
fn send_push(
  vapid: &VapidKeys,
  sub: &Subscription,
  title: &str,
  body: &str,
) -> anyhow::Result<bool> {
  let aud = endpoint_origin(&sub.endpoint).context("endpoint origin 无法解析")?;
  let jwt = vapid_jwt(vapid, &aud)?;
  let payload = serde_json::json!({ "title": title, "body": body, "url": "/" });
  let (body_bytes, encryption, crypto_key) =
    encrypt_payload(payload.to_string().as_bytes(), &sub.keys.p256dh)?;

  let res = ureq::post(&sub.endpoint)
    .header("Content-Encoding", "aes128gcm")
    .header("TTL", &TTL_SECS.to_string())
    .header("Encryption", &encryption)
    .header("Crypto-Key", &crypto_key)
    .header(
      "Authorization",
      &format!("vapid t={jwt}, k={}", vapid.public_b64),
    )
    .header("Content-Type", "application/octet-stream")
    .send(&body_bytes)
    .with_context(|| "推送发送失败")?;

  match res.status().as_u16() {
    200..=202 => Ok(true),
    404 | 410 => Ok(false),
    code => bail!("推送服务返回 HTTP {code}"),
  }
}

impl PushService {
  /// 初始化 VAPID 密钥与订阅存储，并启动每日提醒定时任务。
  ///
  /// VAPID 私钥优先级：`VAPID_PRIVATE_KEY` 环境变量 > `PUSH_STORE` 文件 > 随机生成。
  /// 首次生成时会随订阅一起写回 `PUSH_STORE`，保证重启后密钥与订阅都不丢失。
  pub fn init(&self) -> anyhow::Result<()> {
    let subject =
      std::env::var("VAPID_SUBJECT").unwrap_or_else(|_| "mailto:admin@example.com".to_owned());
    let store = std::env::var(PUSH_STORE_ENV).ok().map(PathBuf::from);
    let mut data = store.as_deref().map_or_else(StoreData::default, load_store);

    let vapid = match std::env::var("VAPID_PRIVATE_KEY") {
      Ok(pk) => VapidKeys::from_private_b64(&pk, &subject)?,
      Err(_) if !data.vapid_private_b64.is_empty() => {
        VapidKeys::from_private_b64(&data.vapid_private_b64, &subject)?
      }
      Err(_) => {
        let v = VapidKeys::generate(&subject);
        data.vapid_private_b64 = v.private_b64();
        v
      }
    };

    let mut state = self.inner.lock().expect("push state poisoned");
    state.vapid = Some(vapid);
    state.subscriptions = std::mem::take(&mut data.subscriptions);
    state.last_reminded = std::mem::take(&mut data.last_reminded);
    state.store = store;
    state.persist();
    if !state.subscriptions.is_empty() {
      tracing::info!("loaded {} push subscriptions", state.subscriptions.len());
    }
    Ok(())
  }

  /// 公开的 VAPID 公钥（base64url）。
  pub fn public_key(&self) -> String {
    self
      .inner
      .lock()
      .expect("push state poisoned")
      .vapid
      .as_ref()
      .map_or_else(String::new, |v| v.public_b64.clone())
  }

  /// 上报 / 更新订阅（以 endpoint 为键），并持久化；超上限时返回错误。
  pub fn subscribe(&self, sub: Subscription) -> Result<(), &'static str> {
    let mut state = self.inner.lock().expect("push state poisoned");
    if state.subscriptions.len() >= MAX_SUBSCRIPTIONS
      && !state.subscriptions.contains_key(&sub.endpoint)
    {
      return Err("subscription limit reached");
    }
    state.subscriptions.insert(sub.endpoint.clone(), sub);
    state.persist();
    Ok(())
  }

  /// 删除订阅，并持久化。
  pub fn unsubscribe(&self, endpoint: &str) {
    let mut state = self.inner.lock().expect("push state poisoned");
    state.subscriptions.remove(endpoint);
    state.persist();
  }

  /// 每分钟检查一次：对设定了提醒时间且到点的订阅发送「该学习啦」。
  pub fn check_reminders(&self) {
    let now = Utc::now();
    let minute = now.hour() * 60 + now.minute();
    let day = now.date_naive().num_days_from_ce();

    // 先在锁内收集到点且当天未发的订阅，并 clone 出 VAPID 密钥，释放锁后再发送。
    let (vapid, due): (Option<VapidKeys>, Vec<(String, Subscription)>) = {
      let state = self.inner.lock().expect("push state poisoned");
      let vapid = state.vapid.clone();
      let due = state
        .subscriptions
        .iter()
        .filter(|(endpoint, sub)| {
          sub.reminder_utc_minutes == Some(minute)
            && state.last_reminded.get(*endpoint) != Some(&day)
        })
        .map(|(e, s)| (e.clone(), s.clone()))
        .collect();
      (vapid, due)
    };

    let Some(vapid) = vapid else {
      return;
    };

    for (endpoint, sub) in due {
      match send_push(&vapid, &sub, "该学习啦", "今天的备考任务待完成。") {
        Ok(true) => {
          let mut state = self.inner.lock().expect("push state poisoned");
          state.last_reminded.insert(endpoint, day);
          tracing::info!("push reminder sent to {}", sub.endpoint);
        }
        Ok(false) => {
          let mut state = self.inner.lock().expect("push state poisoned");
          state.subscriptions.remove(&endpoint);
          tracing::info!("removed expired subscription {}", sub.endpoint);
        }
        Err(e) => tracing::warn!("push reminder failed: {e:#}"),
      }
    }
  }

  /// 后台循环：定时触发提醒检查。
  pub fn spawn_reminder_loop(self) {
    tokio::spawn(async move {
      let mut tick = tokio::time::interval(std::time::Duration::from_secs(TICK_SECS));
      loop {
        tick.tick().await;
        // `check_reminders` 内部用同步的 `ureq` 发送，放到 blocking 线程执行，
        // 避免阻塞 tokio worker 线程、拖累整个运行时。
        let this = self.clone();
        let _ = tokio::task::spawn_blocking(move || this.check_reminders()).await;
      }
    });
  }
}

// —— axum 处理器 ——

/// 校验 `Authorization: Bearer <token>`；未配置 `PUSH_API_TOKEN` 时不鉴权。
fn authorized(headers: &axum::http::HeaderMap) -> bool {
  let Ok(token) = std::env::var("PUSH_API_TOKEN") else {
    return true;
  };
  headers
    .get(axum::http::header::AUTHORIZATION)
    .and_then(|v| v.to_str().ok())
    .and_then(|v| v.strip_prefix("Bearer "))
    .is_some_and(|t| t == token)
}

/// `GET /api/push/vapid-public-key`：分发 VAPID 公钥。
pub async fn public_key_handler(service: PushService) -> axum::response::Response {
  let key = service.public_key();
  json_ok(serde_json::json!({ "publicKey": key }).to_string())
}

/// `POST /api/push/subscribe`：上报订阅（body 为 [`Subscription`]）。
pub async fn subscribe_handler(
  service: PushService,
  headers: axum::http::HeaderMap,
  body: String,
) -> axum::response::Response {
  if !authorized(&headers) {
    return json_err(axum::http::StatusCode::UNAUTHORIZED, "unauthorized");
  }
  match serde_json::from_str::<Subscription>(&body) {
    Ok(sub) if !sub.endpoint.is_empty() && valid_endpoint(&sub.endpoint) => {
      match service.subscribe(sub) {
        Ok(()) => json_ok("{\"ok\":true}".to_owned()),
        Err(e) => json_err(axum::http::StatusCode::TOO_MANY_REQUESTS, e),
      }
    }
    _ => json_err(axum::http::StatusCode::BAD_REQUEST, "invalid subscription"),
  }
}

/// `POST /api/push/unsubscribe`：删除订阅（body 为 `{"endpoint":"…"}`）。
pub async fn unsubscribe_handler(
  service: PushService,
  headers: axum::http::HeaderMap,
  body: String,
) -> axum::response::Response {
  if !authorized(&headers) {
    return json_err(axum::http::StatusCode::UNAUTHORIZED, "unauthorized");
  }
  #[derive(Deserialize)]
  struct Req {
    endpoint: String,
  }
  match serde_json::from_str::<Req>(&body) {
    Ok(req) if !req.endpoint.is_empty() => {
      service.unsubscribe(&req.endpoint);
      json_ok("{\"ok\":true}".to_owned())
    }
    _ => json_err(axum::http::StatusCode::BAD_REQUEST, "invalid endpoint"),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn vapid_jwt_roundtrips() {
    let v = VapidKeys::generate("mailto:test@example.com");
    let jwt = vapid_jwt(&v, "https://push.example.com").expect("jwt");
    let parts: Vec<&str> = jwt.split('.').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(
      unb64url(parts[0]).expect("header"),
      br#"{"typ":"JWT","alg":"ES256"}"#
    );
    // 私钥标量可往返：generate → private_b64 → from_private_b64 得到同一公钥。
    let restored =
      VapidKeys::from_private_b64(&v.private_b64(), "mailto:test@example.com").expect("restore");
    assert_eq!(restored.public_b64, v.public_b64);
  }

  #[test]
  fn store_roundtrips() {
    let data = StoreData {
      vapid_private_b64: "abc".to_owned(),
      subscriptions: HashMap::from([(
        "https://push.example.com/1".to_owned(),
        Subscription {
          endpoint: "https://push.example.com/1".to_owned(),
          keys: SubscriptionKeys {
            p256dh: "dh".to_owned(),
            auth: "auth".to_owned(),
          },
          reminder_utc_minutes: Some(720),
        },
      )]),
      last_reminded: HashMap::new(),
    };
    let json = serde_json::to_string(&data).expect("serialize");
    let back: StoreData = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.vapid_private_b64, "abc");
    assert_eq!(back.subscriptions.len(), 1);
  }

  #[test]
  fn origin_parsing() {
    assert_eq!(
      endpoint_origin("https://fcm.googleapis.com/fcm/send/abc").as_deref(),
      Some("https://fcm.googleapis.com")
    );
    assert!(endpoint_origin("bad").is_none());
  }

  #[test]
  fn encrypt_decrypt_shape() {
    // 用一组临时订阅密钥验证加密产出结构（不验证密文可解，仅校验布局与长度）。
    let secret = SecretKey::generate();
    let pub_b64 = b64url(&secret.public_key().to_sec1_bytes());
    let (body, encryption, crypto_key) = encrypt_payload(b"hello", &pub_b64).expect("encrypt");
    assert_eq!(body.len(), 16 + 4 + 1 + 16 + "hello".len()); // salt + rs + idlen + tag + msg
    assert!(encryption.starts_with("salt="));
    assert!(crypto_key.starts_with("dh="));
    // body 前 16 字节与 Encryption 头的 salt 一致
    assert_eq!(b64url(&body[..16]), encryption.trim_start_matches("salt="));
    assert_eq!(
      u32::from_be_bytes(body[16..20].try_into().expect("rs")),
      RECORD_SIZE
    );
    assert_eq!(body[20], 0);
  }
}
