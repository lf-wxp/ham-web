//! 在线账号：服务端（`crates/server`）与前端（`crates/app`）共用的契约。
//!
//! 只放不依赖浏览器 / 数据库的部分：账号名与口令的规则、请求 / 响应的数据结构、错误码、
//! 路径与头名。两端引用同一份常量，改一处就不会出现「前端以为是 A、后端按 B 校验」。
//! 数据库表结构与整体设计见 `docs/domain/account-sync.md`，接口见 `docs/reference/api.md`。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::delta::{Digest, Manifest};
use crate::remote_sync::Fingerprint;

/// 账号名最短长度（字符数）。
pub const USERNAME_MIN: usize = 3;
/// 账号名最长长度（字符数）。
pub const USERNAME_MAX: usize = 32;
/// 口令最短长度（字节数）。
pub const PASSWORD_MIN: usize = 8;
/// 口令最长长度（字节数）：Argon2 对超长输入的耗时线性增长，不设上限等于给匿名请求一把放大器。
pub const PASSWORD_MAX: usize = 128;

/// 会话 Cookie 的名字。
pub const SESSION_COOKIE: &str = "ham_session";
/// 前端声明自己认的账号 id：服务端核对后不一致就拒绝，防止两个标签页登录了不同账号时串写。
pub const ACCOUNT_ID_HEADER: &str = "x-account-id";

/// 接口路径前缀。
pub const API_BASE: &str = "/api/account";

/// 账号名的校验结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialError {
  /// 长度不在 [`USERNAME_MIN`]..=[`USERNAME_MAX`]。
  UsernameLength,
  /// 含字母、数字、`_`、`-`、`.` 之外的字符，或不是以字母 / 数字开头。
  UsernameChars,
  /// 口令短于 [`PASSWORD_MIN`]。
  PasswordShort,
  /// 口令长于 [`PASSWORD_MAX`]。
  PasswordLong,
  /// 口令与账号名相同（忽略大小写）。
  PasswordIsUsername,
}

/// 校验账号名，返回去掉首尾空白后的写法。
///
/// 只收 ASCII 字母数字与 `_ - .`：呼号（`BG4XXX`）、邮箱前缀都能用；不收空白、斜杠、
/// 全角字符，是为了让「看起来一样的两个名字」不会是两个账号（`COLLATE NOCASE` 只管 ASCII）。
///
/// # Errors
///
/// 见 [`CredentialError`]。
pub fn validate_username(raw: &str) -> Result<String, CredentialError> {
  let name = raw.trim();
  let len = name.chars().count();
  if !(USERNAME_MIN..=USERNAME_MAX).contains(&len) {
    return Err(CredentialError::UsernameLength);
  }
  let first_ok = name
    .chars()
    .next()
    .is_some_and(|c| c.is_ascii_alphanumeric());
  let rest_ok = name
    .chars()
    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'));
  if first_ok && rest_ok {
    Ok(name.to_owned())
  } else {
    Err(CredentialError::UsernameChars)
  }
}

/// 校验口令（不做任何裁剪：空格也是口令的一部分）。
///
/// 规则刻意从简：只管长度与「别和账号名一样」。强制大小写 / 符号的组合规则会让人写出
/// `Password1!` 这类既难记又好猜的口令，长度才是主要的熵来源。
///
/// # Errors
///
/// 见 [`CredentialError`]。
pub fn validate_password(password: &str, username: &str) -> Result<(), CredentialError> {
  if password.len() < PASSWORD_MIN {
    return Err(CredentialError::PasswordShort);
  }
  if password.len() > PASSWORD_MAX {
    return Err(CredentialError::PasswordLong);
  }
  if password.eq_ignore_ascii_case(username.trim()) {
    return Err(CredentialError::PasswordIsUsername);
  }
  Ok(())
}

