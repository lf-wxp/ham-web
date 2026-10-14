"""用 PyNEC（necpp，NEC-2 的 C++ 移植）跑方案库模板，输出交叉验证参照值。

用法（在仓库根目录）：
    python3 -m venv .venv-necpp && .venv-necpp/bin/pip install PyNEC==2.3.4
    .venv-necpp/bin/python crates/nec/reference/necpp_ref.py
把输出抄进 `nec_templates.rs` 的 `NECPP_REF`；对照本实现用
    cargo run -p ham-web-nec --example template_report --release

只用到模板里出现的卡片：GW / GE / EX / FR。GE 1 在 NEC-2 里还要配 GN 1 才是理想导体地面
（GE 只声明「有地面」）。最大增益在 θ 0–90°、φ 0–355°（5° 步进）的上半空间里取。
"""
import math
import re
import sys

from PyNEC import nec_context

src = open("crates/nec/src/nec_templates.rs").read()
blocks = re.findall(r'id: "([^"]+)",\s*name: "[^"]*",\s*text: "\\\n(.*?)",\n', src, re.S)


def run(text):
    ctx = nec_context()
    geo = ctx.get_geometry()
    ground = False
    feed = None
    freq = None
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
    ctx.geometry_complete(1 if ground else 0)
    if ground:
        ctx.gn_card(1, 0, 0, 0, 0, 0, 0, 0)
    ctx.fr_card(0, 1, freq, 0)
    tag, seg, v = feed
    ctx.ex_card(0, tag, seg, 0, v, 0, 0, 0, 0, 0)
    ctx.rp_card(0, 19, 72, 0, 5, 0, 0, 0.0, 0.0, 5.0, 5.0, 0, 0)
    z = ctx.get_input_parameters(0).get_impedance()[0]
    gain = ctx.get_radiation_pattern(0).get_gain()
    return z.real, z.imag, float(gain.max())


print(f"// necpp（PyNEC）参照，{sys.version.split()[0]}")
for tid, text in blocks:
    try:
        r, x, g = run(text)
        print(f'    ("{tid}", {r:.3f}, {x:.3f}, {g:.3f}),')
    except Exception as e:  # noqa: BLE001
        print(f"    // {tid}: necpp 拒绝 —— {e}")
