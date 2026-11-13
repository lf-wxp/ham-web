//! MFSK 收端：音频 → 文本。判决部分逐步对应 fldigi `mfsk::harddecode` / `softdecode` /
//! `decodesymbol` / `recvbit`；前端（中心频率、符号对齐、质量门限）是离线文件专用的重写。

use crate::fec::{Code, Interleaver, PUNCTURE, Viterbi};
use crate::front::{
  Grid, POLISH_REACH, POLISH_REACH_LONG, clean_carriers, detect_base as detect_base_grid,
  find_offset, gate, idle_tone_shift, peak_ratios, polish_base,
};
use crate::lookup::{printable_fraction, readability, render, varidec};
use crate::picture::{self, Header, HeaderScanner, Picture};
use crate::track::{REDO_CLOCK_PPM, bins_tracked, corrected_hz, undo_clock};
use crate::{Mode, gray_decode};

/// fldigi 的回溯深度（`tracepair.trace`）。
const TRACEBACK: usize = 45;

/// 同一音调连续被判为最强多少次就当作窄带干扰（fldigi `CWI_MAXCOUNT`）。
const CWI_MAXCOUNT: i32 = 6;

/// 长交织变体（64L / 128L）不做窄带干扰检测：计数永远到不了这个上限。
///
/// 它们的发端没有前导零位，信号头 `(符号位数 − 1) × 深度` 个符号里交织器还没铺开，数据位只经过零延迟那一路，
/// 符号几乎全是「单个码位为 1」的几个音调，同一音调连发十来次是正常数据（实测 64L 的开头就有连续 10 个音调 15）。
/// 按干扰处理会把这些符号软删除成平均电平，判决只剩其余音调上的噪声，正文头几个字符被随机译歪
/// （`the quick` → `the®uick`；无噪声的合成信号里更明显，因为其余音调只剩浮点尘埃）。
const CWI_OFF: i32 = i32::MAX - 1;

/// 解码结果。
#[derive(Debug, Clone, PartialEq)]
pub struct Decoded {
  /// 正文（`\r` 换成换行，其余控制字符去掉；字节按 Latin-1 解读）。
  pub text: String,
  /// 使用的信号中心频率（Hz）：调用方没给就是自动检测的结果；有频漂时取信号区间中点处的值。
  pub center_hz: f64,
  /// 参与解码的符号数。
  pub symbols: usize,
  /// 符号周期相对名义值的偏差（ppm），正 = 录音时钟偏慢；没有跟踪到时钟误差是 0。
  pub clock_ppm: f64,
  /// 频漂（Hz / 秒），正 = 越往后频率越高；没有跟踪到频漂是 0。
  pub drift_hz_per_s: f64,
  /// 译码用的交织深度（MFSK4 / 8 会在 5 与 10 之间择优，其余模式固定）。
  pub depth: usize,
  /// 从录音里解出来的图片（只有 MFSK16 / 32 / 64 / 128 有图片模式，且录音采样率要等于该模式的标称采样率）。
  /// 正文 `text` 里照常保留 `Pic:…;` 报头，图像段本身的乱码已经从正文里去掉。
  pub pictures: Vec<Picture>,
  /// 正文里有图片报头，但没有解出图片：录音采样率不是该模式的标称值（图片的样本数是按标称采样率给的）、
  /// 报头之后没有录到图像，或报头数量超过上限。页面据此提示，不然用户只看到一段缺了图的正文。
  pub picture_skipped: bool,
}

/// 报头补完的位置：图像段起点就是从这里推出来的。
#[derive(Debug, Clone, Copy)]
struct PicHit {
  header: Header,
  /// 补完报头的那个字符所在符号窗口**结束**的样本位置（小数）。
  window_end: f64,
  /// 补完报头的那个字符是该符号里的第几个软位（1 起）。
  soft: usize,
}

/// 一段连续的 MFSK 文本解码结果，加上里面找到的图片报头。
struct Segment {
  decoded: Decoded,
  hits: Vec<PicHit>,
  /// 跟踪时钟 / 频漂之前的中心频率（Hz）。整段录音里夹着图像段时，跟踪会把图像段当成信号拟合出一条离谱的
  /// 频漂曲线（实测 MFSK64 的图片录音拟合出 +30 Hz），所以要重解一小段图像段之前的录音时从这里出发。
  nominal_center_hz: f64,
}

