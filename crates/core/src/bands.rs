//! 频谱波段划分表：各波段的波长/频率范围，以及其中的业余业务、卫星业余业务频段划分与脚注。
//!
//! 约定：频率范围均含上限、不含下限；C = λf = 3×10⁸ m/s。

/// 业余业务在该频段的使用状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Usage {
  /// 专用
  Exclusive,
  /// 唯一主要
  SolePrimary,
  /// 主要
  Primary,
  /// 次要
  Secondary,
}

impl Usage {
  pub const ALL: [Self; 4] = [
    Self::Exclusive,
    Self::SolePrimary,
    Self::Primary,
    Self::Secondary,
  ];

  #[must_use]
  pub const fn label(self) -> &'static str {
    match self {
      Self::Exclusive => "专用",
      Self::SolePrimary => "唯一主要",
      Self::Primary => "主要",
      Self::Secondary => "次要",
    }
  }
}

/// 备注中的一段：普通文本或脚注编号（见 [`FOOTNOTES`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Note {
  Text(&'static str),
  Ref(&'static str),
}

/// 备注单元格。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Remark {
  Empty,
  Notes(&'static [Note]),
  /// 与上一行的备注合并（表格中为纵向合并单元格）。
  MergedAbove,
}

/// 一条业余业务 / 卫星业余业务频段划分。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Allocation {
  pub range: &'static str,
  pub usage: Usage,
  pub remark: Remark,
}

impl Allocation {
  /// 该频段是否也供卫星业余业务使用。
  ///
  /// 由 [`is_satellite_range`] 依据 ITU 表内划分与脚注 5.282 的**唯一频率来源**
  /// [`AMATEUR_SATELLITE_RANGES_MHZ`] 派生，不再逐条手工标注，避免维护出错。
  #[must_use]
  pub fn is_satellite(&self) -> bool {
    is_satellite_range(self.range)
  }
}

/// 一个波段（带号）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Band {
  pub number: i8,
  pub name: &'static str,
  pub wavelength: &'static str,
  pub freq_name: &'static str,
  pub freq_abbr: &'static str,
  pub freq_range: &'static str,
  /// 属于「微波」范围（分米波 ~ 丝米波）。
  pub microwave: bool,
  pub allocations: &'static [Allocation],
}

impl Band {
  /// 表格中占用的行数（无划分时占 1 行）。
  #[must_use]
  pub fn rows(&self) -> usize {
    self.allocations.len().max(1)
  }

  /// 第 `i` 条划分的备注单元格需要纵向合并的行数；被上一行合并时返回 `None`。
  #[must_use]
  pub fn remark_span(&self, i: usize) -> Option<usize> {
    let list = self.allocations;
    if list.get(i)?.remark == Remark::MergedAbove {
      return None;
    }
    let merged = list[i + 1..]
      .iter()
      .take_while(|a| a.remark == Remark::MergedAbove)
      .count();
    Some(1 + merged)
  }

  /// 第 `i` 条划分实际对应的备注（合并单元格会向上查找）。
  #[must_use]
  pub fn remark_for(&self, i: usize) -> &'static [Note] {
    self.allocations[..=i.min(self.allocations.len().saturating_sub(1))]
      .iter()
      .rev()
      .find_map(|a| match a.remark {
        Remark::MergedAbove => None,
        Remark::Empty => Some(&[][..]),
        Remark::Notes(n) => Some(n),
      })
      .unwrap_or(&[])
  }
}

/// 脚注。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Footnote {
  pub code: &'static str,
  pub text: &'static str,
}

use Note::{Ref, Text};
use Usage::{Exclusive, Primary, Secondary, SolePrimary};

const fn a(range: &'static str, usage: Usage, remark: Remark) -> Allocation {
  Allocation {
    range,
    usage,
    remark,
  }
}

const fn n(notes: &'static [Note]) -> Remark {
  Remark::Notes(notes)
}

const E: Remark = Remark::Empty;
const MERGED: Remark = Remark::MergedAbove;

