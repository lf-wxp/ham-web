//! 开放 API（`/api/v1/*`）的契约：端点清单、错误码、配额与 OpenAPI 3.1 文档。
//!
//! 服务端路由（`ham-web-server`）、`/developers` 文档页与 `openapi.json` 共用本表作为
//! **唯一事实来源**：文档不会与实现脱节 —— 服务端测试会断言这里登记的每个端点
//! 都真实可路由。
//!
//! 与前端专用的同源代理 `/api/*` 的区别：`/api/v1/*` 一经发布即向后兼容
//! （破坏性变更须升 `v2`），显式开放 CORS，并使用独立配额，避免第三方流量
//! 挤占前端。

use serde_json::{Map, Value, json};

/// 匿名访问配额：每 IP 每分钟请求数。
pub const ANON_LIMIT_PER_MIN: usize = 30;
/// 携带 `Authorization: Bearer <key>` 的配额：每 IP 每分钟请求数。
pub const KEYED_LIMIT_PER_MIN: usize = 600;
/// 成功响应的 `Cache-Control: max-age`（秒）。接口全部是纯计算 / 静态数据，可放心缓存。
pub const CACHE_MAX_AGE_SECS: u32 = 3600;

/// 一个查询参数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Param {
  /// 参数名。
  pub name: &'static str,
  /// 是否必填。
  pub required: bool,
  /// JSON Schema 类型：`string` / `number` / `integer`。
  pub schema: &'static str,
  /// 说明。
  pub desc: &'static str,
}

/// 一个 GET 端点。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Endpoint {
  /// 路径（含 `/api/v1` 前缀，不含查询串）。
  pub path: &'static str,
  /// 分类标签。
  pub tag: &'static str,
  /// 一句话说明。
  pub summary: &'static str,
  /// 查询参数。
  pub params: &'static [Param],
  /// 可直接点击的示例请求（含查询串）。
  pub example: &'static str,
}

const fn p(name: &'static str, required: bool, schema: &'static str, desc: &'static str) -> Param {
  Param {
    name,
    required,
    schema,
    desc,
  }
}

/// 全部端点（展示顺序）。
pub const ENDPOINTS: &[Endpoint] = &[
  Endpoint {
    path: "/api/v1/dxcc",
    tag: "参考数据",
    summary: "DXCC 实体列表（340 个现行实体，游标分页）",
    params: &[
      p("limit", false, "integer", "每页条数，1–200，默认 50"),
      p(
        "cursor",
        false,
        "string",
        "上一页响应 meta.next_cursor 的值",
      ),
    ],
    example: "/api/v1/dxcc?limit=5",
  },
  Endpoint {
    path: "/api/v1/dxcc/lookup",
    tag: "参考数据",
    summary: "呼号 → DXCC 实体、CQ / ITU 分区与呼号结构解析（最长前缀匹配）",
    params: &[p(
      "callsign",
      true,
      "string",
      "呼号，支持 /P、/MM 等斜杠后缀，如 BA1XX、VP2E/W1AW",
    )],
    example: "/api/v1/dxcc/lookup?callsign=BA1XX",
  },
  Endpoint {
    path: "/api/v1/bands",
    tag: "参考数据",
    summary: "频谱波段划分表（带号、波长、业余业务划分与使用状态）",
    params: &[],
    example: "/api/v1/bands",
  },
  Endpoint {
    path: "/api/v1/grid/to-latlon",
    tag: "网格计算",
    summary: "Maidenhead 网格 → 中心点经纬度",
    params: &[p("grid", true, "string", "4 或 6 位网格，如 FN31pr")],
    example: "/api/v1/grid/to-latlon?grid=FN31pr",
  },
  Endpoint {
    path: "/api/v1/grid/from-latlon",
    tag: "网格计算",
    summary: "经纬度 → 6 位 Maidenhead 网格",
    params: &[
      p("lat", true, "number", "纬度，-90–90"),
      p("lon", true, "number", "经度，-180–180（东经为正）"),
    ],
    example: "/api/v1/grid/from-latlon?lat=31.23&lon=121.47",
  },
  Endpoint {
    path: "/api/v1/grid/distance",
    tag: "网格计算",
    summary: "两个网格之间的大圆距离与双向方位角",
    params: &[
      p("from", true, "string", "起点网格（4 或 6 位）"),
      p("to", true, "string", "终点网格（4 或 6 位）"),
    ],
    example: "/api/v1/grid/distance?from=OM89&to=FN31",
  },
  Endpoint {
    path: "/api/v1/propagation/muf",
    tag: "传播预测",
    summary: "点对点 HF 传播预测：foF2 / MUF 与各波段可用性和可靠度（简化 VOACAP 模型）",
    params: &[
      p("tx", true, "string", "发射端网格"),
      p("rx", true, "string", "接收端网格"),
      p("month", false, "integer", "月份 1–12，默认 10"),
      p("ssn", false, "number", "太阳黑子数 0–400，默认 100"),
      p(
        "hour",
        false,
        "number",
        "UTC 时刻 0–24，可为小数；省略时按路径日照最佳情况估算",
      ),
    ],
    example: "/api/v1/propagation/muf?tx=OM89&rx=IO91&month=10&ssn=100&hour=12",
  },
  Endpoint {
    path: "/api/v1/status",
    tag: "元信息",
    summary: "服务状态与已开放的端点清单",
    params: &[],
    example: "/api/v1/status",
  },
  Endpoint {
    path: "/api/v1/openapi.json",
    tag: "元信息",
    summary: "OpenAPI 3.1 文档（由本契约表生成）",
    params: &[],
    example: "/api/v1/openapi.json",
  },
];