/// 解码这个子模式用的符号窗口几何（MFSK 的音调间隔 = 波特率）。
fn grid_for(rate: u32, mode: &Mode) -> Option<Grid> {
  Grid::new(rate, mode.baud(), mode.baud(), mode.numtones())
}

/// 估计信号**最低音调**的频率（Hz）：见 [`crate::front::detect_base`]。
#[must_use]
pub fn detect_base(samples: &[f32], rate: u32, mode: &Mode) -> Option<f64> {
  detect_base_grid(samples, &grid_for(rate, mode)?)
}

/// 完整解码：音频 → 文本。
///
/// - `center`：信号中心频率（Hz）；`None` 时自动估计（[`detect_base`]）；
/// - `reverse`：边带反转（LSB 录音）。
///
/// 噪声与静音返回 `None`：滑动平均峰均比划不出信号区间、或区间的平均峰均比不够，就判无信号
/// （见 [`crate::front::gate`]）。
#[must_use]
pub fn decode_audio(
  samples: &[f32],
  rate: u32,
  mode: &Mode,
  center: Option<f64>,
  reverse: bool,
) -> Option<Decoded> {
  let first = decode_with_pictures(samples, rate, mode, center, reverse, MAX_PICTURES);
  if center.is_some() || !mode.has_picture() || first.as_ref().is_some_and(plausible) {
    return first;
  }
  // 自动检测中心频率时，窗口是在整段录音里的「有信号」符号上均匀抽的；录音里夹着一大段模拟图像
  // （调频、不在音调栅格上）时，图像里的窗口会把梳带偏几个音调（实测一张 32 × 24 的斜坡图能带偏 3 个音调），
  // 译出来是乱码。图片发射总是从一段普通 MFSK 文本开始（报头在里面），所以退一步只用录音的开头一段再估一次。
  //
  let mut best = first;
  for fraction in [0.3, 0.5] {
    let part = &samples[..((samples.len() as f64 * fraction) as usize)];
    let Some(base) = detect_base(part, rate, mode) else {
      continue;
    };
    let retry_center = base + mode.bandwidth() / 2.0;
    // 只用开头一段估出来的梳与第一遍落在同一处时，重解只会再得到同一份乱码（用户选错子模式时就是这样），
    // 白白多花一遍整段解码。
    if best
      .as_ref()
      .is_some_and(|d| (d.center_hz - retry_center).abs() < mode.baud() / 2.0)
    {
      continue;
    }
    let retry = decode_with_pictures(
      samples,
      rate,
      mode,
      Some(retry_center),
      reverse,
      MAX_PICTURES,
    );
    if retry.as_ref().is_some_and(plausible) {
      return retry;
    }
    if best.is_none() {
      best = retry;
    }
  }
  best
}

/// 解出来的东西像不像真的：有图片，或正文里九成以上是可打印字符 / 换行。
fn plausible(d: &Decoded) -> bool {
  !d.pictures.is_empty() || printable_fraction(&d.text) >= 0.9
}

/// 解出来的录音里最多处理多少张图片（防止畸形输入把递归拖得很深）。
const MAX_PICTURES: usize = 4;

/// 文本 + 图片：照常解出文本，找到图片报头就把图像段切出来做 FM 解调，正文里去掉图像段的乱码。
///
/// 一次图片发射的结构见 [`crate::picture`]。图像段之前、之后的 MFSK 文本是**两段独立的信号**：
/// 它们的符号对齐不一样（图像段的长度不是符号长度的整数倍），所以分别重新对齐、各自解码，
/// 与 fldigi 在图片结束时 `rx_init()` 清空接收流水线是一回事。
///
/// 一段录音只处理**第一个**报头：报头之后就是图像（fldigi 收到报头即转入模拟接收，不再收字符），
/// 所以「正文里聊到 `Pic:640x480C;` 然后跟一张真图」这种顺序不会被误判；反过来（先假装一个巨型报头、
/// 后面才是真图）会解不出图，但正文保留并置 `picture_skipped`。
fn decode_with_pictures(
  samples: &[f32],
  rate: u32,
  mode: &Mode,
  center: Option<f64>,
  reverse: bool,
  pictures_left: usize,
) -> Option<Decoded> {
  let seg = decode_segment(samples, rate, mode, center, reverse)?;
  let Some(&hit) = seg.hits.first() else {
    return Some(seg.decoded);
  };
  // 图片的采样数（序言 352、每像素 8 个样本）是按模式的标称采样率给的，别的采样率要先重采样，这里不做。
  let picture_result = (pictures_left > 0 && rate == mode.rate)
    .then(|| with_picture(samples, rate, mode, reverse, hit, &seg, pictures_left))
    .flatten();
  // 图片这条路上任何一步走不通，都退回已经解出的正文，不能把整次解码丢掉。
  Some(picture_result.unwrap_or(Decoded {
    picture_skipped: true,
    ..seg.decoded
  }))
}

