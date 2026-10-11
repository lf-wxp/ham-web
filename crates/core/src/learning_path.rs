//! 学习路径与认证路线图：从新手到 A/B/C 各类操作证的循序渐进指引，
//! 加上**专题依赖图（DAG）**与「答错回推前置专题」。
//!
//! 依赖图只声明「学 B 之前最好先会 A」的学习依赖，数组按拓扑序排列
//! （前置节点永远在前），单测钉住无环与前置可达。

/// 学习阶段（步骤）。
pub const LEARNING_PATH: &[(&str, &str)] = &[
  (
    "第 1 步 · 新手入门",
    "了解业余无线电是什么、能做什么，熟悉呼号、频段、简语与基本操作礼仪（见「新手入门」）。",
  ),
  (
    "第 2 步 · A 类",
    "掌握法规与基础操作，考取 A 类操作证，可申请设台、在 VHF/UHF 通联。",
  ),
  (
    "第 3 步 · B 类",
    "掌握短波与电波传播知识，取得 B 类后可在 HF 短波通联、参加 DX 与竞赛。",
  ),
  (
    "第 4 步 · C 类",
    "深入电路、天线与法规，取得 C 类后拥有更高功率权限，可从事中继台等技术工作。",
  ),
  (
    "贯穿始终 · 实战",
    "边学边练：抄收呼号、摩尔斯电码、数字模式，用通联日志记录每一次 QSO。",
  ),
];

/// 各级别重点。
pub const LEARNING_LEVELS: &[(&str, &str, &str)] = &[
  (
    "A 类",
    "法规、安全、基础操作",
    "执照申办 / 新手入门 / 操作实务",
  ),
  (
    "B 类",
    "传播、天线、短波",
    "传播与电离层 / 波段表 / 天线型式",
  ),
  (
    "C 类",
    "电路、数字模式、进阶",
    "电子电路基础 / 数字模式 / 功率放大器",
  ),
];

/// 备考建议。
pub const LEARNING_TIPS: &[&str] = &[
  "先刷题熟悉题型，再用「分类浏览」逐个知识点查漏补缺。",
  "用「备考计划」设定考试日期，系统安排每日任务。",
  "把易混淆点记入「错题本」，按间隔复习巩固。",
];

/// 专题依赖图的一个节点。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TopicNode {
  /// 专题页路由（如 `/regulations`）。
  pub route: &'static str,
  /// 专题名。
  pub name: &'static str,
  /// 覆盖的二级分类码（错题归因回推用；没列 = 错题不会回推到它，但仍可在图里当前置）。
  pub codes: &'static [&'static str],
  /// 前置节点路由（必须先于本节点）。
  pub prerequisites: &'static [&'static str],
}

/// 专题依赖图：数组**按拓扑序**排列（前置永远在前），单测钉住无环。
pub const TOPIC_GRAPH: &[TopicNode] = &[
  TopicNode {
    route: "/regulations",
    name: "法规与管理",
    codes: &["1.1.1", "1.2.1", "1.6.1", "1.6.2"],
    prerequisites: &[],
  },
  TopicNode {
    route: "/beginner",
    name: "新手入门",
    codes: &["1.1.2"],
    prerequisites: &[],
  },
  TopicNode {
    route: "/license-classes",
    name: "操作证类别与权限",
    codes: &["1.3.1", "1.3.2"],
    prerequisites: &["/regulations"],
  },
  TopicNode {
    route: "/bands",
    name: "频率与波段",
    codes: &["2.1.1", "2.2.1", "2.2.3"],
    prerequisites: &["/regulations"],
  },
  TopicNode {
    route: "/frequencies",
    name: "频率使用与划分",
    codes: &["2.1.2", "2.3.1"],
    prerequisites: &["/bands"],
  },
  TopicNode {
    route: "/antennas",
    name: "天线与馈线",
    codes: &["3.3.1", "3.3.2", "3.4.1", "3.4.2"],
    prerequisites: &["/bands"],
  },
  TopicNode {
    route: "/feedline",
    name: "馈线与驻波",
    codes: &["3.4.3", "3.4.4"],
    prerequisites: &["/antennas"],
  },
  TopicNode {
    route: "/antenna-theory",
    name: "天线理论",
    codes: &[],
    prerequisites: &["/antennas"],
  },
  TopicNode {
    route: "/propagation",
    name: "电波传播",
    codes: &["3.5.2", "3.5.3"],
    prerequisites: &["/bands"],
  },
  TopicNode {
    route: "/operating",
    name: "通联程序与规范",
    codes: &["3.1.1", "3.1.2", "3.2.1"],
    prerequisites: &["/beginner", "/regulations"],
  },
  TopicNode {
    route: "/rst",
    name: "信号报告 RST",
    codes: &["3.1.3"],
    prerequisites: &["/operating"],
  },
  TopicNode {
    route: "/digital-comms",
    name: "数字通信",
    codes: &["3.6.1", "3.6.2"],
    prerequisites: &["/bands", "/operating"],
  },
  TopicNode {
    route: "/satellites",
    name: "业余卫星",
    codes: &["3.8.1"],
    prerequisites: &["/bands", "/antennas"],
  },
  TopicNode {
    route: "/spectrum-admin",
    name: "频谱管理",
    codes: &["1.5.1", "1.5.2", "1.5.3"],
    prerequisites: &["/regulations"],
  },
  TopicNode {
    route: "/safety",
    name: "安全与电磁防护",
    codes: &["4.5.1", "4.5.2"],
    prerequisites: &["/regulations"],
  },
];

