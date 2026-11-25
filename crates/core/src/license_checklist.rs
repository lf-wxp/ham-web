//! 设台申请核对清单：把《业余无线电台管理办法》里与「申请设置、使用业余无线电台」有关的要求
//! 逐项列出，每一项都指到条文，能用站内工具核对的再指到工具。
//!
//! # 规则
//!
//! - **办法里没有的材料项不写。** 例如办法没有要求提交天线形式，清单里就没有；这不是疏漏。
//! - 每一项带 [`Cite`]：**条号 + 该条原文里的一句原话**。单测逐条核对这句话真的出现在被引的条文里，
//!   所以清单的每个说法都能回到原文，改了法规数据而没改清单会直接红。
//! - 不代填各省的申请表：这里只列「要准备什么、向谁申请、哪些数要对上」，表格本身以办法附件 1 为准。
//!
//! 界面文字（各项的一句话说明）在 app 侧按 [`Item::id`] 取词条；本模块只管结构与出处。

/// 办法在法规库里的 key（`data/radio-law.txt` 的 `LAW` 行）。
pub const LAW_KEY: &str = "办法";

/// 清单分组，按申请的先后顺序。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
  /// 设置、使用业余无线电台应当具备的条件（第七条）。
  Conditions,
  /// 向谁申请（第十条）。
  Authority,
  /// 提交的材料（第十一、十二条）。
  Materials,
  /// 申请书里要对在一起的数：类别权限、频率、功率、执照载明事项。
  Parameters,
  /// 受理与审批（第十三至十七条）。
  Review,
  /// 领证之后（第十八至二十二条）。
  After,
}

/// 哪些申请人才需要这一项。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Audience {
  /// 所有人。
  Everyone,
  /// 申请人是未成年人。
  Minor,
  /// 以单位名义申请。
  Unit,
}

/// 清单项可以指向的站内工具。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
  /// `/compliance`：按类别核对频率与功率。
  Compliance,
  /// `/license-classes`：各类别的权限。
  LicenseClasses,
  /// `/regulations`：法规与管理。
  Regulations,
  /// `/tools#rf-exposure`：公众暴露距离评估。
  RfExposure,
  /// `/license#expiry-reminder`：本页的执照到期提醒。
  ExpiryReminder,
}

impl Tool {
  /// 站内链接。
  #[must_use]
  pub const fn href(self) -> &'static str {
    match self {
      Self::Compliance => "/compliance",
      Self::LicenseClasses => "/license-classes",
      Self::Regulations => "/regulations",
      Self::RfExposure => "/tools#rf-exposure",
      Self::ExpiryReminder => "/license#expiry-reminder",
    }
  }
}

/// 出处：条号加该条原文里的一句原话。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cite {
  /// 办法的条号。
  pub article: u16,
  /// 必须**原样**出现在这一条条文里的一句话（单测核对）。
  pub phrase: &'static str,
}

/// 一个清单项。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Item {
  /// 稳定 id（界面按它取说明文字、勾选状态按它存）。
  pub id: &'static str,
  pub group: Group,
  pub audience: Audience,
  /// 出处，至少一条。
  pub cites: &'static [Cite],
  /// 可用来核对这一项的站内工具。
  pub tool: Option<Tool>,
}

const fn c(article: u16, phrase: &'static str) -> Cite {
  Cite { article, phrase }
}