/// [`decode_with_pictures`] 的图片分支：按第一个报头切出图像段，前后的文本与可能的下一张图一并解出。
/// 任何一步失败（重解不出、图像段不在录音里）返回 `None`，由调用方退回纯文本。
fn with_picture(
  samples: &[f32],
  rate: u32,
  mode: &Mode,
  reverse: bool,
  hit: PicHit,
  seg: &Segment,
  pictures_left: usize,
) -> Option<Decoded> {
  let grid = grid_for(rate, mode)?;
  // 报头在整段录音的全局对齐下补完；图像段之前那一小段的对齐才是准的（图像段之后的对齐可能差几百个样本），
  // 所以拿「报头之后几个符号为止」的录音重解一遍，用它的对齐来定图像起点。
  let cut = ((hit.window_end + 8.0 * grid.sym).ceil() as usize).min(samples.len());
  // 这一段里没有图像，**重新自动检测**中心频率，不沿用整段估出来的值：整段里那一大段模拟调频会把音调梳
  // 带偏（320 × 240 彩色在 MFSK32 上是 237 秒，实测偏 +15.6 Hz —— 半个音调，图片平均误差从 7.9 涨到 57.6）。
  // 这一段太短（只有报头那几个符号）时自动检测会失败，退回整段的估值。
  let local = decode_segment(&samples[..cut], rate, mode, None, reverse).or_else(|| {
    decode_segment(
      &samples[..cut],
      rate,
      mode,
      Some(seg.nominal_center_hz),
      reverse,
    )
  })?;
  // 图像段之前这一小段里没有图像，跟踪是干净的：它的中心频率才是发端的中心频率。
  let center_hz = local.decoded.center_hz;
  let local_hit = local
    .hits
    .iter()
    .find(|h| h.header == hit.header)
    .copied()
    .unwrap_or(hit);
  let start = picture_start(mode, &local_hit, grid.sym);
  let demod = picture::FmDemod::mfsk(rate, center_hz, mode.bandwidth());
  let picture = picture::decode_picture(samples, &demod, start, &local_hit.header, reverse)?;
  let end = (start + local_hit.header.samples() as f64).ceil() as usize;

  // 图像段之前的文本（含报头）：截到图像起点，避免把图像段当 MFSK 译出乱码。
  let before_end = (start.floor() as usize).min(samples.len());
  let mut decoded = decode_segment(&samples[..before_end], rate, mode, Some(center_hz), reverse)
    .map_or(local.decoded, |s| s.decoded);
  decoded.pictures = vec![picture];
  // 图像段之后的文本与可能的下一张图。
  if end < samples.len()
    && let Some(after) = decode_with_pictures(
      &samples[end..],
      rate,
      mode,
      Some(center_hz),
      reverse,
      pictures_left - 1,
    )
  {
    decoded.text.push_str(&after.text);
    decoded.pictures.extend(after.pictures);
    decoded.symbols += after.symbols;
    decoded.picture_skipped |= after.picture_skipped;
  }
  Some(decoded)
}

/// 图像段第一个样本的位置（小数样本）。
///
/// 报头的最后一个字符是在接收流水线里延迟了一阵才解出来的，而发端在报头之后还要发完 `flushtx` 的冲刷、
/// 再发 352 个样本的序言，所以「字符补完的符号窗口结束」与「图像起点」之间的距离是固定的：
/// 冲刷的符号数减去流水线延迟（交织器 `3 × 深度` 个符号加 Viterbi 回溯约 24 个符号）、乘符号长，再加序言。
/// 这与 fldigi 在 `recvchar` 里硬编码的启动延迟一致（`counter = 352`，MFSK64 是 4956、MFSK128 是 1824，
/// 分别对应冲刷多出 36 / 23 个符号 × 符号长 + 352；MFSK64 差 4 个样本，交给后面的像素对齐去修）。
/// 字符恰好补完在符号的最后一个软位时多等一个符号（MFSK64 / 128 是软位序号为偶数时，fldigi 的 `recvchar`）。
fn picture_start(mode: &Mode, hit: &PicHit, sym: f64) -> f64 {
  let flush = (mode.preamble + 1) * 2 / mode.symbits;
  let latency = 3 * mode.depth + 24;
  let remaining = flush as f64 - latency as f64;
  let late = if matches!(mode.name, "64" | "128") {
    hit.soft.is_multiple_of(2)
  } else {
    hit.soft == mode.symbits
  };
  hit.window_end + (remaining + if late { 1.0 } else { 0.0 }) * sym + picture::PROLOGUE_SAMPLES
}

