//! 像素风「电台前面板」的纯计算：七段读数的字段表与排版、S 表刻度映射、旋钮的落点与步进、
//! 干扰信号（QRM）的频谱特征。
//!
//! 视图只负责把这里算好的格子画成 `<rect>`；为什么用七段屏与 S 表见 `docs/ui/decisions.md` §11.6。

use crate::mistake_book::StudyStats;
use crate::{Bank, ExamRule};

/// S 表的格数：S1–S9 九格，加上 +10 / +20 / +30 / +40 dB 四格。
pub const S_SEGMENTS: u8 = 13;
/// S9 所在的格数：对应合格线。
pub const S9: u8 = 9;
/// 低于这一格（S5）算「该补了」：离合格线还差一半以上，界面上用琥珀标出。
pub const S_WEAK: u8 = 5;

/// 正确率（0–1）映射到 S 表点亮的格数（`0..=S_SEGMENTS`）。
///
/// 合格线 = S9：线以下按比例落在 S0–S8（向下取整 —— 差一点及格不能显示成 S9），
/// 线以上把余量四等分，每份折一格 +10 dB。非有限值或合格线不为正时返回 0。
#[must_use]
pub fn s_units(rate: f64, pass: f64) -> u8 {
  if !rate.is_finite() || !pass.is_finite() || pass <= 0.0 {
    return 0;
  }
  let rate = rate.clamp(0.0, 1.0);
  let pass = pass.min(1.0);
  if rate < pass {
    return (f64::from(S9) * rate / pass).floor().min(f64::from(S9 - 1)) as u8;
  }
  if pass >= 1.0 {
    return S9;
  }
  let over = f64::from(S_SEGMENTS - S9) * (rate - pass) / (1.0 - pass);
  S9 + over.floor().min(f64::from(S_SEGMENTS - S9)) as u8
}

/// 合格线占总题数的比例（0–1）；总题数为 0 时返回 0（`s_units` 把它读成「没有合格线」）。
/// 首页的备考信号与知识频谱共用这一个口径。
#[must_use]
pub fn pass_ratio(rule: &ExamRule) -> f64 {
  if rule.total == 0 {
    0.0
  } else {
    rule.pass as f64 / rule.total as f64
  }
}

/// 用户正在备考的题库与它的合格线比例：作答最多的题库，没有作答记录时取默认题库。
#[must_use]
pub fn main_bank_pass(stats: Option<&StudyStats>) -> (Bank, f64) {
  let bank = stats.and_then(StudyStats::main_bank).unwrap_or_default();
  (bank, pass_ratio(&ExamRule::of(bank)))
}

/// 一次考试的成绩折成 S 表格数（正确率对合格线，见 [`s_units`]）。总题数为 0 —— 没有成绩 —— 时返回 `None`，
/// 界面上画成「无信号」而不是 S0。考试结果弹窗与导出的 QSL 成绩图共用这一个口径。
#[must_use]
pub fn exam_s_units(correct: usize, total: usize, pass_line: usize) -> Option<u8> {
  (total > 0).then(|| {
    s_units(
      correct as f64 / total as f64,
      pass_line as f64 / total as f64,
    )
  })
}

/// S 表读数的文字：`S0`…`S9`，S9 以上写成 `S9+10`…`S9+40`（业余无线电的通行写法，不翻译）。
#[must_use]
pub fn s_reading(units: u8) -> String {
  let units = units.min(S_SEGMENTS);
  if units <= S9 {
    format!("S{units}")
  } else {
    format!("S9+{}", u32::from(units - S9) * 10)
  }
}

/// 一个数字格的宽 / 高（单位：像素格）。
pub const DIGIT_W: u16 = 5;
/// 七段字的高度（像素格）。
pub const DIGIT_H: u8 = 9;

/// 七段的几何：`a`…`g` 依次为 上、右上、右下、下、左下、左上、中，`(x, y, w, h)` 以像素格计。
const SEGMENTS: [(u16, u8, u8, u8); 7] = [
  (1, 0, 3, 1),
  (4, 1, 1, 3),
  (4, 5, 1, 3),
  (1, 8, 3, 1),
  (0, 5, 1, 3),
  (0, 1, 1, 3),
  (1, 4, 3, 1),
];

