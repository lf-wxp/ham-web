# 路线图

> 只记三件事：**还没做的**、**刻意不做的**、以及**新增内容必须过的门禁**。
> 已实现的功能见 [`docs/reference/features.md`](docs/reference/features.md)（README 的「功能一览」是它的索引）；实现细节、设计取舍与精度边界写在各模块的
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
| 天线与射频 | [天线与射频设计](README.md#天线与射频设计)：矩量法求解器、Smith 圆图、波形实验室、测量误差实验室 | 地面用反射系数近似；`TL` 不支持两端并联导纳（交叉线已支持）；交叉验证参照是另一套 NEC-2 实现，不是实测 |
| 传播与实时数据 | [工具与实时数据](README.md#工具与实时数据)：24 小时 × 波段热力矩阵、太阳活动、DX Cluster、PSK Reporter / RBN、卫星过境、地波场强（ITU-R P.368）、大气噪声（ITU-R P.372） | 简化 VOACAP 模型，无 SNR 预测；大气噪声只给中位与十分位、不含脉冲成分；地波按平滑球面、不做海陆混合路径；地磁扰动只提示不建模 |
| 日志与 QSL | [通联日志与 QSL](README.md#通联日志与-qsl)：ADIF 3.1、多台站 / 操作员与统计范围、日志体检、QSL 全生命周期与多源确认同步、卡片设计与标签打印、竞赛与奖状 | 确认报告靠手动导入（无 OAuth 直连）；模式分段校验只覆盖话务 |
| 解码与硬件 | [浏览器内解码与硬件互联](README.md#浏览器内解码与硬件互联)：14 个解码器（13 个走 Worker，PSK31 在主线程）、CAT、KISS TNC、NanoVNA | LRPT 的 OQPSK 只在合成信号上验证；MFSK / THOR 的时钟误差与频漂只拟合 1–3 阶平滑曲线（时钟跳变与半个音调间隔内的跳频能解，更大的跳频不能），THOR 用硬位判决（白噪声下实测不输 fldigi）；串口收发需真实硬件，自动化只覆盖协议层与文件入口 |
| 设备与 DIY | [设备与 DIY](README.md#设备与-diy)：机型库实测指标与加权打分、评测文章、DIY 项目卡片 | 实测数据只覆盖 HF 级接收机；灵敏度、接口、功耗、售价无可引用来源，刻意留空 |
| 国际化 | [用户体验](README.md#用户体验)：界面与知识库正文中 / 英 / 西三语全覆盖 | 法规原文保持中文（只有官方版本有规范效力） |

---

## 未完成

按可行性排序。可行性：高 = 数据与参照都在、只差实现；中 = 需要外部样本或硬件才能闭环；低 = 缺可信来源或收益不抵成本。

下表 2026-10-18 按「组合优势优先」重排：**S 系列**是站内数据与基础设施已齐、只差串联的功能；
**D 系列**是能力空白；**K 系列**是内容补强。每条都写落点与验证口径，完成后移入「已完成」。

> 串口与外部格式类条目（CAT、旋转器、NanoVNA、KISS、CHIRP、SOTA / POTA 导出）的验收口径
> 统一见 [`docs/domain/hardware-verification.md`](docs/domain/hardware-verification.md)：**单测与假串口通过只算「待验」**，
> 真机跑过并记录型号 / 固件 / 实际现象才算「已验证」。

| 条目 | 可行性 | 阻塞点 / 下一步 |
| --- | --- | --- |

| **V1 跨内核冒烟：只剩 Firefox** | 中 | 用例与 project 已写好：`e2e/tests/cross_browser.spec.ts`，`E2E_CROSS=1` 才启用。**WebKit（webkit-2370）2 个用例全过**（核心页面能开、无未捕获异常、三维图落到 SVG 或 WebGL2 渲染器）；原先写的「缓存版本不匹配、需要 install」已不成立 —— 缓存里的 firefox-1555 / webkit-2370 正是当前 Playwright 要的。**Firefox 起不来**：连 Playwright 之外直接 `firefox --headless -profile <目录>` 都报 `Could not find profile folder`，是这台机器的环境问题，换终端 / 换机器再跑；跑过之前 Firefox **仍是零验证**（Web Serial 在 Firefox 上本就不存在，页面应降级而不是报错） |
| **S5 噪声本底的绝对口径** | 中 | 相对比较已上线（见「已完成」）：10% 分位本底 + 占用率 + 同标签前后差。仍未做的是把相对值标成绝对噪声温度。**阻塞点与 P.372 无关**：P.372 的 `Fa` 是「天线端子处相对 kT0b 的 dB」，而这里量的是声卡 / 麦克风输入的 dBFS —— 声卡增益与校准未知，中间差着一个没法测的常数，换算出来的 dBm 是假的（页面因此不给 dBm）。要做就得让用户先用已知信号源标定一次，那是另一个功能。P.372 的中位数现在已内置（台站链路预算里），可以拿来**对照**这里量到的相对值，但不能据此换算 |
| **D4 天线缩短系数 / 直径修正** | 中 | 天线长度计算器现在只有一档「×0.95」式系数；补线径与离地高度的 K 因子修正。ATU 覆盖综合那一半已做完（见「已完成」）。阻塞：K 因子需要 ARRL Antenna Book 表或 EZNEC 结果对拍，不能凭印象给数 |
| SatNOGS 转发器库：FM 卫星与 digipeater | 中 | 已有：线性转发器 JO-97 / TO-108，FM 预置 ISS / SO-50 / AO-91 / AO-123（见「已完成」）；digipeater 一条没有。**入库准则**：每颗候选都要有 IARU / AMSAT / 运营方页面当第二来源（SatNOGS 的 FM 条目引用栏参差、大量 `CITATION NEEDED`），并用 `curl` 核对 Celestrak 业余组（`gp.php?GROUP=amateur&FORMAT=tle`）里 TLE 在位，否则页面会「选了却算不出」；SatNOGS 标「在轨」不等于有地心轨道根数（FO-82 是日心轨道，TLE 接口返回空）；FM 预置还要与 `satellites.rs` 的人工表逐条对照（单测强制）、补中文条目与 en / es 译文。**已评估不收**：AO-27（SatNOGS 436.795 与 AMSAT 页 436.797 MHz 不一致）、LilacSat-2 / CAS-3H（SatNOGS 的 FM 条目已标「不活动」，IARU 写的频率却不同）、PO-101（频率一致但 AMSAT 状态页近一周零报告）、AO-85 / AO-95 / AO-92 / IO-86（不在 Celestrak 业余组或 SatNOGS 不再标活跃）；ISS 的 145.825 MHz APRS digipeater 在 SatNOGS 里是 2019 年的旧条目、实际由 ARISS 间歇开启，预置成常驻会误导。SatNOGS DB 是 CC BY-SA 4.0，页面已写明出处与取回日期。**重开条件**：出现新的 FM 星，或 PO-101 / CAS-3H 恢复规律开机并取到一致的频率 |
| LRPT：M2-3 / M2-4 的 OQPSK 真实录音 | 中 | M2（QPSK）已在真实录音上出图（`crates/lrpt/tests/real_iq.rs`；录音不进仓库，用 `LRPT_REAL_IQ` 给路径）。现役的 M2-3 / M2-4 是 OQPSK，定时与载波环路要按 I/Q 半符号错位改写；**缺一段真实 OQPSK 基带**（SatNOGS 只存 48 kHz 音频与解好的图片） |
| LRPT：M2-2（80k）真实录音对拍 | 中 | 重同步、去交织、差分链路已有往返测试；缺真实 80k 录音 |
| LRPT：软判决 Viterbi | 中 | 弱信号的增益项；现有真实样本硬判决已 99.8% 帧通过，看不出收益，有更弱的样本再做 |
| 卫星联动：双 VFO 卫星模式 / 跟随旋钮 / 真机验证 | 中 | 上行写入、线性转发器、自定义通带与旋转器自动跟踪已完成（见「已完成」）。**仍未做**：① **卫星模式**（IC-9700 / IC-705 / FT-991A 的主 / 副 VFO 各管一个波段）——各机型 CAT 语义不同（Hamlib 里 IC-7600/7610/7800/785x 的 `25` 命令是 Main/Sub 而非选中 / 未选中，FT-991A 的 `FR`/`FT` 也不是 Kenwood 含义），没有真机逐款验证就上线等于没测；② **线性转发器跟随电台旋钮**——现在是页面持有调谐点、转旋钮会触发「手动改频即停」，要反过来得把回读频率同时当输入和输出，而回读最多有 3 次轮询的延迟，发射会滞后于旋钮；③ **真机验证**——上行写入（FB / `25 01` 在各机型的实际行为、分频设置、电台忙时的回应）与旋转器（GS-232 在各控制器上的应答格式、速度、限位、过顶走哪条路）**一次都没实测过**，只有假串口。逐项验收步骤与记录项见 [`docs/domain/hardware-verification.md`](docs/domain/hardware-verification.md) A1–A6 / B1–B4 |
| NanoVNA 串口实机验证 | 中 | 协议解析有单测、`.s1p` 入口有 e2e；串口收发需要真实仪器（与 KISS TNC 同一边界）。验收步骤与记录项见 [`docs/domain/hardware-verification.md`](docs/domain/hardware-verification.md) C1 / C2 |
| 按路由拆分 wasm | 低 | **2026-11-08 已做完三步测量并裁定不立项**：crate 级归因显示非本仓库的框架地板 4,733,302 B 占带符号构建的浅层字节 **56%**，与路由无关；删减实验里摘掉 179 条路由只留外壳，br 2,126,177 → 819,239（**−61.5%**，这只是收益上限），平均每页 7.3 KB br、最贵的 `/nec` 只有 3.8%。A 方案（多 wasm 模块）拿不到上限 —— Leptos CSR 模块间不能共享运行时，每拆一页都要连框架副本一起打。真要拆还要换掉 Trunk 的 rust 流水线（拆分插在 cargo 与 `wasm-bindgen` 之间）、补 chunk 内容哈希、`perf-budget.json` 增加 chunk 与总量项。**重开条件**：主包涨到 2.6 MB 以上且首屏 LCP 被带宽卡住，那时先做 SSR / 岛屿。数字与命令见 [`docs/engineering/performance.md`](docs/engineering/performance.md) §5.4 |
| MSK144 / FSK441 解码 | 中 | 流星散射专用，联动 `/meteor-scatter`；自实现避开 WSJT GPL |
| 天线补充专题的剩余项 | 中 | 页面已上线（见「已完成」）。**还没做的**：①（LPDA 的 NEC 模型已做，见「已完成」的「NEC 传输线：交叉线 / 同段并联」）；②垂直天线的**数值**专题（地网根数 × 土壤电导率 → 效率）：需要 Sommerfeld 地面或实测数据才有可信数字，现在只给了自由空间 / 理想地 / 反射系数近似下核对过的几个点，其余写成定性；③LPDA 增益与 σ、τ 的关系（Carrel 的曲线图没有可程序化引用的数据，没抄）；④ARRL Antenna Book 本身没能取到（原条目写的来源），页面改用 Carrel 1961、47 CFR §73.189、IEEE Std 145、ARRL《QST》1996 年 11 月 Lab Notes 与 necpp 自核 |
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
- **NEC**：`.nec` 的 `TL` 两端非零并联导纳（`F3`–`F6`）本求解器不支持，导入时整卡跳过并说明原因，不只解析一半（交叉线 = 负 `Z0` 已支持）；导线端点上的端口导出时会移到端段段心（NEC 格式本身的表达限制）。
- **VNA 读数**：页面不做校准，显示的就是仪器给出的数。
- **LRPT**：Meteor 不用 CCSDS 双基底（SatDump 的 Meteor 解码同样不做）；M2 不做差分译码。
- **解码器**：DominoEX 的 FEC 与非 FEC 调制听起来一样，无法自动识别，由用户选择。

### 评估后不做的候选

| 条目 | 决定与理由 |
| --- | --- |
| THOR 的软位路径（`softdecodesymbol`） | 白噪声下用同一段加噪音频喂 fldigi 4.2.13 与本仓，本仓不输（THOR16 −13 dB：fldigi 4 / 32 遍，本仓 20 / 32 遍），没有要补的差距；它的软值来自子频点上的载频偏差，本仓的 Goertzel 网格里没有这个量。数字见 `crates/mfsk/reference/fldigi_web/README.md` |
| THOR 前导检测（`THOR_PREAMBLE`） | 两段发射之间隔 3 秒噪声的录音，本仓与 fldigi（默认开着前导检测）解出的正文逐字一致，没有可量出的差距；离线解码没有「实时流里尽快把管线里压着的字符推出来」这个需求 |
| MFSK / THOR 超过半个音调间隔的突然跳频 | 这已经不是漂移：整个音调序号错了一位、边缘音调落到梳外面，要分段重估梳并扩展网格才能救；真实录音里没见过这种发射机行为，不为想象中的场景加复杂度 |
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
| 界面性能微调：`spotlight` 合帧、`spectrum_paint` 按颜色分组 `fillStyle`、`.md-aurora` 去掉 `blur`、`will-change` 常驻 | 2026-11 审查时**没有量到任何卡顿**；这几项都是「可能更快」而非「已知慢」。没有浏览器里的前后对比就不改（盲改动效 / 绘制的风险大于收益）。出现具体的掉帧报告再带着 profile 回来 |
| 合并 `.md-panel table:not(.md-table)` 的两份规则 | 看着像重复，其实不是：一份在 `@layer components` 里服务知识页的**裸表格**（`TableSection`），一份在层外服务计算器里的像素 `Table`（工具类底色压不过层内规则，只能写在层外）。合并会改变层叠结果，收益只是少几十行 CSS |
| 现代风的无障碍小项：进度条 `role`、`exam_clock` 的可访问名与 live region、`skeleton` 的 `role=status`、答题格「已答」状态 | 逐项核对后**不是缺口**：进度条旁边紧挨着文字进度（`第 N / 总数 题`），进度条本身是装饰；倒计时有意不加 `aria-live`（每秒播报会淹没读屏，像素版同一口径，见 `exam_clock.rs` 文件头）；`skeleton` 是 `aria-hidden`，加载态由页面级的 `Loading` 负责播报；答题格的 `aria-label` 已含「标记 / 对 / 错」。真有缺口的是「答题格没有『已答』状态」，影响面小，等有用户反馈再做 |
| 英文 / 西语文案的标点 / 大小写统一、`rpg.you` 与 quest / stage / mission 术语统一 | 没有用户可见的错误，是风格偏好；三语词条由 `i18n-check` 守着齐全与占位符，不守措辞 |
| 与现有工具重复的孤立计算器 | 52 个已过边际收益；新计算器要么能串进「链路预算向导」这类工作流，要么替掉旧的，否则不加 |
| 弹层遮罩（`SimpleSelect` / `DatePicker` / `TimePicker` 的 `popover::overlay_class()`）改 `Portal` | **实测不需要**：担心的是现代风 `.md-page` 的 `container-type` 会让 `fixed` 后代的包含块变成页面容器。用 `/tools#mode-encoder` 在现代风下滚过 28 000 px 后打开 WSPR 功率下拉，遮罩确实在 `.md-page` 内，但 `getBoundingClientRect` 恰为 `0, 0, 1280, 800`（Chromium 与 WebKit 各测一次，像素风同值）；`/countdown` 的日期 / 时间选择器遮罩在现代风下根本不在 `.md-page` 里。原因是 `container-type` 的 layout containment 已从规范里去掉。**Firefox 未验证**（本机起不来），出现 Firefox 上遮罩只盖住页面容器的报告再改。`fixed` 弹层 / 底栏走 `Portal` 的规则不变，只是不把这三处追溯为缺陷；`docs/ui/modern.md` §3 与 `04-overlay.css` 的旧说法已更正 |

---

## 已完成

一条一项，只留结论与指针；过程与细节见对应模块的文档注释，功能清单见 [`docs/reference/features.md`](docs/reference/features.md)。
**一件事一个家**：做了什么、边界是什么写进功能清单与协议文档；机制与取舍写进代码的文档注释；实测数字与复现命令写进该模块的 `reference/` README；踩坑里可复用的规则进下面的「踩过的坑」。这里不写「试过什么又撤销了」「某一天量到什么」「这一轮多了多少字节」这类过程，那些在 git 历史里。

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
| 实体名三语 | `/log` · `i18n::localized_name` | DXCC 实体名按界面语言本地化：中文用 `name`，en/es 优先反查译文、查不到回退 `name_en`。修复录入页提示行、通联表实体列、网格详情、统计标签、网格筛选下拉（提交值仍保持中文以兼容 `dxcc_map` 的查询回环）；e2e 钉死 en 界面显示 `Japan` 而非 `日本` |
| 数字模式基带合成（CW / RTTY / PSK31） | `/tools#mode-encoder` · `core/mode_audio.rs` | 文本 → CW / RTTY / PSK31 基带采样与 16 位 WAV（8 kHz 单声道），补「只有收没有发」。CW 按 Paris 口径点 1 / 划 3 / 字符 3 / 单词 7 单位；RTTY 45.45 波特 AFSK（起始 0 + 5 位 ITA2 低位在先 + 1.5 停止 1）；PSK31 复用 `psk31::synthesize`（BPSK 31.25 波特，往返解码已对拍）。单测钉死时长、零交叉估频与 WAV 头；e2e 钉死下载 `cw.wav` / `rtty.wav` / `psk31.wav` |
| 外部噪声：人为 + 银河噪声的 P.372 预设 | `/tools#station-link` · `core/noise_model.rs` · `pages/tools/station_link.rs` | 「外部噪声来源」下拉：自填（默认，行为与加预设前逐位一致）或 P.372 五条曲线（`Fa = c − d·lg f`）。常数取自 P.372-14 表 1，与 -6 / -7 逐行比对一致，现行 -18 没有直接对照（ITU 拒绝脚本下载）；另用图 39 上量出的 29 个点独立核对，最大差 0.99 dB。超出适用频段时页面说明原因、不出结果、不外推（D / E 原文只给图，取图上量得的范围）。**边界**：曲线是 1970 年代测的，新实测（表 3–4）落在 A–C 范围内的 23 个点里 21 个低于曲线、平均低 4.3 dB，所以城市 / 住宅 / 农村曲线在 VHF 上偏保守；不含大气噪声（另见「大气噪声」一条）、随时间地点的起伏、室内与脉冲成分；参考天线是无损接地短垂直单极子。链路预算数值由逐级功率累加的独立 Python 路径核对。依据与细节见 `noise_model.rs` 的模块文档 |
| 陷波器（trap）偶极子设计与 NEC 自证 | `/tools#trap` · `core/trap.rs` · `nec/reference/trap_necpp.py` | 并联 LC 陷波器的谐振 / 加载计算与两波段偶极子尺寸：内段 = 高波段半波（143/f/2），外段已计入 trap 在低波段的感性加载（每臂当终端开路传输线，线径固定 2 mm；`Z_c` 里的常数取 1.43，来自 necpp 对**裸偶极子**谐振处电抗斜率的测量，没有拟合 trap 数据）。NEC 自证不改 `Load`：求解器每次只算一个频率，该频率上并联 LC 等价同 `ω` 下的串联电抗，逐点重算。对拍对象是 necpp（独立的 NEC-2 实现，不是实测）：17 组 trap 偶极子（含 8 组留出集）低波段谐振偏差 0.04%–0.83%，默认 40 m + 20 m 外段 5.10 → 4.46 m。**边界**：没建模 trap 杂散电容（`C` 请填含杂散的总值）、离地高度、导线绝缘层；线圈 Q 只影响谐振电阻、不影响剪线长度；60 段网格会把高波段近似开路的 trap 吸附到相邻节点（高波段谐振有约 ±0.1% 的离散噪声）；内段敏感度与线圈 Q 的数字是一次性 necpp 测量，脚本没有保留，不能从仓库复现。依据与细节见 `trap.rs` 的模块文档 |
| AX.25 软 TNC（空中链路层 + 定时恢复） | `/tools#soft-tnc` · `core/afsk1200.rs` | Bell 202 AFSK1200 的「帧 ↔ 基带音频」：CRC-16/X-25 FCS + HDLC（旗标、位填充、结束旗标可兼作下一帧起始）+ NRZI + 逐位 Goertzel 相关器解调；符号定时恢复把每个位相位都扫一遍并按帧内容合并（同一帧只算一次，真实重发的两帧保留两份），可解任意起始相位的录音；发送侧带前导 / 尾部 flag（约 213 ms 前导）。复用 `kiss::ax25_ui_frame`。FCS 用 `"123456789"` → `0x906E` 与 RFC 1662 剩余值钉死；e2e 下载 `afsk1200.wav` 再上传解回源 / 目的 / 信息。**边界**：往返与外部已知答案只证明自洽与 FCS 口径，与 Direwolf / 硬件 TNC 的真机互通仍是「待验」（[`docs/domain/hardware-verification.md`](docs/domain/hardware-verification.md) C4） |
| Feld Hell 解码器 | `/hell-decode` · `crates/hell` + `crates/hell-worker` | AM 点阵模式（980 Hz 载波、122.5 像素/秒、7×7 网格）解码：AM 包络 + 逐像素门限 + 5×7 字库匹配。按 `docs/domain/decode-worker-protocol.md` 清单全量接入（core + worker + Makefile + Dockerfile + decode-worker.js + 页面 + e2e 三套实现互锁）。字库自绘、假设字符边界对齐，如实标注「解真实 Feld Hell 需换标准 Hell 字库 + 列相位搜索」 |
| WEFAX 解码器 | `/wefax-decode` · `crates/wefax` + `crates/wefax-worker` | 短波气象传真：AM 视频信号（幅度 = 亮度）→ 行同步检测 + 灰度图重建，按统一 Worker 协议清单全量接入。行速率/同步占比/像素数自定口径（120 LPM、同步 5%、380 像素/行），未对照真实 WMO 传真样本校准；真实 HF 传真是 1500 / 2300 Hz 调频副载波，当前未做 FM 解调，如实标注 |
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
| 主包体积：`erase_components` | `.cargo/config.toml`、`perf-budget.json` | 已启用，`map_view.rs` 的 4 处 `.clone()` 随之改掉；主包 wasm br 2351.2 → 1723.3 KiB（干净 HEAD 上对照 −644 KiB，−28%），完整 e2e 全过、挂载更快，上限收紧到 1 900 000 B 把省下的空间锁住；逐帧热路径仍未压测 |
| 主包体积：分包评估（2026-11-08） | [`docs/engineering/performance.md`](docs/engineering/performance.md) §5.4 | 三步测量做完：①`cargo bloat` 读不了 wasm（`unsupported file format`），crate 级归因改用 `twiggy top` 全量输出按 crate 前缀汇总；②数据段 5,727 段 / 1.64 MB，最大一块是中文词典；③删减实验：去掉 `/nec` 一页 br −3.8%、只留外壳 br −61.5%。**裁定不做 A（多 wasm 模块）**。同日把「中文词典改成运行时语言包」实现了一版，实测**净亏**（wasm −65,458 B br，新增 `data/i18n/zh.json` +94,013 B，中文首访 **+28,555 B**），**已撤销** —— 根因是拆开 brotli 流后各流各自重建字典，省下的不够付新包的账 |
| 全站手机宽度扫描 | `e2e/tests/mobile_overflow.spec.ts` | 全部路由 × 三语 × 320 / 390px；首轮扫出 6 个页面并逐页修完 |
| 横向基础设施 | 搜索 · 存储 · 备份 · 门禁 | 统一搜索（含 `dxcc:` / `band:` / `mode:` 前缀）；`kv.rs` 写入版本仲裁（修「写入后立即刷新丢最后一笔」）与 IndexedDB 权威层；备份按模块勾选；交互页必须有专属 e2e（存量欠账已清零）；体积 / 文案 / 解析 / 像素风校验并入 `cargo make check` |
| 信号听辨向导（S3） | `/signal-id` · `core/signal_id.rs` | 19 个模式的听感特征表（形态 / 节奏 / 占用带宽）＋ 三组条件筛选，末步链到对应解码器页；空结果如实说「条件矛盾」而不猜。单测钉死扇区式筛选与 `pages_exist_in_registry`（所有跳转目标必须在注册表里）；三语文案齐全，e2e 钉死收窄与空态 |
| 测向交汇定位（S4） | `/ardf` · `core/df_fix.rs` | 4 行「坐标 + 方位角 + 误差」→ 加权最小二乘交点、95% 误差椭圆半轴 / 朝向、残差，并给出 Maidenhead 网格；平行方位如实报无解，长半轴 > 50 km 提示几何太差。单测用「目标周边 5 km、间隔 120° 的三次测向」回到目标 10 m 内，并行 / 越界 / 角度回绕都有用例 |
| 方向覆盖（S1a） | `/psk-reporter` · `core/spot_coverage.rs` · `components/spot_coverage.rs` | 把接收台网格按本台为原点分 8 个 45° 扇区，给出报告数与平均 SNR，可按波段筛选；本台网格缺失时提示去填，不硬算。单测钉死扇区边界（337.5 / 22.5 归北）、无效网格跳过、按波段均值；e2e 用桩数据钉死扇区计数与波段切换。**RBN 未接入**（其 spot 没有网格，见「未完成」S1b） |
| WSPR 发端（D3a） | `/tools#mode-encoder` · `crates/wspr` | 消息打包 + 卷积码 + 交织复用解码器同一份 `encode`，4-FSK 用 `synthesize`，导出 12 kHz / 110.592 秒基带 WAV（2.65 MB）。呼号 / 网格 / 功率任一不合法就整条不发并提示。往返闭环由 `crates/wspr` 的 `audio_roundtrip` 与解码器保证，e2e 钉死文件名与**精确字节数**（44 + 162 × 8192 × 2）。主包体积仍在预算内（`size-check` 27 项全过） |
| 模拟中继联网（K1） | `/dv-network` · `core/dv_network.rs` | EchoLink / IRLP / AllStar Link 三套语音网关单列一张表（不与 DMR / D-STAR 混排，单测断言两侧条目不重叠），另加五条接入要点（网关仍占本地频率、呼号验证、DTMF 与节点号、丢包表现、跨境控制操作员）。正文 en / es 译文齐全，e2e 断言分表与要点 |
| 麦克风录音机（D5） | `/sdr-waterfall` · `pages/sdr_waterfall/sdr_waterfall_page.rs` | 监听中点「录音」开始累积 PCM，点「结束并保存」把缓冲转成 WAV（采样率取 `AudioContext` 的实际值，16 位单声道）。实现用 `ScriptProcessorNode` 而不是 AudioWorklet：后者要额外 JS 模块与异步加载，这里只需要按块拿 PCM；script processor 必须接到 destination 才会被驱动，中间串一个零增益 `GainNode`，否则麦克风会直通扬声器啸叫。缓冲用 `RwSignal` 而不是 `Rc<RefCell>`（`Closure` 要求 `Send + Sync`）。e2e 用假音频设备录约 1 秒，钉死文件名、>8 KB 与样本部分为偶数，`--repeat-each=3` 稳定。**没做**：录制时长上限、录完直链到解码器（页面只用一句提示说明用途） |
| 呼号查询三合一（S6） | `/callsign` · `core/callsign_insight.rs` | 查到呼号后补三行：日志里的通联次数与最近一次（日期 / 时段 / 波段）、从本台看的方位、大圆距离；没有本台网格时只答「打过几次」并提示去填，方位留空不猜。**顺带修掉一个既有缺陷**：结果区整列标签原来直接写中文字面量，英文界面下不翻译，已改为语义 key + 渲染时 `t()`。5 个单测钉死大小写 / 空格匹配、同日按时段排序、无网格降级、方位落在东侧 2000 km 量级；e2e 钉死三行与空态 |
| NEC 设计存档对比（S7） | `/nec` 设计存档 · `pages/nec/design_archive.rs` | 勾选两份存档并排比较：设计频率 / 地面模型 / 最大增益 / 前后比 / 驻波比 / 辐射效率 / 最大增益仰角，差值列为「后者减前者」。两份都在本地重新求解，选满两份才跑。**频率或地面模型不同时页面明确警告**——否则「八木 6.86 dBi < 偶极 7.96 dBi」会被读成八木更差，实际是自由空间 vs 理想导体地面。e2e 钉死 7 项指标与这条警告 |
| 噪声本底记录（S5 相对版） | `/sdr-waterfall` · `core/noise_floor.rs` | 从 Web Audio 的 dBFS 频谱取 **10% 分位**作本底（均值会被同频强信号抬高十几 dB），另给 99% 分位与「本底 +6 dB 以上」的 bin 占比；记录按标签存 localStorage（最多 100 条），相邻同标签给出差值。**只做相对比较、不给 dBm**。5 个单测钉死分位、占用率、静音帧与差值；e2e 只覆盖「无频谱时如实提示」——成功路径依赖浏览器音频栈的启动时序（试过 `--repeat-each=3` 全红），不能当硬断言，算法不靠它 |
| e2e 覆盖门禁 | `crates/tools/src/e2e_coverage.rs` · `Makefile.toml` | 把「每个交互页都有专属 e2e」从口头规矩变成硬校验：`coverage-targets.json` 的每条 journey 的 `grep` 必须能在 `e2e/tests` 的 spec 里命中，作为 `e2e-check` 挂进 `cargo make check`。**第一次跑就抓到两条失效条目**：手机宽度用例的标题是模板字符串插值（`（${name}）`），清单里却写死了渲染后的「（中文）」，已改成前缀匹配。临时塞错一条 grep 验证过门禁确实会红。只做正向校验，不反推「每个路由是否都有用例」——后者要靠白名单排除纯展示页，误报率高于收益 |
| Feld Hell 发端 | `/tools#mode-encoder` · `crates/hell` | 直接复用解码器的 `modulate`（自绘字库 + 7×7 点阵合成），导出 8 kHz 开关键控 WAV；字库里没有的字符跳过。往返由 crate 内 `modulate → decode_audio` 单测保证，不需要外部参考向量；e2e 钉文件名与非空。**之前误判为「需先有标准 Hell 字库」——字库本来就是解码器的一部分，一直具备条件** |
| NAVTEX 发端（D3b） | `/tools#mode-encoder` · `crates/navtex` | 报头（台站字母 / 类型字母 / 两位序号）可配置：`synthesize_message` 复用解码侧 `message_codes` 与 `encode_char`，合成 100 波特 FSK 基带 WAV；`synthesize` 改为它的固定报头包装，原有样本生成行为不变。往返单测断言解码端把 `F` / `B` / 42 与正文原样读回。页面三个字段任一非法即不产出音频 |
| 覆盖图的地图与 RBN 粗方位（S1b 收尾） | `/psk-reporter` · `/rbn` · `components/spot_coverage.rs` | PSK Reporter 的接收台按网格中心落到共享 `MapView`（点只表示位置，没有强度）；RBN 的 spot 没有网格，按 DXCC 实体中心换算成 4 位网格，组件用 `coarse` 明确标注「粗定位」，且因接口没有时间戳不画时段条。`/psk-reporter` 因此拆成文件夹模块（页面 + `coverage_map`）。e2e 钉死点数、粗定位提示与「时段条不出现」 |
| 方向覆盖的时段分布（S1b 部分） | `/psk-reporter` · `core/spot_coverage.rs` | 报告按 UTC 小时分桶（`Coverage.hourly` + `busiest_hour()`），页面画 24 格时段条并标出最忙时段；小时由接口时间戳纯算术换算，不经本地时区。单测钉死越界小时取模与「无报告时无最忙时段」，e2e 钉死 14:00 UTC 那条 |
| 竞赛赛前目标与配速（D2） | `/contest-log` · `core/contest_pace.rs` · `contest_log/pace.rs` | 目标 QSO 数与计划时长随场次持久化（`Session` 加 serde default 字段）；赛中用线性进度算「应完成 / 领先落后 / 当前速率 / 剩余时间需要的速率 / ETA / 预计总量」，每 60 秒刷新一次，到点或超额时如实说「已达到目标 / 计划时间已用完」。单测钉死 0 时长、时间用完、负已用时间等边界；e2e 用未来起始的场次种子钉死「领先 2 条」 |
| DXCC 申请记录表（D1） | `/log` 奖状面板 · `core/award_apply.rs` | 每个实体取一条代表通联（**已确认优先、其次最早**），列出实体 / 呼号 / 日期 / 波段 / 模式 / 确认途径，可导出 CSV 作核对清单。**不宣称能直接上传**：ARRL 在线申请没有公开格式，页面正文写明这是逐条核对的清单。单测钉死优先级、去重与排序，e2e 钉死「JA1ABC 被已确认的 JA1XYZ 取代」与 CSV 下载 |
| 方向增益（S2） | `/nec` · `core/antenna_coverage.rs` · `pages/nec/direction_table.rs` | 按 [`docs/domain/antenna-coverage.md`](docs/domain/antenna-coverage.md) 实现：每个 DXCC 实体一行，给方位 / 大圆距离 / 该方向增益 / 相对峰值损失；两种口径（各仰角最大值 / 水平面切面）；本台网格缺失时不硬算，收起时不渲染表格。8 个单测钉死插值、峰值选取、前后比与排序；e2e 钉死提示文案、340 行与「换地面模型数值跟着变」。**已知边界**：`pattern3d` 10° 网格会低估窄波束峰值（设计说明的方案 B 待实测后再决定），实体中心对大国误差可达 10° |
| 低频段（2200m / 630m） | `core/bands.rs` · `core/bandplan.rs` · `/compliance` | `AMATEUR_BAND_EDGES` 补 135.7–137.8 kHz 与 472–479 kHz，`BAND_PLANS` 各补一段「仅 CW / 窄带数据」（LF 标 e.i.r.p. ≤1W，正文 en / es 已译）。`band_of`、日志体检、互调标注、竞赛波段统计、CHIRP 导出随之支持；`/compliance` 对 0.136 MHz 给出「可用 / ≤1 W / 5.67A / 需协调」。单测守住 edges ↔ band_of ↔ bandplan 三方一致，e2e 钉死波段卡片与合规结论 |
| i18n 重复 key 门禁 | `crates/tools/src/i18n.rs` | 解析前用 `MapAccess` 走一遍原始 JSON，检出同一对象内的重复 key（`serde_json::Map` 会静默覆盖，之前两次踩坑就出在这里）。`load_entries` 直接失败并列出 key；单测覆盖顶层 / 嵌套 / 数组内重复，且值里出现同名文字不误报。临时塞一个重复词条验证过门禁确实拦下 |
| EME 计划器（月位 / 月相 / 回波多普勒 / 共同窗口） | `/eme` · `core/moon.rs`（月球算法）· `core/eme_plan.rs`（输入校验与组装）· `pages/eme/eme_planner.rs`（只管 DOM） | 月球位置取 Schlyter 低精度公式，站心量严格带视差，距离变化率用 ±30 s 中心差分。页面给本台（可选加对方）此刻的方位 / 仰角 / 月距 / 相对平均月距的路径损耗 / 回波多普勒（8 个频段）/ 月相，未来 24 小时月球在地平线之上的时刻表，以及本台与共同窗口（1 分钟分辨，最低仰角可调）。**验证**：对拍 JPL Horizons（DE441）的 48 个站心参照值（三个半球 × 2025–2031 年 + 北京 1992–2056 年抽样）：最大误差角度 0.053°、月距 0.12%、变化率 0.0018 km/s、受照 0.70 个百分点；e2e 的期望值全部来自 Horizons。**不收**（没有可对拍的来源）：月面噪声温度、绝对路径损耗、天平动衰落、法拉第旋转。**边界**：真仰角不含大气折射；UTC 直接代力学时（差 0.01°）；精度够 50–432 MHz 阵列，10 GHz 级抛物面（波束约 0.5°）不够；对方 Doppler 与双站联合补偿没做 |
| 共模扼流圈（ferrite 抑制） | `/tools#common-mode-choke` · `core/common_mode_choke.rs` | 选磁环与穿孔匝数，估算 160–6 m 各波段（以中心频率代表）的 \|Z\|、R、X、电阻占比与插入损耗（按纯电阻回路）。数据是 Fair-Rite 官方零件页的复磁导率 CSV（43 号、61 号，只嵌入 ≤ 300 MHz 的点；再高是尺寸谐振与夹具影响，不收），`Z = jωL0(µ' − jµ'')`，Ae / le 复用 `toroid::CORES`。验证：独立 Python 脚本（自带对数插值）算出 5 组 R / X 入单测，10 kHz 处电抗与厂商 AL 一致；e2e 钉死 40 m / 20 m / 160 m 的整行数值。**边界**：不含匝间 / 对地杂散电容，自谐振点之后真实阻抗会下降，所以高频端与大匝数偏高、不给自谐振点；复磁导率是小信号、25 °C、无直流偏置下测的；回路阻抗用户自填（默认 50 Ω 只是厂商口径）；只有 43 / 61 两种材料，**Amidon 型号仍未收**（见「未完成」）；2 m 及以上、2200 m / 630 m 不在表内 |
| FM 预置补 AO-123（ASRTU-1） | `core/sat_doppler.rs`（`PRESETS`）· `core/satellites.rs` · `/satellites` | NORAD 61781，上行 145.850 / 下行 435.400 MHz（V/U FM，67 Hz 亚音）。两个独立来源核对：SatNOGS DB 的 FM 转发器条目与 IARU 业余卫星频率协调页（序号 830）；Celestrak 业余组里有它的 TLE（服务端 `sat_track` 按 NORAD 号取）。单测把预置频率与 `SATELLITES` 人工表逐项对照，上行护栏对它放行；e2e 钉死选中后带出 NORAD 与上下行频率。**边界**：不给发射机功率与开机时段（IARU 页没有写），页面只说「当前是否开机请看 AMSAT 状态页」；没有真机验证上行写入，没有实收。其余 FM 候选（AO-27、LilacSat-2、CAS-3H、PO-101 等）的取舍理由见「未完成」的 SatNOGS 转发器库条目 |
| 线性转发器补 JO-97 / TO-108 | `core/sat_transponder.rs` · `/satellites` 线性调谐 | 通带取自 SatNOGS DB，并用 IARU 业余卫星频率协调页当第二来源核对：JY1SAT 下行 145.855–145.875、反相上行 435.100–435.120；CAS-6 下行 145.925 / 上行 435.280（通带中心）。独立 Python 脚本算出 6 个映射点入单测，上行护栏扫描纳入这两颗；e2e 钉死 NORAD、通带文本、中心点映射与 +1 kHz 反相联动。**逐条过滤的理由写在模块头**（FO-82 日心轨道、QB50P1 / UKUBE-1 无出处、QO-100 静止轨道、CAS-11 / PRECURSOR 未发射，都不收）。**边界**：没有真机验证上行写入 |
| `check-ui` 原生控件封闭清单加条数 | `crates/tools/src/ui_check.rs` · `docs/ui/pixel.md` §10.2b | `NATIVE_CONTROL_ALLOWED` 从「文件 + 理由」改成「文件 + **精确命中行数** + 理由」，多一处、少一处、写光了（过期例外）都红；漂移判定抽成纯函数 `native_control_drift`，单测钉死三个方向。**变异验证过**：故意把 `palette.rs` 的登记数改成 2，`check-ui` 报「2 → 1」后红，还原即绿。pixel.md 的例外表加了「处数」列与它对账 |
| FT8 基带发端 | `/tools#mode-encoder` · `crates/ft8` | 报文单独一个输入框（标准报文 `CQ BG1AAA OM89`，认不出标准格式就回退自由文本、最多 13 字符，空报文 / 超长 / 非法字符报错），复用解码器同一份 `encode` → `ft8_encode` → `synthesize`，导出 12 kHz 整个 15 秒时隙（基频 1500 Hz，信号约 12.6 秒、前后各约 1.2 秒静音）。往返闭环由 `crates/ft8` 的「编码 → 合成 → 解码」单测保证（本次追加页面默认报文、整时隙样本数 180 000、不可编码报文返回 `None` 三条）；e2e 钉死 `ft8.wav` 的**精确字节数**（44 + 15 × 12000 × 2）与不可编码报文的提示。**之前误判为「缺参考向量」—— 编码链路早在做解码器时就有 19 个单测对拍**。**没做**：FT4 发端（FT4 解调尚未在真实信号上验证）、多时隙连续发、与电台 PTT / 时钟同步联动（播放时自己对 0 / 15 / 30 / 45 秒） |
| 字体许可证版权行核对 | `public/fonts/LICENSES` | Bricolage Grotesque：版权行与 Google Fonts 的 `ofl/bricolagegrotesque/OFL.txt` 首行及字体文件内嵌 name 表 ID 0 逐字一致（原先对），但上游**没有声明 Reserved Font Name**，原先写的 "Bricolage Grotesque" 保留字体名已更正。**IBM Plex Mono 原先写错了**：许可文件里的版权行是 `Copyright © 2017 IBM Corp. with Reserved Font Name "Plex"`（Google Fonts 与 IBM 官方 `IBM/plex` 仓库首行逐字一致），而 `Copyright 2017 IBM Corp. All rights reserved.` 只是字体文件内嵌的元数据措辞，已改为前者并把后者作为注解保留；原先附录里的商标句在两处上游都找不到，已删除。核对用的是 fontTools 读 woff2 的 name 表 + 上游原文 |
| 软件许可页入口（原条目前提不成立） | 页脚 · `kit_modern/footer.rs` `app/pixel_footer.rs` | 原条目称 `/license.html` 只有 ⌘K / 直接 URL 能到。实际两套皮肤的页脚都有 `/license.html` 链接，`navigation.spec.ts` 的 `footer a[href="/license.html"]` 已覆盖点击，无需再加目录面板入口，条目撤销 |
| 台站链路预算向导 | `/tools#station-link` · `core/station_link.rs` · `pages/tools/station_link.rs` | 把发射功率、发射馈线、两端天线增益、路径（自由空间 + 附加损耗）、接收馈线、接收机 NF、外部噪声 `Fa`、带宽与所需 SNR 串成一条链，输出余量、到达 SNR、最远距离、所需功率，以及五个环节各自单独做到理想后多出的余量（**回收潜力**）并降序排名。馈线损耗复用 `feedline_loss_db`，FSPL 复用 `link_budget`，不另起常数。信号与噪声都折算到接收天线端子，噪声按 `f = Fa − 1 + L·Fr` 累加，所以排名随 `Fa` 翻转：外部噪声大时接收馈线与 NF 几乎不花余量、只有热噪声时一比一——这正是 HF 上「馈线损耗无所谓」而 VHF / UHF 上相反的原因。验证：不共享公式的 Python 原型（逐级功率累加，与闭式相对误差 4e-16）给出全部锚点；单测含往返（最大距离 / 所需功率代回余量为 0）、回收潜力与闭式逐位一致、非法输入与近场标记；e2e 期望值取自 Python 原型。**边界**：路径只有自由空间（不含天波、绕射、多径，`max_dist_km` 只是「FSPL 用完余量」的距离）；`Fa` 可自填或选 P.372 人为 / 银河预设，留 0 对 HF 与嘈杂环境偏乐观；不含冷天空（卫星 / EME）、失配与极化损耗（请并入附加损耗）；近场（< 10 个波长）只标记不修正；各模式的所需 SNR 门限不内置；回收潜力是各环节单独换成理想的上限、彼此不可加 |
| ATU 覆盖综合（D4 的一半） | `/tools#atu-synth` · `core/atu_synth.rs` · `pages/tools/atu_synth.rs` | 给定负载阻抗矩形区域（R 对数 7 点 × X 线性 9 点）、元件范围与 Z0，逐波段（160–6 m，下沿 / 中心 / 上沿三频都满足才计入）判定 Pi 低通 / T 高通三元件网络能否把整片区域匹配到 Z0；多出的一个自由度取「虚拟电阻 Rv」沿对数网格扫 400 档（单段 Q 到约 30），任一档元件值落进范围就算覆盖。另给每个波段电容 / 电感的**必要下限**（各自必要、合起来不充分，充分性看覆盖点数）；容性负载下会出负电容的组合如实判不可行。验证用往返：`input_impedance` 逐元件正向电路分析（独立于闭式推导），5000+ 个解代回应得 `Z0 + j0`；另有手算锚点与 6 组独立 Python 对拍。**边界**：Pi-L 不做（后接电感是抑制谐波的设计选择、没有唯一解）；2200 m / 630 m 不做（元件范围对不上）；元件按理想无损处理，不含杂散电容、自谐振与损耗；负载与频率无关 |
| 现代风裸表格滚动容器的键盘可达 | `ui/table/scroll_region.rs` · `scroll_overflow.rs` · `table_section.rs` · `pages/reference.rs` | 抽出 `ui::ScrollRegion`：溢出时自动补 `tabindex="0"` + `role="region"` + 可访问名，不溢出不进 Tab 序；溢出检测（`ResizeObserver` 与句柄释放）从 `Table` 抽成 `use_scroll_overflow`，两者共用一份实现。`TableSection` 现代风分支与 `/reference` 的两处手写 `md-table-scroll` 改走它（没有新增 / 删除 `md-*` 类）。e2e 在 `modern_breakpoints.spec.ts` 的 320px 用例里逐个断言溢出容器的三个属性，**变异验证过**（把 `tabindex` 改成 `-1` 即红）；`/eme` 现代风 390px 用 axe 复测 `scrollable-region-focusable` 零违规；现代风 `modern_a11y` / `mobile_overflow` 与 `i18n_layout` 全绿；`Table` 容器的 `aria-label` 随后也改成跟 `role` 走（不溢出时名字只留给内层 `<table>`），与 `ScrollRegion` 同一口径 |
| MFSK / THOR 解码 | `/mfsk-decode` `/thor-decode` · `crates/mfsk` · `crates/thor` · 各自的 `*-worker` | MFSK 4 / 8 / 11 / 16 / 22 / 31 / 32 / 64 / 128 与长交织 64L / 128L，THOR micro / 4 / 5 / 8 / 11 / 16 / 22 / 25 / 32 / 44 / 56 / 25x4 / 50x1 / 50x2 / 100（含 K = 15 的高速模式与二次信道 ID 信标）。转写自 fldigi（GPL-3.0，整个 crate 按 GPL-3.0）；卷积码 / Viterbi / 交织器 / Varicode 在 `ham-web-mfsk`，与 DominoEX 的 FEC 模式共用。**验证四层**：与 fldigi 上游代码逐位对拍（`upstream_xcheck`）、真实录音（gMFSK / G4WNC）、fldigi 4.2.13 自己的收发端双向验证、合成往返；数字与复现命令见 `crates/mfsk/reference/` 的两份 README。**边界**：THOR 用硬位判决（白噪声下实测不输 fldigi 的软位路径，所以不做）；没有长的 THOR 真实录音；MFSK4 / 8 的交织深度 fldigi 与 gMFSK 不一致，两种都试；THOR56（16 kHz、符号长 290）只在合成录音上验证过；协议与门限见 [`docs/domain/decode-worker-protocol.md`](docs/domain/decode-worker-protocol.md) |
| MFSK / THOR 接收前端 | `ham_web_mfsk::{track, front}` · 两个 `rx.rs` | **漂移跟踪**：按块测符号边界与音调梳偏差，拟合 1–3 阶平滑曲线（BIC 选阶；拟合不可信或修正太小就逐位退回不跟踪）；短录音块长缩短；MFSK 估出 ≥ 1500 ppm 的时钟误差时把录音重采样回名义时钟再解一遍（时钟误差会把音调频率一起缩放，而中心频率检测在跟踪之前）。**中心频率**：探测窗口在格内抖动抽样（避免与周期重复的报文混叠）；MFSK 用前导 / 冲刷的最低音调当锚（短报文只用少数音调时评分定不准梳）；MFSK4 / 8 的交织深度按平均路径度量择优。**窄带载波**：整段估载波，不比信号强的在功率域减掉、更强的抹掉该音调位（载波与信号等功率时 fldigi 一遍也解不出）。**边界**：时钟跳变与不超过半个音调间隔的突然跳频能解，超过半个间隔不能；±12000 ppm 的长录音不行；载波比信号强 12 dB 且噪声大时不行；自动检测中心频率那一步不做载波处理（有载波请手填）；没有锚（录音从发射中途开始又没有冲刷）时评分核对仍可能偏。机制与取舍写在各函数的文档注释，实测数字见 [`crates/mfsk/reference/fldigi_web/README.md`](crates/mfsk/reference/fldigi_web/README.md) |
| 天线补充专题（垂直天线与地网 · 陷阱 · 对数周期 · dBi / dBd 口径） | `/antenna-topics` · `core/antenna_topics.rs` · `pages/antenna_topics.rs` · `nec/reference/antenna_topics_necpp.py` | 阅读型，四块各一个 `ConceptsSection` + 要点列表。**写进去的数字都有出处，没有出处的写成定性**：垂直天线 / 偶极子 / 下垂地网 / 反射增益的数字由独立的 necpp（PyNEC 2.4.0）核对（单测钉死文字与脚本输出一致，脚本改了要同步）；LPDA 的 Carrel 设计式写成 `lpda_design`，对着 Carrel 1961 与 Cebik 的整理逐项核对；FCC 对 AM 广播地网的要求逐字取自 47 CFR §73.189(b)(4)；陷阱的定性结论来自 ARRL《QST》1996 年 11 月 Lab Notes 与 ARRL Handbook；增益 / 远场距离 / 替代法来自 IEEE Std 145-2013。三语齐全。**边界**：地网根数与效率的具体百分比、LPDA 的增益数值没有可核对来源，一律定性；下垂地网的 50 Ω 是自由空间结果，近地会变；NEC 数字是另一套 NEC-2 实现的计算，不是实测 |
| 知识库正文：`concat!` 拼中文 | `crates/core/src` · `crates/tools/src/knowledge_i18n.rs` · `e2e/tests/knowledge_concat.spec.ts` | 译文以运行时整句为 key，而导出工具按行取字面量，`concat!(…)` 写的正文在词典里只有片段、命中不到，英文 / 西语界面整段停在中文且没有任何报错。**规则**：知识库正文一律写成单个字面量（不改工具去模拟编译期拼接，免得两处各维护一份「怎么拼」）。**门禁**：`knowledge-i18n-check` 先扫 `crates/core/src`，任何拼中文的 `concat!` 直接红并列出 `文件:行号`；e2e 每页取一条原先拼接的句子，en / es 下译文必须出现、原中文必须消失。**边界**：门禁只管 `crates/core/src`（正文的唯一来源）；合并后的译文句内衔接没有母语审校 |
| NEC 传输线：交叉线 / 同段并联 / 线长符号（LPDA 模型） | `crates/nec/src/nec.rs` · `nec_templates.rs`（`lpda-14-28mhz`）· `reference/tl_necpp.py` · `reference/necpp_deck.py` · `examples/deck_report.rs` | `TL` 的 `Z0` 为负（NEC-2 约定）= 交叉线：把一端的两根导体对调，`.nec` 导入保留符号、导出写回（恒等式：交叉线 ≡ 同一条线再加半个波长）；同一 `(段, 位置)` 上的端口与激励源共用一个间隙电压并加电流守恒（NEC 的口径是并联，以前是串联叠加）；线长符号改正（旧实现等效于负电长度，只测 λ/4 与 λ/2 所以从没红过）；端口落在导线端段内部时不再放开端点，只有恰好在端点上（`at = 0` / `1`）才放开。**对拍**：与 necpp（PyNEC 2.4.0）比两根平行振子 96 组（共段 / 分段、六个 `Z0`、自由空间与理想地）：共段 `ΔZ/\|Z\|` 中位 1.7%、最坏 8.6%，分段中位 4.1%、最坏 18.8%，增益差 ≤ 0.48 dB；8 元件 LPDA 12 组阻抗差最坏 20%、增益差 ≤ 0.16 dB。**LPDA 模板**：τ = 0.90、σ = 0.16、12 根振子，11 条 `Z0 = −150 Ω` 的交叉线、激励在最短振子；五个频点与 necpp 阻抗差 ≤ 2%、增益差 ≤ 0.06 dB；`Z0 = 150 Ω` 只是让馈电阻抗落在 110–130 Ω，**不是最优设计**。**边界**：`TL` 两端并联导纳（`F3`–`F6`）仍不支持；没建模 LPDA 后方的终端支节；12 元件共 180 段（上限 200）；LPDA 增益与 σ、τ 的关系曲线没有可程序化引用的数据。机制见 `nec.rs` 的文档注释 |
| MFSK / THOR 图片模式 | `crates/mfsk/src/picture.rs` · `crates/thor/src/picture.rs` · 两个 `rx.rs` / `tx.rs`（`encode_picture_audio`）· `components/common/decoded_pictures.rs` | MFSK 16 / 32 / 64 / 128 的 `Pic:<宽>x<高>[C][p<2|4|8>];` 与 THOR 11 / 16 / 22 / 25 / 32 / 44 / 56 的 `pic%<码>` 报头 + 模拟调频图像（彩色每行 R / G / B 三遍）。MFSK 由报头位置推图像起点；THOR 先找同步音，用它的频率反推中心频率。页面显示图片、可下载 PNG，正文里去掉图像段乱码、保留报头。**验证**：fldigi 4.2.13 桌面版的真实图片录音（MFSK16、THOR16 / 25，四个象限中心与预期相差 ≤ 1）、fldigi 发端的图片逐字节与原图比（THOR 上本仓平均误差 2.0–14.0，fldigi 自己的接收机 2.2–40.7）、本仓发端往返。**边界**：只在模式的标称采样率上解图（别的采样率只给文本）；没有真实电台的图片录音；没有 fldigi 里的图像校正；THOR 的报头没有同步音（被噪声吃掉）时当普通文本；MFSK 的 `p4` / `p2` 快速模式 FM 带宽不够、边沿很糊（fldigi 自己的接收机一样）。数字见 `crates/mfsk/reference/fldigi_web/README.md` |
| 大气噪声（ITU-R P.372 第 5 部分）与多噪声合成 | `core/src/atmospheric_noise.rs` · `atmospheric_coeffs.rs`（生成物）· `core/reference/p372_coeffs.py` · `p372_check.py` · `pages/tools/{station_link,external_noise}.rs` | 雷电引起的大气噪声：中位 `Fam` 与上下十分位，按月 / UTC 小时 / 纬度 / 经度 / 频率给出，接进台站链路预算的「外部噪声来源」下拉，并有独立工具 `/tools#external-noise`（11 个业余波段的大气 / 人为 / 银河与合成值及主导来源）。系数表由 `p372_coeffs.py` 从 ITU-R SG3 公开仓库 `ITU-R-HF` 的 `P372/Data` 抽出，算法逐行对照官方 `Noise.c`（14.3）；**许可**：该仓库 README 允许实现者免版权声明地使用，**没有**使用推荐书 PDF 的图。**验证**：`p372_check.py` 把官方源码编成命令行程序，大气噪声 235 872 组与合成 334 152 组**最大差 0.0（逐位相同）**；对拍网格含非 15° 整倍数的经度与带小数的小时（整倍数网格测不出本地时折算的截断顺序，曾因此漏过一个最大 32 dB 的缺陷），官方源码固定在 `OFFICIAL_SHA`。系数表存 `f64` 原样（`f32` / `i16` 省体积都破坏逐位一致，见 [`docs/engineering/performance.md`](docs/engineering/performance.md) §5.6）。**与正文不同处**：官方程序对银河噪声不分频段（正文说 10 MHz 以下被电离层屏蔽），本站跟官方程序；合成取 `min`（注释写 Worst-case，代码是 `min`）。**没做**：幅度概率分布（APD）与脉冲成分、室内与局部地形、`P.372-18` 的新常数（仓库那份对应 P.372-14，18 版只改了表述，未核对）、`duss` 表。官方 `ReadFamDud()` 的月份 0 起、`fakp` 的 reshape 顺序等坑都写在模块注释里 |
| 地波场强（ITU-R P.368 的 LFMF 平滑地球法） | `core/src/ground_wave/` · `core/reference/lfmf_check.py` · `examples/lfmf_dump.rs` · `pages/tools/ground_wave.rs`（`/tools#ground-wave`） | 移植 ITU-R P.368-10 的官方源码（NTIA/ITS；**许可**：NTIA 雇员作品不受版权保护并允许免版税衍生，要求改动处注明、致谢 NTIA；P.368 的 PDF 本身仍是 All rights reserved）。两条解法自动选择（平地 + 曲率修正 / 残留级数），返回场强、基本传输损耗、接收功率与所用方法；页面按频率 / 距离 / 极化 / 两端天线高度 / 功率 / Ns 计算，地面类型取 P.368 图 1–9 的 9 组常数（出处 ITU-R R-HDB-59 表 2；平均盐度海水取 εr = 70，与图题和 P.527 一致）或自填 εr / σ，并列出 1–1000 km 十个距离的衰减表，场强低于 −40 dB(µV/m) 时加提示。**验证**：`lfmf_check.py` 用 clang 编译官方原件，与本站比 4458 组，主网格最坏差 4.8e-8 dB、边界角点 3.9e-5 dB；e2e 期望值取自官方程序。Airy 系数表用脚本从 `Airy.cpp` 抽出而不是手抄。**没做**：ITU 区域地面电导率图（没有可程序化引用的数据，推荐书 PDF 也是 All rights reserved，所以让用户填）、海陆混合路径、地形起伏——原件本身也只做平滑球面 |
| 流星雨时刻表（IMO） | `core/src/meteor_showers.rs` · `components/meteor_shower_section.rs` · `/events` · `/meteor-scatter` | IMO《Meteor Shower Calendar 2026》表 5 的 37 条（不含全年弥散的 Antihelion Source 与日间前向散射用的表 7）：代号、中英文名、活动期、峰值、λ⊙、赤经赤纬、进入速度、星等指数、ZHR。**许可**：IMO 允许署名、不暗示背书地「use, reproduce and share」年历数据，这里只取数值字段做站内呈现，页面与卡片都有署名行。按「正在活动」或「30 天内到峰值」筛选、按倒计时排序（逻辑在 `core` 的 `timetable()`，页面只渲染）；峰值已过但活动期未结束的写「峰值已过」；活动期跨年的按年末 / 年初两段判定。**边界**：峰值日**只对 2026 年精确**（峰值由太阳黄经决定，换年会差一天上下）；没有收 IMO 的爆发预报、月球干扰、表 6 逐日辐射点与表 7 日间射电流星雨。中文名走 `core_text_tracked` 词典（en / es 译名各 37 条）；e2e 固定 UTC 时钟，需在带 postbuild 词典的 release 产物上跑 |
| 铁粉芯（Micrometals）补进磁环库 | `core/src/toroid.rs` · `core/src/common_mode_choke.rs` | 补 12 个铁粉芯（`-2` 材料 5 个、`-6` 材料 7 个），AL / Ae / Ie / 尺寸取自 Micrometals 官方 product finder，公差取官方 RF 目录的 MAGNETIC TOLERANCE 表；不用 Amidon 的型号做料号（同一批磁芯的分销型号，两套编号各错一半不如只留一套）。两家厂商都没有数据使用条款，规格数据按 Fair-Rite 先例标「厂商页面 + 料号 + 核对日期」，不说「有许可」。粉芯的校验改查几何恒等式 `Ae ≈ H·(OD−ID)/2`、`Ie ≈ π·(OD+ID)/2`（`AL ≈ µ0·µi·Ae/le` 只对铁氧体成立；`AL ∝ Ae/le` 在大环上同材料就有 ±10% 散布，查不出抄错反而误伤）。**边界**：粉芯不进共模扼流圈（没有复磁导率表，页面按 `material().is_some()` 过滤）；没有 Q 曲线与磁通饱和 |
| `Select` 重开后的 `aria-activedescendant` | `ui/select/select_root.rs` · `select_item.rs` · `e2e/tests/select.spec.ts` | 选中一项后在退场动画内立刻重开，高亮 id 指向已卸载的旧行。**根因不是 `NodeRef` 残留**（最初的假设，补上守卫后仍红）：`SimpleSelect` 的选项是响应式闭包，选项随当前值变化时（`/log` 的「模式」会把当前值补进选项）整批 `SelectItem` 被重建、换新 id，而高亮 id 只在打开时算一次。修法：高亮行卸载时 `SelectItem` 清掉 `active` 并自增 `SelectCtx::reprime`，根组件按代际号重新定位到当前选中行。新用例「选中后再次打开」每种用法 × 两套皮肤各连开合 3 轮，断言指向的 id 与当前选中行一致且真实存在；修复前 5 个组合全红、修复后全绿。此前 `select.spec.ts` 都在「首次打开」或「等收起后再开」的路径上，所以没拦住。**没做**：打开着的列表被重建时，用户用方向键移动过的高亮会回到选中行（旧行已不在，无处可留） |

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

**MFSK / THOR 接收前端**

- **别按能量挑「最响的窗口」去估中心频率**：MFSK / THOR 是恒包络信号，每个窗口的能量几乎一样，差别只来自音调频率与窗口整周期的关系，按能量排序等于偏向某几个音调，梳会偏好几个音调（真实录音上实测：中心偏了 3–5 个音调）。改成「有信号的窗口里沿时间均匀抽」；
- **粗网格的步长要取波特率（窗口分辨力），不是音调间隔**：THOR 的音调间隔是波特率的 2 / 4 倍，音调若恰好落在两个网格点正中，到两边都差整整一个分辨力，矩形窗在那里是零点 —— 干净的合成信号靠泄漏侥幸检出，加一点点噪声就选到 Nyquist 附近（THOR8 / THOR4 加幅度 0.1 的噪声就复现）；
- 中心频率检测放在符号对齐**之前**时，窗口常跨在两个符号上、证据被冲淡；对齐之后再在 ±2 个音调内核对一遍（`polish_base`）；
- **梳评分只罚「梳外邻位比梳内更强」，所以短报文（只用少数音调）上整梳偏低几个音调没有任何惩罚**：MFSK 用发射开头 / 结尾的恒定前导（最低音调）当锚（`idle_tone_shift`）。「并列时取最高」与「向上多核对几个音调」都试过：短报文好了，但周期重复的长报文（前导只占 1% 的窗口）会被错误地推高；
- **在两次解码之间择优，别用绝对分数**：乱码往往比正确文本更长，可读性绝对分数偏向乱码（深度 5 的 `TEST ` 输给深度 10 的 `llll…`）。交织深度用平均路径度量，重采样重来用可打印字符占比；
- **时钟误差会把音调频率一起缩放**（3000 ppm 下 1000 Hz 落在 997 Hz），而中心频率检测在跟踪之前：估出大误差就把录音重采样回名义时钟再解一遍。差分编码的 THOR 不受影响；
- **含图像的 THOR 录音整段 Viterbi 会因「非法半字节占比」直接判无信号**：先找图像前的同步音，用它的频率反推中心频率（比文本里的音调梳检测准得多）；
- 图片测试里「预期图」的左右半判法要和参考实现一致：`x < 宽 / 2` 在 fldigi-web 里是浮点比较，Rust 里写成整数除法会让整幅图误差多 1.5，曾因此误判成解码问题；
- 同名 example 放进不同 crate（`decode_sample` / `generate_sample`），在共享的 `CARGO_TARGET_DIR` 里会**互相覆盖二进制**，直接跑 `target/.../examples/decode_sample` 可能跑到别的 crate 的；用 `cargo run -p <crate> --example`；
- fldigi 的 GitHub 镜像（`w1hkj/fldigi`）停在 4.1.23，比线上的 4.2.13 老：THOR25 / 32 / 44 / 56 在镜像里没有。对拍用的 WebAssembly 版是 4.2.13，两边在共有的模式上完全兼容。

**通用**

- 对防抖 / 异步刷新的输出做快照，要先等它落定再读，别把上一次的值当基准；
- 没有可见标签的输入框，靠 e2e 找不到才暴露；
- `getByText` / `getByLabel` 是子串匹配，短文案要 `exact` 或改用固定 id；
- **增量缓存会掩盖坏代码**：dev 用旧 rlib 能过的改动（缺常量、引用已删除字段、测试缺辅助函数），`wasm-release` 全量构建才会红；改完 core 里被 app 依赖的东西，提交前跑一次 release 构建；
- **长文案会撑破 320 px**：下拉里最长的选项是不换行的一整行，要给容器 `[&>*]:min-w-0`；给 `Table` 外再套 `div.sm:col-span-2` 会让网格子项的最小宽度取表格的 `min-w`，跨列要写进 `wrapper_class`（让 `Table` 自己的滚动容器当网格子项）；
- **对拍网格要覆盖「截断 / 取整 / 取模」这类运算的非整数输入**：网格全是整倍数时，运算顺序写反也测不出来（P.372 本地时折算，见「已完成」大气噪声一行）。出了「逐位一致」的结论后，回头问一句：网格里有没有让每个 `trunc` / `floor` / `%` 真正舍掉小数的点；
- e2e 里「注释写了期望值、断言却只数表格」是空洞断言：改坏内核它照样绿。期望值要么由独立路径（Python）算出来，要么直接来自官方程序，并落到页面读数上。

---

## 工程门禁

新增页面 / 模块必须满足（仓库内已有自动化守护，新增内容自动受益）：

1. **只改两处**：`crates/core/src/registry.rs` 的 `MODULES` 追加一条 + `crates/app/src/app/main_content.rs` 加一行 `<Route>`。
   导航菜单、`sitemap.xml` 均由注册表派生，`registry_check.rs` 断言「注册表 ↔ 导航 ↔ 路由」一致，漏注册会让 `cargo make check` 直接失败。README 的「共 N 个路径」与 [`docs/reference/routes.md`](docs/reference/routes.md) 的完整表都按注册表列出，`cargo make readme-check` 会对账数量。
2. **内核下沉**：不依赖浏览器的逻辑放 `ham-web-core` + 单元测试；技术性数值在 `knowledge_consistency.rs` 加回归断言。
3. **大内核走 Worker**：DSP / 求解器按 `ham-web-apt` 的模式拆 Worker，遵循 [`docs/domain/decode-worker-protocol.md`](docs/domain/decode-worker-protocol.md)，注意 `wasm-bindgen` CLI 版本与 `Cargo.lock` 一致。
4. **文案与翻译**：界面文案同步补 `data/i18n/{lang}/{domain}.json`（语义 key `域.词条`，`cargo make i18n-check` 校验重复 key、占位符、死条目与语言包新鲜度，改完跑 `cargo make i18n-pack`），正文译文放 `data/knowledge-i18n/`（`knowledge-i18n-check` 钉住漂移）。
5. **测试**：新增交互补 Playwright 用例，并把旅程追加到 `e2e/coverage-targets.json`（`grep` 子串需唯一命中）。
   工具 / 考试类路由没有任何功能用例字面量 `page.goto` 过时，`registry_check` 直接失败。依赖 `/api/*` 的页面用 `fixtures.ts` 的 `stubApi` 打桩，时间相关断言用 `page.clock` 固定。
6. **体积**：`cargo make ci` 末尾跑 `size-check`，任何产物超出 `perf-budget.json` 即失败。
   上调上限前先回答「这次增量换来了什么、能不能拆进 Worker」：纯计算且依赖重的放服务端或拆 Worker（像 `/api/sat-track` 那样），只有薄薄一层界面的才进主包。
   预算、现状数字、已评估并排除的方向与工具坑都在 [`docs/engineering/performance.md`](docs/engineering/performance.md) §5（上限怎么定的写在 §5.2，不在这里逐轮记流水）。

---

## 验证方式

```bash
cargo test -p ham-web-core            # 领域逻辑 + knowledge_consistency 回归
cargo test -p ham-web-app             # registry_check（注册表 ↔ 路由一致性、交互页 e2e 门禁）
cargo make check                      # 提交前门禁（组成见 docs/engineering/code-quality.md §2）
cargo make e2e                        # 构建 release + Playwright 全站冒烟与无障碍
cargo make e2e-coverage               # 覆盖率报告 e2e/test-results/coverage.md
cargo make size-check                 # 性能预算（需先 build-web）

# 需要本地样本的测试（样本不进仓库，未设置变量时自动跳过）
LRPT_REAL_IQ=/path/to/meteor-m2.wav cargo test -p ham-web-lrpt --release --test real_iq
```

---

## 相关文档

全部文档的目录与「何时看」见 [`docs/README.md`](docs/README.md)（唯一索引）。与本文直接相关的：

- [`README.md`](README.md) —— 上手、功能一览、常用任务与部署
- [`docs/reference/features.md`](docs/reference/features.md) —— 已实现功能的完整清单（本文「已完成」只留结论与指针）
- [`docs/engineering/code-quality.md`](docs/engineering/code-quality.md) —— 新增内容必须过的门禁（§2）
