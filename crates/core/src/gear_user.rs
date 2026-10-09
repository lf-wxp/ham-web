//! 用户自己录入的实测值：只存本地，用来和原表（Sherwood 表）的值对照着看。
//!
//! # 为什么只收这四项
//!
//! 一个接收机数字有没有意义，取决于**在什么条件下测的**。能在家用普通仪表量出来的只有
//! 这么几项，每项的条件都写死在字段名或文案里：
//!
//! | 字段 | 条件 | 家里怎么量 |
//! | --- | --- | --- |
//! | `current_a` | 接收（或标称发射）时的整机电流 | 带电流表的电源 |
//! | `noise_floor_dbm` | 输入端接匹配负载 | 已知刻度的 S 表 / 信号源 |
//! | `rmdr_db` | **2 kHz** 间隔 | 两台信号源（或一台双音源） |
//! | `phase_noise_10k_dbc` | **10 kHz** 间隔 | 频谱仪 |
//!
//! 相位噪声尤其不能含糊：同一台机器在 2 kHz 与 50 kHz 间隔上能差 20 dB 以上，间隔不写清楚
//! 就没法比 —— 原表的 LO Noise 列带 10 / 50 kHz 两个间隔，本仓连它都没收录，正是这个原因。
//!
//! # 能不能画到雷达图上
//!
//! 只有**本库/原表里有可比基准**的字段才上雷达图（噪声底、RMDR）：其余两项只作个人记录，
//! 界面上照实说明「没有可比基准」，而不是硬凑一条刻度把它们塞进去比。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// 相位噪声的测量间隔（kHz）。只有一个取值，写死在字段名里 —— 不给人选错的机会。
pub const PHASE_NOISE_SPACING_KHZ: f64 = 10.0;

/// 一台机器的用户实测值（都可以缺）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UserMeasurement {
  /// 工作电流（A）。
  #[serde(default)]
  pub current_a: Option<f64>,
  /// 接收机噪声底（dBm，输入端接匹配负载）。
  #[serde(default)]
  pub noise_floor_dbm: Option<f64>,
  /// 2 kHz 间隔的 RMDR / 三阶互调动态范围（dB）。
  #[serde(default)]
  pub rmdr_db: Option<f64>,
  /// 相位噪声（dBc/Hz，**10 kHz** 间隔）。
  #[serde(default)]
  pub phase_noise_10k_dbc: Option<f64>,
  /// 录入日期（`YYYY-MM-DD`；空 = 老数据或没记）。
  #[serde(default)]
  pub measured_on: String,
}

impl UserMeasurement {
  /// 一条数值都没填（连同日期）。空记录应当从存储里删掉，而不是留个壳。
  #[must_use]
  pub fn is_blank(&self) -> bool {
    UserField::ALL.iter().all(|f| f.get(self).is_none())
  }

  /// 这台机器上**有基准、能上雷达图**的项有几条（界面用它决定要不要画虚线）。
  #[must_use]
  pub fn plottable_count(&self) -> usize {
    UserField::ALL
      .iter()
      .filter(|f| f.has_baseline && f.get(self).is_some())
      .count()
  }
}

/// 可录入的一项。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UserField {
  /// 稳定 key（存储与界面的字段名）。
  pub key: &'static str,
  /// 单位（写死，不翻译：`dBm` / `dBc/Hz` 是专有单位）。
  pub unit: &'static str,
  /// 合理的输入区间（越界不采信 —— 界面据此给提示，而不是存下一个明显错的值）。
  pub min: f64,
  /// 区间上界。
  pub max: f64,
  /// 步进（输入框的加减按钮用）。
  pub step: f64,
  /// 本库 / 原表里有没有可比的基准值（没有就不上雷达图）。
  pub has_baseline: bool,
}

impl UserField {
  /// 全部可录入项（界面按此顺序展示）。
  pub const ALL: [Self; 4] = [
    Self {
      key: "current",
      unit: "A",
      min: 0.1,
      max: 40.0,
      step: 0.1,
      has_baseline: false,
    },
    Self {
      key: "noise-floor",
      unit: "dBm",
      min: -150.0,
      max: -80.0,
      step: 0.5,
      has_baseline: true,
    },
    Self {
      key: "rmdr",
      unit: "dB",
      min: 40.0,
      max: 130.0,
      step: 1.0,
      has_baseline: true,
    },
    Self {
      key: "phase-noise",
      unit: "dBc/Hz",
      min: -160.0,
      max: -80.0,
      step: 1.0,
      has_baseline: false,
    },
  ];

  /// 由稳定 key 取字段。
  #[must_use]
  pub fn from_key(key: &str) -> Option<Self> {
    Self::ALL.into_iter().find(|f| f.key == key)
  }

