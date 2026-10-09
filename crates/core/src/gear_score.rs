//! 机型对比的「轴」与打分：把器材库的文本字段与接收机实测指标折算成可比的数值。
//!
//! 三件事在这里定死，界面只负责画：
//!
//! 1. **轴**（[`Axis`]）：六条，取值范围与「越大越好还是越小越好」都写在 [`Axis::range`] /
//!    [`Axis::higher_is_better`] 里。区间是**本库内可比**的固定标尺，不是行业绝对标准 ——
//!    单测保证它覆盖库里所有机型的实际取值，越界就说明该更新标尺了。
//! 2. **权重预设**（[`PRESETS`]）：DX / 竞赛 / 野外 / 卫星，各自对六条轴加权（和为 1）。
//! 3. **缺失即跳过**（[`score`]）：某条轴没数据时不计入，权重按有数据的轴**重新归一化**；
//!    有数据的轴少于两条就判「依据不足」（`points == None`），而不是拿一条轴硬算出一个分数。
//!
//! 功率与频段上限取对数标尺（5 W → 100 W 是 20 倍，线性会把它们全挤在低端）；
//! 动态范围与噪声底本身就是 dB，直接用线性 ✓。

use crate::gear::Gear;
use crate::gear_rx::GearRx;
use crate::gear_user::UserMeasurement;

/// 归一化后的最小值：0 值也要离圆心一点距离，否则多边形会塌成一条线。
pub const MIN_RADIUS: f64 = 0.12;

/// 一台机器的**有效**接收机指标：原表值 + 用户自己测的值（测了的项盖过原表）。
///
/// 打分与雷达图都只看这个结构：**数值的来源不影响几何** —— 界面只负责把「这一项是你测的」
/// 标出来（对比图里画成虚线）。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RxValues {
  /// 20 kHz 间隔三阶互调动态范围（dB）。
  pub imd_wide_db: Option<f64>,
  /// 窄间隔（2 kHz）三阶互调动态范围（dB）。
  pub imd_narrow_db: Option<f64>,
  /// 噪声底（dBm）。
  pub noise_floor_dbm: Option<f64>,
}

impl RxValues {
  /// 只用原表值（库里没有这台机器、或原表没测过它时全空）。
  #[must_use]
  pub fn from_table(rx: Option<&GearRx>) -> Self {
    Self {
      imd_wide_db: rx.map(|r| r.imd_wide_db),
      imd_narrow_db: rx.map(|r| r.imd_narrow_db),
      noise_floor_dbm: rx.map(|r| r.noise_floor_dbm),
    }
  }

  /// 叠加用户实测值：测了的项盖过原表，没测的沿用原表。
  ///
  /// 只认有可比基准的两项（噪声底、RMDR）。电流与相位噪声在本库没有基准（原表也不给），
  /// 硬塞进来只会得到一条没有意义的对比 —— 它们只作个人记录。
  #[must_use]
  pub fn with_user(mut self, user: Option<&UserMeasurement>) -> Self {
    if let Some(m) = user {
      if let Some(v) = m.noise_floor_dbm {
        self.noise_floor_dbm = Some(v);
      }
      if let Some(v) = m.rmdr_db {
        self.imd_narrow_db = Some(v);
      }
    }
    self
  }
}

/// 按器材 id 组装有效指标（原表 + 用户实测）。
#[must_use]
pub fn rx_values(gear_id: &str, user: Option<&UserMeasurement>) -> RxValues {
  RxValues::from_table(crate::gear_rx::for_gear(gear_id)).with_user(user)
}

/// 一条对比轴。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
  /// 窄间隔（2–3 kHz）三阶互调动态范围 —— 强台环境下的抗互调能力。
  ImdNarrow,
  /// 20 kHz 间隔三阶互调动态范围。
  ImdWide,
  /// 接收机噪声底（越小越好）。
  NoiseFloor,
  /// 发射功率。
  Power,
  /// 频段上限（能不能上 VHF / UHF）。
  TopBand,
  /// 支持的模式数。
  Modes,
}

