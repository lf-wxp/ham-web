//! 火腿社区外链聚合：把国内外的论坛、问答与评测站点汇总成一张索引。
//!
//! 这是「社区」模块的第一阶段形态（ROADMAP §5.3-D 方案①）：只做外链聚合，
//! 零后端、零审核成本；自建社区需引入账号与审核体系，留待后续迭代。

/// 一个社区外链。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommunityLink {
  /// 站点名。
  pub name: &'static str,
  /// 分类（见 [`COMMUNITY_CATEGORIES`]）。
  pub category: &'static str,
  /// 一句话说明。
  pub desc: &'static str,
  /// 站点地址（外部链接）。
  pub url: &'static str,
}

/// 分类展示顺序。
pub const COMMUNITY_CATEGORIES: &[&str] = &["中文社区", "国际论坛", "问答与评测"];

/// 社区外链（链接指向各站点主页，均无需注册即可浏览）。
pub const COMMUNITY_LINKS: &[CommunityLink] = &[
  CommunityLink {
    name: "哈罗CQ火腿社区",
    category: "中文社区",
    desc: "国内规模最大的业余无线电论坛，器材、天线、通联与考试讨论一应俱全。",
    url: "https://www.hellocq.net/",
  },
  CommunityLink {
    name: "中国无线电协会业余无线电分会（CRAC）",
    category: "中文社区",
    desc: "官方机构，考试报名、操作证与电台执照政策的权威发布渠道。",
    url: "https://www.crac.org.cn/",
  },
  CommunityLink {
    name: "QRZ.com",
    category: "国际论坛",
    desc: "全球呼号数据库与论坛，火腿世界的「黄页」与日常交流地。",
    url: "https://www.qrz.com/",
  },
  CommunityLink {
    name: "eHam.net",
    category: "国际论坛",
    desc: "老牌论坛与设备评测库，选购收发信机前先看真实用户评价。",
    url: "https://www.eham.net/",
  },
  CommunityLink {
    name: "Reddit r/amateurradio",
    category: "国际论坛",
    desc: "英文社区，全球火腿的日常讨论、晒台与求助。",
    url: "https://www.reddit.com/r/amateurradio/",
  },
  CommunityLink {
    name: "DX Summit",
    category: "国际论坛",
    desc: "实时 DX 热点社区，配合本站「DX 实时热点」页使用。",
    url: "https://www.dxsummit.fi/",
  },
  CommunityLink {
    name: "Ham Radio Stack Exchange",
    category: "问答与评测",
    desc: "结构化问答社区，技术问题最可能找到准确、被验证过的答案。",
    url: "https://ham.stackexchange.com/",
  },
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn links_populated_and_categorized() {
    assert!(COMMUNITY_LINKS.len() >= 5);
    for l in COMMUNITY_LINKS {
      assert!(!l.name.is_empty());
      assert!(!l.desc.is_empty());
      assert!(l.url.starts_with("http"), "链接应为 URL：{}", l.url);
      assert!(
        COMMUNITY_CATEGORIES.contains(&l.category),
        "未知分类：{}",
        l.category
      );
    }
    // 每个分类都应有链接。
    for cat in COMMUNITY_CATEGORIES {
      assert!(
        COMMUNITY_LINKS.iter().any(|l| l.category == *cat),
        "分类无链接：{cat}"
      );
    }
  }
}
