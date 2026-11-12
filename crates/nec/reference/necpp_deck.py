"""把一份 `.nec` 卡片组交给 PyNEC（necpp，NEC-2 的 C++ 移植）求解 —— 参照脚本共用的那一份解析与调用。

`necpp_ref.py`（方案库模板的参照值）与 `tl_necpp.py`（传输线对拍）都 import 它，
免得两处各维护一份「卡片怎么转给 necpp」。只认本仓模板与对拍用到的卡片：
GW / GE / EX / FR / TL（`Z0` 为负是交叉线）；`GE 1` 要配 `GN 1` 才是理想导体地面
（GE 只声明「有地面」）。方向图 φ 0–355°、5° 步进，θ 从 0° 起 5° 步进取 `n_theta` 行
（19 行 = 上半空间，37 行 = 整个球面）。

PyNEC 版本 2.4.0；预编译包不覆盖所有 Python 版本，装法见 `tl_necpp.py` 的文件头。
"""

from PyNEC import nec_context

PHI_POINTS = 72


def run_deck(text: str, n_theta: int):
    """返回 `(R, X, 最大增益 dBi, 最大增益方位 °, 前后比 dB)`。

    前后比取最大增益所在 θ 上、方位相反的那个方向；增益是 necpp 的 dBi（含 `rp_card` 的总增益口径）。
    """
    ctx = nec_context()
    geo = ctx.get_geometry()
    ground = False
    feed = None
    freq = None
    tls = []
    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith(("CM", "CE", "EN")):
            continue
        card, *f = line.replace(",", " ").split()
        if card == "GW":
            tag, n = int(f[0]), int(f[1])
            x1, y1, z1, x2, y2, z2, r = map(float, f[2:9])
            geo.wire(tag, n, x1, y1, z1, x2, y2, z2, r, 1.0, 1.0)
        elif card == "GE":
            ground = int(f[0]) == 1
        elif card == "EX":
            feed = (int(f[1]), int(f[2]), float(f[4]))
        elif card == "FR":
            freq = next(float(v) for v in f[2:] if float(v) > 0)
        elif card == "TL":
            # TL I1 I2 I3 I4 F1 F2 …：两端的段、特性阻抗（负 = 交叉线）、线长（m）、
            # 两端并联导纳（缺省 0）。PyNEC 的 `tl_card` 要把 6 个 F 全给齐，所以补零。
            floats = (list(map(float, f[4:10])) + [0.0] * 6)[:6]
            tls.append((int(f[0]), int(f[1]), int(f[2]), int(f[3]), *floats))
    if feed is None or freq is None:
        raise ValueError("卡片组里缺 EX 或 FR")
    ctx.geometry_complete(1 if ground else 0)
    if ground:
        ctx.gn_card(1, 0, 0, 0, 0, 0, 0, 0)
    ctx.fr_card(0, 1, freq, 0)
    for t in tls:
        ctx.tl_card(*t)
    tag, seg, v = feed
    ctx.ex_card(0, tag, seg, 0, v, 0, 0, 0, 0, 0)
    ctx.rp_card(0, n_theta, PHI_POINTS, 0, 5, 0, 0, 0.0, 0.0, 5.0, 5.0, 0, 0)
    z = ctx.get_input_parameters(0).get_impedance()[0]
    gain = ctx.get_radiation_pattern(0).get_gain().reshape(n_theta, PHI_POINTS)
    ti, pj = divmod(int(gain.argmax()), PHI_POINTS)
    peak = float(gain[ti, pj])
    back = float(gain[ti, (pj + PHI_POINTS // 2) % PHI_POINTS])
    return z.real, z.imag, peak, pj * 5.0, peak - back
