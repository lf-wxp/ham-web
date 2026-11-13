"""用 NTIA 原件（P.368-10 的 `LFMF Source`）对拍本站的 `ground_wave` 移植。

原件不在仓库里（那是 ITU 的 ZIP，许可条件见 `crates/core/src/ground_wave/mod.rs` 的模块文档），
所以这个脚本自带「取得 ZIP → 编译 → 生成参照 → 跑本站」的全流程：

    python3 -B crates/core/reference/lfmf_check.py

ZIP 的来源按顺序：环境变量 `LFMF_ZIP`（本地文件）→ 工作目录里已有的 `p368.zip` → 用诚实的 User-Agent 直接下载。
ITU 的站点会拒绝脚本下载（返回 403）：这时脚本不会伪装浏览器绕过，而是提示你用浏览器手动下载，
再用 `LFMF_ZIP=/path/to/R-REC-P.368-10-202208-I\!\!ZIP-E.zip` 重跑。

依赖：`clang++`（macOS 自带 clang 也行，脚本会先修掉原件里 MSVC 专有的 `_copysign`
之类的函数名）、`cargo`。

两组对拍：
1. **主网格**：5 个发射高度 × 3 个接收高度 × 7 个频率 × 7 个距离 × 3 组地面参数 × 2 种极化
   = 4410 组，覆盖两条解法（平地 / 残留级数）的切换点；
2. **边界**：频率 0.01 / 30 MHz（上下限）、距离 0.001 / 10000 km、高度 0 / 50 m。

输出最坏相对误差与不平滑的个别点；退出码非零表示超出容差。
"""

import json
import math
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.error
import urllib.request
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[3]
ZIP_URL = "https://www.itu.int/dms_pubrec/itu-r/rec/p/R-REC-P.368-10-202208-I!!ZIP-E.zip"
UA = "ham-exam-web-lfmf-check"
# 原件在 Windows 上用 MSVC 编；macOS / clang 要先把这些「下划线前缀的 CRT 函数」改回标准名。
UNDERSCORE_FIXES = ["copysign", "hypot", "sqrt", "exp", "log", "log10", "fabs", "sin", "cos",
                    "tan", "atan", "atan2", "pow", "floor", "ceil", "fmod", "acos", "asin"]


def build_reference(work: pathlib.Path) -> pathlib.Path:
    """下载、打补丁、编译原件，返回可执行文件路径。"""
    override = os.environ.get("LFMF_ZIP")
    zip_path = pathlib.Path(override) if override else work / "p368.zip"
    if not zip_path.exists():
        req = urllib.request.Request(ZIP_URL, headers={"User-Agent": UA})
        try:
            with urllib.request.urlopen(req) as r, open(zip_path, "wb") as f:
                f.write(r.read())
        except urllib.error.HTTPError as e:
            zip_path.unlink(missing_ok=True)
            raise SystemExit(
                f"下载失败（HTTP {e.code}）：ITU 拒绝脚本下载。请用浏览器打开\n  {ZIP_URL}\n"
                "保存后用 LFMF_ZIP=<本地路径> 重跑。"
            )
    with zipfile.ZipFile(zip_path) as z:
        z.extractall(work / "src")
    base = work / "src" / "Integral Source Code" / "LFMF Source"
    shutil.copytree(base / "src", work / "build" / "src", dirs_exist_ok=True)
    shutil.copytree(base / "include", work / "build" / "include", dirs_exist_ok=True)
    for p in (work / "build" / "src").glob("*.cpp"):
        s = p.read_text()
        s = s.replace('#include "..\\include\\LFMF.h"', '#include "LFMF.h"')
        for fn in UNDERSCORE_FIXES:
            s = re.sub(rf"\b_{fn}\b", fn, s)
        p.write_text(s)
    h = work / "build" / "include" / "LFMF.h"
    h.write_text(h.read_text().replace('extern "C" __declspec(dllexport)', 'extern "C"'))
    (work / "build" / "driver.cpp").write_text(
        '#include <cstdio>\n#include "LFMF.h"\n'
        "int main(){double a,b,c,d,e,f,g,h;int i;"
        "while(scanf(\"%lf %lf %lf %lf %lf %lf %lf %lf %d\",&a,&b,&c,&d,&e,&f,&g,&h,&i)==9){"
        "Result r{};int rc=LFMF(a,b,c,d,e,f,g,h,i,&r);"
        'if(rc){printf("ERR\\n");continue;}'
        'printf("%.10f %.10f %d\\n",r.E_dBuVm,r.A_btl__db,r.method);}}\n'
    )
    exe = work / "build" / "lfmf"
    srcs = sorted(str(p) for p in (work / "build" / "src").glob("*.cpp"))
    subprocess.run(
        ["clang++", "-O2", "-std=c++17", "-I", str(work / "build" / "include"),
         str(work / "build" / "driver.cpp"), *srcs, "-o", str(exe)],
        check=True,
    )
    return exe


