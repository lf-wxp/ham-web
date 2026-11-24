//! 合成往返 + 规格对账。
//!
//! 合成信号由**测试内独立写的编码器**按规格文本生成（头部、Martin / Scottie 的分量顺序与同步
//! 位置、Robot 36 的交替色度），**不读** `Mode::scans`：编码器与解码器各自对着规格实现，
//! 才能互相检验。此前两者共用同一份错误假设（VIS 起始位 300 ms、位极性写反、Robot / Scottie
//! DX 时序错误），自洽测试一直是绿的，对真实录音却一份也解不出。
//!
//! 局限（如实）：合成信号没有真实 FM 解调的边沿过渡，对 `Mode::edge_ms` 这类经验偏移不敏感，
//! 所以**它标定不了偏移，只能验证结构**（头部、行长、分量顺序、色度配对、时钟误差吸收）。
//! 偏移的标定与回归见 `real_recordings.rs`。

use std::f32::consts::TAU;

use ham_web_sstv::{EdgeEvidence, MODES, Mode, WORK_RATE, decode_samples};

const BLACK: f32 = 1500.0;
const WHITE: f32 = 2300.0;

struct Tones {
  freq: Vec<f32>,
  elapsed_ms: f64,
  /// 时钟误差：每毫秒实际占用的样本数缩放（1 + ppm × 1e-6）。
  scale: f64,
}

impl Tones {
  fn new(ppm: f64) -> Self {
    Self {
      freq: Vec::new(),
      elapsed_ms: 0.0,
      scale: 1.0 + ppm * 1e-6,
    }
  }

  fn push(&mut self, hz: f32, ms: f64) {
    self.elapsed_ms += ms * self.scale;
    let target = (self.elapsed_ms / 1000.0 * f64::from(WORK_RATE)).round() as usize;
    let n = target.saturating_sub(self.freq.len());
    self.freq.extend(std::iter::repeat_n(hz, n));
  }

  fn scan(&mut self, len_ms: f64, pixels: usize, level: impl Fn(usize) -> f32) {
    let per = len_ms / pixels as f64;
    for i in 0..pixels {
      self.push(level(i), per);
    }
  }
}

/// 图像：每个分量一条独立的渐变，便于发现分量顺序错误。
#[derive(Clone, Copy)]
enum Pattern {
  /// 左 → 右灰度渐变，R = G = B。
  Gray,
  /// R 通道全黑、G 通道全白、B 通道中灰：分量顺序错了会立刻变色。
  RgbSplit,
}

fn level(pattern: Pattern, comp: char, col: usize, w: usize) -> f32 {
  let ramp = BLACK + (col as f32 / (w - 1) as f32) * (WHITE - BLACK);
  match pattern {
    Pattern::Gray => ramp,
    Pattern::RgbSplit => match comp {
      'R' => BLACK,
      'G' => WHITE,
      _ => 1900.0,
    },
  }
}

fn header(t: &mut Tones, vis_code: u8) {
  t.push(1900.0, 300.0);
  t.push(1200.0, 10.0);
  t.push(1900.0, 300.0);
  t.push(1200.0, 30.0);
  let full = vis_code | (((vis_code.count_ones() % 2) as u8) << 7);
  for b in 0..8 {
    t.push(if (full >> b) & 1 == 1 { 1100.0 } else { 1300.0 }, 30.0);
  }
  t.push(1200.0, 30.0);
}

