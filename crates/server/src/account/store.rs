//! 账户数据库（SQLite）：用户、会话、云端备份三张表。
//!
//! 全部是**同步**调用：`rusqlite` 的连接不是 `Sync`，这里用一把 `Mutex` 串行化；
//! 调用方（`handlers`）一律放进 `spawn_blocking`，不在异步运行时线程上等锁或等磁盘。
//! 单连接足够：写入量是「每个用户几十秒一次」，瓶颈是 Argon2 而不是 SQLite；
//! WAL 模式让读不阻塞写，进程崩溃后不会丢已提交的事务。
//!
//! 表结构的设计理由见 `docs/domain/account-sync.md`「数据结构」。

use std::path::Path;
use std::sync::Mutex;

use anyhow::Context;
use ham_web_core::account::{BackupInfo, Precondition};
use ham_web_core::remote_sync::Fingerprint;
use rusqlite::{Connection, OptionalExtension, ffi, params};

/// 按顺序执行的迁移；第 `n` 条把 `user_version` 从 `n` 升到 `n + 1`。
///
/// 只追加、不改写已发布的条目：线上库的 `user_version` 决定从哪一条继续。
const MIGRATIONS: &[&str] = &[
  // 1：初版。
  r"
  CREATE TABLE users (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    -- NOCASE：`BG4XXX` 与 `bg4xxx` 是同一个账号；账号名只收 ASCII（见 core::account）。
    username            TEXT    NOT NULL UNIQUE COLLATE NOCASE,
    password_hash       TEXT    NOT NULL,          -- Argon2id 的 PHC 字符串，盐与参数都在里面
    created_at          INTEGER NOT NULL,          -- Unix 秒
    password_changed_at INTEGER NOT NULL
  );

  CREATE TABLE sessions (
    -- 只存令牌的 SHA-256：库文件泄露（备份、误传）时拿不到可用的登录态。
    token_hash   BLOB    PRIMARY KEY,
    user_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at   INTEGER NOT NULL,
    last_seen_at INTEGER NOT NULL,
    expires_at   INTEGER NOT NULL,
    user_agent   TEXT
  );
  CREATE INDEX sessions_user    ON sessions(user_id);
  CREATE INDEX sessions_expires ON sessions(expires_at);

  -- 每个账号至多一份云端备份：与「自定义远端」一样是整体替换的快照，不做历史版本。
  CREATE TABLE backups (
    user_id     INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    body        BLOB    NOT NULL,                  -- 与「导出备份」同格式的 JSON（UTF-8）
    -- 内容指纹（core::remote_sync::fingerprint）。存十进制文本：u64 超出 SQLite INTEGER（i64）的范围。
    fingerprint TEXT    NOT NULL,
    size        INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
  );
  ",
];

/// 同一账号最多保留的会话数：超出时淘汰最久没用的，免得登录接口被刷成无限增长的表。
pub const MAX_SESSIONS_PER_USER: usize = 10;

/// 刷新「最近使用时间」的最小间隔（秒）：避免每个请求都写一次库。
const TOUCH_INTERVAL_SECS: i64 = 3600;

/// 一行用户记录。
#[derive(Debug, Clone)]
pub struct UserRow {
  pub id: u64,
  pub username: String,
  pub password_hash: String,
  pub created_at: i64,
}

/// 一次成功的会话查找。
#[derive(Debug, Clone)]
pub struct SessionHit {
  pub user: UserRow,
  /// 这次访问顺带把有效期续了：调用方要给浏览器重发 `Set-Cookie`，
  /// 否则库里的有效期往后挪了，浏览器那份 Cookie 仍按登录时的 `Max-Age` 过期。
  pub renewed: bool,
}

/// 预期内的失败与数据库故障分开：前者对应 HTTP 4xx，后者是 5xx。
#[derive(Debug)]
pub enum StoreError {
  /// 账号名已被占用。
  UsernameTaken,
  /// 账号数已达上限。
  UserLimit,
  /// 写入条件（`If-Match` / `If-None-Match`）不满足。
  PreconditionFailed,
  /// 数据库错误。
  Db(rusqlite::Error),
}