/// 全部清单项（顺序即显示顺序）。
pub const ITEMS: &[Item] = &[
  // ---- 设台条件（第七条）----
  Item {
    id: "know-rules",
    group: Group::Conditions,
    audience: Audience::Everyone,
    cites: &[c(7, "熟悉无线电管理规定")],
    tool: Some(Tool::Regulations),
  },
  Item {
    id: "passed-verification",
    group: Group::Conditions,
    audience: Audience::Everyone,
    cites: &[
      c(7, "通过相应的操作技术能力验证"),
      c(25, "分为A类、B类和C类"),
    ],
    tool: Some(Tool::LicenseClasses),
  },
  Item {
    id: "device-qualifies",
    group: Group::Conditions,
    audience: Audience::Everyone,
    cites: &[
      c(7, "依法取得型号核准"),
      c(7, "符合国家标准和国家无线电管理规定"),
      c(7, "无线电发射频率范围仅限于业余业务频段"),
    ],
    tool: None,
  },
  // ---- 向谁申请（第十条、第十六条）----
  Item {
    id: "province-authority",
    group: Group::Authority,
    audience: Audience::Everyone,
    cites: &[
      c(10, "向电台所在地的省、自治区、直辖市无线电管理机构提出申请"),
      c(
        10,
        "向申请人住所地的省、自治区、直辖市无线电管理机构提出申请",
      ),
    ],
    tool: None,
  },
  Item {
    id: "national-authority",
    group: Group::Authority,
    audience: Audience::Everyone,
    cites: &[
      c(
        10,
        "15瓦以上短波业余无线电台以及涉及国家主权、安全的其他重要业余无线电台",
      ),
      c(10, "应当向国家无线电管理机构提出申请"),
      c(16, "使用方式、技术条件、安装环境"),
    ],
    tool: None,
  },
  // ---- 提交的材料（第十一、十二条）----
  Item {
    id: "form",
    group: Group::Materials,
    audience: Audience::Everyone,
    cites: &[c(11, "申请表（样式见附件1）")],
    tool: None,
  },
  Item {
    id: "identity",
    group: Group::Materials,
    audience: Audience::Everyone,
    cites: &[c(11, "身份证明复印件")],
    tool: None,
  },
  Item {
    id: "device-photo",
    group: Group::Materials,
    audience: Audience::Everyone,
    cites: &[c(11, "型号核准代码、出厂序列号等信息的无线电发射设备照片")],
    tool: None,
  },
  Item {
    id: "device-statement",
    group: Group::Materials,
    audience: Audience::Everyone,
    cites: &[c(11, "该设备符合本办法第七条第三项规定条件的说明材料")],
    tool: None,
  },
  Item {
    id: "guardian",
    group: Group::Materials,
    audience: Audience::Minor,
    cites: &[c(
      11,
      "监护人身份证明复印件，以及申请人与监护人关系的说明材料",
    )],
    tool: None,
  },
  Item {
    id: "unit-documents",
    group: Group::Materials,
    audience: Audience::Unit,
    cites: &[c(
      12,
      "单位营业执照等复印件，以及业余无线电台技术负责人为本单位工作人员的说明材料",
    )],
    tool: None,
  },
  // ---- 要对在一起的数（第八、十五、十九、三十四、三十条）----
  Item {
    id: "class-limits",
    group: Group::Parameters,
    audience: Audience::Everyone,
    cites: &[
      c(30, "工作在30-3000MHz频段且最大发射功率不大于25瓦"),
      c(30, "工作在30MHz以下频段且最大发射功率小于15瓦"),
      c(30, "工作在30MHz以下频段且最大发射功率不大于1000瓦"),
    ],
    tool: Some(Tool::Compliance),
  },
  Item {
    id: "minor-limits",
    group: Group::Parameters,
    audience: Audience::Minor,
    cites: &[c(8, "工作在30-3000MHz频段且最大发射功率不大于25瓦")],
    tool: Some(Tool::Compliance),
  },
  Item {
    id: "frequency-coordination",
    group: Group::Parameters,
    audience: Audience::Everyone,
    cites: &[c(15, "次要业务"), c(15, "开展必要的频率协调")],
    tool: None,
  },
  Item {
    id: "licence-fields",
    group: Group::Parameters,
    audience: Audience::Everyone,
    cites: &[
      c(19, "电台呼号、台址/设置区域、使用频率、发射功率"),
      c(31, "应当符合业余无线电台执照载明的事项和要求"),
    ],
    tool: None,
  },
  Item {
    id: "radiation",
    group: Group::Parameters,
    audience: Audience::Everyone,
    cites: &[c(34, "防止无线电波发射产生的电磁辐射污染环境")],
    tool: Some(Tool::RfExposure),
  },
  // ---- 受理与审批（第十三、十四、十七条）----
  Item {
    id: "supplements",
    group: Group::Review,
    audience: Audience::Everyone,
    cites: &[
      c(13, "5个工作日内一次性告知申请人需要补正的全部内容"),
      c(13, "逾期不告知的，自收到申请材料之日起即为受理"),
    ],
    tool: None,
  },
  Item {
    id: "decision-time",
    group: Group::Review,
    audience: Audience::Everyone,
    cites: &[
      c(13, "自受理申请之日起30个工作日作出许可或者不予许可的决定"),
      c(17, "不计算在本办法第十三条第三款规定的审查期限内"),
      c(17, "应当将所需时间告知申请人"),
    ],
    tool: None,
  },
  Item {
    id: "technical-test",
    group: Group::Review,
    audience: Audience::Everyone,
    cites: &[c(14, "进行技术检测"), c(14, "不得收取任何费用")],
    tool: None,
  },
  // ---- 领证之后（第十八、二十至二十二条）----
  Item {
    id: "callsign-issued",
    group: Group::After,
    audience: Audience::Everyone,
    cites: &[
      c(18, "同时向申请人核发业余无线电台呼号"),
      c(18, "不再核发新的业余无线电台呼号"),
    ],
    tool: None,
  },
  Item {
    id: "renewal",
    group: Group::After,
    audience: Audience::Everyone,
    cites: &[
      c(20, "有效期不超过5年"),
      c(
        20,
        "应当在期限届满30个工作日前向作出许可决定的无线电管理机构申请更换业余无线电台执照",
      ),
    ],
    tool: Some(Tool::ExpiryReminder),
  },
  Item {
    id: "changes",
    group: Group::After,
    audience: Audience::Everyone,
    cites: &[c(21, "向作出许可决定的无线电管理机构申请办理变更手续")],
    tool: None,
  },
  Item {
    id: "closing",
    group: Group::After,
    audience: Audience::Everyone,
    cites: &[c(
      22,
      "自执照注销之日起60个工作日内拆除业余无线电台及天线等附属设备",
    )],
    tool: None,
  },
];