const fn band(
  number: i8,
  name: &'static str,
  wavelength: &'static str,
  (freq_name, freq_abbr): (&'static str, &'static str),
  freq_range: &'static str,
  allocations: &'static [Allocation],
) -> Band {
  Band {
    number,
    name,
    wavelength,
    freq_name,
    freq_abbr,
    freq_range,
    microwave: false,
    allocations,
  }
}

const fn microwave(b: Band) -> Band {
  Band {
    microwave: true,
    ..b
  }
}

pub const BANDS: &[Band] = &[
  band(
    -1,
    "至长波/千兆米波",
    "10000-1000Mm",
    ("至低频", "TLF"),
    "0.03-0.3Hz",
    &[],
  ),
  band(
    0,
    "至长波/百兆米波",
    "1000-100Mm",
    ("至低频", "TLF"),
    "0.3-3Hz",
    &[],
  ),
  band(1, "极长波", "100-10Mm", ("极低频", "ELF"), "3-30 Hz", &[]),
  band(2, "超长波", "10-1Mm", ("超低频", "SLF"), "30-300 Hz", &[]),
  band(
    3,
    "特长波",
    "1000-100km",
    ("特低频", "ULF"),
    "300-3000 Hz",
    &[],
  ),
  band(4, "甚长波", "100-10km", ("甚低频", "VLF"), "3-30 kHz", &[]),
  band(
    5,
    "长波",
    "10km-1km",
    ("低频", "LF"),
    "30-300 kHz",
    &[a(
      "135.7-137.8kHz",
      Secondary,
      n(&[Ref("5.64"), Ref("5.67A"), Ref("5.67B")]),
    )],
  ),
  band(
    6,
    "中波",
    "1000-100m",
    ("中频", "MF"),
    "300-3000 kHz",
    &[a(
      "1800-2000kHz",
      Primary,
      n(&[Ref("5.97"), Text("160米业余波段")]),
    )],
  ),
  band(
    7,
    "短波",
    "100-10m",
    ("高频", "HF"),
    "3-30 MHz",
    &[
      a(
        "3.5-3.9MHz",
        Primary,
        n(&[Ref("CHN4"), Text("80米业余波段")]),
      ),
      a(
        "5.3515-5.3665MHz",
        Secondary,
        n(&[Ref("5.133B"), Ref("CHN4")]),
      ),
      a("7-7.1MHz", Exclusive, n(&[Text("40米业余波段")])),
      a(
        "7.1-7.2MHz",
        Primary,
        n(&[Ref("5.141B"), Text("40米业余波段")]),
      ),
      a(
        "10.1-10.15MHz",
        Secondary,
        n(&[Ref("CHN4"), Text("WARC频段；不能用于通话")]),
      ),
      a("14-14.25MHz", Exclusive, n(&[Text("20米业余波段")])),
      a(
        "14.25-14.35MHz",
        Primary,
        n(&[Ref("5.152"), Ref("CHN4"), Text("20米业余波段")]),
      ),
      a(
        "18.068-18.168MHz",
        Primary,
        n(&[Ref("CHN4"), Text("WARC频段")]),
      ),
      a("21-21.45MHz", Exclusive, n(&[Text("15米业余波段")])),
      a(
        "24.89-24.99MHz",
        Primary,
        n(&[Ref("CHN4"), Text("WARC频段")]),
      ),
      a(
        "28-29.7MHz",
        Exclusive,
        n(&[Ref("CHN7"), Text("10米业余波段")]),
      ),
    ],
  ),
  band(
    8,
    "米波/超短波",
    "10-1m",
    ("甚高频", "VHF"),
    "30-300 MHz",
    &[
      a(
        "50-54MHz",
        Primary,
        n(&[Ref("5.162A"), Ref("CHN4"), Ref("CHN8"), Text("6米业余波段")]),
      ),
      a(
        "144-146MHz",
        SolePrimary,
        n(&[Ref("CHN42"), Text("2米业余波段")]),
      ),
      a("146-148MHz", Primary, MERGED),
    ],
  ),
  microwave(band(
    9,
    "分米波",
    "10-1dm",
    ("特高频", "UHF"),
    "300-3000 MHz",
    &[
      a(
        "430-440MHz",
        Secondary,
        n(&[Ref("5.282"), Text("0.7米业余波段")]),
      ),
      a("1240-1260MHz", Secondary, n(&[Ref("5.332")])),
      a("1260-1300MHz", Secondary, n(&[Ref("5.282"), Ref("5.335A")])),
      a(
        "2300-2450MHz",
        Secondary,
        n(&[Ref("5.150"), Ref("5.282"), Ref("CHN28")]),
      ),
    ],
  )),
  microwave(band(
    10,
    "厘米波",
    "10-1cm",
    ("超高频", "SHF"),
    "3-30 GHz",
    &[
      a("3.3-3.4GHz", Secondary, n(&[Ref("5.149"), Ref("CHN12")])),
      a("3.4-3.5GHz", Secondary, n(&[Ref("5.282"), Ref("CHN18")])),
      a("5.65-5.725GHz", Secondary, n(&[Ref("5.282")])),
      a("5.725-5.83GHz", Secondary, n(&[Ref("5.150")])),
      a(
        "5.83-5.85GHz",
        Secondary,
        n(&[Ref("5.150"), Text("空对地")]),
      ),
      a("10-10.4GHz", Secondary, n(&[Ref("5.479"), Ref("5.474D")])),
      a("10.4-10.45GHz", Secondary, E),
      a("10.45-10.5GHz", Secondary, E),
      a("24-24.05GHz", Primary, n(&[Ref("5.150")])),
      a("24.05-24.25GHz", Secondary, n(&[Ref("5.150")])),
    ],
  )),
  microwave(band(
    11,
    "毫米波",
    "10-1mm",
    ("极高频", "EHF"),
    "30-300 GHz",
    &[
      a("47-47.2GHz", Exclusive, E),
      a("76-77.5GHz", Secondary, n(&[Ref("5.149")])),
      a("77.5-78GHz", Primary, n(&[Ref("5.149")])),
      a("78-79GHz", Secondary, n(&[Ref("5.149"), Ref("5.560")])),
      a("79-81GHz", Secondary, n(&[Ref("5.149")])),
      a("122.25-123GHz", Secondary, n(&[Ref("5.138")])),
      a("134-136GHz", SolePrimary, E),
      a("136-141GHz", Secondary, n(&[Ref("5.149")])),
      a("241-248GHz", Secondary, n(&[Ref("5.138"), Ref("5.149")])),
      a("248-250GHz", SolePrimary, n(&[Ref("5.149"), Ref("CHN12")])),
    ],
  )),
  microwave(band(
    12,
    "丝米波/亚毫米波",
    "10-1dmm",
    ("至高频", "THF"),
    "300-3000 GHz",
    &[],
  )),
];

