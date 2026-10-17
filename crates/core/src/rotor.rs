//! 天线旋转器（rotor）控制协议：Yaesu GS-232A / GS-232B ASCII 命令生成与应答解析，以及卫星自动跟踪的写入护栏。
//!
//! GS-232 是业余旋转器最常见的串口协议（G-5500 / G-1000DXA 及众多 DIY 控制器）。命令以回车 `\r` 结尾
//! （Hamlib `gs232a.c` / `gs232b.c` 的 `EOM`），**没有这个回车多数控制器不会执行**：
//!
//! - `W<方位> <仰角>`：**一条命令同时给方位和仰角**，各 3 位补零、中间一个空格（Hamlib `W%03u %03u`）。
//!   单独给仰角的 `W045` 不是合法命令；
//! - `C2`：查询方位与仰角（Hamlib 用它；旧的 `C` 只回方位）；
//! - `S`：全部停止。
//!
//! 应答两种风格（Hamlib 两个后端各解析一种）：
//!
//! - GS-232A：`+0aaa+0eee`（如 `+0145+0045`）；
//! - GS-232B：`AZ=aaa EL=eee`（空格数不定；个别控制器只回 `AZ=aaa`）。
//!
//! # 自动跟踪的护栏
//!
//! 旋转器会驱动真实的机械运动，所以自动跟踪比写电台频率更保守，护栏放在这里（可单测）：
//!
//! - 仰角低于 [`MIN_TRACK_ELEVATION`] 不动：卫星在地平线下没有指向意义；
//! - 方位与仰角都夹进合法范围（方位回绕到 0–359°，仰角 0–90°），**不信任**上游给的任何数；
//! - 写入有死区 [`MIN_MOVE_DEG`]：目标变化小于它不重发，避免电机来回抖；
//! - 不处理线缆缠绕与行程限位：**本站不知道你的旋转器是 360° 还是 450°，也不知道它的限位在哪**。
//!   过顶（方位快速从一侧扫到另一侧）时旋转器会走哪条路由控制器自己决定，页面上写明。

/// 查询方位角命令（仅方位；旧控制器用）。
pub const QUERY_AZ: &[u8] = b"C\r";
/// 查询仰角命令（仅仰角；旧控制器用）。
pub const QUERY_EL: &[u8] = b"B\r";
/// 同时查询方位与仰角（Hamlib 用 `C2`）。
pub const QUERY_BOTH: &[u8] = b"C2\r";
/// 全部停止。
pub const STOP: &[u8] = b"S\r";

/// 自动跟踪时，仰角低于这个值（度）就不动旋转器。
pub const MIN_TRACK_ELEVATION: f64 = 0.0;
/// 目标方位 / 仰角相对上次写入至少变化这么多度才再写（死区）。
pub const MIN_MOVE_DEG: f64 = 2.0;
/// 仰角上限（度）。GS-232 的 `W` 允许到 180°（翻转式），但「天线指向卫星」只用到 0–90°。
pub const MAX_ELEVATION: u16 = 90;

/// 把任意方位角（度，可为负或超过 360）回绕到 0–359 的整数；非有限数为 `None`。
#[must_use]
pub fn wrap_azimuth(az: f64) -> Option<u16> {
  if !az.is_finite() {
    return None;
  }
  // 先回绕再取整：359.6 → 四舍五入是 360，必须再回绕成 0，不能写出 `W360`。
  let wrapped = az.rem_euclid(360.0).round() as i64 % 360;
  u16::try_from(wrapped).ok()
}

/// 把仰角夹进 0–[`MAX_ELEVATION`] 并取整；非有限数为 `None`。
#[must_use]
pub fn clamp_elevation(el: f64) -> Option<u16> {
  el.is_finite()
    .then(|| el.clamp(0.0, f64::from(MAX_ELEVATION)).round() as u16)
}

/// 转向 `(方位, 仰角)` 的命令：`W<aaa> <eee>\r`。方位回绕、仰角夹紧；任一非有限数为 `None`（不写）。
#[must_use]
pub fn set_position(az: f64, el: f64) -> Option<Vec<u8>> {
  let (az, el) = (wrap_azimuth(az)?, clamp_elevation(el)?);
  Some(format!("W{az:03} {el:03}\r").into_bytes())
}

