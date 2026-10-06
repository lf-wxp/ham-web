//! 天线匹配与馈线：同轴电缆、平衡线、巴伦、天调与驻波比。

/// 常见馈线。
pub const FEEDLINES: &[(&str, &str, &str)] = &[
  (
    "RG-58",
    "50 Ω",
    "细同轴，损耗较高（14MHz 约 6 dB/100m），适合短距离与便携。",
  ),
  (
    "RG-8X",
    "50 Ω",
    "细同轴，损耗适中（14MHz 约 4 dB/100m），常用。",
  ),
  (
    "RG-213",
    "50 Ω",
    "粗同轴，损耗低（14MHz 约 3 dB/100m），基地台常用。",
  ),
  (
    "RG-174",
    "50 Ω",
    "极细同轴，损耗高（14MHz 约 11 dB/100m），仅用于短跳线。",
  ),
  (
    "LMR-400",
    "50 Ω",
    "低损耗粗同轴（14MHz 约 2 dB/100m），UHF/高频段表现好。",
  ),
  (
    "75 Ω 同轴（RG-59 / RG-6）",
    "75 Ω",
    "电视用，业余中用作接收与特殊匹配。速度因子随介质而异：实体聚乙烯 RG-59 约 0.66，泡沫介质的 RG-6 约 0.82。",
  ),
  (
    "平行双线 / 梯形线",
    "300/450 Ω",
    "平衡馈线，损耗极低，需配巴伦转换；450Ω 梯形线速度因子约 0.88–0.95。",
  ),
];

/// 馈线规格：特性阻抗、速度因子与参考损耗。
///
/// 损耗为厂家数据表的典型值（dB/100 m，良好接头、常温）。实际值随线材批次、
/// 弯曲、进水老化而变差，用作估算而非保证值。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FeedlineSpec {
  /// 名称。
  pub name: &'static str,
  /// 特性阻抗（Ω）。
  pub z0_ohm: f64,
  /// 速度因子（电磁波在缆中速度与真空光速之比），决定线缆中的波长缩短。
  pub velocity_factor: f64,
  /// 参考损耗点 `(MHz, dB/100 m)`，按频率升序。
  pub loss: &'static [(f64, f64)],
}

/// 常用馈线的速度因子与损耗数据。
pub const FEEDLINE_SPECS: &[FeedlineSpec] = &[
  FeedlineSpec {
    name: "RG-58",
    z0_ohm: 50.0,
    velocity_factor: 0.66,
    loss: &[
      (14.0, 6.2),
      (28.0, 8.9),
      (50.0, 11.8),
      (144.0, 20.7),
      (432.0, 38.1),
    ],
  },
  FeedlineSpec {
    name: "RG-8X",
    z0_ohm: 50.0,
    velocity_factor: 0.79,
    loss: &[
      (14.0, 4.3),
      (28.0, 5.9),
      (50.0, 7.9),
      (144.0, 14.4),
      (432.0, 26.9),
    ],
  },
  FeedlineSpec {
    name: "RG-213",
    z0_ohm: 50.0,
    velocity_factor: 0.66,
    loss: &[
      (14.0, 3.0),
      (28.0, 4.3),
      (50.0, 5.6),
      (144.0, 10.2),
      (432.0, 19.0),
    ],
  },
  FeedlineSpec {
    name: "RG-174",
    z0_ohm: 50.0,
    velocity_factor: 0.66,
    loss: &[(14.0, 11.2), (50.0, 18.4), (144.0, 34.4)],
  },
  FeedlineSpec {
    name: "LMR-400",
    z0_ohm: 50.0,
    velocity_factor: 0.85,
    loss: &[
      (14.0, 2.0),
      (28.0, 2.6),
      (50.0, 3.6),
      (144.0, 6.2),
      (432.0, 10.8),
    ],
  },
  FeedlineSpec {
    name: "450Ω 梯形线",
    z0_ohm: 450.0,
    velocity_factor: 0.91,
    loss: &[(14.0, 0.3), (28.0, 0.4), (50.0, 0.6), (144.0, 1.0)],
  },
];

/// 按名称查馈线规格。
#[must_use]
pub fn feedline_spec(name: &str) -> Option<&'static FeedlineSpec> {
  FEEDLINE_SPECS.iter().find(|s| s.name == name)
}

