//! 自定义远端同步：用户填一个「获取」接口和一个「写入」接口，自己管理那份备份 JSON。
//!
//! 这里只放不依赖浏览器的部分（配置结构、地址 / 请求头校验、哪些 key 不进备份），
//! 真正的 `fetch` 在 `crates/app/src/remote_sync.rs`。
//!
//! # 接口约定
//!
//! - **获取**：`GET <get_url>`，响应体是与「导出备份」同格式的 JSON 文本（`{ key: value }`），
//!   原样返回——不经 JSON 包装、也不经 base64。`404` / `204` / 空响应体都视为
//!   「远端还没有数据」，不算错误。
//! - **写入**：`<method> <put_url>`（`PUT` 或 `POST`），请求体是 `multipart/form-data`，
//!   字段名固定为 `file`、文件名 `backup.json`、文件类型 `application/json`，内容是那份 JSON。
//!   任何 2xx 都算成功。`Content-Type` 由浏览器生成（要带 boundary），用户在写入请求头里填的
//!   `Content-Type` 会被忽略，见 [`is_content_type_header`]。
//! - 鉴权靠用户自填的请求头（如 `Authorization: Bearer …`）；获取与写入**各有一份**请求头，
//!   所以读、写可以用不同的令牌。
//!
//! # 为什么配置不进备份
//!
//! 配置里有鉴权头，写进备份文件等于把令牌推到用户自己的远端、再扩散到每台恢复过的设备；
//! 反过来，导入一份别人给的备份也不应该能改写「数据往哪推」。所以 [`STORAGE_KEY`] 在导出和
//! 导入两侧都被排除（见 [`is_excluded_from_backup`]）。

use std::net::Ipv4Addr;

use serde::{Deserialize, Serialize};

/// 配置在 `localStorage` 里的 key。
pub const STORAGE_KEY: &str = "remote-sync";

/// 写入接口使用的 HTTP 方法。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WriteMethod {
  /// 整体替换（多数对象存储 / 网盘 WebDAV 的写法）。
  #[default]
  Put,
  /// 提交（多数自建后端的写法）。
  Post,
}

impl WriteMethod {
  /// 全部可选项，顺序即界面上的顺序。
  pub const ALL: [Self; 2] = [Self::Put, Self::Post];

  /// 请求行里的方法名。
  #[must_use]
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Put => "PUT",
      Self::Post => "POST",
    }
  }

  /// 从方法名解析（大小写不敏感）；不认识的按默认 `PUT`。
  #[must_use]
  pub fn parse(s: &str) -> Self {
    if s.trim().eq_ignore_ascii_case("POST") {
      Self::Post
    } else {
      Self::Put
    }
  }
}

/// 用户配置的远端接口。
///
/// 字段都是用户输入的原样文本：校验在动作发生时做（[`check_url`] / [`parse_headers`]），
/// 这样半截输入也能存下来，不会因为「现在还不合法」而丢掉用户正在敲的内容。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RemoteSyncConfig {
  /// 获取接口地址。
  pub get_url: String,
  /// 写入接口地址。
  pub put_url: String,
  /// 写入用的 HTTP 方法。
  pub method: WriteMethod,
  /// 获取接口的自定义请求头，每行一个 `Name: Value`。
  pub get_headers: String,
  /// 写入接口的自定义请求头，每行一个 `Name: Value`。
  ///
  /// 与 [`Self::get_headers`] 各存各的：读写常常是两把权限不同的令牌（只读 / 可写），
  /// 共用一份就只能给读取也配上写权限。
  pub put_headers: String,
  /// 同步内容里不带 QSL 影像（卡片扫描件）。
  ///
  /// 自动同步写出的是整份快照，影像是其中体积最大的一块：每次改动都重传一遍、
  /// 每次判断「有没有变」还要把所有影像从 IndexedDB 读出来再算指纹。
  /// 扫描件很多、流量或内存吃紧的设备可以打开它，只同步日志和其他数据。
  pub exclude_qsl_images: bool,
}

impl RemoteSyncConfig {
  /// 是否配了获取接口。
  #[must_use]
  pub fn can_pull(&self) -> bool {
    !self.get_url.trim().is_empty()
  }

  /// 是否配了写入接口。
  #[must_use]
  pub fn can_push(&self) -> bool {
    !self.put_url.trim().is_empty()
  }
}

/// 该 key 是否只属于这台设备、不进备份文件。
#[must_use]
pub fn is_excluded_from_backup(key: &str) -> bool {
  key == STORAGE_KEY || key == STATE_KEY
}

/// 自动同步的状态在 `localStorage` 里的 key（见 [`SyncState`]）。
pub const STATE_KEY: &str = "remote-sync-state";

// ───────────────────────── 自动同步 ─────────────────────────
//
// # 为什么自动获取是「覆盖」而不是「合并」
//
// 合并导入对答题统计是**累加**的（`backup_merge::merge_study_stats`），不幂等：同一份远端
// 数据合并两次，计数就翻倍。自动同步每次打开页面都会发生，不能用它。覆盖是幂等的，
// 所以自动获取只在「覆盖不会丢本机数据」时才做，其余情况交还给用户（见 [`plan_startup`]）。
//
// 判断依据是一条**基线**：上次成功同步时远端内容的指纹（[`SyncState::base`]），加上一个
// 「本机自那以后有没有改动」的标记（[`SyncState::dirty`]）。

