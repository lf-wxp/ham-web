//! 器材库与选购指南（阶段一：静态精选库）。
//!
//! 收录资料公开、社区共识度高的常见机型（含部分已停产经典型号，在 note 中标注），规格
//! 取「公开且稳定」的高层次信息（频段覆盖、功率量级、模式、特点），不写会随批次变动
//! 与渠道不同的精确参数。
//! 价格档为相对定位（入门 / 中端 / 高端 / 旗舰），不写具体售价以免过期。

/// 器材类别：`(key, 名称)`。
pub const GEAR_CATEGORIES: &[(&str, &str)] = &[
  ("hf", "HF / 全段收发信机"),
  ("portable", "便携 / QRP 电台"),
  ("handheld", "手持对讲机"),
  ("mobile", "车载电台"),
  ("sdr", "SDR 接收机"),
  ("accessory", "功放与天调"),
];

/// 一台器材的参考信息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gear {
  /// 稳定标识（用于选择与对比）。
  pub id: &'static str,
  /// 品牌。
  pub brand: &'static str,
  /// 型号。
  pub model: &'static str,
  /// 类别 key（见 [`GEAR_CATEGORIES`]）。
  pub category: &'static str,
  /// 价格档：入门 / 中端 / 高端 / 旗舰。
  pub tier: &'static str,
  /// 频段覆盖。
  pub bands: &'static str,
  /// 功率量级。
  pub power: &'static str,
  /// 支持模式。
  pub modes: &'static str,
  /// 突出特点。
  pub highlight: &'static str,
  /// 选购点评。
  pub note: &'static str,
}

