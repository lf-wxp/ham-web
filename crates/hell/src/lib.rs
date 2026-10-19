//! Feld Hell（Feldhellschreiber）编码与解码：AM 点阵模式。
//!
//! 纯 Rust、无外部依赖，可同时用于 WASM 前端与命令行 / 测试。
//!
//! # 协议速览
//!
//! - **网格**：每字符 7 列 × 7 行（字库 5×7 嵌入 7×7，左右各空一列作字符间隔）；
//! - **传输顺序**：列左到右、列内行**自下而上**（bit 0 = 底行先发）；
//! - **调制**：AM 开关键控，载波 980 Hz 在「黑」像素处出现、「白」像素处静音；
//! - **速率**：2.5 字符/秒 → 122.5 像素/秒（每像素 1/122.5 ≈ 8.16 ms）。
//!
//! # 诚实说明
//!
//! 字库是**自绘 5×7 点阵**（`FONT`，非 fldigi 的 Hell 原字库），参数取公开常见的
//! Feld Hell 口径。往返合成对拍只证明「本实现自洽」；要解真实 Feld Hell 电台信号，
//! 需换成标准 Hell 字库并补字符列相位搜索（本实现假设字符边界对齐）。解调是
//! AM 包络 + 逐像素门限，未做时钟 / 载波恢复。

use std::f64::consts::TAU;

/// 载波频率（Hz）。
pub const CARRIER_HZ: f64 = 980.0;
/// 像素率（2.5 字符/秒 × 49 像素/字符）。
pub const PIXEL_HZ: f64 = 122.5;
/// 网格边长（7×7）。
pub const GRID: usize = 7;
/// 每字符像素数 = 7×7。
pub const PIXELS_PER_CHAR: usize = GRID * GRID;
/// 字库命中率下限：低于此值按「无信号」处理，不把噪声切出的乱码当解码结果。
const MIN_GLYPH_MATCH: f64 = 0.5;
/// 字符峰值低于整段峰值的这个比例时按「整字符空白」处理（见 [`decode_audio`]）。
const BLANK_RATIO: f64 = 0.15;

/// 每像素的**名义**采样数（四舍五入到整数）。
///
/// 只用于估算时长 / 缓冲大小，**不要**拿它逐像素累加来切片：8000/122.5 = 65.31，取整成
/// 65 会让像素率偏 +0.47%，一个字符（49 像素）就差 15 个采样，长报文会整体漂移。
/// 实际切分一律走 [`pixel_range`]。
#[must_use]
pub fn per_pixel(sample_rate: u32) -> usize {
  (sample_rate as f64 / PIXEL_HZ).round().max(1.0) as usize
}

/// 第 `index` 个像素在采样序列里的起止（右开区间）。
///
/// 按「像素边界 = 理想位置的最近整数」切分：相邻边界的取整误差互相抵消，累计偏差始终
/// ≤ 0.5 个采样，不会像整数点数那样随字符数累积。
fn pixel_range(index: usize, sample_rate: u32) -> (usize, usize) {
  let at = |i: usize| (i as f64 * f64::from(sample_rate) / PIXEL_HZ).round() as usize;
  let start = at(index);
  let end = at(index + 1);
  (start, end.max(start + 1))
}

