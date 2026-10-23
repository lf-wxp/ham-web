# 开发运行与部署

> 上手的三条命令在 README；这里放本地开发细节、生产构建、Docker 参数、环境变量与 Web Push。

## 本地开发

```bash
cargo make dev            # 仅前端：http://127.0.0.1:3000 ，修改代码自动重新编译并刷新
cargo make dev-full       # 后端 API（3001）+ 前端（3000，代理 /api 到后端）
```

只调样式与题库功能时 `cargo make dev` 即可；需要调试实时数据页（仪表盘、太阳活动、DX 热点、卫星、网格地图）时用 `cargo make dev-full`。

开发模式下不注册 Service Worker，并会自动注销同源下残留的旧 SW，避免缓存和“发现新版本”弹窗干扰调试。开发产物输出到 `target/dev-dist/`，不会覆盖 release 的 `dist/`。

## 生产构建与运行

```bash
cargo make build          # 图标 + 前端（release）+ sw.js/sitemap + 服务器
cargo make serve          # 用 release 服务器托管 dist/：http://127.0.0.1:8080
```

构建产物位于 `dist/`；`dist/` 为纯静态文件，服务器仅额外提供 `/api/*` 代理与 `healthz`。若部署到纯静态托管（Nginx、CDN、GitHub Pages 等），只需把未命中的路径回退到 `index.html`，并为 `sw.js`、`manifest.json`、`changelog.json`、`index.html` 设置 `Cache-Control: no-cache`；此时依赖 `/api/*` 的实时数据页会降级为“数据暂不可用”，其余功能不受影响。


## Docker 部署

```bash
docker build -t ham-web .
docker run -d --name ham-web -p 3000:3000 \
  -v ham-web-data:/app/data \
  ham-web
# 或
cargo make docker-build && cargo make docker-run
```

构建参数：

| 参数 | 默认值 | 说明 |
| --- | --- | --- |
| `SITE_URL` | `https://ham.onlyxp.me` | 写入 Open Graph 与 `sitemap.xml` 的站点地址 |
| `REBUILD_DATASET` | `0` | 设为 `1` 时构建阶段从远程 CSV 重新生成题库 |
| `RUST_VERSION` | `1.99.1` | 构建阶段基础镜像的 tag。钉死具体版本，避免浮动的大版本 tag 让同一提交隔周重建出不同的二进制 |
| `TRUNK_VERSION` | `0.21.14` | Trunk 版本 |
| `WASM_BINDGEN_VERSION` | 空 | wasm-bindgen CLI 版本（构建全部解码 Worker，清单取自 `crates/*-worker`）；留空时自动取 `Cargo.lock` 中的版本 |

```bash
docker build --build-arg SITE_URL=https://exam.example.com -t ham-web .
```

运行时环境变量：`HOST`（默认 `0.0.0.0`）、`PORT`（默认 `3000`）、`DIST_DIR`（默认 `/app/dist`）、`PUSH_STORE`（Web Push 持久化文件，默认 `/app/data/push-subscriptions.json`）、`VAPID_PRIVATE_KEY` / `VAPID_SUBJECT`（可选，见下文）、`HAMQTH_USER` / `HAMQTH_PASS`（可选，呼号查询的国际数据源，不设则仅覆盖美加 + 内置 DXCC 回退）、`RUST_LOG`。

上游与请求相关的调优变量（均有合理默认值，一般无需设置）：

| 变量 | 默认值 | 说明 |
| --- | --- | --- |
| `UPSTREAM_CONNECT_TIMEOUT_SECS` | `5` | 上游建连（含 DNS 与 TLS 握手）超时；防止上游挂起占满阻塞线程池 |
| `UPSTREAM_TIMEOUT_SECS` | `15` | 上游请求端到端超时（从 DNS 到读完响应体） |
| `REQUEST_TIMEOUT_SECS` | `60` | 单个请求的兜底超时，超时返回 `504` |
| `RATE_LIMIT_PER_MIN` | `120` | `/api/*` 每 IP 每分钟限流阈值 |
| `API_V1_RATE_LIMIT_PER_MIN` | `30` | 开放 API `/api/v1/*` 匿名访问每 IP 每分钟配额 |
| `API_KEYS` | 空 | 开放 API 的 key（逗号分隔）；请求头 `Authorization: Bearer <key>` 命中后配额提升到 600 次/分，仅区分配额档位 |
镜像基于 `gcr.io/distroless/cc-debian12:nonroot`，以非 root 用户运行，内置健康检查（`/healthz`）。`/app/data` 目录已内置为可写，建议挂载 volume 持久化推送订阅与 VAPID 密钥。

### 构建耗时与内存（实测）

arm64 / 4 vCPU / 16GiB 的 Docker 虚拟机下，一次 `docker build` 的耗时构成：

