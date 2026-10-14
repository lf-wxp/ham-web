//! NEC 求解器的「设计存档」：把当前模型按名字存进本地，之后一键载回。
//!
//! # 为什么不直接存 `.nec` 文本
//!
//! `.nec` 导出（[`crate::nec::to_nec`]）是给别的软件读的**有损**格式：馈电与负载位置
//! 按段号写（`EX 0, 导线, 段号`），载回后落在段中心而不是原来的相对位置；导线材质
//! （电导率）与传输线（`TL`）也都不在里面。存档要的是「存什么、回来就是什么」，
//! 所以这里用一份与 [`NecInput`] 字段一一对应的结构化快照，往返**逐位相等**
//! （单测钉住）。`.nec` 导入导出仍然留给与外部软件交换。
//!
//! # 存储
//!
//! 整本存档是一份 JSON，走存储门面（`kv::save_large`，key = [`DESIGNS_KEY`]），
//! 因此自动进入备份的「其他本地数据」分类（见 [`crate::backup_merge::classify_key`]）。

use serde::{Deserialize, Serialize};

use crate::nec::{Feed, Ground, Load, MAX_SEGMENTS, NecInput, TransmissionLine, Wire};

/// 存档的持久化 key。
pub const DESIGNS_KEY: &str = "nec-designs";
/// 解析失败的存档原文的备份 key：坏数据不能因为用户随后再存一份就被覆盖掉。
pub const CORRUPT_KEY: &str = "nec-designs-corrupt";
/// 最多保存多少份：每份几 KB，50 份也不到 localStorage 配额的零头；设上限是为了
/// 列表还能一眼扫完，而不是体积。
pub const MAX_DESIGNS: usize = 50;
/// 名称最长字符数（超出截断）。
pub const MAX_NAME_CHARS: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct SavedWire {
  a: [f64; 3],
  b: [f64; 3],
  radius: f64,
  segments: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct SavedFeed {
  wire: usize,
  at: f64,
  volts: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct SavedLoad {
  wire: usize,
  at: f64,
  r_ohm: f64,
  l_uh: f64,
  c_pf: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SavedGround {
  Free,
  Perfect,
  Lossy { eps_r: f64, sigma: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct SavedLine {
  wire_a: usize,
  at_a: f64,
  wire_b: usize,
  at_b: f64,
  z0: f64,
  length_m: f64,
  velocity_factor: f64,
}

/// 一份模型快照（与 [`NecInput`] + 传输线一一对应）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedModel {
  wires: Vec<SavedWire>,
  feeds: Vec<SavedFeed>,
  #[serde(default)]
  loads: Vec<SavedLoad>,
  freq_hz: f64,
  ground: SavedGround,
  /// 导线电导率（S/m）；`None` = 理想导体。
  #[serde(default)]
  conductivity: Option<f64>,
  #[serde(default)]
  lines: Vec<SavedLine>,
}

impl SavedModel {
  /// 从求解输入与传输线表生成快照。
  #[must_use]
  pub fn from_input(input: &NecInput, lines: &[TransmissionLine]) -> Self {
    Self {
      wires: input
        .wires
        .iter()
        .map(|w| SavedWire {
          a: w.a,
          b: w.b,
          radius: w.radius,
          segments: w.segments,
        })
        .collect(),
      feeds: input
        .feeds
        .iter()
        .map(|f| SavedFeed {
          wire: f.wire,
          at: f.at,
          volts: f.volts,
        })
        .collect(),
      loads: input
        .loads
        .iter()
        .map(|l| SavedLoad {
          wire: l.wire,
          at: l.at,
          r_ohm: l.r_ohm,
          l_uh: l.l_uh,
          c_pf: l.c_pf,
        })
        .collect(),
      freq_hz: input.freq_hz,
      ground: match input.ground {
        Ground::Free => SavedGround::Free,
        Ground::Perfect => SavedGround::Perfect,
        Ground::Lossy { eps_r, sigma } => SavedGround::Lossy { eps_r, sigma },
      },
      conductivity: input.conductivity,
      lines: lines
        .iter()
        .map(|l| SavedLine {
          wire_a: l.wire_a,
          at_a: l.at_a,
          wire_b: l.wire_b,
          at_b: l.at_b,
          z0: l.z0,
          length_m: l.length_m,
          velocity_factor: l.velocity_factor,
        })
        .collect(),
    }
  }

  /// 还原成求解输入与传输线表。
  ///
  /// 导线直接按字段构造而不是走 [`Wire::new`]：`new` 会把半径 / 段数夹到下限，
  /// 而快照本来就来自一份合法输入，再夹一次只会让「往返逐位相等」变得不成立。
  #[must_use]
  pub fn to_input(&self) -> (NecInput, Vec<TransmissionLine>) {
    let input = NecInput {
      wires: self
        .wires
        .iter()
        .map(|w| Wire {
          a: w.a,
          b: w.b,
          radius: w.radius,
          segments: w.segments,
        })
        .collect(),
      feeds: self
        .feeds
        .iter()
        .map(|f| Feed {
          wire: f.wire,
          at: f.at,
          volts: f.volts,
        })
        .collect(),
      loads: self
        .loads
        .iter()
        .map(|l| Load {
          wire: l.wire,
          at: l.at,
          r_ohm: l.r_ohm,
          l_uh: l.l_uh,
          c_pf: l.c_pf,
        })
        .collect(),
      freq_hz: self.freq_hz,
      ground: match self.ground {
        SavedGround::Free => Ground::Free,
        SavedGround::Perfect => Ground::Perfect,
        SavedGround::Lossy { eps_r, sigma } => Ground::Lossy { eps_r, sigma },
      },
      conductivity: self.conductivity,
    };
    let lines = self
      .lines
      .iter()
      .map(|l| TransmissionLine {
        wire_a: l.wire_a,
        at_a: l.at_a,
        wire_b: l.wire_b,
        at_b: l.at_b,
        z0: l.z0,
        length_m: l.length_m,
        velocity_factor: l.velocity_factor,
      })
      .collect();
    (input, lines)
  }

  /// 快照是否可用：能被求解器接受 —— 至少一根导线、一个馈电，所有数值是有限数，
  /// 各处的导线下标都在范围内，总段数不超过 [`MAX_SEGMENTS`]。
  ///
  /// 存档文件可能被手改或来自更旧 / 更新的版本：载回一份坏数据的代价是页面上一片
  /// 「参数无法求解」且用户不知道为什么，所以载入前先判一次，坏的直接不给载。
  /// 判据与求解器的入口检查对齐（`solve_with_lines`），否则「载入」按钮亮着、载回来却算不出。
  #[must_use]
  pub fn is_usable(&self) -> bool {
    let finite = |v: &[f64]| v.iter().all(|x| x.is_finite());
    let n = self.wires.len();
    let total: usize = self.wires.iter().map(|w| w.segments).sum();
    !self.wires.is_empty()
      && !self.feeds.is_empty()
      && self.freq_hz.is_finite()
      && self.freq_hz > 0.0
      && total <= MAX_SEGMENTS
      && self.wires.iter().all(|w| {
        finite(&w.a) && finite(&w.b) && w.radius.is_finite() && w.radius > 0.0 && w.segments > 0
      })
      && self
        .feeds
        .iter()
        .all(|f| f.wire < n && finite(&[f.at, f.volts]))
      && self
        .loads
        .iter()
        .all(|l| l.wire < n && finite(&[l.at, l.r_ohm, l.l_uh, l.c_pf]))
      && self.lines.iter().all(|l| {
        l.wire_a < n
          && l.wire_b < n
          && finite(&[l.at_a, l.at_b, l.z0, l.length_m, l.velocity_factor])
          && l.z0 > 0.0
          && l.length_m >= 0.0
          && l.velocity_factor > 0.0
      })
      && match self.ground {
        SavedGround::Lossy { eps_r, sigma } => finite(&[eps_r, sigma]),
        _ => true,
      }
      && self.conductivity.is_none_or(|c| c.is_finite() && c > 0.0)
  }
}

/// 一份存档。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedDesign {
  /// 本机自增 id（只在存档册内唯一）。
  pub id: u64,
  /// 用户起的名字。
  pub name: String,
  /// 保存日期（`YYYY-MM-DD`，由调用方传入：核心不读时钟）。
  pub saved_at: String,
  /// 模型快照。
  pub model: SavedModel,
}

/// 保存失败的原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveError {
  /// 名字为空。
  EmptyName,
  /// 当前模型不可用（例如导线表里有格子没填好）。
  Unusable,
  /// 已达上限 [`MAX_DESIGNS`]（同名覆盖不受限）。
  Full,
}

/// 存档册（最新的在前）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DesignBook {
  #[serde(default)]
  pub designs: Vec<SavedDesign>,
}

impl DesignBook {
  /// 解析持久化的 JSON；解析失败返回 `None`，由调用方决定怎么保全原文
  /// （见 [`CORRUPT_KEY`]）—— 直接当空册处理的话，用户下一次保存就把坏数据永久覆盖了。
  #[must_use]
  pub fn parse(json: &str) -> Option<Self> {
    serde_json::from_str(json).ok()
  }

  /// 同 [`Self::parse`]，解析失败按空册处理（不让一份坏存档拖垮整页）。
  #[must_use]
  pub fn from_json(json: &str) -> Self {
    Self::parse(json).unwrap_or_default()
  }

  /// 保存一份模型。**同名覆盖**（保留 id，挪到最前）—— 改了再存是最常见的操作，
  /// 每次都新增一条会让列表很快塞满「八木 / 八木 2 / 八木 改」。返回存档 id。
  pub fn save(&mut self, name: &str, model: SavedModel, saved_at: &str) -> Result<u64, SaveError> {
    let name: String = name.trim().chars().take(MAX_NAME_CHARS).collect();
    if name.is_empty() {
      return Err(SaveError::EmptyName);
    }
    if !model.is_usable() {
      return Err(SaveError::Unusable);
    }
    let id = if let Some(pos) = self.designs.iter().position(|d| d.name == name) {
      let old = self.designs.remove(pos);
      old.id
    } else {
      if self.designs.len() >= MAX_DESIGNS {
        return Err(SaveError::Full);
      }
      self.designs.iter().map(|d| d.id).max().unwrap_or(0) + 1
    };
    self.designs.insert(
      0,
      SavedDesign {
        id,
        name,
        saved_at: saved_at.to_owned(),
        model,
      },
    );
    Ok(id)
  }

  /// 按 id 取一份。
  #[must_use]
  pub fn get(&self, id: u64) -> Option<&SavedDesign> {
    self.designs.iter().find(|d| d.id == id)
  }

  /// 删除一份，返回是否删到了。
  pub fn remove(&mut self, id: u64) -> bool {
    let before = self.designs.len();
    self.designs.retain(|d| d.id != id);
    self.designs.len() != before
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn sample() -> (NecInput, Vec<TransmissionLine>) {
    let input = NecInput {
      wires: vec![
        Wire::new([-5.05, 0.0, 10.0], [5.05, 0.0, 10.0], 0.002, 21),
        Wire::new([-5.3, 3.1, 10.0], [5.3, 3.1, 10.0], 0.002, 21),
      ],
      // 0.4321 不在任何段中心：`.nec` 往返会把它吸附走，存档必须原样带回。
      feeds: vec![Feed {
        wire: 0,
        at: 0.4321,
        volts: 1.0,
      }],
      loads: vec![Load {
        wire: 1,
        at: 0.25,
        r_ohm: 1.5,
        l_uh: 12.0,
        c_pf: 0.0,
      }],
      freq_hz: 14.1e6,
      ground: Ground::Lossy {
        eps_r: 13.0,
        sigma: 0.005,
      },
      conductivity: Some(3.5e7),
    };
    let lines = vec![TransmissionLine {
      wire_a: 0,
      at_a: 1.0,
      wire_b: 1,
      at_b: 0.0,
      z0: 50.0,
      length_m: 5.3,
      velocity_factor: 0.66,
    }];
    (input, lines)
  }

  #[test]
  fn a_snapshot_round_trips_exactly_through_json() {
    let (input, lines) = sample();
    let model = SavedModel::from_input(&input, &lines);
    let json = serde_json::to_string(&model).unwrap();
    let back: SavedModel = serde_json::from_str(&json).unwrap();
    let (input2, lines2) = back.to_input();
    // 逐位相等：馈电相对位置、材质、传输线 —— 这三样正是 `.nec` 会丢的。
    assert_eq!(input2, input);
    assert_eq!(lines2, lines);
  }

  #[test]
  fn same_name_overwrites_and_moves_to_front() {
    let (input, lines) = sample();
    let m = SavedModel::from_input(&input, &lines);
    let mut book = DesignBook::default();
    let a = book.save("yagi", m.clone(), "2026-10-01").unwrap();
    let b = book.save("dipole", m.clone(), "2026-10-02").unwrap();
    assert_ne!(a, b);
    let again = book.save("  yagi  ", m, "2026-10-03").unwrap();
    // 同名覆盖保留原 id（名字两侧空白不算区别）。
    assert_eq!(again, a);
    assert_eq!(book.designs.len(), 2);
    assert_eq!(book.designs[0].name, "yagi");
    assert_eq!(book.designs[0].saved_at, "2026-10-03");
  }

  #[test]
  fn rejects_empty_names_unusable_models_and_overflow() {
    let (input, lines) = sample();
    let m = SavedModel::from_input(&input, &lines);
    let mut book = DesignBook::default();
    assert_eq!(book.save("   ", m.clone(), "d"), Err(SaveError::EmptyName));
    let mut broken = input.clone();
    broken.feeds.clear();
    assert_eq!(
      book.save("x", SavedModel::from_input(&broken, &[]), "d"),
      Err(SaveError::Unusable)
    );
    let mut nan = input;
    nan.wires[0].a[0] = f64::NAN;
    assert!(!SavedModel::from_input(&nan, &[]).is_usable());
    for i in 0..MAX_DESIGNS {
      book.save(&format!("d{i}"), m.clone(), "d").unwrap();
    }
    assert_eq!(book.save("one more", m.clone(), "d"), Err(SaveError::Full));
    // 满了也能同名覆盖。
    assert!(book.save("d3", m, "d").is_ok());
  }

  #[test]
  fn names_are_truncated_and_remove_works() {
    let (input, lines) = sample();
    let mut book = DesignBook::default();
    // 多字节字符：截断按字符数而不是字节数。
    let long = "ü".repeat(MAX_NAME_CHARS + 10);
    let id = book
      .save(&long, SavedModel::from_input(&input, &lines), "d")
      .unwrap();
    assert_eq!(book.get(id).unwrap().name.chars().count(), MAX_NAME_CHARS);
    assert!(book.remove(id));
    assert!(!book.remove(id));
    assert!(book.get(id).is_none());
  }

  #[test]
  fn usability_mirrors_the_solver_entry_checks() {
    let (input, lines) = sample();
    let ok = SavedModel::from_input(&input, &lines);
    assert!(ok.is_usable());
    // 下标越界：负载 / 传输线指向不存在的导线。
    let mut m = ok.clone();
    m.loads[0].wire = 9;
    assert!(!m.is_usable());
    let mut m = ok.clone();
    m.lines[0].wire_b = 9;
    assert!(!m.is_usable());
    // 非有限数 / 非法物理量。
    let mut m = ok.clone();
    m.loads[0].l_uh = f64::INFINITY;
    assert!(!m.is_usable());
    let mut m = ok.clone();
    m.lines[0].velocity_factor = 0.0;
    assert!(!m.is_usable());
    let mut m = ok.clone();
    m.conductivity = Some(-1.0);
    assert!(!m.is_usable());
    // 总段数超过求解器上限：求解器会直接返回 `None`。
    let mut m = ok;
    m.wires[0].segments = MAX_SEGMENTS;
    assert!(!m.is_usable());
  }

  #[test]
  fn parse_reports_failure_so_callers_can_keep_the_raw_text() {
    assert!(DesignBook::parse("not json").is_none());
    assert!(DesignBook::parse("{}").is_some());
  }

  #[test]
  fn bad_json_degrades_to_an_empty_book() {
    assert_eq!(DesignBook::from_json("not json"), DesignBook::default());
    assert_eq!(DesignBook::from_json("{}"), DesignBook::default());
    // 存档进备份的「其他」分类（不需要额外登记）。
    assert_eq!(
      crate::backup_merge::classify_key(DESIGNS_KEY),
      crate::backup_merge::BackupModule::Other
    );
  }
}
