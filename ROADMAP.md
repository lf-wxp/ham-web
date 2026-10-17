# 路线图

> 只记三件事：**还没做的**、**刻意不做的**、以及**新增内容必须过的门禁**。
> 已实现的功能见 [`README.md`](README.md#主要特性)；实现细节、设计取舍与精度边界写在各模块的
> 文档注释里（例如 `crates/nec/src/nec.rs` 的离散方式与精度表、`crates/app/src/kv.rs` 的写入版本仲裁），
> 不在这里重复。

**分区即状态**：条目在哪个分区就是什么状态（未完成 / 刻意不做 / 已完成）。
一项做完就从「未完成」删掉、在「已完成」补一句结论与指针；评估后放弃的移进「刻意不做」并写原因。
三个分区的条目都不再回填进度日志。

---

## 目录

- [建设原则](#建设原则)
- [现状一览](#现状一览)
- [未完成](#未完成)
- [刻意不做与设计边界](#刻意不做与设计边界)
- [已完成](#已完成)
- [工程门禁](#工程门禁)
- [验证方式](#验证方式)
- [相关文档](#相关文档)

---

## 建设原则

1. **隐私优先**：个人数据只存浏览器本地（`localStorage` / `IndexedDB`），不上传服务器。需要服务端参与的环节一律做成「本地加密 / 状态合并」而非原始数据托管。
2. **离线优先**：纯计算与静态知识必须无网可用；依赖 `/api/*` 的页面降级为「数据暂不可用」而非报错。
3. **单一事实来源**：波段边界、功率限值、卫星状态、呼号前缀等常量各有唯一出处（如 `bands::AMATEUR_BAND_EDGES`、`license_classes`），新增数据不得复制常量，跨模块一致性由测试守护。
4. **内核下沉**：不依赖浏览器的逻辑放 `ham-web-core`（或独立 crate）并配单元测试；大内核（DSP / 求解器）拆到 Web Worker，不塞进主包。
5. **可计算 > 可阅读**：新增专题优先做「能调参数、出曲线」的交互，其次才是文字。
6. **能验证才上**：没有参考实现、参考数据或真实样本可对拍的算法与数字，宁可不做或如实标「未收录」，也不编一个看起来像真的结果。

---

## 现状一览

| 模块 | 已覆盖（详见 README） | 已知边界 |
| --- | --- | --- |
| 考试与题库 | [考试中心](README.md#考试中心)：A/B/C 题库、练习与模拟考试、错题与间隔复习、计算题变体、错题归因图谱、机考规则演练 | 变体题不进模拟考；FCC / Ofcom 题库全文未收录 |
| 知识库 | [知识库](README.md#知识库)：100+ 专题、法规原文库、知识溯源、KaTeX 公式、学习路径图、多国考试对照 | 溯源只覆盖时效性领域，原理类知识不标 |
| 天线与射频 | [天线与射频设计](README.md#天线与射频设计)：矩量法求解器、Smith 圆图、波形实验室、测量误差实验室 | 地面用反射系数近似；交叉线 `TL` 与并联导纳不支持；交叉验证参照是另一套 NEC-2 实现，不是实测 |
| 传播与实时数据 | [工具与实时数据](README.md#工具与实时数据)：24 小时 × 波段热力矩阵、太阳活动、DX Cluster、PSK Reporter / RBN、卫星过境 | 简化 VOACAP 模型，无 SNR 预测；噪声预算按 290 K 热噪声，未接 ITU-R P.372 外部噪声；地磁扰动只提示不建模 |
| 日志与 QSL | [通联日志与 QSL](README.md#通联日志与-qsl)：ADIF 3.1、多台站 / 操作员与统计范围、日志体检、QSL 全生命周期与多源确认同步、卡片设计与标签打印、竞赛与奖状 | 确认报告靠手动导入（无 OAuth 直连）；模式分段校验只覆盖话务 |
| 解码与硬件 | [浏览器内解码与硬件互联](README.md#浏览器内解码与硬件互联)：10 个解码器（9 个走 Worker，PSK31 在主线程）、CAT、KISS TNC、NanoVNA | LRPT 的 OQPSK 只在合成信号上验证；串口收发需真实硬件，自动化只覆盖协议层与文件入口 |
| 设备与 DIY | [设备与 DIY](README.md#设备与-diy)：机型库实测指标与加权打分、评测文章、DIY 项目卡片 | 实测数据只覆盖 HF 级接收机；灵敏度、接口、功耗、售价无可引用来源，刻意留空 |
| 国际化 | [用户体验](README.md#用户体验)：界面与知识库正文中 / 英 / 西三语全覆盖 | 法规原文保持中文（只有官方版本有规范效力） |

---

## 未完成

按可行性排序。可行性：高 = 数据与参照都在、只差实现；中 = 需要外部样本或硬件才能闭环；低 = 缺可信来源或收益不抵成本。

| 条目 | 可行性 | 阻塞点 / 下一步 |
| --- | --- | --- |
| 竞赛录入页的实体名三语 | 高 | 已知遗留（不是 SCP 补全引入）：录入页实体提示行（`美国 · NA · CQ 5`）与通联表的「实体」列在 en / es 界面仍显示中文国家名。属纯词条 / 数据口径，不涉及判定逻辑 |
| 解码器补位：WEFAX 气象传真 / Feld Hell | 高 | 均为「有公开样本、可合成对拍」的高价值缺口：WEFAX（AM 调制 120/240 LPM 图像传真）与 APT/LRPT 同脉；Feld Hell 点阵量小，可作新解码器样板 |
| 低频段专题（2200m / 630m / 160m） | 高 | 135.7–137.8kHz / 472–479kHz 新开放，`/bands` 只到 70cm；依据可标核对日期 |
| 天线共模扼流 / ferrite 抑制 | 高 | HF 台站最常见的实操（馈线上套 FT-240-43），现在只有 `/balun` 与 `/rfi` 的文字。数据已有一半：磁环库 4 个 Fair-Rite 零件的 AL / 尺寸已逐个核对，零件页同时给阻抗曲线 → 算 N 匝共模阻抗、插损与自谐振点；对拍用官方阻抗表 + 现有 AL 一致性断言 |
| SatNOGS 全量转发器库 | 高 | 现只有 5 个线性转发器（AO-7 A/B、FO-29、RS-44、AO-73），FM 卫星与 ISS digipeater 全缺，而后者才是新手真正会用的。SatNOGS DB 是 CC BY-SA 4.0，按其署名要求入库；上下行等宽、卫星业余业务段等护栏已有 |
| 外部噪声模型（ITU-R P.372）+ 台站链路预算向导 | 中 | 精度缺口里最要紧的一条：`pages/tools/receiver_sensitivity.rs` 只有 `-174 + 10lg(BW) + NF`（纯 290 K），而 80 m 上安静乡村的外部噪声比热噪声高 40–50 dB、城市可高 60 dB——这个计算器现在只在 144 MHz 以上、电磁环境干净时成立。同一改动把 52 个孤立计算器串成「TX 功率 → 馈线 → 天线 → 路径 → RX 天线 → 外部噪声 → 所需 SNR」一条链路，输出余量与最可能失败的环节。阻塞：P.372 的噪声系数-频率曲线多为图，需先取到可程序化的数值表（或按图数字化并记核对日期）；落点 `core/noise_model.rs` + 薄页面 |
| 地波场强（ITU-R P.368） | 中 | 全站传播只有天波与视距，而 630 m / 160 m / 中波收听、NAVTEX、NDB 全靠地波，现在一个数都没有。ITU 提供 P.368 的场强数据文件，可逐点对拍，地面电导率取 ITU 区域图。阻塞：先确认数据文件的下载与许可 |
| EME 计划器 | 中 | `/eme` 只有文字，`core/astro.rs` 只有太阳几何。月位（Meeus 简化月球理论）、月相、月面噪声温度（随月相约 200–1500 K）、月距多普勒、共同可见窗口，面板与 `/satellites` 同形。对拍 Meeus 书内算例 + JPL Horizons 公开数值 |
| 陷波器（trap）/ 梯形馈线 / 多波段偶极子计算 | 中 | 天线 DIY 只有长度与匹配网络（阅读型内容归「天线补强专题」那条）。trap 谐振与 ladder line 阻抗变换用站内 NEC 自证：加 trap 的模型扫频应比无 trap 多一个谐振点 |
| APRS 软 TNC（音频 → AX.25） | 中 | `/kiss-tnc` 要一台真 TNC，`/aprs` 只有文本层编解码。麦克风 / WAV → AFSK1200（Bell 202）→ HDLC → AX.25 帧，是「零硬件玩 APRS」的最后一环。注意 direwolf 是 GPL-3.0：读公开协议自写解调，或照已有 6 个 GPL crate 的先例单独隔离 |
| 流星雨时刻表 | 中 | `/meteor-scatter` 有原理、`/events` 无 MS 窗口。IMO 年度流星雨表（峰值 ZHR、辐射点、日期）串到倒计时与 `/events`，与 MSK144 / FSK441 解码配套。阻塞：IMO 数据的引用许可待确认 |
| LRPT：M2-3 / M2-4 的 OQPSK 真实录音 | 中 | M2（QPSK）已在真实录音上出图（`crates/lrpt/tests/real_iq.rs`；录音不进仓库，用 `LRPT_REAL_IQ` 给路径）。现役的 M2-3 / M2-4 是 OQPSK，定时与载波环路要按 I/Q 半符号错位改写；**缺一段真实 OQPSK 基带**（SatNOGS 只存 48 kHz 音频与解好的图片） |
| LRPT：M2-2（80k）真实录音对拍 | 中 | 重同步、去交织、差分链路已有往返测试；缺真实 80k 录音 |
| LRPT：软判决 Viterbi | 中 | 弱信号的增益项；现有真实样本硬判决已 99.8% 帧通过，看不出收益，有更弱的样本再做 |
| 卫星联动：双 VFO 卫星模式 / 跟随旋钮 / 真机验证 | 中 | 上行写入、线性转发器、自定义通带与旋转器自动跟踪已完成（见「已完成」）。**仍未做**：① **卫星模式**（IC-9700 / IC-705 / FT-991A 的主 / 副 VFO 各管一个波段）——各机型 CAT 语义不同（Hamlib 里 IC-7600/7610/7800/785x 的 `25` 命令是 Main/Sub 而非选中 / 未选中，FT-991A 的 `FR`/`FT` 也不是 Kenwood 含义），没有真机逐款验证就上线等于没测；② **线性转发器跟随电台旋钮**——现在是页面持有调谐点、转旋钮会触发「手动改频即停」，要反过来得把回读频率同时当输入和输出，而回读最多有 3 次轮询的延迟，发射会滞后于旋钮；③ **真机验证**——上行写入（FB / `25 01` 在各机型的实际行为、分频设置、电台忙时的回应）与旋转器（GS-232 在各控制器上的应答格式、速度、限位、过顶走哪条路）**一次都没实测过**，只有假串口 |
| NanoVNA 串口实机验证 | 中 | 协议解析有单测、`.s1p` 入口有 e2e；串口收发需要真实仪器（与 KISS TNC 同一边界） |
| 磁环库补 Amidon / Micrometals 型号 | 中 | 业余圈常用的 `FT-240-43`、`T-200-2`、`T-50-6` 等按 Amidon 命名；要先找到可逐个核对的官方 AL 表（目录 PDF 或零件页），再进 `toroid::CORES`，每条带核对日期；现有内部一致性测试（AL ≈ µ0·µi·Ae/le）对粉芯不适用（分布气隙），需要另想办法对拍。已确认取不到：Amidon 官网 403、Micrometals 页面没有 AL 表 |
| 按路由拆分 wasm | 中 | **只在 `erase_components` 之后重测仍不够时再做**：要换掉 Trunk 的 rust 流水线（拆分要插在 cargo 与 `wasm-bindgen` 之间）、补 chunk 内容哈希（否则无 SW 的首访可能旧 chunk 配新主包）、`perf-budget.json` 增加 chunk 与总量项、加 `cfg(split_wasm)` dev 开关（未拆分的 wasm 残留 `__wasm_split_placeholder__` 导入，浏览器无法实例化）。量级参考（`erase_components` 之前测的）：拆 5 条主包 −399 KiB 但总下载 +8%；拆全部（约 1970 个文件）总量 +62%，不可行。只拆体积最大的少数路由 |
| THOR / MFSK16 解码 | 中 | fldigi 家族，复用 dominoex 的 GPL 转写 + `xcheck` 对拍流程 |
| MSK144 / FSK441 解码 | 中 | 流星散射专用，联动 `/meteor-scatter`；自实现避开 WSJT GPL |
| 声卡数字模式发端（基带合成） | 中 | 生成 PSK31/RTTY/FT8 基带 WAV 供声卡馈入，补「只有收没有发」；合成可对拍，声卡馈入需真机 |
| 天线补强专题：垂直天线与地网、天线陷阱、对数周期；dBi↔dBd 实测口径 | 中 | 阅读型，来源 ARRL Antenna Book；dBi↔dBd 接测量误差实验室调性 |
| RTL-SDR（WebUSB）实时接收 | 低 | 仅 Chromium 且依赖硬件，自动化只能覆盖协议层；能力与 `/sdr-waterfall`（麦克风 / WAV）重叠，净收益低 |
| 自建社区 | 低 | `/community` 目前只做外链聚合；自建需要账号与内容审核体系，与「隐私优先」「零后端运维」冲突，需先想清楚形态 |

---

## 刻意不做与设计边界

提新需求前先看这里，避免重复讨论。

### 已实现功能刻意留下的边界

- **考试**：计算题变体不进模拟考试、不进统计 / 错题本 / 收藏（机考只用官方原题，变体 id 以 `variant-` 开头与官方内容指纹隔离）；机考仿真允许续考、不硬锁全屏，并在规则演练里如实写明与真实考场的差异。
- **法规**：条文原文照录、不译不缩写；《频率划分规定》是划分表不是条文，业余相关部分已结构化在 `bands.rs` / `bandplan.rs`，不整本照录；解析里引用到本库未收录的法律或不存在的条号时不加链接。
- **LoTW**：不调用也不打包 TQSL —— 证书与签名必须在用户本机完成，本站只做导出、指引与状态回写。
- **数据来源**：器材库不写具体售价（只给档位）；没有可引用来源的规格（灵敏度、接口、功耗、重量）留空；接收机实测只收录原表测过的机型，其余显示「未收录」；DIY 项目的 BOM 不指定型号、资料链接只给长期稳定的出处。
- **山峰 / 公园字段**：`my_sota_ref` / `my_pota_ref` 是**本台**所在的（ADIF `MY_SOTA_REF` / `MY_POTA_REF`），`contact_sota_ref` / `contact_pota_ref` 是**对方**的（`SOTA_REF` / `POTA_REF`，S2S / P2P）；地图、编号查询、野外激活导出与 CSV 的 SOTA / POTA 两列只看前者。旧存档 / 旧备份里的 `sota_ref` / `pota_ref` 一律读成「我的」（`serde alias`，与它们在旧版里的实际用法一致，不迁移数据）。**旧版本导出的 ADIF 文件**把「我的」写成了 `SOTA_REF` / `POTA_REF`，现在重新导入会被读成「对方的」—— 文件里没有可靠的版本标记能区分，不猜；要保留请用备份 JSON，或导入后在表单里改。
- **日志修正**：一键修正只做不会错的三件事（按频率回填波段、本地时改 UTC、合并重复）；缺网格不拿实体中心去补，那等于往日志里掺假数据。
- **统计范围**：成就墙、学习进度（个人里程碑）与 SOTA / POTA 地图、稀有实体追踪（只看「做过没有」）按全部日志计算，不接统计范围。
- **NEC**：`.nec` 的交叉线 `TL`（Z0 为负）与非零并联导纳本求解器不支持，导入时整卡跳过并说明原因，不只解析一半；导线端点上的端口导出时会移到端段段心（NEC 格式本身的表达限制）。
- **VNA 读数**：页面不做校准，显示的就是仪器给出的数。
- **LRPT**：Meteor 不用 CCSDS 双基底（SatDump 的 Meteor 解码同样不做）；M2 不做差分译码。
- **解码器**：DominoEX 的 FEC 与非 FEC 调制听起来一样，无法自动识别，由用户选择。

### 评估后不做的候选

| 条目 | 决定与理由 |
| --- | --- |
| 实时 foF2 / MUF 地图 | 2026-10-17 核实**没有「许可明确 + 免账号 + 可程序化取数值」的全球数据源**：SWS 的全球 foF2 是图片、无 API、许可未明确；GIRO 需注册 DIDBase 账号；NOAA SWPC 只有美国区域。现有 `/muf`（VOACAP 模型 + HamQSL 实时太阳活动）已覆盖「模型预测」这一半，缺的「实测探测」留待有干净数据源时再补 |
| 蓝牙 CAT（Web Bluetooth） | 仅 Chromium；电台少有原生 BLE CAT，各家 BLE 串口模块服务 UUID 不统一，也无法自动化测试。需要无线时用 BLE 串口桥接到 Web Serial |
| 地磁扰动对 foF2 的定量折减 | IRI 的 STORM 模型有出处，但要 ap 指数的时间序列，HamQSL 的数据不够；维持「K ≥ 4 时提示」 |
| NEC 真实地面（Sommerfeld） | 工作量大且没有参考解，与反射系数近似的差别只有几个百分点 |
| SNR 预测 / 真实 VOACAP 内核 | 大型 Fortran 代码库，移植与验证成本远超收益 |
| 服务端 `/api/v1/antenna/solve` | 200 段前端求解只要几十到几百毫秒；只有把分段上限提到上千段时才需要 |
| QSL 列表缩略图 | 50 行 × 异步读 + dataURL 常驻内存，开销不值 |
| CSV 导出加操作员列 | 改列数会让已在用的表格错位；需要操作员信息请用 ADIF |
| FCC / Ofcom 题库全文 | 数据授权与翻译质量都没把握；`/exam-comparison` 只给制度对照与官方题库入口 |
| 数字语音 DMR / D-STAR / C4FM 解调 | AMBE 声码器闭源，解到「有话音帧」也出不了声，性价比低；组网 / 对比已由 `/dv-network` 覆盖 |
| POCSAG / DSC / ACARS 解码 | 偏寻呼 / 海事 / 航空，与业余无线电定位关联弱，不做 |
| PACTOR / FreeDV（Codec2）/ M17 解码 | PACTOR 无公开规范；Codec2 声码器的体积与许可在浏览器内都不现实；M17 的参考实现 libm17 是 GPL-3.0，而 4FSK 通道已有 RTTY 可借鉴，收益 / 代价比差。三者的「能不能收」在 `/modern-digital-modes` 里说清即可 |
| 塔架 / 拉线机械强度计算 | EIA-222 一类规范无免费可引数据，算不出可对拍的数；`/antenna-installation` 只给定性与施工要点 |
| Es（突发 E 层）季节预测 / 长期传播气候 | 没有可验证的物理模型，历史统计也没有干净数据源；维持 `/special-prop` 的现象说明 |
| 与现有工具重复的孤立计算器 | 52 个已过边际收益；新计算器要么能串进「链路预算向导」这类工作流，要么替掉旧的，否则不加 |

---

## 已完成

一条一项，只留结论与指针；过程与细节见对应模块的文档注释，功能清单见 README。

| 条目 | 位置 | 结论与已知边界 |
| --- | --- | --- |
| 考试中心与学习闭环 | `/map` `/practice` `/exam` `/mistakes` `/cards` `/weekly` | A/B/C 题库、按真实配额抽题的模拟考、错题自适应间隔复习、闪卡 / 听题 / 周报 / 成就 / 报告；计算题变体与薄弱项组卷不进备考状态与历史趋势 |
| 知识库与法规原文 | `/radio-law` 及 100+ 专题 | 两部法规全文照录（不译不缩写）、题目 ↔ 条款双向关联、KaTeX 自托管、学习路径 DAG、正文三语全覆盖；法规原文保持中文 |
| 天线求解与方向图 | `/nec` | 三角基 Galerkin、200 段上限、负载 / 损耗 / 有耗地面 / 传输线 / 三维方向图 / 频带扫描 / `.nec` 互导 / 画布编辑 / 设计存档 / NVIS 覆盖；与 necpp 交叉验证（参照是另一套程序，不是实测），借此修正了两份坏模板 |
| 波形实验室 / Smith 圆图 / 传播矩阵 | `/waveform-lab` `/smith` `/muf` | 四类滤波器原型对 scipy 对拍、阻抗轨迹与 L 型网络自动求解、24 小时 × 波段热力矩阵（简化 VOACAP，无 SNR） |
| 日志闭环与 QSL | `/log` `/qsl-*` | ADIF 3.1、多台站 / 操作员与统计范围、九类体检与三项一键修正、QSL 状态机与卡片设计 / A4 拼版 / 标签打印、多源三向同步；确认报告手动导入，签名不经手 |
| 设备 DIY 与测量误差 | `/gear` `/rig-reviews` `/diy-projects` `/measurement-lab` | 机型库只收原表测过的机型（其余留空）、六轴加权打分；BOM 不指定型号；VNA 页面不做校准 |
| 解码器矩阵与硬件互联 | 9 个 Worker + `/kiss-tnc` `/vna-reader` | RTTY / NAVTEX / Olivia / DominoEX（FEC 三套实现互锁对拍）/ LRPT（合成逐字节对拍 + M2 真实录音出图）等，PSK31 在主线程；串口只覆盖协议层，需真实硬件 |
| 开放 API | `/api/v1/*` | 单位换算、滤波器设计、链路预算；契约单一事实来源 `core/src/api_v1.rs` |
| FT8 / FT4 浏览器内解码 | `/ft8-decode` · `crates/ft8` | 转写自 MIT 的 `ft8_lib`（WSJT-X 的 GPL-3.0 不用，KISS FFT 按 BSD-3）：CRC-14、LDPC 编 / 软译码、消息打包解包、KissFFT + waterfall + 候选检测 + 似然提取、GFSK 合成、Worker + 页面 + e2e；19 个单测逐例对拍编译输出，真实 20m 录音解出黄金标准；**FT4 解调尚未在真实信号上验证** |
| 卫星多普勒联动 | `/satellites` · `core/{sat_doppler,sat_transponder,rotor,cat}.rs`、`server/sat_track.rs` | 下行逐秒修正 + 可选写电台（默认只读、手动勾选才写、地平线下不写、节流、手动改频即停）；上行写**另一个 VFO**（`FB` / Icom `25 01`），不开分频、不切 VFO、无 PTT，写前过三道护栏（业余波段 → 卫星业余业务段 → 多普勒物理上限，标称取预置表而非输入框）；线性转发器 AO-7 A/B、FO-29、RS-44、AO-73（SatNOGS DB，往返闭环误差 0）；自定义通带与 GS-232 旋转器自动跟踪；SGP4 在服务端，轨迹加载后可断网续用。**一次真机验证都没有**；5 s 插值最坏约 11 Hz、时钟差 1 s 最坏约 175 Hz。护栏会把 AO-7 模式 B 上行与 FO-29 通带顶端约 4 kHz 拒写（不在卫星业余业务段内），这是数据的如实反映 |
| 菲涅尔区与视距 | `/tools#fresnel-zone` · `core/fresnel.rs` | 单障碍物四档净空判定、60% 规则余量、无线电视距；单测对拍 17.32 速算式、4.12√h 与 50 km 隆起 49 / 36.8 m |
| 路径剖面 | `/tools#path-profile` · `core/path_profile.rs` | 任意多高程点，**只在采样点上判定**、至少 3 个点；单点情形与菲涅尔区逐位一致，按建议抬高后回到 60% / 100% 净空 |
| CHIRP CSV 导出 | `/repeater` · `core/chirp_csv.rs` | 列定义来自 CHIRP 通用 CSV 约定，**没有在真实 CHIRP 里实测导入**（单测只钉表头与样例行）；DCS 亚音与手台私有字段未做 |
| SOTA CSV V2 / POTA ADIF | `/log` 野外激活导出 · `core/activation_export.rs` | **两平台的官方格式说明都没能取到**（SOTA 上传页要登录、POTA 文档 403、存档站限流），格式按公开规则生成；只有 ADIF 字段（`MY_SIG` / `MY_SIG_INFO` / `MY_POTA_REF` / `MY_SOTA_REF`）对照 3.1.4 官方文档核对过；POTA two-fer 未做 |
| 磁环绕线 | `/tools#toroid-winding` · `core/toroid.rs` | 只收 4 个 Fair-Rite 零件（5943000301 / 5943001101 / 5943003801 / 5961003801），AL、尺寸、µi、Ae、le 逐个对照厂商官方零件页（2026-10-14）；Amidon 无可靠来源不收录（见「未完成」），页面给「自定义 AL」；共模扼流圈要看复磁导率，不等于这里的低频电感 |
| 互调 / 谐波 / 镜频 | `/tools#intermod` · `core/intermod.rs` | 只算频率不估幅度；波段高亮用全站唯一的 `AMATEUR_BAND_EDGES`（中国三区口径，只到 70 cm），**1.2 GHz 及以上不标** |
| 频率功率合规检查器 | `/compliance` · `core/license_compliance.rs` | 选 A/B/C 类 + 频率 → 可用性、功率上限、是否需协调，每条结论链到办法第三十 / 十五 / 三十二条。功率口径复用 `license_classes` 常量，频率命中复用 `bands::BANDS` 划分（含 `Usage` 决定「需协调」）；2200 m（5.67A ≤1 W）与 60 m（5.133B ≤15 W）的 ITU EIRP 脚注与类别上限取较小者。7 个单测钉死 30 MHz 分界、A 类 30–3000 MHz 上限与 EIRP 压限 |
| VNA TDR 故障定位 | `/vna-reader` · `core/tdr.rs` | 对 S11 扫频做逆 FFT 映射到距离域：输入速率因子，给出距离分辨率（`v/(2·BW)`）与最大不模糊距离（`v/(2·Δf)`），Hann 窗压旁瓣、−30 dB 门限 + 分辨率间距取峰。对拍用「已知失配模型合成 `.s1p` 再反推」，5 个单测钉死单 / 双反射回到已知距离、理想匹配无峰、非均匀扫频拒绝 |
| 元件代码识别（SMD 电阻 / 瓷片电容） | `/tools#smd-resistor` `/tools#capacitor-code` · `core/component_codes.rs` | SMD 电阻三位 / 四位 / EIA-96 / R 小数点 / 0 Ω 跳线，瓷片电容两位 / 三位 pF + 容差字母；代码规则是公开标准，7 个单测钉死各码型与非法输入。色环电阻（4 环）此前已有 |
| 单短截线匹配 | `/tools#stub-match` · `core/stub_match.rs` | 并联单短截线（开路 / 短路），取短截线较短的一个解。**往返验证**（把解代回传输线方程应回到 `1+j0`）钉死求解正确性，经典算例 zL = 2 → d = l = 0.152λ；纯电抗负载如实标「无法匹配」 |
| 磁环小环天线 | `/tools#magnetic-loop` · `core/magnetic_loop.rs` | 圆环天线的电感 / 辐射电阻 / 趋肤损耗 / 效率 / Q / 带宽 / 调谐电容 / 电容电压（铜导体闭式估算）。往返验证 `L·C` 谐振回 `f`，钉死 1 m 环 `L ≈ 2.94 µH` 算例；电压随 √P 缩放；环太「粗」如实标「无法估算」 |
| 离网供电逐日能量平衡 | `/tools#off-grid` · `core/off_grid.rs` | 太阳能板 + 电池 + 负载的逐日能量平衡：每日发电（含控制器 / 充电效率）、电池可用（容量 × 电压 × DOD）、净平衡、盈余自持 / 亏空可撑天数。峰值日照（最坏月）由用户按所在地输入，站内不嵌日照表；算术用已知算例对拍 |
| 微带线 εeff 与带状线阻抗 | `/tools#tx-line` · `core/tx_line.rs` | 微带线（Wheeler 闭式）补 εeff 输出，新增对称带状线（Cohn 闭式，零厚度）。单测钉死 εeff 介于 1 与 εr 之间、带状线宽 / 窄公式在 W/b=0.35 处连续、FR4 上 50Ω 微带线 W/h≈1.9 |
| 电短天线加载线圈 | `/tools#loading-coil` · `core/loading_coil.rs` | 短单极子（车载鞭天线）的辐射电阻 / 天线电容 / 容抗 / 加载电感（闭式估算，h ≪ λ/4）。`L·C` 往返谐振回 `f`、Rr 随 h² 缩放、算例量级合理；未计地面 / 线圈损耗，页面标注「精确谐振点以实测为准」 |
| 抛物面天线 | `/tools#parabolic-dish` · `core/parabolic_dish.rs` | 效率是输入（默认 55% 只是常见量级），表面精度 / 遮挡 / 偏馈 / 双反射面不建模；口径不足 3 个波长时标「增益公式不可靠」 |
| 波导 / 雨衰 / 占空比 / IP3 级联 | `/tools` · `core/{waveguide,rain_fade,duty_power,ip3_cascade}.rs` | 波导只做矩形、TE10、光滑理想表面；雨衰的 P.838-3 与官方验证数据逐组对拍，**P.530-17 部分没有官方数据（当作估算）且 R0.01 要用户自己给**（P.837 未嵌入）；占空比两种窗口口径都做了并写明差异；IP3 只做三阶 |
| 竞赛 SCP 呼号补全 | `/contest-log` · `core/scp.rs` | 官方站保留所有权利且无再分发许可，**本站不带任何 SCP 数据**：候选来自自己日志与自己载入的 `.scp` 文件（只进 IndexedDB，不进备份，不上传）；不做 `.dta`、`SCP.DB`、自动更新；候选按字母序而非出现频率排 |
| 竞赛后分析 | `/contest-log` · `core/contest_analysis.rs` | 与 `score` 共用 `qso_value` / `valid_qsos`，单测断言分析合计 == 记分；「空闲」只是相邻两条通联的间隔；不建模各竞赛自己的休息规则；乘数全集只有 CQ 分区可靠 |
| Cabrillo 导入 | `/contest-log` · `core/cabrillo_import.rs` | 只认「每边一个交换字段」，多字段交换（Sweepstakes / NAQP）明确跳过；加宽松键避免把自己导出的日志再导一遍；**未取到官方规范原文、没用真实 N1MM / DXLog 文件测过**；顺带修了 `8:30` 导出成 `0000` 的缺陷 |
| NOAA 三日 Kp 预报 | `/api/kp-forecast` · `server/kp_forecast.rs` | 取 JSON 而不是解析三日预报文本（文本无契约）；G 级由前端按 NOAA 标度算，4.67（5-）算 G1 是同一规则推出来的、**未被上游直接印证**；不做 Ap、预报置信度与推送 |
| LoRa 链路与专题 | `/lora`、`/tools#lora` | ToA / 灵敏度逐例对拍，但**未取到 Semtech AN1200.13 原文**；只做 SF7–12；最大距离是自由空间理想值 |
| 现代数字模式专题 | `/modern-digital-modes` | 带宽与类型逐字沿用 `modes.rs` 的 `DIGITAL_MODES`，不另起数字；VARA 闭源免费、其余开源经查证 |
| NTS Radiogram / ICS-213 | `/radiogram`、`/tools#radiogram` | 报头、优先级、HX、检查（check）计算按 ARRL 标准，ICS-213 字段按 FEMA 标准；只做格式与计算，不校验报文内容合规 |
| 国内特色专题 | `/china-ham` | XW-2 / CAS-3 六颗卫星频率取自 BA1DU 的发射设计值（在轨状态以 AMSAT 为准）；409 MHz 公众对讲机依据国无办〔2025〕1 号，正文只写「较低功率」、限值在附件 3，不编精确数字 |
| 主包体积：`erase_components` | `.cargo/config.toml`、`perf-budget.json` | 2026-10-16 启用，`map_view.rs` 的 4 处 `.clone()` 随之改掉；主包 wasm br 2351.2 → 1723.3 KiB（干净 HEAD 上对照 −644 KiB，−28%），完整 e2e 全过、挂载更快，上限收紧到 1 900 000 B 把省下的空间锁住；逐帧热路径仍未压测 |
| 全站手机宽度扫描 | `e2e/tests/mobile_overflow.spec.ts` | 全部路由 × 三语 × 320 / 390px；首轮扫出 6 个页面并逐页修完 |
| 横向基础设施 | 搜索 · 存储 · 备份 · 门禁 | 统一搜索（含 `dxcc:` / `band:` / `mode:` 前缀）；`kv.rs` 写入版本仲裁（修「写入后立即刷新丢最后一笔」）与 IndexedDB 权威层；备份按模块勾选；交互页必须有专属 e2e（存量欠账已清零）；体积 / 文案 / 解析 / 像素风校验并入 `cargo make check` |

### 踩过的坑（避免重犯）

**FT8 / FT4 转写**

- `bfly4` 漏了 `a += c` 一步（对照 C 的 `C_ADDTO`），漏则 3840 点 FFT 全错；
- 实数 FFT 把浮点版 `C_FIXDIV`（NOOP）当成「除以 2」，导致差 2 倍；
- `bp_decode` 的软信息约定是 `log(P(1)/P(0))`（正 = 位 1），ft8_lib 注释写反了，照注释会译出垃圾；量级必须先归一化到方差 24（约 ±5），拿 ±50 去跑会因 `fast_tanh` 饱和而译不回来；
- 乘法哈希 `47055833459 * n58` 在 C 里靠 u64 回绕，Rust 需 `wrapping_mul`；
- `encode` 入口用 `split_whitespace` 会把 `CQ POTA` 这类 **CQ 修饰符**拆成两个字段、误判成 4-token 自由文本，要先 `parse_cq_modifier`；
- Worker 导出名 `decode` 与 `ham_web_ft8::decode` 撞名；`index.html` 漏 `copy-dir` 时 trunk 不拷 worker 目录，`importScripts` 拿到 SPA 回退的 HTML（「curl 200 但加载失败」的典型）。

**卫星联动与旋转器**

- 旋转器命令原本不带回车（Hamlib 的 `EOM`）、仰角又写成单独的 `W045`，一直没人发现，是因为该组件此前**没有任何 e2e**；
- 方位 359.6° 要先回绕再取整（照抄 Hamlib 会写 `W360`，超出 0–359）；
- 「有人手动转走」不能用「离目标远」判定（刚写完目标时离几十度是正常转动），改用「离目标连续约 8 拍没变近」；
- 换转发器时忘了同步 NORAD，会拿错卫星的轨迹算多普勒，页面看起来一切正常；
- 变异验证的教训：判据改坏后 e2e 仍然绿，是因为用例只观察 6 秒、而误判要 8 拍才触发。

**通用**

- 对防抖 / 异步刷新的输出做快照，要先等它落定再读，别把上一次的值当基准；
- 没有可见标签的输入框，靠 e2e 找不到才暴露；
- `getByText` / `getByLabel` 是子串匹配，短文案要 `exact` 或改用固定 id。

---

## 工程门禁

新增页面 / 模块必须满足（仓库内已有自动化守护，新增内容自动受益）：

1. **只改两处**：`crates/core/src/registry.rs` 的 `MODULES` 追加一条 + `crates/app/src/app/main_content.rs` 加一行 `<Route>`。
   导航菜单、`sitemap.xml` 均由注册表派生，`registry_check.rs` 断言「注册表 ↔ 导航 ↔ 路由」一致，漏注册会让 `cargo make check` 直接失败。README 的路由表也按注册表分组列出，新增页面时同步补上。
2. **内核下沉**：不依赖浏览器的逻辑放 `ham-web-core` + 单元测试；技术性数值在 `knowledge_consistency.rs` 加回归断言。
3. **大内核走 Worker**：DSP / 求解器按 `ham-web-apt` 的模式拆 Worker，遵循 [`docs/decode-worker-protocol.md`](docs/decode-worker-protocol.md)，注意 `wasm-bindgen` CLI 版本与 `Cargo.lock` 一致。
4. **文案与翻译**：界面文案同步补 `data/i18n/{lang}/{domain}.json`（语义 key `域.词条`，`cargo make i18n-check` 校验重复 key、占位符、死条目与语言包新鲜度，改完跑 `cargo make i18n-pack`），正文译文放 `data/knowledge-i18n/`（`knowledge-i18n-check` 钉住漂移）。
5. **测试**：新增交互补 Playwright 用例，并把旅程追加到 `e2e/coverage-targets.json`（`grep` 子串需唯一命中）。
   工具 / 考试类路由没有任何功能用例字面量 `page.goto` 过时，`registry_check` 直接失败。依赖 `/api/*` 的页面用 `fixtures.ts` 的 `stubApi` 打桩，时间相关断言用 `page.clock` 固定。
6. **体积**：`cargo make ci` 末尾跑 `size-check`，任何产物超出 `perf-budget.json` 即失败。
   上调上限前先回答「这次增量换来了什么、能不能拆进 Worker」，并在本文件记一笔。
   **当前**（2026-10-16，`erase_components` 已启用）：主包 wasm br **1723.3 KiB / 上限 1900 000 B**（基线 1 764 675 B，余量约 130 KiB），单个解码 Worker ≤ 64 KiB。
   - **动手前先决定放哪里**：纯计算且依赖重的放服务端（像 `/api/sat-track` 那样，SGP4 不进浏览器）或拆 Worker；只有薄薄一层界面的才进主包。上调上限不是默认选项。
   - **已用实测排除的方向**（不要重复尝试）：压缩参数（已是 brotli q11 / lgwin 24，复算只差 3.6 KiB）；`wasm-opt` 参数（5 种组合，br 体积在 2.37 MB ± 4 KB）；把内嵌数据挪成运行时 JSON（最多几十 KiB）；单态化（可避免部分只占 1 MB 的 0.16%）。真实结构是均匀的「页面数 × 每页视图树」，没有单个热点。
   - **未验证项**：频谱 / 瀑布图、NEC 三维方向图、摩尔斯训练器这类每帧更新的页面，`erase_components` 前后的运行时开销没压测过，这是体积方向唯一的已知缺口。
   - **工具坑**：`cargo bloat` 不能用于 wasm 目标；归因用 `twiggy` 且要 `strip=none` / `debug=0`（`debug=1` 会膨胀到 646 MB）；带符号构建会被随后的 `trunk build --release` 覆盖。

---

## 验证方式

```bash
cargo test -p ham-web-core            # 领域逻辑 + knowledge_consistency 回归
cargo test -p ham-web-app             # registry_check（注册表 ↔ 路由一致性、交互页 e2e 门禁）
cargo make check                      # fmt + clippy + test + 文案词典 + 解析表 + 像素风
cargo make e2e                        # 构建 release + Playwright 全站冒烟与无障碍
cargo make e2e-coverage               # 覆盖率报告 e2e/test-results/coverage.md
cargo make size-check                 # 性能预算（需先 build-web）

# 需要本地样本的测试（样本不进仓库，未设置变量时自动跳过）
LRPT_REAL_IQ=/path/to/meteor-m2.wav cargo test -p ham-web-lrpt --release --test real_iq
```

---

## 相关文档

| 文档 | 作用 |
| --- | --- |
| [`README.md`](README.md) | 已实现功能、路由表、开发指南 |
| [`AGENTS.md`](AGENTS.md) | 动代码前的入口：按改动领域指向规范与硬规则 |
| [`docs/ui-components.md`](docs/ui-components.md) | UI 组件使用规范（决策表、硬规则、验收） |
| [`docs/i18n.md`](docs/i18n.md) | 界面文案规范与门禁 |
| [`docs/decode-worker-protocol.md`](docs/decode-worker-protocol.md) | 解码 Worker 协议与新增解码器清单 |
| [`docs/explanations-style.md`](docs/explanations-style.md) | 题目解析写作规范 |
| [`docs/typography.md`](docs/typography.md) | 题库排版规则（内容指纹稳定性依赖） |