/// 接口返回的错误码（`{"error":{"code":"…"}}`）。
///
/// 前端按码翻成各语言的文案，服务端只给英文 `message` 便于排查。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
  /// 服务器没启用在线账号（没配 `ACCOUNT_DB`）。
  Unavailable,
  /// 没登录或会话过期。
  Unauthorized,
  /// 账号名或口令不对（两者不区分，免得被用来探测账号是否存在）。
  BadCredentials,
  /// 请求体里的账号名不合规。
  InvalidUsername,
  /// 请求体里的口令不合规。
  InvalidPassword,
  /// 账号名已被占用。
  UsernameTaken,
  /// 服务器关闭了注册。
  RegistrationClosed,
  /// 尝试太频繁。
  TooManyAttempts,
  /// 备份内容不是 `{ key: value }` 形态的 JSON。
  InvalidBackup,
  /// 备份超过大小上限。
  TooLarge,
  /// 写入条件不满足：云端已被别处更新（`If-Match` / `If-None-Match`）。
  PreconditionFailed,
  /// 请求带的账号 id 与当前登录的不是同一个。
  AccountChanged,
  /// 请求格式不对（缺字段、不是 JSON）。
  BadRequest,
  /// 服务器内部错误。
  Internal,
}

impl ErrorCode {
  /// 线上的稳定写法。
  #[must_use]
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Unavailable => "unavailable",
      Self::Unauthorized => "unauthorized",
      Self::BadCredentials => "bad_credentials",
      Self::InvalidUsername => "invalid_username",
      Self::InvalidPassword => "invalid_password",
      Self::UsernameTaken => "username_taken",
      Self::RegistrationClosed => "registration_closed",
      Self::TooManyAttempts => "too_many_attempts",
      Self::InvalidBackup => "invalid_backup",
      Self::TooLarge => "too_large",
      Self::PreconditionFailed => "precondition_failed",
      Self::AccountChanged => "account_changed",
      Self::BadRequest => "bad_request",
      Self::Internal => "internal",
    }
  }

  /// 从线上写法还原；不认识的（比自己新的服务端加了新码）返回 `None`。
  #[must_use]
  pub fn parse(code: &str) -> Option<Self> {
    Self::ALL.into_iter().find(|c| c.as_str() == code)
  }

  /// 全部错误码。
  pub const ALL: [Self; 14] = [
    Self::Unavailable,
    Self::Unauthorized,
    Self::BadCredentials,
    Self::InvalidUsername,
    Self::InvalidPassword,
    Self::UsernameTaken,
    Self::RegistrationClosed,
    Self::TooManyAttempts,
    Self::InvalidBackup,
    Self::TooLarge,
    Self::PreconditionFailed,
    Self::AccountChanged,
    Self::BadRequest,
    Self::Internal,
  ];

  /// 对应的 HTTP 状态码。
  #[must_use]
  pub const fn status(self) -> u16 {
    match self {
      Self::Unavailable => 503,
      Self::Unauthorized | Self::BadCredentials => 401,
      Self::InvalidUsername | Self::InvalidPassword | Self::InvalidBackup | Self::BadRequest => 400,
      Self::UsernameTaken | Self::AccountChanged => 409,
      Self::RegistrationClosed => 403,
      Self::TooManyAttempts => 429,
      Self::TooLarge => 413,
      Self::PreconditionFailed => 412,
      Self::Internal => 500,
    }
  }

  /// 账号名 / 口令校验失败对应的错误码。
  #[must_use]
  pub const fn from_credential(e: CredentialError) -> Self {
    match e {
      CredentialError::UsernameLength | CredentialError::UsernameChars => Self::InvalidUsername,
      CredentialError::PasswordShort
      | CredentialError::PasswordLong
      | CredentialError::PasswordIsUsername => Self::InvalidPassword,
    }
  }
}

/// 登录 / 注册的请求体。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
  /// 账号名。
  pub username: String,
  /// 口令。
  pub password: String,
}

/// 修改口令的请求体。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePassword {
  /// 当前口令。
  pub current: String,
  /// 新口令。
  pub new: String,
}

/// 只需要再确认一次口令的请求体（注销账号）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmPassword {
  /// 当前口令。
  pub password: String,
}

/// 云端备份的概况（不含内容）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupInfo {
  /// 内容字节数。
  pub size: u64,
  /// 最后一次写入的时间（Unix 秒）。
  pub updated_at: i64,
  /// 云端的**版本号**：对云端全部同步 key 的 `(key, 摘要)` 清单求的（见 [`crate::delta::manifest_fingerprint`]），
  /// 也是 `ETag` / `If-Match` 比较的对象。
  ///
  /// 它只回答「云端现在是不是我上次看到的那一版」，不是内容完整性的凭据，也不是本机与云端「谁改了」
  /// 的判据：自动同步靠每个 key 一对摘要（[`crate::delta::Synced`]）逐个判断，整份指纹对它没有用处。
  /// 手动「上传」成功后的回执里也带它，只用来显示与做条件写入。
  pub fingerprint: Fingerprint,
}