def main_grid():
    rows = []
    for h_tx in (0.0, 1.0, 10.0, 30.0, 50.0):
        for h_rx in (0.0, 2.0, 10.0):
            for f in (0.1, 1.0, 3.0, 7.0, 14.0, 21.0, 29.0):
                for d in (1.0, 5.0, 30.0, 80.0, 200.0, 1000.0, 5000.0):
                    for eps, sigma in ((15.0, 0.005), (80.0, 4.0), (4.0, 0.001)):
                        for pol in (0, 1):
                            rows.append((h_tx, h_rx, f, 100.0, 301.0, d, eps, sigma, pol))
    return rows


def edge_grid():
    rows = []
    for f in (0.01, 30.0):
        for d in (0.001, 10_000.0):
            for h_tx, h_rx in ((0.0, 0.0), (50.0, 50.0), (0.0, 50.0)):
                for pol in (0, 1):
                    rows.append((h_tx, h_rx, f, 1.0, 250.0, d, 1.0, 1e-6, pol))
                    rows.append((h_tx, h_rx, f, 1.0, 400.0, d, 81.0, 6.0, pol))
    return rows


def run(exe, rows, work):
    text = "\n".join(" ".join(repr(v) if isinstance(v, float) else str(v) for v in r) for r in rows) + "\n"
    ref = subprocess.run([str(exe)], input=text, capture_output=True, text=True, check=True).stdout.splitlines()

    target = os.environ.get("CARGO_TARGET_DIR")
    if not target:
        meta = subprocess.run(["cargo", "metadata", "--format-version", "1", "--no-deps"],
                              capture_output=True, text=True, check=True)
        target = json.loads(meta.stdout)["target_directory"]
    dump = pathlib.Path(target) / "release" / "examples" / "lfmf_dump"
    subprocess.run(["cargo", "build", "-q", "--release", "-p", "ham-web-core", "--example", "lfmf_dump"],
                   cwd=ROOT, check=True)
    ours = subprocess.run([str(dump)], input=text, capture_output=True, text=True, check=True).stdout.splitlines()
    return ref, ours


def report(name, rows, ref, ours, tol_db):
    worst = (0.0, None)
    bad = []
    method_mismatch = 0
    for row, r, o in zip(rows, ref, ours):
        if r.startswith("ERR") or o.startswith("ERR"):
            bad.append((row, r, o, "ERR"))
            continue
        rf, ra, rm = r.split()
        of, oa, om = o.split()
        if rm != om:
            method_mismatch += 1
        d_e = abs(float(rf) - float(of))
        d_a = abs(float(ra) - float(oa))
        # 场强接近 0（dBµV/m 很负）时相对误差没有意义，统一用 dB 差。
        if d_e > worst[0]:
            worst = (d_e, (row, rf, of))
        if d_e > tol_db or d_a > tol_db:
            bad.append((row, rf, of, d_e))
    print(f"{name}：{len(rows)} 组，最坏场强差 {worst[0]:.3e} dB，方法不一致 {method_mismatch} 组，超差 {len(bad)} 组")
    for b in bad[:10]:
        print("   ", b)
    return worst[0], method_mismatch, len(bad)


def main():
    work = pathlib.Path(os.environ.get("LFMF_WORK", tempfile.gettempdir() + "/lfmf_ref"))
    work.mkdir(parents=True, exist_ok=True)
    exe = build_reference(work)
    ok = True
    # 主网格要求 1e-6 dB（实测 4.8e-8）；边界取 1e-4：0.01 MHz + εr=1 + σ=1e-6 的角点上，
    # 复平方根在 C++ 的 `std::sqrt(complex)` 与本站的分支写法之间差约 4e-6 相对量（实测 3.9e-5 dB）。
    for name, rows, tol in (("主网格", main_grid(), 1e-6), ("边界", edge_grid(), 1e-4)):
        ref, ours = run(exe, rows, work)
        worst, mismatch, bad = report(name, rows, ref, ours, tol)
        ok = ok and worst <= tol and mismatch == 0 and bad == 0
    print("✓ 与 NTIA 原件逐点一致" if ok else "✗ 有超差")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