/// 5×7 点阵字库（`.`=白、`#`=黑），行自上而下。
const FONT: &[(char, [&str; 7])] = &[
  (
    'A',
    [
      ".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#",
    ],
  ),
  (
    'B',
    [
      "####.", "#...#", "#...#", "####.", "#...#", "#...#", "####.",
    ],
  ),
  (
    'C',
    [
      ".###.", "#...#", "#....", "#....", "#....", "#...#", ".###.",
    ],
  ),
  (
    'D',
    [
      "####.", "#...#", "#...#", "#...#", "#...#", "#...#", "####.",
    ],
  ),
  (
    'E',
    [
      "#####", "#....", "#....", "####.", "#....", "#....", "#####",
    ],
  ),
  (
    'F',
    [
      "#####", "#....", "#....", "####.", "#....", "#....", "#....",
    ],
  ),
  (
    'G',
    [
      ".###.", "#...#", "#....", "#.###", "#...#", "#...#", ".###.",
    ],
  ),
  (
    'H',
    [
      "#...#", "#...#", "#...#", "#####", "#...#", "#...#", "#...#",
    ],
  ),
  (
    'I',
    [
      "#####", "..#..", "..#..", "..#..", "..#..", "..#..", "#####",
    ],
  ),
  (
    'J',
    [
      "#####", "...#.", "...#.", "...#.", "...#.", "#..#.", ".##..",
    ],
  ),
  (
    'K',
    [
      "#...#", "#..#.", "#.#..", "##...", "#.#..", "#..#.", "#...#",
    ],
  ),
  (
    'L',
    [
      "#....", "#....", "#....", "#....", "#....", "#....", "#####",
    ],
  ),
  (
    'M',
    [
      "#...#", "##.##", "#.#.#", "#.#.#", "#...#", "#...#", "#...#",
    ],
  ),
  (
    'N',
    [
      "#...#", "##..#", "#.#.#", "#..##", "#...#", "#...#", "#...#",
    ],
  ),
  (
    'O',
    [
      ".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###.",
    ],
  ),
  (
    'P',
    [
      "####.", "#...#", "#...#", "####.", "#....", "#....", "#....",
    ],
  ),
  (
    'Q',
    [
      ".###.", "#...#", "#...#", "#...#", "#.#.#", "#..#.", ".##.#",
    ],
  ),
  (
    'R',
    [
      "####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#",
    ],
  ),
  (
    'S',
    [
      ".###.", "#...#", "#....", ".###.", "....#", "#...#", ".###.",
    ],
  ),
  (
    'T',
    [
      "#####", "..#..", "..#..", "..#..", "..#..", "..#..", "..#..",
    ],
  ),
  (
    'U',
    [
      "#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###.",
    ],
  ),
  (
    'V',
    [
      "#...#", "#...#", "#...#", "#...#", "#...#", ".#.#.", "..#..",
    ],
  ),
  (
    'W',
    [
      "#...#", "#...#", "#...#", "#.#.#", "#.#.#", "##.##", "#...#",
    ],
  ),
  (
    'X',
    [
      "#...#", "#...#", ".#.#.", "..#..", ".#.#.", "#...#", "#...#",
    ],
  ),
  (
    'Y',
    [
      "#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#..",
    ],
  ),
  (
    'Z',
    [
      "#####", "....#", "...#.", "..#..", ".#...", "#....", "#####",
    ],
  ),
  (
    '0',
    [
      ".###.", "#...#", "#..##", "#.#.#", "##..#", "#...#", ".###.",
    ],
  ),
  (
    '1',
    [
      "..#..", ".##..", "..#..", "..#..", "..#..", "..#..", "#####",
    ],
  ),
  (
    '2',
    [
      ".###.", "#...#", "....#", "...#.", "..#..", ".#...", "#####",
    ],
  ),
  (
    '3',
    [
      "#####", "...#.", "..#..", "...#.", "....#", "#...#", ".###.",
    ],
  ),
  (
    '4',
    [
      "...#.", "..##.", ".#.#.", "#..#.", "#####", "...#.", "...#.",
    ],
  ),
  (
    '5',
    [
      "#####", "#....", "####.", "....#", "....#", "#...#", ".###.",
    ],
  ),
  (
    '6',
    [
      ".###.", "#....", "####.", "#...#", "#...#", "#...#", ".###.",
    ],
  ),
  (
    '7',
    [
      "#####", "....#", "...#.", "..#..", ".#...", ".#...", ".#...",
    ],
  ),
  (
    '8',
    [
      ".###.", "#...#", "#...#", ".###.", "#...#", "#...#", ".###.",
    ],
  ),
  (
    '9',
    [
      ".###.", "#...#", "#...#", ".####", "....#", "....#", ".###.",
    ],
  ),
  (
    ' ',
    [
      ".....", ".....", ".....", ".....", ".....", ".....", ".....",
    ],
  ),
  (
    '/',
    [
      "....#", "...#.", "...#.", "..#..", ".#...", "#....", "#....",
    ],
  ),
  (
    '-',
    [
      ".....", ".....", ".....", "#####", ".....", ".....", ".....",
    ],
  ),
  (
    '.',
    [
      ".....", ".....", ".....", ".....", ".....", ".##..", ".##..",
    ],
  ),
  (
    ',',
    [
      ".....", ".....", ".....", ".....", ".##..", ".##..", ".#...",
    ],
  ),
];

/// 7 行串 → 5 列字节（bit 0 = 底行，即行串的末行）。
fn columns(rows: [&str; 7]) -> [u8; 5] {
  let mut cols = [0u8; 5];
  for (row_idx, row) in rows.iter().enumerate() {
    for (col_idx, ch) in row.chars().enumerate() {
      // 字库是手绘表：多敲一个字符不能让解码端 panic，越界列直接忽略。表形由
      // `the_font_table_is_five_by_seven` 钉住，改字库时测试会先红。
      let Some(cell) = cols.get_mut(col_idx) else {
        continue;
      };
      if ch == '#' {
        *cell |= 1 << (6 - row_idx);
      }
    }
  }
  cols
}