/// 字符的七段掩码（第 0 位 = `a` … 第 6 位 = `g`）；不认识的字符返回 `None`。
///
/// 字母只收七段屏上画得出、且不会和数字混淆的几个（电台屏上常见的 `S`、`E`、`r`、`P`…）。
#[must_use]
pub const fn segment_mask(ch: char) -> Option<u8> {
  const A: u8 = 1;
  const B: u8 = 1 << 1;
  const C: u8 = 1 << 2;
  const D: u8 = 1 << 3;
  const E: u8 = 1 << 4;
  const F: u8 = 1 << 5;
  const G: u8 = 1 << 6;
  Some(match ch {
    '0' | 'O' => A | B | C | D | E | F,
    '1' => B | C,
    '2' => A | B | D | E | G,
    '3' => A | B | C | D | G,
    '4' => B | C | F | G,
    '5' | 'S' => A | C | D | F | G,
    '6' => A | C | D | E | F | G,
    '7' => A | B | C,
    '8' => A | B | C | D | E | F | G,
    '9' => A | B | C | D | F | G,
    '-' => G,
    ' ' => 0,
    'A' => A | B | C | E | F | G,
    'b' => C | D | E | F | G,
    'C' => A | D | E | F,
    'd' => B | C | D | E | G,
    'E' => A | D | E | F | G,
    'F' => A | E | F | G,
    'H' => B | C | E | F | G,
    'L' => D | E | F,
    'n' => C | E | G,
    'o' => C | D | E | G,
    'P' => A | B | E | F | G,
    'r' => E | G,
    't' => D | E | F | G,
    'U' => B | C | D | E | F,
    'y' => B | C | D | F | G,
    _ => return None,
  })
}

/// 读数里的一个像素块（以像素格计）。`lit = false` 是未点亮的「幽灵段」。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegRect {
  pub x: u16,
  pub y: u8,
  pub w: u8,
  pub h: u8,
  pub lit: bool,
}

/// 一段读数的排版结果：总宽（像素格，高恒为 [`DIGIT_H`]）与全部像素块。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegLayout {
  pub width: u16,
  pub rects: Vec<SegRect>,
}

/// 把读数文字排成七段像素块。
///
/// 数字格宽 5、字间 1 格；`:` 占 3 格、`.` 占 2 格、`+` 占 4 格，它们只画点亮的块、没有幽灵段。
/// 不认识的字符按「全灭的数字格」处理 —— 位置保留，屏幕上看起来是一格没亮的数字，
/// 而不是整段读数错位。
#[must_use]
pub fn layout(text: &str) -> SegLayout {
  // 横坐标是 `u16`：读数最多排这么多个字（每格最宽 6，远不到 65535），超长输入截断而不是溢出。
  const MAX_CHARS: usize = 1000;
  let mut rects = Vec::new();
  let mut x: u16 = 0;
  let lit = |x: u16, y: u8, w: u8, h: u8| SegRect {
    x,
    y,
    w,
    h,
    lit: true,
  };
  for ch in text.chars().take(MAX_CHARS) {
    match ch {
      ':' => {
        rects.push(lit(x + 1, 2, 1, 1));
        rects.push(lit(x + 1, 6, 1, 1));
        x += 3;
      }
      '.' => {
        rects.push(lit(x, 8, 1, 1));
        x += 2;
      }
      '+' => {
        rects.push(lit(x, 4, 3, 1));
        rects.push(lit(x + 1, 3, 1, 3));
        x += 4;
      }
      _ => {
        let mask = segment_mask(ch).unwrap_or(0);
        for (i, (sx, sy, w, h)) in SEGMENTS.iter().enumerate() {
          rects.push(SegRect {
            x: x + sx,
            y: *sy,
            w: *w,
            h: *h,
            lit: mask & (1 << i) != 0,
          });
        }
        x += DIGIT_W + 1;
      }
    }
  }
  SegLayout {
    width: x.saturating_sub(1),
    rects,
  }
}

/// 旋钮指示点扫过的角度：从 7 点半（225°）顺时针转到 4 点半（-45°）。
pub const KNOB_SWEEP_DEG: f64 = 270.0;
const KNOB_START_DEG: f64 = 225.0;

/// 旋钮上进度 `frac`（0–1）处的点相对圆心的偏移（屏幕坐标，y 向下），吸附到 2px 像素格。
///
/// 旋钮本体不旋转（转动点阵会出现非整数格的糊边），只把指示点与刻度灯摆到这些落点上。
#[must_use]
pub fn knob_point(frac: f64, radius: f64) -> (i32, i32) {
  let frac = if frac.is_finite() {
    frac.clamp(0.0, 1.0)
  } else {
    0.0
  };
  let rad = (KNOB_START_DEG - frac * KNOB_SWEEP_DEG).to_radians();
  let snap = |v: f64| (v / 2.0).round() as i32 * 2;
  (snap(radius * rad.cos()), snap(-radius * rad.sin()))
}

