//! 通联录音：一条通联可以挂一段本地录音（站内录的或选一个文件），存浏览器本机。
//!
//! 规则与 QSL 卡片影像（[`crate::qsl_image`]）一一对应——录音同样是大体积数据，所以走同一条路：
//! 本体存 IndexedDB 的 `qso-audio:<通联 id>`（只有权威层，不进 localStorage 的 5 MB 快照），
//! 「哪些通联有录音」只留一个 id 集合在 localStorage（[`INDEX_KEY`]）供列表与备份读取。
//!
//! 和影像**不同**的地方只有一处，而且是有意的：录音**不上传、不同步**，只在本机，
//! 也**默认不进备份**（备份里是单独一个「通联录音」模块，需要用户勾选）。
//! 所以本模块的 key 一律归「设备专属」（见 [`crate::remote_sync::is_device_local_key`]）。
//!
//! 这里放不依赖浏览器的部分：key 怎么拼（**持久化格式**，改了就读不出已存的录音）、
//! 上传前的类型 / 体积校验、备份里值的校验。

/// 录音本体在存储层的 key 前缀。
pub const KEY_PREFIX: &str = "qso-audio:";

/// 「哪些通联存了录音」的 id 索引（localStorage 里的 `Vec<u64>` JSON，设备专属）。
pub const INDEX_KEY: &str = "qso-audio-index";

/// 单段录音的原始文件大小上限（字节）。
///
/// 站内录音（Opus / WebM）每分钟约 0.4 MB，8 MiB 够录二十分钟；更大的多半是整段电台录音文件，
/// 放进浏览器存储（还要 base64 膨胀三分之一）不划算。
pub const MAX_SOURCE_BYTES: f64 = 8.0 * 1024.0 * 1024.0;

/// 存储 / 备份里单条值（dataURL 字符串）的长度上限：8 MiB 的 base64 约 10.7 MiB，留出余量。
pub const MAX_STORED_CHARS: usize = 12 * 1024 * 1024;

/// 受支持的音频类型（不含参数，小写）。
///
/// 只收常见的音频容器：浏览器录出来的 WebM / Ogg / MP4，手机与电台常见的 MP3 / WAV / M4A / AAC / FLAC。
/// 值最终会被当 `<audio src>` 播放，所以不收 `text/*`、`image/svg+xml` 这类能带脚本的类型。
const SUPPORTED: &[&str] = &[
  "audio/webm",
  "audio/ogg",
  "audio/opus",
  "audio/mpeg",
  "audio/mp3",
  "audio/wav",
  "audio/x-wav",
  "audio/wave",
  "audio/mp4",
  "audio/x-m4a",
  "audio/m4a",
  "audio/aac",
  "audio/flac",
  "audio/x-flac",
];

/// 上传被拒绝的原因。只给原因、不给文案（文案由界面按调用点的 `t("…")` 字面量取）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reject {
  /// 不是受支持的音频类型。
  UnsupportedType,
  /// 超过体积上限。
  TooLarge,
  /// 空文件。
  Empty,
}

/// 去掉 MIME 里的参数并转小写：`audio/webm;codecs=opus` → `audio/webm`。
#[must_use]
pub fn base_type(mime: &str) -> String {
  mime
    .split(';')
    .next()
    .unwrap_or("")
    .trim()
    .to_ascii_lowercase()
}

/// 是否受支持的音频类型（忽略参数与大小写）。
#[must_use]
pub fn is_supported_type(mime: &str) -> bool {
  SUPPORTED.contains(&base_type(mime).as_str())
}

/// 上传前校验：类型、是否为空、体积。
///
/// `bytes` 用 `f64`（与 `web_sys::File::size()` 一致）；非有限值按「过大」拒绝。
///
/// # Errors
/// 见 [`Reject`]；先看类型，再看是否为空，最后看体积。
pub fn check(mime: &str, bytes: f64) -> Result<(), Reject> {
  if !is_supported_type(mime) {
    return Err(Reject::UnsupportedType);
  }
  if bytes == 0.0 {
    return Err(Reject::Empty);
  }
  if !bytes.is_finite() || bytes < 0.0 || bytes > MAX_SOURCE_BYTES {
    return Err(Reject::TooLarge);
  }
  Ok(())
}

/// 录音在存储层的 key —— 一条通联一段，重新挂接即替换。
#[must_use]
pub fn key_for(entry_id: u64) -> String {
  format!("{KEY_PREFIX}{entry_id}")
}