fn synth(mode: &Mode, pattern: Pattern, ppm: f64, rows: usize) -> Vec<f32> {
  let mut t = Tones::new(ppm);
  header(&mut t, mode.vis_code);
  let w = mode.width;
  match mode.name {
    "Martin M1" | "Martin M2" => {
      let scan = if mode.name == "Martin M1" {
        146.432
      } else {
        73.216
      };
      for _ in 0..rows {
        t.push(1200.0, 4.862);
        t.push(BLACK, 0.572);
        for comp in ['G', 'B', 'R'] {
          t.scan(scan, w, |c| level(pattern, comp, c, w));
          t.push(BLACK, 0.572);
        }
      }
    }
    "Scottie S1" | "Scottie S2" | "Scottie DX" => {
      let scan = match mode.name {
        "Scottie S1" => 138.24,
        "Scottie S2" => 88.064,
        _ => 345.6,
      };
      t.push(1200.0, 9.0); // 整幅图开头多一个同步
      for _ in 0..rows {
        t.push(BLACK, 1.5);
        t.scan(scan, w, |c| level(pattern, 'G', c, w));
        t.push(BLACK, 1.5);
        t.scan(scan, w, |c| level(pattern, 'B', c, w));
        t.push(1200.0, 9.0); // 同步：蓝之后、红之前
        t.push(BLACK, 1.5);
        t.scan(scan, w, |c| level(pattern, 'R', c, w));
      }
    }
    "Robot 36" => {
      for row in 0..rows {
        t.push(1200.0, 9.0);
        t.push(BLACK, 3.0);
        t.scan(88.0, w, |c| level(pattern, 'Y', c, w));
        t.push(if row % 2 == 0 { 1500.0 } else { 2300.0 }, 4.5);
        t.push(1900.0, 1.5);
        t.scan(44.0, w / 2, |_| 1900.0);
      }
    }
    "Robot 72" => {
      for _ in 0..rows {
        t.push(1200.0, 9.0);
        t.push(BLACK, 3.0);
        t.scan(138.0, w, |c| level(pattern, 'Y', c, w));
        t.push(BLACK, 4.5);
        t.push(1900.0, 1.5);
        t.scan(69.0, w / 2, |_| 1900.0);
        t.push(BLACK, 4.5);
        t.push(1900.0, 1.5);
        t.scan(69.0, w / 2, |_| 1900.0);
      }
    }
    other => panic!("没有为 {other} 写编码器"),
  }
  // FM 调制。
  let mut out = Vec::with_capacity(t.freq.len());
  let mut phase = 0.0f32;
  for &f in &t.freq {
    out.push(phase.sin());
    phase = (phase + TAU * f / WORK_RATE as f32) % TAU;
  }
  out
}

fn px(img: &ham_web_sstv::SstvImage, x: usize, y: usize) -> [i32; 3] {
  let o = (y * img.width as usize + x) * 4;
  [
    i32::from(img.rgba[o]),
    i32::from(img.rgba[o + 1]),
    i32::from(img.rgba[o + 2]),
  ]
}

fn mode(name: &str) -> &'static Mode {
  MODES.iter().find(|m| m.name == name).expect("模式存在")
}

// ── 规格对账 ──────────────────────────────────────────────────────────────

#[test]
fn whole_image_durations_match_the_published_values() {
  // 公开的整图时长（秒）：SimpleSSTV / SSTV Handbook / 各解码器文档一致。允许 ±1.5%：
  // 公开值多是取整的（Martin M1 实为 114.3 s，常写 114）。
  let published = [
    ("Martin M1", 114.0),
    ("Martin M2", 58.0),
    ("Scottie S1", 110.0),
    ("Scottie S2", 71.0),
    ("Scottie DX", 269.0),
    ("Robot 36", 36.0),
    ("Robot 72", 72.0),
  ];
  for (name, secs) in published {
    let m = mode(name);
    let total = m.line_ms * m.height as f64 / 1000.0;
    assert!(
      (total - secs).abs() / secs < 0.015,
      "{name}：按行长算出 {total:.1} s，公开值约 {secs} s（此前 Scottie DX 只有 44.8 s、Robot 72 只有 23.4 s）"
    );
  }
}

#[test]
fn every_mode_declares_how_it_was_verified() {
  for m in MODES {
    // 经验偏移只有 Martin M1 / Robot 36 有真实录音；其它模式不能冒充已校准。
    let calibrated = m.edge_evidence == EdgeEvidence::Calibrated;
    assert_eq!(
      calibrated,
      matches!(m.name, "Martin M1" | "Robot 36"),
      "{}",
      m.name
    );
  }
}