/// 按路由查节点。
#[must_use]
pub fn node(route: &str) -> Option<&'static TopicNode> {
  TOPIC_GRAPH.iter().find(|n| n.route == route)
}

/// 某一分类码对应的专题节点（错题回推的入口）。
#[must_use]
pub fn node_for_code(code: &str) -> Option<&'static TopicNode> {
  TOPIC_GRAPH.iter().find(|n| n.codes.contains(&code))
}

/// 沿前置链向上收集（图小，直接递归；`out` 保持拓扑序：前置先入）。
fn collect_prerequisites(i: usize, graph: &[TopicNode], out: &mut Vec<usize>) {
  if out.contains(&i) {
    return;
  }
  for pre in graph[i].prerequisites {
    if let Some(j) = graph.iter().position(|n| n.route == *pre) {
      collect_prerequisites(j, graph, out);
    }
  }
  out.push(i);
}

/// **答错回推**：给定错题的知识点码，返回「应当先复习的专题」，
/// 按学习顺序（拓扑序）排列、去重 —— 前置专题排在被依赖专题前面。
///
/// 例：错在「馈线与驻波」→ 先复习「频率与波段」「天线与馈线」，再回到「馈线与驻波」本身。
#[must_use]
pub fn suggested_reviews(wrong_codes: &[&str]) -> Vec<&'static TopicNode> {
  let mut closure: Vec<usize> = Vec::new(); // TOPIC_GRAPH 下标（天然拓扑序）
  for code in wrong_codes {
    if let Some(n) = node_for_code(code)
      && let Some(i) = TOPIC_GRAPH.iter().position(|x| x.route == n.route)
    {
      collect_prerequisites(i, TOPIC_GRAPH, &mut closure);
    }
  }
  closure.iter().map(|&i| &TOPIC_GRAPH[i]).collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_graph_is_acyclic_and_prerequisites_exist() {
    for (i, n) in TOPIC_GRAPH.iter().enumerate() {
      for pre in n.prerequisites {
        // 前置必须真实存在，且必须排在本节点之前（拓扑序 = 无环）。
        let pos = TOPIC_GRAPH
          .iter()
          .position(|x| x.route == *pre)
          .unwrap_or_else(|| panic!("{} 的前置 {} 不在图里", n.route, pre));
        assert!(pos < i, "{} 的前置 {} 排在其后 —— 有环", n.route, pre);
      }
    }
    // 路由与名字不能重。
    let mut routes: Vec<&str> = TOPIC_GRAPH.iter().map(|n| n.route).collect();
    let n = routes.len();
    routes.sort_unstable();
    routes.dedup();
    assert_eq!(routes.len(), n, "路由重复");
  }

  #[test]
  fn wrong_answers_push_back_prerequisites_in_order() {
    // 错在「馈线与驻波」（3.4.3）→ 回推是**传递闭包**：
    // 法规 → 频率与波段 → 天线与馈线 → 馈线与驻波（前置在前、本体在后）。
    let reviews = suggested_reviews(&["3.4.3"]);
    let routes: Vec<&str> = reviews.iter().map(|n| n.route).collect();
    assert_eq!(
      routes,
      vec!["/regulations", "/bands", "/antennas", "/feedline"],
      "前置在前、本体在后"
    );
    // 多个知识点去重合并：错在 RST 与 数字通信 → 并集按学习顺序。
    let reviews = suggested_reviews(&["3.1.3", "3.6.1"]);
    let routes: Vec<&str> = reviews.iter().map(|n| n.route).collect();
    assert!(routes.contains(&"/operating"), "{routes:?}");
    assert_eq!(
      routes.last(),
      Some(&"/digital-comms"),
      "最靠后的节点是它自己"
    );
    // 未收录的码不产出任何回推。
    assert!(suggested_reviews(&["9.9.9"]).is_empty());
  }
}
