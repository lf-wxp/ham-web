#!/bin/sh
# 构建一个解码 Web Worker 的 wasm + 胶水到 public/<codec>-worker/。
#
# 用法：sh scripts/build-decode-worker.sh <codec>      （codec：apt / sstv / wspr / rtty / olivia / dominoex / navtex / lrpt / ft8）
#
# 九个 `[tasks.build-*-worker]` 共用这一份脚本：此前每个任务各复制 19 行，版本校验的报错文案
# 已经分叉（有的带安装提示有的没有）。wasm-bindgen CLI 版本必须与 Cargo.lock 里的 crate 一致，
# 装错不会编译失败，只会在运行时让胶水与 wasm 不匹配。
set -e

codec="${1:?用法：build-decode-worker.sh <codec>}"
case "$codec" in
  apt | sstv | wspr | rtty | olivia | dominoex | navtex | lrpt | ft8) ;;
  *)
    echo "未知解码器：$codec" >&2
    exit 1
    ;;
esac

WB_VERSION=$(awk -F'"' '/^name = "wasm-bindgen"$/{f=1; next} f && /^version = /{print $2; exit}' Cargo.lock)
if ! command -v wasm-bindgen >/dev/null 2>&1; then
  echo "未找到 wasm-bindgen CLI，请先执行 cargo make setup（或 cargo install wasm-bindgen-cli --version $WB_VERSION --locked）" >&2
  exit 1
fi
WB_INSTALLED=$(wasm-bindgen --version | awk '{print $2}')
if [ "$WB_INSTALLED" != "$WB_VERSION" ]; then
  echo "wasm-bindgen CLI 版本 $WB_INSTALLED 与 Cargo.lock 中的 $WB_VERSION 不一致，请执行 cargo install wasm-bindgen-cli --version $WB_VERSION --locked" >&2
  exit 1
fi

cargo build --profile wasm-release --target wasm32-unknown-unknown -p "ham-web-${codec}-worker"
mkdir -p "public/${codec}-worker"
wasm-bindgen --target no-modules --no-typescript --out-dir "public/${codec}-worker" --out-name "${codec}_worker" \
  "target/wasm32-unknown-unknown/wasm-release/ham_web_${codec}_worker.wasm"