// ── 合成往返 ──────────────────────────────────────────────────────────────

#[test]
fn every_mode_decodes_a_gray_ramp_left_to_right() {
  for m in MODES {
    let rows = 40; // 够拟合直线、又不至于让 Scottie DX（每行 1 s）跑太久
    let img = decode_samples(&synth(m, Pattern::Gray, 0.0, rows), WORK_RATE)
      .unwrap_or_else(|e| panic!("{} 解码失败：{e}", m.name));
    assert_eq!(img.mode, m.name);
    assert_eq!(img.width as usize, m.width);
    assert!(
      img.height as usize >= rows - 2,
      "{}：只解出 {} 行（应约 {rows}）",
      m.name,
      img.height
    );
    let y = img.height as usize / 2;
    let (l, mid, r) = (
      px(&img, 12, y),
      px(&img, m.width / 2, y),
      px(&img, m.width - 12, y),
    );
    assert!(l[0] < 60, "{}：左端应偏黑，得到 {l:?}", m.name);
    assert!(
      (100..160).contains(&mid[0]),
      "{}：中点应约 128，得到 {mid:?}",
      m.name
    );
    assert!(r[0] > 195, "{}：右端应偏白，得到 {r:?}", m.name);
  }
}

#[test]
fn rgb_modes_keep_the_component_order() {
  // R 全黑 / G 全白 / B 中灰：Martin 的发送顺序是 G → B → R，Scottie 也是 G → B →（同步）→ R。
  // 顺序读错会让绿 / 红 / 蓝互换。旧实现按「R、G、B 逐像素交错」读，对真实信号全错。
  for name in ["Martin M1", "Martin M2", "Scottie S1", "Scottie S2"] {
    let m = mode(name);
    let img = decode_samples(&synth(m, Pattern::RgbSplit, 0.0, 30), WORK_RATE).expect("应解码");
    let p = px(&img, m.width / 2, img.height as usize / 2);
    assert!(p[0] < 50, "{name}：R 应近 0，得到 {p:?}");
    assert!(p[1] > 205, "{name}：G 应近 255，得到 {p:?}");
    assert!((90..170).contains(&p[2]), "{name}：B 应约 128，得到 {p:?}");
  }
}

#[test]
fn robot_chroma_pairs_even_and_odd_lines() {
  // Robot 36：偶行 R-Y、奇行 B-Y 配对成相邻两行。构造偶行 R-Y 高（红）、奇行 B-Y 中性的信号：
  // 配对后整幅都该偏红；若把 R-Y 当 B-Y 或配反，会偏蓝或灰。
  let m = mode("Robot 36");
  let mut t = Tones::new(0.0);
  header(&mut t, m.vis_code);
  for row in 0..40 {
    t.push(1200.0, 9.0);
    t.push(BLACK, 3.0);
    t.scan(88.0, m.width, |_| 1900.0); // 中灰 Y
    let even = row % 2 == 0;
    t.push(if even { 1500.0 } else { 2300.0 }, 4.5);
    t.push(1900.0, 1.5);
    // 偶行 R-Y = 2200 Hz（偏红），奇行 B-Y = 1900 Hz（中性）。
    t.scan(44.0, m.width / 2, |_| if even { 2200.0 } else { 1900.0 });
  }
  let mut out = Vec::new();
  let mut phase = 0.0f32;
  for &f in &t.freq {
    out.push(phase.sin());
    phase = (phase + TAU * f / WORK_RATE as f32) % TAU;
  }
  let img = decode_samples(&out, WORK_RATE).expect("应解码");
  let p = px(&img, m.width / 2, img.height as usize / 2);
  assert!(p[0] > p[2] + 40, "R-Y 高时应偏红，得到 {p:?}");
}

