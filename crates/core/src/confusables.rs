//! 易混淆概念辨析：考试中高频出现、最容易记混的概念组速查。
//!
//! 每个条目为一个「易混点」，包含若干易混概念（名称 + 说明）与一句「一句话区分」。

/// 一个易混概念组。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Confusable {
  /// 易混点标题，如「dB 家族」。
  pub title: &'static str,
  /// 常见困惑点（一句话描述考生常犯的错）。
  pub confusion: &'static str,
  /// 易混概念列表：(名称, 说明)。
  pub items: &'static [(&'static str, &'static str)],
  /// 一句话区分 / 记忆点。
  pub tip: &'static str,
}

/// 常见易混概念组。
pub const CONFUSABLES: &[Confusable] = &[
  Confusable {
    title: "dB 家族",
    confusion: "分不清哪个是「相对比值」、哪个是「绝对电平」、哪个是「天线增益」。",
    items: &[
      ("dB", "两个功率比值的对数单位，无量纲，只表示倍率关系。"),
      ("dBm", "以 1 mW 为基准的绝对功率电平，如 30 dBm = 1 W。"),
      ("dBi", "天线增益，相对理想点源（各向同性辐射体）。"),
      ("dBd", "天线增益，相对半波振子（偶极天线）。"),
    ],
    tip: "dBi = dBd + 2.15 dB；dBm 是绝对值，dB/dBi/dBd 是比值。",
  },
  Confusable {
    title: "调制方式与带宽",
    confusion: "AM / SSB / FM / CW 各自带宽与用途容易记串。",
    items: &[
      ("CW 等幅电报", "带宽最窄，约 100–300 Hz，弱信号能力强。"),
      ("SSB 单边带", "约 2.7 kHz，功率利用率约为 AM 的 4 倍。"),
      ("AM 调幅", "约 6 kHz，含载波和上下两个边带。"),
      ("FM 调频", "12.5 / 25 kHz，抗干扰强，用于 VHF/UHF 本地。"),
    ],
    tip: "带宽从小到大：CW < SSB < AM < FM。",
  },
  Confusable {
    title: "发射类别标识",
    confusion: "A1A、J3E、F3E 这类 ITU 发射类别记不住每位的含义。",
    items: &[
      (
        "第一位（调制方式）",
        "A 幅度、F 频率、G 相位、J 单边带抑制载波。",
      ),
      ("第二位（信号性质）", "1 单一数字信道、2 数字、3 单路模拟。"),
      ("第三位（信息类型）", "A 电报、B 数据传输、E 电话、F 电视。"),
    ],
    tip: "J3E = 单边带·单路模拟·电话；A1A = 幅度·数字·电报。",
  },
  Confusable {
    title: "频率与波长",
    confusion: "给频率算波长、给波长算频率时方向会搞反。",
    items: &[
      ("换算公式", "λ(m) = 300 ÷ f(MHz)。"),
      ("7 MHz", "对应 40 米波段。"),
      ("14 MHz", "对应 20 米波段。"),
      ("3.5 MHz", "对应 80 米波段。"),
    ],
    tip: "频率越高波长越短，二者成反比。",
  },
  Confusable {
    title: "边带选择惯例",
    confusion: "到底 10 MHz 以上用 USB 还是 LSB 总是记反。",
    items: &[
      ("10 MHz 以下", "如 80m、40m，惯例使用 LSB（下边带）。"),
      ("10 MHz 以上", "如 20m、15m、10m，惯例使用 USB（上边带）。"),
      ("VHF/UHF", "统一使用 USB。"),
    ],
    tip: "口诀：下（低频）用 LSB，上（高频）用 USB。",
  },
  Confusable {
    title: "操作证类别权限",
    confusion: "A / B / C 三类各自频段与功率上限容易记混。",
    items: &[
      ("A 类", "30–3000 MHz，发射功率 ≤ 25 W。"),
      ("B 类", "30 MHz 以下 <15 W 或 30 MHz 以上 ≤ 25 W。"),
      ("C 类", "30 MHz 以下 ≤ 1000 W 或 30 MHz 以上 ≤ 25 W。"),
    ],
    tip: "A 只在 VHF/UHF；B/C 可上 HF，C 功率上限最高。",
  },
  Confusable {
    title: "电阻 / 电容 / 电感串并联",
    confusion: "电容与电阻的串并联规律正好相反，最容易错。",
    items: &[
      ("电阻串联", "R = R₁ + R₂。"),
      ("电阻并联", "1/R = 1/R₁ + 1/R₂。"),
      ("电容并联", "C = C₁ + C₂（与电阻相反）。"),
      ("电感串联", "L = L₁ + L₂（与电阻相同）。"),
    ],
    tip: "电容串并联与电阻相反，电感与电阻相同。",
  },
  Confusable {
    title: "功率与 dB 换算",
    confusion: "功率翻倍对应多少 dB、10 倍对应多少 dB 记不牢。",
    items: &[
      ("功率 ×2", "约 +3 dB。"),
      ("功率 ×10", "约 +10 dB。"),
      ("功率 ÷2", "约 −3 dB。"),
    ],
    tip: "3 dB ≈ 2 倍，10 dB = 10 倍，20 dB = 100 倍。",
  },
  Confusable {
    title: "天线极化",
    confusion: "水平极化和垂直极化如何判断。",
    items: &[
      ("水平极化", "电场方向与地面平行（振子水平放置）。"),
      ("垂直极化", "电场方向与地面垂直（振子垂直放置）。"),
      ("极化损耗", "收发极化不一致时会产生显著损耗。"),
    ],
    tip: "看振子摆放方向：横着是水平极化，竖着是垂直极化。",
  },
  Confusable {
    title: "功率 / 电压 / 电流单位",
    confusion: "瓦特、毫瓦、dBm 与电压、电流之间的关系。",
    items: &[
      ("功率", "W（瓦），1 W = 1000 mW。"),
      ("电压", "V（伏），有效值。"),
      ("电流", "A（安），1 A = 1000 mA。"),
      ("dBm 与 mW", "0 dBm = 1 mW，30 dBm = 1 W。"),
    ],
    tip: "P = U × I；每 +10 dBm 功率 ×10。",
  },
];

