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
for m in MFSK4 MFSK8 MFSK11 MFSK16 MFSK22 MFSK31 MFSK32 MFSK64 MFSK128 MFSK64L MFSK128L \
         THORM THOR4 THOR5 THOR8 THOR11 THOR16 THOR22 THOR25 THOR32 THOR44 THOR56 THOR25x4 THOR50x1 THOR50x2 THOR100; do
  node "$here/gen.mjs" "$m" "$text" 1500 "$out/fw_$m.wav"
done

# 图片录音（fldigi 原版发端发的 MFSK 图片，`fwpic_<名>.wav`，旁边的 `.rgb` 是原图）：给 `real_recordings` 的图片用例用。
# 命名：m<模式>_<图案>[_gray|_spp4]。
for spec in "MFSK16 32 24 color 8 bands m16_bands" "MFSK16 32 24 color 8 ramp m16_ramp" \
            "MFSK32 48 36 color 8 ramp m32_ramp" "MFSK64 48 36 color 8 ramp m64_ramp" \
            "MFSK128 48 36 color 8 ramp m128_ramp" "MFSK16 32 24 gray 8 ramp m16_gray"; do
  set -- $spec
  node "$here/gen_pic.mjs" "$1" "$2" "$3" "$4" "$5" 1500 "$out/fwpic_$7.wav" "$6"
done

# THOR 图片录音（`fwpic_t<模式>_<名>.wav`，尺寸只能取 fldigi 的六档，spp 固定 10）：给 `thor/tests/real_recordings` 用。
for spec in "THOR16 59 74 color 10 bands t16_color_bands" "THOR16 59 74 color 10 ramp t16_color_ramp" \
            "THOR16 59 74 gray 10 ramp t16_gray_ramp" "THOR25 59 74 color 10 ramp t25_ramp" \
            "THOR32 59 74 color 10 ramp t32_ramp" "THOR11 59 74 color 10 ramp t11_ramp" \
            "THOR22 59 74 color 10 ramp t22_ramp" "THOR44 59 74 color 10 ramp t44_ramp" \
            "THOR56 59 74 color 10 ramp t56_ramp" "THOR16 120 150 color 10 ramp t16_big_ramp"; do
  set -- $spec
  node "$here/gen_pic.mjs" "$1" "$2" "$3" "$4" "$5" 1500 "$out/fwpic_$7.wav" "$6"
done

# 长录音（报文重复 8 遍，约 45–175 秒）：给时钟 / 频率漂移跟踪的验证用，文件名 `fwlong_<模式>.wav`。
# 漂移由测试里的重采样（线性与时变转速）加上去，信号本身仍是 fldigi 的原版发端生成的。
long="$text"
for _ in 2 3 4 5 6 7 8; do long="$long $text"; done
for m in MFSK16 MFSK32 THOR8 THOR16; do
  node "$here/gen.mjs" "$m" "$long" 1500 "$out/fwlong_$m.wav"
done