  /// 读这一项在某条记录上的值。
  #[must_use]
  pub fn get(self, m: &UserMeasurement) -> Option<f64> {
    match self.key {
      "current" => m.current_a,
      "noise-floor" => m.noise_floor_dbm,
      "rmdr" => m.rmdr_db,
      "phase-noise" => m.phase_noise_10k_dbc,
      _ => None,
    }
  }

  /// 写这一项（`None` = 清空）。
  pub fn set(self, m: &mut UserMeasurement, value: Option<f64>) {
    match self.key {
      "current" => m.current_a = value,
      "noise-floor" => m.noise_floor_dbm = value,
      "rmdr" => m.rmdr_db = value,
      "phase-noise" => m.phase_noise_10k_dbc = value,
      _ => {}
    }
  }

  /// 这个输入值能不能采信：有限、且在区间内。
  #[must_use]
  pub fn accepts(self, value: f64) -> bool {
    value.is_finite() && value >= self.min && value <= self.max
  }

  /// 输入框里回显的文本（空 = 未填）。
  #[must_use]
  pub fn text(self, m: &UserMeasurement) -> String {
    self.get(m).map_or_else(String::new, |v| {
      // 电流保留一位小数，其余是整数级的量。
      if self.step < 1.0 {
        format!("{v:.1}")
      } else {
        format!("{v:.0}")
      }
    })
  }
}

/// 按机型 id 存的一叠用户实测值。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserMeasurements(BTreeMap<String, UserMeasurement>);

impl UserMeasurements {
  /// 某一台机器的实测值。
  #[must_use]
  pub fn get(&self, gear_id: &str) -> Option<&UserMeasurement> {
    self.0.get(gear_id)
  }

