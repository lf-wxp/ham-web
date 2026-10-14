//! QSL 卡片的打印拼版：一张 A4 上放几张卡、各放在哪、裁切线画在哪。
//!
//! 纯几何（单位 mm），所以放核心单测；界面层只把这些坐标原样写进打印页的
//! `style`。卡片尺寸取 [`crate::qsl_card::CARD_MM`]（140×89mm 标准卡）。
//!
//! 两种方式：
//! - [`Imposition::Single`]：A4 竖向正中一张（原有行为）
//! - [`Imposition::Grid`]：A4 横向 2×2 共四张。卡与卡**共刀**（间距 0）—— 留缝就要
//!   多裁一刀，且 297×210 的纸面在 2×2 之外剩不下多少给缝；裁切线画在纸边的留白里，
//!   不压到卡面上
//!
//! 纸面高度 / 宽度都按 A4 少 1mm 计：浏览器把「刚好 297mm」的内容排到第二页的情况
//! 并不少见（与原有单卡打印页同一处理）。

use crate::qsl_card::CARD_MM;

/// A4 纸（mm，竖向）。
pub const A4_MM: (f64, f64) = (210.0, 297.0);
/// 纸面预留：防止内容恰好等于纸张尺寸时被挤到第二页。
const PAGE_SLACK_MM: f64 = 1.0;
/// 裁切线（纸边的短刻线）长度。
pub const CUT_MARK_MM: f64 = 5.0;
/// 离卡面多远开始画裁切线：贴着卡边画，裁偏一点就会把线留在卡上。
pub const CUT_MARK_GAP_MM: f64 = 2.0;

/// 打印拼版方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Imposition {
  /// A4 竖向正中一张。
  Single,
  /// A4 横向 2×2 四张（共刀）。
  Grid,
}

impl Imposition {
  /// 全部方式（界面按这个顺序列出）。
  pub const ALL: [Self; 2] = [Self::Single, Self::Grid];

  /// 稳定标识（持久化与单选项的值）。
  #[must_use]
  pub const fn id(self) -> &'static str {
    match self {
      Self::Single => "single",
      Self::Grid => "grid",
    }
  }

  /// 由标识还原；认不出按单张处理。
  #[must_use]
  pub fn from_id(id: &str) -> Self {
    Self::ALL
      .into_iter()
      .find(|i| i.id() == id)
      .unwrap_or(Self::Single)
  }
}

/// 一页的拼版结果（mm，原点在纸面左上角）。
#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
  /// 纸面是否横向（`@page { size: A4 landscape }`）。
  pub landscape: bool,
  /// 打印区域尺寸（已扣掉 [`PAGE_SLACK_MM`]）。
  pub page_mm: (f64, f64),
  /// 每张卡的左上角。
  pub slots: Vec<(f64, f64)>,
  /// 竖向裁切线的 x 坐标（去重、升序）。
  pub cut_x: Vec<f64>,
  /// 横向裁切线的 y 坐标（去重、升序）。
  pub cut_y: Vec<f64>,
}

impl Sheet {
  /// 卡片区域的外框 `(x0, y0, x1, y1)`。
  #[must_use]
  pub fn bounds(&self) -> (f64, f64, f64, f64) {
    let (w, h) = CARD_MM;
    let x0 = self.slots.iter().map(|s| s.0).fold(f64::INFINITY, f64::min);
    let y0 = self.slots.iter().map(|s| s.1).fold(f64::INFINITY, f64::min);
    let x1 = self
      .slots
      .iter()
      .map(|s| s.0 + w)
      .fold(f64::NEG_INFINITY, f64::max);
    let y1 = self
      .slots
      .iter()
      .map(|s| s.1 + h)
      .fold(f64::NEG_INFINITY, f64::max);
    (x0, y0, x1, y1)
  }
}