#[test]
fn line_fit_absorbs_a_sender_clock_error() {
  // ±500 ppm 的时钟误差：整幅图（Martin M1 256 行、约 115 s）累计偏差约 57 ms —— 超过 100 个
  // 像素。「每行 = 上一行 + 标称行长」会越漂越远，直线拟合应当把它完全吸收。
  let m = mode("Martin M1");
  for ppm in [500.0, -500.0] {
    let img = decode_samples(&synth(m, Pattern::Gray, ppm, 256), WORK_RATE)
      .unwrap_or_else(|e| panic!("{ppm} ppm 解码失败：{e}"));
    assert!(img.height >= 250, "{ppm} ppm：只解出 {} 行", img.height);
    // 最后一部分的行，渐变仍应在原位（左黑右白，中点约 128）。
    for y in [img.height as usize - 10, img.height as usize / 2] {
      let (l, mid, r) = (px(&img, 12, y), px(&img, 160, y), px(&img, 308, y));
      assert!(
        l[0] < 60 && r[0] > 195,
        "{ppm} ppm y={y}：渐变端点错位 {l:?} {r:?}"
      );
      assert!(
        (95..165).contains(&mid[0]),
        "{ppm} ppm y={y}：中点应约 128，得到 {mid:?}"
      );
    }
  }
}

#[test]
fn rejects_non_sstv_audio() {
  // 纯 1900 Hz 单音（无 VIS）。
  let n = WORK_RATE as usize;
  let samples: Vec<f32> = (0..n)
    .map(|i| (TAU * 1900.0 * i as f32 / WORK_RATE as f32).sin())
    .collect();
  assert!(decode_samples(&samples, WORK_RATE).is_err());
  assert!(decode_samples(&[], WORK_RATE).is_err());
}

#[test]
fn the_old_wrong_header_is_not_accepted() {
  // 旧实现把起始位当成 300 ms：用那种头部合成的信号**不应**再被识别成某个模式。
  let m = mode("Martin M1");
  let mut t = Tones::new(0.0);
  t.push(1200.0, 300.0); // 旧的「300 ms 起始位」
  let full = m.vis_code | (((m.vis_code.count_ones() % 2) as u8) << 7);
  for b in 0..8 {
    t.push(if (full >> b) & 1 == 1 { 1300.0 } else { 1100.0 }, 30.0); // 旧的位极性
  }
  t.push(1200.0, 30.0);
  for _ in 0..5 {
    t.push(1200.0, 4.862);
    t.scan(440.0, 960, |_| 1900.0);
  }
  let mut out = Vec::new();
  let mut phase = 0.0f32;
  for &f in &t.freq {
    out.push(phase.sin());
    phase = (phase + TAU * f / WORK_RATE as f32) % TAU;
  }
  assert!(decode_samples(&out, WORK_RATE).is_err());
}

// ---------- 库里的编码器（`ham_web_sstv::encode_image`） ----------
//
// 库里的编码器与上面这份测试内的编码器各自对着规格写；这里让它们互相对拍，再让解码器读回。

/// 一幅平滑的彩色测试图：红随列、绿随行变化，蓝取两者的混合；没有锐利边缘，
/// Robot 的色度二次采样不会被硬边放大成大误差。
fn colour_image(w: usize, h: usize) -> Vec<u8> {
  let mut rgb = Vec::with_capacity(w * h * 3);
  for y in 0..h {
    for x in 0..w {
      let r = x * 255 / (w - 1);
      let g = y * 255 / (h - 1);
      rgb.extend_from_slice(&[r as u8, g as u8, ((r + (255 - g)) / 2) as u8]);
    }
  }
  rgb
}

