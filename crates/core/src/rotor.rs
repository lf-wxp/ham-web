//! 天线旋转器（rotor）控制协议：Yaesu GS-232A/B ASCII 命令生成与角度应答解析。
//!
//! GS-232 是业余旋转器最常见的串口协议（G-5500 / G-1000DXA 及众多 DIY 控制器），
//! 命令与应答均为 ASCII 文本，以回车结尾：
//! - `C` 查询方位角，`B` 查询仰角，应答形如 `+0XXX`（0–359°）；
//! - `MXXX` 转向目标方位，`WXXX` 抬升到目标仰角；
//! - `S` 停止。

/// 查询方位角命令。
pub const QUERY_AZ: &[u8] = b"C";
/// 查询仰角命令。
pub const QUERY_EL: &[u8] = b"B";
/// 停止命令。
pub const STOP: &[u8] = b"S";

/// 生成转向目标方位（0–359°）的命令。
#[must_use]
pub fn set_azimuth(az: u16) -> Vec<u8> {
  format!("M{:03}", az.min(359)).into_bytes()
}

/// 生成抬升到目标仰角（0–180°）的命令。
#[must_use]
pub fn set_elevation(el: u16) -> Vec<u8> {
  format!("W{:03}", el.min(180)).into_bytes()
}

/// 解析 `+0XXX` 形式的角度应答（方位与仰角共用，0–359°）。
#[must_use]
pub fn parse_angle(frame: &str) -> Option<u16> {
  let digits = frame.trim().strip_prefix('+')?.trim();
  if !digits.bytes().all(|b| b.is_ascii_digit()) {
    return None;
  }
  let v: u16 = digits.parse().ok()?;
  (v <= 359).then_some(v)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_angles() {
    assert_eq!(parse_angle("+0145"), Some(145));
    assert_eq!(parse_angle("+000"), Some(0));
    assert_eq!(parse_angle("+359"), Some(359));
    assert_eq!(parse_angle("+360"), None);
    assert_eq!(parse_angle("+abc"), None);
    assert_eq!(parse_angle("0145"), None);
  }

  #[test]
  fn commands() {
    assert_eq!(set_azimuth(145), b"M145".to_vec());
    assert_eq!(set_azimuth(5), b"M005".to_vec());
    assert_eq!(set_azimuth(400), b"M359".to_vec());
    assert_eq!(set_elevation(45), b"W045".to_vec());
    assert_eq!(set_elevation(200), b"W180".to_vec());
    assert_eq!(STOP, b"S");
    assert_eq!(QUERY_AZ, b"C");
    assert_eq!(QUERY_EL, b"B");
  }
}