// ───────────────────────── 按 key 的增量同步 ─────────────────────────
//
// 整份备份的 `GET` / `PUT /backup` 仍在（手动上传 / 恢复 / 合并用）。自动同步走下面三个接口，
// 只交换有变化的 key，设计见 `crate::delta` 与 `docs/domain/account-sync.md`。

/// 清单接口的路径（相对 [`API_BASE`]）。
pub const SYNC_MANIFEST_PATH: &str = "/sync/manifest";
/// 按 key 取值接口的路径。
pub const SYNC_PULL_PATH: &str = "/sync/pull";
/// 按 key 上传改动接口的路径。
pub const SYNC_PUSH_PATH: &str = "/sync/push";

/// 一次请求最多带多少个 key：账号里的 key 最多几百个（每张 QSL 影像一个），
/// 这个数留足余量，又不让一个请求把服务端的处理拖成无界。
pub const SYNC_MAX_KEYS: usize = 20_000;
/// key 的最大字节数。
pub const SYNC_MAX_KEY_LEN: usize = 512;

/// `GET /sync/manifest`：云端全部同步 key 的清单。还没有备份为 `204`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncManifest {
  /// 清单的指纹，同 [`BackupInfo::fingerprint`]：与本机上次记下的一样就说明云端没变，什么都不用取。
  pub fingerprint: Fingerprint,
  /// 最后一次写入的时间（Unix 秒）。
  pub updated_at: i64,
  /// 内容总字节数。
  pub size: u64,
  /// `key → 摘要`。
  pub keys: Manifest,
}

/// `POST /sync/pull` 的请求体：要哪些 key 的值。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequest {
  pub keys: Vec<String>,
}

/// `POST /sync/pull` 的响应：云端现在有的那些 key 的值（请求了但已不存在的不在里面）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullResponse {
  pub values: BTreeMap<String, String>,
}

/// 一条上传的改动。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
  pub key: String,
  /// 本机看到的这个 key 在云端的摘要；`None` = 本机看到的是云端没有这个 key。
  ///
  /// 云端现在的摘要与它对不上，说明别处在这期间改过这个 key，整批改动都不会生效（`412`）。
  pub base: Option<Digest>,
  /// 新值；`None` = 删除这个 key。
  pub value: Option<String>,
}

/// `POST /sync/push` 的请求体。整批要么全部生效，要么全部不生效。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PushRequest {
  pub changes: Vec<Change>,
}

/// `POST /sync/push` 的响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PushResponse {
  /// 写入之后云端的概况。
  pub info: BackupInfo,
  /// 写入之后云端的完整清单：本机拿它记下刚上传的那些 key 在云端是什么样。
  pub keys: Manifest,
}

/// `PUT /backup`（整份替换）成功的响应：概况，外加写入之后云端的完整清单。
///
/// 与 [`BackupInfo`] 字段平铺在一起，只读概况的调用方（`BackupInfo` 反序列化时忽略多出来的 `keys`）不受影响。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupReceipt {
  #[serde(flatten)]
  pub info: BackupInfo,
  /// 写入之后云端的完整清单：手动上传之后据此建立每个 key 的基线，不必再多发一次请求。
  pub keys: Manifest,
}

/// 已登录的账号。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserInfo {
  /// 账号 id（数据库主键；基线的「主人」靠它认）。
  pub id: u64,
  /// 账号名（保留注册时的大小写）。
  pub username: String,
  /// 注册时间（Unix 秒）。
  pub created_at: i64,
  /// 云端备份概况；还没传过为 `None`。
  pub backup: Option<BackupInfo>,
}

/// `GET /api/account/session` 以及登录 / 注册成功的响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInfo {
  /// 服务器是否启用了在线账号。
  pub enabled: bool,
  /// 是否开放注册。
  pub registration_open: bool,
  /// 当前登录的账号；没登录为 `None`。
  pub user: Option<UserInfo>,
}

/// `If-Match` / `If-None-Match` 的值看不懂。
///
/// 单独成类型（而不是 `Result<_, ()>`）：调用方要显式处理它——悄悄当成「无条件覆盖」，
/// 会把一次防覆盖的保护变成覆盖。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidPrecondition;