/// 每个模式：编码 → 解码，得到满高的图、模式名认对，平均误差很小，中心像素的颜色对得上。
///
/// 阈值取观测值的两倍多（Martin M1 约 0.5、Robot 36 约 1.7 灰阶）：足够小，通道顺序错了、
/// 色差零点错了、Y 的黑电平错了都会变成几十灰阶的误差。逐像素的最大误差不设限 ——
/// 左右两端的列在 FM 解调的边沿过渡里本来就偏得多（真实录音上也一样）。
#[test]
fn library_encoder_round_trips_a_colour_image_in_every_mode() {
  for m in MODES {
    let rgb = colour_image(m.width, m.height);
    let audio = ham_web_sstv::encode_image(m, &rgb).unwrap();
    let img = decode_samples(&audio, WORK_RATE).unwrap();
    assert_eq!(img.mode, m.name, "{}", m.name);
    assert_eq!(
      (img.width as usize, img.height as usize),
      (m.width, m.height),
      "{}",
      m.name
    );
    let mut sum = [0u64; 3];
    for y in 0..m.height {
      for x in 0..m.width {
        let got = px(&img, x, y);
        for c in 0..3 {
          sum[c] += u64::from(got[c].abs_diff(i32::from(rgb[3 * (y * m.width + x) + c])));
        }
      }
    }
    let n = (m.width * m.height) as f64;
    for (c, name) in ["R", "G", "B"].iter().enumerate() {
      let mean = sum[c] as f64 / n;
      assert!(mean < 3.0, "{} 通道 {name} 平均误差 {mean:.2}", m.name);
    }
    let (cx, cy) = (m.width / 2, m.height / 2);
    let got = px(&img, cx, cy);
    for c in 0..3 {
      let want = i32::from(rgb[3 * (cy * m.width + cx) + c]);
      assert!(
        (got[c] - want).abs() <= 12,
        "{} 中心像素通道 {c}: {} vs {want}",
        m.name,
        got[c]
      );
    }
  }
}

/// 与测试内的独立编码器对拍：5 个 RGB 模式的灰度渐变，两份编码器的输出交给同一个解码器，
/// 读回的图应当几乎相同。
///
/// - 两份音频的**时序逐样本一致**（库里的比测试里的多出末尾的 300 ms），只是两边的像素量化不同
///   （库里先取整到字节再映射频率，测试里直接用连续的渐变），正弦的相位随之漂开，所以不是逐样本相等；
/// - 只比去掉两端各 4 列的区域：每个分量扫描段末尾是 2300 Hz → 1500 Hz 的陡降，解码器的 Hilbert
///   解调在那里的边缘效应随相位而变（最右一列实测差到 50 多灰阶、次右列十几），对任何编码器、
///   对真实录音都一样，不是编码器的分歧；中间区域实测最大只差 4–5 灰阶（两边量化的差）；
/// - 只比两边都解出来的行：测试内的编码器让音频恰好在末行结束处截止，Scottie 会丢最后一行
///   （解码器的边缘行为，见库里编码器的 `TAIL_MS`）。
#[test]
fn library_encoder_agrees_with_the_independent_test_encoder_on_rgb_modes() {
  for name in [
    "Martin M1",
    "Martin M2",
    "Scottie S1",
    "Scottie S2",
    "Scottie DX",
  ] {
    let m = mode(name);
    let ramp: Vec<u8> = (0..m.width * m.height)
      .flat_map(|i| {
        let v = ((i % m.width) * 255 / (m.width - 1)) as u8;
        [v, v, v]
      })
      .collect();
    let ours = ham_web_sstv::encode_image(m, &ramp).unwrap();
    let theirs = synth(m, Pattern::Gray, 0.0, m.height);
    assert_eq!(
      ours.len(),
      theirs.len() + 2400,
      "{name}：时序应逐样本一致，只多 300 ms 的结尾"
    );
    let lib = decode_samples(&ours, WORK_RATE).unwrap();
    let other = decode_samples(&theirs, WORK_RATE).unwrap();
    assert_eq!(
      (lib.width, other.width),
      (m.width as u32, m.width as u32),
      "{name}"
    );
    let rows = lib.height.min(other.height) as usize;
    assert!(rows >= m.height - 1, "{name}：只对得上 {rows} 行");
    let mut diffs: Vec<i32> = (0..rows)
      .flat_map(|y| (4..m.width - 4).map(move |x| (x, y)))
      .flat_map(|(x, y)| {
        let (a, b) = (px(&lib, x, y), px(&other, x, y));
        [0, 1, 2].map(|c| (a[c] - b[c]).abs())
      })
      .collect();
    diffs.sort_unstable();
    let p99 = diffs[(diffs.len() - 1) * 99 / 100];
    let worst = diffs[diffs.len() - 1];
    // 差的大小取决于每像素有几个样本（8 kHz 下）：Scottie DX 约 8.6（实测最大差 2）、Martin M1 / Scottie S1
    // 约 3.5（4–5）、Martin M2 / Scottie S2 只有 1.8–2.2（13–14）—— 样本越少，解调对相位越敏感。
    // 分两档卡：绝大多数像素（p99）一律 ≤ 8；最大值在样本数少的两个快速模式上放宽到 16。
    // 分量顺序、同步位置、分隔写错会是几十灰阶，两档都抓得住。
    let samples_per_pixel = m.scans[0].len_ms / m.width as f64 * f64::from(WORK_RATE) / 1000.0;
    let limit = if samples_per_pixel >= 3.0 { 6 } else { 16 };
    assert!(p99 <= 8, "{name}：99% 分位的读回差 {p99}");
    assert!(
      worst <= limit,
      "{name}（{samples_per_pixel:.1} 样本/像素）：最大读回差 {worst}，上限 {limit}"
    );
  }
}

