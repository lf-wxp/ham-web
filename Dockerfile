# syntax=docker/dockerfile:1.7
#
# 业余无线电执照考试模拟 —— 多阶段构建
#
#   docker build -t ham-web .
#   docker run --rm -p 3000:3000 ham-web
#
# 构建参数：
#   SITE_URL              站点地址（写入 Open Graph 与 sitemap.xml），默认 https://ham.onlyxp.me
#   RUST_VERSION          构建阶段基础镜像的 tag（默认 1.99.1，钉死可复现）
#   TRUNK_VERSION         Trunk 版本
#   REBUILD_DATASET       设为 1 时在构建阶段从远程 CSV 重新生成题库（默认使用仓库内已提交的 JSON）
#   WASM_BINDGEN_VERSION  wasm-bindgen CLI 版本；留空时自动取 Cargo.lock 中的版本
#
# 注意：`--mount=type=cache` 只存在于**当前 builder 的本地磁盘**，不会随 `cache-to`
# 导出 —— 本机构建靠它跳过依赖编译，CI（一次性 runner）每次都要重编。实测这些依赖
# 只占 1 分钟上下，占大头的是前端那 ~5 分钟不可缓存的编译，所以没有为 CI 改成
# cargo-chef 式的依赖分层（代价是 gha 缓存多占 3–4GB，收益只有 1.5–2 分钟）。
# 详见 README「Docker 部署 → 构建耗时与缓存」。

# 钉到具体版本而不是浮动的大版本 `1`：rust:1-bookworm 会随上游更新悄悄换工具链，
# 同一个提交隔一周重建可能得到不同的 wasm / 原生二进制。升级工具链时改这里，
# 与 Cargo.toml 的 `rust-version`（1.88）一起看；.github/workflows/check.yml 里的 toolchain
# 也要同步到这个版本（CI 与镜像用不同版本时，lint 结果会不一致）。
ARG RUST_VERSION=1.99.1

# wasm-bindgen CLI 版本需与 Cargo.lock 中的 wasm-bindgen crate 版本一致，
# 否则解码 Worker 生成的胶水代码与 wasm 不匹配（运行时报 "cannot read properties"）。
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

# wasm-bindgen CLI：解码 Worker 的胶水代码由它生成（Trunk 自带的那份不对外暴露）
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/opt/wasm-bindgen-target \
    set -eux; \
    wb="${WASM_BINDGEN_VERSION:-$(awk -F'"' '/^name = "wasm-bindgen"$/{f=1; next} f && /^version = /{print $2; exit}' Cargo.lock)}"; \
    test -n "${wb}" || { echo "无法从 Cargo.lock 解析 wasm-bindgen 版本" >&2; exit 1; }; \
    cargo install wasm-bindgen-cli --version "${wb}" --locked \
      --target-dir /opt/wasm-bindgen-target; \
    actual="$(wasm-bindgen --version | awk '{print $2}')"; \
    [ "${actual}" = "${wb}" ] || { echo "PATH 上的 wasm-bindgen CLI 是 ${actual}，不是刚安装的 ${wb}" >&2; exit 1; }

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

