# 参考实现（互操作对拍用）

参考实现已从纯 Python 移植改为 **Rust 版本**：`../tests/reference/dominoex_ref.rs`
（测试内进程直连，不依赖系统 python3）。算法与表数据出处不变：

- 算法转写自 **fldigi**（GPL-3.0，https://github.com/w1hkj/fldigi ，
  `src/dominoex/`）—— IFK+ 编解码逻辑逐行保持原意，去掉 fltk/音频设备依赖；
- Varicode 表（512 码 + 4096 解码项）转写自同源的 `dominovar.cxx`
  （© 2001–2003 Tomi Manninen OH2BNS），与 crate 核心共用**生成物表数据**
  `ham_web_dominoex::varicode`，算法代码互不共享；
- 生成脚本：`../examples/extract_varicode.rs` —— 从 fldigi 的 `dominovar.cxx`
  生成 Rust（`../src/varicode.rs`）与 TS（`e2e/tests/dominoex_varicode_data.ts`）
  两份表数据，内置 512 项往返自检（已知例外：二次信道的 379 与 381 在上游表
  中同码，如实保留）。

与本仓库的关系：只作互操作对拍用（`cargo test -p ham-web-dominoex --test
reference_interop`），不随前端分发。
