# 参考实现（参数转写源）

本 crate 的链路参数与算法逐字转写自 **artlav/meteor_decoder**（GPL-3.0，
https://github.com/artlav/meteor_decoder ）—— LRPT（Meteor-M 系列气象卫星）
的经典参考实现：

- `viterbi27.pas`：卷积码 `POLYA = 79₁₀ = 0x4F`、`POLYB = 109₁₀ = 0x6D`（不取反）、
  约束长度 7、回溯 15×7；
- `met_to_data.pas`：去随机化 `prand` 表（255 字节，专用表，**不是**通用 CCSDS
  131.0-B 序列）+ 同步字 `$1ACFFC1D`（内存中按 `$1DFCCF1A` 小端存取）；
- `medet.dpr`：卷积交织器（36 分支 × 2048 延迟）与去随机化位置（ASM 之后、
  卷积编码之前 —— TX 视角）；
- `met_packet.pas`：VCDU/MPDU 解析（后续层实现时的转写源）。

注意 MetOp 的 LRPT 已退役（本仓早前路线图写的是 MetOp），改用**同一链路家族**的
Meteor-M 系列；真实过境录音可从 SatNOGS（network.satnogs.org）取观测数据。
