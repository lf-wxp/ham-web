//! QSL 标签打印：常见 Avery 不干胶标签版式，以及把日志通联按呼号合并成标签。

use crate::logbook::LogEntry;

/// 纸张。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Paper {
  A4,
  Letter,
}

impl Paper {
  /// 纸张宽高（毫米）。
  #[must_use]
  pub const fn size_mm(self) -> (f32, f32) {
    match self {
      Self::A4 => (210.0, 297.0),
      Self::Letter => (215.9, 279.4),
    }
  }

  /// CSS `@page size` 取值。
  #[must_use]
  pub const fn css(self) -> &'static str {
    match self {
      Self::A4 => "A4",
      Self::Letter => "letter",
    }
  }
}

/// 一种标签纸版式，尺寸单位均为毫米。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
  pub id: &'static str,
  pub name: &'static str,
  pub paper: Paper,
  pub cols: usize,
  pub rows: usize,
  pub width: f32,
  pub height: f32,
  /// 第一张标签距纸张上边、左边的距离。
  pub top: f32,
  pub left: f32,
  /// 相邻标签的水平、垂直间隙。
  pub col_gap: f32,
  pub row_gap: f32,
  /// 每张标签能容纳的通联行数。
  pub qsos_per_label: usize,
}

impl Layout {
  #[must_use]
  pub const fn per_page(&self) -> usize {
    self.cols * self.rows
  }

  /// 第 `slot` 个位置（按行优先）的左上角坐标。
  #[must_use]
  #[allow(clippy::cast_precision_loss)]
  pub fn origin(&self, slot: usize) -> (f32, f32) {
    let (r, c) = (slot / self.cols, slot % self.cols);
    (
      self.left + c as f32 * (self.width + self.col_gap),
      self.top + r as f32 * (self.height + self.row_gap),
    )
  }
}

/// 支持的版式，第一个为默认。
pub const LAYOUTS: &[Layout] = &[
  Layout {
    id: "L7163",
    name: "Avery L7163 · A4 · 2×7（99.1×38.1 mm）",
    paper: Paper::A4,
    cols: 2,
    rows: 7,
    width: 99.1,
    height: 38.1,
    top: 15.15,
    left: 4.65,
    col_gap: 2.5,
    row_gap: 0.0,
    qsos_per_label: 4,
  },
  Layout {
    id: "L7160",
    name: "Avery L7160 · A4 · 3×7（63.5×38.1 mm）",
    paper: Paper::A4,
    cols: 3,
    rows: 7,
    width: 63.5,
    height: 38.1,
    top: 15.15,
    left: 7.25,
    col_gap: 2.5,
    row_gap: 0.0,
    qsos_per_label: 4,
  },
  Layout {
    id: "L7159",
    name: "Avery L7159 · A4 · 3×8（63.5×33.9 mm）",
    paper: Paper::A4,
    cols: 3,
    rows: 8,
    width: 63.5,
    height: 33.9,
    top: 12.9,
    left: 7.25,
    col_gap: 2.5,
    row_gap: 0.0,
    qsos_per_label: 3,
  },
  Layout {
    id: "5160",
    name: "Avery 5160 · Letter · 3×10（66.7×25.4 mm）",
    paper: Paper::Letter,
    cols: 3,
    rows: 10,
    width: 66.675,
    height: 25.4,
    top: 12.7,
    left: 4.7625,
    col_gap: 3.175,
    row_gap: 0.0,
    qsos_per_label: 2,
  },
  Layout {
    id: "5163",
    name: "Avery 5163 · Letter · 2×5（101.6×50.8 mm）",
    paper: Paper::Letter,
    cols: 2,
    rows: 5,
    width: 101.6,
    height: 50.8,
    top: 12.7,
    left: 3.96875,
    col_gap: 4.7625,
    row_gap: 0.0,
    qsos_per_label: 6,
  },
];

/// 按 id 查找版式，找不到时用默认版式。
#[must_use]
pub fn layout(id: &str) -> &'static Layout {
  LAYOUTS.iter().find(|l| l.id == id).unwrap_or(&LAYOUTS[0])
}