/// [`key_for`] 的逆运算：`qso-audio:<id>` → `id`；格式不对返回 `None`。
///
/// 要求纯数字且无前导零（`+7`、`007` 都拒绝）：否则 `qso-audio:7` 与 `qso-audio:007` 会解析成同一个 id，
/// 在备份里互相覆盖；`key_for` 从不产出前导零的 key。
#[must_use]
pub fn entry_id_from_key(key: &str) -> Option<u64> {
  let id = key.strip_prefix(KEY_PREFIX)?;
  (!id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()) && (id == "0" || !id.starts_with('0')))
    .then(|| id.parse().ok())
    .flatten()
}

/// 把浏览器读出的 dataURL 规范成存储格式：去掉 MIME 参数（`data:audio/webm;codecs=opus;base64,…` →
/// `data:audio/webm;base64,…`）。类型不受支持、不是 base64、内容为空或含杂字符、超长都返回 `None`。
#[must_use]
pub fn normalize_data_url(url: &str) -> Option<String> {
  let rest = url.strip_prefix("data:")?;
  let (head, data) = rest.split_once(";base64,")?;
  let mime = base_type(head);
  (SUPPORTED.contains(&mime.as_str()) && is_base64(data) && url.len() <= MAX_STORED_CHARS)
    .then(|| format!("data:{mime};base64,{data}"))
}

fn is_base64(data: &str) -> bool {
  !data.is_empty()
    && data
      .bytes()
      .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'='))
}

