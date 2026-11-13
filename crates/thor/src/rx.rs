//! THOR 收端：音频 → 文本。判决部分对应 fldigi `thor::decodesymbol`（硬位路径）/ `decodePairs`；
//! 前端（中心频率、符号对齐、质量门限）与 MFSK 共用 [`ham_web_mfsk::front`]。

use ham_web_mfsk::fec::{Interleaver, PUNCTURE, Viterbi};
use ham_web_mfsk::front::{
  Grid, POLISH_REACH, clean_carriers, detect_base as detect_base_grid, find_offset, gate,
  peak_ratios, polish_base,
};
use ham_web_mfsk::lookup::{printable_fraction, render, varidec};
use ham_web_mfsk::picture::{FmDemod, Header, Picture, decode_picture};
use ham_web_mfsk::track::bins_tracked;

use crate::picture::{self, HeaderScanner};
use crate::varicode::SECONDARY;
use crate::{Mode, NUMTONES};

/// 主 / 二次信道的分界：THOR 二次信道的码字都不小于它（fldigi `thorvaridec`）。
const SECONDARY_MIN: u32 = 0xB80;

/// 「最强音调 / 平均幅度」低于它就当作没有清晰音调（宽带脉冲或噪声），整个 nibble 删除（fldigi
/// `staticburst`）。
const STATIC_BURST_RATIO: f64 = 1.2;

/// 区间内音调差不合法（0 / 1 / −17：IFK+ 不会出现）的 nibble 占比上限。
///
/// IFK+ 的增量恒为 `2 + nibble` ∈ 2..=17，同一音调连续出现或只差 1 只可能来自噪声里的随机命中
/// （约 9%）；真实信号上它接近 0。峰均比门限已经挡掉了纯噪声，这里再挡一道「有能量但不是 IFK+」。
const MAX_INVALID_PERCENT: usize = 10;

/// 解码结果。
#[derive(Debug, Clone, PartialEq)]
pub struct Decoded {
  /// 主信道正文（`\r` 换成换行，其余控制字符去掉）。
  pub text: String,
  /// 二次信道（ID 信标）文本，可能为空。
  pub id: String,
  /// 使用的信号中心频率（Hz）：调用方没给就是自动检测的结果；有频漂时取信号区间中点处的值。
  pub center_hz: f64,
  /// 参与解码的符号数。
  pub symbols: usize,
  /// 符号周期相对名义值的偏差（ppm），正 = 录音时钟偏慢；没有跟踪到时钟误差是 0。
  pub clock_ppm: f64,
  /// 频漂（Hz / 秒），正 = 越往后频率越高；没有跟踪到频漂是 0。
  pub drift_hz_per_s: f64,
  /// 从录音里解出来的图片（THOR 11 / 16 / 22 / 25 / 32 / 44 / 56 的图片模式；录音采样率要等于该模式的标称采样率）。
  /// 正文 `text` 里照常保留 `pic%X` 报头，同步音与图像段的乱码已经从正文里去掉。
  pub pictures: Vec<Picture>,
  /// 正文里有图片报头，但没有解出图片（采样率不是该模式的标称值、找不到同步音、报头之后没有录到图像、
  /// 报头数量超过上限）。页面据此提示，不然用户只看到一段缺了图的正文。
  pub picture_skipped: bool,
}

/// 一段连续 THOR 文本的解码结果，加上里面找到的图片报头。
struct Segment {
  decoded: Decoded,
  /// 这段文本里识别出的图片报头（按出现顺序），以及主信道正文到报头为止的文字
  /// （报头之后 fldigi 转入图像接收，不再收字符；离线解码在这里去掉同步音之前那段静音译出的乱码）。
  headers: Vec<(Header, String)>,
}

fn grid_for(rate: u32, mode: &Mode) -> Option<Grid> {
  Grid::new(rate, mode.baud(), mode.spacing(), NUMTONES)
}

/// 估计信号**最低音调**的频率（Hz）：见 [`ham_web_mfsk::front::detect_base`]。
#[must_use]
pub fn detect_base(samples: &[f32], rate: u32, mode: &Mode) -> Option<f64> {
  detect_base_grid(samples, &grid_for(rate, mode)?)
}

/// fldigi `thorvaridec`：码字 → 字符；二次信道字符返回 `0x100 | 字符`。
fn thor_varidec(code: u32) -> Option<u16> {
  if code < SECONDARY_MIN {
    return varidec(code);
  }
  SECONDARY
    .iter()
    .position(|s| u32::from_str_radix(s, 2) == Ok(code))
    .map(|i| 0x100 | (u16::from(b' ') + i as u16))
}