/// 归一化用的标尺类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scale {
  /// 线性。
  Linear,
  /// 对数十倍程（频率、功率这类跨一个量级以上的量）。
  Log,
}

impl Axis {
  /// 全部轴（雷达图的顶点顺序）。
  pub const ALL: [Self; 6] = [
    Self::ImdNarrow,
    Self::ImdWide,
    Self::NoiseFloor,
    Self::Power,
    Self::TopBand,
    Self::Modes,
  ];

  /// 稳定短键（持久化与界面用）。
  #[must_use]
  pub const fn key(self) -> &'static str {
    match self {
      Self::ImdNarrow => "imd-narrow",
      Self::ImdWide => "imd-wide",
      Self::NoiseFloor => "noise-floor",
      Self::Power => "power",
      Self::TopBand => "top-band",
      Self::Modes => "modes",
    }
  }

  /// 由短键还原。
  #[must_use]
  pub fn from_key(key: &str) -> Option<Self> {
    Self::ALL.into_iter().find(|a| a.key() == key)
  }

  /// 是不是「数字越大越好」（噪声底相反）。
  #[must_use]
  pub const fn higher_is_better(self) -> bool {
    !matches!(self, Self::NoiseFloor)
  }

  /// 标尺类型。
  #[must_use]
  pub const fn scale(self) -> Scale {
    match self {
      Self::Power | Self::TopBand => Scale::Log,
      _ => Scale::Linear,
    }
  }

  /// 本库内可比的取值范围 `(最差端, 最好端)`（与 [`Axis::higher_is_better`] 无关）。
  ///
  /// 数值来自库里机型的实际区间并两端留了余量：越界时单测会红，提醒你更新标尺而不是
  /// 让新机型被算成负分。
  #[must_use]
  pub const fn range(self) -> (f64, f64) {
    match self {
      // 收录的实测值 70–110 dB。
      Self::ImdNarrow | Self::ImdWide => (60.0, 115.0),
      // 收录的实测值 −133 … −120 dBm。
      Self::NoiseFloor => (-140.0, -100.0),
      // 5 W 手持到 100 W 基地台（含天调/功放的标称）。
      Self::Power => (5.0, 150.0),
      // 下限取 10m 段的上界（29.7：只写「HF（80–10m）」的机器也算得出来），
      // 上限到 SDR 的 2 GHz。
      Self::TopBand => (28.0, 2500.0),
      // 「由软件解调」这类不是模式表，会被判成没有数据。
      Self::Modes => (1.0, 8.0),
    }
  }
}

/// 从频段写法里认出业余波段名 → 上限 MHz。
///
/// 复用 [`crate::bands::AMATEUR_BAND_EDGES`]（唯一事实来源）：`2m` / `70cm` 这类写法本身
/// 没有数字，各版本自己写一份波长表迟早会不一致。匹配要求**前面不是数字**，
/// 否则 `12m` 会被 `2m` 抢先命中（144 MHz 是错的，24.89 才对）。
fn band_name_top_mhz(text: &str) -> Option<f64> {
  let mut top: Option<f64> = None;
  for &(name, _, hi) in crate::bands::AMATEUR_BAND_EDGES {
    let mut from = 0usize;
    while let Some(pos) = text[from..].find(name) {
      let at = from + pos;
      let prev_is_digit = text[..at]
        .chars()
        .next_back()
        .is_some_and(|c| c.is_ascii_digit());
      if !prev_is_digit {
        top = Some(top.map_or(hi, |t: f64| t.max(hi)));
        break;
      }
      from = at + name.len();
    }
  }
  top
}

/// 数字前面是否紧跟否定词（`HF（不含 50MHz）` 里的 50 不算上限）。
///
/// 括号里既有限定（`不含 50MHz` / `接收` / `标称`）也有规格（`80–10m`），一刀切摘括号会
/// 把规格一起丢掉，所以只针对否定词：命中就跳过这个数字。
fn is_negated(text: &str, at: usize) -> bool {
  const NEGATIONS: &[&str] = &["不含", "不包括", "无", "非", "除"];
  let tail: String = text[..at]
    .chars()
    .rev()
    .take(4)
    .collect::<Vec<_>>()
    .into_iter()
    .rev()
    .collect();
  NEGATIONS.iter().any(|n| tail.contains(n))
}