/// 存储 / 备份里的值是否像本站写出的数据：`data:<受支持类型>;base64,<base64 字符>`（不带参数），
/// 且不超过 [`MAX_STORED_CHARS`]。
///
/// 备份文件是外部输入：导入侧会把值原样写进 IndexedDB、之后当 `<audio src>` 播放，所以类型走白名单。
#[must_use]
pub fn is_valid_stored_value(value: &str) -> bool {
  value.len() <= MAX_STORED_CHARS
    && value
      .strip_prefix("data:")
      .and_then(|rest| rest.split_once(";base64,"))
      .is_some_and(|(mime, data)| SUPPORTED.contains(&mime) && is_base64(data))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn keys_are_stable_persisted_formats() {
    // key 是持久化格式：改了就读不出用户已经存下的录音，所以逐字钉住。
    assert_eq!(KEY_PREFIX, "qso-audio:");
    assert_eq!(INDEX_KEY, "qso-audio-index");
    assert_eq!(key_for(1), "qso-audio:1");
    assert_eq!(key_for(42), "qso-audio:42");
    assert_ne!(key_for(4), key_for(44));
    // 不能与 QSL 影像撞：备份里两者按前缀分流。
    assert_ne!(KEY_PREFIX, crate::qsl_image::KEY_PREFIX);
    assert!(!key_for(7).starts_with(crate::qsl_image::KEY_PREFIX));
  }

  #[test]
  fn entry_id_round_trips_and_rejects_malformed_keys() {
    for id in [0, 1, 42, u64::MAX] {
      assert_eq!(entry_id_from_key(&key_for(id)), Some(id));
    }
    for bad in [
      "qso-audio:",
      "qso-audio:+7",
      "qso-audio:-1",
      "qso-audio:007",
      "qso-audio:00",
      "qso-audio:1a",
      "qso-audio:99999999999999999999",
      "qso-audio-index",
      "qsl-image:7",
      "logbook",
    ] {
      assert_eq!(entry_id_from_key(bad), None, "{bad} 不应解析成 id");
    }
  }

  #[test]
  fn check_looks_at_type_then_emptiness_then_size() {
    assert_eq!(check("audio/webm", 1.0), Ok(()));
    assert_eq!(check("audio/webm;codecs=opus", 100_000.0), Ok(()));
    assert_eq!(check("AUDIO/MPEG", 1000.0), Ok(()), "大小写不敏感");
    assert_eq!(
      check("audio/webm", MAX_SOURCE_BYTES),
      Ok(()),
      "恰好等于上限放行"
    );
    assert_eq!(
      check("audio/webm", MAX_SOURCE_BYTES + 1.0),
      Err(Reject::TooLarge)
    );
    assert_eq!(check("audio/webm", 0.0), Err(Reject::Empty));
    assert_eq!(check("audio/webm", -1.0), Err(Reject::TooLarge));
    assert_eq!(check("audio/webm", f64::NAN), Err(Reject::TooLarge));
    assert_eq!(check("audio/webm", f64::INFINITY), Err(Reject::TooLarge));
    // 类型优先：不支持的类型即使是空文件也报类型。
    assert_eq!(check("text/plain", 0.0), Err(Reject::UnsupportedType));
    for bad in [
      "",
      "video/mp4",
      "image/png",
      "text/html",
      "application/octet-stream",
      "audio/",
    ] {
      assert_eq!(check(bad, 10.0), Err(Reject::UnsupportedType), "{bad:?}");
    }
  }

  #[test]
  fn the_whole_whitelist_is_accepted() {
    for mime in SUPPORTED {
      assert!(is_supported_type(mime), "{mime}");
      assert!(
        is_supported_type(&format!("{mime};codecs=x")),
        "{mime} 带参数"
      );
    }
  }

  #[test]
  fn data_urls_are_normalised_without_mime_parameters() {
    assert_eq!(
      normalize_data_url("data:audio/webm;codecs=opus;base64,GkXfo0AB").as_deref(),
      Some("data:audio/webm;base64,GkXfo0AB")
    );
    assert_eq!(
      normalize_data_url("data:audio/mpeg;base64,SUQz").as_deref(),
      Some("data:audio/mpeg;base64,SUQz")
    );
    assert_eq!(
      normalize_data_url("data:AUDIO/WAV;base64,UklGRg==").as_deref(),
      Some("data:audio/wav;base64,UklGRg==")
    );
    for bad in [
      "data:text/html;base64,PGI+",
      "data:image/png;base64,iVBORw0KGgo=",
      "data:audio/webm,rawtext",
      "data:audio/webm;base64,",
      "data:audio/webm;base64,abc\"onerror=1",
      "javascript:alert(1)",
      "",
    ] {
      assert_eq!(normalize_data_url(bad), None, "{bad:?}");
    }
    let huge = format!("data:audio/webm;base64,{}", "A".repeat(MAX_STORED_CHARS));
    assert_eq!(normalize_data_url(&huge), None, "超长拒绝");
  }

  #[test]
  fn stored_values_must_be_plain_supported_audio_data_urls() {
    assert!(is_valid_stored_value("data:audio/webm;base64,GkXfo0AB"));
    assert!(is_valid_stored_value("data:audio/mpeg;base64,SUQz"));
    assert!(is_valid_stored_value("data:audio/wav;base64,UklGRg=="));
    // 存储格式不带参数（`normalize_data_url` 已去掉）；非音频、非 base64、空内容、杂字符、跳转一律拒绝。
    assert!(!is_valid_stored_value(
      "data:audio/webm;codecs=opus;base64,GkXf"
    ));
    assert!(!is_valid_stored_value("data:image/svg+xml;base64,PHN2Zz4="));
    assert!(!is_valid_stored_value("data:text/html;base64,PGI+"));
    assert!(!is_valid_stored_value("data:audio/webm,rawtext"));
    assert!(!is_valid_stored_value("data:audio/webm;base64,"));
    assert!(!is_valid_stored_value(
      "data:audio/webm;base64,abc\"onerror=1"
    ));
    assert!(!is_valid_stored_value("javascript:alert(1)"));
    assert!(!is_valid_stored_value(""));
    let huge = format!("data:audio/webm;base64,{}", "A".repeat(MAX_STORED_CHARS));
    assert!(!is_valid_stored_value(&huge));
  }

  /// 录音「不上传」：本体与索引都是设备专属，既不进同步内容，也不进增量同步的范围——
  /// 即使用户在账号通道里勾了「同步 QSL 影像」。
  #[test]
  fn audio_is_never_synced_or_uploaded() {
    for key in [
      key_for(0),
      key_for(7),
      key_for(u64::MAX),
      INDEX_KEY.to_owned(),
    ] {
      assert!(crate::remote_sync::is_device_local_key(&key), "{key}");
      assert!(
        !crate::delta::in_scope(&key, true),
        "{key}：即使同步影像也不该带上录音"
      );
      assert!(!crate::delta::in_scope(&key, false), "{key}");
    }
    // 对照：QSL 影像在勾选同步影像时是会同步的。
    assert!(crate::delta::in_scope(&crate::qsl_image::key_for(7), true));
  }

  #[test]
  fn every_normalised_value_is_a_valid_stored_value() {
    for url in [
      "data:audio/webm;codecs=opus;base64,GkXfo0AB",
      "data:audio/ogg;codecs=opus;base64,T2dnUw==",
      "data:audio/mp4;base64,AAAA",
    ] {
      let n = normalize_data_url(url).unwrap();
      assert!(is_valid_stored_value(&n), "{n}");
    }
  }
}
