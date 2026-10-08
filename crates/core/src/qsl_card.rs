//! QSL 卡片设计：必备信息、设计建议，以及**卡片版式的几何计算**。
//!
//! 版式（文字放哪、多大、什么语气）是纯几何，放在核心 crate 里可以脱离浏览器单测；
//! 颜色与字体留给界面层 —— 卡片是打印物，配色要跟着纸走，不该混进领域模型。
//!
//! 所有坐标都是**相对卡片尺寸的比例**（`x/w`、`y/h`、字号取 `h` 的一个比例），
//! 所以同一份版式既能画在屏幕上的预览画布，也能按 140×89mm 打印而不走形。

/// 必备信息。
pub const QSL_REQUIRED: &[(&str, &str)] = &[
  ("双方呼号", "本台呼号与对方呼号（正反面均可）。"),
  ("通联确认", "日期、时间、频率、模式、信号报告。"),
  ("操作员信息", "操作员姓名、QTH、设备。"),
  ("签名", "操作员签名确认。"),
];

/// 设计建议。
pub const QSL_DESIGN_TIPS: &[&str] = &[
  "突出呼号，正反面信息清晰、易读。",
  "正面可用本地风光、设备照片或个性化设计。",
  "留出填写区域与贴邮票/回邮位置。",
  "批量打印或在线定制（如 Gennady/UX5UO 等专业打印服务）。",
];

/// 标准 QSL 卡片物理尺寸（mm）：3.5 × 5.5 英寸。
pub const CARD_MM: (f64, f64) = (140.0, 89.0);

/// 卡片上固定的栏位标题。
///
/// 沿用国际通行的英文惯例而不是走界面词典：QSL 卡片是**邮寄给对方**的实物，
/// 对方未必懂中文；这几行是全世界 QSL 卡的通用记号。
pub const INFO_LABELS: [&str; 5] = ["DATE", "TIME", "FREQ", "MODE", "RST"];

/// 版式模板。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CardTemplate {
  /// 经典：细边框 + 完整栏位 + 73 落款。
  Classic,
  /// 极简：无边框，一条细线分隔，留白多。
  Minimal,
  /// 醒目：顶部色条 + 信息行底衬，呼号最大。
  Bold,
}

impl CardTemplate {
  /// 全部模板（界面按此顺序列出）。
  pub const ALL: [Self; 3] = [Self::Classic, Self::Minimal, Self::Bold];

  /// 稳定短键（存偏好 / 下拉值用）。
  #[must_use]
  pub const fn id(self) -> &'static str {
    match self {
      Self::Classic => "classic",
      Self::Minimal => "minimal",
      Self::Bold => "bold",
    }
  }

  /// 由短键还原；认不出时回到经典版式。
  #[must_use]
  pub fn from_id(id: &str) -> Self {
    Self::ALL
      .into_iter()
      .find(|t| t.id() == id)
      .unwrap_or(Self::Classic)
  }
}

/// 文字语气：核心只说「这是什么层级」，颜色由界面层给。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
  /// 正文（深色）。
  Strong,
  /// 次要信息（灰）。
  Muted,
  /// 强调（模板主色）。
  Accent,
  /// 反白（画在强调色块上）。
  Inverse,
}

/// 文字对齐方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
  /// 左对齐（`x` 是左边缘）。
  Left,
  /// 居中（`x` 是中线）。
  Center,
  /// 右对齐（`x` 是右边缘）。
  Right,
}

/// 卡片上要画的一段文字（左上角为原点，`y` 是**基线**，与 canvas 的 `fillText` 一致）。
#[derive(Debug, Clone, PartialEq)]
pub struct CardText {
  /// 文案。
  pub text: String,
  /// 横向位置。
  pub x: f64,
  /// 基线纵坐标。
  pub y: f64,
  /// 字号。
  pub size: f64,
  /// 加粗。
  pub bold: bool,
  /// 斜体。
  pub italic: bool,
  /// 对齐。
  pub align: Align,
  /// 语气。
  pub tone: Tone,
}