/// 字符 → 7 列字节（5×7 嵌入 7×7，左右各空一列）。
fn glyph(c: char) -> Option<[u8; 7]> {
  let (_, rows) = FONT.iter().find(|(k, _)| *k == c)?;
  let c = columns(*rows);
  Some([0, c[0], c[1], c[2], c[3], c[4], 0])
}

/// 7 列字节 → 字符（找不到返回 `None`）。
fn match_glyph(g: [u8; 7]) -> Option<char> {
  FONT
    .iter()
    .find(|(_, rows)| {
      let c = columns(*rows);
      [0, c[0], c[1], c[2], c[3], c[4], 0] == g
    })
    .map(|(k, _)| *k)
}

/// 文本 → AM 基带采样（载波开关键控；未知字符跳过）。
#[must_use]
pub fn modulate(text: &str, sample_rate: u32) -> Vec<f32> {
  // 采样率为 0 时下面 `CARRIER_HZ / sample_rate` 除零、整段采样变 NaN，调用方拿到的是
  // 「成功但无声」的脏数据；与 `mode_audio` / `afsk1200` 一样显式拒绝。
  if sample_rate == 0 {
    return Vec::new();
  }
  let mut out = Vec::new();
  let mut phase = 0.0f64;
  let mut index = 0usize;
  for ch in text.to_ascii_uppercase().chars() {
    let Some(g) = glyph(ch) else {
      continue;
    };
    for col in g {
      for row in 0..7 {
        let on = col & (1 << row) != 0;
        let (start, end) = pixel_range(index, sample_rate);
        index += 1;
        for _ in start..end {
          if on {
            out.push(phase.sin() as f32);
          } else {
            out.push(0.0);
          }
          phase = (phase + TAU * CARRIER_HZ / sample_rate as f64) % TAU;
        }
      }
    }
  }
  out
}

