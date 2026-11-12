"""用 necpp（PyNEC）核对「天线补充专题」页里写进去的几个数字。

本站求解器与 necpp 是两套独立实现，这里的数字不依赖本站求解器；页面 `/antenna-topics` 与
`crates/core/src/antenna_topics.rs` 只引用这里**核对过**的量，没有对应结果的说法一律写成定性的。

核对的四件事：
  dipole     自由空间半波偶极子：谐振电阻 ≈ 73 Ω，增益 ≈ 2.15 dBi（dBi 与 dBd 的 2.15 dB 差）；
  monopole   理想大地上的 1/4λ 垂直振子：谐振电阻 ≈ 偶极子的一半，增益 ≈ 偶极子 + 3 dB；
  droop      自由空间里 1/4λ 垂直振子 + 4 根 1/4λ 地网，地网水平 / 下垂 30° / 45° / 60° 时的谐振电阻
             （「地网下垂越多、馈点阻抗越接近 50 Ω」这条经验的来源）；
  reflect    水平偶极子架在理想大地上方不同高度时的最大增益（地面反射最多把增益再抬高约 6 dB）。

用法（在仓库根目录；与 `necpp_ref.py` / `trap_necpp.py` 同一套装法）：
    uv pip install --python <python3> --target /tmp/necpp-pkg --only-binary :all: PyNEC
    PYTHONPATH=/tmp/necpp-pkg <python3> -B crates/nec/reference/antenna_topics_necpp.py [dipole|monopole|droop|reflect|all]

口径：1 mm 半径细导线、f = 14.2 MHz（λ ≈ 21.11 m）、每根导线 41 段以上、
「谐振」指馈电电抗过零（二分法找导线长度）。
"""

import math
import sys

from PyNEC import nec_context

F_MHZ = 14.2
LAMBDA = 299.792458 / F_MHZ
RADIUS = 1e-3
REFLECT_HALF = 0.2375  # 「地面反射」一节的水平偶极子半长（λ）；参照是同一根导线在自由空间的增益


def _gain_max_dbi(ctx):
    """整球面最大增益（dBi）；`rp_card` 扫 θ 0–90°（理想大地）或 0–180°（自由空间）。"""
    gains = ctx.get_radiation_pattern(0).get_gain()
    return float(gains.max())


def dipole_z_gain(half):
    ctx = nec_context()
    ctx.get_geometry().wire(1, 81, -half, 0, 0, half, 0, 0, RADIUS, 1.0, 1.0)
    ctx.geometry_complete(0)
    ctx.fr_card(0, 1, F_MHZ, 0)
    ctx.ex_card(0, 1, 41, 0, 1.0, 0, 0, 0, 0, 0)
    ctx.rp_card(0, 37, 72, 0, 5, 0, 0, 0.0, 0.0, 5.0, 5.0, 0, 0)
    z = ctx.get_input_parameters(0).get_impedance()[0]
    return z.real, z.imag, _gain_max_dbi(ctx)


def monopole_z_gain(h):
    """理想大地上的垂直振子（高度 h，底端在地面），馈电在最底下一段。"""
    ctx = nec_context()
    ctx.get_geometry().wire(1, 41, 0, 0, 0.0, 0, 0, h, RADIUS, 1.0, 1.0)
    ctx.geometry_complete(1)  # 1：有理想大地，镜像
    ctx.gn_card(1, 0, 0, 0, 0, 0, 0, 0)
    ctx.fr_card(0, 1, F_MHZ, 0)
    ctx.ex_card(0, 1, 1, 0, 1.0, 0, 0, 0, 0, 0)
    ctx.rp_card(0, 19, 72, 0, 5, 0, 0, 0.0, 0.0, 5.0, 5.0, 0, 0)
    z = ctx.get_input_parameters(0).get_impedance()[0]
    return z.real, z.imag, _gain_max_dbi(ctx)


def ground_plane_z(h, droop_deg, radial_len, nrad=4):
    """自由空间：垂直振子 + `nrad` 根辐条（从振子底端向外，下垂 `droop_deg`）。"""
    ctx = nec_context()
    geo = ctx.get_geometry()
    z0 = 0.5  # 抬起一点，免得与坐标原点的数值噪声混在一起
    geo.wire(1, 41, 0, 0, z0, 0, 0, z0 + h, RADIUS, 1.0, 1.0)
    d = math.radians(droop_deg)
    for i in range(nrad):
        a = 2 * math.pi * i / nrad
        x, y, z = radial_len * math.cos(d) * math.cos(a), radial_len * math.cos(d) * math.sin(a), -radial_len * math.sin(d)
        geo.wire(2 + i, 21, 0, 0, z0, x, y, z0 + z, RADIUS, 1.0, 1.0)
    ctx.geometry_complete(0)
    ctx.fr_card(0, 1, F_MHZ, 0)
    ctx.ex_card(0, 1, 1, 0, 1.0, 0, 0, 0, 0, 0)
    ctx.xq_card(0)
    z = ctx.get_input_parameters(0).get_impedance()[0]
    return z.real, z.imag