/// 旋钮走 `steps` 格（可为负）之后的值：落在 `min + k·step` 的网格上，并夹在 `[min, max]`。
///
/// 结果按 1e-9 收口：`0.1 + 0.2` 这类浮点尾数不会显示成 `0.30000000000000004`。
#[must_use]
pub fn knob_step(value: f64, steps: i32, min: f64, max: f64, step: f64) -> f64 {
  if !(value.is_finite() && min.is_finite() && max.is_finite()) || max < min {
    return min;
  }
  if !(step.is_finite() && step > 0.0) {
    return value.clamp(min, max);
  }
  let k = ((value - min) / step).round() + f64::from(steps);
  let v = (min + k * step).clamp(min, max);
  (v * 1e9).round() / 1e9
}

/// 干扰信号（QRM）的形象：每个一级分类对应一种真实的干扰现象，画成它在频谱上的样子。
///
/// 为什么不再画怪物精灵见 `docs/ui/decisions.md` §11.6。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QrmKind {
  /// 法规：非法电台 —— 不规则的突发块。
  Bursts,
  /// 频率：越界载波 —— 一根窄峰。
  Carrier,
  /// 操作：抢频插入 —— 两个信号叠在同一频点。
  Collision,
  /// 用语：乱码 —— 随机散点。
  Garble,
  /// 调制：过调制溅射 —— 主瓣加宽旁瓣。
  Splatter,
  /// 设备：寄生振荡 —— 等间距杂散梳齿。
  Comb,
  /// 天线：高驻波反射 —— 主峰旁一个回波峰。
  Echo,
  /// 传播：QSB 衰落 —— 幅度周期起伏。
  Fade,
  /// 基础：打火噪声 —— 宽带随机尖刺。
  Spark,
  /// 安全：雷电静电 —— 全频带瞬时突发。
  Static,
}

impl QrmKind {
  /// 全部种类（与一级分类一一对应，顺序同 `categories::TOP_CATEGORIES`）。
  pub const ALL: [Self; 10] = [
    Self::Bursts,
    Self::Carrier,
    Self::Collision,
    Self::Garble,
    Self::Splatter,
    Self::Comb,
    Self::Echo,
    Self::Fade,
    Self::Spark,
    Self::Static,
  ];

  /// 稳定的英文 key（界面文案 `rpg.qrm-<key>` 与 `data-*` 属性用）。
  #[must_use]
  pub const fn key(self) -> &'static str {
    match self {
      Self::Bursts => "bursts",
      Self::Carrier => "carrier",
      Self::Collision => "collision",
      Self::Garble => "garble",
      Self::Splatter => "splatter",
      Self::Comb => "comb",
      Self::Echo => "echo",
      Self::Fade => "fade",
      Self::Spark => "spark",
      Self::Static => "static",
    }
  }
}

/// 一级分类 key → 干扰形象（顺序同 `categories::TOP_CATEGORIES`）。
///
/// 显式写 key 而不是按下标取 [`QrmKind::ALL`]：下标映射在分类重排后会把每个分类的干扰图**静默**
/// 换成另一张（形状都还像频谱，看不出错），而这里的对应关系是语义的，单测也照这张表钉。
const QRM_BY_KEY: [(&str, QrmKind); 10] = [
  ("法规", QrmKind::Bursts),
  ("频率", QrmKind::Carrier),
  ("操作", QrmKind::Collision),
  ("用语", QrmKind::Garble),
  ("调制", QrmKind::Splatter),
  ("设备", QrmKind::Comb),
  ("天线", QrmKind::Echo),
  ("传播", QrmKind::Fade),
  ("基础", QrmKind::Spark),
  ("安全", QrmKind::Static),
];

/// 一级分类 key → 干扰形象；不认识的分类（「其它」）按乱码处理。
#[must_use]
pub fn qrm_kind(top_key: &str) -> QrmKind {
  QRM_BY_KEY
    .iter()
    .find(|(key, _)| *key == top_key)
    .map_or(QrmKind::Garble, |(_, kind)| *kind)
}