/// 错误码：`(HTTP 状态, code, 说明)`。
pub const ERROR_CODES: &[(&str, &str, &str)] = &[
  (
    "400",
    "invalid_parameter",
    "参数缺失，或格式 / 取值范围非法（message 中指明具体参数）",
  ),
  (
    "404",
    "not_found",
    "路径未定义，或呼号无法归属任何 DXCC 实体（如 /MM 海上移动）",
  ),
  (
    "429",
    "rate_limited",
    "超过配额；响应带 Retry-After（秒），到期后重试或改用 API key",
  ),
  ("500", "internal", "服务内部错误，可稍后重试"),
];

/// 生成 OpenAPI 3.1 文档。
#[must_use]
pub fn openapi() -> Value {
  let error_ref = json!({
    "content": {
      "application/json": {
        "schema": {
          "type": "object",
          "properties": {
            "error": {
              "type": "object",
              "properties": {
                "code": { "type": "string" },
                "message": { "type": "string" }
              }
            }
          }
        }
      }
    }
  });

  let mut paths = Map::new();
  for e in ENDPOINTS {
    let params: Vec<Value> = e
      .params
      .iter()
      .map(|p| {
        json!({
          "name": p.name,
          "in": "query",
          "required": p.required,
          "description": p.desc,
          "schema": { "type": p.schema },
        })
      })
      .collect();
    let mut bad = error_ref.clone();
    bad["description"] = json!("参数缺失或非法");
    let mut limited = error_ref.clone();
    limited["description"] = json!("超过配额，见 Retry-After");
    paths.insert(
      e.path.to_owned(),
      json!({
        "get": {
          "tags": [e.tag],
          "summary": e.summary,
          "parameters": params,
          "responses": {
            "200": { "description": "成功。除 openapi.json 外均为 {\"data\": …, \"meta\": {…}} 包装" },
            "304": { "description": "If-None-Match 命中，内容未变化" },
            "400": bad,
            "429": limited,
          }
        }
      }),
    );
  }

  json!({
    "openapi": "3.1.0",
    "info": {
      "title": "业余无线电开放 API",
      "version": "1.0.0",
      "description": format!(
        "纯计算与静态参考数据接口，无上游依赖。匿名每 IP 每分钟 {ANON_LIMIT_PER_MIN} 次；\
         携带 `Authorization: Bearer <key>` 每分钟 {KEYED_LIMIT_PER_MIN} 次。\
         v1 发布后向后兼容，破坏性变更会升到 v2。"
      ),
    },
    "servers": [{ "url": "/" }],
    "paths": paths,
    "components": {
      "securitySchemes": { "bearerAuth": { "type": "http", "scheme": "bearer" } }
    },
    "security": [{}, { "bearerAuth": [] }],
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn endpoints_are_well_formed_and_unique() {
    let mut seen = Vec::new();
    for e in ENDPOINTS {
      assert!(
        e.path.starts_with("/api/v1/"),
        "{} 必须在 /api/v1 下",
        e.path
      );
      assert!(!e.summary.is_empty() && !e.tag.is_empty());
      assert!(
        e.example.starts_with(e.path),
        "{} 的示例应以自身路径开头",
        e.path
      );
      assert!(!seen.contains(&e.path), "{} 重复", e.path);
      seen.push(e.path);
      for p in e.params {
        assert!(
          matches!(p.schema, "string" | "number" | "integer"),
          "{} 的参数 {} 类型非法",
          e.path,
          p.name
        );
        if p.required {
          assert!(
            e.example.contains(&format!("{}=", p.name)),
            "{} 的示例缺少必填参数 {}",
            e.path,
            p.name
          );
        }
      }
    }
  }

  #[test]
  fn openapi_lists_every_endpoint() {
    let doc = openapi();
    assert_eq!(doc["openapi"], "3.1.0");
    let paths = doc["paths"].as_object().expect("paths 应为对象");
    assert_eq!(paths.len(), ENDPOINTS.len());
    for e in ENDPOINTS {
      assert!(paths.contains_key(e.path), "{} 未出现在 OpenAPI 中", e.path);
    }
  }

  #[test]
  fn quotas_are_sane() {
    const { assert!(KEYED_LIMIT_PER_MIN > ANON_LIMIT_PER_MIN) };
    assert!(
      ERROR_CODES
        .iter()
        .any(|(s, c, _)| *s == "429" && *c == "rate_limited")
    );
  }
}
