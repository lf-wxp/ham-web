//! 卫星过境提醒：收藏卫星、位置与提前量设置，以及「哪些过境该提醒了」的判定。
//!
//! 过境数据由服务端 `/api/passes` 计算（Celestrak TLE + SGP4），这里只做纯逻辑，便于测试。

use serde::{Deserialize, Serialize};

/// 提前量可选值（分钟）。
pub const LEAD_CHOICES: &[u32] = &[5, 10, 15, 30];
/// 已提醒记录保留时长（秒）：超过后清理，避免无限增长。
const NOTIFIED_KEEP_SECS: i64 = 86_400;

/// 一次过境（与 `/api/passes` 返回字段一致）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pass {
  pub name: String,
  #[serde(default)]
  pub norad: u64,
  pub aos: i64,
  pub los: i64,
  pub max_elev: f64,
  pub max_elev_time: i64,
  pub azimuth: f64,
}

impl Pass {
  /// 唯一键：同一卫星同一 AOS 只提醒一次。
  #[must_use]
  pub fn key(&self) -> String {
    format!("{}@{}", self.norad, self.aos)
  }
}

/// 收藏的卫星。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FavSat {
  pub norad: u64,
  pub name: String,
}

/// 过境提醒设置（持久化）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SatWatch {
  pub lat: f64,
  pub lon: f64,
  pub min_elev: f64,
  pub favorites: Vec<FavSat>,
  /// 是否开启浏览器通知提醒。
  pub alerts: bool,
  /// 提前多少分钟提醒。
  pub lead_min: u32,
  /// 是否在 NOAA APT 气象卫星过境前提醒录制（配合 `/apt-decoder`）。
  pub apt_alert: bool,
}

/// 已提醒过的过境（`Pass::key` 与 AOS），防止重复提醒；与设置分开存储，
/// 避免设置页保存时覆盖后台提醒的写入。
pub type Notified = Vec<(String, i64)>;

impl Default for SatWatch {
  fn default() -> Self {
    Self {
      lat: 39.9,
      lon: 116.4,
      min_elev: 10.0,
      favorites: Vec::new(),
      alerts: false,
      lead_min: 10,
      apt_alert: false,
    }
  }
}

impl SatWatch {
  #[must_use]
  pub fn is_favorite(&self, norad: u64) -> bool {
    self.favorites.iter().any(|f| f.norad == norad)
  }

  /// 切换收藏，返回切换后是否为收藏。
  pub fn toggle_favorite(&mut self, norad: u64, name: &str) -> bool {
    if let Some(i) = self.favorites.iter().position(|f| f.norad == norad) {
      self.favorites.remove(i);
      false
    } else {
      self.favorites.push(FavSat {
        norad,
        name: name.to_owned(),
      });
      true
    }
  }

  /// 收藏卫星中，`now` 时刻已进入提前量窗口、尚未开始且未提醒过的过境。
  #[must_use]
  pub fn due<'a>(&self, passes: &'a [Pass], notified: &Notified, now: i64) -> Vec<&'a Pass> {
    let lead = i64::from(self.lead_min) * 60;
    passes
      .iter()
      .filter(|p| self.is_favorite(p.norad))
      .filter(|p| p.aos > now && p.aos - lead <= now)
      .filter(|p| !notified.iter().any(|(k, _)| *k == p.key()))
      .collect()
  }
}

/// 记录已提醒，并清理过期记录。
pub fn mark_notified(notified: &mut Notified, passes: &[&Pass], now: i64) {
  notified.retain(|(_, aos)| now - aos < NOTIFIED_KEEP_SECS);
  for p in passes {
    notified.push((p.key(), p.aos));
  }
}

/// 收藏卫星的下一次（尚未结束的）过境，按 AOS 排序。
#[must_use]
pub fn upcoming<'a>(watch: &SatWatch, passes: &'a [Pass], now: i64) -> Vec<&'a Pass> {
  let mut v: Vec<&Pass> = passes
    .iter()
    .filter(|p| watch.is_favorite(p.norad) && p.los > now)
    .collect();
  v.sort_by_key(|p| p.aos);
  v
}

/// APT 气象卫星（NOAA 系列）NORAD 编号：NOAA-15 / NOAA-18 / NOAA-19。
pub const APT_SATS: &[u64] = &[25_338, 28_654, 33_591];

/// 某颗卫星是否为 APT 气象卫星（用于录制提醒）。
#[must_use]
pub fn is_apt(norad: u64) -> bool {
  APT_SATS.contains(&norad)
}

/// 方位角 → 八方位中文。
#[must_use]
pub fn compass(azimuth: f64) -> &'static str {
  const DIRS: [&str; 8] = ["北", "东北", "东", "东南", "南", "西南", "西", "西北"];
  if !azimuth.is_finite() {
    return "北";
  }
  let i = ((azimuth.rem_euclid(360.0) + 22.5) / 45.0) as usize % 8;
  DIRS[i]
}

#[cfg(test)]
mod tests {
  use super::*;

  fn pass(norad: u64, aos: i64) -> Pass {
    Pass {
      name: format!("SAT {norad}"),
      norad,
      aos,
      los: aos + 600,
      max_elev: 40.0,
      max_elev_time: aos + 300,
      azimuth: 90.0,
    }
  }

  #[test]
  fn due_respects_favorites_lead_and_dedupe() {
    let mut w = SatWatch {
      lead_min: 10,
      ..Default::default()
    };
    w.toggle_favorite(1, "ISS");
    let passes = vec![pass(1, 1_000), pass(1, 5_000), pass(2, 1_000)];
    // 提前 10 分钟窗口外
    let mut done = Notified::new();
    assert!(w.due(&passes, &done, 1_000 - 601).is_empty());
    // 窗口内：只有收藏的那颗
    let hits = w.due(&passes, &done, 1_000 - 300);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].norad, 1);
    // 已提醒过不再重复
    mark_notified(&mut done, &hits, 700);
    assert!(w.due(&passes, &done, 800).is_empty());
    // 已开始的过境不提醒
    assert!(w.due(&passes, &done, 1_100).is_empty());
  }

  #[test]
  fn notified_is_pruned() {
    let mut done = Notified::new();
    let p = pass(1, 0);
    mark_notified(&mut done, &[&p], 0);
    mark_notified(&mut done, &[], NOTIFIED_KEEP_SECS + 1);
    assert!(done.is_empty());
  }

  #[test]
  fn toggle_and_upcoming() {
    let mut w = SatWatch::default();
    assert!(w.toggle_favorite(7, "SO-50"));
    let passes = vec![pass(7, 3_000), pass(7, 1_000), pass(8, 500)];
    let up = upcoming(&w, &passes, 0);
    assert_eq!(
      up.iter().map(|p| p.aos).collect::<Vec<_>>(),
      vec![1_000, 3_000]
    );
    // 进行中的过境仍算「即将」
    assert_eq!(upcoming(&w, &passes, 1_200).len(), 2);
    assert!(!w.toggle_favorite(7, "SO-50"));
    assert!(upcoming(&w, &passes, 0).is_empty());
  }

  #[test]
  fn compass_points() {
    assert_eq!(compass(0.0), "北");
    assert_eq!(compass(359.0), "北");
    assert_eq!(compass(132.0), "东南");
    assert_eq!(compass(270.0), "西");
  }
}
