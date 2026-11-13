"""用官方 P372 程序对拍本站 `atmospheric_noise` 的移植。

流程：取 `ITU-R-Study-Group-3/ITU-R-HF`（公开仓库，README 允许「used by implementers … free from
any copyright assertions」），把 `P372/Src/P372` 编成命令行程序，再对同一批输入比较。

    python3 -B crates/core/reference/p372_check.py

依赖：`git`、`clang`、`cargo`。官方源码固定在 `OFFICIAL_SHA` 这一版（`git` 稀疏检出，只取 `P372/`，
约 130 MB；比整仓 zip 小得多）。想对拍别的版本：把 `P372_SRC` 指向本地检出，或改 `OFFICIAL_SHA`。

**官方程序的月份是 0 起的**（0 = 一月）：脚本在喂输入时减 1，本模块用 1 起的月份。
**官方的小时是整数**：带小数的小时（`HOURS` 里的 8.7）喂给官方时取整，喂给本站时原样传入。

网格：12 个月 × 7 个 UTC 小时 × 13 个纬度 × 18 个经度 × 12 个频率 = 235 872 组大气噪声；
合成再取其中 1/4 的点 × 3 个环境（见 `COMBINED_STRIDE`）。
**经度必须包含不是 15° 整倍数的值**：官方把「小时 + (int)(经度 / 15)」只截断经度那一项，而 `经度 / 15` 在 30° 步长的网格上恒为
整数，截断顺序写反也测不出来 —— 这个盲区真实出现过（`LNGS_OFF_GRID`）。
比三组量：大气噪声 `FaA / DuA / DlA`，以及与人为 + 银河噪声合成后的 `FamT / DuT / DlT`（环境类别 0–3 各一遍，
合成只在人为噪声曲线的适用频段内比，其余必须是 `NA`）。
"""

import json
import math
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[3]
REPO_URL = "https://github.com/ITU-R-Study-Group-3/ITU-R-HF.git"
# 固定提交，免得 master 前进后「版本 14.3」的说法悄悄变了（取回日期见 `atmospheric_noise::CHECKED_ON`）。
OFFICIAL_SHA = "82017594a1c6cacfaa7e86954c4ae7b3a5825a3d"

MONTHS = tuple(range(1, 13))
HOURS = (0, 2, 6, 12, 18, 22, 8.7)
LATS = tuple(range(-90, 91, 15))
# 30° 步长的整倍数 + 不是 15° 整倍数的经度（西经与东经都要有，西经才会暴露截断顺序）。
LNGS_ON_GRID = tuple(range(-180, 181, 30))
LNGS_OFF_GRID = (-116.4, -73.9, -0.5, 5.5, 116.4)
LNGS = LNGS_ON_GRID + LNGS_OFF_GRID
COMBINED_STRIDE = 4
FS = (0.1, 0.5, 1, 2, 3.5, 5, 7, 10, 14, 18, 21, 28)

DRIVER = r"""
#include <stdio.h>
#include <stdlib.h>
#include <math.h>
#include "Noise.h"
int main(void){
  struct NoiseParams n;
  if (AllocateNoiseMemory(&n)!=RTN_ALLOCATEP372OK) return 1;
  InitializeNoise(&n);
  const char *path = getenv("COEFF_DIR");
  double lng, lat, f, cat; int hour, month, last = -1;
  while (scanf("%d %d %lf %lf %lf %lf", &month, &hour, &lat, &lng, &f, &cat)==6){
    n.ManMadeNoise = cat;
    if (month != last){
      if (ReadFamDud(&n, path, month)!=RTN_READFAMDUDOK){printf("ERR\n");continue;}
      last = month;
    }
    int rc = Noise(&n, hour, lng*(M_PI/180.0), lat*(M_PI/180.0), f);
    if (rc!=RTN_NOISEOK){printf("ERR\n");continue;}
    printf("%.9f %.9f %.9f %.9f %.9f %.9f\n", n.FaA, n.DuA, n.DlA, n.FamT, n.DuT, n.DlT);
  }
  return 0;
}
"""


