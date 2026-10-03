//! 开放 API 文档页：接入约定、配额、端点清单与错误码。
//!
//! 内容全部来自 `ham_web_core::api_v1` 的契约表（与服务端路由、`openapi.json` 同源），
//! 本页纯静态，不依赖后端即可阅读；点击示例需要后端在线。

use ham_web_core::api_v1::{
  ANON_LIMIT_PER_MIN, CACHE_MAX_AGE_SECS, ENDPOINTS, ERROR_CODES, KEYED_LIMIT_PER_MIN,
};
use leptos::prelude::*;

use crate::components::common::{
  BulletSection, ConceptsSection, KnowledgePage, SectionCard, TableSection,
};
use crate::i18n::{t, tf};
use crate::util::set_title;

/// 接入约定。
const CONVENTIONS: &[(&str, &str)] = &[
  (
    "基础地址",
    "同站点域名下的 /api/v1/*；全部为 GET 请求，返回 JSON。",
  ),
  (
    "成功响应",
    "{ \"data\": …, \"meta\": { \"source\": … } }；列表接口的 meta 带 total 与 next_cursor。",
  ),
  (
    "错误响应",
    "{ \"error\": { \"code\": …, \"message\": … } }，code 见下方错误码表。",
  ),
  (
    "跨域 CORS",
    "对任意来源开放（只读 GET），网页前端可直接 fetch，无需自建代理。",
  ),
  (
    "条件请求",
    "响应带 ETag；再次请求带 If-None-Match，内容未变化时返回 304，不消耗流量。",
  ),
  (
    "版本策略",
    "v1 发布后向后兼容：只增字段、不改含义；破坏性变更会升到 v2 并保留 v1 一段时间。",
  ),
];

/// 页面要点。
const NOTES: &[&str] = &[
  "首批只开放纯计算与静态参考数据，不依赖任何上游服务，结果稳定、可放心做长时间缓存。",
  "引用型接口（太阳活动、中继台、POTA / SOTA）会在复用服务端缓存后陆续开放，并保证缓存 TTL 不低于站内接口，避免放大对上游的压力。",
  "传播预测使用简化 VOACAP 模型，适合估算趋势与规划操作时段，不等同于专业电离层预测软件。",
  "纯静态托管（无后端）时本页文档仍可阅读，但接口不可用。",
];

/// 错误码表头。
const ERROR_HEADERS: &[&str] = &["HTTP", "code", "说明"];

#[component]
pub fn DevelopersPage() -> impl IntoView {
  set_title(&t("开放 API"));
  view! {
    <KnowledgePage title=t("开放 API") subtitle=t("版本化 · 可跨域 · 纯计算与静态数据")>
      <ConceptsSection title="接入约定" items=CONVENTIONS ui=true />

      <SectionCard title=t("配额与鉴权")>
        <div class="space-y-2 p-4 text-sm text-muted-foreground">
          <p>
            {move || {
              tf(
                "匿名访问：每 IP 每分钟 {} 次。请求头携带 Authorization: Bearer <key> 后提升到每分钟 {} 次。",
                &[&ANON_LIMIT_PER_MIN.to_string(), &KEYED_LIMIT_PER_MIN.to_string()],
              )
            }}
          </p>
          <p>
            {move || {
              tf(
                "key 只用于区分配额档位与统计，不做敏感鉴权；成功响应的缓存时长为 {} 秒。需要 key 请联系站点维护者。",
                &[&CACHE_MAX_AGE_SECS.to_string()],
              )
            }}
          </p>
          <p>
            <a
              href="/api/v1/openapi.json"
              target="_blank"
              rel="noopener noreferrer"
              class="font-medium text-primary underline-offset-2 hover:underline"
            >
              {move || t("下载 OpenAPI 3.1 文档（openapi.json）")}
            </a>
          </p>
        </div>
      </SectionCard>

      <SectionCard title=t("接口清单")>
        <div class="divide-y">
          {ENDPOINTS
            .iter()
            .map(|e| {
              view! {
                <div class="space-y-2 px-4 py-3">
                  <div class="flex flex-wrap items-center gap-2">
                    <span class="rounded bg-primary/10 px-1.5 py-0.5 font-mono text-[11px] font-semibold text-primary">
                      "GET"
                    </span>
                    <code class="break-all font-mono text-sm font-medium">{e.path}</code>
                    <span class="rounded-full border px-2 py-0.5 text-[11px] text-muted-foreground">
                      {move || t(e.tag)}
                    </span>
                  </div>
                  <p class="text-sm text-muted-foreground">{move || t(e.summary)}</p>
                  {(!e.params.is_empty())
                    .then(|| {
                      view! {
                        <ul class="space-y-1">
                          {e
                            .params
                            .iter()
                            .map(|p| {
                              view! {
                                <li class="flex flex-wrap items-baseline gap-x-2 text-xs">
                                  <code class="font-mono font-semibold">{p.name}</code>
                                  <span class="text-muted-foreground">
                                    {format!("{} · ", p.schema)}
                                    {move || t(if p.required { "必填" } else { "可选" })}
                                  </span>
                                  <span class="text-muted-foreground">{move || t(p.desc)}</span>
                                </li>
                              }
                            })
                            .collect_view()}
                        </ul>
                      }
                    })}
                  <a
                    href=e.example
                    target="_blank"
                    rel="noopener noreferrer"
                    class="block break-all rounded-lg bg-muted/60 px-3 py-2 font-mono text-xs text-primary transition-colors hover:bg-muted"
                  >
                    {e.example}
                  </a>
                </div>
              }
            })
            .collect_view()}
        </div>
      </SectionCard>

      <TableSection title="错误码" headers=ERROR_HEADERS rows=ERROR_CODES min_width=560 />
      <BulletSection title="说明" items=NOTES ui=true />
    </KnowledgePage>
  }
}
