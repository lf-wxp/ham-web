//! APRS 未压缩位置报文的编解码。
//!
//! 位置字段形如 `3114.50N/12128.50E`：纬度 `ddmm.mm` + 半球（N/S），
//! 经度 `dddmm.mm` + 半球（E/W）。本模块不涉及 AX.25 帧与 Mic-E 压缩。

/// 把十进制度编码为 APRS 纬度字段（如 `31.2417° → "3114.50N"`）。
#[must_use]
pub fn encode_lat(deg: f64) -> Option<String> {
  if !(-90.0..=90.0).contains(&deg) {
    return None;
  }
  let hemi = if deg >= 0.0 { 'N' } else { 'S' };
  let hundredths = (deg.abs() * 6000.0).round() as i64;
  let d = hundredths / 6000;
  let minute_hundredths = hundredths % 6000;
  let m = minute_hundredths / 100;
  let frac = minute_hundredths % 100;
  Some(format!("{d:02}{m:02}.{frac:02}{hemi}"))
}

/// 把 APRS 纬度字段解码为十进制度（如 `"3114.50N" → 31.2417`）。
#[must_use]
pub fn decode_lat(field: &str) -> Option<f64> {
  let s = field.trim();
  // 字段只允许 ASCII（数字、小数点、半球字母）：既符合 APRS 规范，也避免多字节
  // 字符让字节长度恰好等于 4 时在 `d_str[0..2]` 等字节切片处 panic。
  if !s.is_ascii() || s.len() < 8 {
    return None;
  }
  let (num, hemi) = s.split_at(s.len() - 1);
  let hemi = hemi.chars().next()?.to_ascii_uppercase();
  let (d_str, frac_str) = num.split_once('.')?;
  if d_str.len() != 4 || frac_str.len() != 2 {
    return None;
  }
  let d: f64 = d_str[0..2].parse().ok()?;
  let m: f64 = d_str[2..4].parse().ok()?;
  let frac: f64 = frac_str.parse().ok()?;
  let v = d + (m + frac / 100.0) / 60.0;
  match hemi {
    'S' => Some(-v),
    'N' => Some(v),
    _ => None,
  }
}

/// 把十进制度编码为 APRS 经度字段（如 `121.475° → "12128.50E"`）。
#[must_use]
pub fn encode_lon(deg: f64) -> Option<String> {
  if !(-180.0..=180.0).contains(&deg) {
    return None;
  }
  let hemi = if deg >= 0.0 { 'E' } else { 'W' };
  let hundredths = (deg.abs() * 6000.0).round() as i64;
  let d = hundredths / 6000;
  let minute_hundredths = hundredths % 6000;
  let m = minute_hundredths / 100;
  let frac = minute_hundredths % 100;
  Some(format!("{d:03}{m:02}.{frac:02}{hemi}"))
}

/// 把 APRS 经度字段解码为十进制度（如 `"12128.50E" → 121.475`）。
#[must_use]
pub fn decode_lon(field: &str) -> Option<f64> {
  let s = field.trim();
  // 同 [`decode_lat`]：限定 ASCII，避免多字节字符在字节切片处 panic。
  if !s.is_ascii() || s.len() < 9 {
    return None;
  }
  let (num, hemi) = s.split_at(s.len() - 1);
  let hemi = hemi.chars().next()?.to_ascii_uppercase();
  let (d_str, frac_str) = num.split_once('.')?;
  if d_str.len() != 5 || frac_str.len() != 2 {
    return None;
  }
  let d: f64 = d_str[0..3].parse().ok()?;
  let m: f64 = d_str[3..5].parse().ok()?;
  let frac: f64 = frac_str.parse().ok()?;
  let v = d + (m + frac / 100.0) / 60.0;
  match hemi {
    'W' => Some(-v),
    'E' => Some(v),
    _ => None,
  }
}

/// 组合为完整 APRS 位置字段（`"3114.50N/12128.50E"`）。
#[must_use]
pub fn encode_position(lat: f64, lon: f64) -> Option<String> {
  Some(format!("{}/{}", encode_lat(lat)?, encode_lon(lon)?))
}

/// 解析完整 APRS 位置字段为（纬度, 经度）。
#[must_use]
pub fn decode_position(field: &str) -> Option<(f64, f64)> {
  let (lat_str, lon_str) = field.trim().split_once('/')?;
  Some((decode_lat(lat_str)?, decode_lon(lon_str)?))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn lat_round_trip() {
    assert_eq!(encode_lat(31.2417), Some("3114.50N".to_string()));
    assert_eq!(encode_lat(-31.2417), Some("3114.50S".to_string()));
    let v = decode_lat("3114.50N").unwrap();
    assert!((v - 31.241667).abs() < 1e-4, "v={v}");
  }

  #[test]
  fn lon_round_trip() {
    assert_eq!(encode_lon(121.475), Some("12128.50E".to_string()));
    assert_eq!(encode_lon(-121.475), Some("12128.50W".to_string()));
    let v = decode_lon("12128.50E").unwrap();
    assert!((v - 121.475).abs() < 1e-4, "v={v}");
  }

  #[test]
  fn position_round_trip() {
    let pos = encode_position(31.2417, 121.475).unwrap();
    assert_eq!(pos, "3114.50N/12128.50E");
    let (lat, lon) = decode_position(&pos).unwrap();
    assert!((lat - 31.241667).abs() < 1e-4);
    assert!((lon - 121.475).abs() < 1e-4);
  }

  #[test]
  fn rejects_out_of_range() {
    assert!(encode_lat(91.0).is_none());
    assert!(encode_lon(181.0).is_none());
    assert!(decode_lat("bad").is_none());
    assert!(decode_position("no-slash").is_none());
  }

  #[test]
  fn rejects_invalid_hemisphere_and_frac_len() {
    // 非法半球字符不能静默当作北半球 / 东半球。
    assert!(decode_lat("3114.50X").is_none());
    assert!(decode_lon("12128.50Z").is_none());
    // 小数分钟必须是两位数字（APRS 规范）。
    assert!(decode_lat("3114.5N").is_none());
    assert!(decode_lon("12128.5E").is_none());
  }

  #[test]
  fn rejects_multibyte_without_panicking() {
    // 多字节字符（如 `中`）可能让字节长度恰好等于 4/5，若按字节切片会 panic；
    // 这里应安全地返回 None，而不是崩溃。
    assert!(decode_lat("3中.50N").is_none());
    assert!(decode_lon("1中228.50E").is_none());
    assert!(decode_position("3中.50N/12128.50E").is_none());
  }
}