/// 一段连续 MFSK 文本的解码（不处理图片）。
///
/// 估出的时钟误差大到 [`REDO_CLOCK_PPM`] 时，把录音重采样回名义时钟再解一遍，取可打印字符占比更高的那个
/// （并列取重采样的）：时钟误差会把音调频率也一起缩放，中心频率检测在跟踪之前、不知道这件事，
/// 见 [`undo_clock`]。带图片报头的段不重来：报头位置与图像起点是按这份样本的时间轴给的。
fn decode_segment(
  samples: &[f32],
  rate: u32,
  mode: &Mode,
  center: Option<f64>,
  reverse: bool,
) -> Option<Segment> {
  let first = decode_segment_once(samples, rate, mode, center, reverse)?;
  let ppm = first.decoded.clock_ppm;
  if ppm.abs() < REDO_CLOCK_PPM || !first.hits.is_empty() {
    return Some(first);
  }
  let fixed = undo_clock(samples, ppm);
  let Some(mut second) = decode_segment_once(
    &fixed,
    rate,
    mode,
    center.map(|c| corrected_hz(c, ppm)),
    reverse,
  ) else {
    return Some(first);
  };
  if !second.hits.is_empty()
    || printable_fraction(&second.decoded.text) < printable_fraction(&first.decoded.text)
  {
    return Some(first);
  }
  // 结果按录音自己的时间轴 / 频率轴报：总时钟误差是两次相乘，中心频率换回录音里的值。
  let scale = 1.0 + ppm * 1e-6;
  second.decoded.clock_ppm = ((1.0 + second.decoded.clock_ppm * 1e-6) * scale - 1.0) * 1e6;
  second.decoded.center_hz /= scale;
  second.nominal_center_hz /= scale;
  Some(second)
}