| 阶段 | 耗时 | 缓存形态 |
| --- | --- | --- |
| 拉基础镜像 + 编译安装 `wasm-bindgen-cli` | ~1 min | 镜像层，命中即整层跳过 |
| 下载 Trunk 预编译二进制 | ~5 s | 镜像层 |
| 编译第三方依赖（原生 tools/server + wasm 前端与十二个 Worker） | ~1 min | `--mount=type=cache` |
| 编译前端 `ham-web-app` | **~5 min** | 不可缓存（每次提交都在变） |
| Trunk 下载 Tailwind / wasm-opt | ~1 min | `--mount=type=cache` |
| 图标 / 语言包 / 十二个 Worker / postbuild + 产物断言 | < 1 min | — |

**内存要求**：`wasm-release`（`opt-level = "z"` + `codegen-units = 1`）编译前端时，单个 `rustc` 进程峰值约 6–8GB。Docker 虚拟机只给 4GiB / 8GiB 时会在这一步被 OOM 杀掉，而报错只有 BuildKit 的 `cannot allocate memory`（要加 `--progress=plain` 才看得到 `rustc … SIGKILL: kill`，容易误判成 Dockerfile 有问题）。建议：

```bash
colima start --memory 16 --cpu 4     # 或用 Docker Desktop 调大内存
```

**缓存边界**：Dockerfile 里的 `--mount=type=cache` 只存在于当前 builder 的本地磁盘，**不会随 `cache-to` 导出**。所以本机重复构建能靠它跳过依赖编译，而 GitHub Actions（一次性 runner）每次都要重编依赖、重下 Trunk 工具；`type=gha` 实际只缓存镜像层（基础镜像、wasm-bindgen CLI、Trunk 那几个层）。这部分不可缓存的量只占 1–2 分钟，占大头的始终是那 ~5 分钟的前端编译，因此没有为 CI 改用 cargo-chef 式的依赖分层 —— 收益仅 1.5–2 分钟，代价是 gha 缓存（配额 10GB）多占 3–4GB。

## Web Push 后台推送

订阅后即使页面关闭，也能在「备考计划 → 每日提醒时间」到点时收到浏览器系统通知（前提是浏览器允许通知、站点为 HTTPS）。完整链路：

1. 前端在通知中心（`/notifications`）点击「订阅后台推送」，用后端分发的 VAPID 公钥向浏览器 `PushManager` 订阅；
2. 订阅信息（含每日提醒时间换算出的 UTC 分钟数）`POST /api/push/subscribe` 上报后端；
3. 后端每分钟检查一次，到点用 VAPID 私钥签发 JWT、按 RFC 8291（`aes128gcm`）加密消息，推送到订阅 endpoint；`410/404` 时自动清理失效订阅；
4. 前端页面打开 / 复习完成时通过 `POST /api/push/review-count` 静默同步「今日待复习数」（错题 + 卡片），推送时一并带上具体数量。

**订阅接口鉴权**：`POST /api/push/subscribe` 与 `/unsubscribe` **默认匿名可写**，无需任何配置 —— 订阅者本就是浏览器里的匿名访客，而前端是共享的静态资源，无从持有服务端令牌，要求鉴权会让功能对所有人失效。

匿名写入的防护依赖：`/api/*` 每 IP 限流、`valid_endpoint` 的推送服务域名白名单、`MAX_SUBSCRIPTIONS` 总数上限，以及不提供订阅列表读取接口（覆盖 / 删除他人订阅需事先拿到对方那条不可枚举的 endpoint）。

设置 `PUSH_API_TOKEN` 后这两个接口会校验 `Authorization: Bearer <token>`，但**官方前端会因此无法订阅**，仅适用于自建前端注入令牌或脚本管理的场景；设置后启动日志会给出相应提示。若只是想收紧来源，更推荐在反向代理层限制这两个路径。

**VAPID 密钥**按以下优先级确定，无需手工生成：

1. 环境变量 `VAPID_PRIVATE_KEY`（base64url，32 字节 P-256 标量）—— 适合多副本 / 自托管固定密钥；
2. `PUSH_STORE` 文件中已持久化的私钥；
3. 都没有时，**首次启动自动生成**，并随订阅一起写入 `PUSH_STORE` 文件（重启复用，公钥稳定）。

```bash
# 可选：固定私钥与 subject（不设则自动生成）
docker run -d --name ham-web -p 3000:3000 \
  -v ham-web-data:/app/data \
  -e VAPID_PRIVATE_KEY='<base64url 32字节>' \
  -e VAPID_SUBJECT='mailto:you@example.com' \
  ham-web
```

> ⚠️ 浏览器只在安全上下文（HTTPS 或 `localhost`）开放 `PushManager`。本地用 `http://127.0.0.1` 可调试，线上必须走 HTTPS。纯静态托管（无后端）时该功能不可用，前端会自动降级为「未配置推送服务」。