/// 功率写法 → 瓦数（`100W 级` / `QRP 10W 级` / `5W 级（标称）` → 100 / 10 / 5）。
///
/// 认不出（`接收机` / `室外自动天调`）时返回 `None`：附件与纯接收机不该在这一轴上得分。
#[must_use]
pub fn power_w(power: &str) -> Option<f64> {
  let lower = power.replace(' ', "");
  let digits: String = lower
    .char_indices()
    .skip_while(|(_, c)| !c.is_ascii_digit())
    .take_while(|(_, c)| c.is_ascii_digit() || *c == '.')
    .map(|(_, c)| c)
    .collect();
  let value: f64 = digits.parse().ok()?;
  // 数字后面必须紧跟 W（否则「80–10m」里的数字会被当成功率）。
  let after = lower.split(&digits).nth(1).unwrap_or_default();
  (value > 0.0 && after.starts_with('W')).then_some(value)
}

/// 频段写法 → 上限 MHz（`HF + 50MHz` → 50，`2m / 70cm` → 430，`约 24MHz–1.7GHz` → 1700）。
///
/// 认不出时返回 `None`：`HF / VHF / UHF` 是**类别**不是频段，折算成 144/430 会是编数据。
#[must_use]
pub fn top_band_mhz(bands: &str) -> Option<f64> {
  let lower = bands.to_ascii_lowercase();
  let mut top = band_name_top_mhz(bands);
  let mut i = 0usize;
  while i < lower.len() {
    let rest = &lower[i..];
    // 单位按「谁在前」定：`1.7GHz` 与 `24MHz` 混排时不能只看一个。
    let (pos, factor) = match (rest.find("mhz"), rest.find("ghz")) {
      (Some(m), Some(g)) => {
        if m < g {
          (m, 1.0)
        } else {
          (g, 1000.0)
        }
      }
      (Some(m), None) => (m, 1.0),
      (None, Some(g)) => (g, 1000.0),
      (None, None) => break,
    };
    // 单位前面紧挨着的数字就是数值。
    let digits: String = rest[..pos]
      .chars()
      .rev()
      .take_while(|c| c.is_ascii_digit() || *c == '.')
      .collect::<Vec<_>>()
      .into_iter()
      .rev()
      .collect();
    let num_start = i + pos - digits.len();
    if let Ok(v) = digits.parse::<f64>()
      && !is_negated(&lower, num_start)
    {
      let mhz = v * factor;
      top = Some(top.map_or(mhz, |t: f64| t.max(mhz)));
    }
    i += pos + 3;
  }
  top
}

/// 模式写法 → 能认出来的模式个数（`SSB / CW / AM / FM / RTTY / 数字` → 6）。
///
/// `由软件解调` / `自动天线调谐器` 这类不是模式表 → `None`。
#[must_use]
pub fn mode_count(modes: &str) -> Option<usize> {
  const KNOWN: &[&str] = &[
    "CW", "SSB", "AM", "FM", "RTTY", "PSK", "FT8", "数字", "DMR", "C4FM", "D-STAR", "APRS", "DV",
  ];
  let items: Vec<&str> = modes
    .split('/')
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .collect();
  if items.is_empty() {
    return None;
  }
  let known = items
    .iter()
    .filter(|item| {
      let upper = item.to_ascii_uppercase();
      KNOWN.iter().any(|k| upper.contains(k))
    })
    .count();
  (known == items.len()).then_some(known)
}

/// 某条轴在这台器材上的原始取值（缺数据时 `None`）。
#[must_use]
pub fn value(axis: Axis, gear: &Gear, rx: &RxValues) -> Option<f64> {
  match axis {
    Axis::ImdNarrow => rx.imd_narrow_db,
    Axis::ImdWide => rx.imd_wide_db,
    Axis::NoiseFloor => rx.noise_floor_dbm,
    Axis::Power => power_w(gear.power),
    Axis::TopBand => top_band_mhz(gear.bands),
    Axis::Modes => mode_count(gear.modes).map(|n| n as f64),
  }
}