/// 这台设备专属的 key：**不同步，也不算「用户数据」**。
///
/// - 主题 / 语言 / 界面提示 / 悬浮按钮位置：各设备各有偏好（手机深色、桌面浅色）；
/// - 电台 CAT 设置（协议、波特率、CI-V 地址）：跟着这台设备实际连的那台电台走，
///   同步到另一台设备只会把它连不上的串口参数盖上去；
/// - 接口缓存 / 版本号 / 各类「已提醒」记录 / 推送订阅：设备相关或可再生，同步过去没有意义，
///   还会让每次取数都被当成一次「数据变化」；
/// - 同步配置与同步状态本身。
///
/// 同一份名单有三个用处：不触发自动写入、不进同步内容、不计入「本机有没有数据」。
///
/// 新增本地存储 key 时必须在 `docs/storage.md` 登记，并在本模块的 `every_documented_key_is_classified`
/// 测试里归类（同步 / 设备专属）——漏归类会让测试变红，而不是悄悄变成「每次写入都触发一次同步」。
#[must_use]
pub fn is_device_local_key(key: &str) -> bool {
  is_excluded_from_backup(key)
    // 与 `crate::kv` 的版本号后缀一致（app 侧常量 `kv::VERSION_SUFFIX`）。
    || key.ends_with(":kv-version")
    || key.starts_with("api-cache:")
    || key.starts_with("ui:")
    || key.starts_with("bank-digest:")
    || matches!(
      key,
      "theme"
        | "locale"
        | "fab-position"
        | "cat-settings"
        | "push-subscription"
        | "app:changelog-seen"
        | "sat-watch-notified"
        | "sat-apt-notified"
        | "nec.pending-template"
    )
}

/// 本机有没有值得同步的数据（有任何一个非设备专属的 key 即算有）。
///
/// 用来区分「全新浏览器」和「已经用了一阵的浏览器」：前者可以放心拿远端覆盖，后者不行。
#[must_use]
pub fn has_user_data<'a>(keys: impl IntoIterator<Item = &'a str>) -> bool {
  keys.into_iter().any(|k| !is_device_local_key(k))
}

/// 一份备份内容的指纹（FNV-1a 64 位），与键值的出现顺序无关。
///
/// 单独成类型而不是裸 `u64`：基线、远端指纹、本机指纹是同一种东西，混进别的整数
/// （超时毫秒、计数）时编译器要能拦下来。序列化时与裸数字同形，已存下来的状态仍然读得出。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Fingerprint(u64);

/// 一份备份内容的指纹。
///
/// 值本身是不透明字符串，所以服务端重新排版外层 JSON（缩进、键序）不会改变指纹 ——
/// 否则这类服务会被误判成「远端被别人改过」。每段前面加长度，避免 `("a","bc")` 与
/// `("ab","c")` 撞成同一个指纹。
#[must_use]
pub fn fingerprint(map: &std::collections::BTreeMap<String, String>) -> Fingerprint {
  const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
  const PRIME: u64 = 0x0000_0100_0000_01b3;
  let mut h = OFFSET;
  let mut feed = |bytes: &[u8]| {
    for b in (bytes.len() as u64).to_le_bytes().iter().chain(bytes) {
      h ^= u64::from(*b);
      h = h.wrapping_mul(PRIME);
    }
  };
  for (k, v) in map {
    feed(k.as_bytes());
    feed(v.as_bytes());
  }
  Fingerprint(h)
}

/// 解析远端备份 JSON 并求指纹；不是 `{ key: value }` 形态时返回 `None`。
///
/// 求指纹前先去掉设备专属的 key（[`is_device_local_key`]），与本机侧生成同步内容时的口径一致：
/// 远端若是一份手工导出的完整备份（带主题、接口缓存…），不去掉的话它的指纹永远对不上本机，
/// 每次都会被判成「内容不同」而多传一次。
#[must_use]
pub fn fingerprint_json(json: &str) -> Option<Fingerprint> {
  let mut map: std::collections::BTreeMap<String, String> = serde_json::from_str(json).ok()?;
  map.retain(|key, _| !is_device_local_key(key));
  Some(fingerprint(&map))
}

/// 自动同步的持久化状态。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncState {
  /// 上次成功同步时远端内容的指纹；`None` 表示这组接口还没同步成功过。
  pub base: Option<Fingerprint>,
  /// 本机自上次同步以来有改动、且还没成功写出去。
  ///
  /// 持久化的原因：改动后关掉标签页、写入失败或离线，下次打开时它才是「两边是否分叉」的依据。
  /// 它表示「可能有改动」而不是「一定有」：写出前会再比一次指纹，内容一样就不发请求。
  pub dirty: bool,
}