# 工具链 → 可选重建题库 → 图标 → 语言包 → 解码 Worker → 前端 → 构建后处理（sw.js / sitemap）
#
# 顺序与 `cargo make build-web` 保持一致（见 Makefile.toml 的依赖链）：
#
#   * 语言包必须在 trunk build 之前生成 —— index.html 把 public/data/i18n 拷进产物，
#     晚一步拷到的就是旧包（`check-i18n` 会红，但镜像不经过那道门禁）；不生成则
#     en / es 用户会在非内嵌域看到中文（构建期生成的静态表只含 zh 全量与 common / shell）。
#   * 解码 Worker 同样必须在 trunk build 之前生成（清单来自 crates/*-worker，本文件不列名字）：
#     index.html 用 <link data-trunk rel="copy-dir"> 引用 public/*-worker/，这些目录里的
#     *_worker.js / *_bg.wasm 由 wasm-bindgen 生成、且被 .gitignore 排除，
#     漏掉任何一个都会让对应页面在容器里缺资源（/apt-decode、/sstv-decode、
#     /wspr-decode、/rtty-decode、/olivia-decode、/dominoex-decode、/navtex-decode、
#     /lrpt-decode、/ft8-decode、/hell-decode、/wefax-decode、/tools#soft-tnc 的 afsk1200）。
#
# 新增 / 删除一个解码 Worker 时本文件**不需要改**：codec 清单由脚本扫描 crates/*-worker
# 得出（scripts/decode-workers.sh），构建与产物断言都用同一份清单。
# 规范见 docs/decode-worker-protocol.md「wasm 胶水契约 → Dockerfile 里的 worker 构建」。
#
# 收尾对产物做一次存在性断言：上面这些生成物少任何一个，镜像照样能构建成功、
# 直到运行时才暴露（页面缺资源 / 语言包里是旧译文）。断言让它在构建期就红，
# 而不是等到部署后才发现——这是 `cargo make check` 的门禁在 docker build 里的补位。
#
# 内存：`wasm-release`（opt-level="z" + codegen-units=1）编译前端时单个 rustc 进程
# 峰值 6–8GB，4GiB/8GiB 的 Docker 虚拟机会在这一步被 OOM 杀掉（报错只有 BuildKit 的
# "cannot allocate memory"）。本地建议 `colima start --memory 16 --cpu 4`。
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/target \
    --mount=type=cache,target=/root/.cache/trunk \
    set -eux; \
    workers="$(sh scripts/decode-workers.sh)"; \
    test -n "${workers}" || { echo "未发现解码 Worker crate（crates/*-worker）" >&2; exit 1; }; \
    cargo build --release -p ham-web-tools -p ham-web-server; \
    if [ "${REBUILD_DATASET}" = "1" ]; then ./target/release/ham-web-tools dataset; fi; \
    ./target/release/ham-web-tools icons; \
    ./target/release/ham-web-tools i18n-pack; \
    sh scripts/build-decode-worker.sh; \
    (cd crates/app && trunk build --release); \
    ./target/release/ham-web-tools postbuild --dist dist --site-url "${SITE_URL}"; \
    for p in \
      dist/index.html \
      dist/license.html \
      dist/licenses/LICENSE \
      dist/licenses/GPL-3.0-only.txt \
      dist/licenses/MIT-lrpt-encoder.txt \
      dist/licenses/MIT-olivia-python.txt \
      dist/licenses/MIT-ft8_lib.txt \
      dist/licenses/BSD-3-Clause-kissfft.txt \
      dist/sw.js \
      dist/sitemap.xml \
      dist/decode-worker.js \
      dist/dxcc-entities.bin \
      dist/cn-provinces.bin \
      dist/data/i18n/en.json \
      dist/data/i18n/es.json \
      dist/data/knowledge-i18n/en.json \
      dist/data/knowledge-i18n/es.json \
      dist/data/glossary/basics.json; do \
      test -s "${p}" || { echo "缺少构建产物（或为空）：${p}" >&2; exit 1; }; \
    done; \
    for w in ${workers}; do \
      for f in "${w}_worker.js" "${w}_worker_bg.wasm"; do \
        p="dist/${w}-worker/${f}"; \
        test -s "${p}" || { echo "缺少构建产物（或为空）：${p}" >&2; exit 1; }; \
      done; \
    done; \
    mkdir -p /out/data; \
    cp ./target/release/ham-web-server /out/ham-web-server; \
    cp -r dist /out/dist

# ───────────────────────────── 运行阶段 ─────────────────────────────
#
# 基础镜像用的是 tag 而不是 digest：`distroless` 的 `nonroot` tag 会随上游重建而漂移
# （glibc / ca-certificates 的小版本变了，同一个提交重建出的镜像就不完全一致）。构建阶段
# 已经钉了 `RUST_VERSION`，这里若要完全可复现，得改成
# `gcr.io/distroless/cc-debian12@sha256:<digest>`；代价是升级基础镜像要人工改 digest，
# 所以维持 tag + 在发布记录里登记构建时间。glibc 只与二进制的最低版本要求有关，
# `cc-debian12` + glibc 2.36 对本站的 `x86_64-unknown-linux-gnu` 产物够用。
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

LABEL org.opencontainers.image.title="ham-web" \
      org.opencontainers.image.description="业余无线电执照考试模拟（Rust + Leptos，PWA）" \
      org.opencontainers.image.source="https://github.com/lf-wxp/ham-web" \
      org.opencontainers.image.licenses="MIT AND GPL-3.0-only"

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
