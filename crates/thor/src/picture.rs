//! THOR 图片模式：报头识别与同步音。
//!
//! 转写自 **fldigi**（GPL-3.0）`src/thor/thor.cxx` 的 `parse_pic` / `recvpic` / `send_image`（图片模式 © W1HKJ）。
//! 与 MFSK 的图片模式（[`ham_web_mfsk::picture`]）不是一个协议：
//!
//! 1. 正文里有一个 5 字符的**报头** `pic%X`，`X` 选尺寸与色彩（`T` 59×74 彩色、`t` 同尺寸灰度，`S`/`s` 160×120，
//!    `L`/`l` 320×240，`V`/`v` 640×480，`P`/`p` 240×300，`M`/`m` 120×150，`F` 640×480 灰度，`A` 59×74 头像）；
//! 2. 报头之后照常 `flushtx`，接着是 **20 个符号长的静音**、**20 个符号长的同步音**（频率 = 中心 − 0.6 × 带宽，
//!    在 18 个音调所占的带之外）、然后才是图像；
//! 3. 图像每个像素 [`SPP`] 个样本（`THOR_IMAGESPP`），频率 = 中心 + 带宽 × (字节 − 128) / 256，
//!    彩色图每行 R、G、B 三遍；
//! 4. 接收端（fldigi）收到报头字符后立刻转入模拟接收，等同步音出现再消失、之后按 `image_counter % spp` 取像素。
//!
//! 离线接收这里**直接找同步音**：它又长又固定、在音调带之外，同步音结束的位置就是第一个像素的起点，
//! 不需要像 MFSK 那样靠解码流水线的延迟去推。

use ham_web_mfsk::picture::{Header, instantaneous_frequency};

/// 每个像素的样本数（fldigi `THOR_IMAGESPP`）。
pub const SPP: usize = 10;

/// 同步音的频率相对中心的偏移（占带宽的比例）。
pub const SYNC_OFFSET: f64 = -0.6;

/// 同步音与静音各占多少个符号长（fldigi `PHASE_CORR`）。
pub const SYNC_SYMBOLS: usize = 20;

/// 判定「离开了同步音」：瞬时频率离同步音的频率超过带宽的这个比例。
const SYNC_TOLERANCE: f64 = 0.05;

/// 同步音在整段里漂移不超过带宽的这个比例（同步音是恒定的一个频率）。
const RUN_DRIFT: f64 = 0.04;

/// 至少连续多少个符号长才算同步音（真实的是 [`SYNC_SYMBOLS`]；留一半以上的余量，吃掉头尾的过渡）。
const MIN_SYNC_SYMBOLS: usize = 8;

/// 第一个像素离同步音至少这么远（占带宽的比例），才用台阶中点定边界；否则（头几个像素是黑的，
/// 与同步音只差 0.1 带宽）取同步音结束处。
const STEP_MIN: f64 = 0.06;

/// 报头扫描器：逐个主信道字符喂，识别 `pic%X`（fldigi `parse_pic`）。
///
/// 与 MFSK 不同：**每个**主信道字符（含 `\r` 等控制字符）都进 5 字符窗口，识别成功后窗口不清空。
#[derive(Debug, Clone)]
pub struct HeaderScanner {
  window: [u8; 5],
}

impl Default for HeaderScanner {
  fn default() -> Self {
    Self { window: [b' '; 5] }
  }
}

impl HeaderScanner {
  /// 喂一个主信道字符；窗口里是 `pic%X` 且 `X` 合法时返回报头。
  pub fn push(&mut self, c: u16) -> Option<Header> {
    let Ok(c) = u8::try_from(c) else {
      return None;
    };
    self.window.copy_within(1.., 0);
    self.window[4] = c;
    if &self.window[..4] != b"pic%" {
      return None;
    }
    let (width, height, color) = match self.window[4] {
      b'A' | b'T' => (59, 74, true),
      b't' => (59, 74, false),
      b'S' => (160, 120, true),
      b's' => (160, 120, false),
      b'L' => (320, 240, true),
      b'l' => (320, 240, false),
      b'V' => (640, 480, true),
      b'v' | b'F' => (640, 480, false),
      b'P' => (240, 300, true),
      b'p' => (240, 300, false),
      b'M' => (120, 150, true),
      b'm' => (120, 150, false),
      _ => return None,
    };
    Some(Header {
      width,
      height,
      color,
      spp: SPP,
    })
  }
}