impl From<rusqlite::Error> for StoreError {
  fn from(e: rusqlite::Error) -> Self {
    Self::Db(e)
  }
}

type Result<T> = std::result::Result<T, StoreError>;

/// 账户数据库。
pub struct Store {
  conn: Mutex<Connection>,
}

/// 只认「唯一约束」：外键、非空等其他约束冲突是程序错误，不能被说成「账号名已被占用」。
fn is_unique_violation(e: &rusqlite::Error) -> bool {
  matches!(e, rusqlite::Error::SqliteFailure(f, _) if f.extended_code == ffi::SQLITE_CONSTRAINT_UNIQUE)
}

/// 指纹在库里是十进制文本；解析不出来说明库被改坏了，按数据库错误报，
/// 而不是悄悄当成 0（0 也是个合法指纹，会让条件写入的判断在错误的基线上通过）。
fn parse_fingerprint(text: &str, column: usize) -> rusqlite::Result<Fingerprint> {
  text.parse::<u64>().map(Fingerprint::from_u64).map_err(|e| {
    rusqlite::Error::FromSqlConversionFailure(column, rusqlite::types::Type::Text, Box::new(e))
  })
}

/// 创建库文件（已存在则不动内容），新建时只给属主读写：里面有口令哈希与全部用户的备份。
/// SQLite 为库创建的 `-wal` / `-shm` 沿用主文件的权限。
fn ensure_private_file(path: &Path) -> std::io::Result<()> {
  let mut opts = std::fs::OpenOptions::new();
  // append：不截断已有内容。
  opts.create(true).append(true);
  #[cfg(unix)]
  {
    use std::os::unix::fs::OpenOptionsExt;
    opts.mode(0o600);
  }
  opts.open(path).map(|_| ())
}

fn user_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<UserRow> {
  Ok(UserRow {
    id: row.get::<_, i64>(0)?.cast_unsigned(),
    username: row.get(1)?,
    password_hash: row.get(2)?,
    created_at: row.get(3)?,
  })
}

const USER_COLUMNS: &str = "id, username, password_hash, created_at";

