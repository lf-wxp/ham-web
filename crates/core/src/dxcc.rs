//! DXCC 实体识别：根据呼号判断所属 DXCC 实体（国家 / 地区 / 岛屿）。
//!
//! 前缀数据来自 AD1C 维护的 `cty.csv`（由 `ham-web-tools dxcc` 生成 `data/dxcc.txt`），
//! 覆盖全部 340 个现行 DXCC 实体；中文名按 DXCC 编号维护在 [`ZH_NAMES`]。
//!
//! 匹配规则与常见日志软件一致：先查整呼号例外，再做最长前缀匹配；
//! 斜杠呼号取较短的一段作为前缀（`VK9X/BG4XXX`、`KH6/K1ABC`），
//! `/P`、`/M`、`/QRP`、`/数字` 等后缀忽略，`/MM`、`/AM` 不属于任何实体。

use std::collections::HashMap;
use std::sync::OnceLock;

const DATA: &str = include_str!("../data/dxcc.txt");

/// 一个 DXCC 实体。
#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
  /// ADIF `DXCC` 编号。
  pub dxcc: u16,
  /// 主前缀（cty 格式，如 `BY`、`KH6`、`FT/w`）。
  pub prefix: &'static str,
  /// 中文名。
  pub name: &'static str,
  /// 英文名。
  pub name_en: &'static str,
  /// 大洲（`AS` / `EU` / `AF` / `NA` / `SA` / `OC`）。
  pub continent: &'static str,
  /// CQ 分区。
  pub cq: u8,
  /// ITU 分区。
  pub itu: u8,
  /// 纬度。
  pub lat: f64,
  /// 经度（东经为正）。
  pub lon: f64,
}

struct Table {
  entities: Vec<Entity>,
  prefixes: HashMap<&'static str, usize>,
  exact: HashMap<&'static str, usize>,
  max_len: usize,
}

fn table() -> &'static Table {
  static TABLE: OnceLock<Table> = OnceLock::new();
  TABLE.get_or_init(|| {
    let mut t = Table {
      entities: Vec::new(),
      prefixes: HashMap::new(),
      exact: HashMap::new(),
      max_len: 0,
    };
    for line in DATA
      .lines()
      .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
      let f: Vec<&'static str> = line.split('|').collect();
      let [dxcc, prefix, name_en, continent, cq, itu, lat, lon, rules] = f[..] else {
        continue;
      };
      let dxcc: u16 = dxcc.parse().unwrap_or_default();
      let idx = t.entities.len();
      t.entities.push(Entity {
        dxcc,
        prefix,
        name: zh_name(dxcc).unwrap_or(name_en),
        name_en,
        continent,
        cq: cq.parse().unwrap_or_default(),
        itu: itu.parse().unwrap_or_default(),
        lat: lat.parse().unwrap_or_default(),
        lon: lon.parse().unwrap_or_default(),
      });
      for rule in rules.split_whitespace() {
        if let Some(call) = rule.strip_prefix('=') {
          t.exact.insert(call, idx);
        } else {
          t.max_len = t.max_len.max(rule.len());
          t.prefixes.insert(rule, idx);
        }
      }
    }
    t
  })
}

/// 全部 DXCC 实体（按 cty 顺序）。
#[must_use]
pub fn entities() -> &'static [Entity] {
  &table().entities
}

/// 按中文名查找实体。
#[must_use]
pub fn entity_by_name(name: &str) -> Option<&'static Entity> {
  entities().iter().find(|e| e.name == name)
}

/// 按 DXCC 编号查找实体。
#[must_use]
pub fn entity_by_dxcc(dxcc: u16) -> Option<&'static Entity> {
  entities().iter().find(|e| e.dxcc == dxcc)
}

/// 不改变所属实体的斜杠后缀。
fn is_modifier(part: &str) -> bool {
  matches!(
    part,
    "P" | "M" | "QRP" | "QRO" | "A" | "B" | "LH" | "R" | "J"
  ) || part.chars().all(|c| c.is_ascii_digit())
}

/// 从呼号中取出用于前缀匹配的部分；`/MM`、`/AM` 返回 `None`。
fn base_of(call: &str) -> Option<&str> {
  let parts: Vec<&str> = call.split('/').filter(|p| !p.is_empty()).collect();
  if parts.iter().any(|p| matches!(*p, "MM" | "AM")) {
    return None;
  }
  let mut rest = parts.into_iter().filter(|p| !is_modifier(p));
  let first = rest.next()?;
  Some(match rest.next() {
    Some(second) if second.len() < first.len() => second,
    _ => first,
  })
}