/// [`decode_segment`] 的一遍。
fn decode_segment_once(
  samples: &[f32],
  rate: u32,
  mode: &Mode,
  center: Option<f64>,
  reverse: bool,
) -> Option<Segment> {
  let grid = grid_for(rate, mode)?;
  let (base, auto) = match center {
    Some(c) if c.is_finite() => (c - mode.bandwidth() / 2.0, false),
    Some(_) => return None,
    None => (detect_base_grid(samples, &grid)?, true),
  };
  if !grid.fits(base) {
    return None;
  }
  let mut offset = find_offset(samples, &grid, base);
  // 自动检测的中心频率在符号对齐之后再核对一遍；用户给定的不动。
  let polished = if auto {
    // 发射开头 / 结尾的恒定空闲音（最低音调）是最可靠的锚；没有它（录音从中途开始）再靠评分核对。
    match idle_tone_shift(samples, &grid, base, offset, reverse) {
      Some(k) => base + f64::from(k) * grid.spacing,
      None => polish_base(
        samples,
        &grid,
        base,
        offset,
        if mode.long {
          POLISH_REACH_LONG
        } else {
          POLISH_REACH
        },
      ),
    }
  } else {
    base
  };
  if !grid.fits(polished) {
    return None;
  }
  // 核对挪动了梳（64L / 128L 这类证据稀疏的录音会挪好几个音调）时，符号对齐是按旧梳算的，
  // 空闲音以外的音调位置对不上、对齐点会偏一点（实测 64L 因此把 `quick` 译成 `fck`），要重算。
  if polished != base {
    offset = find_offset(samples, &grid, polished);
  }
  let base = polished;
  let nt = grid.ntones;
  // 时钟误差与频漂：没有值得做的修正时 `drift` 就是不跟踪的网格，`bins` 与 `symbol_bins` 逐位相同。
  let (mut bins, drift) = bins_tracked(samples, &grid, base, offset);
  // 比信号弱的载波「减」掉，比信号强的「抹」掉：软判决虽然不靠单个音调位定音，但一个恒定的强载波会让
  // 它所在位的软值永远偏向同一个码位。返回值（两种载波的个数）只给测试与诊断用，解码路径不看。
  clean_carriers(&mut bins, nt);
  let (first, last) = gate(&peak_ratios(&bins, nt))?;
  let span = last - first + 1;
  let mid_base = drift.base_at((first + last) as f64 / 2.0);

  let region: Vec<&[f64]> = bins.chunks_exact(nt).skip(first).take(span).collect();
  let mut best: Option<((i64, f64), Segment)> = None;
  for depth in depth_candidates(mode) {
    let mut rx = Receiver::new(mode, depth);
    let mut tmp = vec![0.0f64; nt];
    for chunk in &region {
      if reverse {
        for (t, &v) in tmp.iter_mut().zip(chunk.iter().rev()) {
          *t = v;
        }
        rx.feed(&tmp);
      } else {
        rx.feed(chunk);
      }
    }
    let mean_metric = rx.metric_sum / rx.metric_n.max(1) as f64;
    rx.flush();
    let text = render(&rx.chars);
    let score = readability(&text);
    if best
      .as_ref()
      .is_none_or(|(old, _)| depth_wins((score, mean_metric), *old))
    {
      let hits = if mode.has_picture() {
        rx.hits
          .iter()
          .map(|&(header, fed, soft)| PicHit {
            header,
            window_end: drift.start_at((first + fed) as f64),
            soft,
          })
          .collect()
      } else {
        Vec::new()
      };
      best = Some((
        (score, mean_metric),
        Segment {
          decoded: Decoded {
            text,
            center_hz: mid_base + mode.bandwidth() / 2.0,
            symbols: span,
            clock_ppm: drift.clock_ppm(),
            drift_hz_per_s: drift.drift_hz_per_s(),
            depth,
            pictures: Vec::new(),
            picture_skipped: false,
          },
          hits,
          nominal_center_hz: base + mode.bandwidth() / 2.0,
        },
      ));
    }
  }
  best.map(|(_, seg)| seg)
}

fn decayavg(average: f64, input: f64, weight: i32) -> f64 {
  if weight <= 1 {
    return input;
  }
  (input - average) / f64::from(weight) + average
}

/// 判决 + 解交织 + 译码 + 切字符（fldigi `mfsk` 的接收链）。
struct Receiver {
  mode: Mode,
  inlv: Interleaver,
  inlv_depth: usize,
  dec1: Viterbi,
  dec2: Viterbi,
  /// 每个音调被连续判为最强的次数（窄带干扰检测）。
  cwi: Vec<i32>,
  /// 软符号的奇偶计数（`true` = 一对里的第一个）。
  first_of_pair: bool,
  pair: [u8; 2],
  met1: f64,
  met2: f64,
  /// 被采用的译码位的路径度量增量之和与个数：整段录音的平均度量，用来在几个交织深度之间挑（见 [`Receiver::mean_metric`]）。
  metric_sum: f64,
  metric_n: usize,
  datashreg: u32,
  chars: Vec<u16>,
  /// 已喂入的符号数、正在处理的是该符号的第几个软位、图片报头扫描器与已识别的报头。
  fed: usize,
  cur_soft: usize,
  scanner: HeaderScanner,
  hits: Vec<(Header, usize, usize)>,
  /// 窄带干扰判定的上限（[`CWI_MAXCOUNT`]，长交织变体为 [`CWI_OFF`]）。
  cwi_max: i32,
  /// 还要丢弃多少个符号的译码输出（长交织变体的头部，见 [`Mode::long`]）。
  settle_left: usize,
}

impl Receiver {
  fn new(mode: &Mode, depth: usize) -> Self {
    Self {
      mode: *mode,
      inlv: Interleaver::new(mode.symbits, depth, false),
      inlv_depth: depth,
      dec1: Viterbi::new(Code::NASA_K7, TRACEBACK),
      dec2: Viterbi::new(Code::NASA_K7, TRACEBACK),
      cwi: vec![0; mode.numtones()],
      first_of_pair: false,
      pair: [0; 2],
      met1: 0.0,
      met2: 0.0,
      metric_sum: 0.0,
      metric_n: 0,
      datashreg: 1,
      chars: Vec::new(),
      fed: 0,
      cur_soft: 0,
      scanner: HeaderScanner::default(),
      hits: Vec::new(),
      cwi_max: if mode.long { CWI_OFF } else { CWI_MAXCOUNT },
      // 反交织延迟，再加 Viterbi 回溯延迟：真数据的第一个字符要等这么多个符号才流得出来。
      settle_left: if mode.long {
        (mode.symbits - 1) * depth + (TRACEBACK * 2).div_ceil(mode.symbits)
      } else {
        0
      },
    }
  }