/// 装饰块的用途（描边还是填充、用什么颜色都由界面层决定）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RectKind {
  /// 外边框（描边）。
  Frame,
  /// 顶部色条（填充）。
  TopBand,
  /// 信息行底衬（填充）。
  InfoBand,
  /// 细分隔线（描边，`h` 即线宽）。
  Rule,
}

/// 一块装饰矩形。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardRect {
  /// 左边。
  pub x: f64,
  /// 上边。
  pub y: f64,
  /// 宽。
  pub w: f64,
  /// 高（`Rule` 时就是线宽）。
  pub h: f64,
  /// 用途。
  pub kind: RectKind,
}

/// 一张卡片要印的内容。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CardContent {
  /// 本台呼号。
  pub my_call: String,
  /// 本台网格。
  pub my_grid: String,
  /// 操作员。
  pub operator: String,
  /// 本台 QTH。
  pub my_qth: String,
  /// 对方呼号。
  pub to_call: String,
  /// 对方网格。
  pub to_grid: String,
  /// 日期。
  pub date: String,
  /// 时间（UTC）。
  pub time: String,
  /// 频率（MHz）。
  pub freq: String,
  /// 模式。
  pub mode: String,
  /// 信号报告。
  pub rst: String,
}

impl CardContent {
  /// 「网格 · QTH」那一行：有哪个写哪个，都没有就返回空（界面层会跳过空文案）。
  #[must_use]
  pub fn location_line(&self) -> String {
    match (self.my_grid.trim(), self.my_qth.trim()) {
      ("", "") => String::new(),
      (g, "") => g.to_owned(),
      ("", q) => q.to_owned(),
      (g, q) => format!("{g} · {q}"),
    }
  }

  /// 栏位值，顺序与 [`INFO_LABELS`] 对应。
  #[must_use]
  pub fn info_values(&self) -> [&str; 5] {
    [
      self.date.as_str(),
      self.time.as_str(),
      self.freq.as_str(),
      self.mode.as_str(),
      self.rst.as_str(),
    ]
  }
}

/// 左右内边距（相对卡片宽度）。
fn pad(w: f64) -> f64 {
  w * 0.05
}

/// 版式的装饰块。
#[must_use]
pub fn decor(template: CardTemplate, size: (f64, f64)) -> Vec<CardRect> {
  let (w, h) = size;
  let p = pad(w);
  match template {
    CardTemplate::Classic => vec![
      CardRect {
        x: p,
        y: p,
        w: w - 2.0 * p,
        h: h - 2.0 * p,
        kind: RectKind::Frame,
      },
      CardRect {
        x: p * 2.0,
        y: h * 0.40,
        w: w - 4.0 * p,
        h: (h * 0.004).max(1.0),
        kind: RectKind::Rule,
      },
    ],
    CardTemplate::Minimal => vec![CardRect {
      x: p * 2.0,
      y: h * 0.42,
      w: w - 4.0 * p,
      h: (h * 0.004).max(1.0),
      kind: RectKind::Rule,
    }],
    CardTemplate::Bold => vec![
      CardRect {
        x: 0.0,
        y: 0.0,
        w,
        h: h * 0.20,
        kind: RectKind::TopBand,
      },
      CardRect {
        x: 0.0,
        y: h * 0.70,
        w,
        h: h * 0.18,
        kind: RectKind::InfoBand,
      },
    ],
  }
}

