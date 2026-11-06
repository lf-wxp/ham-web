//! 账户数据库（SQLite）：用户、会话，以及账号名下的全部同步数据（多张表）。
//!
//! 全部是**同步**调用：`rusqlite` 的连接不是 `Sync`，这里用一把 `Mutex` 串行化；
//! 调用方（`handlers`）一律放进 `spawn_blocking`，不在异步运行时线程上等锁或等磁盘。
//! 单连接足够：写入量是「每个用户几十秒一次」，瓶颈是 Argon2 而不是 SQLite；
//! WAL 模式让读不阻塞写，进程崩溃后不会丢已提交的事务。
//!
//! 同步数据怎么拆成多张表、哪些值进专表哪些原样存放，见 [`super::relational`]；
//! 表结构的设计理由见 `docs/domain/account-sync.md`「数据结构」。

use std::path::Path;
use std::sync::Mutex;

use anyhow::Context;
use ham_web_core::account::{BackupInfo, BackupReceipt, Precondition, PushResponse, SyncManifest};
use ham_web_core::backup_merge::MAX_BACKUP_BYTES;
use ham_web_core::delta::{Manifest, manifest_fingerprint};
use ham_web_core::remote_sync::Fingerprint;
use rusqlite::{Connection, OptionalExtension, ffi, params};

use super::relational::{self, Delta, DeltaOutcome, Split};

/// 按顺序执行的迁移；第 `n` 条把 `user_version` 从 `n` 升到 `n + 1`。
///
/// 现在只有一条：账号与会话（`schema/accounts.sql`）加上账号名下的全部同步数据（`schema/content.sql`）。
/// 这是初版结构——此前没有任何线上数据，所以没有背着旧结构的迁移链。**从这里起只追加、不改写**：
/// 一旦有库在线上跑起来，`user_version` 决定从哪一条继续，改写已发布的条目会让新旧库长得不一样。
/// 开发期间改了结构，删掉本地的库文件重建即可。程序会在启动时拒绝两种不匹配的库：版本号比它认识的新，
/// 或版本号对得上、结构却不对（压成一条之前的初版也是 `user_version = 1`，见 [`REQUIRED_TABLES`]）。
const MIGRATIONS: &[&str] = &[concat!(
  include_str!("schema/accounts.sql"),
  "\n",
  include_str!("schema/content.sql")
)];

/// 同一账号最多保留的会话数：超出时淘汰最久没用的，免得登录接口被刷成无限增长的表。
pub const MAX_SESSIONS_PER_USER: usize = 10;

/// 一个账号在全部专表里最多多少行。
///
/// 体积上限挡不住行数：`question-stats` / `study-stats:seen` 这类每行几十字节的 key，
/// 64 MiB 能展开成上百万行，逐行 INSERT 会把全局写锁占上十几秒（一次上传就能让所有人排队）。
/// 40 万行对真实数据（几万条通联 + 几十万条统计）是够用的余量。
const MAX_ROWS_PER_USER: i64 = 400_000;

/// 刷新「最近使用时间」的最小间隔（秒）：避免每个请求都写一次库。
const TOUCH_INTERVAL_SECS: i64 = 3600;

/// 只读连接的个数：读取是「几十 KB 的清单」与「偶尔几十 MB 的整份下载」，四个够让大下载不挡小请求。
const READERS: usize = 4;

/// 每个版本的库里必须存在的表（只列启动自检用得上的骨架：账号、会话、概况、目录）。
const REQUIRED_TABLES: &[&str] = &["users", "sessions", "backup_meta", "user_kv"];

/// 库里缺少的 [`REQUIRED_TABLES`]。
fn missing_tables(conn: &Connection) -> rusqlite::Result<Vec<&'static str>> {
  let mut missing = Vec::new();
  for table in REQUIRED_TABLES {
    let found: i64 = conn.query_row(
      "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
      [table],
      |r| r.get(0),
    )?;
    if found == 0 {
      missing.push(*table);
    }
  }
  Ok(missing)
}

