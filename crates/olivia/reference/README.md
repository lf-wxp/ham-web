# 参考实现（互操作对拍用）

参考实现已从纯 Python 移植改为 **Rust 版本**：`../tests/reference/olivia_ref.rs`
（测试内进程直连，不依赖系统 python3）。算法出处不变：

- 转写自 **sntfrc/olivia-python**（MIT License，© 2020 Federico Santandrea，
  https://github.com/sntfrc/olivia-python ）—— 原作者说明该实现「依据网上的
  文档和 C 参考实现源码确定，并与 fldigi 对比测试过」；
- 与 crate 核心（`../src/`）**不共享代码**，只共享上游出处 —— 编解码分属两套
  独立实现，双向都能还原同一段文本才是真实现。

离线对齐策略有一处如实改动：接收侧从「先到先得 + 滚动」改为「取零存疑且总
置信度最高的对齐」（干净合成信号里 preamble 的错位窗口可能恰好过门限），门限
`BLOCK_THRESHOLD = 24` 本身保持不变 —— 见 `olivia_ref.rs` 注释。

与本仓库的关系：只作互操作对拍用（`cargo test -p ham-web-olivia --test
reference_interop`），不随前端分发。