/// 把内容按模板排成「要画的文字块」。
///
/// 空文案也会返回（界面层跳过空串即可）—— 版式的位置是固定的，不该因为某个字段没填
/// 就让剩下的字段挪位；顺带让「版式不变」这件事可以被单测断言。
#[must_use]
pub fn layout(template: CardTemplate, size: (f64, f64), c: &CardContent) -> Vec<CardText> {
  let (w, h) = size;
  let p = pad(w);
  let mut out = Vec::new();

  match template {
    CardTemplate::Classic => {
      out.push(bold(text(
        c.my_call.clone(),
        p * 1.6,
        h * 0.175,
        h * 0.145,
        Tone::Strong,
        Align::Left,
      )));
      out.push(text(
        c.location_line(),
        p * 1.6,
        h * 0.255,
        h * 0.052,
        Tone::Muted,
        Align::Left,
      ));
      out.push(text(
        format!("OP: {}", c.operator),
        p * 1.6,
        h * 0.325,
        h * 0.052,
        Tone::Muted,
        Align::Left,
      ));
      out.push(text(
        "Confirming QSO with".to_owned(),
        w / 2.0,
        h * 0.455,
        h * 0.05,
        Tone::Muted,
        Align::Center,
      ));
      out.push(bold(text(
        c.to_call.clone(),
        w / 2.0,
        h * 0.60,
        h * 0.135,
        Tone::Strong,
        Align::Center,
      )));
      out.push(text(
        c.to_grid.clone(),
        w / 2.0,
        h * 0.66,
        h * 0.05,
        Tone::Accent,
        Align::Center,
      ));
      out.extend(info_rows(
        w,
        p,
        c,
        InfoRows {
          caption_y: h * 0.775,
          caption_size: h * 0.042,
          value_y: h * 0.845,
          value_size: h * 0.058,
        },
      ));
      out.push(italic(bold(text(
        "73 · TNX QSO".to_owned(),
        w / 2.0,
        h * 0.95,
        h * 0.055,
        Tone::Strong,
        Align::Center,
      ))));
    }
    CardTemplate::Minimal => {
      out.push(bold(text(
        c.my_call.clone(),
        w / 2.0,
        h * 0.20,
        h * 0.115,
        Tone::Strong,
        Align::Center,
      )));
      out.push(text(
        c.location_line(),
        w / 2.0,
        h * 0.285,
        h * 0.048,
        Tone::Muted,
        Align::Center,
      ));
      out.push(bold(text(
        c.to_call.clone(),
        w / 2.0,
        h * 0.575,
        h * 0.12,
        Tone::Strong,
        Align::Center,
      )));
      out.push(text(
        c.to_grid.clone(),
        w / 2.0,
        h * 0.645,
        h * 0.048,
        Tone::Muted,
        Align::Center,
      ));
      out.extend(info_rows(
        w,
        p,
        c,
        InfoRows {
          caption_y: h * 0.775,
          caption_size: h * 0.038,
          value_y: h * 0.85,
          value_size: h * 0.055,
        },
      ));
    }
    CardTemplate::Bold => {
      out.push(bold(text(
        c.my_call.clone(),
        p * 1.5,
        h * 0.145,
        h * 0.16,
        Tone::Inverse,
        Align::Left,
      )));
      out.push(text(
        format!("OP {}", c.operator),
        w - p * 1.5,
        h * 0.135,
        h * 0.045,
        Tone::Inverse,
        Align::Right,
      ));
      out.push(text(
        c.location_line(),
        p * 1.5,
        h * 0.33,
        h * 0.05,
        Tone::Muted,
        Align::Left,
      ));
      out.push(text(
        "Confirming QSO with".to_owned(),
        w / 2.0,
        h * 0.435,
        h * 0.048,
        Tone::Muted,
        Align::Center,
      ));
      out.push(bold(text(
        c.to_call.clone(),
        w / 2.0,
        h * 0.575,
        h * 0.15,
        Tone::Strong,
        Align::Center,
      )));
      out.push(text(
        c.to_grid.clone(),
        w / 2.0,
        h * 0.635,
        h * 0.05,
        Tone::Accent,
        Align::Center,
      ));
      out.extend(info_rows(
        w,
        p,
        c,
        InfoRows {
          caption_y: h * 0.755,
          caption_size: h * 0.036,
          value_y: h * 0.825,
          value_size: h * 0.058,
        },
      ));
      out.push(italic(text(
        "73".to_owned(),
        w / 2.0,
        h * 0.96,
        h * 0.05,
        Tone::Muted,
        Align::Center,
      )));
    }
  }
  out
}

/// 栏位两行的纵向布局参数。
struct InfoRows {
  /// 标题基线。
  caption_y: f64,
  /// 标题字号。
  caption_size: f64,
  /// 值基线。
  value_y: f64,
  /// 值字号。
  value_size: f64,
}