/// 精选机型库。
pub const GEAR: &[Gear] = &[
  // ── HF / 全段收发信机 ──
  Gear {
    id: "ic-7300",
    brand: "Icom",
    model: "IC-7300",
    category: "hf",
    tier: "中端",
    bands: "HF + 50MHz",
    power: "100W 级",
    modes: "SSB / CW / AM / FM / RTTY / 数字",
    highlight: "内置实时频谱与瀑布图（SDR 架构）",
    note: "入门到进阶最热门的 HF 基地台之一，操作直观、资料多。",
  },
  Gear {
    id: "ft-891",
    brand: "Yaesu",
    model: "FT-891",
    category: "hf",
    tier: "中端",
    bands: "HF + 50MHz",
    power: "100W 级",
    modes: "SSB / CW / AM / FM / 数字",
    highlight: "体积小巧，适合车载与野外基地台",
    note: "无内置天调，需另配天调或使用谐振天线。",
  },
  Gear {
    id: "ft-710",
    brand: "Yaesu",
    model: "FT-710",
    category: "hf",
    tier: "中端",
    bands: "HF + 50MHz",
    power: "100W 级",
    modes: "SSB / CW / AM / FM / 数字",
    highlight: "带带宽频谱显示，性价比高",
    note: "近年热门基地台，面板与数字功能较新。",
  },
  Gear {
    id: "ic-7610",
    brand: "Icom",
    model: "IC-7610",
    category: "hf",
    tier: "旗舰",
    bands: "HF + 50MHz",
    power: "100W 级",
    modes: "SSB / CW / AM / FM / RTTY / 数字",
    highlight: "双接收、独立频谱（SDR 架构）",
    note: "竞赛与 DX 用高端基地台。",
  },
  Gear {
    id: "ftdx10",
    brand: "Yaesu",
    model: "FTdx10",
    category: "hf",
    tier: "高端",
    bands: "HF + 50MHz",
    power: "100W 级",
    modes: "SSB / CW / AM / FM / 数字",
    highlight: "混合 SDR 架构，窄带滤波与接收性能突出",
    note: "高端基地台，适合认真做 DX 与竞赛。",
  },
  // ── 便携 / QRP 电台 ──
  Gear {
    id: "xiegu-g90",
    brand: "Xiegu",
    model: "G90",
    category: "portable",
    tier: "入门",
    bands: "HF（不含 50MHz）",
    power: "20W 级",
    modes: "SSB / CW / AM / FM / 数字",
    highlight: "内置自动天调，价格友好",
    note: "国内常见入门 HF 台，20W 适合 QRP 与新手练手。",
  },
  Gear {
    id: "ic-705",
    brand: "Icom",
    model: "IC-705",
    category: "portable",
    tier: "中端",
    bands: "HF / VHF / UHF",
    power: "10W 级",
    modes: "SSB / CW / AM / FM / D-STAR / 数字",
    highlight: "全频段便携，内置 GPS 与蓝牙",
    note: "可当便携电台或高质量接收机使用。",
  },
  Gear {
    id: "kx2",
    brand: "Elecraft",
    model: "KX2",
    category: "portable",
    tier: "高端",
    bands: "HF（80–10m）",
    power: "QRP 10W 级",
    modes: "SSB / CW / AM / 数字",
    highlight: "轻量 QRP 便携，内置电池与天调",
    note: "徒步与 SOTA / POTA 的高端选择。",
  },
  Gear {
    id: "ft-818",
    brand: "Yaesu",
    model: "FT-818",
    category: "portable",
    tier: "中端",
    bands: "HF / VHF / UHF",
    power: "QRP 5W 级（HF 段约 6W）",
    modes: "SSB / CW / AM / FM / 数字",
    highlight: "经典全频段 QRP 便携机型",
    note: "已停产；老牌机型，配件与改装社区资料丰富。",
  },
  // ── 手持对讲机 ──
  Gear {
    id: "uv-5r",
    brand: "Baofeng",
    model: "UV-5R",
    category: "handheld",
    tier: "入门",
    bands: "2m / 70cm",
    power: "5W 级（标称）",
    modes: "FM",
    highlight: "价格极低、配件丰富",
    note: "入门尝鲜首选；注意实际功率、指标与合法使用。",
  },
  Gear {
    id: "ft-60r",
    brand: "Yaesu",
    model: "FT-60R",
    category: "handheld",
    tier: "入门",
    bands: "2m / 70cm",
    power: "5W 级",
    modes: "FM",
    highlight: "结实耐用、操作简单",
    note: "已停产；传统模拟手台，故障率低，适合日常使用。",
  },
  Gear {
    id: "ft-5dr",
    brand: "Yaesu",
    model: "FT-5DR",
    category: "handheld",
    tier: "中端",
    bands: "2m / 70cm",
    power: "5W 级",
    modes: "FM / C4FM 数字",
    highlight: "支持 System Fusion 数字语音与 APRS",
    note: "数字语音入门热门，按本地中继制式选择。",
  },
  Gear {
    id: "d878uv",
    brand: "Anytone",
    model: "AT-D878UV II",
    category: "handheld",
    tier: "中端",
    bands: "2m / 70cm",
    power: "5W 级",
    modes: "FM / DMR 数字",
    highlight: "DMR 手台热门，支持 GPS / 蓝牙",
    note: "玩 DMR 的常见选择，需配合中继或热点。",
  },
  Gear {
    id: "id-52a",
    brand: "Icom",
    model: "ID-52A",
    category: "handheld",
    tier: "高端",
    bands: "2m / 70cm",
    power: "5W 级",
    modes: "FM / D-STAR 数字",
    highlight: "D-STAR 生态，内置 GPS 与蓝牙",
    note: "D-STAR 用户的高端手台。",
  },
  // ── 车载电台 ──
  Gear {
    id: "ic-2730a",
    brand: "Icom",
    model: "IC-2730A",
    category: "mobile",
    tier: "中端",
    bands: "2m / 70cm",
    power: "50W 级",
    modes: "FM",
    highlight: "双段双显，面板可分离安装",
    note: "经典模拟车载台，操作直观。",
  },
  Gear {
    id: "tm-d710g",
    brand: "Kenwood",
    model: "TM-D710G",
    category: "mobile",
    tier: "中端",
    bands: "2m / 70cm",
    power: "50W 级",
    modes: "FM / APRS",
    highlight: "内置 TNC 与 APRS 导航",
    note: "已停产；想玩 APRS 的车载经典型号。",
  },
  Gear {
    id: "ftm-400xdr",
    brand: "Yaesu",
    model: "FTM-400XDR",
    category: "mobile",
    tier: "中端",
    bands: "2m / 70cm",
    power: "50W 级",
    modes: "FM / C4FM 数字",
    highlight: "大屏触摸操作，支持 APRS",
    note: "已由 FTM-500 系列取代；数字语音 + APRS 车载方案。",
  },
  // ── SDR 接收机 ──
  Gear {
    id: "rtl-sdr",
    brand: "RTL-SDR Blog",
    model: "V3 / V4",
    category: "sdr",
    tier: "入门",
    bands: "约 24MHz–1.7GHz（接收）",
    power: "接收机",
    modes: "由软件解调",
    highlight: "最便宜的入门 SDR 之一",
    note: "收听 HF 需加变频器或做直采改装。",
  },
  Gear {
    id: "airspy-hf",
    brand: "Airspy",
    model: "HF+ Discovery",
    category: "sdr",
    tier: "中端",
    bands: "HF / VHF（接收）",
    power: "接收机",
    modes: "由软件解调",
    highlight: "HF 接收灵敏度与动态范围好",
    note: "短波收听与数字解码常用。",
  },
  Gear {
    id: "rspdx",
    brand: "SDRplay",
    model: "RSPdx",
    category: "sdr",
    tier: "中端",
    bands: "LF – 2GHz（接收）",
    power: "接收机",
    modes: "由软件解调",
    highlight: "宽频段覆盖，多天线端口",
    note: "配合 SDRuno 等软件使用。",
  },
  // ── 功放与天调 ──
  Gear {
    id: "xpa125b",
    brand: "Xiegu",
    model: "XPA125B",
    category: "accessory",
    tier: "中端",
    bands: "HF + 50MHz",
    power: "约 100W 级（低功率输入驱动）",
    modes: "配合 QRP 电台使用",
    highlight: "内置自动天调",
    note: "给 G90 等低功率电台扩容到百瓦级。",
  },
  Gear {
    id: "ldg-z100plus",
    brand: "LDG",
    model: "Z-100Plus",
    category: "accessory",
    tier: "入门",
    bands: "HF",
    power: "100W 级",
    modes: "自动天线调谐器",
    highlight: "体积小、自动调谐",
    note: "给没有内置天调的 HF 电台补上匹配能力。",
  },
  Gear {
    id: "ah-4",
    brand: "Icom",
    model: "AH-4",
    category: "accessory",
    tier: "中端",
    bands: "HF + 50MHz",
    power: "室外自动天调",
    modes: "长线 / 鞭状天线匹配",
    highlight: "室外安装，缩短馈线损耗",
    note: "适合长线天线与固定架设场景。",
  },
];

