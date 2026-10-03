//! CQ / ITU 分区地图数据：分区范围、稳定配色，以及按分区聚合的 DXCC 实体。
//!
//! 分区几何（真实边界）不公开可得，且分区常常横跨国界；本模块据此采用
//! 「以 DXCC 实体的主分区着色」的近似做法 —— 每个实体取 [`crate::dxcc::Entity`]
//! 中的 `cq` / `itu`（多数实体所在分区），地图上以实体着色或中心点标记。
//! 这足以回答「某个分区大致在哪、包含哪些实体」，但不代表精确的分区边界。

use crate::dxcc::{Entity, entities};

/// CQ 分区总数（1–40）。
pub const CQ_ZONE_MAX: u8 = 40;
/// ITU 分区总数（1–90）。
pub const ITU_ZONE_MAX: u8 = 90;

/// 分区号 → 稳定的区分色（HSL 字符串）。
///
/// 同一分区号在任何地图与任何分区制式下都得到同一颜色；相邻分区交替明度，
/// 让色相接近的两块区域仍能区分。返回 `hsl(H S% L%)` 供 SVG `fill` 直接使用。
#[must_use]
pub fn zone_color(zone: u8, max: u8) -> String {
  let total = f64::from(max.max(1));
  let z = f64::from(zone.max(1));
  let hue = ((z - 1.0) * 360.0 / total).rem_euclid(360.0);
  let light = if zone.is_multiple_of(2) { 62 } else { 45 };
  format!("hsl({hue:.0} 68% {light}%)")
}

/// 某个 CQ 分区内的 DXCC 实体（按中文名排序）。
#[must_use]
pub fn entities_in_cq(zone: u8) -> Vec<&'static Entity> {
  sorted(entities().iter().filter(|e| e.cq == zone).collect())
}

/// 某个 ITU 分区内的 DXCC 实体（按中文名排序）。
#[must_use]
pub fn entities_in_itu(zone: u8) -> Vec<&'static Entity> {
  sorted(entities().iter().filter(|e| e.itu == zone).collect())
}

fn sorted(mut list: Vec<&'static Entity>) -> Vec<&'static Entity> {
  list.sort_by(|a, b| a.name.cmp(b.name));
  list
}

fn counts(max: u8, zone_of: fn(&Entity) -> u8) -> Vec<(u8, usize)> {
  (1..=max)
    .map(|z| (z, entities().iter().filter(|e| zone_of(e) == z).count()))
    .collect()
}

/// 每个 CQ 分区（1–40）所含的 DXCC 实体数，含为 0 的分区。
#[must_use]
pub fn cq_zone_counts() -> Vec<(u8, usize)> {
  counts(CQ_ZONE_MAX, |e| e.cq)
}

/// 每个 ITU 分区（1–90）所含的 DXCC 实体数，含为 0 的分区。
#[must_use]
pub fn itu_zone_counts() -> Vec<(u8, usize)> {
  counts(ITU_ZONE_MAX, |e| e.itu)
}

/// 概念说明。
pub const ZONE_CONCEPTS: &[(&str, &str)] = &[
  (
    "CQ 分区",
    "全世界划分为 40 个 CQ 分区，是 WAZ（通联全部 CQ 分区）奖状与竞赛交换信息的基础。",
  ),
  (
    "ITU 分区",
    "全世界划分为 90 个 ITU 分区，源自国际电信联盟的频率与呼号管理划分。",
  ),
  (
    "中国所在分区",
    "中国大陆主要位于 CQ 23 / 24 区、ITU 42–44 区；香港、澳门、台湾各有自己的分区。",
  ),
  (
    "与 DXCC 的区别",
    "DXCC 实体是奖状单位（政治 / 地理划分），分区是地理带的编号，二者并不一一对应。",
  ),
  (
    "地图的近似性",
    "本图按每个 DXCC 实体的主分区着色；俄罗斯、美国、中国等横跨多个分区的大国会被整体归入一个分区。",
  ),
];

/// 使用要点。
pub const ZONE_TIPS: &[&str] = &[
  "竞赛中常需交换 CQ 分区号，可在「呼号查询」页输入呼号直接读出。",
  "WAZ 奖状要求通联全部 40 个 CQ 分区；用本图对照日志查看缺口。",
  "跨分区的大国以主分区编号为准，边境操作时以执照与本地约定为准。",
  "ITU 分区常用于频率协调与官方文书，日常通联交换的通常是 CQ 分区。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn zone_colors_are_stable_and_varied() {
    assert_eq!(zone_color(1, CQ_ZONE_MAX), zone_color(1, CQ_ZONE_MAX));
    assert_ne!(zone_color(1, CQ_ZONE_MAX), zone_color(2, CQ_ZONE_MAX));
    // 相邻分区交替明度，颜色字符串必然不同。
    for z in 1..CQ_ZONE_MAX {
      assert_ne!(zone_color(z, CQ_ZONE_MAX), zone_color(z + 1, CQ_ZONE_MAX));
    }
    assert!(zone_color(1, ITU_ZONE_MAX).starts_with("hsl("));
  }

  #[test]
  fn counts_cover_every_entity_exactly_once() {
    let total = entities().len();
    for (label, list) in [("CQ", cq_zone_counts()), ("ITU", itu_zone_counts())] {
      assert_eq!(
        list.len(),
        usize::from(if label == "CQ" {
          CQ_ZONE_MAX
        } else {
          ITU_ZONE_MAX
        })
      );
      let sum: usize = list.iter().map(|(_, n)| n).sum();
      assert_eq!(sum, total, "{label} 分区计数之和应等于全部实体数");
    }
  }

  #[test]
  fn entities_in_zone_are_consistent() {
    // 以 dxcc 模块已有的断言为准：日本位于 CQ 25 / ITU 45 区。
    let (cq, itu) = crate::dxcc::entity_zones("日本").expect("日本分区");
    let by_cq = entities_in_cq(cq);
    let by_itu = entities_in_itu(itu);
    assert!(by_cq.iter().all(|e| e.cq == cq));
    assert!(by_itu.iter().all(|e| e.itu == itu));
    assert!(by_cq.iter().any(|e| e.dxcc == 339), "CQ {cq} 区应包含日本");
    assert!(
      by_itu.iter().any(|e| e.dxcc == 339),
      "ITU {itu} 区应包含日本"
    );
    // 排序稳定：中文名非降序。
    assert!(by_cq.windows(2).all(|w| w[0].name <= w[1].name));
  }
}