/// 报头里的字符：`Header` 的尺寸与色彩反查（发端用）。`F`（640×480 灰度的另一个码）与 `A`（头像）不会被选中。
#[must_use]
pub fn code_for(header: &Header) -> Option<u8> {
  Some(match (header.width, header.height, header.color) {
    (59, 74, true) => b'T',
    (59, 74, false) => b't',
    (160, 120, true) => b'S',
    (160, 120, false) => b's',
    (320, 240, true) => b'L',
    (320, 240, false) => b'l',
    (640, 480, true) => b'V',
    (640, 480, false) => b'v',
    (240, 300, true) => b'P',
    (240, 300, false) => b'p',
    (120, 150, true) => b'M',
    (120, 150, false) => b'm',
    _ => return None,
  })
}

/// 找到的同步音。
#[derive(Debug, Clone, Copy)]
pub struct Sync {
  /// 同步音开始的位置（样本，糊几十个样本：只用来定「报头在这之前」，不用来对齐像素）。
  pub begin: f64,
  /// 第一个像素的起点（样本，小数）。
  pub start: f64,
  /// 同步音的频率（Hz，整段的平均）。同步音在中心 − 0.6 × 带宽，所以 `hz + 0.6 × 带宽` 就是发端的中心频率
  /// （边带反转时是 `hz − 0.6 × 带宽`）—— 比文本里的梳检测准得多。
  pub hz: f64,
}

impl Sync {
  /// 由同步音的频率推出发端的中心频率。
  #[must_use]
  pub fn center_hz(&self, bandwidth_hz: f64, reverse: bool) -> f64 {
    let dir = if reverse { -1.0 } else { 1.0 };
    self.hz - dir * SYNC_OFFSET * bandwidth_hz
  }
}

/// 在整段录音里找第一段同步音。
///
/// - `center_hz`：对中心频率的**估计**，容许差一两个音调：同步音只要落在 `[中心 − 0.8 × 带宽, 中心 − 0.48 × 带宽]`
///   里、连续不变（漂移不超过 [`RUN_DRIFT`] × 带宽）、长度够就算；它的频率由整段平均给出，不依赖估计的精度；
/// - `sym`：一个符号的样本数；
/// - `reverse`：边带反转时同步音在中心**上方**。
///
/// 找不到足够长的一段就是 `None`。
#[must_use]
pub fn find_sync(
  samples: &[f32],
  rate: u32,
  center_hz: f64,
  bandwidth_hz: f64,
  sym: f64,
  reverse: bool,
) -> Option<Sync> {
  let dir = if reverse { -1.0 } else { 1.0 };
  let edges = [
    center_hz - dir * 0.8 * bandwidth_hz,
    center_hz - dir * 0.48 * bandwidth_hz,
  ];
  let (lo_hz, hi_hz) = (edges[0].min(edges[1]), edges[0].max(edges[1]));
  let tol = SYNC_TOLERANCE * bandwidth_hz;
  // fldigi 的 `picfilter` 截止 = 带宽：同步音在 −0.6 带宽（容许再偏一点），要放得过去。
  let raw = instantaneous_frequency(
    samples,
    f64::from(rate),
    center_hz,
    bandwidth_hz,
    0,
    samples.len(),
  );
  // 判「在不在同步音上」用很长的滑动平均（四分之一个符号）：噪声里单个样本的瞬时频率抖得很厉害，
  // 平均得不够会把一整段同步音切成碎片。代价是头尾的位置会糊掉几十个样本，所以结束位置之后还要用
  // 短窗口（fldigi 的 `pixsyncfilter`，`2 × SPP` 个样本）在它附近重新找。
  let window = ((sym / 4.0).round() as usize).max(2 * SPP);
  let heavy = smooth(&raw, window);
  let min_run = (MIN_SYNC_SYMBOLS as f64 * sym) as usize;
  let inside = |f: f64| (lo_hz..=hi_hz).contains(&f);
  let mut i = 0;
  while i < heavy.len() {
    if !inside(heavy[i]) {
      i += 1;
      continue;
    }
    let (begin, anchor) = (i, heavy[i]);
    let mut end = i;
    while end < heavy.len()
      && inside(heavy[end])
      && (heavy[end] - anchor).abs() <= RUN_DRIFT * bandwidth_hz
    {
      end += 1;
    }
    if end - begin >= min_run {
      // 整段的平均频率（去掉两头被滑动平均抹糊的部分）。
      let inner = &raw[(begin + window).min(end)..end.saturating_sub(window).max(begin + window)];
      let hz = inner.iter().sum::<f64>() / inner.len().max(1) as f64;
      let light = smooth(&raw, 2 * SPP);
      // 长窗口平均把结束位置抹糊了：在它前后各一个窗口里，找短窗口平均第一次连续 3 个样本离开同步音的位置。
      let lo = end.saturating_sub(window);
      let hi = (end + window).min(light.len().saturating_sub(3));
      let last_on = (lo..hi)
        .find(|&k| light[k..k + 3].iter().all(|&f| (f - hz).abs() > tol))
        .unwrap_or(end);
      return Some(Sync {
        begin: begin as f64,
        start: step_boundary(&smooth(&raw, SPP), last_on, hz, bandwidth_hz),
        hz,
      });
    }
    i = end.max(i + 1);
  }
  None
}

