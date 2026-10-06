//! 滤波器与双工器：低通、带通、陷波器与双工器。

/// 器件类型。
pub const FILTER_TYPES: &[(&str, &str, &str)] = &[
  (
    "低通滤波器",
    "LPF（Low Pass Filter）",
    "只通过低频，抑制谐波，接在发射机输出端。",
  ),
  (
    "高通滤波器",
    "HPF（High Pass Filter）",
    "只通过高频，用于抑制低频干扰。",
  ),
  (
    "带通滤波器",
    "BPF（Band Pass Filter）",
    "只通过某频段，竞赛多机工作时隔离收发。",
  ),
  (
    "陷波器",
    "Notch Filter",
    "滤除特定频率的强干扰，如广播干扰。",
  ),
  ("双工器", "Duplexer", "让收发共用一根天线，中继台核心器件。"),
  (
    "晶体滤波器",
    "Crystal Filter",
    "窄带高选择性，用于收信机中频。",
  ),
];

/// 滤波器关键指标。
pub const FILTER_METRICS: &[(&str, &str)] = &[
  (
    "品质因数 Q",
    "Q = f₀ / BW，中心频率与 3dB 带宽之比；Q 越高选择性越强、带宽越窄。",
  ),
  (
    "矩形系数 Shape Factor",
    "60dB 带宽与 6dB 带宽之比，≥1 且越小越好（1 为理想矩形）。晶体滤波器典型 2:1–3:1，LC 滤波器常 >4:1。",
  ),
  (
    "插入损耗 Insertion Loss",
    "信号通过滤波器后的功率损耗（dB），越小越好；多级级联会累加。",
  ),
  (
    "带内纹波 Ripple",
    "通带内增益的起伏（dB），等纹波设计用它换取更陡的滚降。",
  ),
];

/// 四类经典滤波器响应对比。
pub const FILTER_RESPONSES: &[(&str, &str)] = &[
  (
    "Butterworth 巴特沃斯",
    "通带最平坦、无纹波，滚降较缓；适合对通带平坦度要求高的场合。",
  ),
  (
    "Chebyshev 切比雪夫",
    "通带内等纹波，换取更陡的滚降；纹波越大过渡带越窄。",
  ),
  (
    "Bessel 贝塞尔",
    "线性相位、群时延平坦，瞬态响应好；幅频滚降最缓，适合脉冲类信号。",
  ),
  (
    "Elliptic 椭圆（Cauer）",
    "滚降最陡，但通带与阻带都有纹波，且阻带零点有限。",
  ),
];

/// 选用要点。
pub const FILTER_TIPS: &[&str] = &[
  "发射低通滤波器按最高工作频段选择，抑制谐波干扰电视 / 电话。",
  "多机竞赛台用带通滤波器隔离，避免互相压制。",
  "中继台双工器需承受大功率且收发隔离度足够。",
  "接收端陷波器可压制本地强广播干扰，改善信噪比。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn filters_data_populated() {
    assert!(FILTER_TYPES.len() >= 5);
    assert!(!FILTER_TIPS.is_empty());
  }
}
