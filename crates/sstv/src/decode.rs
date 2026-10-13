//! 行同步 + 采样 + 颜色解码 + 图像组装。
//!
//! 行结构由 [`Mode::scans`] 描述（相对行同步脉冲起点的扫描段表），所以 Martin（同步在行首）、
//! Scottie（同步在 **蓝之后、红之前**）、Robot（Y + 色差，Robot 36 每行只发一路交替色度）都走
//! 同一套代码：找到每行的同步脉冲，再按表在固定偏移处采样。

use super::modes::{ColorKind, Comp, Mode, ROBOT_36_SEPARATOR, Scan};
use super::vis::median_filter;
use super::{BLACK_HZ, Error, WHITE_HZ, WORK_RATE};

/// 解码出的 SSTV 图像。
#[derive(Debug, Clone, PartialEq)]
pub struct SstvImage {
  pub width: u32,
  pub height: u32,
  /// 模式名。
  pub mode: String,
  /// RGBA 像素（height × width × 4，行优先）。
  pub rgba: Vec<u8>,
  pub source_sample_rate: u32,
  pub duration_seconds: f64,
}

/// 同步脉冲下降沿的半高阈值（Hz）：像素频率范围 1500 ~ 2300 Hz，同步是 1200 Hz，取两者之间。
const SYNC_EDGE_HZ: f32 = 1700.0;

/// 找所有同步脉冲的下降沿位置（样本，浮点）：频率由 > 阈值穿越到 < 阈值，且随后至少 80% 的
/// 同步时长内都低于 1500 Hz（排除图像内容里短暂掠过阈值的像素）。
///
/// **不用「落在 1130 ~ 1260 Hz 内的连续段」定位**：FM 解调后同步脉冲的边沿是缓变的，低端还有
/// 振铃（实测 1041 ~ 1325 Hz 来回摆），连续段会被振铃截短 / 打断，起点漂移约 4 样本（RMS）；
/// 半高点穿越只取决于边沿本身，同一份真实录音上 RMS 残差只有约 0.6 样本。返回的是**半高点**，
/// 不是脉冲的物理起点，每模式的经验偏移见 [`Mode::edge_ms`]。
fn sync_edges(m: &[f32], sync_len: f64) -> Vec<f64> {
  let w = (sync_len * 0.6) as usize;
  let mut out = Vec::new();
  if w == 0 || m.len() <= w + 1 {
    return out;
  }
  for i in 1..m.len() - w {
    if m[i - 1] >= SYNC_EDGE_HZ && m[i] < SYNC_EDGE_HZ {
      let low = m[i..i + w].iter().filter(|&&f| f < 1500.0).count();
      if low as f64 >= w as f64 * 0.8 {
        // 线性插值到阈值穿越的亚样本位置。
        let frac = f64::from(m[i - 1] - SYNC_EDGE_HZ) / f64::from(m[i - 1] - m[i]);
        out.push(i as f64 - 1.0 + frac);
      }
    }
  }
  out
}

/// 毫秒 → 样本数（浮点：同步 / 扫描段的起点不必落在整数样本上）。
fn ms(v: f64) -> f64 {
  v * f64::from(WORK_RATE) / 1000.0
}

/// 行起点的直线模型：第 `k` 行的同步起点 = `t0 + k × period`（样本，浮点）。
#[derive(Debug, Clone, Copy, PartialEq)]
struct LineModel {
  t0: f64,
  period: f64,
}

impl LineModel {
  fn at(&self, k: usize) -> f64 {
    self.t0 + k as f64 * self.period
  }
}

