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
  // 主机名大小写不敏感：白名单按小写书写，这里归一化后再比对。
  let host = host.to_ascii_lowercase();
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
///
/// `auth` 是订阅里的认证密钥（authentication secret），在 RFC 8291 中作为 HKDF-Extract
/// 的盐参与密钥派生；body 里那条 16 字节 `salt` 只是记录盐，二者不能混用。
fn encrypt_payload(
  plaintext: &[u8],
  client_p256dh_b64: &str,
  client_auth_b64: &str,
) -> anyhow::Result<(Vec<u8>, String, String)> {
  let client_pub =
    PublicKey::from_sec1_bytes(&unb64url(client_p256dh_b64)?).context("订阅 p256dh 无效")?;
  let client_pub_bytes = client_pub.to_sec1_bytes().to_vec();
  let auth_secret = unb64url(client_auth_b64).context("订阅 auth 无效")?;

  // 记录盐：随密文一起发给客户端，用于重建密钥（写入 body 头与 `Encryption` 头）。
  let salt: [u8; 16] = Generate::generate();

  let server_secret = EphemeralSecret::generate();
  let server_pub_bytes = server_secret.public_key().to_sec1_bytes().to_vec();
  let shared = server_secret.diffie_hellman(&client_pub);
  let shared_bytes = shared.raw_secret_bytes();

  // RFC 8291 §3.4 的两步派生，缺一不可：
  //   1. IKM = HKDF-Expand(HKDF-Extract(auth_secret, ecdh_secret), "WebPush: info\0"‖ua‖as, 32)
  //      —— 第一步的盐是订阅的 `auth`（认证密钥）；
  //   2. PRK   = HKDF-Extract(记录盐 `salt`, IKM)
  //      CEK   = HKDF-Expand(PRK, "Content-Encoding: aes128gcm\0", 16)
  //      NONCE = HKDF-Expand(PRK, "Content-Encoding: nonce\0", 12)
  // 第二步的盐才是随密文下发的记录盐 `salt`，与第一步的 `auth` 不可混用；直接把 IKM
  // 切成 CEK‖NONCE（跳过第二步）会被浏览器按 RFC 派生出不同的密钥而无法解密。
  let hkdf = Hkdf::<Sha256>::new(Some(auth_secret.as_slice()), shared_bytes);
  let mut info = Vec::with_capacity(14 + 65 + 65);
  info.extend_from_slice(b"WebPush: info\0");
  info.extend_from_slice(&client_pub_bytes);
  info.extend_from_slice(&server_pub_bytes);
  let mut ikm = [0u8; 32];
  hkdf
    .expand(&info, &mut ikm)
    .map_err(|_| anyhow::anyhow!("HKDF expand IKM 失败"))?;

  let hkdf = Hkdf::<Sha256>::new(Some(salt.as_slice()), &ikm);
  let mut cek = [0u8; 16];
  hkdf
    .expand(b"Content-Encoding: aes128gcm\0", &mut cek)
    .map_err(|_| anyhow::anyhow!("HKDF expand CEK 失败"))?;
  let mut nonce_bytes = [0u8; 12];
  hkdf
    .expand(b"Content-Encoding: nonce\0", &mut nonce_bytes)
    .map_err(|_| anyhow::anyhow!("HKDF expand nonce 失败"))?;

  let cipher = Aes128Gcm::new_from_slice(&cek).context("AES 密钥无效")?;
  let nonce = Nonce::<Aes128Gcm>::try_from(&nonce_bytes[..])
    .map_err(|_| anyhow::anyhow!("nonce 长度错误"))?;
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
  let (body_bytes, encryption, crypto_key) = encrypt_payload(
    payload.to_string().as_bytes(),
    &sub.keys.p256dh,
    &sub.keys.auth,
  )?;

  let res = crate::util::http_agent()
    .post(&sub.endpoint)
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
    // 与失效订阅的清理保持一致：订阅没了，去重记录也一并删除。
    state.last_reminded.remove(endpoint);
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

    let mut changed = false;
    for (endpoint, sub) in due {
      match send_push(&vapid, &sub, "该学习啦", "今天的备考任务待完成。") {
        Ok(true) => {
          let mut state = self.inner.lock().expect("push state poisoned");
          state.last_reminded.insert(endpoint, day);
          changed = true;
          tracing::info!("push reminder sent to {}", sub.endpoint);
        }
        Ok(false) => {
          let mut state = self.inner.lock().expect("push state poisoned");
          state.subscriptions.remove(&endpoint);
          // 订阅没了，它的去重记录也一并清掉，避免 `last_reminded` 无限增长。
          state.last_reminded.remove(&endpoint);
          changed = true;
          tracing::info!("removed expired subscription {}", sub.endpoint);
        }
        Err(e) => tracing::warn!("push reminder failed: {e:#}"),
      }
    }
    // 统一落盘一次：`send_push` 是网络操作，若在循环里逐个 `persist`，同一分钟到点的
    // 大量订阅会触发同等次数的全量文件写入。不落盘则重启后同一天会重复推送、且已清理
    // 的失效订阅会从磁盘复活。
    if changed {
      self.inner.lock().expect("push state poisoned").persist();
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

/// 订阅写入接口的鉴权结果。
enum Auth {
  /// 通过。
  Ok,
  /// 配置了令牌但不匹配。
  Denied,
}

/// 是否要求订阅接口携带令牌（即设置了 `PUSH_API_TOKEN`）。
pub(crate) fn token_required() -> bool {
  std::env::var("PUSH_API_TOKEN").is_ok()
}

/// 校验 `Authorization: Bearer <token>`。
///
/// **未配置 `PUSH_API_TOKEN` 时有意放行**（返回 [`Auth::Ok`]）。这不是疏忽：
/// 订阅者就是浏览器里的匿名访客，而前端是所有访客共享的静态 wasm，无从持有服务端
/// 环境变量里的令牌 —— 一旦要求鉴权，合法用户同样无法订阅，功能等于对所有人关闭。
/// 曾改为「无令牌即拒绝」，结果正是把推送做废，故回退为放行。
///
/// 匿名写入的实际防护来自四层：
/// 1. `/api/*` 每 IP 限流（默认 120 次/分钟）；
/// 2. `valid_endpoint` 只接受主流推送服务的 `https://` endpoint；
/// 3. `MAX_SUBSCRIPTIONS` 订阅总数上限；
/// 4. 不提供订阅列表读取接口，且覆盖 / 删除他人订阅需事先拿到对方那条不可枚举的
///    endpoint（FCM / APNs 的长随机 URL）。
///
/// 需要更严格管控时，请在反向代理层限制这两个路径；或自行构建前端、把令牌注入
/// `Authorization` 头后再设置 `PUSH_API_TOKEN`。
fn authorize(headers: &axum::http::HeaderMap) -> Auth {
  let Ok(token) = std::env::var("PUSH_API_TOKEN") else {
    return Auth::Ok;
  };
  let matched = headers
    .get(axum::http::header::AUTHORIZATION)
    .and_then(|v| v.to_str().ok())
    .and_then(|v| v.strip_prefix("Bearer "))
    .is_some_and(|t| t == token);
  if matched { Auth::Ok } else { Auth::Denied }
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
  if let Auth::Denied = authorize(&headers) {
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
  if let Auth::Denied = authorize(&headers) {
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

  /// 匿名订阅必须保持放行：前端是共享的静态资源，拿不到服务端令牌。
  ///
  /// 曾改为「无令牌即拒绝」，结果官方前端的订阅请求全部失败，功能对所有人关闭。
  /// 这个测试用于防止再次改回去。
  #[test]
  fn anonymous_subscribe_is_allowed_without_token() {
    let headers = axum::http::HeaderMap::new();
    assert!(matches!(authorize(&headers), Auth::Ok));
  }

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
    let auth_b64 = b64url(&[0u8; 16]);
    let (body, encryption, crypto_key) =
      encrypt_payload(b"hello", &pub_b64, &auth_b64).expect("encrypt");
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

  /// 端到端往返：`encrypt_payload` 产出的密文，客户端按同一套 RFC 8291 步骤派生密钥后
  /// 必须能解开。长度/布局校验发现不了「少做一步 HKDF」这类问题，只有真解密才能。
  #[test]
  fn encrypt_payload_roundtrips_per_rfc8291() {
    let client_secret = EphemeralSecret::generate();
    let client_pub_bytes = client_secret.public_key().to_sec1_bytes().to_vec();
    let auth_secret = [7u8; 16];

    let (body, encryption, crypto_key) = encrypt_payload(
      b"hello web push",
      &b64url(&client_pub_bytes),
      &b64url(&auth_secret),
    )
    .expect("encrypt");

    // 从密文头与 `Crypto-Key` 头还原记录盐与服务端临时公钥。
    let salt: [u8; 16] = body[..16].try_into().expect("salt");
    assert_eq!(b64url(&salt), encryption.trim_start_matches("salt="));
    let server_pub_bytes = unb64url(crypto_key.trim_start_matches("dh=")).expect("dh");
    let server_pub = PublicKey::from_sec1_bytes(&server_pub_bytes).expect("server pub");

    // 客户端侧：ECDH → 两步 HKDF → AES-GCM 解密。
    let shared = client_secret.diffie_hellman(&server_pub);
    let shared_bytes = shared.raw_secret_bytes();
    let mut info = Vec::new();
    info.extend_from_slice(b"WebPush: info\0");
    info.extend_from_slice(&client_pub_bytes);
    info.extend_from_slice(&server_pub_bytes);
    let mut ikm = [0u8; 32];
    Hkdf::<Sha256>::new(Some(auth_secret.as_slice()), shared_bytes)
      .expand(&info, &mut ikm)
      .expect("ikm");
    let hkdf = Hkdf::<Sha256>::new(Some(salt.as_slice()), &ikm);
    let mut cek = [0u8; 16];
    hkdf
      .expand(b"Content-Encoding: aes128gcm\0", &mut cek)
      .expect("cek");
    let mut nonce = [0u8; 12];
    hkdf
      .expand(b"Content-Encoding: nonce\0", &mut nonce)
      .expect("nonce");

    let cipher = Aes128Gcm::new_from_slice(&cek).expect("cipher");
    let plaintext = cipher
      .decrypt(
        &Nonce::<Aes128Gcm>::try_from(&nonce[..]).expect("nonce"),
        &body[21..],
      )
      .expect("decrypt");
    assert_eq!(plaintext, b"hello web push");
  }
}
