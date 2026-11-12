#!/bin/sh
# 用 fldigi 4.2.13 的原版发端生成全部 MFSK / THOR 子模式的测试录音（中心 1500 Hz）到指定目录。
#
#   git clone --depth 1 https://github.com/crashdemons-external/fldigi-web /tmp/fldigi-web
#   FLDIGI_WEB=/tmp/fldigi-web sh gen_all.sh /tmp/fw
#
# 产出 `fw_<fldigi 模式名>.wav`，供 `ham-web-mfsk` / `ham-web-thor` 的 `real_recordings` 测试
# （环境变量 `MFSK_REAL_DIR` / `THOR_REAL_DIR`）使用。
set -eu
out=${1:?用法: sh gen_all.sh <输出目录>}
here=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$out"
text="the quick brown fox jumps over the lazy dog 1234567890"
for m in MFSK4 MFSK8 MFSK11 MFSK16 MFSK22 MFSK31 MFSK32 MFSK64 MFSK128 \
         THORM THOR4 THOR5 THOR8 THOR11 THOR16 THOR22 THOR25x4 THOR50x1 THOR50x2 THOR100; do
  node "$here/gen.mjs" "$m" "$text" 1500 "$out/fw_$m.wav"
done
