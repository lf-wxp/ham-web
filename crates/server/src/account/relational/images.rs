//! `qsl-image:<通联编号>`：QSL 卡片影像。
//!
//! 值在前端是 `data:<类型>;base64,<数据>` 文本。库里把它拆成类型 + 二进制（`BLOB`）：
//! 省掉 base64 的 1/3 体积，也让影像能被当成真正的文件取出来。
//!
//! 入表前核对：key 的编号合规、类型在白名单内、数据是合法的 base64、长度不超过 `MAX_STORED_CHARS`。
//! **写法上的差异会被规范化**，不会因此拒收：
//! 缺 padding 的 base64 照样解得出来，取回时补齐；`image/jpg`、大小写不同的类型名统一成 `image/jpeg` 等
//! 规范写法（表里的 `CHECK` 只认规范写法，所以这里必须先规范化，否则这样的备份会在落库时才失败）。

use base64::Engine;
use base64::alphabet::STANDARD as ALPHABET;
use base64::engine::general_purpose::STANDARD;
use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};
use ham_web_core::qsl_image::{
  KEY_PREFIX, MAX_STORED_CHARS, entry_id_from_key, is_supported_mime, key_for,
};
use rusqlite::{Connection, Transaction, params};

use super::{Key, Table, corrupt};

/// 解码时有没有 padding 都认；编码一律补齐（见 [`STANDARD`]）。
const LENIENT: GeneralPurpose = GeneralPurpose::new(
  &ALPHABET,
  GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

/// 白名单里的类型名 → 表里存的规范写法。
fn canonical_mime(mime: &str) -> Option<&'static str> {
  if !is_supported_mime(mime) {
    return None;
  }
  Some(match mime.trim().to_ascii_lowercase().as_str() {
    "image/png" => "image/png",
    "image/webp" => "image/webp",
    // `image/jpeg` 与 `image/jpg`。
    _ => "image/jpeg",
  })
}

pub(super) struct QslImages {
  /// 所属通联的编号（`LogEntry.id`）。
  qso_id: u64,
  mime: String,
  data: Vec<u8>,
}

impl Table for QslImages {
  const KEY: Key = Key::Prefix(KEY_PREFIX);
  const TABLES: &'static [&'static str] = &["qsl_images"];

  fn decode(key: &str, raw: &str) -> Option<Self> {
    let qso_id = entry_id_from_key(key)?;
    // 超过客户端导入时也会拒收的上限，不是本站写出的数据：不进影像表，原样留在目录里
    // （仍计入账号的总量上限），免得单张影像就占满整个配额。
    if raw.len() > MAX_STORED_CHARS {
      return None;
    }
    let (mime, b64) = raw.strip_prefix("data:")?.split_once(";base64,")?;
    let mime = canonical_mime(mime)?;
    let data = LENIENT.decode(b64).ok()?;
    Some(Self {
      qso_id,
      mime: mime.to_owned(),
      data,
    })
  }

  fn encode(&self) -> Option<String> {
    Some(format!(
      "data:{};base64,{}",
      self.mime,
      STANDARD.encode(&self.data)
    ))
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    tx.execute(
      "INSERT INTO qsl_images (user_id, qso_id, mime, data) VALUES (?1, ?2, ?3, ?4)",
      params![user, self.qso_id.cast_signed(), self.mime, self.data],
    )?;
    Ok(())
  }

  fn load(conn: &Connection, user: i64, key: &str) -> rusqlite::Result<Self> {
    let qso_id = entry_id_from_key(key).ok_or_else(|| corrupt(format!("{key} 不是影像的 key")))?;
    conn
      .query_row(
        "SELECT mime, data FROM qsl_images WHERE user_id = ?1 AND qso_id = ?2",
        params![user, qso_id.cast_signed()],
        |r| {
          Ok(Self {
            qso_id,
            mime: r.get(0)?,
            data: r.get(1)?,
          })
        },
      )
      .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => corrupt(format!(
          "{} 标为专表存放，但表里没有这张影像",
          key_for(qso_id)
        )),
        other => other,
      })
  }

  fn clear(conn: &Connection, user: i64, key: &str) -> rusqlite::Result<()> {
    let Some(qso_id) = entry_id_from_key(key) else {
      return Ok(());
    };
    conn
      .execute(
        "DELETE FROM qsl_images WHERE user_id = ?1 AND qso_id = ?2",
        params![user, qso_id.cast_signed()],
      )
      .map(|_| ())
  }
}