/// 某一组的清单项。
pub fn items_in(group: Group) -> impl Iterator<Item = &'static Item> {
  ITEMS.iter().filter(move |i| i.group == group)
}

/// 全部分组，按显示顺序。
pub const GROUPS: [Group; 6] = [
  Group::Conditions,
  Group::Authority,
  Group::Materials,
  Group::Parameters,
  Group::Review,
  Group::After,
];

/// 条文链接：法规原文库里的锚点（与 `ExplanationCard` 的引用链接同一格式）。
#[must_use]
pub fn article_href(article: u16) -> String {
  format!("/radio-law#{LAW_KEY}-{article}")
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{radio_law, registry};
  use std::collections::HashSet;

  /// 去掉全角 / 半角空白与换行：原文在款与款之间有真实换行，核对短语时不受它影响。
  fn squash(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
  }

  #[test]
  fn every_quoted_phrase_really_is_in_the_cited_article() {
    for item in ITEMS {
      assert!(!item.cites.is_empty(), "{}：至少要有一条出处", item.id);
      for cite in item.cites {
        let article = radio_law::article(LAW_KEY, cite.article)
          .unwrap_or_else(|| panic!("{}：办法第 {} 条不在法规库里", item.id, cite.article));
        assert!(
          squash(&article.text).contains(&squash(cite.phrase)),
          "{}：第 {} 条里找不到「{}」",
          item.id,
          cite.article,
          cite.phrase
        );
      }
    }
  }

  #[test]
  fn ids_are_unique_and_every_group_is_used() {
    let ids: HashSet<_> = ITEMS.iter().map(|i| i.id).collect();
    assert_eq!(ids.len(), ITEMS.len(), "id 有重复");
    for g in GROUPS {
      assert!(items_in(g).count() > 0, "{g:?} 没有任何一项");
    }
    // `GROUPS` 要覆盖 `ITEMS` 里出现过的所有分组，否则有的项永远不会被渲染。
    let shown: usize = GROUPS.iter().map(|g| items_in(*g).count()).sum();
    assert_eq!(shown, ITEMS.len());
  }

  #[test]
  fn items_follow_the_order_of_the_groups() {
    // 显示顺序 = 数组顺序：同一组必须连在一起，否则分组标题会重复出现。
    let mut seen: Vec<Group> = Vec::new();
    for item in ITEMS {
      if seen.last() != Some(&item.group) {
        assert!(
          !seen.contains(&item.group),
          "{}：{:?} 被拆成了两段",
          item.id,
          item.group
        );
        seen.push(item.group);
      }
    }
    assert_eq!(seen, GROUPS);
  }

  #[test]
  fn tools_are_real_routes() {
    for item in ITEMS {
      let Some(tool) = item.tool else { continue };
      let path = tool.href().split('#').next().unwrap();
      assert!(
        registry::by_path(path).is_some(),
        "{}：{} 不是站内路由",
        item.id,
        tool.href()
      );
    }
  }

  #[test]
  fn article_links_use_the_radio_law_anchor_format() {
    assert_eq!(article_href(7), "/radio-law#办法-7");
    // 与 `ExplanationCard` 的引用链接同一格式：`/radio-law#{law}-{number}`。
    for item in ITEMS {
      for cite in item.cites {
        assert!(radio_law::article(LAW_KEY, cite.article).is_some());
      }
    }
  }

  #[test]
  fn the_checklist_does_not_invent_materials_the_measures_do_not_ask_for() {
    // 办法没有要求提交天线形式 / 台址平面图 / 无线电设备购买凭证：清单里不得出现这些项。
    for forbidden in ["antenna", "floor-plan", "receipt", "photo-of-station"] {
      assert!(
        !ITEMS.iter().any(|i| i.id.contains(forbidden)),
        "{forbidden}"
      );
    }
    // 提交材料一组只有第十一、十二条两条出处，没有别的条款偷塞进来。
    for item in items_in(Group::Materials) {
      assert!(
        item
          .cites
          .iter()
          .all(|c| c.article == 11 || c.article == 12),
        "{}",
        item.id
      );
    }
  }

  #[test]
  fn minor_and_unit_items_are_marked() {
    let minors: Vec<_> = ITEMS
      .iter()
      .filter(|i| i.audience == Audience::Minor)
      .map(|i| i.id)
      .collect();
    let units: Vec<_> = ITEMS
      .iter()
      .filter(|i| i.audience == Audience::Unit)
      .map(|i| i.id)
      .collect();
    assert_eq!(minors, ["guardian", "minor-limits"]);
    assert_eq!(units, ["unit-documents"]);
  }
}