/// 云端备份写入的前置条件（来自 `If-Match` / `If-None-Match`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precondition {
  /// 无条件覆盖（用户手动「上传」）。
  None,
  /// 云端必须还没有备份。
  Absent,
  /// 云端当前内容的指纹必须是这个值。
  Matches(Fingerprint),
}

impl Precondition {
  /// 云端现状（`current`：没有备份为 `None`）是否满足条件。
  #[must_use]
  pub fn holds(self, current: Option<Fingerprint>) -> bool {
    match self {
      Self::None => true,
      Self::Absent => current.is_none(),
      Self::Matches(fp) => current == Some(fp),
    }
  }

  /// 按请求头的值解析：`if_match` 是指纹数值，`if_none_match` 只认 `*`。
  ///
  /// 两个头都给时 `If-Match` 优先。值不合法返回 [`InvalidPrecondition`]，由调用方回 400，
  /// 而不是悄悄当成「无条件覆盖」——后者会把一次防覆盖的保护变成覆盖。
  ///
  /// # Errors
  ///
  /// 头存在但格式不对。
  pub fn from_headers(
    if_match: Option<&str>,
    if_none_match: Option<&str>,
  ) -> Result<Self, InvalidPrecondition> {
    if let Some(v) = if_match {
      let n = v
        .trim()
        .trim_matches('"')
        .parse::<u64>()
        .map_err(|_| InvalidPrecondition)?;
      return Ok(Self::Matches(Fingerprint::from_u64(n)));
    }
    match if_none_match.map(str::trim) {
      None => Ok(Self::None),
      Some("*") => Ok(Self::Absent),
      Some(_) => Err(InvalidPrecondition),
    }
  }

  /// 要发的请求头 `(名, 值)`；无条件覆盖不带头。
  #[must_use]
  pub fn header(self) -> Option<(&'static str, String)> {
    match self {
      Self::None => None,
      Self::Absent => Some(("If-None-Match", "*".to_owned())),
      Self::Matches(fp) => Some(("If-Match", fp.get().to_string())),
    }
  }