  /// 是否一台都没有。
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.0.is_empty()
  }

  /// 存一项；`None` 或越界的值不采信。
  ///
  /// 写入后如果这台机器一条数值都不剩（也没记日期），整条记录就删掉 ——
  /// 否则存储里会堆一堆空壳，导出备份时也难看。
  pub fn set_field(&mut self, gear_id: &str, field: UserField, value: Option<f64>) -> bool {
    if let Some(v) = value
      && !field.accepts(v)
    {
      return false;
    }
    let entry = self.0.entry(gear_id.to_owned()).or_default();
    field.set(entry, value);
    if entry.is_blank() {
      self.0.remove(gear_id);
    }
    true
  }

  /// 记下录入日期（保存时由界面传入当天日期）。
  pub fn stamp(&mut self, gear_id: &str, date: &str) {
    if let Some(m) = self.0.get_mut(gear_id)
      && !m.is_blank()
    {
      m.measured_on = date.to_owned();
    }
  }

  /// 全部记录（按机型 id）。
  pub fn iter(&self) -> impl Iterator<Item = (&String, &UserMeasurement)> {
    self.0.iter()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn measurement() -> UserMeasurement {
    UserMeasurement {
      noise_floor_dbm: Some(-126.5),
      rmdr_db: Some(98.0),
      measured_on: "2026-10-08".to_owned(),
      ..Default::default()
    }
  }

  #[test]
  fn fields_round_trip_and_report_what_they_can_plot() {
    for f in UserField::ALL {
      assert_eq!(UserField::from_key(f.key), Some(f));
      assert!(f.accepts(f.min) && f.accepts(f.max));
    }
    assert_eq!(UserField::from_key("nope"), None);

    let m = measurement();
    assert_eq!(
      UserField::from_key("noise-floor").unwrap().get(&m),
      Some(-126.5)
    );
    assert_eq!(UserField::from_key("rmdr").unwrap().get(&m), Some(98.0));
    assert_eq!(UserField::from_key("current").unwrap().get(&m), None);
    // 只有噪声底与 RMDR 有可比基准；另外两项不上雷达图。
    assert_eq!(m.plottable_count(), 2);
    let only_current = UserMeasurement {
      current_a: Some(20.0),
      ..Default::default()
    };
    assert_eq!(only_current.plottable_count(), 0);
    assert!(!only_current.is_blank());
    assert!(UserMeasurement::default().is_blank());
  }

  #[test]
  fn values_outside_the_sane_range_are_not_accepted() {
    let rmdr = UserField::from_key("rmdr").unwrap();
    assert!(rmdr.accepts(70.0));
    assert!(!rmdr.accepts(200.0), "200 dB 的动态范围不存在");
    assert!(!rmdr.accepts(f64::NAN));
    assert!(!rmdr.accepts(f64::INFINITY));
    let current = UserField::from_key("current").unwrap();
    assert!(!current.accepts(0.0), "整机电流不会是 0");
    assert!(!current.accepts(-3.0));

    // 存储层也拦住越界值：不采信 ≠ 存下来再骂用户。
    let mut all = UserMeasurements::default();
    assert!(!all.set_field("ft-710", rmdr, Some(999.0)));
    assert!(all.get("ft-710").is_none());
    assert!(all.set_field("ft-710", rmdr, Some(98.0)));
    assert_eq!(all.get("ft-710").unwrap().rmdr_db, Some(98.0));
  }

  #[test]
  fn clearing_the_last_value_removes_the_record() {
    let mut all = UserMeasurements::default();
    let current = UserField::from_key("current").unwrap();
    all.set_field("ft-710", current, Some(22.0));
    assert!(!all.is_empty());
    all.set_field("ft-710", current, None);
    assert!(all.is_empty(), "一条数值都不剩时不该留空壳");
  }

  #[test]
  fn stamp_only_records_a_date_for_real_measurements() {
    let mut all = UserMeasurements::default();
    let current = UserField::from_key("current").unwrap();
    all.stamp("ft-710", "2026-10-08");
    assert!(all.is_empty(), "没有实测值就不该凭空造一条记录");
    all.set_field("ft-710", current, Some(22.0));
    all.stamp("ft-710", "2026-10-08");
    assert_eq!(all.get("ft-710").unwrap().measured_on, "2026-10-08");
  }

  #[test]
  fn it_serializes_as_a_plain_map_and_survives_old_data() {
    let mut all = UserMeasurements::default();
    let rmdr = UserField::from_key("rmdr").unwrap();
    all.set_field("ic-7300", rmdr, Some(94.0));
    let json = serde_json::to_string(&all).expect("序列化");
    assert!(json.contains("\"ic-7300\""), "{json}");
    let back: UserMeasurements = serde_json::from_str(&json).expect("反序列化");
    assert_eq!(back, all);
    // 缺字段（老版本或手工编辑过的）也要能读。
    let sparse: UserMeasurements =
      serde_json::from_str("{\"g90\":{\"rmdr_db\":76.0}}").expect("缺字段也能读");
    assert_eq!(sparse.get("g90").unwrap().rmdr_db, Some(76.0));
    assert_eq!(sparse.iter().count(), 1);
  }

  #[test]
  fn text_echoes_values_with_the_right_precision() {
    let m = measurement();
    assert_eq!(UserField::from_key("rmdr").unwrap().text(&m), "98");
    // 噪声底的步进是 0.5 dB，所以回显保留一位小数。
    assert_eq!(
      UserField::from_key("noise-floor").unwrap().text(&m),
      "-126.5"
    );
    let with_current = UserMeasurement {
      current_a: Some(21.5),
      ..m.clone()
    };
    assert_eq!(
      UserField::from_key("current").unwrap().text(&with_current),
      "21.5"
    );
    assert_eq!(
      UserField::from_key("current").unwrap().text(&m),
      "",
      "没填就是空串"
    );
  }

  #[test]
  fn every_field_round_trips_through_set_and_get() {
    // `get` / `set` 按字符串 key 分派，兜底是 `_ => {}`：key 打错一个字就会静默失效，
    // 所以逐个字段钉一遍「写进去能读回来、清空能清掉」。
    for f in UserField::ALL {
      let mut m = UserMeasurement::default();
      assert_eq!(f.get(&m), None, "{} 初始应为空", f.key);
      f.set(&mut m, Some(f.min));
      assert_eq!(f.get(&m), Some(f.min), "{} set 后读不回来", f.key);
      f.set(&mut m, None);
      assert_eq!(f.get(&m), None, "{} 清空失败", f.key);
      assert!(m.is_blank(), "{} 清空后记录应为空", f.key);
    }
    // 每个字段只写自己那一格，不串台。
    let mut m = UserMeasurement::default();
    for (i, f) in UserField::ALL.iter().enumerate() {
      f.set(&mut m, Some(i as f64));
    }
    for (i, f) in UserField::ALL.iter().enumerate() {
      assert_eq!(f.get(&m), Some(i as f64), "{} 被别的字段覆盖了", f.key);
    }
    // 未知 key（手工编辑过存储、或字段改名的残留）既不 panic 也不该写进任何一格。
    let ghost = UserField {
      key: "ghost",
      unit: "x",
      min: 0.0,
      max: 1.0,
      step: 1.0,
      has_baseline: false,
    };
    let mut m = UserMeasurement::default();
    ghost.set(&mut m, Some(0.5));
    assert!(m.is_blank(), "未知 key 不该写进记录");
    assert_eq!(ghost.get(&m), None);
  }
}