/// 启动时看到的远端。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Remote {
  /// 远端还没有数据（`404` / `204` / 空内容）。
  Empty,
  /// 远端有数据，附内容指纹。
  Data(Fingerprint),
}

/// 启动时该怎么办。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupPlan {
  /// 远端没变：不用拉取。本机若有未写出的改动由调用方继续写。
  InSync,
  /// 远端更新了而本机没有未同步的改动：直接用远端覆盖本机。
  FastForward,
  /// 这组接口以前同步成功过，现在远端空了而本机有数据：把本机传上去。
  Upload,
  /// 这组接口从没同步成功过，远端却是空的而本机有数据：**不自动上传**，等用户手动写入一次。
  ///
  /// 「404 / 空内容」不一定是真的没数据：获取地址写错、鉴权失败的服务伪装成 404、
  /// 读写地址不是同一份……此时自动上传会把本机的数据推到一个没人确认过的地方，还可能盖掉别处的内容。
  /// 手动「写入远端」成功才说明这组地址是对的（之后自动同步才上岗）。
  NeedsFirstPush,
  /// 远端变了，本机也有未同步的改动（或本机已有数据却从没同步过）：自动选哪边都可能丢数据，
  /// 暂停并让用户选。
  Conflict,
}

/// 启动时的决策。
///
/// `stale_dirty` 是**本机有没有未同步的改动**，由调用方合成两部分：
/// - 上次会话留下的 [`SyncState::dirty`]；
/// - 本次启动后**用户**又改了数据（取回远端要等网络，这段时间用户已经在答题 / 记通联了）。
///
/// 应用自己在启动时也会写（每日打卡、迁移标记），这部分**不能**算进去，否则正常的跨设备使用
/// 会天天报冲突。`local_has_data` 见 [`has_user_data`]。
#[must_use]
pub fn plan_startup(
  state: &SyncState,
  stale_dirty: bool,
  remote: Remote,
  local_has_data: bool,
) -> StartupPlan {
  match remote {
    Remote::Empty => {
      if !(stale_dirty || local_has_data) {
        StartupPlan::InSync
      } else if state.base.is_some() {
        StartupPlan::Upload
      } else {
        StartupPlan::NeedsFirstPush
      }
    }
    Remote::Data(fp) if state.base == Some(fp) => StartupPlan::InSync,
    Remote::Data(_) => {
      // 本机「没动过」：有基线就看有没有未同步改动；没有基线（从没同步过）就看本机是否真是空的。
      let untouched = if state.base.is_some() {
        !stale_dirty
      } else {
        !stale_dirty && !local_has_data
      };
      if untouched {
        StartupPlan::FastForward
      } else {
        StartupPlan::Conflict
      }
    }
  }
}

/// 写入失败后重试的基础间隔（毫秒）。
pub const RETRY_BASE_MS: u64 = 60_000;
/// 写入失败后重试间隔的上限（毫秒）：持续失败时退避到这个值为止。
pub const RETRY_MAX_MS: u64 = 30 * 60_000;

/// 连续失败 `failures` 次（从 1 起）之后，下一次重试前要等多久（毫秒）：指数退避，封顶 [`RETRY_MAX_MS`]。
///
/// 每次重试都要把整个本地库读一遍再上传，服务出了问题时按固定间隔重试等于一直在给它添乱。
#[must_use]
pub fn retry_delay_ms(failures: u32) -> u64 {
  let doublings = failures.saturating_sub(1).min(16);
  RETRY_BASE_MS
    .saturating_mul(1 << doublings)
    .min(RETRY_MAX_MS)
}

/// 这个 HTTP 状态码是不是「过一会儿再试可能就好了」。
///
/// 5xx 是服务端的问题；`408` / `425` / `429` 明说了让客户端稍后重试。其余非 2xx
/// （`401` / `403` 令牌不对、`404` / `405` 地址或方法不对、`413` 超限……）自己重试多少次都一样，
/// 要用户改了配置才行：这类错误应当暂停自动写入并说明原因，而不是每分钟上传一遍整份数据。
#[must_use]
pub const fn is_retryable_status(status: u16) -> bool {
  matches!(status, 408 | 425 | 429 | 500..=599)
}

/// 地址不合格的原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrlError {
  /// 没填。
  Empty,
  /// 不是 `http://` / `https://`，或缺主机名、含空白。
  Invalid,
  /// 地址里带了 `用户名:密码@`：`fetch` 会直接拒绝这样的 URL，鉴权应改用请求头。
  Credentials,
  /// 本页是 HTTPS 而地址是明文 HTTP：浏览器按混合内容拦截（本机回环地址除外）。
  MixedContent,
}