/// 栏位标题 + 值两行：横坐标在内边距之间等距铺开（首末贴边，`n - 1` 个间隔）。
fn info_rows(w: f64, p: f64, c: &CardContent, rows: InfoRows) -> Vec<CardText> {
  let left = p * 2.0;
  let right = w - p * 2.0;
  let step = (right - left) / (INFO_LABELS.len() - 1) as f64;
  let mut out = Vec::with_capacity(INFO_LABELS.len() * 2);
  for (i, label) in INFO_LABELS.iter().enumerate() {
    out.push(text(
      (*label).to_owned(),
      left + i as f64 * step,
      rows.caption_y,
      rows.caption_size,
      Tone::Muted,
      Align::Center,
    ));
  }
  for (i, value) in c.info_values().iter().enumerate() {
    out.push(bold(text(
      (*value).to_owned(),
      left + i as f64 * step,
      rows.value_y,
      rows.value_size,
      Tone::Strong,
      Align::Center,
    )));
  }
  out
}

/// 造一段文字（默认不加粗不斜体，便于用 [`bold`] / [`italic`] 叠加）。
fn text(t: String, x: f64, y: f64, size: f64, tone: Tone, align: Align) -> CardText {
  CardText {
    text: t,
    x,
    y,
    size,
    bold: false,
    italic: false,
    align,
    tone,
  }
}

fn bold(mut t: CardText) -> CardText {
  t.bold = true;
  t
}

fn italic(mut t: CardText) -> CardText {
  t.italic = true;
  t
}

#[cfg(test)]
mod tests {
  use super::*;

  fn sample() -> CardContent {
    CardContent {
      my_call: "BG4XXX".into(),
      my_grid: "OM89".into(),
      operator: "WXP".into(),
      my_qth: "上海".into(),
      to_call: "JA1AAA".into(),
      to_grid: "PM95".into(),
      date: "2026-10-01".into(),
      time: "12:34".into(),
      freq: "14.074".into(),
      mode: "FT8".into(),
      rst: "-12".into(),
    }
  }

  fn sizes() -> [(f64, f64); 2] {
    // 屏幕预览用的画布 + 打印用的 140×89mm 等比放大。
    [(1400.0, 890.0), (420.0, 267.0)]
  }

  #[test]
  fn qsl_card_data_populated() {
    assert!(QSL_REQUIRED.len() >= 3);
    assert!(!QSL_DESIGN_TIPS.is_empty());
    assert_eq!(INFO_LABELS.len(), 5);
  }

  #[test]
  fn template_ids_round_trip() {
    for t in CardTemplate::ALL {
      assert_eq!(CardTemplate::from_id(t.id()), t);
    }
    assert_eq!(CardTemplate::from_id(""), CardTemplate::Classic);
    assert_eq!(CardTemplate::from_id("nope"), CardTemplate::Classic);
  }

  #[test]
  fn everything_stays_inside_the_card() {
    // 版式是**不可裁剪**的：越界的字会被印不出来，所以逐个断言。
    for t in CardTemplate::ALL {
      for (w, h) in sizes() {
        for item in layout(t, (w, h), &sample()) {
          assert!(
            (0.0..=w).contains(&item.x) && (0.0..=h).contains(&item.y),
            "{} 版式有文字越界：{:?}",
            t.id(),
            item
          );
          assert!(
            item.size > 0.0 && item.size < h * 0.25,
            "{} 版式字号异常：{}",
            t.id(),
            item.size
          );
        }
        for r in decor(t, (w, h)) {
          assert!(
            r.x >= 0.0 && r.y >= 0.0 && r.x + r.w <= w + 1e-9 && r.y + r.h <= h + 1e-9,
            "{} 版式装饰越界：{r:?}",
            t.id()
          );
        }
      }
    }
  }