/// Theil–Sen 稳健直线拟合：斜率 = 所有点对斜率的中位数，截距 = 残差中位数。
///
/// 点数最多约 256，点对 3 万量级，开销可忽略；对不超过约 29 % 的离群点有抵抗力。
fn theil_sen(points: &[(f64, f64)]) -> (f64, f64) {
  let mut slopes = Vec::with_capacity(points.len() * points.len() / 2);
  for (i, p) in points.iter().enumerate() {
    for q in &points[i + 1..] {
      if (q.0 - p.0).abs() > 0.5 {
        slopes.push((q.1 - p.1) / (q.0 - p.0));
      }
    }
  }
  if slopes.is_empty() {
    return (0.0, 0.0);
  }
  let median = |v: &mut Vec<f64>| {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
  };
  let b = median(&mut slopes);
  let mut resid: Vec<f64> = points.iter().map(|p| p.1 - b * p.0).collect();
  (median(&mut resid), b)
}

/// 稳健拟合行起点直线。
///
/// 每个下降沿按「最像第几行」归到行号 `k`，得到残差 `e − (first + k × 标称行长)`；再对残差做
/// `a + b·k` 的最小二乘并**迭代剔除离群点**（图像里碰巧出现的同步样内容、噪声假沿）。结果
/// `period = 标称行长 + b`，`b` 就是发射机与声卡之间的时钟误差。
///
/// 为什么不用「上一行锁定点 + 标称行长」的链式推算：一次被假同步骗走的锁定会带偏之后所有行
/// （真实 Martin M1 录音上实测 42 / 256 行锁定偏差超过 1 ms，最大约 8 ms，图像大片水平撕裂）。
/// 直线由全部可信点拟合，单个坏点影响不了它。
///
/// 可信点太少（< 8 个）、覆盖不到四分之一的行、或时钟误差大得离谱（> 1 %）时，拟合没有
/// 意义，退回标称行长。
fn fit_line_model(edges: &[f64], first: f64, line_len: f64, height: usize) -> LineModel {
  let fallback = LineModel {
    t0: first,
    period: line_len,
  };
  // 第一个下降沿之前的都不是图像行（可能是 VIS 尾巴）。
  let mut points: Vec<(f64, f64)> = edges
    .iter()
    .filter_map(|&e| {
      let k = ((e - first) / line_len).round();
      (0.0..height as f64)
        .contains(&k)
        .then_some((k, e - (first + k * line_len)))
    })
    .collect();
  if points.len() < 8 {
    return fallback;
  }
  // 容差 1.5 ms（Martin 约 3 个像素）：真实时钟漂移在整幅图内只有几 ms，由直线吸收，不属于离群。
  let tol = ms(1.5);
  // 首轮不能直接对全部点做最小二乘：普通最小二乘对离群点没有任何抵抗力，一批偏离几百样本的
  // 假沿会把直线拖偏，随后「用偏直线判内点」把真点也全剔掉。改用 Theil–Sen（斜率取所有点对
  // 斜率的**中位数**）起步，再以它的残差中位数定截距——单个假沿对中位数几乎没有影响。
  let (mut a, mut b) = theil_sen(&points);
  let mut keep: Vec<(f64, f64)> = points
    .iter()
    .copied()
    .filter(|&(k, r)| (r - (a + b * k)).abs() < tol)
    .collect();
  if keep.len() < 8 {
    return fallback;
  }
  // 内点集确定后再用最小二乘精修（此时已无离群点，最小二乘是最优的）。
  for _ in 0..6 {
    let n = keep.len() as f64;
    let (sk, sr) = keep.iter().fold((0.0, 0.0), |(x, y), p| (x + p.0, y + p.1));
    let (skk, skr) = keep
      .iter()
      .fold((0.0, 0.0), |(x, y), p| (x + p.0 * p.0, y + p.0 * p.1));
    let den = n * skk - sk * sk;
    if den.abs() < 1e-9 {
      return fallback;
    }
    b = (n * skr - sk * sr) / den;
    a = (sr - b * sk) / n;
    let next: Vec<(f64, f64)> = points
      .iter()
      .copied()
      .filter(|&(k, r)| (r - (a + b * k)).abs() < tol)
      .collect();
    if next.len() < 8 {
      break;
    }
    let settled = next.len() == keep.len();
    keep = next;
    if settled {
      break;
    }
  }
  points.clear();
  let rows: std::collections::BTreeSet<i64> = keep.iter().map(|p| p.0 as i64).collect();
  if rows.len() * 4 < height || b.abs() > line_len * 0.01 {
    return fallback;
  }
  LineModel {
    t0: first + a,
    period: line_len + b,
  }
}