/// 只转方位：仰角保持 `keep_el`（调用方给，通常是最近一次读到的仰角）。
#[must_use]
pub fn set_azimuth_keep(az: f64, keep_el: f64) -> Option<Vec<u8>> {
  set_position(az, keep_el)
}

/// 解析一条应答，得到 `(方位, 仰角)`；仰角缺失（只回方位的控制器）为 `None`。
///
/// 接受 GS-232A 的 `+0aaa+0eee` 与 GS-232B 的 `AZ=aaa EL=eee` / `AZ=aaa`；方位必须在 0–359、仰角在 0–180
/// （GS-232 本身的量程）。认不出的返回 `None`，**不猜**。
#[must_use]
pub fn parse_position(frame: &str) -> Option<(u16, Option<u16>)> {
  let s = frame.trim();
  if let Some(rest) = s.strip_prefix("AZ=") {
    let (az, el) = match rest.split_once("EL=") {
      Some((a, e)) => (a.trim(), Some(e.trim())),
      None => (rest.trim(), None),
    };
    let az = digits(az)?;
    let el = match el {
      Some(e) => Some(digits(e)?),
      None => None,
    };
    return in_range(az, el);
  }
  // GS-232A：+0aaa+0eee（共 10 个字符）。
  let body = s.strip_prefix("+0")?;
  let (az, rest) = body.split_at_checked(3)?;
  let el = rest.strip_prefix("+0")?;
  if el.len() != 3 {
    return None;
  }
  in_range(digits(az)?, Some(digits(el)?))
}

fn digits(s: &str) -> Option<u16> {
  (!s.is_empty() && s.len() <= 3 && s.bytes().all(|b| b.is_ascii_digit()))
    .then(|| s.parse().ok())
    .flatten()
}

fn in_range(az: u16, el: Option<u16>) -> Option<(u16, Option<u16>)> {
  (az <= 359 && el.is_none_or(|e| e <= 180)).then_some((az, el))
}

/// 兼容旧调用：解析**只有一个数**的应答（`+0XXX`，0–359）。新代码用 [`parse_position`]。
#[must_use]
pub fn parse_angle(frame: &str) -> Option<u16> {
  let digits = frame.trim().strip_prefix('+')?.trim();
  if !digits.bytes().all(|b| b.is_ascii_digit()) {
    return None;
  }
  let v: u16 = digits.parse().ok()?;
  (v <= 359).then_some(v)
}

/// 一次要写的目标；`None` 表示这次不动。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
  pub az: u16,
  pub el: u16,
}

/// 自动跟踪这一拍要不要动旋转器。
///
/// - 卫星在地平线下（`el_deg < ` [`MIN_TRACK_ELEVATION`]）或数值非有限：不动；
/// - 与上次**写入**的目标相比，方位（走短弧）与仰角的变化都小于 [`MIN_MOVE_DEG`]：不动（死区）；
/// - 否则返回整数化后的目标。
///
/// `last` 是上次写给旋转器的目标（不是旋转器回读的位置：回读有延迟，用它判断会反复重发）。
#[must_use]
pub fn next_target(az_deg: f64, el_deg: f64, last: Option<Target>) -> Option<Target> {
  if !el_deg.is_finite() || el_deg < MIN_TRACK_ELEVATION {
    return None;
  }
  let want = Target {
    az: wrap_azimuth(az_deg)?,
    el: clamp_elevation(el_deg)?,
  };
  match last {
    None => Some(want),
    Some(l) => {
      // 方位按短弧算差：359° → 1° 只差 2°，不是 358°。
      let d_az = (f64::from(want.az) - f64::from(l.az) + 540.0).rem_euclid(360.0) - 180.0;
      let d_el = f64::from(want.el) - f64::from(l.el);
      (d_az.abs() >= MIN_MOVE_DEG || d_el.abs() >= MIN_MOVE_DEG).then_some(want)
    }
  }
}

/// 把目标变成命令。
#[must_use]
pub fn target_command(t: Target) -> Vec<u8> {
  format!("W{:03} {:03}\r", t.az, t.el).into_bytes()
}

/// 读到的位置离目标这么近（度）就算「到位」。
pub const ARRIVED_DEG: f64 = 2.0;
/// 离目标的距离连续这么多次读数没有变小，才判定「旋转器没在往目标走」（约 8 秒）。
pub const STALL_CONFIRM_POLLS: u32 = 8;
/// 距离至少缩小这么多（度）才算「在靠近」；小于它的抖动不算。
const CLOSING_EPS_DEG: f64 = 0.5;

