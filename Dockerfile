# syntax=docker/dockerfile:1.7
#
# 业余无线电执照考试模拟 —— 多阶段构建
#
#   docker build -t ham-web .
#   docker run --rm -p 3000:3000 ham-web
#
# 构建参数：
#   SITE_URL              站点地址（写入 Open Graph 与 sitemap.xml），默认 https://ham.onlyxp.me
#   TRUNK_VERSION         Trunk 版本
#   REBUILD_DATASET       设为 1 时在构建阶段从远程 CSV 重新生成题库（默认使用仓库内已提交的 JSON）
#   WASM_BINDGEN_VERSION  wasm-bindgen CLI 版本；留空时自动取 Cargo.lock 中的版本

ARG RUST_VERSION=1

# wasm-bindgen CLI 版本需与 Cargo.lock 中的 wasm-bindgen crate 版本一致，
# 否则 APT 解码 Worker 生成的胶水代码与 wasm 不匹配（运行时报 "cannot read properties"）。
#
# 默认为空 → 构建时从 Cargo.lock 提取（与 `cargo make setup` / `build-worker` 一致），
# 这样升级 wasm-bindgen 后不会出现「本机与镜像装了两个不同版本」的漂移。
ARG WASM_BINDGEN_VERSION=

# ───────────────────────────── 构建阶段 ─────────────────────────────
FROM rust:${RUST_VERSION}-bookworm AS builder

ARG TARGETARCH
ARG WASM_BINDGEN_VERSION
ARG TRUNK_VERSION=0.21.14
ARG SITE_URL=https://ham.onlyxp.me
ARG REBUILD_DATASET=0

RUN rustup target add wasm32-unknown-unknown

WORKDIR /src
# 先只拷贝锁文件：wasm-bindgen CLI 的版本要与 crate 版本一致，而整份源码这一层还用不到
COPY Cargo.lock ./

# wasm-bindgen CLI：APT 解码 Worker 由它生成 worker.js（Trunk 自带的那份不对外暴露）
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/opt/wasm-bindgen-target \
    set -eux; \
    wb="${WASM_BINDGEN_VERSION:-$(awk -F'"' '/^name = "wasm-bindgen"$/{f=1; next} f && /^version = /{print $2; exit}' Cargo.lock)}"; \
    test -n "${wb}" || { echo "无法从 Cargo.lock 解析 wasm-bindgen 版本" >&2; exit 1; }; \
    cargo install wasm-bindgen-cli --version "${wb}" --locked \
      --target-dir /opt/wasm-bindgen-target; \
    wasm-bindgen --version

# 安装 Trunk 预编译二进制（Tailwind、wasm-bindgen、wasm-opt 由 Trunk 在构建时自动下载）
RUN set -eux; \
    case "${TARGETARCH:-amd64}" in \
      amd64) arch=x86_64 ;; \
      arm64) arch=aarch64 ;; \
      *) echo "unsupported arch ${TARGETARCH}"; exit 1 ;; \
    esac; \
    curl -fsSL "https://github.com/trunk-rs/trunk/releases/download/v${TRUNK_VERSION}/trunk-${arch}-unknown-linux-gnu.tar.gz" \
      | tar -xz -C /usr/local/bin trunk; \
    trunk --version

COPY . .

# 工具链 → 可选重建题库 → 图标 → APT Worker → 前端 → 构建后处理（sw.js / sitemap）
#
# APT Worker 必须在 trunk build 之前生成：index.html 用 <link data-trunk rel="copy-dir">
# 引用 public/apt-worker/，该目录在 trunk 启动时就要存在。
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/target \
    --mount=type=cache,target=/root/.cache/trunk \
    set -eux; \
    cargo build --release -p ham-web-tools -p ham-web-server; \
    if [ "${REBUILD_DATASET}" = "1" ]; then ./target/release/ham-web-tools dataset; fi; \
    ./target/release/ham-web-tools icons; \
    cargo build --profile wasm-release --target wasm32-unknown-unknown -p ham-web-apt-worker; \
    mkdir -p public/apt-worker; \
    wasm-bindgen --target no-modules --no-typescript \
      --out-dir public/apt-worker --out-name apt_worker \
      target/wasm32-unknown-unknown/wasm-release/ham_web_apt_worker.wasm; \
    (cd crates/app && trunk build --release); \
    ./target/release/ham-web-tools postbuild --dist dist --site-url "${SITE_URL}"; \
    mkdir -p /out/data; \
    cp ./target/release/ham-web-server /out/ham-web-server; \
    cp -r dist /out/dist

# ───────────────────────────── 运行阶段 ─────────────────────────────
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

LABEL org.opencontainers.image.title="ham-web" \
      org.opencontainers.image.description="业余无线电执照考试模拟（Rust + Leptos，PWA）" \
      org.opencontainers.image.source="https://github.com/lf-wxp/ham-web" \
      org.opencontainers.image.licenses="MIT"

WORKDIR /app
COPY --from=builder --chown=nonroot:nonroot /out/ham-web-server /app/ham-web-server
COPY --from=builder --chown=nonroot:nonroot /out/dist /app/dist
# 可写数据目录：VAPID 密钥与推送订阅持久化（首次启动自动生成密钥）
COPY --from=builder --chown=nonroot:nonroot /out/data /app/data

ENV HOST=0.0.0.0 \
    PORT=3000 \
    DIST_DIR=/app/dist \
    PUSH_STORE=/app/data/push-subscriptions.json \
    RUST_LOG=info,tower_http=warn

EXPOSE 3000
USER nonroot

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD ["/app/ham-web-server", "healthcheck"]

ENTRYPOINT ["/app/ham-web-server"]