/// 干扰频谱一格与格间距的边长（像素）：画布尺寸与列坐标都由这两个数派生，视图层不自己算。
pub const QRM_CELL: u16 = 4;
/// 见 [`QRM_CELL`]。
pub const QRM_GAP: u16 = 2;

/// 一次能画出来的最大列数：坐标与尺寸都是 `u16`，再多就放不下（超出的列视图直接不画）。
#[must_use]
pub const fn qrm_max_cols() -> usize {
  (u16::MAX / (QRM_CELL + QRM_GAP)) as usize
}

/// 干扰频谱画布的像素尺寸 `(宽, 高)`：`cols` 列 × `rows` 行点阵，末列 / 末行不留格间距。
///
/// 退化输入（0 列 / 0 行）与超大列数都按饱和算：视图据此画一块空屏或截断，而不是整型下溢 panic。
#[must_use]
pub fn qrm_canvas(cols: usize, rows: u8) -> (u16, u16) {
  let step = QRM_CELL + QRM_GAP;
  let cols = cols.min(qrm_max_cols());
  let w = u16::try_from(cols)
    .unwrap_or(u16::MAX)
    .saturating_mul(step)
    .saturating_sub(QRM_GAP);
  (w, qrm_height(rows))
}

/// 画布高度（像素）：`rows` 行点阵，末行不留格间距。0 行返回 0。
#[must_use]
fn qrm_height(rows: u8) -> u16 {
  u16::from(rows)
    .saturating_mul(QRM_CELL + QRM_GAP)
    .saturating_sub(QRM_GAP)
}

/// 一列在画布里的左边界（像素）。`col` 超出 [`qrm_max_cols`] 时取上限。
#[must_use]
pub fn qrm_cell_x(col: usize) -> u16 {
  u16::try_from(col)
    .unwrap_or(u16::MAX)
    .saturating_mul(QRM_CELL + QRM_GAP)
}

/// 一行在画布里的上边界（像素，**行从底部数**）：第 0 行是底噪，末行落在 `y = 0`。
///
/// 先加后减：写成 `height - (row+1)·step + gap` 时最末一行会先下溢（debug 构建 panic、
/// release 靠回绕侥幸得到正确的 0），这里从 `height + gap` 起减。
#[must_use]
pub fn qrm_cell_y(row: u8, rows: u8) -> u16 {
  let step = QRM_CELL + QRM_GAP;
  (qrm_height(rows) + QRM_GAP).saturating_sub(u16::from(row).saturating_add(1).saturating_mul(step))
}