/// 居中的滑动平均（窗口两头收窄）。
///
/// 用前缀和做到 O(N)：整段录音有几百万个点、窗口是符号长的四分之一（上百个点），逐点重新累加会比
/// 前面的 FIR 本身还贵。前缀和相减的舍入误差远小于判决用的 Hz 级门限。
fn smooth(values: &[f64], window: usize) -> Vec<f64> {
  let half = window / 2;
  let mut prefix = Vec::with_capacity(values.len() + 1);
  prefix.push(0.0f64);
  let mut acc = 0.0f64;
  for &v in values {
    acc += v;
    prefix.push(acc);
  }
  (0..values.len())
    .map(|i| {
      let lo = i.saturating_sub(half);
      let hi = (i + half + 1).min(values.len());
      (prefix[hi] - prefix[lo]) / (hi - lo) as f64
    })
    .collect()
}

/// 同步音结束处的像素边界：同步音电平与第一个像素电平之间的**台阶中点**（低通是对称的，台阶中点就是边界）。
fn step_boundary(freqs: &[f64], last_on: usize, target: f64, bandwidth_hz: f64) -> f64 {
  let at = |i: usize| freqs.get(i).copied().unwrap_or(target);
  // 第一个像素的电平：同步音结束之后一个半像素处（台阶已经过去，平台还没走完）。
  let level = at(last_on + SPP + SPP / 2);
  if (level - target).abs() < STEP_MIN * bandwidth_hz {
    return last_on as f64 - SPP as f64 / 2.0;
  }
  let mid = (target + level) / 2.0;
  let rising = level > target;
  // 从「同步音结束处」往前留 3 个像素：短窗口平均在小台阶上会把结束位置判晚十几个样本。
  let begin = last_on.saturating_sub(3 * SPP);
  (begin..freqs.len())
    .find(|&i| if rising { at(i) >= mid } else { at(i) <= mid })
    .map_or(last_on as f64, |i| i as f64 + 0.5)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn scan(text: &str) -> Vec<Header> {
    let mut s = HeaderScanner::default();
    text.bytes().filter_map(|b| s.push(u16::from(b))).collect()
  }

  #[test]
  fn smooth_matches_the_naive_moving_average() {
    let values: Vec<f64> = (0..500)
      .map(|i| 1000.0 + ((i * 37) % 101) as f64 - 50.0)
      .collect();
    for window in [1usize, 2, 7, 20, 128, 600] {
      let half = window / 2;
      let got = smooth(&values, window);
      assert_eq!(got.len(), values.len());
      for (i, g) in got.iter().enumerate() {
        let lo = i.saturating_sub(half);
        let hi = (i + half + 1).min(values.len());
        let want = values[lo..hi].iter().sum::<f64>() / (hi - lo) as f64;
        assert!(
          (g - want).abs() < 1e-9,
          "window {window} 第 {i} 点：{g} != {want}"
        );
      }
    }
    assert!(smooth(&[], 8).is_empty());
  }

  #[test]
  fn headers_follow_fldigi_parse_pic() {
    let h = |w, h, color| Header {
      width: w,
      height: h,
      color,
      spp: SPP,
    };
    assert_eq!(scan("pic%T"), [h(59, 74, true)]);
    assert_eq!(scan("pic%t"), [h(59, 74, false)]);
    assert_eq!(scan("pic%A"), [h(59, 74, true)]);
    assert_eq!(scan("pic%S"), [h(160, 120, true)]);
    assert_eq!(scan("pic%l"), [h(320, 240, false)]);
    assert_eq!(scan("pic%V"), [h(640, 480, true)]);
    assert_eq!(scan("pic%F"), [h(640, 480, false)]);
    assert_eq!(scan("pic%p"), [h(240, 300, false)]);
    assert_eq!(scan("pic%M"), [h(120, 150, true)]);
    // 前面有别的文字、控制字符也进窗口。
    assert_eq!(scan("CQ\rpic%T"), [h(59, 74, true)]);
  }

  #[test]
  fn unknown_codes_and_partial_headers_are_ignored() {
    for text in ["pic%", "pic%X", "pic%3", "pic T", "PIC%T", "pi%T", "pic%"] {
      assert!(scan(text).is_empty(), "{text}");
    }
    // 窗口只有 5 个字符：报头被别的字符隔开就不算。
    assert!(scan("pic%\rT").is_empty());
    assert_eq!(HeaderScanner::default().push(0x154), None);
  }

  /// 构造一段信号：静音 → 同步音（中心 − 0.6 × 带宽）→ 一串像素（每个 [`SPP`] 个样本）。
  fn signal(pixels: &[u8], lead: usize, sync: usize, rate: f64, center: f64, bw: f64) -> Vec<f32> {
    let mut out = vec![0.0f32; lead];
    let mut phase = 0.0f64;
    let mut tone = |freq: f64, n: usize, out: &mut Vec<f32>| {
      for _ in 0..n {
        out.push(phase.cos() as f32);
        phase += std::f64::consts::TAU * freq / rate;
      }
    };
    tone(center + SYNC_OFFSET * bw, sync, &mut out);
    for &b in pixels {
      tone(center + bw * (f64::from(b) - 128.0) / 256.0, SPP, &mut out);
    }
    tone(center + SYNC_OFFSET * bw, 200, &mut out);
    out
  }

  #[test]
  fn the_sync_tone_ends_where_the_pixels_start() {
    let (rate, center, bw) = (8000.0, 1500.0, 281.25);
    let sym = 512.0;
    let lead = 4000usize;
    let sync = 20 * 512usize;
    let pixels = [220u8, 220, 40, 40, 180, 70, 30, 210];
    let samples = signal(&pixels, lead, sync, rate, center, bw);
    let found = find_sync(&samples, 8000, center, bw, sym, false).unwrap();
    let (begin, start) = (found.begin, found.start);
    assert!(
      (found.center_hz(bw, false) - center).abs() < 1.5,
      "中心 {}",
      found.center_hz(bw, false)
    );
    let want = (lead + sync) as f64;
    assert!(
      // 开头是静音、判决用的滑动平均有 128 个样本宽：开始位置糊几十个样本（只用来定「报头在这之前」，不用来对齐像素）。
      (begin - lead as f64).abs() <= 80.0,
      "同步音开始 {begin}，预期 {lead}"
    );
    assert!((start - want).abs() <= 2.0, "像素起点 {start}，预期 {want}");
  }

  #[test]
  fn a_black_first_pixel_falls_back_to_the_end_of_the_sync_tone() {
    // 字节 0 的频率是中心 − 0.5 × 带宽，与同步音只差 0.1 × 带宽：台阶太小，取同步音结束处。
    let (rate, center, bw) = (8000.0, 1500.0, 281.25);
    let samples = signal(&[0, 0, 200, 200], 3000, 20 * 512, rate, center, bw);
    let start = find_sync(&samples, 8000, center, bw, 512.0, false)
      .unwrap()
      .start;
    let want = (3000 + 20 * 512) as f64;
    assert!(
      (start - want).abs() <= 16.0,
      "像素起点 {start}，预期 {want}"
    );
  }

  #[test]
  fn nothing_that_is_not_a_long_sync_tone_is_found() {
    let (rate, center, bw) = (8000.0, 1500.0, 281.25);
    // 太短的音（3 个符号长）不算。
    let short = signal(&[200, 200], 2000, 3 * 512, rate, center, bw);
    assert!(find_sync(&short, 8000, center, bw, 512.0, false).is_none());
    // 静音与纯噪声都找不到。
    assert!(find_sync(&vec![0.0; 40_000], 8000, center, bw, 512.0, false).is_none());
    let mut noise = vec![0.0f32; 40_000];
    ham_web_dsp::add_noise(&mut noise, 1.0);
    assert!(find_sync(&noise, 8000, center, bw, 512.0, false).is_none());
    // 同步音在中心的另一侧（边带反转）。
    let long = signal(&[200, 200], 2000, 20 * 512, rate, center, bw);
    assert!(find_sync(&long, 8000, center, bw, 512.0, true).is_none());
  }
}