/// 校验一个接口地址。
///
/// `page_is_https` 是当前页面是否 HTTPS：明文 `http://` 在 HTTPS 页面里会被浏览器拦下，
/// 提前说清原因比让用户对着一句「Failed to fetch」猜要好。
///
/// # Errors
///
/// 见 [`UrlError`]。
pub fn check_url(url: &str, page_is_https: bool) -> Result<(), UrlError> {
  let url = url.trim();
  if url.is_empty() {
    return Err(UrlError::Empty);
  }
  if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
    return Err(UrlError::Invalid);
  }
  let lower = url.to_ascii_lowercase();
  let (is_https, rest) = if let Some(rest) = lower.strip_prefix("https://") {
    (true, rest)
  } else if let Some(rest) = lower.strip_prefix("http://") {
    (false, rest)
  } else {
    return Err(UrlError::Invalid);
  };
  let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
  if authority.contains('@') {
    return Err(UrlError::Credentials);
  }
  let host = host_of(authority);
  if host.is_empty() {
    return Err(UrlError::Invalid);
  }
  if !is_https && page_is_https && !is_loopback_host(host) {
    return Err(UrlError::MixedContent);
  }
  Ok(())
}

/// 从 `host[:port]` / `[v6][:port]` 里取出主机部分。
fn host_of(authority: &str) -> &str {
  if let Some(rest) = authority.strip_prefix('[') {
    // IPv6 字面量：保留方括号，便于和 `[::1]` 比较。
    return rest.find(']').map_or("", |end| &authority[..end + 2]);
  }
  authority.split(':').next().unwrap_or_default()
}

/// 浏览器视为「可信来源」的回环主机：混合内容规则对它们放行。
fn is_loopback_host(host: &str) -> bool {
  host == "localhost"
    || host.ends_with(".localhost")
    || host == "[::1]"
    || host.parse::<Ipv4Addr>().is_ok_and(|ip| ip.is_loopback())
}

/// 请求头解析失败：`line` 是出错的行号（从 1 起）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeaderError {
  /// 出错的行号（从 1 起，空行也计数，和用户在文本框里看到的一致）。
  pub line: usize,
}

/// 把多行 `Name: Value` 文本解析成请求头列表。
///
/// 空行忽略；名字必须是合法的 HTTP token，值不能含控制字符，也不能含超出 Latin-1 的字符
/// （`Headers.set` 对这类值会直接抛 `TypeError`，提前拦下才能报出是第几行）。
///
/// # Errors
///
/// 第一行不合格的行号。
pub fn parse_headers(text: &str) -> Result<Vec<(String, String)>, HeaderError> {
  let mut out = Vec::new();
  for (i, raw) in text.lines().enumerate() {
    let line = raw.trim();
    if line.is_empty() {
      continue;
    }
    let bad = HeaderError { line: i + 1 };
    let (name, value) = line.split_once(':').ok_or(bad)?;
    let (name, value) = (name.trim(), value.trim());
    if !is_header_name(name) || !is_header_value(value) {
      return Err(bad);
    }
    out.push((name.to_owned(), value.to_owned()));
  }
  Ok(out)
}

/// RFC 9110 的 `token`：非空，仅由字母数字与 ``!#$%&'*+-.^_`|~`` 组成。
fn is_header_name(name: &str) -> bool {
  !name.is_empty()
    && name
      .chars()
      .all(|c| c.is_ascii_alphanumeric() || "!#$%&'*+-.^_`|~".contains(c))
}

/// 请求头名是不是 `Content-Type`（名字不分大小写）。
///
/// 带请求体的写入是 `multipart/form-data`，值里必须有浏览器生成的 `boundary`；用户填的
/// `Content-Type` 会把它盖掉，服务端就读不到 `file` 字段。旧版本的写入是裸 JSON，那时允许用这一行
/// 改类型（如某些网盘要 `text/plain`），升级后这类配置还留在本机——所以**发请求时忽略它**，
/// 而不是在校验里报错：报「第 N 行格式不对」既不准，又会让已经配好的自动写入无声地停摆。
#[must_use]
pub fn is_content_type_header(name: &str) -> bool {
  name.eq_ignore_ascii_case("content-type")
}

fn is_header_value(value: &str) -> bool {
  value
    .chars()
    .all(|c| c == '\t' || (!c.is_control() && u32::from(c) <= 0xFF))
}

/// 获取接口的响应怎么处理。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullOutcome {
  /// 拿到了内容，交给导入。
  Data,
  /// 远端还没有数据（`404` / `204` / 空响应体）。
  Empty,
  /// 其余状态码：失败，附状态码。
  Failed(u16),
}

