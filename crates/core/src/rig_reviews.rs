//! 设备评测文章：固定模板「测试条件 → 指标实测 → 主观评价 → 适用人群 → 结论」。
//!
//! # 两条立场（写进数据结构，不靠自觉）
//!
//! - **条件在前**：[`ReviewConditions`] 是 [`RigReview`] 的第一个字段，页面也最先渲染它。
//!   换一根天线，同一台接收机可以「好」也可以「差」—— 没有条件的评测没有意义。
//! - **指标实测不抄数字，与机型库同源**：评测文章**不**自己复制一遍数值，`指标实测` 那一栏
//!   直接由 [`crate::gear_rx`]（Sherwood 表）按器材 id 生成 —— 表格更新时评测不会过期；
//!   原表没测过的机器那一栏如实显示「未收录」，而不是找一个别的数字顶上。
//! - **主观与实测分开**：主观条目明确标注「主观」，其依据是公开资料与社区共识，
//!   不冒充本站实测（页面脚注写明这一点）。

use crate::gear_rx::GearRx;
use crate::gear_score::Axis;

/// 测试条件。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReviewConditions {
  /// 测试用天线（台架测试的数据写「不适用」，见各篇）。
  pub antenna: &'static str,
  /// 测试环境（实验室 / 场地、噪声情况）。
  pub environment: &'static str,
  /// 测试仪表。
  pub instruments: &'static str,
  /// 数据采集时间（有实测数据的机型，这一项必须等于原表该行时间，见单测）。
  pub when: &'static str,
}

impl ReviewConditions {
  /// 四项都写了吗 —— 「条件前置」的机械保证：单测要求每篇都齐。
  #[must_use]
  pub fn is_complete(&self) -> bool {
    !self.antenna.is_empty()
      && !self.environment.is_empty()
      && !self.instruments.is_empty()
      && !self.when.is_empty()
  }
}

/// 一篇评测文章。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RigReview {
  /// 器材 id（[`crate::gear::Gear::id`]；标题、频段等一律从器材库取，不另写一份）。
  pub rig_id: &'static str,
  /// 测试条件 —— 结构上的第一位。
  pub conditions: ReviewConditions,
  /// 主观评价（每条一行；与实测指标分开，不冒充测量）。
  pub subjective: &'static [&'static str],
  /// 适用人群。
  pub for_whom: &'static str,
  /// 结论（一句话）。
  pub conclusion: &'static str,
}

/// 「指标实测」栏的一行：取 [`crate::gear_rx`] 的哪一轴 + 术语表哪一条。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReviewMetricRow {
  /// 取数轴（数值一律来自原表，评测不存自己的数字）。
  pub axis: Axis,
  /// [`REVIEW_GLOSSARY`] 的下标。
  pub glossary: usize,
}

/// 「指标实测」栏用到的行（与术语表一一对应，顺序即页面顺序）。
pub const REVIEW_METRICS: &[ReviewMetricRow] = &[
  ReviewMetricRow {
    axis: Axis::ImdNarrow,
    glossary: 0,
  },
  ReviewMetricRow {
    axis: Axis::ImdWide,
    glossary: 1,
  },
  ReviewMetricRow {
    axis: Axis::NoiseFloor,
    glossary: 2,
  },
];

/// 指标名词表：`(名词, 含义, 家里怎么量)` —— 评测里的指标名互链到这里。
pub const REVIEW_GLOSSARY: &[(&str, &str, &str)] = &[
  (
    "窄间隔三阶互调动态范围（RMDR）",
    "2–3 kHz 间隔下同时处理强台与弱台的能力；测量受相位噪声限制时，这一列就是 ARRL RMDR。",
    "两台信号源（或一台双音源），间隔 2 kHz",
  ),
  (
    "20 kHz 间隔三阶互调动态范围",
    "较宽间隔下的互调抑制能力，反映滤波器与混频级的整体线性度。",
    "同上，间隔拉到 20 kHz",
  ),
  (
    "噪声底",
    "接收机本身的底噪电平（dBm），越接近 0 越好；主要由前级噪声系数决定。",
    "输入端接匹配负载，看频谱或电平表",
  ),
];