/// 音调判决 + IFK+ 差分 + 解交织 + 译码 + 切字符。
struct Receiver {
  depth: usize,
  traceback: usize,
  inlv: Interleaver,
  vit: Viterbi,
  pair: [u8; 2],
  first_of_pair: bool,
  datashreg: u32,
  prev: Option<usize>,
  chars: Vec<u16>,
  scanner: HeaderScanner,
  /// 识别出的图片报头，以及补完它时 `chars` 已有多少个字符。
  headers: Vec<(Header, usize)>,
  /// 参与差分的 nibble 数与其中音调差不合法的个数。
  events: usize,
  invalid: usize,
}

impl Receiver {
  fn new(mode: &Mode) -> Self {
    Self {
      depth: mode.depth,
      traceback: mode.traceback(),
      inlv: Interleaver::new(4, mode.depth, false),
      vit: Viterbi::new(mode.code(), mode.traceback()),
      pair: [0; 2],
      first_of_pair: false,
      datashreg: 1,
      prev: None,
      chars: Vec::new(),
      scanner: HeaderScanner::default(),
      headers: Vec::new(),
      events: 0,
      invalid: 0,
    }
  }

  /// 一个符号的各音调幅度 → 与上一个符号的音调差 → 4 个码位软值。
  fn feed(&mut self, bins: &[f64]) {
    let n = bins.len() as f64;
    let avg = (bins.iter().sum::<f64>() / n).max(1e-20);
    let mut best = (0.0f64, 0usize);
    for (i, &x) in bins.iter().enumerate() {
      if x > best.0 {
        best = (x, i);
      }
    }
    let burst = best.0 / avg < STATIC_BURST_RATIO;
    let Some(prev) = self.prev.replace(best.1) else {
      return;
    };
    let d = best.1 as i32 - prev as i32;
    self.events += 1;
    // 合法的音调差是 2..=17 或 −16..=−1；0 / 1 / −17 不是 IFK+ 能产生的。
    let valid = !matches!(d, 0 | 1 | -17);
    if !valid {
      self.invalid += 1;
    }
    let mut syms = [PUNCTURE; 4];
    if valid && !burst {
      let c = (d - 2).rem_euclid(NUMTONES as i32) as u8;
      for (i, s) in syms.iter_mut().enumerate() {
        *s = if (c >> (3 - i)) & 1 == 1 { 255 } else { 0 };
      }
    }
    self.inlv.symbols(&mut syms);
    for s in syms {
      self.decode_pair(s);
    }
  }

  /// 信号结束后把管线里压着的字符推出来：喂若干个「全是删除位」的 nibble。
  ///
  /// 反向交织器（`3 × 深度` 个 nibble）与 Viterbi 回溯（每个 nibble 译出 2 位）合起来有一段固定延迟，
  /// 信号最后那几个字符要等这么多个 nibble 才流得出来 —— THOR 发端只补 4 个空闲符的冲刷，远不够填满
  /// 这段延迟，fldigi 靠接收机收完信号后继续运行（噪声把它们推出来）。离线文件没有「后面」，所以在这里补。
  /// 删除位的度量增量为 0，不会改变已有路径的取舍，只是让回溯窗口往前走。
  fn flush(&mut self) {
    for _ in 0..3 * self.depth + self.traceback.div_ceil(2) {
      let mut syms = [PUNCTURE; 4];
      self.inlv.symbols(&mut syms);
      for s in syms {
        self.decode_pair(s);
      }
    }
  }

  /// fldigi `decodePairs`：软符号两两配对进 Viterbi，输出位按 `001` 切字符。
  fn decode_pair(&mut self, symbol: u8) {
    self.pair = [self.pair[1], symbol];
    self.first_of_pair = !self.first_of_pair;
    if self.first_of_pair {
      return;
    }
    let (bit, _) = self.vit.decode(self.pair);
    self.datashreg = (self.datashreg << 1) | u32::from(bit);
    if self.datashreg & 7 == 1 {
      if let Some(c) = thor_varidec(self.datashreg >> 1) {
        self.chars.push(c);
        if c < 0x100
          && let Some(header) = self.scanner.push(c)
        {
          self.headers.push((header, self.chars.len()));
        }
      }
      self.datashreg = 1;
    }
  }
}

/// 完整解码：音频 → 文本。
///
/// - `center`：信号中心频率（Hz）；`None` 时自动估计（[`detect_base`]）；
/// - `reverse`：边带反转（LSB 录音）。
///
/// 噪声与静音返回 `None`：峰均比划不出信号区间、区间的平均峰均比不够、或区间里不合法的音调差
/// 太多（见 [`ham_web_mfsk::front::gate`] 与 [`MAX_INVALID_PERCENT`]）。
#[must_use]
pub fn decode_audio(
  samples: &[f32],
  rate: u32,
  mode: &Mode,
  center: Option<f64>,
  reverse: bool,
) -> Option<Decoded> {
  decode_with_pictures(samples, rate, mode, center, reverse, MAX_PICTURES)
}