/// 干扰在频谱上的形状：`cols` 列，每列点亮 `1..=rows` 格（底噪至少 1 格）。
///
/// `frame` 只取 0 / 1：两帧交替就是「信号在抖」，第 1 帧在第 0 帧上做小幅扰动，形状不变。
/// 结果只由参数决定（没有随机数），同一个干扰在任何页面、任何时刻都长一样。
#[must_use]
pub fn qrm_levels(kind: QrmKind, cols: usize, rows: u8, frame: u8) -> Vec<u8> {
  let rows = rows.max(1);
  let top = f64::from(rows);
  let span = cols.saturating_sub(1).max(1) as f64;
  let jitter = |i: usize| (i * 7 + usize::from(frame) * 3).is_multiple_of(5);
  (0..cols)
    .map(|i| {
      let x = i as f64 / span;
      let d = (x - 0.5).abs();
      let level = match kind {
        QrmKind::Carrier => {
          if d < 0.06 {
            top
          } else {
            1.0
          }
        }
        QrmKind::Splatter => {
          if d < 0.1 {
            top * 0.9
          } else if d < 0.24 {
            top * 0.6
          } else if d < 0.4 {
            top * 0.35
          } else {
            1.0
          }
        }
        QrmKind::Collision => {
          let a = (x - 0.42).abs();
          let b = (x - 0.58).abs();
          if a < 0.07 || b < 0.07 {
            top * 0.85
          } else if a.min(b) < 0.15 {
            top * 0.45
          } else {
            1.0
          }
        }
        QrmKind::Comb => {
          if i % 4 == 1 {
            top * 0.7
          } else {
            1.0
          }
        }
        QrmKind::Echo => {
          if (x - 0.4).abs() < 0.06 {
            top
          } else if (x - 0.68).abs() < 0.05 {
            top * 0.5
          } else {
            1.0
          }
        }
        QrmKind::Fade => {
          let amp = if frame == 0 { 0.9 } else { 0.45 };
          if d < 0.16 { top * amp } else { 1.0 }
        }
        QrmKind::Bursts => {
          if (i / 3) % 3 == usize::from(frame) {
            top * 0.75
          } else {
            1.0
          }
        }
        QrmKind::Garble => 1.0 + ((i * 5 + usize::from(frame) * 11) % 7) as f64 * top / 9.0,
        QrmKind::Spark => {
          if (i * 13 + usize::from(frame) * 7).is_multiple_of(6) {
            top * 0.85
          } else {
            1.0
          }
        }
        QrmKind::Static => top * if frame == 0 { 0.7 } else { 0.55 },
      };
      let level = if jitter(i) && level > 1.0 {
        level - 1.0
      } else {
        level
      };
      level.round().clamp(1.0, top) as u8
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn exam_scores_read_s9_exactly_at_the_pass_line() {
    // A 类：40 题、合格线 30 题。
    assert_eq!(exam_s_units(30, 40, 30), Some(S9));
    // 差一题不能显示成 S9。
    assert_eq!(exam_s_units(29, 40, 30), Some(8));
    assert_eq!(exam_s_units(40, 40, 30), Some(S_SEGMENTS));
    assert_eq!(exam_s_units(0, 40, 30), Some(0));
    assert_eq!(exam_s_units(0, 0, 30), None);
    assert_eq!(s_reading(exam_s_units(35, 40, 30).unwrap()), "S9+20");
  }

  #[test]
  fn the_knob_sweeps_from_seven_thirty_to_four_thirty() {
    // 起点在左下（x < 0, y > 0），终点在右下（x > 0, y > 0），中点在正上方。
    let (x0, y0) = knob_point(0.0, 20.0);
    let (x1, y1) = knob_point(1.0, 20.0);
    let (xm, ym) = knob_point(0.5, 20.0);
    assert!(x0 < 0 && y0 > 0, "({x0}, {y0})");
    assert!(x1 > 0 && y1 > 0, "({x1}, {y1})");
    assert_eq!((xm, ym), (0, -20));
    // 左右对称，且都落在 2px 网格上。
    assert_eq!(x0, -x1);
    for f in [0.0, 0.13, 0.5, 0.77, 1.0] {
      let (x, y) = knob_point(f, 13.0);
      assert_eq!((x % 2, y % 2), (0, 0), "{f}");
    }
    // 越界与非有限值按边界算。
    assert_eq!(knob_point(f64::NAN, 20.0), knob_point(0.0, 20.0));
    assert_eq!(knob_point(3.0, 20.0), knob_point(1.0, 20.0));
  }

  #[test]
  fn knob_steps_stay_on_the_grid_and_inside_the_range() {
    assert_eq!(knob_step(700.0, 1, 300.0, 1200.0, 10.0), 710.0);
    assert_eq!(knob_step(700.0, -5, 300.0, 1200.0, 10.0), 650.0);
    assert_eq!(knob_step(1195.0, 3, 300.0, 1200.0, 10.0), 1200.0);
    assert_eq!(knob_step(-40.0, -1, -40.0, 40.0, 1.0), -40.0);
    // 不在网格上的起点先吸附再走：703 → 700 → 710。
    assert_eq!(knob_step(703.0, 1, 300.0, 1200.0, 10.0), 710.0);
    // 浮点尾数收口。
    assert_eq!(knob_step(0.2, 1, 0.0, 1.0, 0.1), 0.3);
    // 坏参数不炸。
    assert_eq!(knob_step(f64::NAN, 1, 0.0, 10.0, 1.0), 0.0);
    assert_eq!(knob_step(5.0, 1, 0.0, 10.0, 0.0), 5.0);
  }

  #[test]
  fn every_top_category_has_its_own_qrm_kind() {
    let kinds: Vec<QrmKind> = crate::categories::TOP_CATEGORIES
      .iter()
      .map(|c| qrm_kind(c.key))
      .collect();
    assert_eq!(kinds.len(), QrmKind::ALL.len());
    for (i, k) in kinds.iter().enumerate() {
      assert!(!kinds[..i].contains(k), "{k:?} 被两个分类共用");
    }
    assert_eq!(qrm_kind("频率"), QrmKind::Carrier);
    assert_eq!(qrm_kind("调制"), QrmKind::Splatter);
    assert_eq!(qrm_kind("其它"), QrmKind::Garble);
  }

  #[test]
  fn the_qrm_table_covers_every_category_in_the_same_order() {
    // 语义映射写在 `QRM_BY_KEY` 里：分类改名 / 重排 / 增删时这条会红，而不是静默换掉某个分类的干扰图。
    assert_eq!(QRM_BY_KEY.len(), crate::categories::TOP_CATEGORIES.len());
    for (i, (key, kind)) in QRM_BY_KEY.iter().enumerate() {
      assert_eq!(*key, crate::categories::TOP_CATEGORIES[i].key, "第 {i} 条");
      assert_eq!(qrm_kind(key), *kind);
      assert_eq!(QrmKind::ALL[i], *kind, "ALL 的顺序要与表一致");
    }
  }

  #[test]
  fn the_canvas_fits_degenerate_and_huge_grids() {
    // 16 列 × 8 行：15 个格间距 + 一格。
    assert_eq!(qrm_canvas(16, 8), (16 * (QRM_CELL + QRM_GAP) - QRM_GAP, 46));
    // 0 列 / 0 行：一块空屏，不是整型下溢。
    assert_eq!(qrm_canvas(0, 0), (0, 0));
    assert_eq!(qrm_canvas(0, 8), (0, 46));
    // 超出的列被截断到 u16 放得下的最大值，尺寸不再增长。
    let capped = qrm_canvas(qrm_max_cols() + 1, 8);
    assert_eq!(capped, qrm_canvas(qrm_max_cols(), 8));
    assert_eq!(capped.1, 46);
    // 截断后的列坐标（`末列 × step`）仍在 u16 之内：视图直接乘不会溢出。
    assert!(qrm_max_cols() * usize::from(QRM_CELL + QRM_GAP) <= usize::from(u16::MAX));
  }

  #[test]
  fn cells_land_inside_the_canvas_without_underflow() {
    let (w, h) = qrm_canvas(16, 8);
    assert_eq!(qrm_cell_x(0), 0);
    assert_eq!(qrm_cell_x(15), w - QRM_CELL);
    // 行从底部数：第 0 行贴底，末行贴顶；格子都不越界。
    assert_eq!(qrm_cell_y(0, 8), h - QRM_CELL);
    assert_eq!(qrm_cell_y(7, 8), 0);
    // 退化行数不下溢（修复前最末一行会先做 `46 - 48`，debug 构建 panic）。
    assert_eq!(qrm_cell_y(0, 0), 0);
    assert_eq!(qrm_cell_y(0, 1), 0);
    assert_eq!(qrm_cell_y(255, 255), 0);
  }

  #[test]
  fn qrm_levels_are_bounded_and_deterministic() {
    for kind in QrmKind::ALL {
      for frame in [0, 1] {
        let l = qrm_levels(kind, 18, 8, frame);
        assert_eq!(l.len(), 18, "{kind:?}");
        assert!(l.iter().all(|v| (1..=8).contains(v)), "{kind:?} {l:?}");
        assert_eq!(l, qrm_levels(kind, 18, 8, frame), "{kind:?} 不确定");
      }
      // 每种干扰都要有高出底噪的部分，否则画出来就是一条平线。
      assert!(
        qrm_levels(kind, 18, 8, 0).iter().any(|v| *v > 2),
        "{kind:?}"
      );
    }
  }

  #[test]
  fn a_carrier_is_one_narrow_peak_in_the_middle() {
    let l = qrm_levels(QrmKind::Carrier, 17, 8, 0);
    let peaks: Vec<usize> = (0..l.len()).filter(|i| l[*i] > 4).collect();
    assert!(!peaks.is_empty() && peaks.len() <= 3, "{l:?}");
    assert!(peaks.iter().all(|i| (6..=10).contains(i)), "{l:?}");
  }

  #[test]
  fn degenerate_sizes_do_not_panic() {
    assert!(qrm_levels(QrmKind::Splatter, 0, 8, 0).is_empty());
    assert_eq!(qrm_levels(QrmKind::Splatter, 1, 0, 0), vec![1]);
  }

  #[test]
  fn the_pass_line_reads_exactly_s9() {
    assert_eq!(s_units(0.75, 0.75), S9);
    // 差一点点及格不能显示成 S9。
    assert_eq!(s_units(0.749, 0.75), 8);
    assert_eq!(s_units(0.0, 0.75), 0);
  }

  #[test]
  fn the_margin_above_the_pass_line_folds_into_plus_db() {
    // A 类 36 / 40，合格线 30 / 40：余量 6 / 10，折成两格（+20 dB）。
    assert_eq!(s_units(36.0 / 40.0, 30.0 / 40.0), 11);
    assert_eq!(s_reading(11), "S9+20");
    assert_eq!(s_units(1.0, 0.75), S_SEGMENTS);
    assert_eq!(s_reading(S_SEGMENTS), "S9+40");
  }

  #[test]
  fn s_units_is_monotonic_and_bounded() {
    let mut prev = 0;
    for i in 0..=1000 {
      let u = s_units(f64::from(i) / 1000.0, 0.78);
      assert!(u >= prev, "{i}");
      assert!(u <= S_SEGMENTS);
      prev = u;
    }
  }

  #[test]
  fn junk_inputs_read_as_no_signal() {
    assert_eq!(s_units(f64::NAN, 0.75), 0);
    assert_eq!(s_units(0.9, 0.0), 0);
    assert_eq!(s_units(0.9, f64::INFINITY), 0);
    // 越界的正确率按边界算，而不是溢出。
    assert_eq!(s_units(1.7, 0.75), S_SEGMENTS);
    assert_eq!(s_units(-0.2, 0.75), 0);
    // 合格线 100% 时只有满分到 S9，没有 +dB 段可分。
    assert_eq!(s_units(1.0, 1.0), S9);
  }

  #[test]
  fn readings_below_s9_have_no_plus() {
    assert_eq!(s_reading(0), "S0");
    assert_eq!(s_reading(7), "S7");
    assert_eq!(s_reading(9), "S9");
    assert_eq!(s_reading(200), "S9+40");
  }

  #[test]
  fn eight_lights_every_segment_and_space_lights_none() {
    assert_eq!(segment_mask('8'), Some(0b111_1111));
    assert_eq!(segment_mask(' '), Some(0));
    assert_eq!(segment_mask('?'), None);
    // 每个数字都画得出来。
    for d in '0'..='9' {
      assert!(segment_mask(d).is_some_and(|m| m != 0), "{d}");
    }
  }

  #[test]
  fn a_clock_reading_lays_out_on_the_cell_grid() {
    let l = layout("38:24");
    // 4 个数字格（各 6 格含字间）+ 冒号 3 格，去掉末尾的字间。
    assert_eq!(l.width, 4 * 6 + 3 - 1);
    // 每个数字 7 段，冒号 2 个点。
    assert_eq!(l.rects.len(), 4 * 7 + 2);
    assert!(l.rects.iter().all(|r| r.x + u16::from(r.w) <= l.width));
    assert!(l.rects.iter().all(|r| r.y + r.h <= DIGIT_H));
  }

  #[test]
  fn unknown_characters_keep_their_slot_as_an_unlit_digit() {
    let known = layout("1?3");
    let blank = layout("1 3");
    assert_eq!(known, blank);
    assert_eq!(known.width, 3 * 6 - 1);
  }

  #[test]
  fn separators_only_draw_lit_dots() {
    let l = layout(".:+");
    assert!(l.rects.iter().all(|r| r.lit));
    assert_eq!(l.width, 2 + 3 + 4 - 1);
  }

  #[test]
  fn overlong_readings_are_truncated_instead_of_overflowing() {
    let l = layout(&"8".repeat(5000));
    assert_eq!(l.width, 1000 * 6 - 1);
  }

  #[test]
  fn pass_ratio_is_the_pass_line_over_the_total() {
    let a = ExamRule::of(Bank::A);
    assert!((pass_ratio(&a) - a.pass as f64 / a.total as f64).abs() < 1e-12);
    let none = ExamRule { total: 0, ..a };
    assert_eq!(pass_ratio(&none), 0.0);
  }

  #[test]
  fn no_study_record_falls_back_to_the_default_bank() {
    let (bank, ratio) = main_bank_pass(None);
    assert_eq!(bank, Bank::default());
    assert_eq!(ratio, pass_ratio(&ExamRule::of(Bank::default())));
  }

  #[test]
  fn an_unknown_category_reads_as_garble() {
    assert_eq!(qrm_kind("不存在的分类"), QrmKind::Garble);
  }

  #[test]
  fn an_empty_reading_is_empty() {
    assert_eq!(
      layout(""),
      SegLayout {
        width: 0,
        rects: Vec::new()
      }
    );
  }
}
