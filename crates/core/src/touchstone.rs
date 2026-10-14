//! 实测 S11：Touchstone `.s1p` 解析 / 导出，以及 NanoVNA 串口 shell 回显的解析。
//!
//! 两条入口殊途同归，都落成一份 [`Sweep`]（频点 + 反射系数 Γ），之后的阻抗、驻波比、
//! 最低驻波与可用带宽都只看这一份数据：
//!
//! - **`.s1p` 文件**：任何 VNA / 天线分析仪软件都能导出，离线可用，也是自动化测试的入口
//! - **NanoVNA 串口**：固件 shell 的 `frequencies` / `data 0` 两条命令各返回一列
//!   （频率 Hz；`实部 虚部`），本模块只负责从回显里把数据行抠出来，串口收发在界面层
//!
//! 阻抗换算复用 [`crate::smith`]（`z_from_gamma` / `swr_and_return_loss`），与 Smith
//! 圆图页同一套公式，不另写一份。
//!
//! 只支持**单端口 S 参数**（`.s1p`）：Y / Z / H / G 参数与多端口文件直接拒绝并说明原因，
//! 而不是挑几列「看起来像」的数字硬算。

use crate::smith::{swr_and_return_loss, z_from_gamma};

/// 一个频点的反射系数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct S11Point {
  /// 频率（Hz）。
  pub f_hz: f64,
  /// Γ 实部。
  pub re: f64,
  /// Γ 虚部。
  pub im: f64,
}

impl S11Point {
  /// 驻波比。
  #[must_use]
  pub fn swr(&self) -> f64 {
    swr_and_return_loss(self.re, self.im).0
  }

  /// 回波损耗（dB，正数）。
  #[must_use]
  pub fn return_loss_db(&self) -> f64 {
    swr_and_return_loss(self.re, self.im).1
  }

  /// 阻抗（Ω）；Γ = 1（开路）时为 `None`。
  #[must_use]
  pub fn impedance(&self, z0: f64) -> Option<(f64, f64)> {
    z_from_gamma(self.re, self.im).map(|(r, x)| (r * z0, x * z0))
  }
}

/// 一次扫频。
#[derive(Debug, Clone, PartialEq)]
pub struct Sweep {
  /// 参考阻抗（Ω）。
  pub z0: f64,
  /// 频点（按频率升序）。
  pub points: Vec<S11Point>,
}

/// 解析失败的原因（界面按种类给出提示）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
  /// 没有任何数据行。
  NoData,
  /// 选项行（`# …`）里有不支持的参数类型或格式。
  Unsupported(String),
  /// 第 N 行（1 起）的列数不是 3 —— 多半是多端口文件。
  NotOnePort(usize),
  /// 第 N 行（1 起）有非数字或非有限值。
  BadNumber(usize),
  /// 频率没有严格递增（第 N 行起）。
  NotIncreasing(usize),
  /// NanoVNA 两条命令返回的行数对不上。
  LengthMismatch(usize, usize),
}

/// `.s1p` 的数据格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
  /// 实部 / 虚部。
  Ri,
  /// 线性幅度 / 角度（度）。
  Ma,
  /// dB 幅度 / 角度（度）。
  Db,
}