/// 取 `[a, b)` 内的平均频率；先把每个样本夹到 1400 ~ 2400 Hz，免得 FM 解调的尖峰拖偏均值。
fn mean_clamped(freq: &[f32], a: f64, b: f64) -> f32 {
  let from = a.round().max(0.0) as usize;
  let to = (b.round().max(a.round() + 1.0) as usize).min(freq.len());
  if from >= to {
    return BLACK_HZ;
  }
  freq[from..to]
    .iter()
    .map(|f| f.clamp(1400.0, 2400.0))
    .sum::<f32>()
    / (to - from) as f32
}

/// 频率 → 8 位编码值（1500 Hz = 0，2300 Hz = 255）。亮度与色差共用这一映射，色差的零点是 128
/// （= 1900 Hz），不是 0。
fn code(freq: f32) -> u8 {
  (((freq - BLACK_HZ) / (WHITE_HZ - BLACK_HZ)).clamp(0.0, 1.0) * 255.0).round() as u8
}

/// 采样一个扫描段：返回 `pixels` 个 8 位编码值。
fn sample_scan(freq: &[f32], anchor: f64, scan: &Scan) -> Vec<u8> {
  let t0 = anchor + ms(scan.start_ms);
  let step = ms(scan.len_ms) / scan.pixels as f64;
  (0..scan.pixels)
    .map(|i| {
      code(mean_clamped(
        freq,
        t0 + i as f64 * step,
        t0 + (i + 1) as f64 * step,
      ))
    })
    .collect()
}

/// 一行的采样结果。
#[derive(Default)]
struct Line {
  r: Vec<u8>,
  g: Vec<u8>,
  b: Vec<u8>,
  y: Vec<u8>,
  ry: Vec<u8>,
  by: Vec<u8>,
  /// Robot 36：这一行的交替色度是 B-Y（奇行）还是 R-Y（偶行）。
  chroma_is_by: bool,
}

fn read_line(freq: &[f32], anchor: f64, mode: &Mode) -> Line {
  let mut line = Line::default();
  for scan in mode.scans {
    let px = sample_scan(freq, anchor, scan);
    match scan.comp {
      Comp::R => line.r = px,
      Comp::G => line.g = px,
      Comp::B => line.b = px,
      Comp::Y => line.y = px,
      Comp::Ry => line.ry = px,
      Comp::By => line.by = px,
      Comp::ChromaAlt => {
        // 行内的分隔脉冲：1500 Hz = 偶行（R-Y），2300 Hz = 奇行（B-Y）。比逐行数奇偶更稳——
        // 丢了一行也不会让之后所有行的色度配反。
        let (sep_ms, sep_len) = ROBOT_36_SEPARATOR;
        let sep = mean_clamped(
          freq,
          anchor + ms(sep_ms + 0.5),
          anchor + ms(sep_ms + sep_len - 0.5),
        );
        line.chroma_is_by = sep > 1900.0;
        if line.chroma_is_by {
          line.by = px;
        } else {
          line.ry = px;
        }
      }
    }
  }
  line
}

/// 视频范围 BT.601 的 YCbCr → RGB（Robot 的 Y 以 16 为黑电平，与 Robot36 / MMSSTV 一致）。
fn ycc_to_rgb(y: u8, ry: u8, by: u8) -> [u8; 3] {
  let c = i32::from(y) - 16;
  let v = i32::from(ry) - 128;
  let u = i32::from(by) - 128;
  let clamp = |x: i32| (x >> 8).clamp(0, 255) as u8;
  [
    clamp(298 * c + 409 * v + 128),
    clamp(298 * c - 100 * u - 208 * v + 128),
    clamp(298 * c + 516 * u + 128),
  ]
}

fn push_rgb_row(rgba: &mut Vec<u8>, width: usize, px: impl Fn(usize) -> [u8; 3]) {
  for col in 0..width {
    let [r, g, b] = px(col);
    rgba.extend_from_slice(&[r, g, b, 255]);
  }
}