/// 已收录的评测文章。
pub const RIG_REVIEWS: &[RigReview] = &[
  RigReview {
    rig_id: "ft-710",
    conditions: ReviewConditions {
      antenna: "不适用（台架测试，不经过天线）",
      environment: "实验室，受控信号源（Sherwood 台架）",
      instruments: "双音信号源、频谱仪",
      when: "2022-10-01",
    },
    subjective: &[
      "107 dB 的窄间隔动态范围在中端价位里是第一梯队：竞赛环境里邻频大台不易压住弱台。",
      "菜单层级较深，社区普遍反映上手需要一段时间。",
    ],
    for_whom: "预算有限、想打比赛或追弱信号的短波爱好者。",
    conclusion: "动态范围的性价比标杆；操作学习成本稍高。",
  },
  RigReview {
    rig_id: "ic-7300",
    conditions: ReviewConditions {
      antenna: "不适用（台架测试，不经过天线）",
      environment: "实验室，受控信号源（Sherwood 台架）",
      instruments: "双音信号源、频谱仪",
      when: "2016-04-25",
    },
    subjective: &[
      "噪声底 −133 dBm 比同价位更好，弱信号收听有底子。",
      "实时频谱与瀑布图对追 DX 帮助明显。",
      "94 dB 的窄间隔动态范围够日常用，但大台密集时不如 FT-710。",
    ],
    for_whom: "第一台短波电台、喜欢频谱显示的用户。",
    conclusion: "均衡全面的入门经典，胜在好用而不是参数顶尖。",
  },
  RigReview {
    rig_id: "xiegu-g90",
    conditions: ReviewConditions {
      antenna: "不适用（台架测试，不经过天线）",
      environment: "实验室，受控信号源（Sherwood 台架）",
      instruments: "双音信号源、频谱仪",
      when: "2021-05-15",
    },
    subjective: &[
      "76 dB 的窄间隔动态范围意味着强信号环境下更容易互调，这是价格与体积换来的。",
      "内置天调对野外架台省一套设备。",
    ],
    for_whom: "预算极低、以 QRP / 野外玩法为主的爱好者。",
    conclusion: "低价的代价在接收动态范围；野外轻量玩法够用。",
  },
  RigReview {
    rig_id: "kx2",
    conditions: ReviewConditions {
      antenna: "不适用（台架测试，不经过天线）",
      environment: "实验室，受控信号源（Sherwood 台架）",
      instruments: "双音信号源、频谱仪",
      when: "2017-07-26",
    },
    subjective: &[
      "86 dB（3 kHz 间隔）的窄间隔动态范围在 QRP 便携机里靠前。",
      "体积能装进口袋，但 10W 级功率决定它更适合 CW 与轻量 SSB。",
    ],
    for_whom: "背包客、SOTA 玩家。",
    conclusion: "便携优先的选择，接收性能在同类里靠前。",
  },
  RigReview {
    rig_id: "uv-5r",
    conditions: ReviewConditions {
      antenna: "未收录 —— 本机不在 Sherwood 实测表内",
      environment: "未收录",
      instruments: "未收录",
      when: "—",
    },
    subjective: &[
      "价格极低、配件生态庞大，是很多人第一台对讲机。",
      "接收性能没有公开的第三方实测数据，本文不给它打分或排名。",
    ],
    for_whom: "入门练手、对参数没有要求的用户。",
    conclusion: "便宜易得；想比接收性能的话，先在机型库里录入自己的实测值。",
  },
];

/// 按器材 id 找评测。
#[must_use]
pub fn for_rig(id: &str) -> Option<&'static RigReview> {
  RIG_REVIEWS.iter().find(|r| r.rig_id == id)
}