/// 按状态码与响应体判定获取接口的结果。
///
/// 把「空」单独拎出来是因为第一次使用时写入接口还从没写过，此时 `GET` 返回 `404` 是常态，
/// 不该吓用户一句「拉取失败」。
#[must_use]
pub fn classify_pull(status: u16, body: &str) -> PullOutcome {
  match status {
    404 | 204 => PullOutcome::Empty,
    200..=299 if body.trim().is_empty() => PullOutcome::Empty,
    200..=299 => PullOutcome::Data,
    other => PullOutcome::Failed(other),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn config_round_trips_and_tolerates_missing_fields() {
    let cfg = RemoteSyncConfig {
      get_url: "https://example.com/data.json".into(),
      put_url: "https://example.com/data".into(),
      method: WriteMethod::Post,
      get_headers: "Authorization: Bearer read-token".into(),
      put_headers: "Authorization: Bearer write-token".into(),
      exclude_qsl_images: true,
    };
    let json = serde_json::to_string(&cfg).unwrap();
    assert_eq!(
      serde_json::from_str::<RemoteSyncConfig>(&json).unwrap(),
      cfg
    );
    // 读写令牌互不串：各自原样保存。
    let back: RemoteSyncConfig = serde_json::from_str(&json).unwrap();
    assert_ne!(back.get_headers, back.put_headers);
    // 以后加字段 / 旧版本只存了一部分：缺的按默认补，不能让整份配置读不出来。
    let partial: RemoteSyncConfig = serde_json::from_str(r#"{"get_url":"https://a.b/c"}"#).unwrap();
    assert_eq!(partial.get_url, "https://a.b/c");
    assert_eq!(partial.method, WriteMethod::Put);
    assert!(!partial.can_push());
    assert!(partial.can_pull());
    // 新加的开关缺省关闭：升级前存下来的配置行为不变（同步内容仍含 QSL 影像）。
    assert!(!partial.exclude_qsl_images);
  }

  #[test]
  fn write_method_parses_case_insensitively_and_defaults_to_put() {
    assert_eq!(WriteMethod::parse("post"), WriteMethod::Post);
    assert_eq!(WriteMethod::parse(" POST "), WriteMethod::Post);
    assert_eq!(WriteMethod::parse("PUT"), WriteMethod::Put);
    assert_eq!(WriteMethod::parse("PATCH"), WriteMethod::Put);
    for m in WriteMethod::ALL {
      assert_eq!(WriteMethod::parse(m.as_str()), m);
    }
  }

  #[test]
  fn only_the_sync_config_is_kept_out_of_backups() {
    assert!(is_excluded_from_backup("remote-sync"));
    assert!(!is_excluded_from_backup("logbook"));
    assert!(!is_excluded_from_backup("remote-sync-other"));
  }

  #[test]
  fn accepts_ordinary_http_and_https_urls() {
    for url in [
      "https://example.com/backup.json",
      "https://example.com:8443/a/b?token=1#x",
      "HTTPS://Example.com/",
      "  https://example.com  ",
      "http://192.168.1.2:8080/data",
    ] {
      assert_eq!(check_url(url, false), Ok(()), "{url}");
    }
  }

  #[test]
  fn rejects_malformed_urls() {
    assert_eq!(check_url("", false), Err(UrlError::Empty));
    assert_eq!(check_url("   ", false), Err(UrlError::Empty));
    for url in [
      "example.com/data",
      "ftp://example.com/data",
      "javascript:alert(1)",
      "https://",
      "https:///path",
      "https://:8080/x",
      "https://example .com",
      "https://example.com/\nx",
    ] {
      assert_eq!(check_url(url, false), Err(UrlError::Invalid), "{url:?}");
    }
  }

  #[test]
  fn credentials_in_the_url_are_reported_separately() {
    assert_eq!(
      check_url("https://user:pass@example.com/x", false),
      Err(UrlError::Credentials)
    );
    // `@` 在路径 / 查询里不是凭据。
    assert_eq!(check_url("https://example.com/@me?a=b@c", false), Ok(()));
  }

  #[test]
  fn plain_http_is_blocked_only_from_https_pages_and_not_for_loopback() {
    assert_eq!(
      check_url("http://example.com/x", true),
      Err(UrlError::MixedContent)
    );
    assert_eq!(check_url("http://example.com/x", false), Ok(()));
    for url in [
      "http://localhost:3000/x",
      "http://app.localhost/x",
      "http://127.0.0.1:8080/x",
      "http://127.1.2.3/x",
      "http://[::1]:8080/x",
    ] {
      assert_eq!(check_url(url, true), Ok(()), "{url}");
    }
    // 看起来像回环、其实不是：不能被放行。
    for url in ["http://localhost.evil.com/x", "http://127.0.0.1.evil.com/x"] {
      assert_eq!(check_url(url, true), Err(UrlError::MixedContent), "{url}");
    }
    // HTTPS 页面访问 HTTPS 接口永远没问题。
    assert_eq!(check_url("https://example.com/x", true), Ok(()));
  }

  #[test]
  fn parses_header_lines() {
    let got = parse_headers("Authorization: Bearer abc\n\n  X-Api-Key :  k:v  \r\n").unwrap();
    assert_eq!(
      got,
      vec![
        ("Authorization".to_owned(), "Bearer abc".to_owned()),
        // 只在第一个冒号处切：值里可以有冒号。
        ("X-Api-Key".to_owned(), "k:v".to_owned()),
      ]
    );
    assert_eq!(parse_headers("").unwrap(), vec![]);
    assert_eq!(parse_headers("\n  \n").unwrap(), vec![]);
    // 空值合法（`X-Empty:`）。
    assert_eq!(
      parse_headers("X-Empty:").unwrap(),
      vec![("X-Empty".to_owned(), String::new())]
    );
  }

  #[test]
  fn header_errors_report_the_line_the_user_sees() {
    // 空行也计数，行号才和文本框里数出来的一致。
    assert_eq!(
      parse_headers("A: 1\n\nnot a header"),
      Err(HeaderError { line: 3 })
    );
    assert_eq!(parse_headers(": no-name"), Err(HeaderError { line: 1 }));
    assert_eq!(parse_headers("Bad Name: 1"), Err(HeaderError { line: 1 }));
    assert_eq!(
      parse_headers("A: 1\nB: 日本語"),
      Err(HeaderError { line: 2 })
    );
    assert_eq!(parse_headers("A: x\u{7}y"), Err(HeaderError { line: 1 }));
  }

  #[test]
  fn content_type_is_recognised_regardless_of_case() {
    for name in [
      "Content-Type",
      "content-type",
      "CONTENT-TYPE",
      "cOnTeNt-TyPe",
    ] {
      assert!(is_content_type_header(name), "{name}");
    }
    // 只认整个名字：相近的头不能被连带忽略。
    for name in [
      "Content-Length",
      "Content-Type-Options",
      "X-Content-Type",
      "Accept",
      "",
    ] {
      assert!(!is_content_type_header(name), "{name:?}");
    }
  }

  #[test]
  fn pull_treats_missing_or_empty_remote_data_as_empty_not_failure() {
    assert_eq!(classify_pull(200, "{\"a\":\"b\"}"), PullOutcome::Data);
    assert_eq!(classify_pull(200, ""), PullOutcome::Empty);
    assert_eq!(classify_pull(200, " \n"), PullOutcome::Empty);
    assert_eq!(classify_pull(204, ""), PullOutcome::Empty);
    assert_eq!(classify_pull(404, "Not Found"), PullOutcome::Empty);
    assert_eq!(classify_pull(401, ""), PullOutcome::Failed(401));
    assert_eq!(classify_pull(500, "{}"), PullOutcome::Failed(500));
  }

  fn map(pairs: &[(&str, &str)]) -> std::collections::BTreeMap<String, String> {
    pairs
      .iter()
      .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
      .collect()
  }

  #[test]
  fn fingerprint_ignores_json_layout_but_sees_every_change() {
    let a = map(&[("bookmarks", "[\"x\"]"), ("logbook", "{}")]);
    // 键序 / 缩进是服务端可能改写的部分，不能影响指纹。
    let reordered = r#"{ "logbook": "{}",
      "bookmarks": "[\"x\"]" }"#;
    assert_eq!(fingerprint_json(reordered), Some(fingerprint(&a)));

    let changed = map(&[("bookmarks", "[\"y\"]"), ("logbook", "{}")]);
    assert_ne!(fingerprint(&a), fingerprint(&changed));
    let extra = map(&[("bookmarks", "[\"x\"]"), ("logbook", "{}"), ("z", "")]);
    assert_ne!(fingerprint(&a), fingerprint(&extra));
    assert_ne!(fingerprint(&a), fingerprint(&map(&[])));
  }

  #[test]
  fn fingerprint_does_not_confuse_where_a_key_ends() {
    assert_ne!(
      fingerprint(&map(&[("a", "bc")])),
      fingerprint(&map(&[("ab", "c")]))
    );
    assert_ne!(
      fingerprint(&map(&[("a", ""), ("b", "")])),
      fingerprint(&map(&[("a", "b")]))
    );
  }

  #[test]
  fn fingerprint_json_ignores_device_local_keys_like_the_local_side_does() {
    let synced = map(&[("bookmarks", "[]"), ("logbook", "{}")]);
    // 远端是手工导出的完整备份：多出来的设备专属 key 不能让指纹对不上本机生成的同步内容。
    let full_export = r#"{"bookmarks":"[]","logbook":"{}","theme":"dark","api-cache:/api/solar":"x","remote-sync":"{}"}"#;
    assert_eq!(fingerprint_json(full_export), Some(fingerprint(&synced)));
    // 真正的用户数据变了，指纹仍然要变。
    let changed = r#"{"bookmarks":"[1]","logbook":"{}","theme":"dark"}"#;
    assert_ne!(fingerprint_json(changed), Some(fingerprint(&synced)));
  }

  #[test]
  fn fingerprint_json_rejects_what_is_not_a_backup() {
    assert_eq!(fingerprint_json("not json"), None);
    assert_eq!(fingerprint_json("[1,2]"), None);
    assert_eq!(fingerprint_json(r#"{"a":1}"#), None);
    assert_eq!(fingerprint_json("{}"), Some(fingerprint(&map(&[]))));
  }

  #[test]
  fn device_local_keys_are_kept_out_of_sync() {
    for key in [
      "remote-sync",
      "remote-sync-state",
      "logbook:kv-version",
      "api-cache:/api/solar",
      "ui:shortcutsHelpSeen:practice",
      "bank-digest:A",
      "theme",
      "locale",
      "fab-position",
      "cat-settings",
      "push-subscription",
      "app:changelog-seen",
      "sat-watch-notified",
      "sat-apt-notified",
      "nec.pending-template",
    ] {
      assert!(is_device_local_key(key), "{key} 应是设备专属");
    }
    // 真正的用户数据必须同步（含名字相近的）。
    for key in [
      "logbook",
      "bookmarks",
      "mistake-book",
      "study-stats",
      "daily-checkin",
      "practice:2024:A",
      "exam:savedState:2024:A",
      "qsl-image-index",
      "remote-sync-other",
      "themes",
    ] {
      assert!(!is_device_local_key(key), "{key} 不应是设备专属");
    }
  }

  #[test]
  fn a_browser_with_only_preferences_has_no_user_data() {
    assert!(!has_user_data([]));
    assert!(!has_user_data([
      "theme",
      "locale",
      "ui:shortcutsHelpSeen:exam"
    ]));
    assert!(!has_user_data(["api-cache:/api/spots", "remote-sync"]));
    assert!(has_user_data(["theme", "bookmarks"]));
  }

  const fn fp(n: u64) -> Fingerprint {
    Fingerprint(n)
  }

  const CLEAN: SyncState = SyncState {
    base: Some(fp(7)),
    dirty: false,
  };

  #[test]
  fn startup_does_nothing_when_the_remote_is_unchanged() {
    // 不论本机是否有未写出的改动，远端没变就不拉取。
    for stale_dirty in [false, true] {
      assert_eq!(
        plan_startup(&CLEAN, stale_dirty, Remote::Data(fp(7)), true),
        StartupPlan::InSync
      );
    }
  }

  #[test]
  fn startup_fast_forwards_only_when_local_was_untouched() {
    // 远端被别处更新、本机自上次同步后没动过 → 可以放心覆盖。
    assert_eq!(
      plan_startup(&CLEAN, false, Remote::Data(fp(8)), true),
      StartupPlan::FastForward
    );
    // 本机有没写出去的改动（含启动后用户又改的）→ 覆盖会丢数据，合并会累加统计：交给用户。
    assert_eq!(
      plan_startup(&CLEAN, true, Remote::Data(fp(8)), true),
      StartupPlan::Conflict
    );
  }

  #[test]
  fn startup_without_a_baseline_only_overwrites_an_empty_browser() {
    let never = SyncState::default();
    // 全新浏览器：本机没有任何用户数据，拿远端覆盖不丢东西。
    assert_eq!(
      plan_startup(&never, false, Remote::Data(fp(8)), false),
      StartupPlan::FastForward
    );
    // 已经用过的浏览器第一次连上有数据的远端：不知道谁新，不替用户选。
    assert_eq!(
      plan_startup(&never, false, Remote::Data(fp(8)), true),
      StartupPlan::Conflict
    );
    assert_eq!(
      plan_startup(&never, true, Remote::Data(fp(8)), false),
      StartupPlan::Conflict
    );
  }

  #[test]
  fn startup_uploads_an_emptied_remote_only_for_endpoints_that_synced_before() {
    // 同步成功过的接口，远端空了（被清掉了）而本机有数据：自动补传。
    assert_eq!(
      plan_startup(&CLEAN, false, Remote::Empty, true),
      StartupPlan::Upload
    );
    assert_eq!(
      plan_startup(&CLEAN, true, Remote::Empty, false),
      StartupPlan::Upload
    );
    // 两边都是空的：没有可传的。
    for state in [SyncState::default(), CLEAN] {
      assert_eq!(
        plan_startup(&state, false, Remote::Empty, false),
        StartupPlan::InSync
      );
    }
  }

  #[test]
  fn startup_never_uploads_to_an_endpoint_nobody_has_confirmed() {
    // 从没同步成功过：「404 / 空」可能是地址写错或服务伪装的，不能自动把本机推上去。
    let never = SyncState::default();
    assert_eq!(
      plan_startup(&never, false, Remote::Empty, true),
      StartupPlan::NeedsFirstPush
    );
    assert_eq!(
      plan_startup(&never, true, Remote::Empty, false),
      StartupPlan::NeedsFirstPush
    );
  }

  #[test]
  fn retry_delay_backs_off_exponentially_and_is_capped() {
    assert_eq!(retry_delay_ms(0), RETRY_BASE_MS);
    assert_eq!(retry_delay_ms(1), 60_000);
    assert_eq!(retry_delay_ms(2), 120_000);
    assert_eq!(retry_delay_ms(3), 240_000);
    assert_eq!(retry_delay_ms(6), 1_800_000);
    for failures in [7, 20, u32::MAX] {
      assert_eq!(retry_delay_ms(failures), RETRY_MAX_MS, "{failures}");
    }
    let mut last = 0;
    for failures in 1..=30 {
      let d = retry_delay_ms(failures);
      assert!(d >= last, "退避不能回头：第 {failures} 次");
      last = d;
    }
  }

  #[test]
  fn only_server_side_and_explicit_retry_statuses_are_worth_retrying() {
    for status in [408, 425, 429, 500, 502, 503, 504, 599] {
      assert!(is_retryable_status(status), "{status} 应当重试");
    }
    // 令牌 / 地址 / 方法 / 体积不对：重试多少次都一样，要用户改配置。
    for status in [301, 400, 401, 403, 404, 405, 409, 413, 415, 422] {
      assert!(!is_retryable_status(status), "{status} 不应重试");
    }
  }

  #[test]
  fn sync_state_tolerates_missing_fields() {
    let s: SyncState = serde_json::from_str("{}").unwrap();
    assert_eq!(s, SyncState::default());
    let s: SyncState = serde_json::from_str(r#"{"base":18446744073709551615}"#).unwrap();
    assert_eq!(s.base, Some(fp(u64::MAX)));
    assert!(!s.dirty);
  }

  #[test]
  fn fingerprint_is_stored_as_a_bare_number() {
    // 换成新类型之后，旧版本已经写进 localStorage 的状态仍然读得出来。
    let json = serde_json::to_string(&SyncState {
      base: Some(fp(5)),
      dirty: true,
    })
    .unwrap();
    assert_eq!(json, r#"{"base":5,"dirty":true}"#);
  }

  /// `docs/storage.md` 登记的每个 key 必须在这里归类（同步 / 设备专属），且与 [`is_device_local_key`] 一致。
  ///
  /// 新增 key 却忘了归类，会让它的每次写入都触发一次同步，或者把只属于这台设备的东西同步到别处。
  /// 这条测试把「新增 key 要想一想它该不该同步」变成必须回答的问题。
  #[test]
  fn every_documented_key_is_classified() {
    // 同步的用户数据。
    const SYNCED: &[&str] = &[
      "practice",
      "practice:lastMode",
      "practice:noResumePrompt",
      "exam:savedState",
      "exam:answerCardFilter",
      "exam:showExplanation",
      "bookmarks",
      "exam-history",
      "logbook",
      "station-info",
      "daily-checkin",
      "countdowns",
      "morse-stats",
      "dxcc_wanted_done",
      "mistake-book",
      "study-stats",
      "mistake-book:seeded",
      "study-stats:seen-seeded",
      "morse-koch",
      "dx-alerts",
      "learning-plan",
      "study-daily",
      "morse-runner",
      "contest-session",
      "sat-watch",
      "grid-map-filters",
      "listen-settings",
      "qsl-label-layout",
      "card-review",
      "question-notes",
      "bookmark-groups",
      "question-stats",
      "achievements-seen",
      "morse-trainer",
      "morse-send",
      "exam:last-review",
    ];
    // 只属于这台设备。
    const DEVICE: &[&str] = &[
      "ui:shortcutsHelpSeen",
      "theme",
      "locale",
      "cat-settings",
      "app:changelog-seen",
      "bank-digest",
      "push-subscription",
      "sat-watch-notified",
      "sat-apt-notified",
      "remote-sync",
      "remote-sync-state",
    ];

    /// 文档表格第一列里每个反引号包起来的 key；带 `{…}` 的是前缀模式，取 `{` 之前的部分。
    fn documented_keys() -> Vec<(String, bool)> {
      include_str!("../../../docs/storage.md")
        .lines()
        .filter(|line| line.starts_with("| `"))
        .flat_map(|line| {
          let cell = line
            .trim_start_matches("| ")
            .split(" | ")
            .next()
            .unwrap_or_default();
          cell
            .split('`')
            .skip(1)
            .step_by(2)
            .map(|token| match token.find('{') {
              Some(i) => (token[..i].trim_end_matches(':').to_owned(), true),
              None => (token.to_owned(), false),
            })
            .collect::<Vec<_>>()
        })
        .collect()
    }

    let documented = documented_keys();
    assert!(
      documented.len() > 40,
      "没解析出 docs/storage.md 的 key 表：{}",
      documented.len()
    );
    for (token, is_prefix) in &documented {
      let synced = SYNCED.contains(&token.as_str());
      let device = DEVICE.contains(&token.as_str());
      assert!(
        synced ^ device,
        "docs/storage.md 登记的 `{token}` 必须且只能在 SYNCED / DEVICE 之一里归类（改 is_device_local_key 或本测试）"
      );
      // 前缀模式（`practice:{版本}:{题库}`）用一个真实长相的 key 去探。
      let probe = if *is_prefix {
        format!("{token}:probe")
      } else {
        token.clone()
      };
      assert_eq!(
        is_device_local_key(&probe),
        device,
        "`{probe}` 的归类与 is_device_local_key 不一致"
      );
    }
    // 名单里不许留文档里已经没有的 key，免得它悄悄腐烂。
    for name in SYNCED.iter().chain(DEVICE) {
      assert!(
        documented.iter().any(|(token, _)| token == name),
        "归类名单里的 `{name}` 在 docs/storage.md 里找不到"
      );
    }
  }

  #[test]
  fn only_sync_bookkeeping_is_excluded_from_backups() {
    assert!(is_excluded_from_backup("remote-sync-state"));
    // 备份仍包含主题等设备偏好（手动导出 / 恢复要完整），只有同步自己的两个 key 例外。
    assert!(!is_excluded_from_backup("theme"));
  }
}