/// 把 `cols × rows` 张卡（间距 0）居中排在纸面上。
fn grid(landscape: bool, cols: usize, rows: usize) -> Sheet {
  let (pw, ph) = if landscape {
    (A4_MM.1 - PAGE_SLACK_MM, A4_MM.0 - PAGE_SLACK_MM)
  } else {
    (A4_MM.0 - PAGE_SLACK_MM, A4_MM.1 - PAGE_SLACK_MM)
  };
  let (w, h) = CARD_MM;
  let x0 = (pw - w * cols as f64) / 2.0;
  let y0 = (ph - h * rows as f64) / 2.0;
  let mut slots = Vec::with_capacity(cols * rows);
  for r in 0..rows {
    for c in 0..cols {
      slots.push((x0 + w * c as f64, y0 + h * r as f64));
    }
  }
  let cut_x = (0..=cols).map(|c| x0 + w * c as f64).collect();
  let cut_y = (0..=rows).map(|r| y0 + h * r as f64).collect();
  Sheet {
    landscape,
    page_mm: (pw, ph),
    slots,
    cut_x,
    cut_y,
  }
}

/// 按拼版方式算出一页。
#[must_use]
pub fn sheet(imposition: Imposition) -> Sheet {
  match imposition {
    Imposition::Single => grid(false, 1, 1),
    Imposition::Grid => grid(true, 2, 2),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ids_round_trip_and_unknown_falls_back_to_single() {
    for i in Imposition::ALL {
      assert_eq!(Imposition::from_id(i.id()), i);
    }
    assert_eq!(Imposition::from_id("nope"), Imposition::Single);
  }

  #[test]
  fn every_slot_fits_on_the_page_and_slots_never_overlap() {
    let (w, h) = CARD_MM;
    for i in Imposition::ALL {
      let s = sheet(i);
      for &(x, y) in &s.slots {
        assert!(x >= 0.0 && y >= 0.0, "{i:?}");
        assert!(
          x + w <= s.page_mm.0 + 1e-9 && y + h <= s.page_mm.1 + 1e-9,
          "{i:?}"
        );
      }
      for (a, p) in s.slots.iter().enumerate() {
        for q in &s.slots[a + 1..] {
          let apart = p.0 + w <= q.0 + 1e-9
            || q.0 + w <= p.0 + 1e-9
            || p.1 + h <= q.1 + 1e-9
            || q.1 + h <= p.1 + 1e-9;
          assert!(apart, "{i:?}: {p:?} / {q:?}");
        }
      }
    }
  }

  #[test]
  fn single_is_one_centred_card_and_grid_is_four_on_landscape() {
    let one = sheet(Imposition::Single);
    assert!(!one.landscape);
    assert_eq!(one.slots.len(), 1);
    let (x0, y0, x1, y1) = one.bounds();
    assert!((x0 - (one.page_mm.0 - x1)).abs() < 1e-9);
    assert!((y0 - (one.page_mm.1 - y1)).abs() < 1e-9);

    let four = sheet(Imposition::Grid);
    assert!(four.landscape);
    assert_eq!(four.slots.len(), 4);
    // 共刀：3 条竖线 + 3 条横线就能裁出四张（而不是 4 + 4）。
    assert_eq!(four.cut_x.len(), 3);
    assert_eq!(four.cut_y.len(), 3);
    let (x0, y0, x1, y1) = four.bounds();
    assert!((x0 - (four.page_mm.0 - x1)).abs() < 1e-9);
    assert!((y0 - (four.page_mm.1 - y1)).abs() < 1e-9);
  }

  #[test]
  fn cut_marks_fit_in_the_margins() {
    // 刻线画在卡区外的留白里：留白要容得下「间隙 + 刻线」，否则线会被纸边吃掉
    // 或压到卡面上。这条也顺带钉住「A4 横向放得下 2×2」。
    for i in Imposition::ALL {
      let s = sheet(i);
      let (x0, y0, x1, y1) = s.bounds();
      let need = CUT_MARK_GAP_MM + CUT_MARK_MM;
      let margins = [x0, y0, s.page_mm.0 - x1, s.page_mm.1 - y1];
      assert!(margins.iter().all(|m| *m >= need), "{i:?}: {margins:?}");
    }
  }
}
