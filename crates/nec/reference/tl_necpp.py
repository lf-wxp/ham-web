"""用 necpp（PyNEC）对拍带传输线（含交叉线）的结构 —— `crates/nec/src/nec.rs` 的独立参照。

同一份 `.nec` 卡片组分别交给 necpp 与本仓求解器（`examples/deck_report.rs`），逐点比较馈电点
阻抗与最大增益。传输线只用 `TL` 卡的最简形式：两端各一个段、`F1` = 特性阻抗（**负号 = 交叉线**）、
`F2` = 线长（m），两端并联导纳为 0。

用法（在仓库根目录）：
    uv venv --python 3.12 /tmp/necvenv && uv pip install --python /tmp/necvenv/bin/python --only-binary :all: PyNEC
    cargo build -q --release -p ham-web-nec --example deck_report
    # 若设置了 CARGO_TARGET_DIR：export DECK_REPORT=$CARGO_TARGET_DIR/release/examples/deck_report
    /tmp/necvenv/bin/python -B crates/nec/reference/tl_necpp.py            # 两根振子：共段（激励在 11 段）
    FEED_SEG=5 /tmp/necvenv/bin/python -B crates/nec/reference/tl_necpp.py # 两根振子：分段（激励在 5 段）
    /tmp/necvenv/bin/python -B crates/nec/reference/tl_necpp.py lpda       # 8 元件 LPDA 扫频
    /tmp/necvenv/bin/python -B crates/nec/reference/tl_necpp.py template   # 方案库 LPDA 模板的 5 个频点
共段 / 分段各跑一遍就是 `nec.rs` 里 `transmission_lines_agree_with_necpp` 注释引用的 96 组
（2 种地面 × 4 组间距 / 线长 × 6 个 Z0 × 2 种激励位置）。

PyNEC 版本 2.4.0。最大增益在 θ 0–180°、φ 0–355°（5° 步进）里取；自由空间与带地面都算。
"""

import math
import os
import re
import subprocess
import sys

from necpp_deck import run_deck

# 可执行文件路径：默认在 `target/` 下；`CARGO_TARGET_DIR` 改过就用环境变量 `DECK_REPORT` 指过去。
EXAMPLE = os.environ.get("DECK_REPORT", "target/release/examples/deck_report")
N_THETA = 37  # θ 0–180°，整个球面


def necpp(deck: str):
    return run_deck(deck, N_THETA)


def ours(deck: str):
    """`(R, X, 最大增益, 方位, 前后比)`，与 `necpp` 同一口径。"""
    out = subprocess.run([EXAMPLE], input=deck, capture_output=True, text=True)
    m = re.search(r"Z=(\S+) (\S+) SWR=(\S+) G=(\S+) AZ=(\S+) FB=(\S+)", out.stdout)
    if out.returncode != 0 or not m:
        raise RuntimeError(f"deck_report 失败（退出码 {out.returncode}）：{out.stdout}{out.stderr}")
    return float(m[1]), float(m[2]), float(m[4]), float(m[5]), float(m[6])


def dipoles(spacing_m: float, tl_len: float, z0: float, ground: bool, feed_seg: int):
    """两根平行的半波振子（沿 z），元件 1 馈电，TL 接两根振子的中段。"""
    h = 5.1
    zc = 12.0 if ground else 0.0
    cards = [
        f"GW 1 21 0 0 {zc - h} 0 0 {zc + h} 0.005",
        f"GW 2 21 {spacing_m} 0 {zc - h} {spacing_m} 0 {zc + h} 0.005",
        f"GE {1 if ground else 0}",
        f"TL 1 11 2 11 {z0} {tl_len} 0 0 0 0",
        f"EX 0 1 {feed_seg} 0 1 0",
        "FR 0 1 0 0 14.2 0",
        "EN",
    ]
    return "\n".join(cards) + "\n"


