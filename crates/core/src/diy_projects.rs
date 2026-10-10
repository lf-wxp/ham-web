//! DIY 实战项目：结构化卡片（难度 / 成本 / 工时 / BOM / 资料 / 测试步骤 / 常见问题）
//! + 制作进度（本地存储）。
//!
//! # 两条立场
//!
//! - **成本只给档位，不给具体价格**：元器件价格随市场与渠道变动，给一个具体数字很快就会
//!   过期（与器材库「不写具体售价」同一立场）。档位口径写在常量 [`COST_CHECKED_ON`] 附近。
//! - **BOM 是「典型配置」不是采购清单**：同一个项目有多种成熟做法，这里列的是最常见、
//!   最容易买到的一套；具体型号不指定（避免替商家背书）。
//! - **资料链接只给长期稳定的**：站内知识页为主，站外只有 AA5TB 小环、RTL-SDR 官网这类
//!   存在二十年以上的公认出处；没有公认出处的项目（如 CW 练习器）资料留空，如实显示。

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// 成本档位的估算口径核对日期（页面标注「价格随市场变动」）。
pub const COST_CHECKED_ON: &str = "2026-10";

/// 难度。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difficulty {
  /// 入门：不需要特殊仪器与经验。
  Beginner,
  /// 进阶：需要调测仪器或制作经验。
  Advanced,
}

/// 成本档位（按常见零售价估算，随市场变动）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cost {
  /// 低：百元内。
  Low,
  /// 中：数百元。
  Mid,
}

/// 项目涉及的波段（筛选器用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiyBand {
  /// 短波 HF。
  Hf,
  /// VHF / UHF。
  VhfUhf,
  /// 不涉及射频（如练习器）。
  Other,
}

/// 制作 / 调测所需的仪表（筛选器用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instrument {
  /// 驻波表。
  SwrMeter,
  /// 天线分析仪 / VNA。
  Vna,
  /// 万用表。
  Multimeter,
}

/// 一个 DIY 项目卡片。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiyProject {
  /// 项目名（本地进度的 key；全表唯一）。
  pub name: &'static str,
  pub difficulty: Difficulty,
  pub cost: Cost,
  /// 工时（估算区间文本，如 `2–4 小时`）。
  pub hours: &'static str,
  pub band: DiyBand,
  /// 所需仪表（制作 + 调测全程）。
  pub instruments: &'static [Instrument],
  /// 材料清单：`(材料, 用量 / 说明)`。典型配置，不指定具体型号。
  pub bom: &'static [(&'static str, &'static str)],
  /// 资料：`(名称, 站内路径或 https 链接)`。可为空（没有公认出处就留空，不凑数）。
  pub links: &'static [(&'static str, &'static str)],
  /// 测试步骤（做完怎么验收）。
  pub tests: &'static [&'static str],
  /// 常见问题（前人踩过的坑）。
  pub pitfalls: &'static [&'static str],
  /// 一句话要点。
  pub notes: &'static str,
}