/// 脚注原文摘自《中华人民共和国无线电频率划分规定》（2023-07-01 施行）中国内地一栏。
pub const FOOTNOTES: &[Footnote] = &[
  Footnote {
    code: "5.64",
    text: "固定业务电台在划分给该业务的90kHz与160kHz（在1区为148.5kHz）之间频段内和水上移动业务电台在划分给该业务的110kHz与160kHz（在1区为148.5kHz）之间频段内，只准使用A1A或F1B、A2C、A3C、F1C或F3C类发射。水上移动业务电台在110kHz与160kHz（1区为148.5kHz）之间频段内，例外地也可准予使用J2B或J7B类发射。",
  },
  Footnote {
    code: "5.67A",
    text: "使用135.7-137.8kHz频段内频率的业余业务台站，其最大辐射功率不得超过1瓦（e.i.r.p.），且不应对在第5.67款所列国家内运行的无线电导航业务台站造成有害干扰。（WRC-07）",
  },
  Footnote {
    code: "5.67B",
    text: "在阿尔及利亚、埃及、伊拉克、黎巴嫩、阿拉伯叙利亚共和国、苏丹、南苏丹和突尼斯，135.7-137.8kHz频段的使用限于固定和水上移动业务。在上述国家，业余业务不得使用135.7-137.8kHz频段，授权此类使用的国家应将此考虑在内。（WRC-19）",
  },
  Footnote {
    code: "5.97",
    text: "在3区，罗兰系统工作在1850kHz或1950kHz上，其分别占用1825-1875kHz和1925-1975kHz频段。划分在1800-2000kHz频段内的其他业务，在不对工作在1850kHz和1950kHz的罗兰系统造成有害干扰的条件下，可以使用该频段内的任一频率。",
  },
  Footnote {
    code: "CHN4",
    text: "该频段可有限制地用于无线电定位业务，不得对其他业务产生有害干扰。（2001年）",
  },
  Footnote {
    code: "5.133B",
    text: "使用5351.5-5366.5kHz频段的业余业务电台的最大辐射功率不得超过15W（e.i.r.p.）。但是，在2区的墨西哥，使用5351.5-5366.5kHz频段的业余业务电台的最大辐射功率不得超过20W（e.i.r.p.）。在以下2区国家：安提瓜和巴布达、阿根廷、巴哈马、巴巴多斯、伯利兹、玻利维亚、巴西、智利、哥伦比亚、哥斯达黎加、古巴、多米尼加共和国、多米尼克、萨尔瓦多、厄瓜多尔、格林纳达、危地马拉、圭亚那、海地、洪都拉斯、牙买加、尼加拉瓜、巴拿马、巴拉圭、秘鲁、圣卢西亚、圣基茨和尼维斯、圣文森特和格林纳丁斯、苏里南、特立尼达和多巴哥、乌拉圭、委内瑞拉以及荷兰王国在2区的海外特别行政区和海外属地，使用5351.5-5366.5kHz频段的业余业务电台的最大辐射功率不得超过25W（e.i.r.p.）。（WRC-19）",
  },
  Footnote {
    code: "5.141B",
    text: "附加划分：在阿尔及利亚、沙特阿拉伯、澳大利亚、巴林、博茨瓦纳、文莱达鲁萨兰国、中国、科摩罗、韩国、迪戈加西亚岛、吉布提、埃及、阿拉伯联合国酋长国、厄立特里亚、几内亚、印度尼西亚、伊朗（伊斯兰共和国）、日本、约旦、科威特、利比亚、马里、摩洛哥、毛里塔尼亚、尼日尔、新西兰、阿曼、巴布亚新几内亚、卡塔尔、阿拉伯叙利亚共和国、朝鲜民主主义人民共和国、新加坡、苏丹、南苏丹、突尼斯、越南和也门，7100-7200kHz频段亦划分给作为主要业务的固定和除航空移动（R）以外的移动业务。（WRC-19）",
  },
  Footnote {
    code: "5.152",
    text: "附加划分：在亚美尼亚、阿塞拜疆、中国、科特迪瓦、俄罗斯、格鲁吉亚、伊朗、哈萨克斯坦、乌兹别克斯坦、吉尔吉斯斯坦、塔吉克斯坦、土库曼斯坦和乌克兰，14250-14350kHz频段以主要使用条件也划分给固定业务。固定业务电台的辐射功率不得超过24dBW。（WRC-03）",
  },
  Footnote {
    code: "CHN7",
    text: "31-35MHz频段可用于水上移动业务，为主要业务。其中33.0MHz可用于近海安全救助通信，其他业务不得对其产生有害干扰。27.5-29.7MHz频段内现有渔业电台可用至报废为止。29.7-39.5MHz频段内的其他频率可用于水上移动业务，在沿海各省、直辖市和自治区为主要业务，在其他地区为次要业务。（2010年）",
  },
  Footnote {
    code: "5.162A",
    text: "附加划分：在德国、奥地利、比利时、波斯尼亚和黑塞哥维那、中国、梵蒂冈、丹麦、西班牙、爱沙尼亚、俄罗斯联邦、芬兰、法国、爱尔兰、冰岛、意大利、拉托维亚、列支敦士登、立陶宛、卢森堡、北马其顿、摩纳哥、黑山、挪威、荷兰、波兰、葡萄牙、捷克共和国、英国、塞尔维亚、斯洛文尼亚、瑞典和瑞士，46-68MHz频段亦划分给作为次要业务的无线电定位业务。这项使用限定用于按照第217号决议（WRC-97）运行的风廓线雷达。（WRC-19）",
  },
  Footnote {
    code: "CHN8",
    text: "在不干扰广播业务条件下，48.5-72.5MHz、76-84MHz可用于固定、移动业务。（2001年）",
  },
  Footnote {
    code: "CHN42",
    text: "用于承担短期任务的空间操作业务（空对地）非对地静止系统使用137-148MHz频段前需征得相关部门同意。（2023年）",
  },
  Footnote {
    code: "5.282",
    text: "在435-438MHz，1260-1270MHz，2400-2450MHz，3400-3410MHz（仅限于2区和3区）和5650-5670MHz频段，卫星业余业务在对按频率划分表工作的其他业务不造成有害干扰的条件下可以使用（见5.43款）。主管部门在批准这种使用时，应确保一旦卫星业余业务电台的发射造成有害干扰时，立即根据25.11款的规定予以消除。卫星业余业务使用1260-1270MHz和5650-5670MHz频段仅限于地对空方向。",
  },
  Footnote {
    code: "5.332",
    text: "在1215-1260MHz频段，卫星地球探测业务和空间研究业务中的星载有源传感器不得对处于主要使用条件的无线电定位业务、卫星无线电导航业务和其他业务造成有害干扰。不能提出保护要求，或限制这些业务的操作或发展。（WRC-2000）",
  },
  Footnote {
    code: "5.335A",
    text: "在1260-1300MHz频段，卫星地球探测业务和空间研究业务中的星载有源传感器不得对脚注中处于主要使用条件的无线电定位业务和其他业务造成有害干扰，要求得到其保护，或限制其操作与发展。（WRC-2000）",
  },
  Footnote {
    code: "5.150",
    text: "下列频段：13553-13567kHz（中心频率为13560kHz），26957-27283kHz（中心频率为27120kHz），40.66-40.70MHz（中心频率为40.68MHz），902-928MHz（中心频率为915MHz）在2区，2400-2500MHz（中心频率为2450MHz），5725-5875MHz（中心频率为5800MHz），和24-24.25GHz（中心频率为24.125GHz），也指定给工业、科学和医疗（ISM）使用。在这些频段内工作的无线电通信业务必须承受由于这些应用可能产生的有害干扰。在这些频段内操作的ISM设备应遵守15.13款的规定。",
  },
  Footnote {
    code: "CHN28",
    text: "该频段引入的有关IMT应用的国际注脚，不改变移动业务在划分表中现有业务主次地位。同时，应尽快研究该频段已划分业务的应用模式、频率使用规划、业务间的兼容共存条件及协调程序。在此之前，IMT应用不投入实际部署使用，但在2300-2400MHz频段，IMT可在室内使用。（2010年）",
  },
  Footnote {
    code: "5.149",
    text: "在向已划分到下列频段的其它业务的电台进行指配时：13360-13410kHz，4950-4990MHz，102-109.5GHz，25550-25670kHz，4990-5000MHz，111.8-114.25GHz，37.5-38.25MHz，6650-6675.2MHz，128.33-128.59GHz，1区和3区的73-74.6MHz，10.6-10.68GHz，129.23-129.49GHz，1区的150.05-153MHz，14.47-14.5GHz，130-134GHz，322-328.6MHz，22.01-22.21GHz，136-148.5GHz，406.1-410MHz，22.21-22.5GHz，151.5-158.5GHz，1区和3区的608-614MHz，22.81-22.86GHz，168.59-168.93GHz，1330-1400MHz，23.07-23.12GHz，171.11-171.45GHz，1610.6-1613.8MHz，31.2-31.3GHz，172.31-172.65GHz，1660-1670MHz，1区和3区的31.5-31.8GHz，173.52-173.85GHz，1718.8-1722.2MHz，36.43-36.5GHz，195.75-196.15GHz，2655-2690MHz，42.5-43.5GHz，209-226GHz，3260-3267MHz，48.94-49.04GHz，241-250GHz，3332-3339MHz，76-86GHz，252-275GHz，3345.8-3352.5MHz，92-94GHz，4825-4835MHz，94.1-100GHz，敦促主管部门采用一切实际可行的措施保护射电天文业务免受有害干扰。星载电台或机载电台的发射对射电天文业务可能是特别严重的干扰源（见4.5和4.6款以及第29条）。（WRC-07）",
  },
  Footnote {
    code: "CHN12",
    text: "608-614MHz频段射电天文为主要业务，现用于北京密云区不老屯镇、新疆乌鲁木齐南山地区、贵州省黔南州、内蒙古正镶白旗陶林宝拉格嘎查；1330-1400MHz频段射电天文为主要业务，现用于北京怀柔区和密云区不老屯镇、上海佘山、云南昆明凤凰山、新疆乌鲁木齐南山地区、贵州省黔南州、内蒙古正镶白旗陶林宝拉格嘎查、新疆奇台县、新疆巴里坤县、云南普洱市景东县；1718.8-1722.2MHz频段射电天文为主要业务，现用于北京怀柔区和密云区不老屯镇，上海佘山、云南昆明凤凰山、新疆乌鲁木齐南山地区、贵州省黔南州、内蒙古正镶白旗陶林宝拉格嘎查、新疆奇台县、云南普洱市景东县；2655-2690MHz频段射电天文为主要业务，现用于北京怀柔区、江苏淮阴、贵州省黔南州、内蒙古正镶白旗陶林宝拉格嘎查、新疆奇台县、云南普洱市景东县；3260-3267MHz、3332-3339MHz、3345.8-3352.5MHz频段射电天文为主要业务，现用于贵州省黔南州、内蒙古正镶白旗陶林宝拉格嘎查、新疆奇台县、云南普洱市景东县；4825-4835MHz、4950-4990MHz、4990-5000MHz频段射电天文为主要业务，现用于新疆乌鲁木齐南山地区、贵州省黔南州、内蒙古正镶白旗陶林宝拉格嘎查、新疆奇台县、云南普洱市景东县；6650-6675.2MHz频段射电天文为主要业务，现用于北京怀柔区、江苏南京紫金山、新疆乌鲁木齐南山地区、贵州省黔南州、内蒙古正镶白旗陶林宝拉格嘎查、新疆奇台县、云南普洱市景东县；14.47-14.50GHz频段射电天文为主要业务，现用于北京密云区不老屯镇、内蒙古正镶白旗陶林宝拉格嘎、新疆奇台县；22.01-22.21GHz、22.81-22.86GHz、23.07-23.12GHz频段射电天文为主要业务，现用于青海德令哈市、上海佘山、新疆乌鲁木齐南山地区、北京密云区不老屯镇、新疆奇台县；248-250GHz频段射电天文为主要业务，现用于青海德令哈市、西藏拉萨市当雄市羊八井镇、新疆奇台县。其他业务台站不得对上述射电天文业务台站产生有害干扰。（2023年修订）",
  },
  Footnote {
    code: "CHN18",
    text: "现有无线电定位业务应尽早移出1535-1544MHz、1545-1645.5MHz、1645.5-1660MHz、1850-1880MHz、2085-2120MHz、3400-3800MHz、5925-6425MHz、7500-8185MHz、14-15.35GHz频段，从2005年底起不能启用新设备，但现有设备可用至设备报废为止。（2001年）",
  },
  Footnote {
    code: "5.474D",
    text: "卫星地球探测业务（有源）台站不得对9200-9300MHz频段的水上无线电导航和无线电定位业务台站、9900-10000MHz频段内的无线电导航和无线电定位业务台站以及10.0-10.4GHz频段内的无线电定位业务台站产生有害干扰，亦不得要求这些台站提供保护。（WRC-15）",
  },
  Footnote {
    code: "5.479",
    text: "9975-10025MHz频段以次要使用条件也划分给卫星气象业务，供气象雷达使用。",
  },
  Footnote {
    code: "5.560",
    text: "在卫星地球探测业务和空间研究业务中，空间站雷达可按主要使用条件在78-79GHz频段内工作。",
  },
  Footnote {
    code: "5.138",
    text: "下列频段：6765-6795kHz（中心频率为6780kHz），433.05-434.79MHz（中心频率为433.92MHz），除5.280款所列国家以外的1区61-61.5GHz（中心频率为61.25GHz），122-123GHz（中心频率为122.5GHz），和244-246GHz（中心频率为245GHz）。指定给工业、科学和医疗（ISM）使用，但须经有关部门与那些无线电通信业务可能受到影响的主管部门达成协议后给予特别批准。援用本规定时，主管部门应考虑有关的ITU-R最新建议书。",
  },
];

