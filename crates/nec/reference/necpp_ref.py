"""用 PyNEC（necpp，NEC-2 的 C++ 移植）跑方案库模板，输出交叉验证参照值。

用法（在仓库根目录；与 `trap_necpp.py` 同一套装法）：
    uv pip install --python <python3> --target /tmp/necpp-pkg --only-binary :all: PyNEC
    PYTHONPATH=/tmp/necpp-pkg <python3> crates/nec/reference/necpp_ref.py
PyNEC 的预编译包不覆盖所有 Python 版本（没有 SWIG 就编不了源码包）；装不上时换一个较新的解释器。
`-B` 可免得在仓库里留下 `__pycache__`。
把输出抄进 `nec_templates.rs` 的 `NECPP_REF`；对照本实现用
    cargo run -p ham-web-nec --example template_report --release

PyNEC 版本：`NECPP_REF` 当初用 2.3.4 生成；2.4.0 下重跑，前 6 组输出与它逐位相同（LPDA 模板是 2.4.0 下新增的）。

卡片怎么转给 necpp 见 `necpp_deck.py`（GW / GE / EX / FR / TL，`GE 1` 补 `GN 1` = 理想导体地面）。
最大增益在 θ 0–90°、φ 0–355°（5° 步进）的上半空间里取。
"""

import re
import sys

from necpp_deck import run_deck

src = open("crates/nec/src/nec_templates.rs", encoding="utf-8").read()
blocks = re.findall(r'id: "([^"]+)",\s*name: "[^"]*",\s*text: "\\\n(.*?)",\n', src, re.S)

print(f"// necpp（PyNEC）参照，{sys.version.split()[0]}")
for tid, text in blocks:
    try:
        r, x, g = run_deck(text, 19)[:3]
        print(f'    ("{tid}", {r:.3f}, {x:.3f}, {g:.3f}),')
    except Exception as e:  # noqa: BLE001
        print(f"    // {tid}: necpp 拒绝 —— {e}")
