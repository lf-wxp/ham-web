//! 账号接口的错误：统一渲染为 `{ "error": { "code": …, "message": … } }`，与开放 API 同形。
//!
//! `code` 是前端翻译文案的依据（见 `ham_web_core::account::ErrorCode`），`message` 只给排查用。

use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use ham_web_core::account::{CredentialError, ErrorCode};

use super::store::StoreError;

/// 账号接口的失败。
#[derive(Debug)]
pub struct ApiError {
  pub code: ErrorCode,
}

impl ApiError {
  pub const fn new(code: ErrorCode) -> Self {
    Self { code }
  }

  fn message(&self) -> &'static str {
    match self.code {
      ErrorCode::Unavailable => "online accounts are not enabled on this server",
      ErrorCode::Unauthorized => "not signed in, or the session has expired",
      ErrorCode::BadCredentials => "wrong username or password",
      ErrorCode::InvalidUsername => "username must be 3-32 letters, digits, '_', '-' or '.'",
      ErrorCode::InvalidPassword => "password must be 8-128 bytes and differ from the username",
      ErrorCode::UsernameTaken => "username is already taken",
      ErrorCode::RegistrationClosed => "registration is closed on this server",
      ErrorCode::TooManyAttempts => "too many attempts, try again later",
      ErrorCode::InvalidBackup => "backup must be a JSON object of string values",
      ErrorCode::TooLarge => "backup is too large",
      ErrorCode::PreconditionFailed => "the cloud backup changed since the given baseline",
      ErrorCode::AccountChanged => "the signed-in account is not the one the request expects",
      ErrorCode::BadRequest => "malformed request",
      ErrorCode::Internal => "internal error",
    }
  }
}

impl From<CredentialError> for ApiError {
  fn from(e: CredentialError) -> Self {
    Self::new(ErrorCode::from_credential(e))
  }
}

impl From<StoreError> for ApiError {
  fn from(e: StoreError) -> Self {
    match e {
      StoreError::UsernameTaken => Self::new(ErrorCode::UsernameTaken),
      // 人数满了对用户就是「这台服务器不收新账号了」：复用同一个错误码，前端不必多一条文案。
      StoreError::UserLimit => {
        tracing::warn!("在线账号已达人数上限（ACCOUNT_MAX_USERS），拒绝了一次注册");
        Self::new(ErrorCode::RegistrationClosed)
      }
      StoreError::PreconditionFailed => Self::new(ErrorCode::PreconditionFailed),
      StoreError::Db(db) => {
        tracing::error!("账户数据库错误：{db}");
        Self::new(ErrorCode::Internal)
      }
    }
  }
}

impl IntoResponse for ApiError {
  fn into_response(self) -> Response {
    let body = serde_json::json!({
      "error": { "code": self.code.as_str(), "message": self.message() }
    });
    let status =
      StatusCode::from_u16(self.code.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (
      status,
      [(CONTENT_TYPE, "application/json")],
      body.to_string(),
    )
      .into_response()
  }
}