def horizontal_dipole_over_ground(height_wl, average=False):
    """水平偶极子；`average=True` 用平均大地（εr = 13、σ = 5 mS/m）的反射系数近似，否则理想大地。"""
    ctx = nec_context()
    h = height_wl * LAMBDA
    half = REFLECT_HALF * LAMBDA
    ctx.get_geometry().wire(1, 41, -half, 0, h, half, 0, h, RADIUS, 1.0, 1.0)
    ctx.geometry_complete(1)
    if average:
        ctx.gn_card(0, 0, 13.0, 0.005, 0, 0, 0, 0)
    else:
        ctx.gn_card(1, 0, 0, 0, 0, 0, 0, 0)
    ctx.fr_card(0, 1, F_MHZ, 0)
    ctx.ex_card(0, 1, 21, 0, 1.0, 0, 0, 0, 0, 0)
    ctx.rp_card(0, 19, 72, 0, 5, 0, 0, 0.0, 0.0, 5.0, 5.0, 0, 0)
    z = ctx.get_input_parameters(0).get_impedance()[0]
    return z.real, z.imag, _gain_max_dbi(ctx)


def bisect(g, lo, hi, tol=0.02):
    """`g`（电抗）随长度单调上升：二分到 |g| < tol。"""
    glo, ghi = g(lo), g(hi)
    for _ in range(12):
        if glo <= 0 <= ghi:
            break
        w = hi - lo
        if glo > 0:
            lo, glo = lo - w, g(lo - w)
        if ghi < 0:
            hi, ghi = hi + w, g(hi + w)
    for _ in range(60):
        mid = 0.5 * (lo + hi)
        gm = g(mid)
        if abs(gm) < tol:
            return mid
        lo, hi = (mid, hi) if gm < 0 else (lo, mid)
    return 0.5 * (lo + hi)


def mode_dipole():
    half = bisect(lambda h: dipole_z_gain(h)[1], 0.23 * LAMBDA, 0.25 * LAMBDA)
    r, x, g = dipole_z_gain(half)
    print(f"自由空间半波偶极子：半长 {half / LAMBDA:.4f} λ，R = {r:.2f} Ω，X = {x:.3f} Ω，最大增益 {g:.2f} dBi")
    return r, g


def mode_monopole(dipole_r, dipole_g):
    h = bisect(lambda hh: monopole_z_gain(hh)[1], 0.23 * LAMBDA, 0.25 * LAMBDA)
    r, x, g = monopole_z_gain(h)
    print(f"理想大地上的垂直振子：高 {h / LAMBDA:.4f} λ，R = {r:.2f} Ω，X = {x:.3f} Ω，最大增益 {g:.2f} dBi")
    if dipole_r:
        print(f"  与偶极子比：R 之比 {r / dipole_r:.3f}，增益差 {g - dipole_g:+.2f} dB")
    print(f"  接 50 Ω 的驻波比 ≈ {max(50 / r, r / 50):.2f}")


def mode_droop():
    print("自由空间 1/4λ 垂直振子 + 4 根 1/4λ（物理长度）辐条，调垂直振子长度到谐振：")
    for droop in (0, 30, 45, 60):
        radial = 0.25 * LAMBDA
        h = bisect(lambda hh: ground_plane_z(hh, droop, radial)[1], 0.22 * LAMBDA, 0.26 * LAMBDA)
        r, x = ground_plane_z(h, droop, radial)
        print(f"  下垂 {droop:2d}°：垂直振子 {h / LAMBDA:.4f} λ，R = {r:.1f} Ω，X = {x:.2f} Ω")


def mode_reflect():
    # 参照取「同一根导线在自由空间」的增益，而不是写死 2.15：两者差约 0.01 dB，但参照应与被比的天线同源。
    g_free = dipole_z_gain(REFLECT_HALF * LAMBDA)[2]
    print(f"水平偶极子（半长 {REFLECT_HALF} λ）在大地上方的最大增益（dBi）；同一根导线在自由空间 {g_free:.2f} dBi：")
    print("  高度      理想大地（比自由空间多）      平均大地 εr=13 σ=5 mS/m（比自由空间多）")
    for hw in (0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 1.0):
        _, _, gp = horizontal_dipole_over_ground(hw)
        _, _, ga = horizontal_dipole_over_ground(hw, average=True)
        print(f"  {hw:5.3f} λ   {gp:5.2f} dBi（{gp - g_free:+.2f} dB）       {ga:5.2f} dBi（{ga - g_free:+.2f} dB）")


def main():
    which = sys.argv[1] if len(sys.argv) > 1 else "all"
    dr = dg = None
    if which in ("dipole", "monopole", "all"):
        dr, dg = mode_dipole()
    if which in ("monopole", "all"):
        mode_monopole(dr, dg)
    if which in ("droop", "all"):
        mode_droop()
    if which in ("reflect", "all"):
        mode_reflect()


if __name__ == "__main__":
    main()
