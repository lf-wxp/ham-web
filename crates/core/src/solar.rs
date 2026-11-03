//! 太阳活动与短波传播科普：太阳活动指数及其对 HF 传播的影响。

/// 太阳活动指数（名称、取值范围、含义）。
pub const SOLAR_INDICES: &[(&str, &str, &str)] = &[
  (
    "SFI 太阳通量",
    "10.7cm 射电流量，取值范围约 50–300（低于 50 几乎不出现，300 为可能上限）",
    "数值越高，HF 高波段传播越好。",
  ),
  (
    "SSN 太阳黑子数",
    "太阳黑子相对数",
    "黑子越多，电离层电离越强、MUF 越高。",
  ),
  (
    "A 指数",
    "地磁活动日指数，0–400",
    "数值越高，传播条件越差（磁暴）。",
  ),
  (
    "K 指数",
    "地磁活动 3 小时指数，0–9",
    "K≥5 为磁暴，HF 传播明显恶化。",
  ),
  (
    "X 射线耀斑",
    "A/B/C/M/X 五个等级",
    "M/X 级会引发短波中断（黑子活动强时）。",
  ),
];

/// 传播条件分级。
pub const CONDITIONS: &[(&str, &str)] = &[
  ("优秀", "SFI 高、A/K 低，HF 各波段普遍开放。"),
  ("良好", "SFI 中等，HF 中低波段稳定可用。"),
  ("一般", "SFI 偏低，仅较低波段可用。"),
  ("差", "磁暴或耀斑爆发，短波中断。"),
];

/// 太阳活动周期科普要点。
pub const CYCLE_NOTES: &[&str] = &[
  "太阳活动约 11 年一个周期，峰值年份 HF 高波段（10m/12m/15m）传播最佳。",
  "低谷年份则依赖较低波段（40m/80m/160m）进行远距离通信。",
  "地磁暴会使极区吸收增强、MUF 下降，同时可能带来极光散射通信机会。",
];

/// 传播条件等级。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropagationLevel {
  /// 优秀。
  Excellent,
  /// 良好。
  Good,
  /// 一般。
  Fair,
  /// 差。
  Poor,
}

impl PropagationLevel {
  /// 中文标签。
  #[must_use]
  pub const fn label(self) -> &'static str {
    match self {
      Self::Excellent => "优秀",
      Self::Good => "良好",
      Self::Fair => "一般",
      Self::Poor => "差",
    }
  }
}

/// 首页实时传播卡用的「可能缺数据」版等级：任何**决定结论**的指数缺失就返回 `None`。
///
/// 为什么不直接把缺失当 0 喂给 [`propagation_level`]：接口里 SFI / SSN 都是 `Option`，
/// 缺失当 0 会在 K 正常时判成「一般」—— 编造了一个没有数据支撑的等级，与「不编造等级」的口径矛盾。
/// 只有结论真的依赖它时才要求它存在：K≥4 时等级只看 K；SFI≥130 时「优秀 / 良好」才取决于 SSN。
#[must_use]
pub fn live_level(kp: Option<i32>, ssn: Option<i32>, sfi: Option<i32>) -> Option<PropagationLevel> {
  let kp = f64::from(kp?);
  if kp >= 4.0 {
    return Some(propagation_level(kp, 0.0, 0.0));
  }
  let f107 = f64::from(sfi?);
  if f107 >= 130.0 {
    ssn?;
  }
  Some(propagation_level(kp, ssn.map_or(0.0, f64::from), f107))
}

/// 一条波段条件（HamQSL 的 `time` 为 `day` / `night`）。
///
/// 三个字段都给 `default`：接口里只要有一行缺词（`condition` 为空、`time` 拼错），
/// 严格反序列化会让**整张传播卡**进失败态 —— 一行脏数据不该换掉另外十几行读数。
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
pub struct BandCondition {
  #[serde(default)]
  pub band: String,
  #[serde(default)]
  pub time: String,
  #[serde(default)]
  pub condition: String,
}

/// 从全量条件里挑当前时段的那一批；时段字段缺失（全挑不出来）时退回全部。
#[must_use]
pub fn select_bands(all: &[BandCondition], period: &str) -> Vec<BandCondition> {
  let rows: Vec<BandCondition> = all
    .iter()
    .filter(|c| !c.band.is_empty() && c.time == period)
    .cloned()
    .collect();
  if !rows.is_empty() {
    return rows;
  }
  all.iter().filter(|c| !c.band.is_empty()).cloned().collect()
}

