#!/bin/sh
# 输出解码 Worker 的 codec 名，每行一个（apt / sstv / … / wspr）。
#
# 唯一来源是 crates/<codec>-worker/Cargo.toml：新增一个 Worker 只需加这个目录，
# `scripts/build-decode-worker.sh`、`Makefile.toml` 与 `Dockerfile` 都从这里取清单，
# 不再各自维护一份。清单写错不会有编译错误，只会让对应页面在镜像里缺资源
# （`worker-load-failed`），所以只留一处来源。
set -eu

cd "$(dirname "$0")/.." || exit 1

found=0
# 排序固定成 `LC_ALL=C`：glob 顺序受 `LC_COLLATE` 影响，macOS（en_US.UTF-8）与镜像里的
# `C` 会给出不同顺序，进而让 `cargo build … $pkgs` 的参数顺序两端不同、轻微扰动缓存键。
for manifest in $(printf '%s\n' crates/*-worker/Cargo.toml | LC_ALL=C sort); do
  [ -f "$manifest" ] || continue
  dir=${manifest#crates/}
  dir=${dir%/Cargo.toml}
  codec=${dir%-worker}
  # 目录名 → 包名 → wasm 产物名是一条链（ham_web_<codec>_worker.wasm），断了就报出来
  name=$(awk -F'"' '/^name = /{print $2; exit}' "$manifest")
  if [ "$name" != "ham-web-${codec}-worker" ]; then
    echo "crates/${dir} 的包名是 ${name}，不符合 ham-web-${codec}-worker 的约定" >&2
    exit 1
  fi
  found=1
  echo "$codec"
done

if [ "$found" = 0 ]; then
  echo "未发现解码 Worker crate（crates/*-worker）" >&2
  exit 1
fi
