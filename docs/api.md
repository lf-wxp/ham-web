# 后端接口

> `/api/*` 是前端专用的同源代理（随时可变）；第三方用 `/api/v1/*`。
> 路由与处理函数在 `crates/server`，契约的单一事实来源是 `crates/core/src/api_v1.rs`。

## 后端 API 与数据源

前端通过 `fetch` 调用以下同源接口，由 Axum 服务器代为请求上游数据（规避 CORS），并在服务端做内存缓存：

| 方法 | 路径 | 作用 | 上游数据源 | 服务端缓存 |
| --- | --- | --- | --- | --- |
| GET | `/healthz` | 健康检查（Docker HEALTHCHECK 使用） | — | — |
| GET | `/api/solar` | 太阳通量、A/K 指数、黑子数、各波段传播条件 | HamQSL | 300s |
| GET | `/api/alerts` | NOAA 空间天气警报（含 G/S/R 级别） | NOAA SWPC | 300s |
| GET | `/api/kp-forecast` | Kp 三日预报（约十天、每 3 小时一条，标明实测 / 估计 / 预报；服务端校验，G 级由前端按 NOAA 标度算） | NOAA SWPC | 600s |
| GET | `/api/xray` | GOES X 射线通量（6 小时时序、最新通量与耀斑级别 A/B/C/M/X） | NOAA SWPC | 300s |
| GET | `/api/spots` | DX Cluster 实时热点（最多 50 条） | DXWatch | 60s |
| GET | `/api/most-wanted` | DXCC 最稀有实体榜（前 30） | Club Log | 6h |
| GET | `/api/passes?lat=&lon=&min_elev=` | 卫星过境预报（TLE + SGP4，未来 24 小时） | Celestrak | TLE 缓存 6h |
| GET | `/api/sat-track?norad=&lat=&lon=&start=&secs=&step=` | 一颗业余卫星在一段时间内的方位 / 仰角 / 距离 / 径向速度（固定步长，供前端逐秒插值多普勒；`secs` ≤ 1800、`step` ≤ 30、`start` 在现在前 1 h 到后 2 d 内） | Celestrak | 复用 TLE 缓存 6h |
| GET | `/api/iss` | 国际空间站实时位置 | wheretheiss.at | 30s |
| GET | `/api/geocode?lat=&lon=` | 反向地理编码（国家/城市，中文） | BigDataCloud | 24h |
| GET | `/api/voacap?tx=&rx=&month=&ssn=&hour=` | 点对点 HF 传播预测（简化 VOACAP 模型，`hour` 为 UTC 时刻，省略则按最佳时段） | 本地计算 | 6h |
| GET | `/api/psk-reporter?callsign=` | 数字模式接收报告（谁收到了我） | PSK Reporter | 60s |
| GET | `/api/rbn?callsign=` | CW / RTTY 信标台接收报告 | Reverse Beacon Network | 30s |
| GET | `/api/callsign?callsign=` | 呼号查询（姓名 / 网格 / QTH，日志录入自动补全） | Callook / HamQTH | 7d |
| GET | `/api/sota?ref=` | SOTA 山峰详情（名称 / 海拔 / 积分 / 位置） | SOTA API | 30d |
| GET | `/api/pota?ref=` | POTA 公园详情（名称 / 实体 / 网格 / 位置） | POTA API | 30d |
| GET | `/api/repeaters?country=` | 中继台列表（频率 / 频差 / 亚音 / 城市） | RepeaterBook | 7d |
| GET | `/api/push/vapid-public-key` | 分发 Web Push 的 VAPID 公钥（前端订阅用） | 本地 | — |
| POST | `/api/push/subscribe` | 上报 Web Push 订阅（含每日提醒时间），持久化到 `PUSH_STORE` | 本地 | — |
| POST | `/api/push/unsubscribe` | 删除 Web Push 订阅 | 本地 | — |
| POST | `/api/push/review-count` | 同步「今日待复习数」（错题 + 卡片），供每日提醒推送附带数量 | 本地 | — |

> **关于 `/api/callsign` 的覆盖范围**：上游 Callook 只覆盖**美国 / 加拿大**呼号；其他国家和地区需要在服务端配置 `HAMQTH_USER` / `HAMQTH_PASS` 才会返回姓名 / QTH / 网格。两者都拿不到时，接口退回**内置 DXCC 前缀库**，只返回国家 / 地区（响应里的 `source` 为 `DXCC`），**不会**凭空生成网格或 QTH。状态码：`200` 有结果、`400` 呼号非法、`404` 前缀无法识别（多为拼写错误）、`503` 仅表示服务内部异常。
>
> 只启动前端（`cargo make dev`，仅 3000 端口）时 `/api/*` 无处转发，请求会失败；调试这些接口请用 `cargo make dev-full`（后端 3001 + 前端 3000 代理）。

其余 `/api/*` 之外的路径均由静态文件服务处理：命中文件直接返回，未命中且不带扩展名的路径回退到 `index.html`（SPA 路由）。

### 开放 API（`/api/v1/*`）

上表是**前端专用的同源代理**，接口随时可变。第三方请使用与之物理隔离、向后兼容的 `/api/v1/*`（文档页 `/developers`，OpenAPI 3.1：`/api/v1/openapi.json`）。首批仅开放纯计算与静态参考数据，无上游依赖：

| 路径 | 作用 |
| --- | --- |
| `GET /api/v1/dxcc` | DXCC 实体列表（`limit` 1–200 + `cursor` 游标分页） |
| `GET /api/v1/dxcc/lookup?callsign=` | 呼号 → DXCC 实体、CQ / ITU 分区与呼号结构 |
| `GET /api/v1/bands` | 频谱波段划分表 |
| `GET /api/v1/grid/to-latlon` · `from-latlon` · `distance` | Maidenhead 网格换算与大圆距离 / 方位角 |
| `GET /api/v1/propagation/muf` | 点对点 HF 传播预测（简化 VOACAP） |
| `GET /api/v1/units/convert` | dBm ↔ mW / W、dB ↔ 倍率、MHz ↔ 波长 |
| `GET /api/v1/filter/design` | Butterworth LC 滤波器设计（低通 / 高通 / 带通 / 带阻，1–9 阶） |
| `GET /api/v1/link-budget` | 自由空间链路预算（Friis） |
| `GET /api/v1/status` · `openapi.json` | 服务状态 / OpenAPI 文档 |

约定：成功 `{ "data", "meta" }`、失败 `{ "error": { "code", "message" } }`；带 `ETag`（`If-None-Match` → 304）；`/api/v1/*` 开放 CORS（内部 `/api/*` 保持同源）；独立配额——匿名 30 次/分/IP，携带 `Authorization: Bearer <key>` 600 次/分，超限返回 429 + `Retry-After`。契约单一事实来源是 `crates/core/src/api_v1.rs`，文档页、`openapi.json` 与服务端路由测试共用，不会漂移。

> 依赖 `/api/*` 的页面：`/dashboard`、`/solar`、`/dx-spots`、`/most-wanted`、`/satellites`、`/grid-map`、`/portable-map`、`/log`、`/repeater`、`/portable`、`/psk-reporter`、`/rbn` 与首页传播卡片；纯静态托管（无后端）时这些页面降级为「数据暂不可用」。其余专题页与题库页均为本地静态数据或纯前端计算（如 `/tools`、`/grayline`、`/morse`、`/sdr-map`、`/events`），无网络也完全可用。