/// 全部业余业务频段划分条数。
#[must_use]
pub fn allocation_count() -> usize {
  BANDS.iter().map(|b| b.allocations.len()).sum()
}

/// 可供卫星业余业务使用的频段条数。
#[must_use]
pub fn satellite_count() -> usize {
  BANDS
    .iter()
    .flat_map(|b| b.allocations)
    .filter(|a| a.is_satellite())
    .count()
}

/// 解析划分条目里的频率范围为 MHz 区间（半开 `[低, 高)`）。
///
/// 支持 `kHz` / `MHz` / `GHz` 后缀，如 `135.7-137.8kHz`、`18.068-18.168MHz`、`47-47.2GHz`。
#[must_use]
pub fn parse_range_mhz(range: &str) -> Option<(f64, f64)> {
  let (body, scale) = if let Some(s) = range.strip_suffix("kHz") {
    (s, 1e-3)
  } else if let Some(s) = range.strip_suffix("MHz") {
    (s, 1.0)
  } else {
    (range.strip_suffix("GHz")?, 1e3)
  };
  let (lo, hi) = body.split_once('-')?;
  let lo = lo.trim().parse::<f64>().ok()? * scale;
  let hi = hi.trim().parse::<f64>().ok()? * scale;
  Some((lo, hi))
}

