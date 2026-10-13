//! 真实录音回归：`samples/` 里的两份开源 SSTV 录音（来源与许可见 `samples/README.md`）。
//!
//! 这是本 crate 里**唯一不依赖自己编码器**的测试。此前所有测试都用自己的编码器生成信号，
//! 编码器与解码器共用同一份错误假设，所以全是绿的，而对真实录音一份也解不出（VIS 起始位
//! 300 ms、位极性写反、Robot / Scottie DX 时序错误）。
//!
//! 断言刻意用**统计量**而不是逐像素比对：真实录音带信道噪声，逐像素必然抖动；但结构性错误
//! （丢行、水平错位、颜色互换、色度配反）会让这些统计量大幅偏离。

use ham_web_sstv::{SstvImage, decode};

fn load(name: &str) -> SstvImage {
  let path = format!("{}/samples/{name}", env!("CARGO_MANIFEST_DIR"));
  let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("读取 {path} 失败：{e}"));
  decode(&bytes).unwrap_or_else(|e| panic!("{name} 解码失败：{e}"))
}

fn px(img: &SstvImage, x: usize, y: usize) -> [i32; 3] {
  let o = (y * img.width as usize + x) * 4;
  [
    i32::from(img.rgba[o]),
    i32::from(img.rgba[o + 1]),
    i32::from(img.rgba[o + 2]),
  ]
}

/// 一个矩形区域的平均 RGB。
fn mean(img: &SstvImage, x0: usize, x1: usize, y0: usize, y1: usize) -> [i32; 3] {
  let mut acc = [0i64; 3];
  let mut n = 0i64;
  for y in y0..y1 {
    for x in x0..x1 {
      let p = px(img, x, y);
      for c in 0..3 {
        acc[c] += i64::from(p[c]);
      }
      n += 1;
    }
  }
  [
    (acc[0] / n) as i32,
    (acc[1] / n) as i32,
    (acc[2] / n) as i32,
  ]
}

fn near(got: [i32; 3], want: [i32; 3], tol: i32) -> bool {
  (0..3).all(|c| (got[c] - want[c]).abs() <= tol)
}

#[test]
fn robot36_real_recording_is_identified_and_complete() {
  let img = load("robot36_wikipedia_fr.wav");
  assert_eq!(img.mode, "Robot 36");
  assert_eq!(
    (img.width, img.height),
    (320, 240),
    "应是完整的 320 × 240，不丢行"
  );
}

#[test]
fn robot36_has_no_edge_artifacts_and_the_colour_bars_are_in_order() {
  let img = load("robot36_wikipedia_fr.wav");
  // 白底（图像中部偏上，避开文字与 logo）：左右边缘都应是白，不能有绿边（旧实现把 Y 窗口整体
  // 错开了约 1.6 ms，最右 6 个像素读成白→绿的渐变）。
  //
  // 左侧从 x = 3 起算：这份真实录音最左 2 列本来就是暗的过渡（实测 x=0 约 (42,46,44)、x=1 约
  // (170,174,172)，x ≥ 2 才是干净的白），属于录音本身而不是解码缺陷。
  for (x0, x1) in [(3, 11), (312, 320)] {
    let m = mean(&img, x0, x1, 55, 95);
    assert!(
      near(m, [255, 255, 255], 20),
      "白底 x={x0}..{x1} 应接近白，得到 {m:?}"
    );
  }
  // 底部 / 顶部色条：灰 黄 青 绿 品红 红 蓝，各占约 1/7 宽（取每段中心）。
  let bars: [[i32; 3]; 7] = [
    [190, 190, 190],
    [255, 255, 0],
    [0, 255, 255],
    [0, 255, 0],
    [255, 0, 255],
    [255, 0, 0],
    [0, 0, 255],
  ];
  for (i, want) in bars.iter().enumerate() {
    let cx = (i * 320 + 160) / 7;
    let m = mean(&img, cx - 6, cx + 6, 8, 16); // 顶部色条
    assert!(
      near(m, *want, 70),
      "顶部第 {i} 条色条应约 {want:?}，得到 {m:?}"
    );
  }
  // 最右一条（蓝）必须一直到边缘：旧实现在这里是绿。
  let edge = mean(&img, 316, 320, 8, 16);
  assert!(
    edge[2] > 150 && edge[1] < 100,
    "最右色条应是蓝，得到 {edge:?}"
  );
}