  /// 自动写入用的条件：有基线就要求云端还停在基线上，没有基线就要求云端是空的。
  #[must_use]
  pub const fn from_base(base: Option<Fingerprint>) -> Self {
    match base {
      Some(fp) => Self::Matches(fp),
      None => Self::Absent,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn usernames_accept_callsigns_and_reject_lookalikes() {
    for ok in [
      "BG4XXX",
      "bg4xxx",
      "a_b-c.d",
      "  ham123  ",
      "abc",
      &"a".repeat(32),
    ] {
      assert!(validate_username(ok).is_ok(), "{ok:?}");
    }
    // 返回的是去掉空白后的写法。
    assert_eq!(validate_username("  ham123 ").unwrap(), "ham123");
    for short in ["", "ab", "   "] {
      assert_eq!(
        validate_username(short),
        Err(CredentialError::UsernameLength),
        "{short:?}"
      );
    }
    assert_eq!(
      validate_username(&"a".repeat(33)),
      Err(CredentialError::UsernameLength)
    );
    for bad in [
      "a b c",
      "ab/c",
      "_abc",
      ".abc",
      "-abc",
      "名字名字",
      // 全角字母：看起来是 abcd，却是另一组码点。
      "\u{ff41}\u{ff42}\u{ff43}\u{ff44}",
      "ab\nc",
    ] {
      assert_eq!(
        validate_username(bad),
        Err(CredentialError::UsernameChars),
        "{bad:?}"
      );
    }
  }

  #[test]
  fn passwords_are_judged_by_length_and_not_trimmed() {
    assert!(validate_password("correct horse", "ham").is_ok());
    // 空格算口令的一部分：8 个空格是合法口令（不鼓励，但不能悄悄裁成空串）。
    assert!(validate_password("        ", "ham").is_ok());
    assert_eq!(
      validate_password("short", "ham"),
      Err(CredentialError::PasswordShort)
    );
    assert!(validate_password(&"x".repeat(PASSWORD_MAX), "ham").is_ok());
    assert_eq!(
      validate_password(&"x".repeat(PASSWORD_MAX + 1), "ham"),
      Err(CredentialError::PasswordLong)
    );
    assert_eq!(
      validate_password("BG4XXX123", "bg4xxx123"),
      Err(CredentialError::PasswordIsUsername)
    );
    // 长度按字节：多字节字符更快达到上限，而不是绕过它。
    assert_eq!(
      validate_password(&"密".repeat(43), "ham"),
      Err(CredentialError::PasswordLong)
    );
  }

  #[test]
  fn every_error_code_round_trips_and_maps_to_a_distinct_wire_name() {
    let mut seen = std::collections::BTreeSet::new();
    for code in ErrorCode::ALL {
      assert_eq!(ErrorCode::parse(code.as_str()), Some(code));
      assert!(
        seen.insert(code.as_str()),
        "重复的线上名：{}",
        code.as_str()
      );
      assert!((400..600).contains(&code.status()), "{}", code.as_str());
    }
    assert_eq!(ErrorCode::parse("from_the_future"), None);
  }

  #[test]
  fn credential_errors_map_to_the_matching_field_code() {
    assert_eq!(
      ErrorCode::from_credential(CredentialError::UsernameChars),
      ErrorCode::InvalidUsername
    );
    assert_eq!(
      ErrorCode::from_credential(CredentialError::PasswordShort),
      ErrorCode::InvalidPassword
    );
  }

  fn fp(n: u64) -> Fingerprint {
    Fingerprint::from_u64(n)
  }

  #[test]
  fn preconditions_decide_whether_a_write_may_overwrite() {
    assert!(Precondition::None.holds(None));
    assert!(Precondition::None.holds(Some(fp(1))));
    assert!(Precondition::Absent.holds(None));
    assert!(!Precondition::Absent.holds(Some(fp(1))));
    assert!(Precondition::Matches(fp(1)).holds(Some(fp(1))));
    assert!(!Precondition::Matches(fp(1)).holds(Some(fp(2))));
    // 云端被清空后，「必须停在基线上」不成立：不能把别人的清空当成可覆盖。
    assert!(!Precondition::Matches(fp(1)).holds(None));
  }

  #[test]
  fn preconditions_are_parsed_strictly_from_headers() {
    assert_eq!(
      Precondition::from_headers(None, None),
      Ok(Precondition::None)
    );
    assert_eq!(
      Precondition::from_headers(Some("42"), None),
      Ok(Precondition::Matches(fp(42)))
    );
    // 标准写法带引号：两种都认。
    assert_eq!(
      Precondition::from_headers(Some("\"42\""), None),
      Ok(Precondition::Matches(fp(42)))
    );
    assert_eq!(
      Precondition::from_headers(None, Some("*")),
      Ok(Precondition::Absent)
    );
    assert_eq!(
      Precondition::from_headers(Some("7"), Some("*")),
      Ok(Precondition::Matches(fp(7)))
    );
    // 看不懂的条件不能退化成「无条件覆盖」。
    for (m, n) in [
      (Some("abc"), None),
      (None, Some("\"x\"")),
      (Some("-1"), None),
    ] {
      assert_eq!(Precondition::from_headers(m, n), Err(InvalidPrecondition));
    }
  }

  #[test]
  fn preconditions_survive_the_trip_through_their_own_header() {
    for pre in [
      Precondition::None,
      Precondition::Absent,
      Precondition::Matches(fp(u64::MAX)),
    ] {
      let (m, n) = match pre.header() {
        Some(("If-Match", v)) => (Some(v), None),
        Some(("If-None-Match", v)) => (None, Some(v)),
        _ => (None, None),
      };
      assert_eq!(
        Precondition::from_headers(m.as_deref(), n.as_deref()),
        Ok(pre)
      );
    }
  }

  #[test]
  fn auto_writes_are_conditioned_on_the_baseline() {
    assert_eq!(
      Precondition::from_base(Some(fp(5))),
      Precondition::Matches(fp(5))
    );
    assert_eq!(Precondition::from_base(None), Precondition::Absent);
  }

  #[test]
  fn session_info_round_trips_with_a_full_width_fingerprint() {
    let info = SessionInfo {
      enabled: true,
      registration_open: false,
      user: Some(UserInfo {
        id: 3,
        username: "BG4XXX".into(),
        created_at: 1_700_000_000,
        backup: Some(BackupInfo {
          size: 12,
          updated_at: 1_700_000_100,
          // 超过 2^53：走 JS 的 `JSON.parse` 会丢精度，两端都必须用 serde_json 解析。
          fingerprint: fp(u64::MAX),
        }),
      }),
    };
    let json = serde_json::to_string(&info).unwrap();
    assert_eq!(serde_json::from_str::<SessionInfo>(&json).unwrap(), info);
  }
}