/// 卫星业余业务可用的频率范围（MHz，半开区间）——ITU 表内划分与脚注 5.282。
///
/// 这是「某频段能否用于卫星业余业务」的**唯一事实来源**：[`Allocation::is_satellite`]
/// 与 [`satellite_count`] 都由它派生，不再逐条手工标注。
pub const AMATEUR_SATELLITE_RANGES_MHZ: &[(f64, f64)] = &[
  // 短波（表内业余卫星业务划分）
  (7.0, 7.1),
  (14.0, 14.25),
  (18.068, 18.168),
  (21.0, 21.45),
  (24.89, 24.99),
  (28.0, 29.7),
  // 2m
  (144.0, 146.0),
  // 脚注 5.282：435-438 / 1260-1270 / 2400-2450 / 3400-3410 / 5650-5670
  (435.0, 438.0),
  (1260.0, 1270.0),
  (2400.0, 2450.0),
  (3400.0, 3410.0),
  (5650.0, 5670.0),
  // 厘米波 / 毫米波（表内划分）
  (5830.0, 5850.0),
  (10450.0, 10500.0),
  (24000.0, 24050.0),
  (47000.0, 47200.0),
  (76000.0, 81000.0),
  (134000.0, 141000.0),
  (241000.0, 250000.0),
];