/// 解析 Touchstone v1 单端口文件。
///
/// 缺省选项与规范一致：`# GHz S MA R 50`。`!` 起到行尾是注释；选项行只认第一条
/// （规范如此，后面的 `#` 行忽略）。
pub fn parse_s1p(text: &str) -> Result<Sweep, ParseError> {
  let mut scale = 1e9;
  let mut format = Format::Ma;
  let mut z0 = 50.0;
  let mut seen_option = false;
  let mut points: Vec<S11Point> = Vec::new();
  for (idx, raw) in text.lines().enumerate() {
    let line_no = idx + 1;
    let line = raw.split('!').next().unwrap_or("").trim();
    if line.is_empty() {
      continue;
    }
    if let Some(opts) = line.strip_prefix('#') {
      if seen_option {
        continue;
      }
      seen_option = true;
      let toks: Vec<String> = opts
        .split_whitespace()
        .map(str::to_ascii_uppercase)
        .collect();
      let mut i = 0;
      while i < toks.len() {
        match toks[i].as_str() {
          "HZ" => scale = 1.0,
          "KHZ" => scale = 1e3,
          "MHZ" => scale = 1e6,
          "GHZ" => scale = 1e9,
          "S" => {}
          "Y" | "Z" | "H" | "G" => {
            return Err(ParseError::Unsupported(toks[i].clone()));
          }
          "RI" => format = Format::Ri,
          "MA" => format = Format::Ma,
          "DB" => format = Format::Db,
          "R" => {
            let v = toks
              .get(i + 1)
              .and_then(|s| s.parse::<f64>().ok())
              .filter(|v| v.is_finite() && *v > 0.0)
              .ok_or_else(|| ParseError::Unsupported("R".to_owned()))?;
            z0 = v;
            i += 1;
          }
          other => return Err(ParseError::Unsupported(other.to_owned())),
        }
        i += 1;
      }
      continue;
    }
    let nums: Vec<&str> = line.split_whitespace().collect();
    if nums.len() != 3 {
      return Err(ParseError::NotOnePort(line_no));
    }
    let mut v = [0.0f64; 3];
    for (slot, s) in v.iter_mut().zip(&nums) {
      *slot = s
        .parse::<f64>()
        .ok()
        .filter(|x| x.is_finite())
        .ok_or(ParseError::BadNumber(line_no))?;
    }
    let f_hz = v[0] * scale;
    if points.last().is_some_and(|p| f_hz <= p.f_hz) {
      return Err(ParseError::NotIncreasing(line_no));
    }
    let (re, im) = match format {
      Format::Ri => (v[1], v[2]),
      Format::Ma => polar(v[1], v[2]),
      Format::Db => polar(10f64.powf(v[1] / 20.0), v[2]),
    };
    points.push(S11Point { f_hz, re, im });
  }
  if points.is_empty() {
    return Err(ParseError::NoData);
  }
  Ok(Sweep { z0, points })
}

fn polar(mag: f64, deg: f64) -> (f64, f64) {
  let a = deg.to_radians();
  (mag * a.cos(), mag * a.sin())
}

/// 导出为 `.s1p`（`# Hz S RI R z0`）：RI 格式不经过角度换算，往返无损。
#[must_use]
pub fn to_s1p(sweep: &Sweep) -> String {
  let mut out = String::from("! exported by ham-exam-web\n");
  out.push_str(&format!("# Hz S RI R {}\n", sweep.z0));
  for p in &sweep.points {
    out.push_str(&format!("{} {:.9e} {:.9e}\n", p.f_hz, p.re, p.im));
  }
  out
}

/// NanoVNA shell 一条命令的回显里，去掉命令回显行与结尾提示符（`ch> `）后的数据行。
///
/// 固件会先把收到的命令原样回显一行，再输出数据，最后打印提示符；分块到达的串口
/// 数据由调用方拼好整段再交给这里。
#[must_use]
pub fn shell_lines<'a>(raw: &'a str, command: &str) -> Vec<&'a str> {
  raw
    .lines()
    .map(str::trim)
    .filter(|l| !l.is_empty() && *l != command.trim() && !l.starts_with("ch>"))
    .collect()
}

/// 回显里是否已经出现提示符（命令执行完毕）。
#[must_use]
pub fn shell_done(raw: &str) -> bool {
  raw.trim_end().ends_with("ch>")
}

/// 由 `frequencies` 与 `data 0` 两条命令的回显拼成一份扫频。
pub fn nanovna_sweep(freq_raw: &str, data_raw: &str, z0: f64) -> Result<Sweep, ParseError> {
  let freqs = shell_lines(freq_raw, "frequencies");
  let data = shell_lines(data_raw, "data 0");
  if freqs.is_empty() || data.is_empty() {
    return Err(ParseError::NoData);
  }
  if freqs.len() != data.len() {
    return Err(ParseError::LengthMismatch(freqs.len(), data.len()));
  }
  let mut points = Vec::with_capacity(freqs.len());
  for (i, (f, d)) in freqs.iter().zip(&data).enumerate() {
    let line = i + 1;
    let f_hz = f
      .parse::<f64>()
      .ok()
      .filter(|v| v.is_finite() && *v > 0.0)
      .ok_or(ParseError::BadNumber(line))?;
    let nums: Vec<f64> = d
      .split_whitespace()
      .map(|s| s.parse::<f64>().ok().filter(|v| v.is_finite()))
      .collect::<Option<Vec<_>>>()
      .ok_or(ParseError::BadNumber(line))?;
    let [re, im] = nums[..] else {
      return Err(ParseError::BadNumber(line));
    };
    if points.last().is_some_and(|p: &S11Point| f_hz <= p.f_hz) {
      return Err(ParseError::NotIncreasing(line));
    }
    points.push(S11Point { f_hz, re, im });
  }
  Ok(Sweep { z0, points })
}

