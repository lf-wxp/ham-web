# 增量扩展路线图

> 「大而全」的建设清单与进度追踪。当前站点已具备骨架：6 大分组 **100+ 专题知识页**、
> **41 项计算器**、**34 个交互工具页**、A/B/C 题库与备考闭环。
> 本路线图只记录**缺口与增量**，已实现能力见 [`README.md`](README.md)。

状态图例：`[x]` 已完成 · `[~]` 已部分完成 · `[ ]` 待处理 · `[-]` 评估后不做

---

## 目录

- [建设原则](#建设原则)
- [现状基线](#现状基线)
- [P0 · 从静态到可计算](#p0--从静态到可计算)
- [P1 · 天线仿真与日志闭环](#p1--天线仿真与日志闭环)
- [P2 · 设备 DIY 结构化与法规原文](#p2--设备-diy-结构化与法规原文)
- [P3 · 内容深度与国际化](#p3--内容深度与国际化)
- [P4 · 硬件互联与解码器矩阵](#p4--硬件互联与解码器矩阵)
- [横向能力（决定"全"而不"散"）](#横向能力决定全而不散)
- [工程门禁](#工程门禁)
- [验证方式](#验证方式)

---

## 建设原则

1. **隐私优先**：个人数据只存浏览器本地（`localStorage` / `IndexedDB`），不上传服务器。需要服务端参与的环节一律做成「本地加密/状态合并」而非原始数据托管。
2. **离线优先**：纯计算与静态知识必须无网可用；依赖 `/api/*` 的页面降级为「数据暂不可用」而非报错。
3. **单一事实来源**：波段边界已在 `bands::AMATEUR_BAND_EDGES` 收敛；功率限值/卫星标志/呼号前缀等同理，新增数据不得复制常量。
4. **内核下沉**：不依赖浏览器的逻辑放 `ham-web-core` 并配单元测试；大内核（DSP / 求解器）拆到 Web Worker，不塞进主包。
5. **可计算 > 可阅读**：新增专题优先做「能调参数、出曲线」的交互，其次才是文字。

---

## 现状基线

| 模块 | 已完成 | 主要缺口 |
| --- | --- | --- |
| 基础理论 | `modulation_theory` / `digital_comms` / `dsp_basics` / `electronics` / `filters` / `analog_modes` 六模块 + 专题页 | 无交互式仿真；知识项缺 `source` / `updated` 溯源 |
| 天线 | 软件对比、建模步骤、**解析式**方向图、偶极计算器、Smith 换算、线圈/Yagi 振子 | **无数值求解器**，无 3D 方向图，无 `.nec` 互导 |
| 日志与 QSL | ADIF 3.1、IndexedDB、呼号补全、LoTW/eQSL 粘贴同步、QSL 标签打印、奖状进度、Cabrillo、CAT 联动 | 同步为单向手工；无卡片影像；无多台站/多操作员；无冲突合并 |
| 传播 | 简化 VOACAP、太阳活动/X 射线/警报、DX Cluster、灰线、SGP4 过境、PSK/RBN/WSPR | 无小时级热图；无 SNR 预测；无真实 VOACAP 内核 |
| 设备与 DIY | `gear` 机型库 + 对比表、`receiver` 指标、`diy_projects`、`antenna_diy` | 无实测数据与结构化评测；DIY 是文章列表而非可执行项目 |
| 法规与题库 | 题库 + 内容指纹 + 2000 条解析 + 术语注入 + 完整备考闭环 | 法规原文未结构化；题目无法参数化变体 |

---

## P0 · 从静态到可计算

目标：把已有的公式与数据表变成可调参数、出曲线的工具。

### 波形实验室 `/waveform-lab`

- [ ] 时域 / 频域联动面板：AM / FM / SSB 调制波形，载波与边带功率分配条（复用 `crates/core/src/fft.rs`）
- [ ] 星座图 + 眼图：BPSK / QPSK / 16QAM，可调 SNR 观察噪声扩散与判决门限
- [ ] 滤波器响应曲线：Butterworth / Chebyshev / 椭圆，阶数滑块 → 幅频 / 相频 / 群延迟（数据已在 `filter_design.rs`）
- [ ] 奈奎斯特-香农沙盒：`C = B·log₂(1+S/N)` 三滑块实时求解链路上限
- [ ] 落地：`crates/core/src/waveform_lab.rs` + `crates/app/src/pages/waveform_lab.rs` + `registry.rs` 一条 + `main_content.rs` 一行

### Smith 圆图交互

- [ ] `smith.rs` 从换算升级为可拖拽阻抗点的圆图组件
- [ ] 串 / 并联 L、C 元件即时走轨迹，输出匹配网络取值与可用带宽
- [ ] 与 `/tools` 现有「天线匹配网络」计算器互链

### 传播预测 2.0

- [ ] **24 小时 × 波段热力矩阵**：双方网格 + 日期 + SSN → 每波段每小时 MUF / LUF / 可用度 / 可靠度
- [ ] 矩阵格子可下钻查看路径细节（仰角、跳数、路径长度）
- [ ] 智能推荐卡：结合实时 Kp / A / 太阳通量，直接给「现在打 XX 用哪个波段」排序
- [ ] 服务端保留简化模型作为离线回退（`/api/v1/propagation/muf` 契约不变）

---

## P1 · 天线仿真与日志闭环

目标：把站点从「能查」推向「能设计、能沉淀」。

### NEC-2 精简矩量法求解器

- [ ] 新建 `crates/nec`：线天线 GW / GE / EX / LD / FR 卡片子集 → Pocklington + 矩量法 → 复阻抗 Z(f)、增益、F/B、方向图、电流分布
- [ ] `crates/nec-worker`：编译为 Web Worker（沿用 `ham-web-apt` 的 `wasm-bindgen --target no-modules` 流水线）+ `cargo make build-nec-worker`
- [ ] 规模预算：前端 ≤ 200 段；更大模型走 `/api/v1/antenna/solve` 服务端异步
- [ ] 单元测试：半波偶极 `Z ≈ 73+j42 Ω`、增益 ≈ 2.15 dBi 作为基准断言

### 天线可视化编辑器

- [ ] Canvas 线框编辑：坐标输入 / 拖拽 / 阵列复制，段数警告与波长归一化
- [ ] **3D 方向图**：增益 dB 上色球壳（WebGL2，复用 `crates/app/src/web_threads.rs` 的 GL 封装）
- [ ] 2D 切片：水平 / 垂直方向图叠加、仰角-距离覆盖图（NVIS 场景）
- [ ] `.nec` 文本 ↔ 可视化几何双向导入导出，便于与 4NEC2 / EZNEC 交叉验证
- [ ] 地面参数 σ/ε 与导线损耗模型 → 效率与真实增益
- [ ] 全波段扫频 SWR 曲线（11 点插值到连续曲线）
- [ ] 设计向导：波段 + 目标增益 + 臂长 → Yagi 单元数 / 间距 / 振子长度
- [ ] 方案库：`practical_antennas` / `antenna_diy` 每条方案挂可加载 `.nec` 模板

### QSL 全生命周期

- [ ] 状态机：`未寄出 → 已寄出（卡片局 / 直寄）→ LoTW 已确认 / eQSL 已确认 / 纸质已收`，四色徽章
- [ ] 卡片影像：扫描件缩略图存 IndexedDB，与 QSO 关联
- [ ] `/qsl-designer` 闭环：模板 + 呼号/网格/RST 自动填入 → 导出 PDF / PNG → 打印
- [ ] Club Log / eQSL / QRZ Logbook 多源三向 diff（本地 / 远端 / 冲突），逐条人工裁决（扩展 `qsl_sync.rs`）
- [ ] LoTW TQSL 签名上传流程指引 + 状态回写（服务端只做状态合并，不做代签）

### 日志基础设施

- [ ] 多台站档案：固定台 / 车载 / 野外 / POTA，`station-info` 升级为列表，日志归属台站
- [ ] 多操作员分账：`OPERATOR` / `MY_*` 字段，奖状进度按台站与操作员分面统计
- [ ] 日志体检报告：缺网格、时区错、频率越界（用 `bands::AMATEUR_BAND_EDGES` 判定）、模式与频率不匹配、重复 QSO、DXCC 前缀存疑
- [ ] 一键批量修正

---

## P2 · 设备 DIY 结构化与法规原文

### 机型库结构化 `/gear`

- [ ] 统一 schema：频段 / 模式 / 功率 / 接收机指标（灵敏度、RMDR、相位噪声）/ 接口（CAT / CI-V、USB 音频）/ 功耗 / 重量 / 参考价 / 固件生态
- [ ] 对比器升级：雷达图 + 差异高亮，4 机同屏
- [ ] 按权重打分：DX / 竞赛 / 野外 / 卫星 四种预设权重
- [ ] 用户实测值录入（本地保存）叠加到对比图：RMDR / 相位噪声 / 工作电流

### 评测文章体系

- [ ] 模板化：测试条件（天线 / 环境 / 仪表）→ 指标实测 → 主观评价 → 适用人群 → 结论
- [ ] 条件前置保证可比性；指标名词与术语表互链（悬停释义）

### DIY 项目工程化 `/diy-projects`

- [ ] 结构化卡片：难度 / 成本 / 工时 / BOM（可导出）/ 原理图与 PCB 链接 / 固件仓库 / 测试步骤 / 常见问题
- [ ] 筛选器：难度、成本、波段、所需仪表（电桥 / VNA / 示波器）
- [ ] 制作进度追踪：项目打勾、BOM 采购状态勾选

### 仪表与测量教学

- [ ] 驻波测量误差来源交互演示
- [ ] VNA SOLT 校准步骤可视化（`vna.rs`）
- [ ] 功率计与驻波读数陷阱（`meters.rs` / `antenna_analyzer.rs`）

### 法规原文库

- [ ] 《无线电管理条例》《业余无线电台管理办法》（工信部令 67 号）《频率划分规定》条文结构化，条款级检索
- [ ] 题目 ↔ 条款双向关联：解析引用「第 X 条」可点直达原文；原文页列出引用它的全部题目

### 题库增强

- [ ] **计算题参数化变体**：波长 / dB / 功率 / 衰减 / LC 谐振类题生成同型变体，防背答案；变体仅本地生成，不污染官方内容指纹
- [ ] 错题归因图谱：`/mistake-topics` 升级为知识树（法规 / 电路 / 天线 / 传播 / 操作）热力图，点击直达补弱练习
- [ ] 模考产品化：真实倒计时 + 考场规则演练 + 证件照（`/photo-processor`）一条龙

---

## P3 · 内容深度与国际化

### 知识溯源

- [ ] 为知识项增加 `source`（法规版本 / 标准号 / URL）与 `updated`（时效）字段，页面标注「依据 XX，YYYY-MM 核对」
- [ ] 时效性数据（法规版本、卫星状态、频率惯例）优先补齐

### 原理层缺口

- [ ] **频谱管理深化**：频谱监测与干扰查处流程；ITU RR 业余业务条款结构；共用条件细则；频率申请与指配流程
- [ ] **协议标准深化**：ITU 发射类别完整表（现仅 7 项）；PACTOR / ARDOP / VARA / JS8 机理；DMR 时隙 / 色码 / Tier；LoTW / TQSL 流程
- [ ] **天线理论深化**：近场/远场边界 `r = 2D²/λ`；天线 Q 与带宽；噪声温度与 G/T；互耦
- [ ] **单一事实来源（功率 / 卫星标志）**：彻底收敛 `regulations` / `license_classes` / `reference` / `categories` / `bands.rs` 中的分散常量

### 学习路径与图谱

- [ ] `/learning-path` 从静态列表升级为 DAG：专题前置依赖图，答错自动回推前置专题
- [ ] 专题页统一挂「自测 3 题」（`topic_quiz.rs`）+「相关主题」（`related.rs`），织成知识图谱
- [ ] 公式改用 KaTeX / MathJax 渲染，便于搜索与 i18n

### 国际化

- [ ] `data/knowledge-i18n/` 知识库正文 en / es 补全（缺失自动回退中文，已有机制）
- [ ] 新增语言只需补一份 `crates/app/src/i18n.rs` 词典 + 一份正文译文
- [ ] 多国题库对照：FCC Technician / General / Extra、Ofcom Foundation / Intermediate / Full

---

## P4 · 硬件互联与解码器矩阵

### 解码器扩展

- [ ] RTTY、Olivia、DominoEX、NAVTEX 解码 Worker
- [ ] 气象卫星 LRPT（MetOp）解码
- [ ] 统一 Worker 协议与 `cargo make <name>-sample` / `-decode` / `-test` 样本命令

### 硬件互联

- [ ] KISS TNC over Web Serial：Packet / APRS 实际收发（现有 `aprs_codec.rs` 已具备编解码内核）
- [ ] 蓝牙 CAT（Web Bluetooth）
- [ ] WebUSB 仪表读数（天线分析仪 / VNA）

### 开放 API

- [ ] 把纯计算内核逐步开放为 `/api/v1/*`：天线求解、滤波设计、链路预算、单位换算
- [ ] 契约单一事实来源仍在 `crates/core/src/api_v1.rs`，文档页 / `openapi.json` / 路由测试共用

---

## 横向能力（决定"全"而不"散"）

| 能力 | 现状 | 目标 | 状态 |
| --- | --- | --- | --- |
| 统一搜索 | `/` 唤起命令面板，检索术语 / 简语 / 知识条目 | 扩展到工具、页面、日志 QSO、法规条文；支持 `dxcc:` `band:` 等领域前缀语法 | [ ] |
| 存储门面 | `kv.rs` + IndexedDB 双写，容量预警 | 新增大数据（QSL 影像、天线方案）统一走 `kv` | [ ] |
| 数据备份 | 导出 / 导入 / 按类型合并 | 纳入 QSL 影像与天线方案的导出（受体积限制，提供按模块勾选） | [ ] |
| 无障碍与冒烟 | 全路由自动纳入 `smoke.spec.ts` + axe | 新增交互工具需补专属 e2e 用例与 `coverage-targets.json` 旅程 | [ ] |
| 性能预算 | wasm-opt `z`、预压缩、Worker 拆分 | 新增大内核前先评估主包体积增量 | [ ] |

---

## 工程门禁

新增页面 / 模块必须满足（仓库内已有自动化守护，新增内容自动受益）：

1. **只改两处**：`crates/core/src/registry.rs` 的 `MODULES` 追加一条 + `crates/app/src/app/main_content.rs` 加一行 `<Route>`。
   导航菜单、`sitemap.xml` 均由注册表派生，`registry_check.rs` 断言「注册表 ↔ 导航 ↔ 路由」一致，漏注册会让 `cargo make check` 直接失败。
2. **内核下沉**：不依赖浏览器的逻辑放 `ham-web-core` + 单元测试；技术性数值在 `knowledge_consistency.rs` 加回归断言。
3. **大内核走 Worker**：DSP / 求解器按 `ham-web-apt` 的模式拆 Worker，注意 `wasm-bindgen` CLI 版本与 `Cargo.lock` 一致。
4. **文案与翻译**：界面文案同步补 `crates/app/src/i18n.rs`（`cargo make i18n-check` 校验重复 key 与占位符），正文译文放 `data/knowledge-i18n/`。
5. **测试**：新增交互补 Playwright 用例，并把旅程追加到 `e2e/coverage-targets.json`（`grep` 子串需唯一命中）。

---

## 验证方式

```bash
cargo test -p ham-web-core            # 领域逻辑 + knowledge_consistency 回归
cargo test -p ham-web-app             # registry_check（注册表 ↔ 路由一致性）
cargo make check                      # fmt + clippy + test + 文案词典 + 解析表
cargo make e2e                        # 构建 release + Playwright 全站冒烟与无障碍
cargo make e2e-coverage               # 覆盖率报告 e2e/test-results/coverage.md
```

---

## 相关文档

| 文档 | 作用 |
| --- | --- |
| [`README.md`](README.md) | 已实现能力的完整说明、路由表、开发指南 |
| [`docs/explanations-style.md`](docs/explanations-style.md) | 题目解析写作规范 |
| [`docs/typography.md`](docs/typography.md) | 题库排版规则（内容指纹稳定性依赖） |