/// 色度（水平分辨率减半）取第 `col` 列对应的值；缺失时给中性灰 128。
fn chroma_at(plane: &[u8], col: usize) -> u8 {
  plane.get(col / 2).copied().unwrap_or(128)
}

/// 从 VIS 结束位置起，逐行锁定同步、采样并组装图像。
pub fn assemble(
  freq: &[f32],
  mode: &Mode,
  vis_end: usize,
  source_rate: u32,
) -> Result<SstvImage, Error> {
  let sync_len = ms(mode.sync_ms);
  // 同步判定用中值滤波后的副本（去 FM 解调的单点尖峰），采样仍用原始 `freq`（不糊像素边沿）。
  let m = median_filter(freq, 9);
  let (_, hi_ms) = mode.extent_ms();
  let line_len = ms(mode.line_ms);

  // 第一行的锚点。停止位（1200 Hz）与第一行的同步脉冲（1200 Hz）**连成一段**，中间没有
  // 「> 1700 → < 1700」的下降沿，所以第一行同步没有独立的边沿可找：它的起点就是 VIS 的终点。
  // 此前用「VIS 之后的第一个下降沿」当第一行，实际抓到的是**第二行**，整幅图被丢掉第 0 行、
  // 垂直方向整体错位一行（真实 Martin M1 录音上联合扫描得到 dy = +1 时 MAE 24.3 → 13.6）。
  //
  // VIS 终点本身由起始位的上升沿推算（见 `vis::detect`），有几个样本的误差；这里用它只定
  // 第 0 行的粗位置，真正的行起点交给下面的直线拟合（由后面几百个精确的下降沿决定）。
  let edges = sync_edges(&m, sync_len);
  let first_anchor = vis_end as f64 + ms(mode.lead_in_ms);
  let line_model = fit_line_model(&edges, first_anchor, line_len, mode.height);
  let mut rgba: Vec<u8> = Vec::with_capacity(mode.width * mode.height * 4);
  let mut rows = 0usize;
  // Robot 36：偶行（R-Y）先存着，等到随后的奇行（B-Y）再一起输出两行。
  let mut pending: Option<Line> = None;
  let mut lines_read = 0usize;

  while lines_read < mode.height {
    // 拟合值是下降沿半高点的位置；换算到脉冲标称起点要加上该模式的经验偏移。
    let anchor = line_model.at(lines_read) - ms(mode.edge_ms);
    if anchor + ms(hi_ms) > freq.len() as f64 {
      break;
    }
    let line = read_line(freq, anchor, mode);
    lines_read += 1;
    match mode.color {
      ColorKind::Rgb => {
        push_rgb_row(&mut rgba, mode.width, |c| [line.r[c], line.g[c], line.b[c]]);
        rows += 1;
      }
      ColorKind::Yc => {
        push_rgb_row(&mut rgba, mode.width, |c| {
          ycc_to_rgb(line.y[c], chroma_at(&line.ry, c), chroma_at(&line.by, c))
        });
        rows += 1;
      }
      ColorKind::YcInterlaced => {
        if line.chroma_is_by {
          let even = pending.take();
          // 配对：偶行贡献 R-Y，奇行贡献 B-Y；两行 Y 各用各的。没有偶行就用中性 R-Y。
          let ry = even.as_ref().map(|e| e.ry.clone()).unwrap_or_default();
          if let Some(e) = &even {
            push_rgb_row(&mut rgba, mode.width, |c| {
              ycc_to_rgb(e.y[c], chroma_at(&ry, c), chroma_at(&line.by, c))
            });
            rows += 1;
          }
          push_rgb_row(&mut rgba, mode.width, |c| {
            ycc_to_rgb(line.y[c], chroma_at(&ry, c), chroma_at(&line.by, c))
          });
          rows += 1;
        } else {
          // 连续两个偶行：前一个没等到配对的奇行，只能用中性 B-Y 单独输出。
          if let Some(e) = pending.replace(line) {
            push_rgb_row(&mut rgba, mode.width, |c| {
              ycc_to_rgb(e.y[c], chroma_at(&e.ry, c), 128)
            });
            rows += 1;
          }
        }
      }
    }
  }
  // 收尾：最后一个没等到奇行的偶行。
  if let Some(e) = pending {
    push_rgb_row(&mut rgba, mode.width, |c| {
      ycc_to_rgb(e.y[c], chroma_at(&e.ry, c), 128)
    });
    rows += 1;
  }

  if rows < 2 {
    return Err(Error::TooShort);
  }
  Ok(SstvImage {
    width: mode.width as u32,
    height: rows as u32,
    mode: mode.name.to_owned(),
    rgba,
    source_sample_rate: source_rate,
    duration_seconds: freq.len() as f64 / f64::from(WORK_RATE),
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn code_maps_black_grey_white() {
    assert_eq!(code(1500.0), 0);
    assert_eq!(code(2300.0), 255);
    assert_eq!(code(1900.0), 128);
  }

  #[test]
  fn neutral_chroma_is_grey_and_video_levels_reach_full_range() {
    // 色差零点是 128（1900 Hz），不是 0；Y = 16 → 黑，Y = 235 → 白。
    assert_eq!(ycc_to_rgb(16, 128, 128), [0, 0, 0]);
    assert_eq!(ycc_to_rgb(235, 128, 128), [255, 255, 255]);
    let [r, g, b] = ycc_to_rgb(128, 128, 128);
    assert!(r.abs_diff(g) <= 1 && g.abs_diff(b) <= 1, "{r} {g} {b}");
  }

  #[test]
  fn red_difference_pushes_red_up_and_blue_difference_pushes_blue_up() {
    let [r, _, b] = ycc_to_rgb(128, 200, 128);
    // Y = 128 的中性灰是 130；R-Y 只抬高红，不动蓝。
    assert!(r > 200 && b <= 131, "{r} {b}");
    let [r, _, b] = ycc_to_rgb(128, 128, 200);
    assert!(b > 200 && r <= 131, "{r} {b}");
  }

  #[test]
  fn fitted_line_model_ignores_outliers_and_absorbs_clock_error() {
    // 100 行，名义行长 3000 样本，真实行长 3000.6（200 ppm），夹 6 个假沿。
    let (first, nominal) = (500.0, 3000.0);
    let mut edges: Vec<f64> = (0..100).map(|k| first + k as f64 * 3000.6).collect();
    for k in [3, 17, 40, 41, 77, 90] {
      edges.push(first + k as f64 * 3000.6 + 700.0); // 图像内容里的假「同步」
    }
    edges.sort_by(f64::total_cmp);
    let model = fit_line_model(&edges, first, nominal, 100);
    // 此前对全部点做最小二乘，6 个偏离 700 样本的假沿把直线拖偏（得到 3000.385），再用偏直线
    // 判内点又把真点剔光；Theil–Sen 起步后应精确恢复。
    assert!((model.period - 3000.6).abs() < 0.01, "{}", model.period);
    assert!((model.at(99) - (first + 99.0 * 3000.6)).abs() < 0.5);
  }

  #[test]
  fn too_few_edges_fall_back_to_the_nominal_line_length() {
    let model = fit_line_model(&[500.0, 3500.0], 500.0, 3000.0, 256);
    assert_eq!(
      model,
      LineModel {
        t0: 500.0,
        period: 3000.0
      }
    );
  }

  #[test]
  fn sync_edges_use_the_half_height_crossing_and_skip_brief_dips() {
    let mut m = vec![2000.0f32; 400];
    for f in &mut m[100..140] {
      *f = 1200.0; // 40 样本的同步
    }
    m[250] = 1000.0; // 单点毛刺：不是同步
    let e = sync_edges(&m, 40.0);
    assert_eq!(e.len(), 1, "{e:?}");
    assert!((e[0] - 99.0).abs() < 1.5, "{}", e[0]);
  }
}