  #[test]
  fn info_columns_are_ordered_and_do_not_collide() {
    for t in CardTemplate::ALL {
      let (w, h) = (1400.0, 890.0);
      let items = layout(t, (w, h), &sample());
      // 栏位标题：正好 5 个，按文案认出来（顺序与 INFO_LABELS 一致）。
      let mut columns: Vec<(f64, f64)> = items
        .iter()
        .filter(|i| INFO_LABELS.contains(&i.text.as_str()))
        .map(|i| (i.x, i.size))
        .collect();
      assert_eq!(
        columns.len(),
        INFO_LABELS.len(),
        "{} 版式栏位数量不对",
        t.id()
      );
      columns.sort_by(|a, b| a.0.total_cmp(&b.0));
      for pair in columns.windows(2) {
        let gap = pair[1].0 - pair[0].0;
        // 相邻栏位的间距必须大于字号，否则数字会挤在一起（真实字宽由字体决定，
        // 这里只钉住「不会因为版式改动把 5 栏压成一团」）。
        assert!(
          gap > pair[1].1,
          "{} 版式栏位间距 {gap} 小于字号 {}",
          t.id(),
          pair[1].1
        );
      }
      // 栏位关于中线对称：中间那栏必须正好压在中线上，首末两栏到中线的距离相等。
      let xs: Vec<f64> = columns.iter().map(|c| c.0).collect();
      assert!(
        (xs[2] - w / 2.0).abs() < 1e-9,
        "{} 版式中间栏位不在中线上：{}",
        t.id(),
        xs[2]
      );
      for (i, x) in xs.iter().enumerate() {
        let mirror = w - xs[xs.len() - 1 - i];
        assert!(
          (x - mirror).abs() < 1e-9,
          "{} 版式栏位不对称：{x} vs {mirror}",
          t.id()
        );
      }
      // 值也要跟着栏位对齐。
      let values: Vec<&str> = items
        .iter()
        .filter(|i| {
          matches!(
            i.text.as_str(),
            "2026-10-01" | "12:34" | "14.074" | "FT8" | "-12"
          )
        })
        .map(|i| i.text.as_str())
        .collect();
      assert_eq!(values.len(), 5, "{} 版式栏位值不齐", t.id());
    }
  }

  #[test]
  fn no_two_lines_overlap() {
    // 「Confirming QSO with」压在对方呼号上这种事故，光看坐标是看不出来的 —— 这里按
    // 保守的墨迹盒估算（上升部 ≈ 0.8 倍字号、不含下降部；宽度 ≈ 0.6 倍字号 × 字符数）
    // 做两两相交检查。同一行（基线相同）的栏位互相跳过：它们本来就并排。
    for t in CardTemplate::ALL {
      let (w, h) = (1400.0, 890.0);
      let items: Vec<CardText> = layout(t, (w, h), &sample())
        .into_iter()
        .filter(|i| !i.text.trim().is_empty())
        .collect();
      let span = |i: &CardText| -> (f64, f64) {
        let ink = i.text.chars().count() as f64 * i.size * 0.6;
        match i.align {
          Align::Left => (i.x, i.x + ink),
          Align::Center => (i.x - ink / 2.0, i.x + ink / 2.0),
          Align::Right => (i.x - ink, i.x),
        }
      };
      for (idx, a) in items.iter().enumerate() {
        for b in items.iter().skip(idx + 1) {
          if (a.y - b.y).abs() < 1e-9 {
            continue; // 同一行
          }
          let (a_top, a_bottom) = (a.y - a.size * 0.8, a.y);
          let (b_top, b_bottom) = (b.y - b.size * 0.8, b.y);
          let vertical = a_top < b_bottom && b_top < a_bottom;
          let (a_l, a_r) = span(a);
          let (b_l, b_r) = span(b);
          let horizontal = a_l < b_r && b_l < a_r;
          assert!(
            !(vertical && horizontal),
            "{} 版式两行文字重叠：「{}」{a:?} / 「{}」{b:?}",
            t.id(),
            a.text,
            b.text
          );
        }
      }
    }
  }