/// 一道易混淆辨析判断题。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfusableQuizItem {
  /// 判断陈述。
  pub statement: &'static str,
  /// 该陈述是否正确。
  pub answer: bool,
  /// 解析（为什么对 / 错）。
  pub explain: &'static str,
  /// 关联的易混点标题（答错时用于回看辨析表）。
  pub topic: &'static str,
}

/// 辨析练习判断题（主动回忆，答错回看对应辨析表）。
pub const CONFUSABLE_QUIZ: &[ConfusableQuizItem] = &[
  ConfusableQuizItem {
    statement: "dBi 比 dBd 大 2.15 dB。",
    answer: true,
    explain: "dBi 以理想点源为基准，dBd 以半波振子为基准，两者相差 2.15 dB。",
    topic: "dB 家族",
  },
  ConfusableQuizItem {
    statement: "dBm 是一个无量纲的比值单位。",
    answer: false,
    explain: "dBm 是以 1 mW 为基准的绝对功率电平，不是比值。",
    topic: "dB 家族",
  },
  ConfusableQuizItem {
    statement: "0 dBm 等于 1 mW。",
    answer: true,
    explain: "dBm 以 1 mW 为 0 dB 基准，0 dBm = 1 mW。",
    topic: "dB 家族",
  },
  ConfusableQuizItem {
    statement: "SSB 的带宽比 AM 更宽。",
    answer: false,
    explain: "SSB 约 2.7 kHz，AM 约 6 kHz，SSB 更窄。",
    topic: "调制方式与带宽",
  },
  ConfusableQuizItem {
    statement: "FM 的带宽通常比 CW 宽。",
    answer: true,
    explain: "带宽从小到大：CW（100–300 Hz）< SSB < AM < FM（12.5/25 kHz）。",
    topic: "调制方式与带宽",
  },
  ConfusableQuizItem {
    statement: "发射类别 J3E 表示单边带电话。",
    answer: true,
    explain: "J = 单边带抑制载波，3 = 单路模拟，E = 电话。",
    topic: "发射类别标识",
  },
  ConfusableQuizItem {
    statement: "A1A 表示调频电话。",
    answer: false,
    explain: "A = 幅度调制，1 = 数字，A = 电报；A1A 是等幅电报（CW）。",
    topic: "发射类别标识",
  },
  ConfusableQuizItem {
    statement: "频率越高，波长越长。",
    answer: false,
    explain: "波长与频率成反比：λ(m) = 300 ÷ f(MHz)。",
    topic: "频率与波长",
  },
  ConfusableQuizItem {
    statement: "7 MHz 对应 40 米波段。",
    answer: true,
    explain: "300 ÷ 7 ≈ 43，即 40 米波段。",
    topic: "频率与波长",
  },
  ConfusableQuizItem {
    statement: "10 MHz 以上惯例使用 LSB（下边带）。",
    answer: false,
    explain: "10 MHz 以上惯例使用 USB（上边带），以下用 LSB。",
    topic: "边带选择惯例",
  },
  ConfusableQuizItem {
    statement: "40 米波段惯例使用 LSB。",
    answer: true,
    explain: "40m 在 10 MHz 以下，惯例使用 LSB（下边带）。",
    topic: "边带选择惯例",
  },
  ConfusableQuizItem {
    statement: "A 类操作证可以在 30 MHz 以下发射。",
    answer: false,
    explain: "A 类仅限 30–3000 MHz；B/C 类才可上 HF。",
    topic: "操作证类别权限",
  },
  ConfusableQuizItem {
    statement: "C 类操作证在 30 MHz 以下的功率上限是 1000 W。",
    answer: true,
    explain: "C 类：30 MHz 以下 ≤ 1000 W，30 MHz 以上 ≤ 25 W。",
    topic: "操作证类别权限",
  },
  ConfusableQuizItem {
    statement: "两个电容并联，总电容等于两者之和。",
    answer: true,
    explain: "电容并联相加（与电阻相反），串联则倒数相加。",
    topic: "电阻 / 电容 / 电感串并联",
  },
  ConfusableQuizItem {
    statement: "两个电阻并联，总电阻等于两者之和。",
    answer: false,
    explain: "电阻串联才是相加；并联总电阻小于任一支路。",
    topic: "电阻 / 电容 / 电感串并联",
  },
  ConfusableQuizItem {
    statement: "功率增加一倍，约等于 +3 dB。",
    answer: true,
    explain: "3 dB ≈ 2 倍功率，10 dB = 10 倍。",
    topic: "功率与 dB 换算",
  },
  ConfusableQuizItem {
    statement: "功率增加 10 倍，等于 +20 dB。",
    answer: false,
    explain: "10 倍功率是 +10 dB，100 倍才是 +20 dB。",
    topic: "功率与 dB 换算",
  },
  ConfusableQuizItem {
    statement: "振子水平放置时是垂直极化。",
    answer: false,
    explain: "振子横放是水平极化（电场水平），竖放才是垂直极化。",
    topic: "天线极化",
  },
  ConfusableQuizItem {
    statement: "30 dBm 等于 1 W。",
    answer: true,
    explain: "0 dBm = 1 mW，每 +10 dBm 功率 ×10，故 30 dBm = 1 W。",
    topic: "功率 / 电压 / 电流单位",
  },
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn confusables_populated() {
    assert!(CONFUSABLES.len() >= 8);
    for c in CONFUSABLES {
      assert!(!c.title.is_empty());
      assert!(!c.confusion.is_empty());
      assert!(!c.tip.is_empty());
      assert!(c.items.len() >= 2);
      for (name, desc) in c.items {
        assert!(!name.is_empty());
        assert!(!desc.is_empty());
      }
    }
  }

  #[test]
  fn quiz_populated_and_topics_exist() {
    assert!(CONFUSABLE_QUIZ.len() >= 15);
    let topics: std::collections::HashSet<&str> = CONFUSABLES.iter().map(|c| c.title).collect();
    for q in CONFUSABLE_QUIZ {
      assert!(!q.statement.is_empty());
      assert!(!q.explain.is_empty());
      assert!(topics.contains(q.topic), "未知易混点：{}", q.topic);
    }
  }
}