/// HamQSL 的条件值 → 界面词条 key；不认识的值为 `None`（显示破折号，不编造等级）。
#[must_use]
pub fn condition_label_key(condition: &str) -> Option<&'static str> {
  match condition {
    "Good" => Some("home.good"),
    "Fair" => Some("home.fair"),
    "Poor" => Some("home.poor"),
    _ => None,
  }
}

/// 条件值 → 色条档位（颜色由界面层的 `[data-tone]` 决定）。
#[must_use]
pub fn condition_tone(condition: &str) -> &'static str {
  match condition {
    "Good" => "good",
    "Fair" => "warn",
    "Poor" => "muted",
    _ => "unknown",
  }
}

/// 根据 Kp 指数、太阳黑子数与太阳通量判断传播条件。
///
/// 规则：Kp≥5 视为磁暴（差）；Kp≥4 视为地磁活跃（一般）；
/// 其余按太阳活动强弱分为优秀/良好/一般。
#[must_use]
pub fn propagation_level(kp: f64, ssn: f64, f107: f64) -> PropagationLevel {
  if kp >= 5.0 {
    PropagationLevel::Poor
  } else if kp >= 4.0 {
    PropagationLevel::Fair
  } else if f107 >= 130.0 && ssn >= 80.0 {
    PropagationLevel::Excellent
  } else if f107 >= 100.0 {
    PropagationLevel::Good
  } else {
    PropagationLevel::Fair
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn live_level_never_invents_a_level() {
    use PropagationLevel::*;
    // K 缺失：一律未知。
    assert_eq!(live_level(None, Some(100), Some(150)), None);
    // K≥4 时等级只看 K，SFI / SSN 缺失也能给出结论。
    assert_eq!(live_level(Some(5), None, None), Some(Poor));
    assert_eq!(live_level(Some(4), None, None), Some(Fair));
    // K 正常：SFI 缺失 = 未知（以前当 0 会判成「一般」，是编造的）。
    assert_eq!(live_level(Some(2), Some(90), None), None);
    // SFI≥130 时优秀 / 良好取决于 SSN：SSN 缺失 = 未知。
    assert_eq!(live_level(Some(2), None, Some(150)), None);
    assert_eq!(live_level(Some(2), Some(90), Some(150)), Some(Excellent));
    // SFI 在 100–129：只看 SFI，SSN 缺失不影响。
    assert_eq!(live_level(Some(2), None, Some(110)), Some(Good));
    assert_eq!(live_level(Some(2), None, Some(80)), Some(Fair));
  }

  #[test]
  fn band_selection_prefers_the_period_and_falls_back() {
    let c = |band: &str, time: &str| BandCondition {
      band: band.into(),
      time: time.into(),
      condition: "Good".into(),
    };
    let all = [
      c("20m", "day"),
      c("20m", "night"),
      c("", "day"),
      c("40m", "dusk"),
    ];
    let day = select_bands(&all, "day");
    assert_eq!(day.len(), 1);
    assert_eq!(day[0].time, "day");
    // 时段字段拼错 / 缺失：挑不出来时退回全部有波段名的行，而不是空表。
    let fallback = select_bands(&all, "noon");
    assert_eq!(fallback.len(), 3);
    assert!(fallback.iter().all(|r| !r.band.is_empty()));
    assert!(select_bands(&[], "day").is_empty());
  }

  #[test]
  fn condition_words_map_to_keys_and_tones() {
    assert_eq!(condition_label_key("Good"), Some("home.good"));
    assert_eq!(condition_label_key("Poor"), Some("home.poor"));
    assert_eq!(condition_label_key("Band Closed"), None);
    assert_eq!(condition_tone("Fair"), "warn");
    assert_eq!(condition_tone(""), "unknown");
    // 大小写敏感：HamQSL 的取值固定，宁可显示破折号也不猜。
    assert_eq!(condition_label_key("good"), None);
  }

  #[test]
  fn solar_data_populated() {
    assert!(!SOLAR_INDICES.is_empty());
    assert!(!CONDITIONS.is_empty());
    assert!(!CYCLE_NOTES.is_empty());
    for &(n, r, d) in SOLAR_INDICES {
      assert!(!n.is_empty() && !r.is_empty() && !d.is_empty());
    }
  }

  #[test]
  fn propagation_levels() {
    assert_eq!(propagation_level(6.0, 100.0, 150.0), PropagationLevel::Poor);
    assert_eq!(propagation_level(4.5, 100.0, 150.0), PropagationLevel::Fair);
    assert_eq!(
      propagation_level(2.0, 120.0, 140.0),
      PropagationLevel::Excellent
    );
    assert_eq!(propagation_level(2.0, 50.0, 110.0), PropagationLevel::Good);
    assert_eq!(propagation_level(2.0, 30.0, 80.0), PropagationLevel::Fair);
  }
}
