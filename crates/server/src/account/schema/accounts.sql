-- 账号与会话。

CREATE TABLE users (
  id                  INTEGER PRIMARY KEY AUTOINCREMENT,
  -- NOCASE：`BG4XXX` 与 `bg4xxx` 是同一个账号；账号名只收 ASCII（见 core::account）。
  username            TEXT    NOT NULL UNIQUE COLLATE NOCASE,
  password_hash       TEXT    NOT NULL,          -- Argon2id 的 PHC 字符串，盐与参数都在里面
  created_at          INTEGER NOT NULL,          -- Unix 秒
  password_changed_at INTEGER NOT NULL
) STRICT;

CREATE TABLE sessions (
  -- 只存令牌的 SHA-256：库文件泄露（备份、误传）时拿不到可用的登录态。
  token_hash   BLOB    PRIMARY KEY,
  user_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  created_at   INTEGER NOT NULL,
  last_seen_at INTEGER NOT NULL,
  expires_at   INTEGER NOT NULL,
  user_agent   TEXT
) STRICT;
CREATE INDEX sessions_user    ON sessions(user_id);
CREATE INDEX sessions_expires ON sessions(expires_at);
