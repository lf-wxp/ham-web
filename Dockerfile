# syntax=docker/dockerfile:1.7
#
# 业余无线电执照考试模拟 —— 多阶段构建
#
#   docker build -t ham-web .
#   docker run --rm -p 3000:3000 ham-web
#
# 构建参数：
#   SITE_URL        站点地址（写入 Open Graph 与 sitemap.xml），默认 https://ham.onlyxp.me
#   TRUNK_VERSION   Trunk 版本
#   REBUILD_DATASET 设为 1 时在构建阶段从远程 CSV 重新生成题库（默认使用仓库内已提交的 JSON）

ARG RUST_VERSION=1

# ───────────────────────────── 构建阶段 ─────────────────────────────
FROM rust:${RUST_VERSION}-bookworm AS builder

ARG TARGETARCH
ARG TRUNK_VERSION=0.21.14
ARG SITE_URL=https://ham.onlyxp.me
ARG REBUILD_DATASET=0

RUN rustup target add wasm32-unknown-unknown

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

WORKDIR /src
COPY . .

# 工具链 → 可选重建题库 → 前端 → 构建后处理（sw.js / sitemap）→ 服务器
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/target \
    --mount=type=cache,target=/root/.cache/trunk \
    set -eux; \
    cargo build --release -p ham-exam-tools -p ham-exam-server; \
    if [ "${REBUILD_DATASET}" = "1" ]; then ./target/release/ham-exam-tools dataset; fi; \
    ./target/release/ham-exam-tools icons; \
    (cd crates/app && trunk build --release); \
    ./target/release/ham-exam-tools postbuild --dist dist --site-url "${SITE_URL}"; \
    mkdir -p /out; \
    cp ./target/release/ham-exam-server /out/ham-exam-server; \
    cp -r dist /out/dist

# ───────────────────────────── 运行阶段 ─────────────────────────────
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

LABEL org.opencontainers.image.title="ham-web" \
      org.opencontainers.image.description="业余无线电执照考试模拟（Rust + Leptos，PWA）" \
      org.opencontainers.image.source="https://github.com/lf-wxp/ham-web" \
      org.opencontainers.image.licenses="MIT"

WORKDIR /app
COPY --from=builder --chown=nonroot:nonroot /out/ham-exam-server /app/ham-exam-server
COPY --from=builder --chown=nonroot:nonroot /out/dist /app/dist

ENV HOST=0.0.0.0 \
    PORT=3000 \
    DIST_DIR=/app/dist \
    RUST_LOG=info,tower_http=warn

EXPOSE 3000
USER nonroot

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD ["/app/ham-exam-server", "healthcheck"]

ENTRYPOINT ["/app/ham-exam-server"]