/// 全部项目（页面按此顺序展示）。
pub const DIY_PROJECTS: &[DiyProject] = &[
  DiyProject {
    name: "半波偶极天线",
    difficulty: Difficulty::Beginner,
    cost: Cost::Low,
    hours: "2–4 小时",
    band: DiyBand::Hf,
    instruments: &[Instrument::SwrMeter],
    bom: &[
      ("铜导线", "2 根，各 1/4 波长 + 修剪余量"),
      ("绝缘子", "2–3 只"),
      ("1:1 巴伦", "可自制（见巴伦项目）"),
      ("同轴馈线", "按架设距离"),
      ("扎带、防水胶带", "若干"),
    ],
    links: &[
      ("天线 DIY 做法", "/antenna-diy"),
      ("1:1 巴伦做法", "/balun"),
    ],
    tests: &[
      "接上馈线后测驻波比，谐振点应接近设计频率。",
      "谐振点偏低（频率低）→ 修剪缩短振子；两边同时修剪保持对称。",
    ],
    pitfalls: &[
      "高度不足时谐振点整体偏移，先固定架设高度再修剪。",
      "修剪过头无法加长 —— 下料时留足余量。",
      "馈线外皮也是辐射体的一部分，巴伦不可省。",
    ],
    notes: "按 143/f 计算长度，接巴伦，测驻波修剪",
  },
  DiyProject {
    name: "1:1 电流巴伦",
    difficulty: Difficulty::Beginner,
    cost: Cost::Low,
    hours: "1–2 小时",
    band: DiyBand::Hf,
    instruments: &[Instrument::Multimeter, Instrument::SwrMeter],
    bom: &[
      ("铁氧体磁环", "FT240-43 一类（功率用）"),
      ("漆包线", "双线并绕 8–12 圈"),
      ("防水接线盒", "1 只"),
      ("SO-239 座", "2 只"),
    ],
    links: &[("扼流型巴伦原理", "/balun")],
    tests: &[
      "万用表测通断与绝缘：输入输出各自导通，对外壳绝缘。",
      "输出接假负载，输入端看驻波应接近 1:1。",
    ],
    pitfalls: &[
      "磁环碎裂会明显改变性能，固定螺丝别上太紧。",
      "大功率连续发射时留意磁环饱和发烫。",
      "双线并绕别绞得太紧，容易伤漆皮造成短路。",
    ],
    notes: "磁环绕 8–12 圈同轴/双线，抑制共模",
  },
  DiyProject {
    name: "磁环小环天线",
    difficulty: Difficulty::Advanced,
    cost: Cost::Mid,
    hours: "4–8 小时",
    band: DiyBand::Hf,
    instruments: &[Instrument::Vna],
    bom: &[
      ("粗铜管或铜带", "环径约 1 米"),
      ("真空可变电容", "高耐压型"),
      ("防水连接盒", "1 只"),
      ("小馈电环", "铜管弯制"),
    ],
    links: &[("AA5TB 小环设计", "https://www.aa5tb.com/loop.html")],
    tests: &[
      "用 VNA 看谐振与驻波，电容调谐范围应覆盖目标频段。",
      "先用小功率试发，确认无异常再逐步加大，观察电容与连接点是否打火；发射时人体与环体保持安全距离。",
    ],
    pitfalls: &[
      "谐振带宽很窄，换频段要重新调谐。",
      "电容耐压不够会打火，按发射功率选耐压。",
      "环体电阻大则效率剧降，压接点必须接触良好。",
    ],
    notes: "高压电容 + 粗管环形，注意耐压",
  },
  DiyProject {
    name: "RTL-SDR 接收机",
    difficulty: Difficulty::Beginner,
    cost: Cost::Low,
    hours: "1 小时",
    band: DiyBand::VhfUhf,
    instruments: &[],
    bom: &[
      ("RTL2832U 电视棒", "1 只"),
      ("SMA 转接头", "按天线接口"),
      ("上变频器", "收 HF 波段时才需要"),
    ],
    links: &[("RTL-SDR 官网", "https://www.rtl-sdr.com")],
    tests: &[
      "装好驱动后用 SDR 软件收到本地 FM 广播即算通。",
      "再试业余段信号（144 / 430 MHz 中继等）。",
    ],
    pitfalls: &[
      "早期批次晶振频偏明显，先预热几分钟再校频。",
      "直采收不到 HF，需要上变频器。",
      "前端容易过载，强信号环境降低增益。",
    ],
    notes: "电视棒 + 上变频器，配合 SDR 软件",
  },
  DiyProject {
    name: "CW 练习器",
    difficulty: Difficulty::Beginner,
    cost: Cost::Low,
    hours: "2–3 小时",
    band: DiyBand::Other,
    instruments: &[Instrument::Multimeter],
    bom: &[
      ("Arduino 开发板", "1 块"),
      ("蜂鸣器", "1 只"),
      ("按键或电键接口", "1 套"),
      ("电池盒", "1 只"),
    ],
    links: &[],
    tests: &["按键输出正确的点划与间隔。", "换用电键接口再测一遍。"],
    pitfalls: &[
      "点划比例与间隔由程序控制，先用默认节拍练。",
      "蜂鸣器音量可以串一只电位器调节。",
    ],
    notes: "单片机 / Arduino 生成摩尔斯音频",
  },
  DiyProject {
    name: "陷波器",
    difficulty: Difficulty::Advanced,
    cost: Cost::Mid,
    hours: "2–3 小时",
    band: DiyBand::Hf,
    instruments: &[Instrument::Vna],
    bom: &[
      ("线圈", "骨架绕制"),
      ("高耐压电容", "按承受功率选"),
      ("接线柱", "2 只"),
      ("防水外壳", "1 只"),
    ],
    links: &[("滤波器基础", "/filters")],
    tests: &[
      "用 VNA 看陷波频率与深度。",
      "装到振子上，验证对目标频段的抑制。",
    ],
    pitfalls: &[
      "电容耐压与损耗直接决定能承受的功率。",
      "陷波深度与带宽是权衡，别一味求深。",
      "装进外壳后参数会漂，先粗调再细调。",
    ],
    notes: "LC 并联谐振，抑制本地强台",
  },
];