  #[test]
  fn centered_text_is_either_a_column_or_on_the_middle_axis() {
    let (w, h) = (1400.0, 890.0);
    for t in CardTemplate::ALL {
      let items = layout(t, (w, h), &sample());
      let mut middle = 0;
      for item in items.iter().filter(|i| i.align == Align::Center) {
        // 栏位（标题与值）是**列内居中**；其余居中行必须真的落在中线上。
        let is_column = INFO_LABELS.contains(&item.text.as_str())
          || ["2026-10-01", "12:34", "14.074", "FT8", "-12"].contains(&item.text.as_str());
        if is_column {
          continue;
        }
        assert!(
          (item.x - w / 2.0).abs() < 1e-9,
          "{} 版式居中行的 x 不在中线上：{:?}",
          t.id(),
          item
        );
        middle += 1;
      }
      assert!(middle >= 3, "{} 版式的居中行太少：{middle}", t.id());
    }
  }

  #[test]
  fn content_is_carried_through_and_layout_ignores_empty_fields() {
    let (w, h) = (1400.0, 890.0);
    let c = sample();
    for t in CardTemplate::ALL {
      let texts: Vec<String> = layout(t, (w, h), &c).into_iter().map(|i| i.text).collect();
      for expected in [
        "BG4XXX",
        "JA1AAA",
        "PM95",
        "2026-10-01",
        "12:34",
        "14.074",
        "FT8",
        "-12",
      ] {
        assert!(
          texts.iter().any(|x| x == expected),
          "{} 版式丢了字段 {expected}",
          t.id()
        );
      }
      // 本台那一行把网格与 QTH 拼在一起。
      assert!(
        texts
          .iter()
          .any(|x| x.contains("OM89") && x.contains("上海"))
      );
    }

    // 空字段不挪位：条目数与有值时一致（界面层跳过空串）。
    let empty = CardContent::default();
    for t in CardTemplate::ALL {
      assert_eq!(
        layout(t, (w, h), &empty).len(),
        layout(t, (w, h), &sample()).len(),
        "{} 版式在空字段下条目数变了",
        t.id()
      );
      assert!(empty.location_line().is_empty());
    }
  }

  #[test]
  fn decor_matches_the_template_style() {
    let (w, h) = (1400.0, 890.0);
    let kinds =
      |t: CardTemplate| -> Vec<RectKind> { decor(t, (w, h)).iter().map(|r| r.kind).collect() };

    // 经典有边框，极简没有（只有一条线）。
    assert!(kinds(CardTemplate::Classic).contains(&RectKind::Frame));
    assert!(!kinds(CardTemplate::Minimal).contains(&RectKind::Frame));
    assert_eq!(kinds(CardTemplate::Minimal), vec![RectKind::Rule]);

    // 醒目版：顶部色条铺满宽度，信息行底衬确实盖住了那 5 栏文字。
    let bold = decor(CardTemplate::Bold, (w, h));
    let top = bold
      .iter()
      .find(|r| r.kind == RectKind::TopBand)
      .expect("顶部色条");
    assert_eq!((top.x, top.w), (0.0, w));
    let band = bold
      .iter()
      .find(|r| r.kind == RectKind::InfoBand)
      .expect("信息行底衬");
    let info_y: Vec<f64> = layout(CardTemplate::Bold, (w, h), &sample())
      .into_iter()
      .filter(|i| INFO_LABELS.contains(&i.text.as_str()))
      .map(|i| i.y)
      .collect();
    for y in info_y {
      assert!(
        y >= band.y && y <= band.y + band.h,
        "信息行（y={y}）没有落在底衬 {band:?} 上"
      );
    }
  }

  #[test]
  fn a_blank_card_still_has_a_frame_to_print_on() {
    // 一路空着点「打印」不该得到一张全白纸：装饰与固定文案（栏位标题）必须还在。
    let (w, h) = (1400.0, 890.0);
    for t in CardTemplate::ALL {
      assert!(!decor(t, (w, h)).is_empty());
      let texts: Vec<String> = layout(t, (w, h), &CardContent::default())
        .into_iter()
        .map(|i| i.text)
        .collect();
      for label in INFO_LABELS {
        assert!(
          texts.iter().any(|x| x == label),
          "{} 版式缺栏位 {label}",
          t.id()
        );
      }
    }
  }
}