/// 按 id 查找机型。
#[must_use]
pub fn by_id(id: &str) -> Option<&'static Gear> {
  GEAR.iter().find(|g| g.id == id)
}

/// 某个类别下的全部机型。
#[must_use]
pub fn in_category(category: &str) -> Vec<&'static Gear> {
  GEAR.iter().filter(|g| g.category == category).collect()
}

/// 按用途推荐：`(用途, 推荐机型, 理由)`。
pub const GEAR_PICKS: &[(&str, &str, &str)] = &[
  (
    "新手入门（基地台）",
    "Icom IC-7300 / Yaesu FT-710",
    "内置频谱、100W 级、教程与资料多，上手最快。",
  ),
  (
    "低预算入门 HF",
    "Xiegu G90",
    "20W 且内置天调，价格友好；先从 QRP 与自制天线练手。",
  ),
  (
    "便携 / SOTA / POTA",
    "Icom IC-705 / Elecraft KX2 / Yaesu FT-818",
    "全频段、可电池供电、重量轻。",
  ),
  (
    "首次购买手台",
    "Yaesu FT-60R / Baofeng UV-5R",
    "模拟 FM 手台简单可靠；预算极低可先玩 UV-5R。",
  ),
  (
    "数字语音（DMR / C4FM / D-STAR）",
    "Anytone AT-D878UV II / Yaesu FT-5DR / Icom ID-52A",
    "分别对应 DMR / C4FM / D-STAR 生态，按本地中继制式选择。",
  ),
  (
    "车载 + APRS",
    "Kenwood TM-D710G / Icom IC-2730A",
    "前者内置 APRS，后者操作直观。",
  ),
  (
    "短波接收 / 监听入门",
    "RTL-SDR Blog V3/V4 / Airspy HF+ Discovery",
    "几十到几百元即可上手 SDR 接收与数字解码。",
  ),
  (
    "低功率电台扩容",
    "Xiegu XPA125B",
    "搭配 G90 等 QRP 电台提升到百瓦级，并自带天调。",
  ),
];

/// 选购要点。
pub const GEAR_TIPS: &[&str] = &[
  "先定用途与预算，再看参数：基地台、车载、便携、QRP 是完全不同的取舍。",
  "接收性能看中频与动态范围，不要只盯着发射功率和屏幕大小。",
  "数字语音（DMR / C4FM / D-STAR）互不兼容，先查本地中继支持哪种制式再买。",
  "预算里要给天线、馈线、电源与天调留位置：天线对效果的影响往往比电台更大。",
  "二手交易注意频率范围与发射权限是否合法，以及是否被锁频、缺少配件。",
  "本页为参考整理，型号与规格随批次更新，购买前以厂商最新资料与经销商信息为准。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn gear_ids_are_unique_and_categories_known() {
    let mut ids: Vec<&str> = GEAR.iter().map(|g| g.id).collect();
    let total = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), total, "器材 id 必须唯一");
    assert!(total >= 15, "机型数量偏少：{total}");
    for g in GEAR {
      assert!(
        GEAR_CATEGORIES.iter().any(|(k, _)| *k == g.category),
        "{} 的类别 {} 未在 GEAR_CATEGORIES 中登记",
        g.id,
        g.category
      );
      assert!(!g.brand.is_empty() && !g.model.is_empty() && !g.note.is_empty());
    }
  }

  #[test]
  fn every_category_has_models() {
    for (key, _) in GEAR_CATEGORIES {
      assert!(
        !in_category(key).is_empty(),
        "类别 {key} 没有任何机型，对比表会为空"
      );
    }
  }

  #[test]
  fn picks_and_tips_are_populated() {
    assert!(GEAR_PICKS.len() >= 6);
    assert!(!GEAR_TIPS.is_empty());
    for (use_case, pick, why) in GEAR_PICKS {
      assert!(!use_case.is_empty() && !pick.is_empty() && !why.is_empty());
    }
  }

  #[test]
  fn lookup_by_id_works() {
    assert_eq!(by_id("ic-7300").map(|g| g.model), Some("IC-7300"));
    assert_eq!(by_id("nope"), None);
  }
}