#[test]
fn robot36_red_text_is_red_and_the_chroma_is_not_swapped() {
  let img = load("robot36_wikipedia_fr.wav");
  // 「SSTV TEST」红字所在区域（约 y 45..60、x 85..230）里最红的像素：R 高、G / B 低。
  let mut best = [0, 255, 255];
  for y in 44..62 {
    for x in 85..232 {
      let p = px(&img, x, y);
      if p[0] - p[1].max(p[2]) > best[0] - best[1].max(best[2]) {
        best = p;
      }
    }
  }
  assert!(
    best[0] > 180 && best[1] < 90 && best[2] < 90,
    "红字应是红色（R-Y / B-Y 没配反），得到 {best:?}"
  );
}

#[test]
fn martin_m1_real_recording_is_identified_and_complete() {
  let img = load("martin_m1_sunset.wav");
  assert_eq!(img.mode, "Martin M1");
  assert_eq!((img.width, img.height), (320, 256));
}

#[test]
fn martin_m1_sunset_has_the_right_layout_and_colours() {
  let img = load("martin_m1_sunset.wav");
  // 日落：橙色晚霞在 y ≈ 96..128、x ≈ 200..260（实测约 (238,209,188) / (229,172,120)），
  // 左下是近乎全黑的林木 / 水面。晚霞应明显偏暖（R > B）、比暗部亮得多；Martin 的颜色顺序是
  // G → B → R，顺序读错会让红蓝互换，晚霞变成偏蓝。
  let glow = mean(&img, 200, 260, 100, 124);
  let dark = mean(&img, 0, 60, 170, 250);
  let lum = |c: [i32; 3]| (c[0] * 299 + c[1] * 587 + c[2] * 114) / 1000;
  assert!(
    lum(glow) > lum(dark) + 120,
    "晚霞应远比暗部亮：晚霞 {glow:?} 暗部 {dark:?}"
  );
  assert!(
    glow[0] > glow[2] + 25,
    "晚霞应偏暖（R > B），得到 {glow:?}（G/B/R 顺序错会变成偏蓝）"
  );
}

#[test]
fn martin_m1_line_sync_is_stable() {
  // 度量行同步稳定性，不依赖图里有什么物体：相邻两行的内容高度相似（垂直方向很平滑），如果
  // 每行同步都对准，两行的最佳水平对齐位移应集中在 0；链式重锁被假同步骗走时，位移会散开。
  // 逐行亮度剖面取一阶差分再互相关：对缓变的亮度基线不敏感，只看边缘位置。
  let img = load("martin_m1_sunset.wav");
  let w = img.width as usize;
  let profile = |y: usize| -> Vec<f64> {
    let lum: Vec<f64> = (0..w)
      .map(|x| {
        let p = px(&img, x, y);
        f64::from(p[0] * 299 + p[1] * 587 + p[2] * 114) / 1000.0
      })
      .collect();
    lum.windows(2).map(|d| d[1] - d[0]).collect()
  };
  let mut zero = 0usize;
  let mut within_one = 0usize;
  let mut total = 0usize;
  for y in (20..230).step_by(2) {
    let (a, b) = (profile(y), profile(y + 1));
    let mut best = (f64::MIN, 0i32);
    for shift in -6i32..=6 {
      let dot: f64 = (8..a.len() - 8)
        .map(|x| a[x] * b[(x as i32 + shift) as usize])
        .sum();
      if dot > best.0 {
        best = (dot, shift);
      }
    }
    total += 1;
    if best.1 == 0 {
      zero += 1;
    }
    if best.1.abs() <= 1 {
      within_one += 1;
    }
  }
  // 修复前（链式重锁）：相邻行位移为 0 的比例明显更低（实测整图 |位移| ≤ 1 的行只有 69%，
  // 现在 97%）。阈值留余量，只拦结构性退化。
  assert!(
    within_one * 100 >= total * 85,
    "相邻两行 |对齐位移| ≤ 1 像素的只有 {within_one}/{total}（位移 = 0 的 {zero}）"
  );
}
