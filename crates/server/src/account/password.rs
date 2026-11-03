//! 口令的哈希与校验（Argon2id）。
//!
//! 参数用库的默认值（内存 19 MiB、2 轮、单线程，OWASP 的最低推荐档）：盐与参数都编码在
//! PHC 字符串里，将来调高参数只影响新哈希，旧哈希照样能验。
//!
//! Argon2 是故意做得又慢又吃内存的：调用方必须放进 `spawn_blocking`，并用信号量限制并发数
//! （见 `AccountService::hash_slots`），否则一批登录请求就能把进程的内存撑满。

use std::sync::OnceLock;

use argon2::Argon2;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};

/// 哈希一个口令，返回 PHC 字符串。
pub fn hash(password: &str) -> Result<String, argon2::password_hash::Error> {
  Argon2::default()
    .hash_password(password.as_bytes())
    .map(|h| h.to_string())
}

/// 口令与哈希是否匹配。哈希坏掉（解析失败）按不匹配处理。
pub fn verify(password: &str, phc: &str) -> bool {
  Argon2::default()
    .verify_password(password.as_bytes(), phc)
    .is_ok()
}

/// 账号不存在时也做一次等价的校验，让「账号不存在」与「口令不对」的耗时看不出差别，
/// 不能靠响应时间探测哪些账号存在。
pub fn verify_dummy(password: &str) {
  static DUMMY: OnceLock<String> = OnceLock::new();
  let phc = DUMMY.get_or_init(|| hash("dummy password for timing").unwrap_or_default());
  let _ = verify(password, phc);
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_hash_verifies_only_its_own_password() {
    let phc = hash("correct horse").unwrap();
    assert!(phc.starts_with("$argon2id$"));
    assert!(verify("correct horse", &phc));
    assert!(!verify("correct horsf", &phc));
    assert!(!verify("", &phc));
  }

  #[test]
  fn the_same_password_gets_a_different_salt_every_time() {
    assert_ne!(hash("same").unwrap(), hash("same").unwrap());
  }

  #[test]
  fn a_corrupt_hash_never_matches() {
    assert!(!verify("anything", "not a phc string"));
    assert!(!verify("anything", ""));
  }
}
