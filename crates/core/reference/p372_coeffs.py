"""从官方 `COEFFxxW.txt` 抽出 P.372 大气噪声用到的四张系数表，生成 Rust 常量。

数据源：ITU-R SG3 公开仓库 `ITU-R-Study-Group-3/ITU-R-HF` 的 `P372/Data/COEFFxxW.txt`
（12 个月各一份）。仓库 README 写明「The ITURHFProp, P533 and P372 software … may be used by
implementers in their implementation of the Recommendation … free from any copyright assertions」，
这些系数文件是同一份包的一部分、是该算法不可拆分的输入。出处与致谢见
`crates/core/src/atmospheric_noise.rs` 的模块文档。

文件是「段名一行 + 数字若干行」的格式（段名形如 `if2(10)`、`fakp(29,16,6)`）。
**按段名解析、按官方 `ReadFamDud()` 的 reshape 公式重排**，不手抄也不猜排版。

用法：
    python3 -B crates/core/reference/p372_coeffs.py <DATA_DIR> > crates/core/src/atmospheric_coeffs.rs
    cargo fmt -p ham-web-core
`<DATA_DIR>` 指 `ITU-R-HF/P372/Data`（用 `p372_check.py` 固定的那一版 `OFFICIAL_SHA`）。
**生成之后必须跑 `cargo fmt`**：脚本每行只输出一整行数组，入库的是 rustfmt 展开后的版本，
不格式化直接提交会让 `cargo make fmt-check` 红。
"""

import pathlib
import re
import sys


def dims_of(line):
    """`fakp(29,16,6)` → `("fakp", [29, 16, 6])`；不是段名返回 `(None, [])`。"""
    m = re.fullmatch(r"(\w+)\(([\d,\s]+)\)", line.strip())
    if not m:
        return None, []
    return m.group(1), [int(v) for v in m.group(2).split(",") if v.strip()]


def parse(path):
    """返回 `{段名: 数字列表}`（保持文件顺序）。"""
    sections = {}
    cur = None
    for raw in path.read_text(errors="replace").splitlines():
        stripped = raw.strip()
        name, _ = dims_of(stripped)
        if name is not None:
            cur = name
            sections[cur] = []
            continue
        if cur is None:
            continue  # 文件头
        for tok in stripped.split():
            try:
                sections[cur].append(float(tok))
            except ValueError:
                # DOS 的 Ctrl-Z 之类的杂字符。
                if tok.strip("\x1a\x00 \t"):
                    raise
    return sections


def fmt_f64(v):
    """`f64` 的最短往返十进制（Python 的 `repr`，与 Rust 的 `Debug` 同口径）。"""
    return repr(float(v))


def need(sections, name, dims):
    vals = sections.get(name)
    if vals is None:
        raise SystemExit(f"缺段 {name}")
    want = 1
    for d in dims:
        want *= d
    if len(vals) != want:
        raise SystemExit(f"{name}: 读到 {len(vals)} 个数，按维度 {dims} 应有 {want}")
    return vals


def main():
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    data = pathlib.Path(sys.argv[1])

    fakp, fakabp, dud, fam = [], [], [], []
    for month in range(1, 13):
        s = parse(data / f"COEFF{month:02d}W.txt")
        # fakp：官方 `noiseP->fakp[i][j][k] = A[16*29*i + 29*j + k]`
        #   i = 时段（0..5）、j = 系数序号（0..15）、k = 阶（0..28）。
        v = need(s, "fakp", (29, 16, 6))
        fakp.append(
            [[[v[16 * 29 * i + 29 * j + k] for k in range(29)] for j in range(16)] for i in range(6)]
        )
        # fakabp：`fakabp[i][j] = A[2*i + j]`，i = 时段。
        v = need(s, "fakabp", (2, 6))
        fakabp.append([[v[2 * i], v[2 * i + 1]] for i in range(6)])
        # dud：`dud[i][j][k] = A[5*12*i + 5*j + k]`，i = 5 类偏差、j = 12 行、k = 5 项。
        v = need(s, "dud", (5, 12, 5))
        dud.append(
            [[[v[5 * 12 * i + 5 * j + k] for k in range(5)] for j in range(12)] for i in range(5)]
        )
        # fam：`fam[j][k] = A[14*j + k]`，j = 12 行、k = 14 项。
        v = need(s, "fam", (14, 12))
        fam.append([[v[14 * j + k] for k in range(14)] for j in range(12)])

    out = [
        "//! P.372 大气噪声的系数表（生成物，不要手改）。",
        "//!",
        "//! 由 `crates/core/reference/p372_coeffs.py` 从 ITU-R SG3 的 `ITU-R-HF` 仓库",
        "//! （`P372/Data/COEFFxxW.txt`，12 个月各一份）抽出四张表：`FAKP`（经度球谐级数）、",
        "//! `FAKABP`（纬度线性项）、`DUD`（十分位偏差）、`FAM`（频率多项式）。",
        "//! 出处、许可与致谢见 `atmospheric_noise` 的模块文档。",
        "//!",
        "//! 系数按官方文件的双精度原样存 `f64`（不量化、不降精度）：这是「与官方程序逐位一致」的前提。",
        "//! 试过 `f32`（最大差 1.5e-5 dB）与 `i16`（2.5e-2 dB）省体积，验收口径是逐位一致，所以不用。",
        "",
        "/// 经度球谐系数：`FAKP[月 − 1][时段][系数序号（0..14 正弦项、15 常数项）][阶]`。",
        "pub(super) static FAKP: [[[[f64; 29]; 16]; 6]; 12] = [",
    ]
    for m in fakp:
        out.append("  [")
        for t in m:
            out.append("    [")
            for row in t:
                out.append("      [" + ", ".join(fmt_f64(v) for v in row) + "],")
            out.append("    ],")
        out.append("  ],")
    out += [
        "];",
        "",
        "/// 纬度的 `a + b·(φ + π/2)` 两项：`FAKABP[月 − 1][时段][2]`。",
        "pub(super) static FAKABP: [[[f64; 2]; 6]; 12] = [",
    ]
    for m in fakabp:
        out.append("  [")
        for row in m:
            out.append("    [" + ", ".join(fmt_f64(v) for v in row) + "],")
        out.append("  ],")
    out += [
        "];",
        "",
        "/// 十分位偏差的频率多项式：`DUD[类（0 = Du、1 = Dl、4 = SigmaFam）][行（南半球时段 +6）][5 项]`。",
        "pub(super) static DUD: [[[[f64; 5]; 12]; 5]; 12] = [",
    ]
    for m in dud:
        out.append("  [")
        for row in m:
            out.append("    [")
            for j in row:
                out.append("      [" + ", ".join(fmt_f64(v) for v in j) + "],")
            out.append("    ],")
        out.append("  ],")
    out += [
        "];",
        "",
        "/// 频率多项式系数：`FAM[月 − 1][行][0..7]` 是 F_a 的 z 多项式、`[7..14]` 是 x 项。",
        "pub(super) static FAM: [[[f64; 14]; 12]; 12] = [",
    ]
    for m in fam:
        out.append("  [")
        for row in m:
            out.append("    [" + ", ".join(fmt_f64(v) for v in row) + "],")
        out.append("  ],")
    out.append("];")
    print("\n".join(out))


if __name__ == "__main__":
    main()
