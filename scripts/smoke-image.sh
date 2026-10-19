#!/bin/sh
# 对构建出的镜像做运行时冒烟：起容器，检查健康检查端点、首页、SPA 回退与静态资源。
#
# 用法：sh scripts/smoke-image.sh <镜像名>
#
# Dockerfile 末尾的产物断言只证明文件在镜像层里，不证明 server 起得来、`DIST_DIR`
# 指对了目录、非 root 用户读得到 `dist/` —— 这些错了都能构建成功，只有部署后才暴露。
# 两条流水线（PR 的 check 与 main 的 build-docker）都调这个脚本，避免两处断言分叉。
set -eu

cd "$(dirname "$0")/.."

image=${1:?用法: sh scripts/smoke-image.sh <镜像名>}
name="ham-web-smoke-$$"
# 固定写 /tmp/xxx 会在共享机器或并发运行时互相覆盖。
index_file=$(mktemp)

cleanup() {
  docker rm -f "$name" >/dev/null 2>&1 || true
  rm -f "$index_file"
}
trap cleanup EXIT INT TERM

fail() {
  echo "$1" >&2
  docker logs "$name" >&2 || true
  exit 1
}

running() {
  [ "$(docker inspect -f '{{.State.Running}}' "$name" 2>/dev/null || echo false)" = "true" ]
}

# 每个请求都带超时：端口已监听但迟迟不应答时，没有超时的 curl 会一直挂着，
# 下面探活循环的 60 秒上限也就形同虚设。
http() { curl -sS -m 5 "$@"; }

# 宿主端口交给 Docker 随机分配：固定 3000 在开发者机器上常被占用，而在 macOS 的
# NAT 下端口冲突不报错 —— 容器照样起来，curl 却打到宿主上那个进程，探活给出的是
# 别人的响应（本机就踩过一次：`/` 返回 200 但没有 boot 容器）。
docker run -d --name "$name" -p 127.0.0.1::3000 "$image" >/dev/null
running || fail "容器启动后立即退出"

port=$(docker port "$name" 3000/tcp 2>/dev/null | head -1 | sed 's/.*://')
[ -n "$port" ] || fail "无法从 docker port 取得宿主端口"
base="http://127.0.0.1:${port}"

# 冷启动要解压 dist 里的 wasm：给 60 秒。healthz 由 server 自己应答、不读磁盘，
# 因此它绿只说明进程起来了，下面的静态资源检查才是关键。
i=0
while [ "$i" -lt 60 ]; do
  ! running && fail "容器在就绪前退出"
  http -f "${base}/healthz" >/dev/null 2>&1 && break
  i=$((i + 1))
  sleep 1
done
[ "$i" -lt 60 ] || fail "健康检查端点 60 秒内没有就绪"

# 首页必须是真正的应用壳：`ServeDir` 找不到 index.html 时也可能返回别的 200，
# 只有 boot 容器能区分「读到了 dist/index.html」和「回退到了别的东西」。
http -f "${base}/" -o "$index_file" || fail "GET / 失败"
grep -q 'id="boot"' "$index_file" || fail "GET / 的响应里没有 boot 容器（读到的不是 dist/index.html）"

# 无扩展名的前端路由回 index.html；带扩展名的缺失资源必须 404，不能被回退吞成 HTML。
code=$(http -o /dev/null -w '%{http_code}' "${base}/bands")
[ "$code" = 200 ] || fail "GET /bands 返回 ${code}（SPA 回退失效）"
code=$(http -o /dev/null -w '%{http_code}' "${base}/definitely-missing.js")
[ "$code" = 404 ] || fail "GET /definitely-missing.js 返回 ${code}（预期 404）"

# 生成物抽查：语言包、知识库译文。它们分别由 i18n-pack / postbuild 在构建阶段生成。
for p in /data/i18n/en.json /data/knowledge-i18n/en.json; do
  code=$(http -o /dev/null -w '%{http_code}' "${base}${p}")
  [ "$code" = 200 ] || fail "GET ${p} 返回 ${code}"
done

# 每个解码 Worker 的 wasm 都要真的能被 server 吐出来：清单与构建脚本、Dockerfile 的产物断言
# 同源（scripts/decode-workers.sh）。只抽查一个的话，新增的 codec 漏拷 / 漏构建时这里仍是绿的，
# 而 Dockerfile 的断言只证明文件在镜像层里，不证明 server 能 serve。
#
# 先赋值再遍历：`for w in $(cmd)` 里 `cmd` 失败不会触发 `set -e`，清单一空循环就悄悄跳过，
# 这一步等于没跑。
workers=$(sh scripts/decode-workers.sh)
[ -n "$workers" ] || fail "解码 Worker 清单为空（scripts/decode-workers.sh 没有输出）"
for w in $workers; do
  p="/${w}-worker/${w}_worker_bg.wasm"
  code=$(http -o /dev/null -w '%{http_code}' "${base}${p}")
  [ "$code" = 200 ] || fail "GET ${p} 返回 ${code}"
done

echo "镜像冒烟通过：${image}"