/// 一张标签：同一呼号的若干通联。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
  pub callsign: String,
  /// 通联记录的 id，按时间先后。
  pub ids: Vec<u64>,
}

/// 把通联按呼号合并为标签：同呼号按时间排序，超出每张容量时拆成多张；
/// 标签按每个呼号最早一次通联的时间排序。
#[must_use]
pub fn build(entries: &[&LogEntry], per_label: usize) -> Vec<Label> {
  let per_label = per_label.max(1);
  let mut sorted: Vec<&LogEntry> = entries.to_vec();
  sorted.sort_by(|a, b| (&a.date, &a.time, a.id).cmp(&(&b.date, &b.time, b.id)));
  let mut groups: Vec<(String, Vec<u64>)> = Vec::new();
  for e in sorted {
    let call = e.callsign.trim().to_ascii_uppercase();
    if call.is_empty() {
      continue;
    }
    match groups.iter_mut().find(|(c, _)| *c == call) {
      Some((_, ids)) => ids.push(e.id),
      None => groups.push((call, vec![e.id])),
    }
  }
  groups
    .into_iter()
    .flat_map(|(callsign, ids)| {
      ids
        .chunks(per_label)
        .map(|c| Label {
          callsign: callsign.clone(),
          ids: c.to_vec(),
        })
        .collect::<Vec<_>>()
    })
    .collect()
}

/// 排版到各页：开头空出 `skip` 个位置（接着用已用过一部分的标签纸），返回每页各位置的标签下标。
#[must_use]
pub fn paginate(count: usize, layout: &Layout, skip: usize) -> Vec<Vec<Option<usize>>> {
  let per_page = layout.per_page();
  let skip = skip.min(per_page - 1);
  let total = skip + count;
  if count == 0 {
    return Vec::new();
  }
  (0..total.div_ceil(per_page))
    .map(|p| {
      (0..per_page)
        .map(|s| (p * per_page + s).checked_sub(skip).filter(|&i| i < count))
        .collect()
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn qso(id: u64, call: &str, date: &str, time: &str) -> LogEntry {
    LogEntry {
      id,
      callsign: call.into(),
      date: date.into(),
      time: time.into(),
      ..LogEntry::default()
    }
  }

  #[test]
  fn layouts_fit_on_paper() {
    for l in LAYOUTS {
      let (w, h) = l.paper.size_mm();
      let (x, y) = l.origin(l.per_page() - 1);
      assert!(x + l.width <= w + 0.5, "{} 超出纸宽：{}", l.id, x + l.width);
      assert!(
        y + l.height <= h + 0.5,
        "{} 超出纸高：{}",
        l.id,
        y + l.height
      );
    }
    assert_eq!(layout("5160").per_page(), 30);
    assert_eq!(layout("nope").id, "L7163");
  }

  #[test]
  fn groups_by_call_and_splits() {
    let list = [
      qso(1, "w1aw", "2024-05-02", "01:00"),
      qso(2, "JA1AA", "2024-05-01", "12:30"),
      qso(3, "W1AW", "2024-05-01", "13:00"),
      qso(4, "W1AW", "2024-05-03", "02:00"),
      qso(5, "", "2024-05-03", "02:00"),
    ];
    let refs: Vec<&LogEntry> = list.iter().collect();
    let labels = build(&refs, 2);
    let got: Vec<(&str, &[u64])> = labels
      .iter()
      .map(|l| (l.callsign.as_str(), l.ids.as_slice()))
      .collect();
    assert_eq!(
      got,
      vec![
        ("JA1AA", &[2][..]),
        ("W1AW", &[3, 1][..]),
        ("W1AW", &[4][..])
      ]
    );
  }

  #[test]
  fn paginate_with_skip() {
    let l = layout("5163"); // 每页 10 张
    let pages = paginate(12, l, 3);
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0][..4], [None, None, None, Some(0)]);
    assert_eq!(pages[1][4], Some(11));
    assert_eq!(pages[1][5], None);
    assert!(paginate(0, l, 3).is_empty());
  }
}
