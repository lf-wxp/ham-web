#!/bin/sh
# 构建解码 Web Worker 的 wasm + 胶水到 public/<codec>-worker/。
#
# 用法：sh scripts/build-decode-worker.sh [<codec> ...]
#       codec 取自 crates/*-worker（清单见 scripts/decode-workers.sh）；不传参则构建全部。
#
# `Makefile.toml` 的 `[tasks.build-workers]` 与 `Dockerfile` 都调无参版本，新增 codec 不必
# 改它们；单跑某个 codec 用有参版本。全部 worker 共用这一份脚本：此前每个 codec 各复制
# 19 行，版本校验的报错文案已经分叉。
# wasm-bindgen CLI 版本必须与 Cargo.lock 里的 crate 一致，装错不会编译失败，只会在运行时
# 让胶水与 wasm 不匹配。
# `-u`：后面有一堆 ${VAR} 展开（版本号、codec 名），变量名打错时应该立刻报错，
# 而不是拿空串继续往下跑。
set -eu

cd "$(dirname "$0")/.." || exit 1

if [ $# -eq 0 ]; then
  # 先赋值再 `set --`：`set -- $(cmd)` 自身永远返回 0，`set -e` 拦不住 `cmd` 的失败——
  # 清单脚本在后面的 crate 报错之前已经输出了前几个 codec，那样只会悄悄构建前几个、甚至 0 个
  # （再退化成编译 default-members，报错很误导）。赋值语句会带出命令替换的退出码。
  list=$(sh scripts/decode-workers.sh)
  # shellcheck disable=SC2086 # 清单按空白分词是预期行为
  set -- $list
fi

for codec in "$@"; do
  case "$codec" in
    "" | *[!a-z0-9]*)
      echo "未知解码器：$codec（可选：$(sh scripts/decode-workers.sh | tr '\n' ' ')）" >&2
      exit 1
      ;;
  esac
  if [ ! -f "crates/${codec}-worker/Cargo.toml" ]; then
    echo "未找到 crates/${codec}-worker/Cargo.toml（可选：$(sh scripts/decode-workers.sh | tr '\n' ' ')）" >&2
    exit 1
  fi
done

WB_VERSION=$(awk -F'"' '/^name = "wasm-bindgen"$/{f=1; next} f && /^version = /{print $2; exit}' Cargo.lock)
# 空版本号常见于「Cargo.lock 还没生成」或锁文件格式变化：这时下面那句「版本不一致」
# 会把原因误报成版本漂移，单独拦一次并说清是锁文件找不到。
if [ -z "$WB_VERSION" ]; then
  echo "未能在 Cargo.lock 找到 wasm-bindgen 的版本（锁文件缺失或格式变化），请先在仓库根执行一次 cargo fetch" >&2
  exit 1
fi
if ! command -v wasm-bindgen >/dev/null 2>&1; then
  echo "未找到 wasm-bindgen CLI，请先执行 cargo make setup（或 cargo install wasm-bindgen-cli --version $WB_VERSION --locked）" >&2
  exit 1
fi
WB_INSTALLED=$(wasm-bindgen --version | awk '{print $2}')
if [ "$WB_INSTALLED" != "$WB_VERSION" ]; then
  echo "wasm-bindgen CLI 版本 $WB_INSTALLED 与 Cargo.lock 中的 $WB_VERSION 不一致，请执行 cargo install wasm-bindgen-cli --version $WB_VERSION --locked" >&2
  exit 1
fi

pkgs=""
for codec in "$@"; do
  pkgs="$pkgs -p ham-web-${codec}-worker"
done
# 一次 cargo build 编全部（cargo 内部并行）；逐个 cargo 调用会让镜像构建多出十几次
# workspace 解析与锁竞争
# shellcheck disable=SC2086 # $pkgs 需要分词
cargo build --profile wasm-release --target wasm32-unknown-unknown $pkgs

for codec in "$@"; do
  mkdir -p "public/${codec}-worker"
  wasm-bindgen --target no-modules --no-typescript \
    --out-dir "public/${codec}-worker" --out-name "${codec}_worker" \
    "target/wasm32-unknown-unknown/wasm-release/ham_web_${codec}_worker.wasm"
done