/// 估算馈线损耗（dB）。
///
/// 频率落在两个参考点之间时按 √f 规律插值（同轴电缆的介质与导体损耗近似正比于 √f）；
/// 超出参考表范围时用最近的端点按 √f 外推。长度按线性折算。
#[must_use]
pub fn feedline_loss_db(spec: &FeedlineSpec, freq_mhz: f64, length_m: f64) -> f64 {
  if freq_mhz <= 0.0 || length_m <= 0.0 {
    return 0.0;
  }
  let table = spec.loss;
  let per_100m = if freq_mhz <= table[0].0 {
    table[0].1 * (freq_mhz / table[0].0).sqrt()
  } else if freq_mhz >= table[table.len() - 1].0 {
    let last = table[table.len() - 1];
    last.1 * (freq_mhz / last.0).sqrt()
  } else {
    let i = table
      .windows(2)
      .position(|w| freq_mhz <= w[1].0)
      .unwrap_or(0);
    let (f1, l1) = table[i];
    let (f2, l2) = table[i + 1];
    // 在 (f1,l1) 与 (f2,l2) 之间按 √f 插值：l = k·√f，两端各自定标后线性过渡。
    let k1 = l1 / f1.sqrt();
    let k2 = l2 / f2.sqrt();
    let t = (freq_mhz.sqrt() - f1.sqrt()) / (f2.sqrt() - f1.sqrt());
    (k1 + (k2 - k1) * t) * freq_mhz.sqrt()
  };
  per_100m * length_m / 100.0
}

/// 线缆中的波长（米）：真空波长 × 速度因子。
///
/// 做 1/4 波长变换线、共模扼流圈或巴伦时必须用线缆内波长，而不是自由空间波长。
#[must_use]
pub fn wavelength_in_line_m(freq_mhz: f64, velocity_factor: f64) -> f64 {
  if freq_mhz <= 0.0 {
    return 0.0;
  }
  299.792_458 / freq_mhz * velocity_factor
}

/// 匹配器件与概念。
pub const MATCHING: &[(&str, &str)] = &[
  (
    "巴伦 Balun",
    "平衡-不平衡转换器，连接平衡天线（偶极）与不平衡馈线（同轴），兼作阻抗变换（1:1、4:1、9:1）。",
  ),
  (
    "天线调谐器",
    "把天线系统阻抗变换到发射机所需的 50Ω，使末级输出满功率并保护功放；但不提高天线辐射效率，也不减小馈线损耗。",
  ),
  (
    "阻抗匹配",
    "源、馈线、负载三者阻抗一致时传输功率最大、反射最小。",
  ),
  (
    "驻波比 VSWR",
    "反射波与入射波叠加形成的驻波最大值与最小值之比，理想为 1:1。",
  ),
  (
    "反射系数",
    "反射电压与入射电压之比，|Γ| = (VSWR-1)/(VSWR+1)。",
  ),
  (
    "VSWR ↔ 回波损耗",
    "1.5:1 → 14.0 dB；2:1 → 9.5 dB；3:1 → 6.0 dB（RL = 20·lg(1/|Γ|)）。",
  ),
  (
    "回波损耗",
    concat!(
      "入射功率与反射功率之比的分贝数，越大表示匹配越好。",
      "惯例取**正值** RL = −20·lg|Γ| = 20·lg(1/|Γ|)；若仪表给出负值，那是 S11（反射系数 dB）。",
    ),
  ),
];

/// 失配程度与后果对照。
///
/// `回波损耗` 与 `失配损耗` 是两回事：前者是反射功率比（越大越好），
/// 后者是因此损失掉的功率（越小越好）。2:1 的失配损耗只有约 0.5 dB，
/// 常被高估 —— 真正的问题在于馈线上来回反射会成倍放大馈线本身的损耗。
pub const MISMATCH: &[(&str, &str)] = &[
  ("VSWR 1.0:1", "完全匹配，功率全部传输。"),
  (
    "VSWR 1.5:1",
    "反射约 4%，回波损耗 14.0 dB，失配损耗约 0.18 dB，可接受。",
  ),
  (
    "VSWR 2.0:1",
    "反射约 11%，回波损耗 9.5 dB，失配损耗约 0.51 dB，多数电台可正常工作。",
  ),
  (
    "VSWR 3.0:1",
    "反射约 25%，回波损耗 6.0 dB，失配损耗约 1.25 dB，电台可能自动降功率。",
  ),
  (
    "VSWR > 3:1",
    "反射严重，需检查天线或馈线。现代电台会自动降功率保护，不会立即损毁，但效率低下的馈线损耗会被反复反射放大。",
  ),
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn feedline_data_populated() {
    assert!(FEEDLINES.len() >= 5);
    assert!(MATCHING.len() >= 5);
    assert!(MISMATCH.len() >= 4);
    for (name, z, desc) in FEEDLINES {
      assert!(!name.is_empty());
      assert!(!z.is_empty());
      assert!(!desc.is_empty());
    }
  }
}
