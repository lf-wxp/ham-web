//! 各国 / 地区业余电台呼号前缀速查（ITU 分配，按大洲分组）。

/// 一条呼号前缀。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Prefix {
  pub prefix: &'static str,
  pub entity: &'static str,
}

/// 一个大洲分组。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrefixGroup {
  pub region: &'static str,
  pub prefixes: &'static [Prefix],
}

const fn p(prefix: &'static str, entity: &'static str) -> Prefix {
  Prefix { prefix, entity }
}

/// 常见国家 / 地区呼号前缀（按大洲分组）。
pub const PREFIX_GROUPS: &[PrefixGroup] = &[
  PrefixGroup {
    region: "亚洲",
    prefixes: &[
      p("B", "中国（BA–BH 个人、BR 中继、BJ 信标/空间）"),
      p("BV", "台湾地区"),
      p("VR2", "香港地区"),
      p("XX9", "澳门地区"),
      p("JA–JS", "日本"),
      p("HL", "韩国"),
      p("VU", "印度"),
      p("HS", "泰国"),
      p("9M", "马来西亚"),
      p("9V", "新加坡"),
      p("YB", "印度尼西亚"),
      p("DU", "菲律宾"),
    ],
  },
  PrefixGroup {
    region: "欧洲",
    prefixes: &[
      p("G / M", "英国"),
      p("F", "法国"),
      p("DA–DR / DL", "德国"),
      p("I", "意大利"),
      p("EA", "西班牙"),
      p("CT", "葡萄牙"),
      p("ON", "比利时"),
      p("PA", "荷兰"),
      p("LX", "卢森堡"),
      p("HB", "瑞士"),
      p("OE", "奥地利"),
      p("OH", "芬兰"),
      p("SM", "瑞典"),
      p("LA", "挪威"),
      p("OZ", "丹麦"),
      p("SP", "波兰"),
      p("OK", "捷克"),
      p("OM", "斯洛伐克"),
      p("HA", "匈牙利"),
      p("YO", "罗马尼亚"),
      p("LZ", "保加利亚"),
      p("9A", "克罗地亚"),
      p("S5", "斯洛文尼亚"),
      p("E7", "波黑"),
      p("Z3", "北马其顿"),
      p("4O", "黑山"),
      p("RA–RZ / UA–UZ", "俄罗斯"),
      p("UR–UZ", "乌克兰"),
      p("EW", "白俄罗斯"),
      p("LY", "立陶宛"),
      p("YL", "拉脱维亚"),
      p("ES", "爱沙尼亚"),
      p("YT", "塞尔维亚"),
      p("ER", "摩尔多瓦"),
    ],
  },
  PrefixGroup {
    region: "美洲",
    prefixes: &[
      p("K / N / W", "美国"),
      p("VE", "加拿大"),
      p("XE", "墨西哥"),
      p("TG", "危地马拉"),
      p("HR", "洪都拉斯"),
      p("YS", "萨尔瓦多"),
      p("TI", "哥斯达黎加"),
      p("HP", "巴拿马"),
      p("HI", "多米尼加"),
      p("CO", "古巴"),
      p("LU", "阿根廷"),
      p("PY", "巴西"),
      p("CX", "乌拉圭"),
      p("CE", "智利"),
      p("HC", "厄瓜多尔"),
      p("HK", "哥伦比亚"),
      p("YV", "委内瑞拉"),
      p("OA", "秘鲁"),
      p("CP", "玻利维亚"),
    ],
  },
  PrefixGroup {
    region: "非洲",
    prefixes: &[
      p("ZS", "南非"),
      p("3V", "突尼斯"),
      p("7X", "阿尔及利亚"),
      p("CN", "摩洛哥"),
      p("SU", "埃及"),
      p("C9", "莫桑比克"),
      p("ET", "埃塞俄比亚"),
      p("9G", "加纳"),
      p("5H", "坦桑尼亚"),
    ],
  },
  PrefixGroup {
    region: "大洋洲",
    prefixes: &[
      p("VK", "澳大利亚"),
      p("ZL", "新西兰"),
      p("3D2", "斐济"),
      p("KH6", "夏威夷（美国）"),
    ],
  },
  PrefixGroup {
    region: "中东",
    prefixes: &[
      p("4X / 4Z", "以色列"),
      p("EK", "亚美尼亚"),
      p("4L", "格鲁吉亚"),
      p("HZ", "沙特阿拉伯"),
      p("A6", "阿联酋"),
    ],
  },
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn prefixes_grouped_and_non_empty() {
    let total: usize = PREFIX_GROUPS.iter().map(|g| g.prefixes.len()).sum();
    assert!(total >= 60, "应有足够多的前缀条目，实际 {total}");
    for g in PREFIX_GROUPS {
      assert!(!g.region.is_empty());
      for p in g.prefixes {
        assert!(!p.prefix.is_empty());
        assert!(!p.entity.is_empty());
      }
    }
  }
}