/// 最低驻波比所在的频点（下标）。
#[must_use]
pub fn min_swr_index(sweep: &Sweep) -> Option<usize> {
  sweep
    .points
    .iter()
    .enumerate()
    .filter(|(_, p)| p.swr().is_finite())
    .min_by(|a, b| a.1.swr().total_cmp(&b.1.swr()))
    .map(|(i, _)| i)
}

/// 包含最低驻波点、驻波比不超过 `limit` 的连续频段 `(下沿, 上沿)`（Hz）。
///
/// 边沿在相邻两个实测点之间**线性插值**（实测数据只有这些点，插值是最不武断的估计）；
/// 一直到扫频边界都不超标时，边沿就是扫频边界 —— 此时真实带宽可能更宽，
/// 界面应如实提示「超出扫频范围」。最低点本身就超标时返回 `None`。
#[must_use]
pub fn swr_band(sweep: &Sweep, limit: f64) -> Option<(f64, f64)> {
  let best = min_swr_index(sweep)?;
  let pts = &sweep.points;
  if pts[best].swr() > limit {
    return None;
  }
  let cross = |a: &S11Point, b: &S11Point| {
    let (sa, sb) = (a.swr(), b.swr());
    if (sb - sa).abs() < 1e-12 {
      a.f_hz
    } else {
      a.f_hz + (limit - sa) / (sb - sa) * (b.f_hz - a.f_hz)
    }
  };
  let mut lo = pts[0].f_hz;
  for i in (0..best).rev() {
    if pts[i].swr() > limit {
      lo = cross(&pts[i + 1], &pts[i]);
      break;
    }
  }
  let mut hi = pts[pts.len() - 1].f_hz;
  for i in best + 1..pts.len() {
    if pts[i].swr() > limit {
      hi = cross(&pts[i - 1], &pts[i]);
      break;
    }
  }
  Some((lo, hi))
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 归一化阻抗 → Γ（与 `smith::gamma` 同式，这里独立写一份当对照）。
  fn gamma_of(r: f64, x: f64) -> (f64, f64) {
    let d = (r + 1.0).powi(2) + x * x;
    ((r * r + x * x - 1.0) / d, 2.0 * x / d)
  }

  #[test]
  fn parses_ri_ma_and_db_formats_to_the_same_gamma() {
    // 同一个 Γ = 0.6∠30° 用三种格式写，解出来必须一致。
    let (re, im) = (
      0.6 * 30f64.to_radians().cos(),
      0.6 * 30f64.to_radians().sin(),
    );
    let db = 20.0 * 0.6f64.log10();
    for text in [
      format!("# MHz S RI R 50\n14.0 {re} {im}\n"),
      "# MHz S MA R 50\n14.0 0.6 30\n".to_owned(),
      format!("# MHz S DB R 50\n14.0 {db} 30\n"),
    ] {
      let s = parse_s1p(&text).unwrap();
      assert_eq!(s.points.len(), 1);
      let p = s.points[0];
      assert!((p.f_hz - 14e6).abs() < 1e-6);
      assert!(
        (p.re - re).abs() < 1e-12 && (p.im - im).abs() < 1e-12,
        "{text}"
      );
    }
  }

  #[test]
  fn defaults_comments_and_reference_impedance() {
    // 无选项行 = GHz / MA / 50 Ω；`!` 注释与空行都跳过。
    let s = parse_s1p("! hello\n\n1.0 0.5 0 ! trailing\n").unwrap();
    assert!((s.points[0].f_hz - 1e9).abs() < 1e-3);
    assert_eq!(s.z0, 50.0);
    let s = parse_s1p("# khz s ri r 75\n7000 0 0\n").unwrap();
    assert_eq!(s.z0, 75.0);
    assert!((s.points[0].f_hz - 7e6).abs() < 1e-6);
    // Γ = 0 时阻抗正好是参考阻抗。
    let z = s.points[0].impedance(s.z0).unwrap();
    assert!((z.0 - 75.0).abs() < 1e-9 && z.1.abs() < 1e-9);
  }

  #[test]
  fn rejects_what_it_cannot_honour() {
    assert_eq!(parse_s1p(""), Err(ParseError::NoData));
    assert_eq!(
      parse_s1p("# MHz Z RI R 50\n1 0 0\n"),
      Err(ParseError::Unsupported("Z".into()))
    );
    // 两端口文件：一行 9 列。
    assert_eq!(
      parse_s1p("# MHz S RI R 50\n1 0 0 0 0 0 0 0 0\n"),
      Err(ParseError::NotOnePort(2))
    );
    assert_eq!(
      parse_s1p("# MHz S RI R 50\n1 x 0\n"),
      Err(ParseError::BadNumber(2))
    );
    assert_eq!(
      parse_s1p("# MHz S RI R 50\n2 0 0\n1 0 0\n"),
      Err(ParseError::NotIncreasing(3))
    );
  }

  #[test]
  fn export_round_trips_exactly() {
    let text = "# MHz S MA R 50\n7.0 0.3 -40\n7.1 0.1 10\n7.2 0.35 60\n";
    let a = parse_s1p(text).unwrap();
    let b = parse_s1p(&to_s1p(&a)).unwrap();
    assert_eq!(a.z0, b.z0);
    for (p, q) in a.points.iter().zip(&b.points) {
      assert_eq!(p.f_hz, q.f_hz);
      assert!((p.re - q.re).abs() < 1e-9 && (p.im - q.im).abs() < 1e-9);
    }
  }

  #[test]
  fn nanovna_echo_and_prompt_are_stripped() {
    let freq = "frequencies\r\n7000000\r\n7100000\r\n7200000\r\nch> ";
    let data = "data 0\r\n0.2 -0.1\r\n0.01 0.0\r\n0.3 0.2\r\nch> ";
    assert!(shell_done(freq) && shell_done(data));
    assert!(!shell_done("frequencies\r\n7000000\r\n"));
    let s = nanovna_sweep(freq, data, 50.0).unwrap();
    assert_eq!(s.points.len(), 3);
    assert_eq!(s.points[1].f_hz, 7.1e6);
    assert_eq!((s.points[2].re, s.points[2].im), (0.3, 0.2));
    assert_eq!(
      nanovna_sweep(freq, "data 0\r\n0 0\r\nch> ", 50.0),
      Err(ParseError::LengthMismatch(3, 1))
    );
    assert_eq!(
      nanovna_sweep(freq, "data 0\r\n0 0\r\n1\r\n0 0\r\nch> ", 50.0),
      Err(ParseError::BadNumber(2))
    );
  }

  #[test]
  fn min_swr_and_band_edges_interpolate_between_points() {
    // 构造一条 SWR 已知的扫频：在 7.0–7.3 MHz 上 r 从 2 → 1 → 2（x = 0），
    // 对应 SWR 2 → 1 → 2。SWR = r（r ≥ 1 时），所以 SWR ≤ 1.5 的边沿可以手算。
    let rs = [2.0, 1.5, 1.0, 1.5, 2.0];
    let points: Vec<S11Point> = rs
      .iter()
      .enumerate()
      .map(|(i, &r)| {
        let (re, im) = gamma_of(r, 0.0);
        S11Point {
          f_hz: 7.0e6 + i as f64 * 0.075e6,
          re,
          im,
        }
      })
      .collect();
    let s = Sweep { z0: 50.0, points };
    assert_eq!(min_swr_index(&s), Some(2));
    assert!((s.points[2].swr() - 1.0).abs() < 1e-9);
    // SWR ≤ 1.5：恰好落在第 1、3 个点上。
    let (lo, hi) = swr_band(&s, 1.5).unwrap();
    assert!(
      (lo - 7.075e6).abs() < 1.0 && (hi - 7.225e6).abs() < 1.0,
      "{lo} {hi}"
    );
    // SWR ≤ 1.75：在 1.5 与 2.0 之间插值到一半。
    let (lo, hi) = swr_band(&s, 1.75).unwrap();
    assert!(
      (lo - 7.0375e6).abs() < 1.0 && (hi - 7.2625e6).abs() < 1.0,
      "{lo} {hi}"
    );
    // 一直不超标：边沿就是扫频边界。
    assert_eq!(swr_band(&s, 3.0), Some((7.0e6, 7.3e6)));
    // 最低点也超标：没有可用频段。
    assert_eq!(swr_band(&s, 0.9), None);
  }
}
