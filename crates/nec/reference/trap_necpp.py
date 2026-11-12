"""用 necpp（PyNEC）对拍 trap 偶极子的外段缩短量 —— `crates/core/src/trap.rs` 的独立参照。

本站求解器与 necpp 是两套独立实现，这里的数字不依赖本站求解器。三种模式：

    cut    9 组设计的「每臂缩短量」（直导线谐振半长 − 带 trap 的谐振半长），
           输出抄进 `trap.rs` 测试里的 `NECPP_CUT_M`；
    slope  裸偶极子谐振处电抗斜率 → 有效平均特性阻抗 Z_a = 120·(ln(2h/a) − c) 的常数 c
           （不涉及 trap，`trap.rs` 的 `Z_C_OFFSET` 取这里的均值）；
    freq   按闭式设计出的 trap 偶极子，在 necpp 里实际谐振的频率偏离目标多少（17 组，
           其中 8 组「留出集」不参与任何取值）。可传参数 c，默认取 `Z_C_OFFSET`。

用法（在仓库根目录；与 `necpp_ref.py` 同一套装法）：
    uv pip install --python <python3> --target /tmp/necpp-pkg --only-binary :all: PyNEC
    PYTHONPATH=/tmp/necpp-pkg <python3> crates/nec/reference/trap_necpp.py cut
PyNEC 的预编译包不覆盖所有 Python 版本（没有 SWIG 就编不了源码包）；装不上时换一个较新的解释器。
`-B` 可免得在仓库里留下 `__pycache__`。

PyNEC 版本：`necpp_ref.py` 当初钉的是 2.3.4，本脚本用 **2.4.0** 跑出；
`necpp_ref.py` 的 6 组输出在 2.4.0 下与 2.3.4 逐位相同，两者同为 necpp 的封装，口径一致。

口径：自由空间、单根沿 x 轴的 1 mm 半径导线、馈电在正中（601 段）；trap 在 ±p 处各放一个，
用 `LD` type 4 给出**该频率下**的阻抗（并联 LC 在单一频率上就是一个纯电抗，
见 `trap.rs` 模块文档）；「谐振」指馈电电抗过零。
"""

import math
import sys

from PyNEC import nec_context

A = 1e-3  # 导线半径（m），与 trap.rs 的 WIRE_RADIUS_M 相同
N = 601  # 单根导线分段数（奇数，正中一段馈电）
H143 = 143.0  # 与 trap.rs 的 HALF_WAVE_M_MHZ 相同
Z_C_OFFSET = 1.43  # 与 trap.rs 的 Z_C_OFFSET 相同
TAU = 2 * math.pi

# (高波段 MHz, 低波段 MHz, 电容 pF)。前 6 组是低 / 高 ≈ 0.26–0.51 的典型设计，后 3 组是两个波段靠得近。
CUT_CASES = [
    (14.175, 7.05, 67.0), (14.175, 7.05, 30.0), (14.175, 7.05, 150.0),
    (14.175, 3.65, 67.0), (28.5, 14.175, 67.0), (7.1, 3.65, 100.0),
    (14.175, 10.12, 67.0), (14.175, 12.0, 67.0), (14.175, 13.5, 67.0),
]
# 留出集：频段对与电容都不同于上面，不参与 c 的取值。
HELD_OUT = [
    (21.2, 14.175, 50.0), (18.1, 10.12, 80.0), (10.12, 7.1, 90.0), (24.9, 14.175, 60.0),
    (14.175, 7.05, 100.0), (21.2, 7.05, 40.0), (14.175, 10.12, 150.0), (28.5, 21.2, 67.0),
]