  /// fldigi `harddecode`：最强音调，以及「所有音调都过强」的宽带脉冲（staticburst）。
  fn hard_decode(bins: &[f64]) -> (usize, bool) {
    let n = bins.len() as f64;
    let avg = (bins.iter().sum::<f64>() / n).max(1e-20);
    let mut best = (0.0f64, 0usize);
    let mut burst = 0usize;
    for (i, &x) in bins.iter().enumerate() {
      if x > best.0 {
        best = (x, i);
      }
      if x > 2.0 * avg {
        burst += 1;
      }
    }
    (best.1, burst == bins.len())
  }

  /// fldigi `softdecode`：逐音调幅度按格雷码折成每个码位的软值（0..=255，128 = 无把握）。
  fn soft_decode(&mut self, bins: &[f64], current: usize, static_burst: bool) -> Vec<u8> {
    let (n, bits) = (bins.len(), self.mode.symbits);
    let mut sum = 0.0;
    for (i, &x) in bins.iter().enumerate() {
      if self.cwi[i] < self.cwi_max {
        sum += x;
      }
    }
    let avg = sum / n as f64;
    // 避免后面除零。
    let sum = sum.max(1e-10);
    // 动态窄带干扰：用硬判决的结果给每个音调的「连续最强」计数加减，从 1 起（0 号音调不参与）。
    for k in 1..n {
      if k == current {
        self.cwi[k] += 1;
      } else {
        self.cwi[k] -= 1;
      }
      self.cwi[k] = self.cwi[k].clamp(0, self.cwi_max + 1);
    }
    let mut b = vec![0.0f64; bits];
    for (i, &mag) in bins.iter().enumerate() {
      let j = gray_decode(i as u32) as usize;
      let binmag = if self.cwi[i] > self.cwi_max {
        avg // 窄带干扰：软删除成平均电平
      } else if i == current {
        2.0 * mag // 硬判决也投一票
      } else {
        mag
      };
      for (k, bk) in b.iter_mut().enumerate() {
        *bk += if j & (1 << (bits - k - 1)) != 0 {
          binmag
        } else {
          -binmag
        };
      }
    }
    b.iter()
      .map(|&bk| {
        if static_burst {
          PUNCTURE
        } else {
          (128.0 + bk / sum * 256.0).clamp(0.0, 255.0) as u8
        }
      })
      .collect()
  }

  /// 一个符号的各音调幅度 → 若干字符（写进 `self.chars`）。
  fn feed(&mut self, bins: &[f64]) {
    let (current, burst) = Self::hard_decode(bins);
    let mut syms = self.soft_decode(bins, current, burst);
    self.inlv.symbols(&mut syms);
    let before = self.chars.len();
    self.fed += 1;
    for (k, s) in syms.into_iter().enumerate() {
      self.cur_soft = k + 1;
      self.decode_symbol(s);
    }
    // 反交织器还没填满：这一段译出的是发端没发过的「发送前历史」，不是正文。
    if self.settle_left > 0 {
      self.settle_left -= 1;
      self.chars.truncate(before);
    }
  }

  /// 信号结束后把管线里压着的字符推出来：喂若干个「全是删除位」的符号。
  ///
  /// 反向交织器与 Viterbi 回溯合起来有一段固定延迟（`(符号位数 − 1) × 深度` 个符号加 `回溯 / 每符号的
  /// 译码位数` 个符号），信号最后那几个字符要等这么多个符号才流得出来。fldigi 的接收机收完信号后
  /// 还会一直跑（噪声会把它们推出来），离线文件没有「后面」，所以在这里补。删除位的度量增量为 0，
  /// 不会改变已有路径的取舍，只是让回溯窗口往前走。
  fn flush(&mut self) {
    // 长交织变体：发端自带的冲刷（2500 / 5000 个零位）比交织延迟加回溯还长，最后的字符早已流出；
    // 再补删除位只会把冲刷区里的零位与删除位混在一起、译出乱码（见 [`Mode::long`]）。
    if self.mode.long {
      return;
    }
    let bits = self.mode.symbits;
    // 每个符号含 `bits / 2` 对码位 = 这么多个译码位；向上取整。
    let tail = (TRACEBACK * 2).div_ceil(bits);
    for _ in 0..(bits - 1) * self.inlv_depth + tail {
      let mut syms = vec![PUNCTURE; bits];
      self.inlv.symbols(&mut syms);
      for s in syms {
        self.decode_symbol(s);
      }
    }
  }

