//! 常见业余天线型式速查：名称、缩写、增益参考、典型用途、简述与示意简图（SVG 内联内容）。

/// 一种天线型式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AntennaType {
  pub name: &'static str,
  pub abbr: &'static str,
  /// 增益参考（dBi，随结构/频率变化，仅作量级参考）。
  pub gain: &'static str,
  /// 典型用途。
  pub usage: &'static str,
  pub desc: &'static str,
  /// SVG 内联内容（不含外层 `<svg>`），viewBox 为 `0 0 100 60`。
  pub svg: &'static str,
}

#[allow(clippy::too_many_arguments)]
const fn antenna(
  name: &'static str,
  abbr: &'static str,
  gain: &'static str,
  usage: &'static str,
  desc: &'static str,
  svg: &'static str,
) -> AntennaType {
  AntennaType {
    name,
    abbr,
    gain,
    usage,
    desc,
    svg,
  }
}

/// 常见业余天线型式。
pub const ANTENNAS: &[AntennaType] = &[
  antenna(
    "半波偶极天线",
    "DP",
    "约 2.15 dBi",
    "HF 远距离基础天线",
    "总长约半个波长、从中间馈电的基础天线，输入阻抗约 73Ω。",
    r#"<line x1="10" y1="30" x2="46" y2="30"/><line x1="54" y1="30" x2="90" y2="30"/><line x1="50" y1="30" x2="50" y2="52"/><line x1="44" y1="52" x2="56" y2="52"/>"#,
  ),
  antenna(
    "地网天线",
    "GP",
    "约 2–5 dBi",
    "VHF/UHF 全向，基地台与中继",
    "四分之一波长垂直振子加若干地网，全向、低仰角，俗称垂直天线。",
    r#"<line x1="50" y1="6" x2="50" y2="48"/><line x1="50" y1="48" x2="30" y2="56"/><line x1="50" y1="48" x2="70" y2="56"/><line x1="50" y1="48" x2="50" y2="56"/>"#,
  ),
  antenna(
    "八木天线",
    "YAGI",
    "5–20 dBi",
    "DX 与定向通联",
    "有源振子加反射器与若干引向器组成的定向天线，增益高、前后比大。",
    r#"<line x1="18" y1="10" x2="18" y2="50"/><line x1="38" y1="10" x2="38" y2="50"/><line x1="54" y1="14" x2="54" y2="46"/><line x1="68" y1="18" x2="68" y2="42"/><line x1="82" y1="22" x2="82" y2="38"/><line x1="18" y1="30" x2="82" y2="30"/>"#,
  ),
  antenna(
    "鞭状天线",
    "Whip",
    "约 2 dBi",
    "手台 / 车台",
    "单根垂直导体，常见于手台和车台，便于携带与移动。",
    r#"<line x1="50" y1="6" x2="50" y2="48"/><line x1="42" y1="48" x2="58" y2="48"/><line x1="42" y1="54" x2="58" y2="54"/>"#,
  ),
  antenna(
    "环形天线",
    "Loop",
    "约 1–2 dBi",
    "测向、便携接收",
    "导线绕成环状，方向性明显，常用于测向。",
    r#"<circle cx="50" cy="30" r="20"/><line x1="50" y1="50" x2="50" y2="56"/><line x1="42" y1="56" x2="58" y2="56"/>"#,
  ),
  antenna(
    "磁环天线",
    "MagLoop",
    "约 -1~1 dBi（效率低）",
    "空间受限 / 阳台",
    "周长远小于波长的小型环天线，带宽很窄、需精细调谐。",
    r#"<circle cx="50" cy="30" r="10"/><line x1="50" y1="40" x2="50" y2="56"/><line x1="42" y1="56" x2="58" y2="56"/>"#,
  ),
  antenna(
    "对数周期天线",
    "LPDA",
    "5–12 dBi",
    "宽带监测、DX",
    "振子长度和间距按比例递增，宽频带定向天线。",
    r#"<line x1="15" y1="8" x2="15" y2="52"/><line x1="30" y1="14" x2="30" y2="46"/><line x1="45" y1="20" x2="45" y2="40"/><line x1="60" y1="24" x2="60" y2="36"/><line x1="75" y1="28" x2="75" y2="32"/><line x1="15" y1="30" x2="75" y2="30"/>"#,
  ),
  antenna(
    "抛物面天线",
    "Dish",
    "15–30 dBi+",
    "微波 / EME",
    "用抛物面反射器聚焦电波的高增益天线，用于微波和 EME。",
    r#"<path d="M 18 8 Q 55 58 82 8"/><circle cx="55" cy="24" r="3"/><line x1="55" y1="27" x2="55" y2="34"/>"#,
  ),
  antenna(
    "长线天线",
    "LongWire",
    "1–5 dBi",
    "野外简易架设",
    "长度为多个波长的单根导线天线，架设简单。",
    r#"<path d="M 10 14 L 90 46"/><line x1="10" y1="14" x2="10" y2="52"/>"#,
  ),
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn antennas_have_svg_and_meta() {
    assert!(!ANTENNAS.is_empty());
    for a in ANTENNAS {
      assert!(!a.name.is_empty());
      assert!(!a.svg.is_empty());
      assert!(!a.gain.is_empty());
      assert!(!a.usage.is_empty());
    }
  }
}