/// 打开只读连接（WAL 下与写连接互不阻塞）。
fn open_reader(path: &Path) -> rusqlite::Result<Connection> {
  use rusqlite::OpenFlags;
  let conn = Connection::open_with_flags(
    path,
    OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
  )?;
  // 别的连接正在写时等一会儿，而不是立刻报错。
  conn.busy_timeout(std::time::Duration::from_millis(5000))?;
  Ok(conn)
}

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
  /// 库里的内容拼不回一份完整的备份（被改坏，或本程序不认识的存放方式）。
  Inconsistent,
  /// 写入之后账号的内容会超过体积上限。
  TooLarge,
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
  /// 只读连接池（懒打开）：整份读取（`get_backup` / `pull_keys` / 清单）走它，不占写锁。
  ///
  /// WAL 下读写互不阻塞，所以把「读几十 MB 的备份」放在写连接上毫无必要：那会让这次下载期间
  /// 每一次会话校验都排队。内存库（测试）没有第二个连接可开，见 [`Store::with_read`]。
  readers: [Mutex<Option<Connection>>; READERS],
  /// 全忙时轮转挑哪一个排队。
  next_reader: std::sync::atomic::AtomicUsize,
  /// 库文件路径；内存库为 `None`。
  path: Option<std::path::PathBuf>,
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

/// 云端现在的指纹；还没有备份为 `None`。
fn current_fingerprint(
  tx: &rusqlite::Transaction<'_>,
  uid: i64,
) -> rusqlite::Result<Option<Fingerprint>> {
  let current: Option<String> = tx
    .query_row(
      "SELECT fingerprint FROM backup_meta WHERE user_id = ?1",
      [uid],
      |r| r.get(0),
    )
    .optional()?;
  current.map(|s| parse_fingerprint(&s, 0)).transpose()
}

/// 写入之后更新概况：指纹与体积都从目录里算（对规范内容），不依赖调用方传。
///
/// 清单只读一遍：指纹由它推出，回执也要带它。
fn record_meta(
  tx: &rusqlite::Transaction<'_>,
  uid: i64,
  now: i64,
) -> rusqlite::Result<(BackupInfo, Manifest)> {
  let keys = relational::manifest(tx, uid)?;
  let fingerprint = manifest_fingerprint(&keys);
  let size = relational::total_bytes(tx, uid)?;
  tx.execute(
    "INSERT INTO backup_meta (user_id, fingerprint, size, updated_at)
     VALUES (?1, ?2, ?3, ?4)
     ON CONFLICT(user_id) DO UPDATE SET
       fingerprint = excluded.fingerprint, size = excluded.size,
       updated_at = excluded.updated_at",
    params![uid, fingerprint.get().to_string(), size.cast_signed(), now],
  )?;
  let info = BackupInfo {
    size,
    updated_at: now,
    fingerprint,
  };
  Ok((info, keys))
}