/// 把原始值折算成「好坏比例」`0..=1`（1 = 该轴最好，0 = 最差），再抬到 [`MIN_RADIUS`] 以上。
#[must_use]
pub fn normalize(axis: Axis, raw: f64) -> f64 {
  let (worst, best) = axis.range();
  let t = match axis.scale() {
    Scale::Linear => (raw - worst) / (best - worst),
    Scale::Log => {
      let (lo, hi) = (worst.max(1e-9).ln(), best.max(1e-9).ln());
      (raw.max(1e-9).ln() - lo) / (hi - lo)
    }
  };
  let t = t.clamp(0.0, 1.0);
  let good = if axis.higher_is_better() { t } else { 1.0 - t };
  MIN_RADIUS + good * (1.0 - MIN_RADIUS)
}

/// 一台机型在某个预设下的得分。
#[derive(Debug, Clone, PartialEq)]
pub struct Score {
  /// 分数：每条轴折算后的比例（最低 [`MIN_RADIUS`]）乘 100，所以下界是
  /// `MIN_RADIUS * 100` 而不是 0；有数据的轴少于两条时是 `None`（依据不足，不硬算）。
  pub points: Option<f64>,
  /// 该预设用到的轴里，这台机型**有数据**的那几条。
  pub used: Vec<Axis>,
  /// 缺数据的轴。
  pub missing: Vec<Axis>,
}

impl Score {
  /// 是否依据不足（界面据此显示提示而不是分数）。
  #[must_use]
  pub fn is_insufficient(&self) -> bool {
    self.points.is_none()
  }
}

/// 权重预设：一组「按什么用途选机」的权重。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Preset {
  /// 稳定短键。
  pub key: &'static str,
  /// 各轴权重（只列出用到的轴；权重和为 1）。
  pub weights: &'static [(Axis, f64)],
}

impl Preset {
  /// 该预设用到的轴（按 [`Axis::ALL`] 的顺序，雷达图顶点顺序一致）。
  #[must_use]
  pub fn axes(self) -> Vec<Axis> {
    Axis::ALL
      .into_iter()
      .filter(|a| self.weights.iter().any(|(w, _)| w == a))
      .collect()
  }
}

/// 四个预设：DX / 竞赛 / 野外 / 卫星。
pub const PRESETS: &[Preset] = &[
  Preset {
    key: "dx",
    // 追稀有台：弱信号接收能力放最重（动态范围 + 噪声底），发射功率最不重要。
    weights: &[
      (Axis::ImdNarrow, 0.35),
      (Axis::NoiseFloor, 0.35),
      (Axis::ImdWide, 0.10),
      (Axis::TopBand, 0.10),
      (Axis::Power, 0.05),
      (Axis::Modes, 0.05),
    ],
  },
  Preset {
    key: "contest",
    // 比赛：强台扎堆，抗互调是第一位。
    weights: &[
      (Axis::ImdNarrow, 0.40),
      (Axis::ImdWide, 0.30),
      (Axis::NoiseFloor, 0.20),
      (Axis::Power, 0.10),
    ],
  },
  Preset {
    key: "field",
    // 野外 / POTA：带得动、够用、能上高频段。
    weights: &[
      (Axis::Power, 0.35),
      (Axis::TopBand, 0.30),
      (Axis::NoiseFloor, 0.20),
      (Axis::ImdNarrow, 0.15),
    ],
  },
  Preset {
    key: "satellite",
    // 卫星：先看有没有 2m/70cm，再看模式与接收质量。
    weights: &[
      (Axis::TopBand, 0.45),
      (Axis::Modes, 0.20),
      (Axis::NoiseFloor, 0.20),
      (Axis::ImdNarrow, 0.15),
    ],
  },
];

/// 默认预设（页面的初始选择）。
pub const DEFAULT_PRESET: &str = "dx";

/// 按短键取预设。
#[must_use]
pub fn preset(key: &str) -> Option<&'static Preset> {
  PRESETS.iter().find(|p| p.key == key)
}