  /// fldigi `decodesymbol`：软符号两两配对进 Viterbi。
  ///
  /// 奇数位的模式（5 位 / 3 位）码位对与符号边界错开，两个译码器交替喂入，
  /// 取路径度量滑动平均更高的那个的输出；偶数位只用一个译码器。
  fn decode_symbol(&mut self, symbol: u8) {
    self.pair = [self.pair[1], symbol];
    self.first_of_pair = !self.first_of_pair;
    let bit = if self.mode.symbits % 2 == 1 {
      if self.first_of_pair {
        let (c, met) = self.dec1.decode(self.pair);
        self.met1 = decayavg(self.met1, f64::from(met), 50);
        if self.met1 < self.met2 {
          return;
        }
        self.metric_sum += f64::from(met);
        self.metric_n += 1;
        c
      } else {
        let (c, met) = self.dec2.decode(self.pair);
        self.met2 = decayavg(self.met2, f64::from(met), 50);
        if self.met2 < self.met1 {
          return;
        }
        self.metric_sum += f64::from(met);
        self.metric_n += 1;
        c
      }
    } else {
      if self.first_of_pair {
        return;
      }
      let (c, met) = self.dec2.decode(self.pair);
      self.met2 = decayavg(self.met2, f64::from(met), 50);
      self.metric_sum += f64::from(met);
      self.metric_n += 1;
      c
    };
    self.recv_bit(bit);
  }

  /// fldigi `recvbit`：Varicode 码字以 `001` 结尾切分。
  fn recv_bit(&mut self, bit: bool) {
    self.datashreg = (self.datashreg << 1) | u32::from(bit);
    if self.datashreg & 7 == 1 {
      if let Some(c) = varidec(self.datashreg >> 1) {
        self.chars.push(c);
        if let Some(header) = self.scanner.push(c) {
          self.hits.push((header, self.fed, self.cur_soft));
        }
      }
      self.datashreg = 1;
    }
  }
}

/// 两个交织深度的平均路径度量相差这个比例以上，就以度量定胜负，否则看文本可读性。
const DEPTH_METRIC_GAP: f64 = 0.08;

/// 新候选 `(可读性, 平均路径度量)` 是否胜过当前最好的。
///
/// 为什么先看路径度量：深度选错时译出的是乱码，而乱码往往**更长**（前导 / 冲刷的零位被当成数据译成
/// `llll…`），可读性的绝对分数偏向它——`TEST ` 这种极短报文里，深度 10 的乱码（18 分）胜过深度 5 的
/// 正确文本（10 分）。路径度量不受长度影响：正确的深度下码字自洽、度量高（干净信号 160–180），选错的深度
/// 是随机位流（120–137），差距在 20% 以上；噪声里两者都会降，差距小于 [`DEPTH_METRIC_GAP`] 时才退回可读性。
/// 并列（可读性也相同）留先试的（fldigi 的深度）。
fn depth_wins(new: (i64, f64), old: (i64, f64)) -> bool {
  let gap = (new.1 - old.1) / old.1.abs().max(1e-9);
  if gap.abs() > DEPTH_METRIC_GAP {
    return gap > 0.0;
  }
  new.0 > old.0
}

/// 译码时要试的交织深度。
///
/// MFSK4 / 8 在 fldigi 里是 5，但 gMFSK（这个模式的原作者实现）发出的 MFSK8 录音实测用 10 才能译出
/// 原文（用 gmfsk.connect.fi 的官方样本验证过），所以这两个模式两种都试、按平均路径度量择优（度量接近时才看可读性，见 [`depth_wins`]）。
/// 其余模式两家一致，只有一个深度。
fn depth_candidates(mode: &Mode) -> Vec<usize> {
  let mut v = vec![mode.depth];
  if matches!(mode.name, "4" | "8") {
    v.push(10);
  }
  v
}