/// 通用制作流程（步骤）。
pub const DIY_STEPS: &[(&str, &str)] = &[
  ("选项目", "从天线 / 巴伦等低门槛项目入手，备齐工具与材料。"),
  (
    "算参数",
    "用站内计算器（天线长度、LC 谐振、巴伦匝数）确定尺寸。",
  ),
  ("制作", "按图施工，焊接 / 压接牢固，留好调试余量。"),
  ("测量", "用驻波表 / 天线分析仪 / VNA 测量，逐步调整。"),
  ("记录", "记录尺寸与结果，形成可复现的制作笔记。"),
];

/// 制作安全提醒。
pub const DIY_SAFETY: &[&str] = &[
  "高处架设天线注意防坠落与防雷，雷雨天停止操作。",
  "发射前确认天线驻波与匹配，避免损坏功放。",
  "电容储能与高压电路注意放电后再触碰。",
  "遵守当地法规，控制发射功率与频段。",
];

/// 筛选条件（任一维度 `None` = 不限）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DiyFilters {
  pub difficulty: Option<Difficulty>,
  pub cost: Option<Cost>,
  pub band: Option<DiyBand>,
  pub instrument: Option<Instrument>,
}

impl DiyFilters {
  /// 项目是否满足筛选条件。
  #[must_use]
  pub fn matches(&self, p: &DiyProject) -> bool {
    self.difficulty.is_none_or(|d| p.difficulty == d)
      && self.cost.is_none_or(|c| p.cost == c)
      && self.band.is_none_or(|b| p.band == b)
      && self.instrument.is_none_or(|i| p.instruments.contains(&i))
  }
}

/// 单个项目的制作进度（只存本地）。
///
/// BOM 勾选按**材料名称**存：BOM 行改名字会丢对应勾选 —— 本地标注而已，可接受；
/// 比按行号存强在加行 / 调序不会错位。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiyProgress {
  /// 项目是否完成。
  #[serde(default)]
  pub done: bool,
  /// 已采购的 BOM 材料名。
  #[serde(default)]
  pub bought: BTreeSet<String>,
}

/// 全部项目的进度：`项目名 → 进度`。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DiyProgressBook(BTreeMap<String, DiyProgress>);

impl DiyProgressBook {
  /// 项目是否完成。
  #[must_use]
  pub fn is_done(&self, project: &str) -> bool {
    self.0.get(project).is_some_and(|p| p.done)
  }

  /// 某样材料是否已采购。
  #[must_use]
  pub fn is_bought(&self, project: &str, item: &str) -> bool {
    self.0.get(project).is_some_and(|p| p.bought.contains(item))
  }

  /// 切换「项目完成」；返回切换后的值。
  pub fn toggle_done(&mut self, project: &str) -> bool {
    let entry = self.0.entry(project.to_owned()).or_default();
    entry.done = !entry.done;
    entry.done
  }

  /// 切换某样材料的采购状态；返回切换后的值。
  pub fn toggle_bought(&mut self, project: &str, item: &str) -> bool {
    let entry = self.0.entry(project.to_owned()).or_default();
    if !entry.bought.remove(item) {
      entry.bought.insert(item.to_owned());
      return true;
    }
    false
  }