def solve_z(f_mhz, inner, outer, x_trap=None, radius=None):
    """馈电点阻抗 (R, X)。`x_trap` 是 trap 在该频率的电抗（Ω）；None 时是半长 `outer` 的直导线
    （inner 忽略）。"""
    half = outer if x_trap is None else inner + outer
    n = N
    ctx = nec_context()
    ctx.get_geometry().wire(1, n, -half, 0, 0, half, 0, 0, radius or A, 1.0, 1.0)
    ctx.geometry_complete(0)
    if x_trap is not None:
        j = min(max(301 + round(inner / (2 * half / n)), 1), n)
        for seg in (j, n + 1 - j):
            ctx.ld_card(4, 1, seg, seg, 0.0, x_trap, 0.0)
    ctx.fr_card(0, 1, f_mhz, 0)
    ctx.ex_card(0, 1, (n + 1) // 2, 0, 1.0, 0, 0, 0, 0, 0)
    ctx.xq_card(0)
    z = ctx.get_input_parameters(0).get_impedance()[0]
    return z.real, z.imag


def root(g, x0, x1, tol=0.01):
    """馈电电抗随长度 / 频率单调上升（负 → 正）：先向两侧扩张到变号，再二分。"""
    lo, hi = min(x0, x1), max(x0, x1)
    glo, ghi = g(lo), g(hi)
    for _ in range(12):
        if glo <= 0 <= ghi:
            break
        w = hi - lo
        if glo > 0:
            lo, glo = lo - w, g(lo - w)
        if ghi < 0:
            hi, ghi = hi + w, g(hi + w)
    else:
        raise RuntimeError(f"括不住根 [{lo}, {hi}]")
    for _ in range(60):
        mid = 0.5 * (lo + hi)
        gm = g(mid)
        if abs(gm) < tol:
            return mid
        lo, hi = (mid, hi) if gm < 0 else (lo, mid)
    return 0.5 * (lo + hi)


def trap_x(l_uh, c_pf, f_mhz):
    """并联 LC 在该频率的电抗（Ω），与 trap.rs 的 trap_reactance_ohm 同一公式。"""
    w = TAU * f_mhz * 1e6
    xl, xc = w * l_uh * 1e-6, 1.0 / (w * c_pf * 1e-12)
    return xl * xc / (xc - xl)


def trap_l_uh(fh, c_pf):
    return 1e6 / ((TAU * fh * 1e6) ** 2 * c_pf * 1e-12)


def plain_h(f_mhz, radius=None):
    """直导线的谐振半长（m）。"""
    arm = H143 / f_mhz / 2
    return root(lambda h: solve_z(f_mhz, 0, h, radius=radius)[1], arm, arm * 1.01)


def closed_outer(fh, fl, c_pf, inner, arm, c_off):
    """trap.rs 的 loaded_outer_m：外段长度。`arm` 是求 Z_c 时用的每臂长度。"""
    k = TAU * fl / (2 * H143)
    zc = 0.5 * 120 * (math.log(2 * arm / A) - c_off)
    x = trap_x(trap_l_uh(fh, c_pf), c_pf, fl)
    return math.atan2(1, math.tan(k * inner) + x / zc) / k


def mode_cut():
    print(f"// necpp（PyNEC）参照：每臂缩短量（m），自由空间 / {A * 1e3:g} mm 半径 / {N} 段")
    for fh, fl, c in CUT_CASES:
        inner = H143 / fh / 2
        x = trap_x(trap_l_uh(fh, c), c, fl)
        guess = closed_outer(fh, fl, c, inner, H143 / fl / 2, Z_C_OFFSET)
        plain = plain_h(fl)
        q = root(lambda q: solve_z(fl, inner, q, x_trap=x)[1], guess, guess * 0.9)
        print(f"    ({fh}, {fl}, {c}, {plain - (inner + q):.3f}),")


def mode_slope():
    print("f_MHz  a_mm   h0      Za_necpp  Za(c=1)  c")
    cs = []
    for f in (3.65, 7.05, 14.175, 28.5):
        for a in (0.5e-3, 1e-3, 2e-3, 4e-3):
            h0 = plain_h(f, a)
            k, d = math.pi / (2 * h0), 0.02 * h0
            slope = (solve_z(f, 0, h0 + d, radius=a)[1] - solve_z(f, 0, h0 - d, radius=a)[1]) / (2 * d)
            za, base = slope / k, 120 * (math.log(2 * h0 / a) - 1)
            cs.append(math.log(2 * h0 / a) - za / 120)
            print(f"{f:6.3f} {a * 1e3:5.2f} {h0:7.3f} {za:9.1f} {base:8.1f} {cs[-1]:.3f}")
    print(f"c 范围 {min(cs):.3f}–{max(cs):.3f}，均值 {sum(cs) / len(cs):.3f}")


def self_consistent_design(fh, fl, c_pf, c_off):
    """内段取「该求解器里正好谐振于 fh 的直导线半长」，这样 143/f 经验常数自身的偏差
    （自由空间里谐振偏高约 1.7%）不会混进结果。返回 (电感 µH, 内段, 外段)。"""
    p, hl = plain_h(fh), plain_h(fl)
    l_uh = trap_l_uh(fh, c_pf)
    k = math.pi / (2 * hl)
    zc = 0.5 * 120 * (math.log(2 * hl / A) - c_off)
    return l_uh, p, math.atan2(1, math.tan(k * p) + trap_x(l_uh, c_pf, fl) / zc) / k


def resonance(l_uh, c_pf, p, q, f0):
    """串联谐振频率（MHz）。trap 的电抗随频率重算。"""
    return root(lambda f: solve_z(f, p, q, x_trap=trap_x(l_uh, c_pf, f))[1], f0, f0 * 1.004, 0.02)


def low_high_error(fh, fl, c_pf, c_off):
    """在 necpp 里，闭式设计的低 / 高波段谐振偏离目标的百分比。"""
    l_uh, p, q = self_consistent_design(fh, fl, c_pf, c_off)
    return ((resonance(l_uh, c_pf, p, q, fl) / fl - 1) * 100,
            (resonance(l_uh, c_pf, p, q, fh * 1.01) / fh - 1) * 100)


def mode_freq(c_off):
    print(f"c = {c_off}：低 / 高波段谐振偏离目标 %")
    for label, cases in (("设计集", CUT_CASES), ("留出集", HELD_OUT)):
        errs = []
        for fh, fl, c in cases:
            lo, hi = low_high_error(fh, fl, c, c_off)
            errs.append(lo)
            print(f"  {label} {fh}/{fl}/{c:<5} 低 {lo:+6.2f}  高 {hi:+6.2f}")
        print(f"  {label}：低波段平均 {sum(errs) / len(errs):+.3f}，最大 |e| {max(map(abs, errs)):.3f}")


if __name__ == "__main__":
    mode = sys.argv[1] if len(sys.argv) > 1 else "cut"
    if mode == "cut":
        mode_cut()
    elif mode == "slope":
        mode_slope()
    elif mode == "freq":
        mode_freq(float(sys.argv[2]) if len(sys.argv) > 2 else Z_C_OFFSET)
    else:
        sys.exit(f"未知模式 {mode!r}。\n\n{__doc__}")