/// 一段录音里最多处理多少张图片（防止畸形输入把递归拖得很深）。
const MAX_PICTURES: usize = 4;

/// 文本 + 图片：先按普通文本解一遍；它不像干净的纯文本（判无信号 / 乱码 / 文本里读到了报头）时，
/// 再在整段录音里找同步音，找到了就把录音切成「图像之前的文本 / 图像 / 图像之后」三段，
/// 文本各自解码、图像做 FM 解调；没有同步音就退回那段普通文本。
///
/// 为什么图片要靠同步音、而不是靠整段解码出来的报头：整段录音里夹着图像时，图像段会让区间里不合法的音调差占比
/// 远超门限（[`MAX_INVALID_PERCENT`]），整段解码直接判无信号；同步音又长又固定、在 18 个音调所占的带之外
/// （见 [`crate::picture`]），而且它是否存在本身就能验证中心频率。
fn decode_with_pictures(
  samples: &[f32],
  rate: u32,
  mode: &Mode,
  center: Option<f64>,
  reverse: bool,
  pictures_left: usize,
) -> Option<Decoded> {
  let plain = decode_segment(samples, rate, mode, center, reverse);
  if !mode.has_picture() {
    return plain.map(|s| s.decoded);
  }
  // 图片的像素样本数（10）是按模式的标称采样率给的，别的采样率要先重采样，这里不做。
  if rate != mode.rate || pictures_left == 0 {
    return plain.map(|s| Decoded {
      picture_skipped: !s.headers.is_empty(),
      ..s.decoded
    });
  }
  // 绝大多数录音只有文本：整段解出来是可读文本、里面又没有图片报头，就不必再去找同步音
  // （整段瞬时频率 + 最多 6 个候选中心频率，比解码本身还贵）。录音里夹着图像时，整段解码要么判无信号，
  // 要么译出乱码，要么至少能在图像之前的文本里读到报头，三种都会落到下面的同步音搜索。
  if let Some(seg) = &plain
    && seg.headers.is_empty()
    && printable_fraction(&seg.decoded.text) >= 0.9
  {
    return plain.map(|s| s.decoded);
  }
  let grid = grid_for(rate, mode)?;
  // 中心频率：给了就用；没给就自动检测。录音里夹着图像时整段检测可能被带偏（图像是调频、不在音调栅格上），
  // 所以再试只用开头 30% / 50% 的检测结果；哪一个找得到同步音，就说明它对。
  let mut candidates: Vec<f64> = Vec::new();
  match center {
    Some(c) => candidates.push(c),
    None => {
      // 相差不到 0.05 × 带宽（不足一个音调间隔）的候选等价：同步音的搜索带两侧各有约 0.12 × 带宽的余量，
      // 估计差这么点不会改变结果，各扫一遍整段 FIR 只是白白多花时间（选错子模式 / 弱信号时候选会全部落空）。
      let merge_hz = 0.05 * mode.bandwidth();
      for fraction in [1.0, 0.5, 0.3, 0.2, 0.12, 0.08] {
        let part = &samples[..((samples.len() as f64 * fraction) as usize)];
        if let Some(base) = detect_base_grid(part, &grid) {
          let c = base - mode.tone_offset(0);
          if !candidates.iter().any(|&known| (known - c).abs() < merge_hz) {
            candidates.push(c);
          }
        }
      }
    }
  }
  for c in candidates {
    if let Some(decoded) = with_picture(samples, rate, mode, c, reverse, &grid, pictures_left) {
      return Some(decoded);
    }
  }
  // 没有一个候选走通：退回纯文本；文本里有报头就说明图片没解出来。
  plain.map(|s| Decoded {
    picture_skipped: !s.headers.is_empty(),
    ..s.decoded
  })
}