/// 打分：只按**有数据**的轴加权，缺的轴跳过并把权重归一化；有数据的轴少于两条即判依据不足。
#[must_use]
pub fn score(gear: &Gear, rx: &RxValues, preset: &Preset) -> Score {
  let mut weighted = 0.0;
  let mut total = 0.0;
  let mut used = Vec::new();
  let mut missing = Vec::new();
  for &(axis, weight) in preset.weights {
    match value(axis, gear, rx) {
      Some(raw) => {
        weighted += weight * normalize(axis, raw);
        total += weight;
        used.push(axis);
      }
      None => missing.push(axis),
    }
  }
  let points = (used.len() >= 2 && total > 0.0).then(|| weighted / total * 100.0);
  Score {
    points,
    used,
    missing,
  }
}

/// 雷达图顶点：单位圆坐标（y **向上**，第一条轴在正上方，顺时针），半径 = 归一化值。
///
/// 界面负责缩放与翻转 y（SVG 的 y 朝下）；把几何留在核心是为了能单测。
#[must_use]
pub fn radar_points(values: &[f64]) -> Vec<(f64, f64)> {
  let n = values.len();
  if n == 0 {
    return Vec::new();
  }
  let step = std::f64::consts::TAU / n as f64;
  values
    .iter()
    .enumerate()
    .map(|(i, &r)| {
      let angle = std::f64::consts::FRAC_PI_2 - i as f64 * step;
      let r = r.clamp(0.0, 1.0);
      (r * angle.cos(), r * angle.sin())
    })
    .collect()
}

/// 轴名文字的中心半径（图形外圈 1.0 之外一点，免得压到多边形）。
pub const LABEL_RADIUS: f64 = 1.22;

/// 轴名字号（SVG 用户单位）。
pub const LABEL_FONT: f64 = 0.09;

/// 轴名的对齐方式：按标签位于圆心的哪一侧。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelAnchor {
  /// 靠右：文字从锚点向右展开。
  Start,
  /// 靠左：文字从锚点向左展开。
  End,
  /// 正上 / 正下：以锚点居中。
  Middle,
}

impl LabelAnchor {
  /// 由标签的横坐标定对齐方式。阈值与旧实现一致（`±0.15`），改这里等于改所有页面。
  #[must_use]
  pub const fn of(x: f64) -> Self {
    if x > 0.15 {
      Self::Start
    } else if x < -0.15 {
      Self::End
    } else {
      Self::Middle
    }
  }

  /// `text-anchor` 的取值。
  #[must_use]
  pub const fn css(self) -> &'static str {
    match self {
      Self::Start => "start",
      Self::End => "end",
      Self::Middle => "middle",
    }
  }
}

/// 文字在用户单位下的估算宽度：CJK 按 1em、其余按 0.55em（拉丁字母平均宽度）。
///
/// 估宽只为「视口够不够放」：真实排版要问浏览器，而这条判定要能进核心单测
/// （见 [`radar_viewbox`]）。
#[must_use]
pub fn label_width(text: &str) -> f64 {
  text
    .chars()
    .map(|c| if c.is_ascii() { 0.55 } else { 1.0 })
    .sum::<f64>()
    * LABEL_FONT
}