/// 识别呼号所属实体，同时返回匹配到的前缀（整呼号例外时为整个呼号）。
#[must_use]
pub fn lookup_with_prefix(callsign: &str) -> Option<(&'static str, &'static Entity)> {
  let call = callsign.trim().to_ascii_uppercase();
  if call.is_empty() || !call.is_ascii() {
    return None;
  }
  let t = table();
  if let Some((&k, &i)) = t.exact.get_key_value(call.as_str()) {
    return Some((k, &t.entities[i]));
  }
  let base = base_of(&call)?;
  if let Some((&k, &i)) = t.exact.get_key_value(base) {
    return Some((k, &t.entities[i]));
  }
  (1..=base.len().min(t.max_len)).rev().find_map(|n| {
    t.prefixes
      .get_key_value(&base[..n])
      .map(|(&k, &i)| (k, &t.entities[i]))
  })
}

/// 识别呼号所属实体。
#[must_use]
pub fn lookup(callsign: &str) -> Option<&'static Entity> {
  lookup_with_prefix(callsign).map(|(_, e)| e)
}

/// 根据呼号判断 DXCC 实体中文名，无法识别返回 `None`。
#[must_use]
pub fn dxcc_entity(callsign: &str) -> Option<&'static str> {
  lookup(callsign).map(|e| e.name)
}

/// 根据 DXCC 实体中文名返回 `(CQ 分区, ITU 分区)`。
#[must_use]
pub fn entity_zones(entity: &str) -> Option<(u8, u8)> {
  entity_by_name(entity).map(|e| (e.cq, e.itu))
}

fn zh_name(dxcc: u16) -> Option<&'static str> {
  ZH_NAMES.iter().find(|(n, _)| *n == dxcc).map(|(_, s)| *s)
}