/// [`decode_with_pictures`] 的图片分支：以 `c` 为中心频率的估计找同步音，找到了就把录音切成
/// 「图像之前的文本 / 图像 / 图像之后」三段。任何一步走不通（没有同步音、之前没有报头、图像不在录音里）返回 `None`。
fn with_picture(
  samples: &[f32],
  rate: u32,
  mode: &Mode,
  c: f64,
  reverse: bool,
  grid: &Grid,
  pictures_left: usize,
) -> Option<Decoded> {
  let sync = picture::find_sync(samples, rate, c, mode.bandwidth(), grid.sym, reverse)?;
  let (sync_begin, start) = (sync.begin, sync.start);
  // 同步音的频率比文本里的梳检测准得多：用它推出来的中心频率去解文本与图像。
  let c = sync.center_hz(mode.bandwidth(), reverse);
  // 同步音之前的文本，里面要有报头；没有就当这个音不是图片的同步音。
  let cut = (sync_begin.floor() as usize).min(samples.len());
  let before = decode_segment(&samples[..cut], rate, mode, Some(c), reverse)?;
  // 取**最后**一个报头：同步音紧跟在报头之后，报头与同步音之间只会隔着冲刷的静音。
  // （MFSK 那边相反 —— 图像段的起点是从报头位置推出来的，所以用第一个，见 `mfsk::rx`。）
  let (header, through) = before.headers.last().cloned()?;
  // 图像之前这一小段里没有图像，跟踪是干净的：它的中心频率才是发端的中心频率。
  let center_hz = before.decoded.center_hz;
  let demod = FmDemod::thor(rate, center_hz, mode.bandwidth());
  let picture = decode_picture(samples, &demod, start, &header, reverse)?;
  let end = (start + header.samples() as f64).ceil() as usize;
  let mut decoded = before.decoded;
  decoded.text = through;
  decoded.pictures = vec![picture];
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
    decoded.id.push_str(&after.id);
    decoded.symbols += after.symbols;
    decoded.pictures.extend(after.pictures);
    decoded.picture_skipped |= after.picture_skipped;
  }
  Some(decoded)
}

/// 一段连续 THOR 文本的解码（不处理图片）。
fn decode_segment(
  samples: &[f32],
  rate: u32,
  mode: &Mode,
  center: Option<f64>,
  reverse: bool,
) -> Option<Segment> {
  let grid = grid_for(rate, mode)?;
  let (base, auto) = match center {
    Some(c) if c.is_finite() => (c + mode.tone_offset(0), false),
    Some(_) => return None,
    None => (detect_base_grid(samples, &grid)?, true),
  };
  if !grid.fits(base) {
    return None;
  }
  let offset = find_offset(samples, &grid, base);
  // 自动检测的中心频率在符号对齐之后再核对一遍；用户给定的不动。
  let base = if auto {
    polish_base(samples, &grid, base, offset, POLISH_REACH)
  } else {
    base
  };
  if !grid.fits(base) {
    return None;
  }
  let nt = grid.ntones;
  // 时钟误差与频漂：没有值得做的修正时 `drift` 就是不跟踪的网格，`bins` 与 `symbol_bins` 逐位相同。
  let (mut bins, drift) = bins_tracked(samples, &grid, base, offset);
  // 硬判决的接收机更需要「抹」：比信号强的载波在每个符号里都是最强，不抹掉就会判出一串恒定音调。
  // 返回值（两种载波的个数）只给测试与诊断用，解码路径不看。
  clean_carriers(&mut bins, nt);
  let (first, last) = gate(&peak_ratios(&bins, nt))?;
  let span = last - first + 1;
  let mid_base = drift.base_at((first + last) as f64 / 2.0);

  let mut rx = Receiver::new(mode);
  let mut tmp = vec![0.0f64; nt];
  for chunk in bins.chunks_exact(nt).skip(first).take(span) {
    if reverse {
      for (t, &v) in tmp.iter_mut().zip(chunk.iter().rev()) {
        *t = v;
      }
      rx.feed(&tmp);
    } else {
      rx.feed(chunk);
    }
  }
  rx.flush();
  if rx.events == 0 || rx.invalid * 100 > rx.events * MAX_INVALID_PERCENT {
    return None;
  }
  let primary: Vec<u16> = rx.chars.iter().copied().filter(|&c| c < 0x100).collect();
  let secondary: Vec<u16> = rx
    .chars
    .iter()
    .filter(|&&c| c >= 0x100)
    .map(|&c| c & 0xFF)
    .collect();
  Some(Segment {
    decoded: Decoded {
      text: render(&primary),
      id: render(&secondary),
      center_hz: mid_base - mode.tone_offset(0),
      symbols: span,
      clock_ppm: drift.clock_ppm(),
      drift_hz_per_s: drift.drift_hz_per_s(),
      pictures: Vec::new(),
      picture_skipped: false,
    },
    headers: if mode.has_picture() {
      rx.headers
        .iter()
        .map(|&(header, n)| {
          let through: Vec<u16> = rx.chars[..n]
            .iter()
            .copied()
            .filter(|&c| c < 0x100)
            .collect();
          (header, render(&through))
        })
        .collect()
    } else {
      Vec::new()
    },
  })
}