impl Store {
  /// 打开（必要时创建）数据库文件并迁移到最新结构。
  ///
  /// # Errors
  ///
  /// 目录不可写、文件损坏、迁移失败。
  pub fn open(path: &Path) -> anyhow::Result<Self> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
      std::fs::create_dir_all(dir)
        .with_context(|| format!("无法创建账户数据库目录 {}", dir.display()))?;
    }
    ensure_private_file(path)
      .with_context(|| format!("无法创建账户数据库文件 {}", path.display()))?;
    let conn =
      Connection::open(path).with_context(|| format!("无法打开账户数据库 {}", path.display()))?;
    Self::init(conn)
  }

  /// 内存数据库（测试用）。
  ///
  /// # Errors
  ///
  /// 迁移失败。
  #[cfg(test)]
  pub fn open_in_memory() -> anyhow::Result<Self> {
    Self::init(Connection::open_in_memory()?)
  }

  fn init(mut conn: Connection) -> anyhow::Result<Self> {
    // WAL：读写互不阻塞，且崩溃后以日志为准恢复；对内存库无效（返回 "memory"），不影响。
    // 外键默认关闭，必须每个连接各开一次：`ON DELETE CASCADE` 靠它。
    conn.execute_batch(
      "PRAGMA journal_mode = WAL;
       PRAGMA synchronous = NORMAL;
       PRAGMA foreign_keys = ON;
       PRAGMA busy_timeout = 5000;",
    )?;
    let version =
      usize::try_from(conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))?)
        .unwrap_or(usize::MAX);
    anyhow::ensure!(
      version <= MIGRATIONS.len(),
      "账户数据库版本（{version}）比本程序认识的（{}）新：拒绝用旧程序改写新库",
      MIGRATIONS.len()
    );
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(version) {
      let tx = conn.transaction()?;
      tx.execute_batch(sql)?;
      // `user_version` 不接受绑定参数，只能拼；`i` 是循环下标，不是外部输入。
      tx.execute_batch(&format!("PRAGMA user_version = {}", i + 1))?;
      tx.commit()?;
    }
    Ok(Self {
      conn: Mutex::new(conn),
    })
  }

  fn with<R>(&self, f: impl FnOnce(&mut Connection) -> rusqlite::Result<R>) -> Result<R> {
    // 持锁时 panic 会毒化锁；库本身没有被破坏（事务会回滚），继续用比让之后每个请求都 panic 好。
    let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut conn).map_err(StoreError::from)
  }

  // ───────────────────────── 用户 ─────────────────────────

  /// 创建用户。
  ///
  /// # Errors
  ///
  /// [`StoreError::UsernameTaken`]（含大小写不同的同名）。
  #[cfg(test)]
  pub fn create_user(&self, username: &str, password_hash: &str, now: i64) -> Result<UserRow> {
    self.create_user_capped(username, password_hash, now, None)
  }

  /// 创建用户，账号总数不得超过 `cap`（`None` = 不限）。计数与插入在**同一个写事务**里，
  /// 并发注册不会一起越过上限。
  ///
  /// # Errors
  ///
  /// [`StoreError::UsernameTaken`]（含大小写不同的同名）；[`StoreError::UserLimit`]：已满。
  pub fn create_user_capped(
    &self,
    username: &str,
    password_hash: &str,
    now: i64,
    cap: Option<usize>,
  ) -> Result<UserRow> {
    let created = self
      .with(|c| {
        let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        if let Some(cap) = cap {
          let count: i64 = tx.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))?;
          if count.cast_unsigned() >= cap as u64 {
            return Ok(None);
          }
        }
        tx.execute(
          "INSERT INTO users (username, password_hash, created_at, password_changed_at)
           VALUES (?1, ?2, ?3, ?3)",
          params![username, password_hash, now],
        )?;
        let id = tx.last_insert_rowid().cast_unsigned();
        tx.commit()?;
        Ok(Some(UserRow {
          id,
          username: username.to_owned(),
          password_hash: password_hash.to_owned(),
          created_at: now,
        }))
      })
      .map_err(|e| match e {
        StoreError::Db(ref db) if is_unique_violation(db) => StoreError::UsernameTaken,
        other => other,
      })?;
    created.ok_or(StoreError::UserLimit)
  }

  /// 按账号名查（不分大小写）。
  pub fn find_user_by_name(&self, username: &str) -> Result<Option<UserRow>> {
    self.with(|c| {
      c.query_row(
        &format!("SELECT {USER_COLUMNS} FROM users WHERE username = ?1"),
        [username],
        user_from_row,
      )
      .optional()
    })
  }

  /// 按 id 查（目前只有测试用它核对改口令的结果）。
  #[cfg(test)]
  pub fn find_user(&self, id: u64) -> Result<Option<UserRow>> {
    self.with(|c| {
      c.query_row(
        &format!("SELECT {USER_COLUMNS} FROM users WHERE id = ?1"),
        [id.cast_signed()],
        user_from_row,
      )
      .optional()
    })
  }

  /// 改口令哈希。
  pub fn set_password_hash(&self, id: u64, password_hash: &str, now: i64) -> Result<()> {
    self.with(|c| {
      c.execute(
        "UPDATE users SET password_hash = ?2, password_changed_at = ?3 WHERE id = ?1",
        params![id.cast_signed(), password_hash, now],
      )
      .map(|_| ())
    })
  }

  /// 注销账号：会话与云端备份随外键一并删除。
  pub fn delete_user(&self, id: u64) -> Result<()> {
    self.with(|c| {
      c.execute("DELETE FROM users WHERE id = ?1", [id.cast_signed()])
        .map(|_| ())
    })
  }

  // ───────────────────────── 会话 ─────────────────────────

  /// 新建会话；同一账号的会话数超过 [`MAX_SESSIONS_PER_USER`] 时淘汰最久没用的。
  pub fn create_session(
    &self,
    user_id: u64,
    token_hash: &[u8],
    now: i64,
    expires_at: i64,
    user_agent: Option<&str>,
  ) -> Result<()> {
    self.with(|c| {
      let tx = c.transaction()?;
      tx.execute(
        "INSERT INTO sessions (token_hash, user_id, created_at, last_seen_at, expires_at, user_agent)
         VALUES (?1, ?2, ?3, ?3, ?4, ?5)",
        params![token_hash, user_id.cast_signed(), now, expires_at, user_agent],
      )?;
      tx.execute(
        "DELETE FROM sessions WHERE user_id = ?1 AND token_hash NOT IN (
           SELECT token_hash FROM sessions WHERE user_id = ?1
           ORDER BY last_seen_at DESC, created_at DESC LIMIT ?2)",
        params![user_id.cast_signed(), MAX_SESSIONS_PER_USER as i64],
      )?;
      tx.commit()
    })
  }

  /// 凭令牌找到登录的用户；过期的当没有。顺带做滑动续期（至多每小时写一次），
  /// 续了期就在 [`SessionHit::renewed`] 里说明。
  pub fn find_session(
    &self,
    token_hash: &[u8],
    now: i64,
    ttl_secs: i64,
  ) -> Result<Option<SessionHit>> {
    self.with(|c| {
      let found = c
        .query_row(
          "SELECT u.id, u.username, u.password_hash, u.created_at, s.last_seen_at
           FROM sessions s JOIN users u ON u.id = s.user_id
           WHERE s.token_hash = ?1 AND s.expires_at > ?2",
          params![token_hash, now],
          |r| Ok((user_from_row(r)?, r.get::<_, i64>(4)?)),
        )
        .optional()?;
      let Some((user, last_seen)) = found else {
        return Ok(None);
      };
      let renewed = now - last_seen >= TOUCH_INTERVAL_SECS;
      if renewed {
        c.execute(
          "UPDATE sessions SET last_seen_at = ?2, expires_at = ?3 WHERE token_hash = ?1",
          params![token_hash, now, now + ttl_secs],
        )?;
      }
      Ok(Some(SessionHit { user, renewed }))
    })
  }

  /// 删除一个会话（退出登录）。令牌不存在也算成功。
  pub fn delete_session(&self, token_hash: &[u8]) -> Result<()> {
    self.with(|c| {
      c.execute("DELETE FROM sessions WHERE token_hash = ?1", [token_hash])
        .map(|_| ())
    })
  }

  /// 删除该账号除 `keep` 之外的所有会话（改口令后踢掉其他设备）。
  pub fn delete_other_sessions(&self, user_id: u64, keep: &[u8]) -> Result<()> {
    self.with(|c| {
      c.execute(
        "DELETE FROM sessions WHERE user_id = ?1 AND token_hash <> ?2",
        params![user_id.cast_signed(), keep],
      )
      .map(|_| ())
    })
  }

  /// 清掉过期会话，返回删掉的条数。
  pub fn purge_expired_sessions(&self, now: i64) -> Result<usize> {
    self.with(|c| c.execute("DELETE FROM sessions WHERE expires_at <= ?1", [now]))
  }

  // ───────────────────────── 云端备份 ─────────────────────────

  fn info_from_row(row: &rusqlite::Row<'_>, at: usize) -> rusqlite::Result<BackupInfo> {
    let fp: String = row.get(at)?;
    Ok(BackupInfo {
      fingerprint: parse_fingerprint(&fp, at)?,
      size: row.get::<_, i64>(at + 1)?.cast_unsigned(),
      updated_at: row.get(at + 2)?,
    })
  }

  /// 云端备份的概况（不读内容）。
  pub fn backup_info(&self, user_id: u64) -> Result<Option<BackupInfo>> {
    self.with(|c| {
      c.query_row(
        "SELECT fingerprint, size, updated_at FROM backups WHERE user_id = ?1",
        [user_id.cast_signed()],
        |r| Self::info_from_row(r, 0),
      )
      .optional()
    })
  }

  /// 云端备份的内容与概况。
  pub fn get_backup(&self, user_id: u64) -> Result<Option<(Vec<u8>, BackupInfo)>> {
    self.with(|c| {
      c.query_row(
        "SELECT fingerprint, size, updated_at, body FROM backups WHERE user_id = ?1",
        [user_id.cast_signed()],
        |r| Ok((r.get::<_, Vec<u8>>(3)?, Self::info_from_row(r, 0)?)),
      )
      .optional()
    })
  }

  /// 写入云端备份（整体替换）。条件检查与写入在**同一个写事务**里：
  /// 两台设备同时带着同一个基线来写，只有一个能成功，另一个拿到 `PreconditionFailed`。
  ///
  /// # Errors
  ///
  /// [`StoreError::PreconditionFailed`]：云端现状不满足 `pre`。
  pub fn put_backup(
    &self,
    user_id: u64,
    body: &[u8],
    fingerprint: Fingerprint,
    now: i64,
    pre: Precondition,
  ) -> Result<BackupInfo> {
    let uid = user_id.cast_signed();
    let size = body.len() as u64;
    let rejected = self.with(|c| {
      // IMMEDIATE：一开始就拿写锁，读到的「当前指纹」在提交前不会被别的写入改掉。
      let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
      let current: Option<String> = tx
        .query_row(
          "SELECT fingerprint FROM backups WHERE user_id = ?1",
          [uid],
          |r| r.get(0),
        )
        .optional()?;
      let current = current.map(|s| parse_fingerprint(&s, 0)).transpose()?;
      if !pre.holds(current) {
        return Ok(true);
      }
      tx.execute(
        "INSERT INTO backups (user_id, body, fingerprint, size, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(user_id) DO UPDATE SET
           body = excluded.body, fingerprint = excluded.fingerprint,
           size = excluded.size, updated_at = excluded.updated_at",
        params![
          uid,
          body,
          fingerprint.get().to_string(),
          size.cast_signed(),
          now
        ],
      )?;
      tx.commit()?;
      Ok(false)
    })?;
    if rejected {
      return Err(StoreError::PreconditionFailed);
    }
    Ok(BackupInfo {
      size,
      updated_at: now,
      fingerprint,
    })
  }

  /// 删除云端备份。没有也算成功。
  pub fn delete_backup(&self, user_id: u64) -> Result<()> {
    self.with(|c| {
      c.execute(
        "DELETE FROM backups WHERE user_id = ?1",
        [user_id.cast_signed()],
      )
      .map(|_| ())
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn store() -> Store {
    Store::open_in_memory().expect("open")
  }

  fn fp(n: u64) -> Fingerprint {
    Fingerprint::from_u64(n)
  }

  #[test]
  fn usernames_are_unique_regardless_of_case() {
    let s = store();
    let u = s.create_user("BG4XXX", "h", 1).unwrap();
    assert!(matches!(
      s.create_user("bg4xxx", "h2", 2),
      Err(StoreError::UsernameTaken)
    ));
    // 查找同样不分大小写，并保留注册时的写法。
    let found = s.find_user_by_name("bg4xxx").unwrap().unwrap();
    assert_eq!(found.id, u.id);
    assert_eq!(found.username, "BG4XXX");
    assert!(s.find_user_by_name("nobody").unwrap().is_none());
  }

  #[test]
  fn sessions_expire_and_slide() {
    let s = store();
    let u = s.create_user("alice", "h", 0).unwrap();
    s.create_session(u.id, b"tok", 0, 1000, Some("ua")).unwrap();
    let hit = s.find_session(b"tok", 10, 1000).unwrap().unwrap();
    assert_eq!(hit.user.id, u.id);
    // 刚登录不久：不续期，也就不用重发 Cookie。
    assert!(!hit.renewed);
    assert!(s.find_session(b"other", 10, 1000).unwrap().is_none());
    // 过期的当没有。
    assert!(s.find_session(b"tok", 1000, 1000).unwrap().is_none());
    // 超过一小时没写过：访问时续期（并告诉调用方去重发 Cookie），原来的过期点之后仍然有效。
    s.create_session(u.id, b"tok2", 0, 5000, None).unwrap();
    assert!(
      s.find_session(b"tok2", 4000, 5000)
        .unwrap()
        .unwrap()
        .renewed
    );
    assert!(s.find_session(b"tok2", 8000, 5000).unwrap().is_some());
    assert_eq!(s.purge_expired_sessions(100_000).unwrap(), 2);
  }

  #[test]
  fn the_user_count_is_capped_atomically() {
    let s = store();
    s.create_user_capped("alice", "h", 0, Some(2)).unwrap();
    s.create_user_capped("bob", "h", 0, Some(2)).unwrap();
    assert!(matches!(
      s.create_user_capped("carol", "h", 0, Some(2)),
      Err(StoreError::UserLimit)
    ));
    // 没满时同名仍然报「被占用」，而不是「已满」。
    assert!(matches!(
      s.create_user_capped("ALICE", "h", 0, Some(3)),
      Err(StoreError::UsernameTaken)
    ));
    // `None` 不限。
    s.create_user_capped("carol", "h", 0, None).unwrap();
  }

  #[cfg(unix)]
  #[test]
  fn a_new_database_file_is_private_to_its_owner() {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("ham-web-account-perm-{}", std::process::id()));
    let path = dir.join("accounts.db");
    let _ = std::fs::remove_dir_all(&dir);
    drop(Store::open(&path).unwrap());
    let mode = std::fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(
      mode & 0o777,
      0o600,
      "库里有口令哈希与全部备份，不能对其他用户可读"
    );
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn a_damaged_fingerprint_is_an_error_not_a_silent_zero() {
    let s = store();
    let u = s.create_user("alice", "h", 0).unwrap();
    s.put_backup(u.id, b"{}", fp(1), 0, Precondition::None)
      .unwrap();
    s.with(|c| {
      c.execute("UPDATE backups SET fingerprint = 'not a number'", [])
        .map(|_| ())
    })
    .unwrap();
    assert!(matches!(s.backup_info(u.id), Err(StoreError::Db(_))));
    assert!(matches!(
      s.put_backup(u.id, b"{}", fp(2), 1, Precondition::Absent),
      Err(StoreError::Db(_))
    ));
  }

  #[test]
  fn only_the_most_recent_sessions_are_kept() {
    let s = store();
    let u = s.create_user("alice", "h", 0).unwrap();
    for i in 0..(MAX_SESSIONS_PER_USER as i64 + 3) {
      s.create_session(u.id, format!("t{i}").as_bytes(), i, 10_000, None)
        .unwrap();
    }
    // 最旧的三个被淘汰，最新的都在。
    for i in 0..3 {
      assert!(
        s.find_session(format!("t{i}").as_bytes(), 50, 10_000)
          .unwrap()
          .is_none()
      );
    }
    assert!(
      s.find_session(
        format!("t{}", MAX_SESSIONS_PER_USER + 2).as_bytes(),
        50,
        10_000
      )
      .unwrap()
      .is_some()
    );
  }

  #[test]
  fn changing_password_can_keep_only_the_current_session() {
    let s = store();
    let u = s.create_user("alice", "old", 0).unwrap();
    for t in [&b"here"[..], b"phone", b"laptop"] {
      s.create_session(u.id, t, 0, 1000, None).unwrap();
    }
    s.set_password_hash(u.id, "new", 5).unwrap();
    s.delete_other_sessions(u.id, b"here").unwrap();
    assert_eq!(s.find_user(u.id).unwrap().unwrap().password_hash, "new");
    assert!(s.find_session(b"here", 1, 1000).unwrap().is_some());
    assert!(s.find_session(b"phone", 1, 1000).unwrap().is_none());
    assert!(s.find_session(b"laptop", 1, 1000).unwrap().is_none());
  }

  #[test]
  fn deleting_a_user_removes_their_sessions_and_backup_but_not_others() {
    let s = store();
    let a = s.create_user("alice", "h", 0).unwrap();
    let b = s.create_user("bob", "h", 0).unwrap();
    for u in [&a, &b] {
      s.create_session(u.id, u.username.as_bytes(), 0, 1000, None)
        .unwrap();
      s.put_backup(u.id, b"{}", fp(1), 0, Precondition::None)
        .unwrap();
    }
    s.delete_user(a.id).unwrap();
    assert!(s.find_user(a.id).unwrap().is_none());
    assert!(s.find_session(b"alice", 1, 1000).unwrap().is_none());
    assert!(s.backup_info(a.id).unwrap().is_none());
    // 外键级联只波及被删的账号。
    assert!(s.find_session(b"bob", 1, 1000).unwrap().is_some());
    assert!(s.backup_info(b.id).unwrap().is_some());
  }

  #[test]
  fn backups_are_replaced_whole_and_report_their_fingerprint() {
    let s = store();
    let u = s.create_user("alice", "h", 0).unwrap();
    assert!(s.get_backup(u.id).unwrap().is_none());
    let info = s
      .put_backup(u.id, br#"{"a":"1"}"#, fp(u64::MAX), 10, Precondition::None)
      .unwrap();
    assert_eq!((info.size, info.updated_at), (9, 10));
    // u64 最大值也能无损往返（库里是十进制文本）。
    let (body, got) = s.get_backup(u.id).unwrap().unwrap();
    assert_eq!(body, br#"{"a":"1"}"#);
    assert_eq!(got.fingerprint, fp(u64::MAX));
    s.put_backup(u.id, b"{}", fp(2), 20, Precondition::None)
      .unwrap();
    assert_eq!(s.get_backup(u.id).unwrap().unwrap().1.fingerprint, fp(2));
    s.delete_backup(u.id).unwrap();
    assert!(s.backup_info(u.id).unwrap().is_none());
  }

  #[test]
  fn conditional_writes_stop_a_stale_device_from_overwriting() {
    let s = store();
    let u = s.create_user("alice", "h", 0).unwrap();
    // 云端为空：只有「必须为空」的条件成立。
    assert!(matches!(
      s.put_backup(u.id, b"{}", fp(1), 1, Precondition::Matches(fp(0))),
      Err(StoreError::PreconditionFailed)
    ));
    s.put_backup(u.id, b"{}", fp(1), 1, Precondition::Absent)
      .unwrap();
    // 已有内容：「必须为空」不成立；基线对得上才能写。
    assert!(matches!(
      s.put_backup(u.id, b"{}", fp(9), 2, Precondition::Absent),
      Err(StoreError::PreconditionFailed)
    ));
    s.put_backup(u.id, b"{}", fp(2), 2, Precondition::Matches(fp(1)))
      .unwrap();
    // 另一台设备还停在旧基线上：被拦下，云端内容没被动。
    assert!(matches!(
      s.put_backup(
        u.id,
        b"{\"x\":\"y\"}",
        fp(3),
        3,
        Precondition::Matches(fp(1))
      ),
      Err(StoreError::PreconditionFailed)
    ));
    assert_eq!(s.get_backup(u.id).unwrap().unwrap().1.fingerprint, fp(2));
  }

  #[test]
  fn migrations_are_idempotent_and_the_file_survives_reopening() {
    let dir = std::env::temp_dir().join(format!("ham-web-account-{}", std::process::id()));
    let path = dir.join("nested/accounts.db");
    let _ = std::fs::remove_dir_all(&dir);
    {
      let s = Store::open(&path).unwrap();
      let u = s.create_user("alice", "h", 1).unwrap();
      s.put_backup(u.id, b"{}", fp(7), 1, Precondition::None)
        .unwrap();
    }
    // 重新打开（容器重建后挂回同一个卷就是这个场景）：数据还在，迁移不重复执行。
    let s = Store::open(&path).unwrap();
    let u = s.find_user_by_name("ALICE").unwrap().unwrap();
    assert_eq!(s.backup_info(u.id).unwrap().unwrap().fingerprint, fp(7));
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn a_database_from_a_newer_version_is_refused() {
    let conn = Connection::open_in_memory().unwrap();
    conn
      .execute_batch(&format!("PRAGMA user_version = {}", MIGRATIONS.len() + 1))
      .unwrap();
    assert!(Store::init(conn).is_err());
  }
}