def build(work: pathlib.Path) -> pathlib.Path:
    override = os.environ.get("P372_SRC")
    src = pathlib.Path(override) if override else work / "hf"
    if not (src / "P372").exists():
        print(f"稀疏检出 ITU-R-HF @ {OFFICIAL_SHA[:10]}（只取 P372/）…", file=sys.stderr)
        subprocess.run(["git", "clone", "--filter=blob:none", "--no-checkout", REPO_URL, str(src)],
                       check=True)
        subprocess.run(["git", "-C", str(src), "sparse-checkout", "set", "P372"], check=True)
        subprocess.run(["git", "-C", str(src), "checkout", OFFICIAL_SHA], check=True)
    if not override:
        head = subprocess.run(["git", "-C", str(src), "rev-parse", "HEAD"],
                              capture_output=True, text=True, check=True).stdout.strip()
        if head != OFFICIAL_SHA:
            raise SystemExit(f"{src} 在 {head[:10]}，不是固定的 {OFFICIAL_SHA[:10]}；删掉它让脚本重新检出")
    build_dir = work / "build"
    build_dir.mkdir(exist_ok=True)
    for p in (src / "P372/Src/P372").glob("*.c"):
        shutil.copy(p, build_dir)
    for p in (src / "P372/Src/P372").glob("*.h"):
        shutil.copy(p, build_dir)
    # `NoiseDriver.c` 自带 main，不要。
    (build_dir / "NoiseDriver.c").unlink(missing_ok=True)
    (build_dir / "drv.c").write_text(DRIVER)
    exe = build_dir / "p372"
    subprocess.run(
        ["clang", "-O2", "-fcommon", "-I", str(build_dir), str(build_dir / "drv.c"),
         # 只要 P372 的核心：`NoiseDriver.c` / `MakeNoise.c` 自带 main 或是别的程序的入口。
         *(str(build_dir / n) for n in ("InitializeNoise.c", "Noise.c", "NoiseMemory.c")),
         "-lm", "-o", str(exe)],
        check=True,
    )
    return exe


def main():
    work = pathlib.Path(os.environ.get("P372_WORK", tempfile.gettempdir() + "/p372_ref"))
    work.mkdir(parents=True, exist_ok=True)
    exe = build(work)
    coeff = pathlib.Path(os.environ.get("P372_SRC") or work / "hf") / "P372" / "Data"

    base = [
        (m, h, la, lo, f)
        for m in MONTHS for h in HOURS for la in LATS for lo in LNGS for f in FS
    ]
    # 每个点跑一遍环境 0；其中每 `COMBINED_STRIDE` 个点再跑环境 1–3，让四种环境都有足够的覆盖。
    cases = [(*c, 0) for c in base]
    for i, c in enumerate(base):
        if i % COMBINED_STRIDE == 0:
            cases += [(*c, e) for e in (1, 2, 3)]
    # 官方 `ReadFamDud()` 的 `month` 是 0 起的（内部按 `month + 1` 拼 `COEFF%02dW.txt`）。
    # 官方的小时是 `int`：带小数的小时取整后喂给它（本站收到的是原值，由 `atmospheric` 自己取整）。
    text = "\n".join(f"{m - 1} {int(h)} {la} {lo} {f} {e}" for m, h, la, lo, f, e in cases) + "\n"
    ref = subprocess.run(
        [str(exe)], input=text, capture_output=True, text=True, check=True,
        env={**os.environ, "COEFF_DIR": str(coeff) + "/"},
    ).stdout.splitlines()

    target = os.environ.get("CARGO_TARGET_DIR")
    if not target:
        meta = subprocess.run(["cargo", "metadata", "--format-version", "1", "--no-deps"],
                              cwd=ROOT, capture_output=True, text=True, check=True)
        target = json.loads(meta.stdout)["target_directory"]
    subprocess.run(["cargo", "build", "-q", "--release", "-p", "ham-web-core", "--example", "p372_dump"],
                   cwd=ROOT, check=True)
    dump = pathlib.Path(target) / "release" / "examples" / "p372_dump"
    ours = subprocess.run([str(dump)], input="".join(f"{m} {h} {la} {lo} {f} {e}\n" for m, h, la, lo, f, e in cases),
                          capture_output=True, text=True, check=True).stdout.splitlines()

    assert len(ref) == len(cases) == len(ours), (len(ref), len(cases), len(ours))
    errs = []
    worst_atm = (0.0, None)
    worst_cmb = (0.0, None)
    n_cmb = 0
    for case, r, o in zip(cases, ref, ours):
        if r.startswith("ERR") or o.startswith("ERR"):
            errs.append((case, r, o))
            continue
        rv = [float(x) for x in r.split()]
        ov = o.split()
        e = max(abs(float(ov[i]) - rv[i]) for i in range(3))
        if e > worst_atm[0]:
            worst_atm = (e, (case, rv[:3], ov[:3]))
        f, env = case[4], case[5]
        in_curve = f >= 0.3 and (env != 3 or f <= 30.0)
        if in_curve:
            if ov[3] == "NA":
                errs.append((case, "合成应有值", o))
                continue
            e = max(abs(float(ov[3 + i]) - rv[3 + i]) for i in range(3))
            n_cmb += 1
            if e > worst_cmb[0]:
                worst_cmb = (e, (case, rv[3:], ov[3:]))
        elif ov[3] != "NA":
            errs.append((case, "合成应是 NA", o))
    print(f"{len(cases)} 组：大气噪声最坏差 {worst_atm[0]:.3e} dB；"
          f"合成 {n_cmb} 组最坏差 {worst_cmb[0]:.3e} dB；ERR {len(errs)} 组")
    for w in (worst_atm, worst_cmb):
        if w[1]:
            print("  最坏点：", w[1])
    for e in errs[:5]:
        print("  ERR:", e)
    ok = worst_atm[0] < 1e-6 and worst_cmb[0] < 1e-6 and not errs
    print("✓ 与官方程序一致" if ok else "✗ 有偏差")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