  /// 是否一个勾都没有（存储里不留空壳）。
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.0.values().all(|p| !p.done && p.bought.is_empty())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn every_card_is_complete_and_names_are_unique() {
    assert_eq!(DIY_PROJECTS.len(), 6, "六张卡片");
    let mut names: Vec<&str> = DIY_PROJECTS.iter().map(|p| p.name).collect();
    let n = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), n, "项目名必须唯一（它是进度存储的 key）");
    for p in DIY_PROJECTS {
      assert!(!p.hours.is_empty(), "{} 缺工时", p.name);
      assert!(!p.notes.is_empty(), "{} 缺要点", p.name);
      assert!(!p.bom.is_empty(), "{} 缺 BOM", p.name);
      assert!(!p.tests.is_empty(), "{} 缺测试步骤", p.name);
      assert!(!p.pitfalls.is_empty(), "{} 缺常见问题", p.name);
      for &(item, how) in p.bom {
        assert!(!item.is_empty() && !how.is_empty(), "{} 的 BOM 行", p.name);
      }
      // 资料链接：站内路径或 https；没有公认出处的留空（不凑数）。
      for &(label, url) in p.links {
        assert!(!label.is_empty(), "{} 的链接缺名称", p.name);
        assert!(
          url.starts_with('/') || url.starts_with("https://"),
          "{} 的链接不是站内路径也不是 https：{url}",
          p.name
        );
      }
      // BOM 材料名在项目内唯一（进度勾选按名称存）。
      let mut items: Vec<&str> = p.bom.iter().map(|&(i, _)| i).collect();
      let m = items.len();
      items.sort_unstable();
      items.dedup();
      assert_eq!(items.len(), m, "{} 的 BOM 有重名材料", p.name);
    }
  }

  #[test]
  fn every_filter_value_is_used_by_some_project() {
    // 筛选器的每个选项都要有项目可筛出来，否则是死选项。
    for d in [Difficulty::Beginner, Difficulty::Advanced] {
      assert!(
        DIY_PROJECTS.iter().any(|p| p.difficulty == d),
        "{d:?} 没有项目"
      );
    }
    for c in [Cost::Low, Cost::Mid] {
      assert!(DIY_PROJECTS.iter().any(|p| p.cost == c), "{c:?} 没有项目");
    }
    for b in [DiyBand::Hf, DiyBand::VhfUhf, DiyBand::Other] {
      assert!(DIY_PROJECTS.iter().any(|p| p.band == b), "{b:?} 没有项目");
    }
    for i in [
      Instrument::SwrMeter,
      Instrument::Vna,
      Instrument::Multimeter,
    ] {
      assert!(
        DIY_PROJECTS.iter().any(|p| p.instruments.contains(&i)),
        "{i:?} 没有项目需要"
      );
    }
  }

  #[test]
  fn filters_match_on_each_dimension() {
    let filters = DiyFilters {
      difficulty: Some(Difficulty::Advanced),
      cost: None,
      band: Some(DiyBand::Hf),
      instrument: Some(Instrument::Vna),
    };
    let hits: Vec<&str> = DIY_PROJECTS
      .iter()
      .filter(|p| filters.matches(p))
      .map(|p| p.name)
      .collect();
    assert_eq!(hits, vec!["磁环小环天线", "陷波器"]);
    // 单独按仪表筛：只有小环与陷波器要 VNA。
    let by_vna = DiyFilters {
      instrument: Some(Instrument::Vna),
      ..Default::default()
    };
    assert_eq!(DIY_PROJECTS.iter().filter(|p| by_vna.matches(p)).count(), 2);
    // 空条件 = 全过。
    assert_eq!(
      DIY_PROJECTS
        .iter()
        .filter(|p| DiyFilters::default().matches(p))
        .count(),
      DIY_PROJECTS.len()
    );
  }

  #[test]
  fn progress_toggles_and_round_trips() {
    let mut book = DiyProgressBook::default();
    assert!(book.is_empty());
    assert!(!book.is_done("半波偶极天线"));
    assert!(book.toggle_done("半波偶极天线"));
    assert!(book.is_done("半波偶极天线"));
    assert!(book.toggle_bought("半波偶极天线", "铜导线"));
    assert!(book.is_bought("半波偶极天线", "铜导线"));
    assert!(
      !book.toggle_bought("半波偶极天线", "铜导线"),
      "再点一次取消"
    );
    assert!(!book.is_empty());

    let json = serde_json::to_string(&book).expect("序列化");
    let back: DiyProgressBook = serde_json::from_str(&json).expect("反序列化");
    assert_eq!(back, book);
    assert!(back.is_done("半波偶极天线"));
    // 缺字段的老数据也能读。
    let sparse: DiyProgressBook =
      serde_json::from_str("{\"陷波器\":{\"done\":true}}").expect("缺字段也能读");
    assert!(sparse.is_done("陷波器"));
  }

  #[test]
  fn steps_and_safety_stay_populated() {
    assert!(!DIY_STEPS.is_empty());
    assert!(!DIY_SAFETY.is_empty());
    assert!(COST_CHECKED_ON.starts_with("20"));
  }
}