/// 「指标实测」某一行的数值（只从原表取；`None` = 未收录）。
#[must_use]
pub fn metric_value(row: &ReviewMetricRow, rx: Option<&GearRx>) -> Option<f64> {
  let rx = rx?;
  match row.axis {
    Axis::ImdNarrow => Some(rx.imd_narrow_db),
    Axis::ImdWide => Some(rx.imd_wide_db),
    Axis::NoiseFloor => Some(rx.noise_floor_dbm),
    _ => None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::gear;

  #[test]
  fn every_review_points_at_a_rig_in_the_library() {
    assert!(
      RIG_REVIEWS.len() >= 4,
      "评测太少（{} 篇）",
      RIG_REVIEWS.len()
    );
    let mut ids: Vec<&str> = RIG_REVIEWS.iter().map(|r| r.rig_id).collect();
    let n = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), n, "同一台机器不该有两篇评测");
    for r in RIG_REVIEWS {
      let gear = gear::by_id(r.rig_id)
        .unwrap_or_else(|| panic!("{} 不在器材库里：评测必须挂在真实机型上", r.rig_id));
      assert!(
        !gear.model.is_empty() && !r.conclusion.is_empty() && !r.for_whom.is_empty(),
        "{} 的结论与适用人群不能空",
        r.rig_id
      );
      assert!(!r.subjective.is_empty(), "{} 的主观条目不能空", r.rig_id);
      assert!(r.subjective.iter().all(|s| !s.is_empty()));
    }
  }

  #[test]
  fn conditions_come_first_and_are_complete() {
    // 「条件前置」落成两件事：字段在结构第一位（页面按字段顺序渲染），且每篇都写全。
    for r in RIG_REVIEWS {
      assert!(r.conditions.is_complete(), "{} 的测试条件没写全", r.rig_id);
    }
  }

  #[test]
  fn the_review_date_matches_the_table_row() {
    // 评测引用的实测数据与它的「数据时间」必须同源：表格哪天更新，这一项就得跟着改，
    // 否则页面会出现「评测说 10 月测的、表格里是 4 月的行」这种对不上的情况。
    for r in RIG_REVIEWS {
      if let Some(rx) = crate::gear_rx::for_gear(r.rig_id) {
        assert_eq!(
          r.conditions.when, rx.measured,
          "{} 的数据时间（{}）与原表该行（{}）不一致",
          r.rig_id, r.conditions.when, rx.measured
        );
      }
    }
  }

  #[test]
  fn metric_rows_line_up_with_the_glossary() {
    // 每行指向的术语条都在表内，且三条轴正好是原表收录的那三列 —— 不是这三列的
    // 「指标」没数据可画，不该出现在行里。
    for row in REVIEW_METRICS {
      assert!(
        row.glossary < REVIEW_GLOSSARY.len(),
        "行指向的术语条 {} 越界",
        row.glossary
      );
      assert!(
        matches!(row.axis, Axis::ImdNarrow | Axis::ImdWide | Axis::NoiseFloor),
        "{:?} 不是实测表的轴",
        row.axis
      );
    }
    // 术语表没有「没人引用的条目」（否则页面下会出现孤儿名词）。
    for (i, _) in REVIEW_GLOSSARY.iter().enumerate() {
      assert!(
        REVIEW_METRICS.iter().any(|row| row.glossary == i),
        "术语第 {i} 条没有被任何指标行引用"
      );
    }
  }

  #[test]
  fn metric_values_come_from_the_table_only() {
    let rx = crate::gear_rx::for_gear("ft-710");
    assert!(rx.is_some());
    assert_eq!(
      metric_value(&REVIEW_METRICS[0], rx),
      Some(107.0),
      "窄间隔动态范围来自原表"
    );
    assert_eq!(
      metric_value(&REVIEW_METRICS[0], None),
      None,
      "未收录就是未收录"
    );
  }

  /// 取出文本里「数字 + dB / dBm」的数值（`107 dB`、`−133 dBm`；前面带减号 / 负号的取负）。
  fn db_figures(text: &str) -> Vec<f64> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
      if !chars[i].is_ascii_digit() {
        i += 1;
        continue;
      }
      let start = i;
      while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
        i += 1;
      }
      let number: String = chars[start..i].iter().collect();
      let mut unit = i;
      if chars.get(unit) == Some(&' ') {
        unit += 1;
      }
      if chars[unit.min(chars.len())..].starts_with(&['d', 'B']) {
        let value: f64 = number.parse().expect("数字");
        let negative = start > 0 && matches!(chars[start - 1], '−' | '-' | '–');
        out.push(if negative { -value } else { value });
      }
    }
    out
  }

  #[test]
  fn db_figures_in_subjective_lines_match_the_table() {
    // 评测不抄数字：主观评价里引用的每个 dB / dBm 数值都必须能在原表（同一机型）里找到，
    // 表格改了数字、文字没跟着改，这里就红；没有实测数据的机型不许出现这类数字。
    assert_eq!(
      db_figures("噪声底 −133 dBm，94 dB（3 kHz），10W"),
      vec![-133.0, 94.0]
    );
    for r in RIG_REVIEWS {
      let figures: Vec<f64> = r.subjective.iter().flat_map(|s| db_figures(s)).collect();
      match crate::gear_rx::for_gear(r.rig_id) {
        Some(rx) => {
          let table = [rx.imd_narrow_db, rx.imd_wide_db, rx.noise_floor_dbm];
          for f in figures {
            assert!(
              table.iter().any(|t| (t - f).abs() < 0.5),
              "{} 的主观评价写了 {f} dB，原表只有 {table:?}",
              r.rig_id
            );
          }
        }
        None => assert!(
          figures.is_empty(),
          "{} 没有实测数据，主观评价里不该出现 dB 数值：{figures:?}",
          r.rig_id
        ),
      }
    }
  }
}