/// 雷达图视口 `(min_x, min_y, width, height)`，直接喂给 SVG 的 `viewBox`。
///
/// 视口按**当前语言**的轴名实宽撑开：标签长度与语言强相关（`Power` 5 个字符，
/// 西语 `DR de separación estrecha` 26 个），写死一个宽度必然在某一种语言里被裁掉。
#[must_use]
pub fn radar_viewbox(labels: &[String]) -> (f64, f64, f64, f64) {
  let n = labels.len().max(1);
  // 图形外圈（半径 1）两侧留一点呼吸位。
  let mut half_w = LABEL_RADIUS;
  let mut half_h = LABEL_RADIUS;
  for (label, (x, y)) in labels.iter().zip(radar_points(&vec![LABEL_RADIUS; n])) {
    let w = label_width(label);
    let (left, right) = match LabelAnchor::of(x) {
      LabelAnchor::Start => (x, x + w),
      LabelAnchor::End => (x - w, x),
      LabelAnchor::Middle => (x - w / 2.0, x + w / 2.0),
    };
    half_w = half_w.max(right).max(-left);
    half_h = half_h.max(y.abs() + LABEL_FONT);
  }
  (-half_w, -half_h, half_w * 2.0, half_h * 2.0)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::gear::{GEAR, by_id};

  fn rx(id: &str) -> Option<&'static GearRx> {
    crate::gear_rx::for_gear(id)
  }

  /// 测试里最常用的形态：只用原表值。
  fn table(id: &str) -> RxValues {
    RxValues::from_table(rx(id))
  }

  #[test]
  fn presets_are_well_formed() {
    assert_eq!(PRESETS.len(), 4);
    assert!(preset(DEFAULT_PRESET).is_some());
    let mut keys: Vec<&str> = PRESETS.iter().map(|p| p.key).collect();
    let before = keys.len();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), before, "预设短键重复");
    for p in PRESETS {
      let sum: f64 = p.weights.iter().map(|(_, w)| w).sum();
      assert!((sum - 1.0).abs() < 1e-9, "{} 的权重和是 {sum}", p.key);
      let mut axes: Vec<Axis> = p.weights.iter().map(|(a, _)| *a).collect();
      let before = axes.len();
      axes.sort_unstable_by_key(|a| a.key());
      axes.dedup();
      assert_eq!(axes.len(), before, "{} 里同一条轴出现两次", p.key);
      for (_, w) in p.weights {
        assert!(*w > 0.0, "{} 里有零权重", p.key);
      }
      // axes() 的顺序必须与雷达顶点顺序一致。
      assert_eq!(p.axes().len(), p.weights.len());
      let mut sorted = p.axes();
      sorted.sort_by_key(|a| a.key());
      axes.sort_unstable_by_key(|a| a.key());
      assert_eq!(sorted, axes);
    }
  }

  #[test]
  fn the_scale_covers_every_rig_in_the_library() {
    // 标尺是「本库内可比」的固定区间：库里任何机型的实际取值都必须落在里面，
    // 否则新机型会被算成 0 或 1 分，标尺就该更新了。
    for g in GEAR {
      for axis in Axis::ALL {
        if let Some(raw) = value(axis, g, &RxValues::from_table(rx(g.id))) {
          let (worst, best) = axis.range();
          assert!(
            raw >= worst.min(best) && raw <= worst.max(best),
            "{} 的 {} = {raw} 超出标尺 {worst}–{best}",
            g.id,
            axis.key()
          );
          let n = normalize(axis, raw);
          assert!((MIN_RADIUS..=1.0).contains(&n), "{} {}", g.id, axis.key());
        }
      }
    }
  }

  #[test]
  fn hf_and_portable_rigs_have_a_quantified_top_band() {
    // field / satellite 预设给 TopBand 的权重最高（0.30 / 0.45），HF 台与便携台要是解析不出
    // 频段上限，这条轴会整条被跳过 —— 库里这些主力机型逐个钉住，别再退回 `None`。
    for id in [
      "ic-7300",
      "ft-891",
      "ft-710",
      "ic-7610",
      "ftdx10",
      "xiegu-g90",
      "ic-705",
      "kx2",
      "ft-818",
    ] {
      let g = by_id(id).unwrap();
      assert!(
        top_band_mhz(g.bands).is_some(),
        "{id} 的频段上限（{}）解析不出，TopBand 轴会被跳过",
        g.bands
      );
    }
  }

  #[test]
  fn normalization_respects_direction_and_monotonicity() {
    // 动态范围：越大越好。
    assert!(normalize(Axis::ImdNarrow, 107.0) > normalize(Axis::ImdNarrow, 94.0));
    assert!(normalize(Axis::ImdNarrow, 94.0) > normalize(Axis::ImdNarrow, 70.0));
    // 噪声底：越小越好（−133 比 −120 好）。
    assert!(normalize(Axis::NoiseFloor, -133.0) > normalize(Axis::NoiseFloor, -120.0));
    // 功率与频段是对数标尺：100 W 与 5 W 的差距要明显大于 100 W 与 150 W。
    let (p100, p5, p150) = (
      normalize(Axis::Power, 100.0),
      normalize(Axis::Power, 5.0),
      normalize(Axis::Power, 150.0),
    );
    assert!(p100 - p5 > 2.0 * (p150 - p100), "功率轴的对数标尺没生效");
    // 两端被夹住，不会越界。
    assert_eq!(normalize(Axis::Power, 1.0), MIN_RADIUS);
    assert_eq!(normalize(Axis::ImdNarrow, 999.0), 1.0);
  }

  #[test]
  fn text_parsers_follow_the_librarys_own_wording() {
    // 功率：库里出现过的各种写法。
    assert_eq!(power_w("100W 级"), Some(100.0));
    assert_eq!(power_w("QRP 10W 级"), Some(10.0));
    assert_eq!(power_w("5W 级（标称）"), Some(5.0));
    assert_eq!(power_w("约 100W 级（低功率输入驱动）"), Some(100.0));
    assert_eq!(power_w("接收机"), None, "纯接收机没有发射功率");
    assert_eq!(power_w("室外自动天调"), None);
    // 频段上限。
    assert_eq!(top_band_mhz("HF + 50MHz"), Some(50.0));
    assert_eq!(top_band_mhz("2m / 70cm"), Some(440.0), "70cm 段的顶是 440");
    assert_eq!(
      top_band_mhz("HF + 2m / 70cm"),
      Some(440.0),
      "HF 便携台的顶是 70cm"
    );
    assert!(
      (top_band_mhz("HF（80–10m）").unwrap() - 29.7).abs() < 1e-6,
      "HF 段的顶是 10m 段的上界（复用 bands.rs）"
    );
    assert_eq!(top_band_mhz("约 24MHz–1.7GHz（接收）"), Some(1700.0));
    assert_eq!(top_band_mhz("LF – 2GHz（接收）"), Some(2000.0));
    assert_eq!(top_band_mhz("HF / VHF / UHF"), None, "类别不是频段，不折算");
    assert_eq!(
      top_band_mhz("HF（不含 50MHz）"),
      None,
      "「不含」的那一段不是上限"
    );
    // 模式数。
    assert_eq!(mode_count("SSB / CW / AM / FM / RTTY / 数字"), Some(6));
    assert_eq!(mode_count("FM / APRS"), Some(2));
    assert_eq!(mode_count("由软件解调"), None);
    assert_eq!(mode_count("自动天线调谐器"), None);
  }

  #[test]
  fn the_contest_preset_ranks_the_best_receiver_first() {
    // 真数据上的真结论：FT-710 / FTdx10 的窄间隔动态范围最高（107 dB），G90 最低（76 dB）。
    let contest = preset("contest").expect("竞赛预设");
    let top = score(by_id("ft-710").unwrap(), &table("ft-710"), contest);
    let g90 = score(by_id("xiegu-g90").unwrap(), &table("xiegu-g90"), contest);
    let ic7300 = score(by_id("ic-7300").unwrap(), &table("ic-7300"), contest);
    assert!(!top.is_insufficient() && !g90.is_insufficient());
    assert!(top.points > ic7300.points && ic7300.points > g90.points);
    // 四条轴全有数据，一条都不缺。
    assert!(top.missing.is_empty() && top.used.len() == 4);
  }

  #[test]
  fn a_rig_without_receiver_data_says_so_instead_of_scoring_low() {
    // 手持台不在 Sherwood 表里：竞赛预设只剩功率一条轴 → 依据不足，而不是 0 分。
    let uv5r = by_id("uv-5r").expect("UV-5R");
    assert!(rx("uv-5r").is_none());
    let contest = score(uv5r, &table("uv-5r"), preset("contest").unwrap());
    assert!(contest.is_insufficient(), "只有一条轴不该给出分数");
    assert_eq!(contest.used, vec![Axis::Power]);
    assert_eq!(contest.missing.len(), 3);
    // 卫星预设按「频段 + 模式 + 功率」看，手持台是有数据的。
    let sat = score(uv5r, &table("uv-5r"), preset("satellite").unwrap());
    assert!(!sat.is_insufficient());
    assert!(sat.used.contains(&Axis::TopBand) && sat.used.contains(&Axis::Modes));
  }

  #[test]
  fn radar_points_are_on_the_unit_circle_starting_at_the_top() {
    let points = radar_points(&[1.0, 1.0, 0.5, 0.0]);
    assert_eq!(points.len(), 4);
    // 第一条轴在正上方（y 向上），x 为 0。
    assert!((points[0].0).abs() < 1e-9 && (points[0].1 - 1.0).abs() < 1e-9);
    // 第二条轴在正右方（顺时针 90°）。
    assert!((points[1].0 - 1.0).abs() < 1e-9 && points[1].1.abs() < 1e-9);
    // 半径等于归一化值。
    let r = (points[2].0.powi(2) + points[2].1.powi(2)).sqrt();
    assert!((r - 0.5).abs() < 1e-9);
    assert!(radar_points(&[]).is_empty());
    // 六条轴时首尾对称（正上方那条的对面是正下方）。
    let six = radar_points(&[1.0; 6]);
    assert!((six[3].1 + 1.0).abs() < 1e-9);
  }

  #[test]
  fn every_axis_round_trips_and_is_reachable_from_the_library() {
    for axis in Axis::ALL {
      assert_eq!(Axis::from_key(axis.key()), Some(axis));
      // 至少有一台机型在这条轴上有数据（否则这条轴就是摆设）。
      assert!(
        GEAR
          .iter()
          .any(|g| value(axis, g, &RxValues::from_table(rx(g.id))).is_some()),
        "{} 没有任何机型有数据",
        axis.key()
      );
    }
    assert_eq!(Axis::from_key("nope"), None);
  }

  #[test]
  fn the_viewbox_grows_with_the_language_so_labels_are_not_clipped() {
    let short = vec!["Power".to_owned(); 6];
    let zh = vec!["窄间隔动态范围".to_owned(); 6];
    let es = vec!["DR de separación estrecha".to_owned(); 6];
    let (_, _, w_short, _) = radar_viewbox(&short);
    let (_, _, w_zh, _) = radar_viewbox(&zh);
    let (_, _, w_es, _) = radar_viewbox(&es);
    assert!(
      w_zh > w_short && w_es > w_zh,
      "视口宽度应随标签变宽：{w_short} / {w_zh} / {w_es}"
    );
    for (x, y, w, h) in [
      radar_viewbox(&short),
      radar_viewbox(&zh),
      radar_viewbox(&es),
    ] {
      // 图形本体（半径 1）不能落在视口外，且视口左右对称（图形始终居中）。
      assert!(
        x < -1.0 && y < -1.0 && x + w > 1.0 && y + h > 1.0,
        "图形被裁了"
      );
      assert!((w / 2.0 + x).abs() < 1e-9, "视口不居中");
      assert!((h / 2.0 + y).abs() < 1e-9, "视口不居中");
    }
    // 没有标签时也要给出可用的视口（函数不该 panic）。
    let (x, y, w, h) = radar_viewbox(&[]);
    assert!(w > 2.0 && h > 2.0 && x < 0.0 && y < 0.0);
  }

  #[test]
  fn labels_sit_outside_the_vertex_and_cjk_counts_as_wider() {
    // 正上 / 正下居中，靠左靠右各贴外沿。
    assert_eq!(LabelAnchor::of(0.0), LabelAnchor::Middle);
    assert_eq!(LabelAnchor::of(0.1), LabelAnchor::Middle);
    assert_eq!(LabelAnchor::of(1.06), LabelAnchor::Start);
    assert_eq!(LabelAnchor::of(-1.06), LabelAnchor::End);
    assert_eq!(LabelAnchor::Start.css(), "start");
    // 估宽：CJK 按 1em、ASCII 按 0.55em。
    assert!(label_width("窄间隔") > label_width("abcd"));
    assert!(label_width("") < f64::EPSILON);
  }
}