/// 判断「旋转器没在往我们写的目标走」：正常转动（含到位后停住）不报，被人手动转走或卡住才报。
///
/// 为什么不直接比较「读到的位置」和「目标」：旋转器转得慢（G-5500 约 3°/s），刚写完目标时位置本来就离目标几十度，
/// 那是正常的转动，不是有人动了它。真正的异常特征是：**离目标的距离不再变小**。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StallDetector {
  best: Option<f64>,
  stalled: u32,
}

impl StallDetector {
  /// 喂入一次读数（位置与我们最后写入的目标），返回「现在可以判定为没在往目标走」。
  ///
  /// 方位按短弧算距离；仰角缺失（只回方位的控制器）时只看方位。
  pub fn observe(&mut self, az: u16, el: Option<u16>, want: Target) -> bool {
    let d_az = (f64::from(az) - f64::from(want.az) + 540.0).rem_euclid(360.0) - 180.0;
    let d_el = el.map_or(0.0, |e| f64::from(e) - f64::from(want.el));
    let dist = d_az.hypot(d_el);
    if dist <= ARRIVED_DEG {
      // 到位：重新开始计（之后目标变了，会从新的距离重新比较）。
      self.best = Some(dist);
      self.stalled = 0;
      return false;
    }
    match self.best {
      Some(b) if dist < b - CLOSING_EPS_DEG => {
        self.best = Some(dist);
        self.stalled = 0;
      }
      Some(_) => self.stalled = self.stalled.saturating_add(1),
      None => self.best = Some(dist),
    }
    self.stalled >= STALL_CONFIRM_POLLS
  }