/// 整图时长对得上公开值（±1.5%，与 `whole_image_durations_match_the_published_values` 同一口径）；
/// 头部约 0.97 s、结尾 0.3 s 不计入整图时长，所以先扣掉再比。
#[test]
fn library_encoder_audio_length_matches_the_published_durations() {
  let published = [
    ("Martin M1", 114.0),
    ("Martin M2", 58.0),
    ("Scottie S1", 110.0),
    ("Scottie S2", 71.0),
    ("Scottie DX", 269.0),
    ("Robot 36", 36.0),
    ("Robot 72", 72.0),
  ];
  for (name, secs) in published {
    let m = mode(name);
    let audio = ham_web_sstv::encode_image(m, &vec![128; m.width * m.height * 3]).unwrap();
    let image = audio.len() as f64 / f64::from(WORK_RATE) - 0.97 - 0.3;
    assert!(
      (image - secs).abs() / secs < 0.015,
      "{name}：图像部分 {image:.1} s，公开值约 {secs} s"
    );
  }
}

#[test]
fn library_encoder_output_is_a_clean_unit_amplitude_signal() {
  let m = mode("Robot 36");
  let audio = ham_web_sstv::encode_image(m, &colour_image(m.width, m.height)).unwrap();
  assert!(audio.iter().all(|v| v.is_finite() && v.abs() <= 1.0));
  // 振幅 1 的正弦：峰值接近 1，不是被削过或缩小过。
  assert!(audio.iter().fold(0.0f32, |a, v| a.max(v.abs())) > 0.99);
}

#[test]
fn library_encoder_rejects_bad_input() {
  let m = mode("Martin M1");
  let ok = vec![0u8; m.width * m.height * 3];
  assert!(ham_web_sstv::encode_image(m, &ok).is_ok());
  for bad in [&ok[1..], &ok[..ok.len() - 3], &[][..]] {
    assert_eq!(
      ham_web_sstv::encode_image(m, bad),
      Err(ham_web_sstv::EncodeError::ImageSize)
    );
  }
  // 解码器认得、编码器没写的模式（这里用改了名字的副本模拟，如 PD120）：明确拒绝。
  let pd = Mode {
    name: "PD120",
    ..*m
  };
  assert!(!ham_web_sstv::has_encoder(&pd));
  assert_eq!(
    ham_web_sstv::encode_image(&pd, &ok),
    Err(ham_web_sstv::EncodeError::UnsupportedMode)
  );
  // 解码器的七个模式都有编码器：新增解码模式时忘了写编码器会在这里红。
  assert!(
    MODES.iter().all(ham_web_sstv::has_encoder),
    "有模式没有编码器"
  );
  assert_eq!(ham_web_sstv::EncodeError::ImageSize.code(), "image-size");
  assert_eq!(
    ham_web_sstv::EncodeError::UnsupportedMode.code(),
    "unsupported-mode"
  );
}