/// AM 采样 → 文本；不足一个字符或纯静音返回 `None`。
///
/// 逐像素 RMS 包络 + 0.5×**本字符**峰值门限判定黑白，再按 7×7 网格匹配字库（假设字符边界
/// 对齐）。门限按字符取而不是按整段取：整段共用一个门限时，前一个强突发（衰落、按键瞬态）
/// 会把后面正常的字符整片判成白，而按字符取只影响那一个字符。
#[must_use]
pub fn decode_audio(samples: &[f32], sample_rate: u32) -> Option<String> {
  let mut rms: Vec<f64> = Vec::new();
  let mut index = 0usize;
  loop {
    let (start, end) = pixel_range(index, sample_rate);
    if end > samples.len() {
      break;
    }
    let seg = &samples[start..end];
    let e = seg
      .iter()
      .map(|&s| f64::from(s) * f64::from(s))
      .sum::<f64>()
      / (end - start) as f64;
    rms.push(e.sqrt());
    index += 1;
  }
  if rms.len() < PIXELS_PER_CHAR {
    return None;
  }
  let max = rms.iter().cloned().fold(0.0f64, f64::max);
  if max < 1e-4 {
    return None; // 纯静音
  }
  let mut out = String::new();
  // 命中率只统计「有内容」的块：空白块天然会匹配到字库里的空格，把它们算进分母 / 分子，
  // 一段几乎全静音、夹一声咔哒的音频命中率就接近 100%，会返回 `"   ?   "`。
  let mut inked = 0usize;
  let mut matched_inked = 0usize;
  for chunk in rms.chunks(PIXELS_PER_CHAR) {
    if chunk.len() < PIXELS_PER_CHAR {
      break;
    }
    // 本字符的门限；峰值远低于整段最强字符时按「整字符空白」处理（字符间隔、静音段），
    // 否则噪声会把自己切出一堆随机点阵，命中率被这些噪声字符拖垮。
    let peak = chunk.iter().cloned().fold(0.0f64, f64::max);
    let blank = peak < max * BLANK_RATIO;
    let threshold = 0.5 * peak;
    let mut g = [0u8; 7];
    if !blank {
      for (i, &e) in chunk.iter().enumerate() {
        if e > threshold {
          g[i / GRID] |= 1 << (i % GRID);
        }
      }
    }
    if !blank {
      inked += 1;
    }
    match match_glyph(g) {
      Some(c) => {
        if !blank {
          matched_inked += 1;
        }
        out.push(c);
      }
      None => out.push('?'),
    }
  }
  // 只要不是纯静音，逐像素门限对白噪声也能切出一串随机点阵；命中率过低说明这根本不是
  // 本字库的 Hell 信号，报「无信号」比丢一串 `?` 给用户更有用（约定见
  // docs/decode-worker-protocol.md 的质量门限）。一个有内容的块都没有（全是空白）同样
  // 没有可报告的文本。
  if inked == 0 || (matched_inked as f64) / (inked as f64) < MIN_GLYPH_MATCH {
    return None;
  }
  Some(out)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn wideband_noise_is_not_reported_as_text() {
    // 白噪声每个像素都有能量，门限切出来的是随机点阵；命中率门限必须把它判成无信号，
    // 而不是丢一串 `?` 给用户。
    let mut seed = 0x1234_5678u32;
    let noise: Vec<f32> = (0..8000 * 4)
      .map(|_| {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        ((seed >> 8) as f32 / 16_777_216.0) * 2.0 - 1.0
      })
      .collect();
    assert_eq!(decode_audio(&noise, 8000), None);
  }

  #[test]
  fn silence_with_one_click_is_not_reported_as_text() {
    // 大部分是静音、只夹一声咔哒：空白块天然匹配到字库里的空格，若把它们算进命中率，
    // 命中率会接近 100% 而返回 `"   ?   "`。
    let pp = per_pixel(8000);
    let mut samples = vec![0.0f32; pp * PIXELS_PER_CHAR * 8];
    let at = pp * PIXELS_PER_CHAR * 3 + pp * 10;
    for s in &mut samples[at..at + pp] {
      *s = 0.9;
    }
    assert_eq!(decode_audio(&samples, 8000), None);
  }

  #[test]
  fn spaces_between_words_do_not_dilute_the_match_rate() {
    // 反过来：正常报文里的空格是空白块，不能把整条报文拖成「无信号」。
    let samples = modulate("A   B", 8000);
    assert_eq!(decode_audio(&samples, 8000).as_deref(), Some("A   B"));
  }

  #[test]
  fn round_trips_text() {
    for text in ["CQ DE BG1AAA", "HELLO WORLD", "12345", "CQ/CQ"] {
      let samples = modulate(text, 8000);
      assert_eq!(decode_audio(&samples, 8000).as_deref(), Some(text));
    }
  }

  #[test]
  fn a_loud_burst_does_not_blank_later_characters() {
    // 前一个字符满幅、后面的只有两成：整段共用 0.5×全局峰值时后面整片会被判成白（空格），
    // 门限按字符取才不会一损俱损。
    let mut samples = modulate("A", 8000);
    let mut quiet = modulate("B", 8000);
    for s in &mut quiet {
      *s *= 0.2;
    }
    samples.extend(quiet);
    assert_eq!(decode_audio(&samples, 8000).as_deref(), Some("AB"));
  }

  #[test]
  fn case_is_normalized() {
    let samples = modulate("cq de bg1aaa", 8000);
    assert_eq!(
      decode_audio(&samples, 8000).as_deref(),
      Some("CQ DE BG1AAA")
    );
  }

  #[test]
  fn silence_and_short_audio_return_none() {
    assert_eq!(decode_audio(&[], 8000), None);
    assert_eq!(decode_audio(&[0.0f32; 10], 8000), None);
    // 48 像素 = 不到一个字符（7×7）。
    let short = modulate("CQ", 8000);
    let pp = per_pixel(8000);
    assert_eq!(decode_audio(&short[..48 * pp], 8000), None);
  }

  #[test]
  fn pixel_boundaries_do_not_drift_across_rates() {
    // 名义点数仍是 65 / 392（供估算用），但切片按理想边界取整，累计偏差 ≤ 0.5 采样。
    assert_eq!(per_pixel(8000), 65); // 8000 / 122.5 ≈ 65.3
    assert_eq!(per_pixel(48000), 392);
    for rate in [8000u32, 11_025, 48_000] {
      for n in [1usize, 49, 490, 4_900] {
        let (start, _) = pixel_range(n, rate);
        let ideal = n as f64 * f64::from(rate) / PIXEL_HZ;
        assert!(
          (start as f64 - ideal).abs() < 1.0,
          "{rate} Hz 的第 {n} 个像素边界漂了：{start} vs {ideal:.2}"
        );
      }
    }
  }

  #[test]
  fn the_font_table_is_five_by_seven() {
    // 手绘字库：行宽多一个字符会让 `columns` 的位运算错位（现在改成了忽略越界列），
    // 少一个字符则字形整体缺列。表形由这条用例钉住。
    for (ch, rows) in FONT {
      for (i, row) in rows.iter().enumerate() {
        assert_eq!(
          row.chars().count(),
          5,
          "{ch} 第 {} 行不是 5 列：{row:?}",
          i + 1
        );
        assert!(
          row.chars().all(|c| c == '.' || c == '#'),
          "{ch} 第 {} 行有非法像素：{row:?}",
          i + 1
        );
      }
    }
  }

  #[test]
  fn zero_sample_rate_is_rejected() {
    // 采样率为 0 时 `CARRIER_HZ / sample_rate` 除零、整段采样变 NaN，应显式拒绝。
    assert!(modulate("CQ", 0).is_empty());
  }
}
