//! 真实短波气象传真录音（`samples/`，来源与加工方式见 `samples/README.md`）。
//!
//! 页面只给了录音、没有给出解码后的图像，所以这里断言的是**结构**：相位线、行周期、黑白电平、
//! 以及图里确实有该有的东西（黑块、图头的白字、相邻行的连贯）——而不是逐像素对错。
//! 解出的图能不能读懂（DWD 那幅是带文字与等压线的地面预报图），开发时人工看过。

use ham_web_wefax::WefaxImage;
use ham_web_wefax::fax::{Decoded, Ioc, LevelSource, Params, decode};

fn load(name: &str, lpm: u32) -> Decoded {
  let path = format!("{}/samples/{name}", env!("CARGO_MANIFEST_DIR"));
  let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("读不到样本 {path}：{e}"));
  decode(
    &bytes,
    Params {
      lpm,
      ioc: Ioc::I576,
    },
  )
  .unwrap_or_else(|e| panic!("{name} 解不出来：{e}"))
}

fn mean(img: &WefaxImage) -> f64 {
  img.gray.iter().map(|&v| f64::from(v)).sum::<f64>() / img.gray.len() as f64
}

/// 每行里离 `near` 最近的一段 6–20 像素宽的暗条（灰度 > 180）的起点；没有就是 `None`。
///
/// DWD 图的左边有一条竖直的黑边：行周期准，各行的位置就一样；不准，每行错开几个像素。
fn bar_positions(img: &WefaxImage, near: usize) -> Vec<Option<usize>> {
  (0..img.height)
    .map(|r| {
      let row = &img.gray[r * img.width..(r + 1) * img.width];
      let mut best: Option<usize> = None;
      let mut c = 0;
      while c < img.width {
        if row[c] > 180 {
          let start = c;
          while c < img.width && row[c] > 180 {
            c += 1;
          }
          if (6..=20).contains(&(c - start))
            && best.is_none_or(|b| start.abs_diff(near) < b.abs_diff(near))
          {
            best = Some(start);
          }
        } else {
          c += 1;
        }
      }
      best.filter(|b| b.abs_diff(near) <= 20)
    })
    .collect()
}

#[test]
fn northwood_intro_has_the_phasing_and_nominal_levels() {
  let d = load("northwood_intro_8k.wav", 120);
  assert!(
    matches!(d.phasing_lines, Some(n) if n >= 35),
    "{:?}",
    d.phasing_lines
  );
  assert_eq!(d.levels, LevelSource::Phasing);
  assert!((d.black_hz - 1500.0).abs() < 10.0, "黑 {}", d.black_hz);
  assert!((d.white_hz - 2300.0).abs() < 10.0, "白 {}", d.white_hz);
  assert!((d.lpm_measured - 120.0).abs() < 0.03, "{}", d.lpm_measured);
  // 相位线之后只剩几行图。
  assert!((10..=20).contains(&d.image.height), "{}", d.image.height);
}

#[test]
fn northwood_data_has_no_phasing_and_a_solid_black_block() {
  let d = load("northwood_data_8k.wav", 120);
  assert_eq!(d.phasing_lines, None);
  assert_eq!(d.levels, LevelSource::Nominal);
  assert!((d.lpm_measured - 120.0).abs() < 0.03, "{}", d.lpm_measured);
  assert_eq!(d.image.height, 71);
  // 绝大部分是白底。
  assert!(mean(&d.image) < 60.0, "平均灰度 {}", mean(&d.image));
  // 图里有一块实心黑块（页面缩略图里可见的两条黑杠）：找一个 12 列 × 4 行、平均 > 230 的窗口。
  let img = &d.image;
  let found = (0..img.height - 4).any(|r| {
    (0..img.width - 12).step_by(4).any(|c| {
      let sum: u32 = (r..r + 4)
        .flat_map(|rr| (c..c + 12).map(move |cc| u32::from(img.gray[rr * img.width + cc])))
        .sum();
      f64::from(sum) / 48.0 > 230.0
    })
  });
  assert!(found, "没找到实心黑块");
}

#[test]
fn argentina_header_is_white_text_on_a_black_band_at_240_lpm() {
  // 页面上的 240 LPM 示例：图头是黑底白字的一行「日期 时间 SMN - Argentina」。
  let d = load("fax240_8k.wav", 240);
  assert_eq!(d.phasing_lines, None, "黑底白字的图头不能被当成相位线");
  assert!((d.lpm_measured - 240.0).abs() < 0.06, "{}", d.lpm_measured);
  assert!((50..=54).contains(&d.image.height), "{}", d.image.height);
  let img = &d.image;
  // 找一段连续 ≥ 5 行：每行多数是黑、又有一小撮白（字）。
  let band_rows: Vec<bool> = (0..img.height)
    .map(|r| {
      let row = &img.gray[r * img.width..(r + 1) * img.width];
      let dark = row.iter().filter(|&&v| v > 200).count() as f64 / img.width as f64;
      let light = row.iter().filter(|&&v| v < 80).count() as f64 / img.width as f64;
      dark > 0.7 && light > 0.02
    })
    .collect();
  let longest = band_rows
    .split(|&b| !b)
    .map(<[bool]>::len)
    .max()
    .unwrap_or(0);
  assert!(longest >= 5, "黑底白字的图头只有 {longest} 行");
}

#[test]
fn dwd_chart_is_a_narrow_band_recording_with_clock_drift() {
  let d = load("dwd_chart_8k.wav", 120);
  assert!(
    matches!(d.phasing_lines, Some(n) if n >= 55),
    "{:?}",
    d.phasing_lines
  );
  assert_eq!(d.levels, LevelSource::Phasing);
  // 接收机带宽被削窄：黑 ≈ 1747 Hz、白 ≈ 2051 Hz，不是标称的 1500 / 2300。
  assert!((d.black_hz - 1747.0).abs() < 20.0, "黑 {}", d.black_hz);
  assert!((d.white_hz - 2051.0).abs() < 20.0, "白 {}", d.white_hz);
  // 发收时钟差约 0.18%（120.22 LPM）：不修的话图每行错开约 3 个像素。
  assert!(
    (d.lpm_measured - 120.219).abs() < 0.03,
    "{}",
    d.lpm_measured
  );
  assert!((20..=30).contains(&d.image.height), "{}", d.image.height);
  // 左边那条竖直的黑边（第 91–100 列）在各行里的位置：斜率修正到位，它就是一条竖线（实测各行相差
  // ≤ 1 个像素）；不修，25 行会斜出 75 个像素。
  let found: Vec<usize> = bar_positions(&d.image, 95).into_iter().flatten().collect();
  assert!(found.len() >= 15, "只有 {} 行里找到黑边", found.len());
  let (lo, hi) = (found.iter().min().unwrap(), found.iter().max().unwrap());
  assert!(hi - lo <= 3, "黑边在各行里的位置从 {lo} 漂到 {hi}");
}