def lpda(freq_mhz: float, n: int = 8, tau: float = 0.9, sigma: float = 0.14, z0: float = -100.0):
    """对数周期偶极阵：元件沿 y、排列沿 x（最长的在 x = 0，向顶点变短），相邻元件中心之间接一条线，
    激励在最短的元件中心（顶点）。`z0 < 0` 是交叉线（相邻元件反相馈电，LPDA 的做法）。"""
    h = [5.8 * tau**i for i in range(n)]  # 半长
    x = [0.0]
    for i in range(n - 1):
        x.append(x[-1] + 2 * sigma * 2 * h[i])
    cards = []
    for i in range(n):
        cards.append(f"GW {i + 1} 21 {x[i]:.6f} {-h[i]:.6f} 0 {x[i]:.6f} {h[i]:.6f} 0 0.005")
    cards.append("GE 0")
    for i in range(n - 1):
        # 线从元件 i 接到元件 i+1：中间元件的中心段上挂两个端口，顶点那一段上还有激励源
        cards.append(f"TL {i + 1} 11 {i + 2} 11 {z0} {x[i + 1] - x[i]:.6f} 0 0 0 0")
    cards += [f"EX 0 {n} 11 0 1 0", f"FR 0 1 0 0 {freq_mhz} 0", "EN"]
    return "\n".join(cards) + "\n"


def template_deck(template_id: str) -> str:
    """从 `nec_templates.rs` 里取出某个模板的 `.nec` 文本。"""
    src = open("crates/nec/src/nec_templates.rs", encoding="utf-8").read()
    m = re.search(rf'id: "{re.escape(template_id)}",\s*name: "[^"]*",\s*text: "\\\n(.*?)",\n', src, re.S)
    if not m:
        raise SystemExit(f"找不到模板 {template_id}")
    return m.group(1)


def compare(name: str, deck: str, stats):
    a = necpp(deck)
    b = ours(deck)
    dz = math.hypot(a[0] - b[0], a[1] - b[1]) / max(1.0, math.hypot(a[0], a[1]))
    dg = abs(a[2] - b[2])
    dfb = abs(a[4] - b[4])
    stats.append((dz, dg, dfb))
    print(
        f"{name:40s} necpp Z={a[0]:9.3f}{a[1]:+10.3f}j G={a[2]:6.3f} az={a[3]:4.0f} FB={a[4]:6.2f}"
        f" | 本仓 Z={b[0]:9.3f}{b[1]:+10.3f}j G={b[2]:6.3f} az={b[3]:4.0f} FB={b[4]:6.2f}"
        f" | ΔZ/|Z|={dz:.4f} ΔG={dg:.3f} ΔFB={dfb:.2f}"
    )


def summary(stats):
    print(
        f"最坏：ΔZ/|Z| = {max(s[0] for s in stats):.4f}，ΔG = {max(s[1] for s in stats):.3f} dB，"
        f"ΔFB = {max(s[2] for s in stats):.2f} dB（Python {sys.version.split()[0]}）"
    )


def main_lpda():
    stats = []
    for z0 in (-100.0, 100.0):
        for f in (12.0, 14.2, 18.1, 21.2, 24.9, 28.5):
            compare(f"LPDA 8 元件 Z0 {z0:+.0f} {f} MHz", lpda(f, z0=z0), stats)
    summary(stats)


def main_template():
    """方案库里 `lpda-14-28mhz` 在 5 个频点上与 necpp 的差（`antenna_topics` 的说明文字与单测引用它）。"""
    deck = template_deck("lpda-14-28mhz")
    stats = []
    for f in (14.2, 18.1, 21.2, 24.9, 28.5):
        compare(f"lpda-14-28mhz {f} MHz", deck.replace("FR 0 1 14.2 0.0", f"FR 0 1 {f} 0.0"), stats)
    summary(stats)


def main():
    if len(sys.argv) > 1 and sys.argv[1] == "lpda":
        return main_lpda()
    if len(sys.argv) > 1 and sys.argv[1] == "template":
        return main_template()
    feed_seg = int(os.environ.get("FEED_SEG", "11"))
    stats = []
    for ground in (False, True):
        for spacing, tl_len in [(5.3, 5.3), (5.3, 3.0), (8.0, 12.0), (3.0, 7.9)]:
            for z0 in (50.0, -50.0, 300.0, -300.0, 75.0, -75.0):
                name = f"间距 {spacing} 线长 {tl_len} Z0 {z0:+} 地面 {int(ground)}"
                compare(name, dipoles(spacing, tl_len, z0, ground, feed_seg), stats)
    summary(stats)


if __name__ == "__main__":
    main()