/// 判断某划分频率范围是否与卫星业余业务频段重叠。
#[must_use]
pub fn is_satellite_range(range: &str) -> bool {
  parse_range_mhz(range).is_some_and(|r| {
    AMATEUR_SATELLITE_RANGES_MHZ
      .iter()
      .any(|&s| r.0 < s.1 && s.0 < r.1)
  })
}

/// 微波波段在表格中占用的总行数。
#[must_use]
pub fn microwave_rows() -> usize {
  BANDS.iter().filter(|b| b.microwave).map(Band::rows).sum()
}

/// 常用业余波段的数字边界（MHz，半开区间 `[下, 上)`），供
/// [`crate::frequencies::band_of`] 等复用。
///
/// 边界按中国（ITU 三区）口径：80m 到 3.9、40m 到 7.2、17m 从 18.068 起。
/// 这是这些边界在**全站的唯一事实来源**——`band_of` 与波段规划都据此判断，
/// 避免历史上「一处写二区值、另一处写三区值」造成的漂移（见
/// `knowledge_consistency::amateur_band_edges_are_single_source`）。
pub const AMATEUR_BAND_EDGES: &[(&str, f64, f64)] = &[
  ("160m", 1.8, 2.0),
  ("80m", 3.5, 3.9),
  ("60m", 5.3515, 5.3665),
  ("40m", 7.0, 7.2),
  ("30m", 10.1, 10.15),
  ("20m", 14.0, 14.35),
  ("17m", 18.068, 18.168),
  ("15m", 21.0, 21.45),
  ("12m", 24.89, 24.99),
  ("10m", 28.0, 29.7),
  ("6m", 50.0, 54.0),
  ("2m", 144.0, 148.0),
  ("70cm", 430.0, 440.0),
];