/// 创建库文件（已存在则不动内容），并且始终只给属主读写：里面有口令哈希与全部用户的备份。
/// SQLite 为库创建的 `-wal` / `-shm` 沿用主文件的权限。
///
/// `mode` 只在**创建**时生效，所以已存在的文件要单独改权限：从别处复制、旧版本创建、
/// 数据卷挂载过来的库可能是 0644，别的本地用户可以读到全部内容和口令哈希。
fn ensure_private_file(path: &Path) -> std::io::Result<()> {
  let mut opts = std::fs::OpenOptions::new();
  // append：不截断已有内容。
  opts.create(true).append(true);
  #[cfg(unix)]
  {
    use std::os::unix::fs::OpenOptionsExt;
    opts.mode(0o600);
  }
  let file = opts.open(path)?;
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    // 只在权限确实过宽时改：权限本来就对的库（含别人挂载的只读卷）不会碰，所以不会因此启动失败。
    // 权限过宽而又改不了（文件不属于本进程）时**会**启动失败，这是有意的：里面有口令哈希，
    // 宁可让运维看到错误去修，也不带着一个别人可读的库悄悄跑起来。
    if file.metadata()?.permissions().mode() & 0o077 != 0 {
      file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
  }
  drop(file);
  Ok(())
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
    let mut store = Self::init(conn)?;
    store.path = Some(path.to_path_buf());
    Ok(store)
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

  pub(super) fn init(mut conn: Connection) -> anyhow::Result<Self> {
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
    // 版本号对得上不代表结构对得上：迁移链压成一条之前，`user_version = 1` 的库是带 `backups` 表的旧结构，
    // 版本检查会放行它，服务照常启动、登录照常成功，之后每个同步接口才报「no such table」——
    // 比启动失败难查得多。所以已有版本的库必须真有本程序要用的表，否则当场拒绝并说清怎么办。
    if version > 0 {
      let missing = missing_tables(&conn)?;
      anyhow::ensure!(
        missing.is_empty(),
        "账户数据库的结构与本程序不符（缺少表：{}）：它是旧版本留下的初版结构，本程序不做升级。\
         没有需要保留的数据时，停掉服务、删除数据库文件（含 -wal / -shm）后重启即可重建",
        missing.join("、")
      );
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(version) {
      let tx = conn.transaction()?;
      tx.execute_batch(sql)?;
      // `user_version` 不接受绑定参数，只能拼；`i` 是循环下标，不是外部输入。
      tx.execute_batch(&format!("PRAGMA user_version = {}", i + 1))?;
      tx.commit()?;
    }
    Ok(Self {
      conn: Mutex::new(conn),
      readers: std::array::from_fn(|_| Mutex::new(None)),
      next_reader: std::sync::atomic::AtomicUsize::new(0),
      path: None,
    })
  }

  pub(super) fn with<R>(
    &self,
    f: impl FnOnce(&mut Connection) -> rusqlite::Result<R>,
  ) -> Result<R> {
    // 持锁时 panic 会毒化锁；库本身没有被破坏（事务会回滚），继续用比让之后每个请求都 panic 好。
    let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut conn).map_err(StoreError::from)
  }

  /// 纯读取：走只读连接，不占写锁。
  ///
  /// **整次读取在一个读事务里**：`get_backup` / `sync_manifest` / `pull_keys` 都是多条语句，
  /// WAL 下自动提交的每条语句各取一个快照，中间夹一次别人的提交就会读到「目录是旧的、专表是新的」
  /// （表现为 `Inconsistent` 或指纹与内容对不上）。事务让它们共用同一个快照。
  ///
  /// 读连接是一个小池（[`READERS`] 个）：一次几十 MB 的下载占着一个连接，不挡别人的清单与取值。
  /// 内存库没有第二个连接（另开一个会是空库），退回写连接——同样包进事务，行为一样，只是没有并发收益。
  pub(super) fn with_read<R>(
    &self,
    f: impl FnOnce(&Connection) -> rusqlite::Result<R>,
  ) -> Result<R> {
    let Some(path) = &self.path else {
      return self.with(|c| {
        let tx = c.transaction()?;
        f(&tx)
      });
    };
    let slot = self.checkout_reader();
    let mut guard = slot.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_none() {
      *guard = Some(open_reader(path).map_err(StoreError::from)?);
    }
    let conn = guard.as_mut().ok_or(StoreError::Inconsistent)?;
    // DEFERRED：第一条 SELECT 才拿快照；`f` 返回后事务随 `tx` 丢弃回滚（只读，没有东西要提交）。
    let tx = conn.transaction().map_err(StoreError::from)?;
    f(&tx).map_err(StoreError::from)
  }

  /// 挑一个读连接的槽：先找当下没人用的，全忙就按轮转排到某一个后面。
  fn checkout_reader(&self) -> &Mutex<Option<Connection>> {
    for slot in &self.readers {
      if let Ok(guard) = slot.try_lock() {
        drop(guard);
        return slot;
      }
    }
    let next = self
      .next_reader
      .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    &self.readers[next % READERS]
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
  /// 改口令并踢掉其它设备的会话，**同一个事务**。
  ///
  /// 分两条自动提交语句做的话，第二条失败就是「口令已改、旧会话仍然有效」——
  /// 「改口令会退出其它设备」这个承诺正好在最需要它的时候失效。
  pub fn set_password_and_drop_sessions(
    &self,
    id: u64,
    password_hash: &str,
    now: i64,
    keep: &[u8],
  ) -> Result<()> {
    self.with(|c| {
      let tx = c.transaction()?;
      tx.execute(
        "UPDATE users SET password_hash = ?2, password_changed_at = ?3 WHERE id = ?1",
        params![id.cast_signed(), password_hash, now],
      )?;
      tx.execute(
        "DELETE FROM sessions WHERE user_id = ?1 AND token_hash <> ?2",
        params![id.cast_signed(), keep],
      )?;
      tx.commit()
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
    self.with_read(|c| {
      c.query_row(
        "SELECT fingerprint, size, updated_at FROM backup_meta WHERE user_id = ?1",
        [user_id.cast_signed()],
        |r| Self::info_from_row(r, 0),
      )
      .optional()
    })
  }

  /// 云端备份的内容与概况：从各表读出来，拼回与「导出备份」同格式的 JSON。
  ///
  /// 读库走只读连接（WAL 下与写互不阻塞），拼接与序列化在锁外：
  /// 一次大备份的下载不会让别人的登录校验排队。
  ///
  /// # Errors
  ///
  /// [`StoreError::Inconsistent`]：库里的内容拼不回完整备份。
  pub fn get_backup(&self, user_id: u64) -> Result<Option<(Vec<u8>, BackupInfo)>> {
    let uid = user_id.cast_signed();
    let loaded = self.with_read(|c| {
      let Some(info) = c
        .query_row(
          "SELECT fingerprint, size, updated_at FROM backup_meta WHERE user_id = ?1",
          [uid],
          |r| Self::info_from_row(r, 0),
        )
        .optional()?
      else {
        return Ok(None);
      };
      Ok(Some((info, relational::load(c, uid)?)))
    })?;
    let Some((info, split)) = loaded else {
      return Ok(None);
    };
    let map = split.into_map().ok_or(StoreError::Inconsistent)?;
    let body = serde_json::to_vec(&map).map_err(|_| StoreError::Inconsistent)?;
    Ok(Some((body, info)))
  }

  /// 写入云端备份（整体替换）。条件检查与写入在**同一个写事务**里：
  /// 两台设备同时带着同一个基线来写，只有一个能成功，另一个拿到 `PreconditionFailed`。
  ///
  /// `split` 是已经拆好的内容（解析与算摘要是 CPU 重活，调用方放在锁外做）。库里的内容会变成
  /// **恰好**等于 `split`：新备份里没有的 key 被删，没变的 key 不重写。写完之后的指纹与体积
  /// 从目录里算（对规范内容），所以返回的就是前端该当作基线的那个值。
  ///
  /// # Errors
  ///
  /// [`StoreError::PreconditionFailed`]：云端现状不满足 `pre`；
  /// [`StoreError::TooLarge`]：规范化之后的内容超过体积上限（请求体没超限不代表它没超限：
  /// 规范化会补齐缺省字段，比原文大得多也不奇怪）。
  pub fn put_backup(
    &self,
    user_id: u64,
    split: &Split,
    now: i64,
    pre: Precondition,
  ) -> Result<BackupReceipt> {
    self.put_backup_within(
      user_id,
      split,
      now,
      pre,
      MAX_BACKUP_BYTES as u64,
      MAX_ROWS_PER_USER,
    )
  }

  /// 同 [`Store::put_backup`]，上限可调：测试用小值，免得为了触顶去造 64 MiB 的数据。
  fn put_backup_within(
    &self,
    user_id: u64,
    split: &Split,
    now: i64,
    pre: Precondition,
    max_bytes: u64,
    max_rows: i64,
  ) -> Result<BackupReceipt> {
    if split.total_bytes() > max_bytes {
      return Err(StoreError::TooLarge);
    }
    let uid = user_id.cast_signed();
    let done = self.with(|c| {
      // IMMEDIATE：一开始就拿写锁，读到的「当前指纹」在提交前不会被别的写入改掉。
      let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
      let current = current_fingerprint(&tx, uid)?;
      if !pre.holds(current) {
        return Ok(None);
      }
      relational::apply(&tx, uid, split)?;
      // 行数在事务里核对；超了就不提交（等于一行没写），而不是留一份走到一半的库。
      if relational::row_count(&tx, uid)? > max_rows {
        return Ok(Some(Err(())));
      }
      let (info, keys) = record_meta(&tx, uid, now)?;
      tx.commit()?;
      Ok(Some(Ok(BackupReceipt { info, keys })))
    })?;
    match done {
      Some(Ok(receipt)) => Ok(receipt),
      Some(Err(())) => Err(StoreError::TooLarge),
      None => Err(StoreError::PreconditionFailed),
    }
  }

  /// 云端的清单：全部同步 key 的摘要。还没有备份为 `None`。
  pub fn sync_manifest(&self, user_id: u64) -> Result<Option<SyncManifest>> {
    let uid = user_id.cast_signed();
    self.with_read(|c| {
      let Some(info) = c
        .query_row(
          "SELECT fingerprint, size, updated_at FROM backup_meta WHERE user_id = ?1",
          [uid],
          |r| Self::info_from_row(r, 0),
        )
        .optional()?
      else {
        return Ok(None);
      };
      Ok(Some(SyncManifest {
        fingerprint: info.fingerprint,
        updated_at: info.updated_at,
        size: info.size,
        keys: relational::manifest(c, uid)?,
      }))
    })
  }

  /// 取指定 key 的值（规范形态）；云端已经没有的不在结果里。
  ///
  /// # Errors
  ///
  /// [`StoreError::Inconsistent`]：库里的内容拼不回。
  pub fn pull_keys(
    &self,
    user_id: u64,
    keys: &[String],
  ) -> Result<std::collections::BTreeMap<String, String>> {
    let uid = user_id.cast_signed();
    let split = self.with_read(|c| relational::load_some(c, uid, keys))?;
    split.into_map().ok_or(StoreError::Inconsistent)
  }

  /// 按 key 增量写入。每个 key 带着「调用方看到的摘要」，云端现在的摘要与它对不上就整批不生效。
  ///
  /// # Errors
  ///
  /// - [`StoreError::PreconditionFailed`]：有 key 在云端已经不是调用方看到的那一版；
  /// - [`StoreError::TooLarge`]：写完之后账号的内容会超过上限。
  pub fn push_delta(&self, user_id: u64, delta: &Delta, now: i64) -> Result<PushResponse> {
    let uid = user_id.cast_signed();
    let done = self.with(|c| {
      let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
      match relational::apply_delta(&tx, uid, delta, MAX_BACKUP_BYTES as u64, MAX_ROWS_PER_USER)? {
        DeltaOutcome::Applied => {}
        refused => return Ok(Err(refused)),
      }
      let (info, keys) = record_meta(&tx, uid, now)?;
      tx.commit()?;
      Ok(Ok((info, keys)))
    })?;
    match done {
      Ok((info, keys)) => Ok(PushResponse { info, keys }),
      Err(DeltaOutcome::TooLarge) => Err(StoreError::TooLarge),
      Err(_) => Err(StoreError::PreconditionFailed),
    }
  }

  /// 删除云端备份（各表里这个账号的全部内容）。没有也算成功。
  pub fn delete_backup(&self, user_id: u64) -> Result<()> {
    let uid = user_id.cast_signed();
    self.with(|c| {
      let tx = c.transaction()?;
      relational::clear_all(&tx, uid)?;
      tx.execute("DELETE FROM backup_meta WHERE user_id = ?1", [uid])?;
      tx.commit()
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn store() -> Store {
    Store::open_in_memory().expect("open")
  }

  impl Store {
    /// 测试里直接拿 JSON 文本当备份写。指纹由内容决定（对云端的 key 清单求），不能由测试指定。
    fn put_json(
      &self,
      user_id: u64,
      body: &[u8],
      now: i64,
      pre: Precondition,
    ) -> Result<BackupInfo> {
      let map = serde_json::from_slice(body).expect("测试给的备份必须是 {key: value}");
      self
        .put_backup(user_id, &Split::from_map(map), now, pre)
        .map(|receipt| receipt.info)
    }
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

  #[cfg(unix)]
  #[test]
  fn an_existing_loose_file_is_tightened_on_open() {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("ham-web-account-loose-{}", std::process::id()));
    let path = dir.join("accounts.db");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // 从别处复制 / 旧版本 / 数据卷挂载过来的库：0644。
    std::fs::write(&path, b"").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

    drop(Store::open(&path).unwrap());
    let mode = std::fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(
      mode & 0o777,
      0o600,
      "已存在的库文件也要收紧：里面有口令哈希与全部备份"
    );
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn a_backup_with_too_many_rows_is_refused_whole() {
    let s = store();
    let u = s.create_user("alice", "h", 0).unwrap();
    s.put_json(u.id, br#"{"a":"1"}"#, 1, Precondition::None)
      .unwrap();

    // `bookmarks` 是一个数组，元素在表里一行一个：三条就是三行。
    let split = Split::from_map([("bookmarks".to_owned(), r#"["a","b","c"]"#.to_owned())].into());
    assert!(matches!(
      s.put_backup_within(u.id, &split, 2, Precondition::None, u64::MAX, 2),
      Err(StoreError::TooLarge)
    ));
    // 被拒的写入什么都没动。
    let (body, _) = s.get_backup(u.id).unwrap().unwrap();
    assert_eq!(body, br#"{"a":"1"}"#);

    // 放宽到刚好三行：收。
    s.put_backup_within(u.id, &split, 3, Precondition::None, u64::MAX, 3)
      .unwrap();
    let values = s.pull_keys(u.id, &["bookmarks".to_owned()]).unwrap();
    assert_eq!(values["bookmarks"], r#"["a","b","c"]"#);
  }

  #[test]
  fn a_damaged_fingerprint_is_an_error_not_a_silent_zero() {
    let s = store();
    let u = s.create_user("alice", "h", 0).unwrap();
    s.put_json(u.id, b"{}", 0, Precondition::None).unwrap();
    s.with(|c| {
      c.execute("UPDATE backup_meta SET fingerprint = 'not a number'", [])
        .map(|_| ())
    })
    .unwrap();
    assert!(matches!(s.backup_info(u.id), Err(StoreError::Db(_))));
    assert!(matches!(
      s.put_json(u.id, b"{}", 1, Precondition::Absent),
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
    s.set_password_and_drop_sessions(u.id, "new", 5, b"here")
      .unwrap();
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
      s.put_json(u.id, b"{}", 0, Precondition::None).unwrap();
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
  fn backups_are_replaced_whole_and_report_a_fingerprint_that_follows_the_content() {
    let s = store();
    let u = s.create_user("alice", "h", 0).unwrap();
    assert!(s.get_backup(u.id).unwrap().is_none());
    let info = s
      .put_json(u.id, br#"{"a":"1"}"#, 10, Precondition::None)
      .unwrap();
    assert_eq!(
      (info.size, info.updated_at),
      (1, 10),
      "体积是规范文本的字节数之和"
    );
    let (body, got) = s.get_backup(u.id).unwrap().unwrap();
    assert_eq!(body, br#"{"a":"1"}"#);
    assert_eq!(got, info);
    // 内容变了指纹就变，内容回到原样指纹也回到原样（与写入次序无关）。
    let other = s.put_json(u.id, b"{}", 20, Precondition::None).unwrap();
    assert_ne!(other.fingerprint, info.fingerprint);
    assert_eq!(
      s.get_backup(u.id).unwrap().unwrap().1.fingerprint,
      other.fingerprint
    );
    let again = s
      .put_json(u.id, br#"{"a":"1"}"#, 30, Precondition::None)
      .unwrap();
    assert_eq!(again.fingerprint, info.fingerprint);
    s.delete_backup(u.id).unwrap();
    assert!(s.backup_info(u.id).unwrap().is_none());
  }

  #[test]
  fn conditional_writes_stop_a_stale_device_from_overwriting() {
    let s = store();
    let u = s.create_user("alice", "h", 0).unwrap();
    // 云端为空：只有「必须为空」的条件成立。
    assert!(matches!(
      s.put_json(
        u.id,
        b"{}",
        1,
        Precondition::Matches(Fingerprint::from_u64(0))
      ),
      Err(StoreError::PreconditionFailed)
    ));
    let v1 = s
      .put_json(u.id, br#"{"k":"1"}"#, 1, Precondition::Absent)
      .unwrap();
    // 已有内容：「必须为空」不成立；基线对得上才能写。
    assert!(matches!(
      s.put_json(u.id, b"{}", 2, Precondition::Absent),
      Err(StoreError::PreconditionFailed)
    ));
    let v2 = s
      .put_json(
        u.id,
        br#"{"k":"2"}"#,
        2,
        Precondition::Matches(v1.fingerprint),
      )
      .unwrap();
    // 另一台设备还停在旧基线上：被拦下，云端内容没被动。
    assert!(matches!(
      s.put_json(
        u.id,
        b"{\"x\":\"y\"}",
        3,
        Precondition::Matches(v1.fingerprint)
      ),
      Err(StoreError::PreconditionFailed)
    ));
    assert_eq!(
      s.get_backup(u.id).unwrap().unwrap().1.fingerprint,
      v2.fingerprint
    );
  }

  #[test]
  fn migrations_are_idempotent_and_the_file_survives_reopening() {
    let dir = std::env::temp_dir().join(format!("ham-web-account-{}", std::process::id()));
    let path = dir.join("nested/accounts.db");
    let _ = std::fs::remove_dir_all(&dir);
    {
      let s = Store::open(&path).unwrap();
      let u = s.create_user("alice", "h", 1).unwrap();
      s.put_json(u.id, br#"{"a":"1"}"#, 1, Precondition::None)
        .unwrap();
    }
    // 重新打开（容器重建后挂回同一个卷就是这个场景）：数据还在，迁移不重复执行。
    let s = Store::open(&path).unwrap();
    let u = s.find_user_by_name("ALICE").unwrap().unwrap();
    let info = s.backup_info(u.id).unwrap().unwrap();
    assert_eq!(s.get_backup(u.id).unwrap().unwrap().1, info);
    assert_eq!(info.size, 1);
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

  #[test]
  fn the_pre_squash_schema_at_version_one_is_refused_with_a_clear_reason() {
    // 迁移链压成一条之前的初版：`user_version = 1`、只有 `users` / `sessions` / `backups`。
    // 版本号一样，结构不一样：不拒绝的话服务正常启动，之后每个同步接口才报 500。
    let conn = Connection::open_in_memory().unwrap();
    conn
      .execute_batch(
        "CREATE TABLE users (id INTEGER PRIMARY KEY);
         CREATE TABLE sessions (token_hash BLOB PRIMARY KEY);
         CREATE TABLE backups (user_id INTEGER PRIMARY KEY, body BLOB);
         PRAGMA user_version = 1;",
      )
      .unwrap();
    let err = Store::init(conn)
      .err()
      .expect("旧结构必须被拒绝")
      .to_string();
    assert!(
      err.contains("backup_meta") && err.contains("user_kv"),
      "{err}"
    );
    assert!(err.contains("删除数据库文件"), "要告诉运维怎么办：{err}");
  }

  #[test]
  fn a_whole_read_never_sees_half_of_a_concurrent_commit() {
    // 一个线程不停地写（每次目录里的摘要与专表里的行一起变），另一个线程不停地整份读。
    // 没有读事务时，读取的各条语句各取一个快照，会读到「目录是新的、专表是旧的」：
    // 要么 `Inconsistent`，要么 `get_backup` 回来的内容与它自己回来的指纹对不上。
    let dir = std::env::temp_dir().join(format!("ham-web-account-snap-{}", std::process::id()));
    let path = dir.join("accounts.db");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let s = std::sync::Arc::new(Store::open(&path).unwrap());
    let u = s.create_user("alice", "h", 0).unwrap();
    let book = |n: usize| {
      let items: Vec<String> = (0..n).map(|i| format!("q{i}")).collect();
      serde_json::to_string(&items).unwrap()
    };
    s.put_json(
      u.id,
      format!(
        r#"{{"bookmarks":{}}}"#,
        serde_json::to_string(&book(1)).unwrap()
      )
      .as_bytes(),
      1,
      Precondition::None,
    )
    .unwrap();

    let writer = {
      let s = s.clone();
      std::thread::spawn(move || {
        for n in 2..200 {
          let body = format!(
            r#"{{"bookmarks":{}}}"#,
            serde_json::to_string(&book(n)).unwrap()
          );
          s.put_json(u.id, body.as_bytes(), n as i64, Precondition::None)
            .unwrap();
        }
      })
    };
    let mut reads = 0usize;
    while !writer.is_finished() {
      let (body, info) = s.get_backup(u.id).expect("读取不能撕裂").unwrap();
      let map: std::collections::BTreeMap<String, String> = serde_json::from_slice(&body).unwrap();
      // 内容与概况来自同一个快照：按内容重新算出的指纹必须等于概况里的。
      let keys = ham_web_core::delta::manifest_of(&map, true);
      assert_eq!(
        ham_web_core::delta::manifest_fingerprint(&keys),
        info.fingerprint,
        "读到了半个提交"
      );
      reads += 1;
    }
    writer.join().unwrap();
    assert!(reads > 0);
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn a_big_read_in_progress_does_not_block_other_reads() {
    // 读连接是池：占住其中一个（模拟一次大下载），别的读取仍能完成。
    let dir = std::env::temp_dir().join(format!("ham-web-account-pool-{}", std::process::id()));
    let path = dir.join("accounts.db");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let s = std::sync::Arc::new(Store::open(&path).unwrap());
    let u = s.create_user("alice", "h", 0).unwrap();
    s.put_json(u.id, br#"{"a":"1"}"#, 1, Precondition::None)
      .unwrap();

    let (held_tx, held_rx) = std::sync::mpsc::channel::<()>();
    let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
    let big = {
      let s = s.clone();
      std::thread::spawn(move || {
        s.with_read(|_| {
          held_tx.send(()).unwrap();
          release_rx.recv().unwrap();
          Ok(())
        })
        .unwrap();
      })
    };
    held_rx.recv().unwrap();
    // 另一个读取在大读取还没结束时完成。
    assert!(s.sync_manifest(u.id).unwrap().is_some());
    release_tx.send(()).unwrap();
    big.join().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn a_database_this_program_created_passes_the_structure_check_on_reopen() {
    // 自检不能误伤正常的库：新建、关闭、再打开。
    let dir = std::env::temp_dir().join(format!("ham-web-account-reopen-{}", std::process::id()));
    let path = dir.join("accounts.db");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    drop(Store::open(&path).unwrap());
    assert!(Store::open(&path).is_ok());
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn a_backup_over_the_cap_once_normalized_is_refused_and_the_old_one_stays() {
    let s = store();
    let u = s.create_user("alice", "h", 0).unwrap();
    s.put_json(u.id, br#"{"a":"1234"}"#, 1, Precondition::None)
      .unwrap();

    // 原文只有 14 字节，规范化补齐缺省字段之后更大：按规范文本算，上限 14 就放不下。
    let raw = r#"{"answered":3}"#;
    let split = Split::from_map([("study-stats".to_owned(), raw.to_owned())].into());
    assert!(split.total_bytes() > raw.len() as u64);
    let cap = raw.len() as u64;
    assert!(matches!(
      s.put_backup_within(u.id, &split, 2, Precondition::None, cap, MAX_ROWS_PER_USER),
      Err(StoreError::TooLarge)
    ));
    // 被拒的写入什么都没动：旧内容与旧概况都在。
    let (body, info) = s.get_backup(u.id).unwrap().unwrap();
    assert_eq!(body, br#"{"a":"1234"}"#);
    assert_eq!((info.size, info.updated_at), (4, 1));

    // 上限放宽到刚好放得下：收。
    let fits = split.total_bytes();
    s.put_backup_within(u.id, &split, 3, Precondition::None, fits, MAX_ROWS_PER_USER)
      .unwrap();
    assert_eq!(s.backup_info(u.id).unwrap().unwrap().size, fits);
  }
}