  /// 清零：写了新目标（旧的「最近距离」不再可比）、重新连接、重新开启、手动接管时都用它。
  pub fn reset(&mut self) {
    *self = Self::default();
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn position_commands_match_the_hamlib_format() {
    // Hamlib `W%03u %03u` + `\r`；数值来自独立脚本（az<0 先 +360，再四舍五入）。
    for (az, el, want) in [
      (145.0, 45.0, "W145 045\r"),
      (5.0, 0.0, "W005 000\r"),
      (-10.0, 12.0, "W350 012\r"),
      (0.0, 90.0, "W000 090\r"),
      (270.0, 30.0, "W270 030\r"),
      (359.4, 89.6, "W359 090\r"),
    ] {
      assert_eq!(
        set_position(az, el),
        Some(want.as_bytes().to_vec()),
        "{az} {el}"
      );
    }
  }

  #[test]
  fn azimuth_wraps_instead_of_writing_360() {
    // Hamlib 会写出 `W360`；这里必须回绕成 0，因为 GS-232 的方位范围是 0–359。
    assert_eq!(wrap_azimuth(359.6), Some(0));
    assert_eq!(wrap_azimuth(359.5), Some(0));
    assert_eq!(wrap_azimuth(359.4), Some(359));
    assert_eq!(wrap_azimuth(360.0), Some(0));
    assert_eq!(wrap_azimuth(725.0), Some(5));
    assert_eq!(wrap_azimuth(-10.0), Some(350));
    assert_eq!(wrap_azimuth(-370.0), Some(350));
    assert_eq!(set_position(359.6, 0.4), Some(b"W000 000\r".to_vec()));
    // 全范围扫一遍：永远在 0–359。
    let mut a = -720.0;
    while a <= 720.0 {
      assert!(wrap_azimuth(a).unwrap() <= 359, "{a}");
      a += 0.37;
    }
  }

  #[test]
  fn elevation_is_clamped_to_zero_to_ninety() {
    assert_eq!(clamp_elevation(-5.0), Some(0));
    assert_eq!(clamp_elevation(45.4), Some(45));
    assert_eq!(clamp_elevation(89.6), Some(90));
    assert_eq!(clamp_elevation(120.0), Some(90));
    assert_eq!(clamp_elevation(180.0), Some(90));
    assert_eq!(set_position(100.0, 200.0), Some(b"W100 090\r".to_vec()));
  }

  #[test]
  fn non_finite_input_is_never_written() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
      assert_eq!(wrap_azimuth(bad), None);
      assert_eq!(clamp_elevation(bad), None);
      assert_eq!(set_position(bad, 10.0), None);
      assert_eq!(set_position(10.0, bad), None);
      assert_eq!(next_target(bad, 10.0, None), None);
      assert_eq!(next_target(10.0, bad, None), None);
    }
  }

  #[test]
  fn fixed_commands_end_with_a_carriage_return() {
    assert_eq!(STOP, b"S\r");
    assert_eq!(QUERY_BOTH, b"C2\r");
    assert_eq!(QUERY_AZ, b"C\r");
    assert_eq!(QUERY_EL, b"B\r");
    for c in [STOP, QUERY_BOTH, QUERY_AZ, QUERY_EL] {
      assert_eq!(c.last(), Some(&b'\r'));
    }
  }

  #[test]
  fn parses_both_reply_styles_and_the_az_only_controller() {
    // GS-232A
    assert_eq!(parse_position("+0145+0045"), Some((145, Some(45))));
    assert_eq!(parse_position("+0359+0000"), Some((359, Some(0))));
    assert_eq!(parse_position("+0000+0180"), Some((0, Some(180))));
    // GS-232B（空格数不定）
    assert_eq!(parse_position("AZ=145 EL=045"), Some((145, Some(45))));
    assert_eq!(parse_position("AZ=145   EL=045"), Some((145, Some(45))));
    assert_eq!(parse_position("AZ=005 EL=000\r\n"), Some((5, Some(0))));
    // 只回方位的控制器（Hamlib 注释里提到的 12PR1A）
    assert_eq!(parse_position("AZ=145"), Some((145, None)));
  }

  #[test]
  fn rejects_what_it_cannot_recognise() {
    for bad in [
      "",
      "garbage",
      "+0145",
      "+0145+045",
      "+0145+0045+0",
      "0145+0045",
      "AZ=",
      "AZ=abc",
      "AZ=145 EL=",
      "AZ=145 EL=xyz",
      "AZ=360 EL=000",
      "AZ=145 EL=181",
      "+0360+0000",
      "+0145+0181",
      "AZ=1450",
      "AZ=-10",
      "+0145 +0045",
    ] {
      assert_eq!(parse_position(bad), None, "{bad:?}");
    }
  }

  #[test]
  fn the_old_single_value_parser_still_works() {
    assert_eq!(parse_angle("+0145"), Some(145));
    assert_eq!(parse_angle("+000"), Some(0));
    assert_eq!(parse_angle("+359"), Some(359));
    assert_eq!(parse_angle("+360"), None);
    assert_eq!(parse_angle("+abc"), None);
    assert_eq!(parse_angle("0145"), None);
  }

  #[test]
  fn tracking_stays_still_below_the_horizon() {
    assert_eq!(next_target(100.0, -0.1, None), None);
    assert_eq!(next_target(100.0, -30.0, None), None);
    assert_eq!(
      next_target(100.0, 0.0, None),
      Some(Target { az: 100, el: 0 })
    );
  }

  #[test]
  fn tracking_has_a_dead_zone_and_takes_the_short_arc() {
    let last = Some(Target { az: 359, el: 30 });
    // 359° → 1°：短弧只差 2°，刚好到死区边界，要动。
    assert_eq!(next_target(1.0, 30.0, last), Some(Target { az: 1, el: 30 }));
    // 359° → 0°：差 1°，在死区内，不动（若按 359 算会误以为差 359°）。
    assert_eq!(next_target(0.0, 30.0, last), None);
    // 仰角变化 1°：不动；2°：动。
    assert_eq!(next_target(359.0, 31.0, last), None);
    assert_eq!(
      next_target(359.0, 32.0, last),
      Some(Target { az: 359, el: 32 })
    );
    // 没有上次目标时一定写。
    assert!(next_target(10.0, 10.0, None).is_some());
  }

  #[test]
  fn tracking_compares_against_the_last_written_target_not_the_readback() {
    // 缓慢移动：每拍变化 1°（< 2° 死区），不会每拍都重发；累计到 2° 才发。
    let mut last = next_target(100.0, 20.0, None);
    assert_eq!(last, Some(Target { az: 100, el: 20 }));
    let mut writes = 1;
    for step in 1..=10 {
      let az = 100.0 + f64::from(step);
      if let Some(t) = next_target(az, 20.0, last) {
        last = Some(t);
        writes += 1;
      }
    }
    // 10° 的移动、2° 死区：最多约 5 次，不是 10 次。
    assert_eq!(writes, 6, "{last:?}");
  }

  #[test]
  fn target_command_matches_set_position() {
    for (az, el) in [(145.0, 45.0), (0.0, 0.0), (359.0, 90.0)] {
      let t = next_target(az, el, None).unwrap();
      assert_eq!(Some(target_command(t)), set_position(az, el));
    }
  }

  fn run(det: &mut StallDetector, positions: &[(u16, Option<u16>)], want: Target) -> Option<usize> {
    positions
      .iter()
      .position(|&(az, el)| det.observe(az, el, want))
  }

  /// 数值来自独立模拟：旋转器 3°/s 追目标时从不误报；被人手动转走 / 卡住后约 8 拍被抓到。
  #[test]
  fn a_rotor_that_is_closing_in_on_the_target_is_never_flagged() {
    let want = Target { az: 200, el: 0 };
    let mut det = StallDetector::default();
    // 从 100° 以 3°/s 追到 200°，到位后一直停着 30 拍。
    let mut path: Vec<(u16, Option<u16>)> = (0..=34)
      .map(|i| ((100 + 3 * i).min(200), Some(0)))
      .collect();
    path.extend(std::iter::repeat_n((200, Some(0)), 30));
    assert_eq!(run(&mut det, &path, want), None);
  }

  #[test]
  fn a_rotor_moved_away_or_stuck_is_flagged_after_eight_polls() {
    let want = Target { az: 200, el: 0 };
    // 追了 10 拍（100→127°）后被人手动转回 100° 并一直停着。
    let mut path: Vec<(u16, Option<u16>)> = (0..10).map(|i| (100 + 3 * i, Some(0))).collect();
    path.extend(std::iter::repeat_n((100, Some(0)), 40));
    let mut det = StallDetector::default();
    let at = run(&mut det, &path, want).expect("应当被判定为没在往目标走");
    // 下标 10 是第一个「回到 100」的读数：它离目标比最近的 73 远，从这里开始计，数满 8 次时判定。
    assert_eq!(at, 10 + (STALL_CONFIRM_POLLS as usize) - 1);

    // 卡在 127° 不动（有人按了控制器上的停止）。
    let mut stuck: Vec<(u16, Option<u16>)> = (0..10).map(|i| (100 + 3 * i, Some(0))).collect();
    stuck.extend(std::iter::repeat_n((127, Some(0)), 40));
    let mut det = StallDetector::default();
    assert!(run(&mut det, &stuck, want).is_some());
  }

  #[test]
  fn the_stall_detector_takes_the_short_arc_and_ignores_missing_elevation() {
    // 目标 5°，位置 340° → 4°：短弧只差 25° → 1°，一路在靠近。
    let want = Target { az: 5, el: 0 };
    let mut det = StallDetector::default();
    let path: Vec<(u16, Option<u16>)> = [340u16, 343, 346, 349, 352, 355, 358, 1, 4]
      .iter()
      .map(|&a| (a, None))
      .collect();
    assert_eq!(run(&mut det, &path, want), None);
    // 仰角缺失时只看方位：目标仰角再高也不影响。
    let mut det = StallDetector::default();
    assert!(!det.observe(5, None, Target { az: 5, el: 80 }));
  }

  #[test]
  fn writing_a_new_target_forgets_the_old_distance() {
    let mut det = StallDetector::default();
    let a = Target { az: 200, el: 0 };
    for p in [190u16, 195, 199] {
      det.observe(p, Some(0), a);
    }
    // 目标改成远处的 b：不清零的话，新距离比旧的「最近距离」大，会立刻开始误计数。
    let b = Target { az: 300, el: 0 };
    let mut stale = det;
    let mut fresh = det;
    fresh.reset();
    let path = [200u16, 203, 206, 209, 212, 215, 218, 221, 224, 227];
    let flagged = |d: &mut StallDetector| path.iter().any(|&p| d.observe(p, Some(0), b));
    assert!(flagged(&mut stale), "不清零会误报");
    assert!(!flagged(&mut fresh), "清零后正常");
  }

  #[test]
  fn az_only_keeps_the_given_elevation() {
    assert_eq!(set_azimuth_keep(145.0, 12.0), Some(b"W145 012\r".to_vec()));
    assert_eq!(set_azimuth_keep(145.0, f64::NAN), None);
  }
}