#[cfg(test)]
mod tests {
  use std::collections::HashSet;

  use super::*;

  fn refs() -> impl Iterator<Item = &'static str> {
    BANDS
      .iter()
      .flat_map(|b| b.allocations)
      .filter_map(|a| match a.remark {
        Remark::Notes(n) => Some(n),
        _ => None,
      })
      .flatten()
      .filter_map(|n| match n {
        Ref(code) => Some(*code),
        Text(_) => None,
      })
  }

  #[test]
  fn band_numbers_are_consecutive() {
    let nums: Vec<i8> = BANDS.iter().map(|b| b.number).collect();
    assert_eq!(nums, (-1..=12).collect::<Vec<_>>());
  }

  #[test]
  fn microwave_bands_are_contiguous() {
    let first = BANDS.iter().position(|b| b.microwave).unwrap();
    let last = BANDS.iter().rposition(|b| b.microwave).unwrap();
    assert!(BANDS[first..=last].iter().all(|b| b.microwave));
    assert_eq!(microwave_rows(), 4 + 10 + 10 + 1);
  }

  #[test]
  fn footnote_refs_resolve_and_all_used() {
    let codes: HashSet<_> = FOOTNOTES.iter().map(|f| f.code).collect();
    let used: HashSet<_> = refs().collect();
    assert_eq!(codes.len(), FOOTNOTES.len(), "duplicate footnote code");
    assert_eq!(used, codes);
  }

  #[test]
  fn merged_remarks_have_a_head() {
    for b in BANDS {
      if let Some(first) = b.allocations.first() {
        assert_ne!(first.remark, Remark::MergedAbove, "band {}", b.number);
      }
    }
  }

  #[test]
  fn remark_span_and_resolution() {
    let vhf = BANDS.iter().find(|b| b.number == 8).unwrap();
    assert_eq!(vhf.remark_span(0), Some(1));
    assert_eq!(vhf.remark_span(1), Some(2));
    assert_eq!(vhf.remark_span(2), None);
    assert_eq!(vhf.remark_for(2), &[Ref("CHN42"), Text("2米业余波段")]);
    let shf = BANDS.iter().find(|b| b.number == 10).unwrap();
    assert!(shf.remark_for(6).is_empty());
  }

  #[test]
  fn counts() {
    assert_eq!(allocation_count(), 1 + 1 + 11 + 3 + 4 + 10 + 10);
    // 按波段分组统计卫星业余业务划分：HF 6、VHF 1、UHF 3、SHF 5、EHF 9（合计 24）。
    // 该结果由 AMATEUR_SATELLITE_RANGES_MHZ 派生（含脚注 5.282 的 435-438 等）。
    assert_eq!(satellite_count(), 6 + 1 + 3 + 5 + 9);
  }

  #[test]
  fn satellite_derivation_matches_expected() {
    // 半开区间端点不误判：430-440 含 435-438，故为卫星业务。
    assert!(is_satellite_range("430-440MHz"));
    assert!(!is_satellite_range("1240-1260MHz"));
    assert!(is_satellite_range("1260-1300MHz"));
    assert!(!is_satellite_range("146-148MHz"));
    assert!(is_satellite_range("144-146MHz"));
    // 单位解析：GHz / kHz 都要正确折算到 MHz。
    assert_eq!(parse_range_mhz("47-47.2GHz"), Some((47000.0, 47200.0)));
    assert_eq!(parse_range_mhz("135.7-137.8kHz"), Some((0.1357, 0.1378)));
    assert_eq!(parse_range_mhz("18.068-18.168MHz"), Some((18.068, 18.168)));
    assert_eq!(parse_range_mhz("0.03-0.3Hz"), None);
  }
}