/// DXCC 编号 → 中文名。
pub const ZH_NAMES: &[(u16, &str)] = &[
  (1, "加拿大"),
  (3, "阿富汗"),
  (4, "阿加莱加与圣布兰登"),
  (5, "奥兰群岛"),
  (6, "阿拉斯加"),
  (7, "阿尔巴尼亚"),
  (9, "美属萨摩亚"),
  (10, "阿姆斯特丹与圣保罗岛"),
  (11, "安达曼和尼科巴群岛"),
  (12, "安圭拉"),
  (13, "南极洲"),
  (14, "亚美尼亚"),
  (15, "亚洲俄罗斯"),
  (16, "新西兰亚南极群岛"),
  (17, "阿韦斯岛"),
  (18, "阿塞拜疆"),
  (20, "贝克与豪兰岛"),
  (21, "巴利阿里群岛"),
  (22, "帕劳"),
  (24, "布韦岛"),
  (27, "白俄罗斯"),
  (29, "加那利群岛"),
  (31, "中基里巴斯"),
  (32, "休达和梅利利亚"),
  (33, "查戈斯群岛"),
  (34, "查塔姆群岛"),
  (35, "圣诞岛"),
  (36, "克利珀顿岛"),
  (37, "科科斯岛"),
  (38, "科科斯（基林）群岛"),
  (40, "克里特岛"),
  (41, "克罗泽群岛"),
  (43, "德塞切奥岛"),
  (45, "多德卡尼斯群岛"),
  (46, "东马来西亚"),
  (47, "复活节岛"),
  (48, "东基里巴斯"),
  (49, "赤道几内亚"),
  (50, "墨西哥"),
  (51, "厄立特里亚"),
  (52, "爱沙尼亚"),
  (53, "埃塞俄比亚"),
  (54, "欧洲俄罗斯"),
  (56, "费尔南多-迪诺罗尼亚"),
  (60, "巴哈马"),
  (61, "法兰士约瑟夫地群岛"),
  (62, "巴巴多斯"),
  (63, "法属圭亚那"),
  (64, "百慕大"),
  (65, "英属维尔京群岛"),
  (66, "伯利兹"),
  (69, "开曼群岛"),
  (70, "古巴"),
  (71, "加拉帕戈斯群岛"),
  (72, "多米尼加共和国"),
  (74, "萨尔瓦多"),
  (75, "格鲁吉亚"),
  (76, "危地马拉"),
  (77, "格林纳达"),
  (78, "海地"),
  (79, "瓜德罗普"),
  (80, "洪都拉斯"),
  (82, "牙买加"),
  (84, "马提尼克"),
  (86, "尼加拉瓜"),
  (88, "巴拿马"),
  (89, "特克斯和凯科斯群岛"),
  (90, "特立尼达和多巴哥"),
  (91, "阿鲁巴"),
  (94, "安提瓜和巴布达"),
  (95, "多米尼克"),
  (96, "蒙特塞拉特"),
  (97, "圣卢西亚"),
  (98, "圣文森特"),
  (99, "格洛里厄斯群岛"),
  (100, "阿根廷"),
  (103, "关岛"),
  (104, "玻利维亚"),
  (105, "关塔那摩湾"),
  (106, "根西岛"),
  (107, "几内亚"),
  (108, "巴西"),
  (109, "几内亚比绍"),
  (110, "夏威夷"),
  (111, "赫德岛"),
  (112, "智利"),
  (114, "马恩岛"),
  (116, "哥伦比亚"),
  (117, "国际电联总部"),
  (118, "扬马延岛"),
  (120, "厄瓜多尔"),
  (122, "泽西岛"),
  (123, "约翰斯顿岛"),
  (124, "新胡安与欧罗巴岛"),
  (125, "胡安·费尔南德斯群岛"),
  (126, "加里宁格勒"),
  (129, "圭亚那"),
  (130, "哈萨克斯坦"),
  (131, "凯尔盖朗群岛"),
  (132, "巴拉圭"),
  (133, "克马德克群岛"),
  (135, "吉尔吉斯斯坦"),
  (136, "秘鲁"),
  (137, "韩国"),
  (138, "库雷岛"),
  (140, "苏里南"),
  (141, "福克兰群岛"),
  (142, "拉克沙群岛"),
  (143, "老挝"),
  (144, "乌拉圭"),
  (145, "拉脱维亚"),
  (146, "立陶宛"),
  (147, "豪勋爵岛"),
  (148, "委内瑞拉"),
  (149, "亚速尔群岛"),
  (150, "澳大利亚"),
  (152, "澳门"),
  (153, "麦夸里岛"),
  (157, "瑙鲁"),
  (158, "瓦努阿图"),
  (159, "马尔代夫"),
  (160, "汤加"),
  (161, "马尔佩洛岛"),
  (162, "新喀里多尼亚"),
  (163, "巴布亚新几内亚"),
  (165, "毛里求斯"),
  (166, "北马里亚纳群岛"),
  (167, "马基特礁"),
  (168, "马绍尔群岛"),
  (169, "马约特"),
  (170, "新西兰"),
  (171, "梅利什礁"),
  (172, "皮特凯恩岛"),
  (173, "密克罗尼西亚"),
  (174, "中途岛"),
  (175, "法属波利尼西亚"),
  (176, "斐济"),
  (177, "南鸟岛"),
  (179, "摩尔多瓦"),
  (180, "阿索斯山"),
  (181, "莫桑比克"),
  (182, "纳弗沙岛"),
  (185, "所罗门群岛"),
  (187, "尼日尔"),
  (188, "纽埃"),
  (189, "诺福克岛"),
  (190, "萨摩亚"),
  (191, "北库克群岛"),
  (192, "小笠原群岛"),
  (195, "安诺本岛"),
  (197, "巴尔米拉与贾维斯岛"),
  (199, "彼得一世岛"),
  (201, "爱德华王子与马里昂群岛"),
  (202, "波多黎各"),
  (203, "安道尔"),
  (204, "雷维利亚希赫多群岛"),
  (205, "阿森松岛"),
  (206, "奥地利"),
  (207, "罗德里格斯岛"),
  (209, "比利时"),
  (211, "塞布尔岛"),
  (212, "保加利亚"),
  (213, "法属圣马丁"),
  (214, "科西嘉岛"),
  (215, "塞浦路斯"),
  (216, "圣安德烈斯与普罗维登西亚"),
  (217, "圣费利克斯与圣安布罗西奥岛"),
  (219, "圣多美和普林西比"),
  (221, "丹麦"),
  (222, "法罗群岛"),
  (223, "英格兰"),
  (224, "芬兰"),
  (225, "撒丁岛"),
  (227, "法国"),
  (230, "德国"),
  (232, "索马里"),
  (233, "直布罗陀"),
  (234, "南库克群岛"),
  (235, "南乔治亚岛"),
  (236, "希腊"),
  (237, "格陵兰"),
  (238, "南奥克尼群岛"),
  (239, "匈牙利"),
  (240, "南桑威奇群岛"),
  (241, "南设得兰群岛"),
  (242, "冰岛"),
  (245, "爱尔兰"),
  (246, "马耳他骑士团"),
  (247, "南沙群岛"),
  (248, "意大利"),
  (249, "圣基茨和尼维斯"),
  (250, "圣赫勒拿"),
  (251, "列支敦士登"),
  (252, "圣保罗岛"),
  (253, "圣彼得和圣保罗岩"),
  (254, "卢森堡"),
  (256, "马德拉群岛"),
  (257, "马耳他"),
  (259, "斯瓦尔巴群岛"),
  (260, "摩纳哥"),
  (262, "塔吉克斯坦"),
  (263, "荷兰"),
  (265, "北爱尔兰"),
  (266, "挪威"),
  (269, "波兰"),
  (270, "托克劳群岛"),
  (272, "葡萄牙"),
  (273, "特林达德与马丁瓦斯"),
  (274, "特里斯坦-达库尼亚与戈夫岛"),
  (275, "罗马尼亚"),
  (276, "特罗姆兰岛"),
  (277, "圣皮埃尔和密克隆"),
  (278, "圣马力诺"),
  (279, "苏格兰"),
  (280, "土库曼斯坦"),
  (281, "西班牙"),
  (282, "图瓦卢"),
  (283, "英属塞浦路斯基地区"),
  (284, "瑞典"),
  (285, "美属维尔京群岛"),
  (286, "乌干达"),
  (287, "瑞士"),
  (288, "乌克兰"),
  (289, "联合国总部"),
  (291, "美国"),
  (292, "乌兹别克斯坦"),
  (293, "越南"),
  (294, "威尔士"),
  (295, "梵蒂冈"),
  (296, "塞尔维亚"),
  (297, "威克岛"),
  (298, "瓦利斯和富图纳"),
  (299, "西马来西亚"),
  (301, "西基里巴斯"),
  (302, "西撒哈拉"),
  (303, "威利斯岛"),
  (304, "巴林"),
  (305, "孟加拉国"),
  (306, "不丹"),
  (308, "哥斯达黎加"),
  (309, "缅甸"),
  (312, "柬埔寨"),
  (315, "斯里兰卡"),
  (318, "中国"),
  (321, "香港"),
  (324, "印度"),
  (327, "印度尼西亚"),
  (330, "伊朗"),
  (333, "伊拉克"),
  (336, "以色列"),
  (339, "日本"),
  (342, "约旦"),
  (344, "朝鲜"),
  (345, "文莱"),
  (348, "科威特"),
  (354, "黎巴嫩"),
  (363, "蒙古"),
  (369, "尼泊尔"),
  (370, "阿曼"),
  (372, "巴基斯坦"),
  (375, "菲律宾"),
  (376, "卡塔尔"),
  (378, "沙特阿拉伯"),
  (379, "塞舌尔"),
  (381, "新加坡"),
  (382, "吉布提"),
  (384, "叙利亚"),
  (386, "台湾"),
  (387, "泰国"),
  (390, "土耳其"),
  (391, "阿联酋"),
  (400, "阿尔及利亚"),
  (401, "安哥拉"),
  (402, "博茨瓦纳"),
  (404, "布隆迪"),
  (406, "喀麦隆"),
  (408, "中非"),
  (409, "佛得角"),
  (410, "乍得"),
  (411, "科摩罗"),
  (412, "刚果（布）"),
  (414, "刚果（金）"),
  (416, "贝宁"),
  (420, "加蓬"),
  (422, "冈比亚"),
  (424, "加纳"),
  (428, "科特迪瓦"),
  (430, "肯尼亚"),
  (432, "莱索托"),
  (434, "利比里亚"),
  (436, "利比亚"),
  (438, "马达加斯加"),
  (440, "马拉维"),
  (442, "马里"),
  (444, "毛里塔尼亚"),
  (446, "摩洛哥"),
  (450, "尼日利亚"),
  (452, "津巴布韦"),
  (453, "留尼汪"),
  (454, "卢旺达"),
  (456, "塞内加尔"),
  (458, "塞拉利昂"),
  (460, "罗图马岛"),
  (462, "南非"),
  (464, "纳米比亚"),
  (466, "苏丹"),
  (468, "斯威士兰"),
  (470, "坦桑尼亚"),
  (474, "突尼斯"),
  (478, "埃及"),
  (480, "布基纳法索"),
  (482, "赞比亚"),
  (483, "多哥"),
  (489, "康威礁"),
  (490, "巴纳巴岛"),
  (492, "也门"),
  (497, "克罗地亚"),
  (499, "斯洛文尼亚"),
  (501, "波黑"),
  (502, "北马其顿"),
  (503, "捷克"),
  (504, "斯洛伐克"),
  (505, "东沙群岛"),
  (506, "黄岩岛"),
  (507, "泰莫图省"),
  (508, "南方群岛"),
  (509, "马克萨斯群岛"),
  (510, "巴勒斯坦"),
  (511, "东帝汶"),
  (512, "切斯特菲尔德群岛"),
  (513, "迪西岛"),
  (514, "黑山"),
  (515, "斯温斯岛"),
  (516, "圣巴泰勒米"),
  (517, "库拉索"),
  (518, "荷属圣马丁"),
  (519, "萨巴与圣尤斯特歇斯"),
  (520, "博奈尔"),
  (521, "南苏丹"),
  (522, "科索沃"),
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn covers_all_entities_with_chinese_names() {
    assert_eq!(entities().len(), 340);
    let missing: Vec<&str> = entities()
      .iter()
      .filter(|e| zh_name(e.dxcc).is_none())
      .map(|e| e.name_en)
      .collect();
    assert!(missing.is_empty(), "missing zh names: {missing:?}");
    assert_eq!(ZH_NAMES.len(), 340);
  }

  #[test]
  fn matches_longest_prefix_first() {
    assert_eq!(dxcc_entity("BG4XXX"), Some("中国"));
    assert_eq!(dxcc_entity("BV2AA"), Some("台湾"));
    assert_eq!(dxcc_entity("BV9PA"), Some("东沙群岛"));
    assert_eq!(dxcc_entity("JA1ABC"), Some("日本"));
    assert_eq!(dxcc_entity("K1ZZ"), Some("美国"));
    assert_eq!(dxcc_entity("KH6ABC"), Some("夏威夷"));
    assert_eq!(dxcc_entity("DL1AA"), Some("德国"));
    assert_eq!(dxcc_entity("VR2XY"), Some("香港"));
    assert_eq!(dxcc_entity("RX3ABC"), Some("欧洲俄罗斯"));
    assert_eq!(dxcc_entity("UA0ABC"), Some("亚洲俄罗斯"));
    assert_eq!(dxcc_entity("G4ABC"), Some("英格兰"));
    assert_eq!(dxcc_entity("MM0ABC"), Some("苏格兰"));
    assert_eq!(dxcc_entity("P5ABC"), Some("朝鲜"));
    assert_eq!(dxcc_entity("BS7H"), Some("黄岩岛"));
    assert_eq!(dxcc_entity(""), None);
  }

  #[test]
  fn handles_slashed_callsigns() {
    assert_eq!(dxcc_entity("VK9X/BG4XXX"), Some("圣诞岛"));
    assert_eq!(dxcc_entity("BG4XXX/VK9X"), Some("圣诞岛"));
    assert_eq!(dxcc_entity("KH6/K1ABC"), Some("夏威夷"));
    assert_eq!(dxcc_entity("BG4XXX/P"), Some("中国"));
    assert_eq!(dxcc_entity("JA1ABC/3"), Some("日本"));
    assert_eq!(dxcc_entity("BG4XXX/MM"), None);
  }

  #[test]
  fn entity_zones_lookup() {
    assert_eq!(entity_zones("日本"), Some((25, 45)));
    assert_eq!(entity_zones("未知"), None);
    let cn = entity_by_dxcc(318).expect("china");
    assert_eq!((cn.continent, cn.lon > 0.0), ("AS", true));
  }
}
