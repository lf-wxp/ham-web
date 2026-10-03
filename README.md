# 业余无线电｜题库 · 知识 · 工具

[![Rust 2024](https://img.shields.io/badge/Rust-2024_edition-orange?logo=rust)](https://www.rust-lang.org)
[![Leptos 0.8](https://img.shields.io/badge/Leptos-0.8-ef3939)](https://leptos.dev)
[![PWA](https://img.shields.io/badge/PWA-offline-5a0fc8)](#pwa-与离线)

## 项目简介

一站式业余无线电学习平台：**A/B/C 三类题库**的练习与模拟考试、**100+ 专题知识库**速查，以及通联日志、实时数据与常用计算工具。

**全项目使用 Rust（2024 edition）编写**：前端为 Leptos（CSR → WebAssembly），服务器为 Axum（静态站点 + `/api/*` 数据代理），领域数据与算法在 `ham-web-core`，题库/解析/构建工具为 `ham-web-tools` 命令行程序。

目标读者：备考 A/B/C 类操作证的考生、希望系统入门的新手，以及需要日志、奖状与传播工具的进阶火腿。所有个人数据只保存在浏览器本地，不上传服务器；纯静态托管也能离线使用除实时数据外的全部功能。

在线 demo：[业余无线电考试](https://ham.onlyxp.me/)

---

## 目录

- [业余无线电｜题库 · 知识 · 工具](#业余无线电题库--知识--工具)
  - [项目简介](#项目简介)
  - [目录](#目录)
  - [主要特性](#主要特性)
    - [考试中心](#考试中心)
    - [知识库](#知识库)
    - [工具与实时数据](#工具与实时数据)
    - [用户体验](#用户体验)
  - [技术栈](#技术栈)
  - [项目结构](#项目结构)
  - [后端 API 与数据源](#后端-api-与数据源)
  - [快速开始](#快速开始)
    - [环境要求](#环境要求)
    - [本地开发](#本地开发)
    - [生产构建与运行](#生产构建与运行)
  - [使用示例](#使用示例)
  - [cargo make 任务一览](#cargo-make-任务一览)
  - [Docker 部署](#docker-部署)
  - [Web Push 后台推送](#web-push-后台推送)
  - [数据集构建](#数据集构建)
  - [题目解析维护流程](#题目解析维护流程)
    - [核心概念：内容指纹](#核心概念内容指纹)
    - [维护流程总览](#维护流程总览)
    - [第 1 步：统计缺失解析](#第-1-步统计缺失解析)
    - [第 2 步：撰写解析](#第-2-步撰写解析)
    - [第 3 步：合并、增强、写入](#第-3-步合并增强写入)
    - [第 4 步：预览与提交](#第-4-步预览与提交)
    - [维护术语表](#维护术语表)
    - [题库更新后的解析迁移](#题库更新后的解析迁移)
  - [新增题库版本](#新增题库版本)
  - [PWA 与离线](#pwa-与离线)
  - [本地存储与兼容性](#本地存储与兼容性)
  - [开发指南](#开发指南)
    - [快捷键](#快捷键)
    - [代码规范](#代码规范)
    - [端到端测试](#端到端测试)
      - [覆盖率统计口径](#覆盖率统计口径)
      - [集成到 CI](#集成到-ci)
    - [修改 UI 的注意事项](#修改-ui-的注意事项)
    - [动态背景（Web Threads）](#动态背景web-threads)
  - [路由](#路由)
  - [常见问题](#常见问题)
  - [贡献指南](#贡献指南)
  - [许可](#许可)
  - [致谢](#致谢)

---

## 主要特性

### 考试中心

- **📝 模拟考试**：A/B/C 三类考试，按真实规则抽题（单选/多选配额：A 类 40 题/40 分钟、B 类 60 题/60 分钟、C 类 90 题/90 分钟），倒计时、标记、答题卡、交卷计分与合格判定，中途退出可恢复；交卷后可回看题目解析，并展示最近 10 次成绩趋势与本次各分类正确率对比以往
- **🎯 薄弱项组卷**（`/exam?bank=A&mode=weak`）：同样题量与配额，按分类正确率、错题本与未做过的题加权抽题；不计入备考状态与历史趋势，首页复习卡片可一键进入
- **✅ 备考状态**：按 A/B/C 分别绘制模拟考试成绩曲线（含合格线），根据最近 5 次成绩给出「还需多考几次 / 再巩固一下 / 接近合格 / 可以去考了」判定
- **⏱️ 每日挑战**（`/daily-challenge`）：每天一组 10 题限时闯关，同一天抽到同一组题，交卷计入打卡；展示当前连胜与历史最长连胜，首页有今日状态入口
- **🎯 练习模式**：顺序/随机练习、即时显示答案与解析、进度自动保存与恢复；可「只练没做过」的题，专项练习不覆盖顺序进度
- **🗂️ 分类浏览**：按 10 大题目类型与官方分类码浏览，仅显示正确答案，附解析、知识点与参考依据
- **⚡ 闪卡刷题**：看题 → 心里想答案 → 显示答案 → 自评，适合考前高强度过题；移动端可左右滑动（翻面后右滑「会」、左滑「不会」）
- **🎧 听题模式**（`/listen`）：自动朗读题干与选项，停顿思考后读出答案（可选读解析），题源可选 A/B/C 类、错题集或收藏集，可调语速与思考时间，按来源记住听到第几题
- **📻 呼号抄收**（`/callsign-copy`）：听字母解释法（或摩尔斯电码）拼读随机呼号，抄写核对，附正确率与连续答对统计，练习呼号听抄基本功
- **🃏 知识卡片**（`/cards`）：Q 简语、常用缩略语、字母解释法、莫尔斯字符与术语五组卡片间隔复习：翻面后自评「忘了 / 模糊 / 记得」，按难度系数安排下次复习（最长 180 天，忘了 10 分钟后重来），每组每天最多 10 张新卡；字母解释法与莫尔斯可点读 / 播放；支持空格翻面、1/2/3 评分与左右滑动；首页显示今日待复习张数
- **❌ 错题本**：练习、模拟考试与闪卡「不会」自动入本并持久保存；按自适应间隔安排「今日待复习」：每道题有自己的难度系数，答对后间隔按实际间隔 × 难度系数增长（最长 60 天），答错降低难度系数并当天重来；常错的题需要连续答对更多次（3～5 次）才移出，旧数据自动兼容；可按 A/B/C 类筛选与复习
- **🔖 收藏集**：手动收藏重点题目，随时复习与取消收藏
- **🖨️ 打印版**：错题 / 收藏一键生成 A4 打印页（`/print`），可按题库筛选，答案集中在末尾、随题显示或不显示，可附解析
- **🗓️ 备考计划**：设定考试日期与类别，按剩余未做题量、待复习错题与临考阶段（最后 7 天每天一套模拟考试）给出今日任务，并记录每天的作答量（首页与学习进度页展示）
- **📊 学习进度**：练习 / 考试 / 错题 / 收藏 / 日志 / 打卡 / DXCC 的统计总览，按 A/B/C 分别展示题库覆盖率与 10 大分类正确率、定位薄弱知识点；首页可一键进入待复习与最薄弱分类的专项练习
- **📈 学习周报**（`/weekly`）：近两周作答趋势、本周新题 / 复习占比、连续打卡天数与最薄弱分类回顾，附最近模拟考试成绩，一眼看清学习节奏
- **⏰ 倒计时与提醒**：管理多个目标时间（考试日、执照到期等），本地持久化并实时刷新，到期用浏览器通知提醒
- **🔍 全站搜索**：`/` 键或导航栏按钮唤起搜索面板，检索术语、简语与全部知识库条目，命中高亮、点击直达对应页面
- **🧩 只看本类新增**：基于题目内容指纹识别 A/B/C 重合题，只看 B（相对 A）或 C（相对 A、B）新增的题目
- **📷 照片处理**：报名证件照/人像照尺寸处理，完全在浏览器本地完成
- **💾 数据备份**：一键把全部本地数据（进度、错题、收藏、日志…）导出为 JSON，可换设备或清缓存后恢复；支持覆盖恢复或按类型合并导入（日志按 QSO 去重、统计累加、收藏并集），显示本地存储占用与各项明细，写入失败（存储已满）时全站提示
- **🏆 成就墙**（`/achievements`）：21 项里程碑成就（首考合格、连续打卡、题库过关、DXCC / 通联 / 网格 / 竞赛 / 收藏等），达成时右下角弹出解锁提示；成就墙集中展示已解锁与未解锁项及进度
- **📊 学习报告**（`/report`）：汇总累计作答、正确率、连续打卡、错题、收藏、成就与通联 / DXCC 统计，一键导出 PNG 卡片分享
- **📝 题目笔记**：练习 / 复习时可为每道题写私人笔记（本地保存、随题加载）
- **🎯 只练多选**（练习页顶部开关，或 `/practice?multi=1`）：切到只刷多选题，随机顺序

### 知识库

按 5 个分组组织的 100+ 专题速查页（桌面端多列下拉，移动端分组平铺）：

| 分组 | 专题 |
| --- | --- |
| 备考速查 | 考试速查、考点速查手册、易混淆辨析、公式速查、呼号前缀、术语表、简语、字母解释法、RST 信号报告、莫尔斯电码、CW 操作、操作证权限 |
| 模式 · 传播 | 模拟模式、业余电视、SSTV 慢扫描电视、数字模式、数字语音组网、Packet 分组无线电、RTTY/PSK31、FT8/FT4、SDR、GNU Radio、APRS、常用频率、频率协调、CQ/ITU 分区地图、传播与电离层、国际信标网络、特殊传播、流星散射、EME、传播预测、WSPR、气象卫星接收、极光通信 |
| 天线 · 设备 | 天线型式、天线阵列/相控阵、天线极化、匹配与馈线、巴伦与不平衡变压器、天线 DIY、实用天线专题、天线架设、天线农场、天线调试、天线分析仪、VNA 矢量网络分析仪、天线建模、NVIS、电子电路基础、滤波器与双工器、测量仪表、电源与电池、电源供应、收发信机、接收机指标、设备评测与选购、功率放大器、波段表、波段规划、微波通信、车载/移动电台 |
| 通联 · 活动 | 通联实务、卫星通联操作、通联竞赛、竞赛日志 Cabrillo、DX 奖状、IOTA、DX 技巧、DX 远征、DXCC 世纪俱乐部、QRP、电子 QSL、QSL 卡片设计、无线电测向、应急通信、Winlink 无线邮件、活动日历、SOTA/POTA、网格定位、呼号查询、中继台与网关、中继台建设、日志与竞赛软件 |
| 进阶 · 关于 | 国际组织与分区、射频安全、接地与防雷、射频干扰排查、接收环境与噪声、新手入门、学习路径、学习资源、SWL 短波监听、执照申办、法规与管理、业余无线电历史、远程电台、开源项目与 DIY、火腿社区、DIY 实战项目 |

其中几项为富交互页面：莫尔斯电码可点击试听、Koch 法抄收训练（Farnsworth 间隔、逐字符错误率、达标自动加字符）、CW 呼号抄收竞赛模拟（每轮 10 个通联、抄呼号与序号计分、自适应速度、截短数字、常错字符统计）、解码练习与按键发报练习、麦克风 CW 解码（带通滤波 + 自适应门限，自动识别发报速度）、字母解释法可朗读、RST 页可试听不同强度信号、波段表按带号 -1～12 呈现完整频段划分（移动端为卡片）、天线建模页含偶极振子计算器与方向图绘制；设备评测页内置机型库与多机参数对比表。每个专题页底部还有「相关主题」内链与「自测 3 题」（从题库相关考点随机抽取、即时判分），把专题串成知识网。

### 工具与实时数据

- **🧮 小工具（41 项计算器 + 数据备份）**：频率 ↔ 波长、dBm ↔ 功率、分贝增益、欧姆定律、CW 必要带宽、LC 谐振、容抗/感抗、天线长度、天线匹配网络、史密斯圆图、驻波比 ↔ 反射系数、级联增益、电阻串并联、频率单位换算、电池续航、dBm ↔ dBμV、馈线损耗、滤波器设计、呼号查询（DXCC 实体/稀有度）、两点距离与方位角、传播预测 MUF、卫星多普勒、自由空间路径损耗、EIRP、链路预算、接收机灵敏度、噪声系数级联、天线增益换算、电阻色环、竞赛记分、线圈/Yagi 振子计算、T/π 型衰减器、变压器阻抗、UTC 时间、传输线阻抗、线径压降、晶体振荡、射频暴露评估、亚音与中继频差、APRS 编解码、数字模式编码
- **🧭 点对点传播预测**（`/muf` 页内）：输入双方 Maidenhead 网格、月份与太阳黑子数，估算两点间各 HF 波段的可用性与可靠度（简化 VOACAP 模型，本地计算 + 服务端缓存）
- **📈 实时仪表盘**：太阳活动、空间天气警报、DX 热点、ISS 位置、DXCC 稀有度聚合一屏展示
- **☀️ 太阳活动**：太阳通量、A/K 指数、黑子数与各波段传播条件（含 K 指数 1 分钟曲线与太阳黑子周期趋势）
- **📡 DX 实时热点** / **🏆 DXCC 稀有度榜单**：拉取 DX Cluster 热点与 Club Log 最稀有实体；对照本地日志标出新 DXCC / 新波段，可只看需要的；可开启新 DXCC / 新波段 / 关注呼号（支持 `*` 通配）浏览器通知，开启后每 60 秒自动刷新
- **📡 接收报告查询**：PSK Reporter（`/psk-reporter`）与 RBN 信标网络（`/rbn`）查询「谁收到了我发的信号」，展示接收台、频率、波段、SNR（RBN 另有 WPM），数据来自 PSK Reporter 与 Reverse Beacon Network 实时流
- **🛰️ 业余卫星**：TLE + SGP4 计算未来 24 小时过境（AOS/LOS、最大仰角与方位角），可一键使用本台网格；收藏卫星后可开启过境提醒（提前 5 / 10 / 15 / 30 分钟浏览器通知，页面打开期间每 30 秒检查）；另有 ISS 实时位置追踪与追踪软件推荐
- **🖼️ APT 云图解码**（`/apt-decoder`）：上传 NOAA 气象卫星 APT 录音（WAV），在浏览器本地解调 2400 Hz 副载波并重建可见光 / 红外云图（纯 Rust DSP，解码在 Web Worker 后台线程完成，不阻塞页面、音频不上传服务器）；可配合过境预报提前录制，并可开启过境前的 APT 录制提醒
- **📡 SDR 在线接收站地图**（`/sdr-map`）：世界地图标注长期公开接收站，一键定位并以新标签直达 WebSDR / KiwiSDR 的在线频谱与瀑布图，并汇总公开接收机目录；无需本地硬件即可收听短波
- **🌊 SDR 瀑布图**（`/sdr-waterfall`）：用麦克风输入实时绘制频谱与瀑布图（FFT 2048、可调 dB 标度），或导入 WAV 文件离线分析（含合成样本），全部在浏览器本地完成、音频不上传；色彩映射与 dB 换算位于 `ham-web-core` 的 `spectrum` 模块
- **🗓️ 活动日历**（`/events`）：展会 / 火腿节与年度通联活动按下一届开始时间排序，可一键加入倒计时提醒，并汇总 DX 远征与竞赛的追踪入口
- **🌗 灰线地图**：实时晨昏圈（日出/日落分界），用于判断低频 DX 灰线窗口
- **🌍 DXCC 世界地图**（`/dxcc-map`）：按通联日志把世界地图着色为「未通联 / 已通联 / 已确认」choropleth，支持按波段切换着色、悬停查看实体、点击实体跳转日志过滤，岛屿等无国界数据的实体以中心点标记；一眼看清 DXCC 进度缺口（边界数据由 `cargo make dxcc-map` 生成）
- **🗺️ 网格地图**：全球已通联 Maidenhead 网格可视化，输入网格码可反查位置（反向地理编码）；日志网格地图可按本台网格绘制按波段着色的大圆通联路径（跨日期变更线正确处理，最多 600 条）
- **⛰️ SOTA / POTA 查询**（`/portable` 页内）：按编号查询山峰 / 公园的名称、积分 / 位置与网格，并对照本地日志统计激活次数
- **📻 中继台数据库**（`/repeater` 页内）：按国家 / 地区拉取 RepeaterBook 收录的中继台（频率、频差、亚音、城市、状态），支持按呼号 / 城市筛选
- **📓 通联日志**：字段对齐 ADIF 3.1 的在线日志（含 MODE/SUBMODE、卫星、SOTA/POTA 等），录入时提示重复 / 新 DXCC / 新波段，并按网格（或实体中心）估算距离与方位、自动填 CQ / ITU 分区，可一键在线查询呼号（Callook / HamQTH）自动补全姓名、网格与 QTH；列表可搜索筛选分页；ADIF / CSV 导入导出（含 DXCC / CQZ / ITUZ、STATE、IOTA），导入时读取 QSL（含 LoTW / eQSL）状态并自动去重；可粘贴或上传 LoTW / eQSL 下载的确认报告（ADIF），一键把匹配到的 QSO 同步为已确认
- **🏆 竞赛录入**（`/contest-log`）：选定竞赛后专注键盘录入（呼号后回车 / 空格跳到交换信息，回车记录，Esc 清空），实时提示重复与实体，CQ 分区类竞赛自动填分区；实时计分、分波段统计，一键导出 Cabrillo 3.0；内置 CQ WW / CQ WPX / ARRL DX / All Asian / JIDX / WAE / IARU HF / 俄罗斯 DX 等模板
- **🗓️ 竞赛日历**（`/contest-calendar`）：全球主要竞赛按下一届开赛时间排序，一键把开赛时间加入倒计时提醒，有对应模板的可直接「开新场次」跳转竞赛录入
- **📻 电台 CAT 联动**：日志与竞赛录入页可通过 Web Serial 连接电台（桌面版 Chrome / Edge），每秒读取频率与模式自动回填；支持 Kenwood / Elecraft / 新款 Yaesu 的 ASCII 命令与 Icom CI-V（可设地址）
- **🏷️ QSL 标签打印**（`/qsl-labels`）：把日志按呼号合并（每张最多 2～6 条通联），排到 Avery L7163 / L7160 / L7159 / 5160 / 5163 标签纸上直接打印；可只打未寄出的、按起始日期筛选、跳过已用掉的标签，打印后一键标记「QSL 已寄出」
- **🏅 奖状进度**：由日志统计 DXCC（总计 / 分模式 / 分波段）、WAZ、WAC、VUCC、WPX 前缀奖、WAS 美国州、IOTA 岛屿组与 DXCC Challenge 分波段积分，可切换已通联 / 已确认
- **🌍 DXCC 前缀库**：内置 340 个现行 DXCC 实体（数据源 AD1C cty.csv），最长前缀匹配，支持 `VP2E/W1AW`、`/P`、`/MM` 等斜杠呼号
- **📊 通联统计**：DXCC / 波段 / 模式分布、QSL 确认率与月度 QSO 趋势

### 用户体验

- **🧭 全局导航**：所有页面顶部常驻导航栏，按「考试中心 / 知识库 / 工具」三大模块组织，移动端为分组平铺菜单
- **🔍 全局搜索**：任意页面按 `/` 或点击导航栏搜索按钮唤起命令面板，结果按页面分组并高亮关键词
- **🌗 明暗主题**：导航栏内随时切换，支持跟随系统 / 浅色 / 深色
- **🌐 多语言界面**：导航栏内切换中文 / English / Español，选择存本地并同步 `<html lang>`；当前覆盖导航、页脚、全站搜索、首页（含各卡片）与页面标题，其余页面正文按模块增量翻译（见 `crates/app/src/i18n.rs`，以中文原文为 key 补词条即可）
- **🔊 语音与音频**：Web Speech API 朗读题干、解析与字母解释法；Web Audio API 合成摩尔斯电码与 RST 信号音（可调 WPM）
- **✨ 流畅动效**：页面切换淡入过渡、按钮按压反馈、Logo 悬停动效，并尊重系统「减少动态效果」偏好
- **🌌 动态背景**：内容之下的 Web Threads 发光丝线（WebGL2），随明暗主题切换配色且不遮挡前景；移动端与「减少动态效果」下退化为静态一帧（详见[动态背景（Web Threads）](#动态背景web-threads)）
- **⌨️ 键盘快捷键**：`/` 唤起搜索，方向键切题，数字键选择选项
- **👆 滑动切题**：手机上在练习 / 模拟考试题目区域左右滑动切换上下题，并有轻微振动反馈
- **♿ 无障碍**：「跳到主要内容」链接、导航与对话框地标命名、对话框焦点循环与关闭后焦点还原、页面标题使用 `h1`、明暗主题均通过 axe 对比度检查；快捷键不会抢占按钮 / 链接上的回车
- **⚡ 首屏性能**：wasm 加载期间显示内联启动画面；术语表数据从 wasm 中拆出按需加载；构建时预压缩 brotli / gzip，服务器直接返回预压缩文件
- **🧯 错误兜底**：wasm 加载失败、浏览器不支持或运行中 panic 时显示兜底页，可重新加载、导出数据备份或复制错误信息
- **🔄 题库更新提示**：记录每个题库的题目摘要，题库内容变化（修改 / 新增题目）时提示「题库已更新：修改 N 题，新增 N 题…」
- **📋 答题卡**：快速导航、标记、未答/标记筛选，交卷后显示对错
- **📱 移动优先**：响应式设计，PWA 可安装、可离线使用
- **🔔 通知中心**（`/notifications`）：集中管理浏览器通知权限与倒计时 / DX 热点 / 卫星过境三类提醒开关
- **📤 成绩分享**：模拟考试成绩与学习周报可导出 PNG 卡片，支持下载、复制到剪贴板与系统分享面板
- **💾 本地存储**：所有数据仅保存在浏览器本地（小数据走 `localStorage`，通联日志等大体积数据由 `IndexedDB` 权威存储 + `localStorage` 快照双写），不上传任何个人数据

## 技术栈

| 层 | 技术 |
| --- | --- |
| 语言 | Rust 2024 edition（MSRV 1.88） |
| 前端 | [Leptos 0.8](https://leptos.dev)（CSR）+ `leptos_router`，编译为 WebAssembly |
| 前端构建 | [Trunk](https://trunkrs.dev)（自动下载 Tailwind CSS v4 独立版、wasm-bindgen、wasm-opt） |
| 样式 | Tailwind CSS v4 + tw-animate-css，沿用原 shadcn/ui（new-york）设计令牌与类名 |
| 图标 | Lucide（内联 SVG，与原版路径一致） |
| 浏览器 API | Web Speech API（朗读）、Web Audio API（摩尔斯 / 信号音 / 实时频谱）、Canvas（照片处理、瀑布图）、WebGL2（全站动态背景，见[动态背景（Web Threads）](#动态背景web-threads)）、Notification（倒计时提醒）、IndexedDB（大体积日志存储） |
| 服务器 | Axum 0.8 + tower-http（SPA 回退、缓存头、gzip/brotli），并提供 `/api/*` 外部数据代理（`ureq` + 内存缓存 + `sgp4` 过境计算） |
| 数字信号解码 | `ham-web-apt` / `ham-web-sstv` / `ham-web-wspr`（纯 Rust DSP，编译为 Web Worker，音频不上传服务器） |
| 工具链 | `ham-web-tools`（clap、ureq、resvg、sha2、brotli、walkdir） |
| 任务编排 | [cargo-make](https://github.com/sagiegurari/cargo-make) |
| PWA | 构建时由 Rust 生成 Service Worker（预缓存 + 运行时缓存策略） |
| 许可 | MIT（见 `Cargo.toml` 的 `license` 字段） |

## 项目结构

```text
.
├── Cargo.toml              # workspace（edition 2024、MSRV 1.88、统一依赖与 lint）
├── Makefile.toml           # cargo make 任务
├── Dockerfile              # 多阶段构建（distroless 运行镜像）
├── rustfmt.toml            # 2 空格缩进、行宽 100
├── cspell.json             # 专有名词拼写白名单
├── ROADMAP.md              # 增量扩展路线图
├── crates/
│   ├── core/               # 领域模型与纯计算：题目结构、内容指纹、考试规则/抽题/计分、分类体系、
│   │                       #   术语表、波段/传播/天线等知识数据、ADIF/Cabrillo/竞赛记分、网格与呼号解析、
│   │                       #   摩尔斯电码、全站知识搜索索引、成就与题库摘要、专题自测/相关主题、CQ/ITU 分区、
│   │                       #   设备机型库、音频频谱/FFT、本地存储结构、能力注册表（registry）
│   ├── app/                # Leptos 前端
│   │   ├── index.html      # Trunk 入口（meta / manifest / 静态资源拷贝）
│   │   ├── Trunk.toml
│   │   ├── style/          # Tailwind 入口 CSS（主题变量、Geist 字体、表格边框等全局细节）
│   │   └── src/
│   │       ├── app/        # 路由表（main_content.rs）+ 全局布局（导航 / 页脚 / 搜索面板）
│   │       ├── pages/      # 全部页面：首页、练习、考试、日志、地图、实时数据与知识库专题
│   │       ├── components/ # 题目卡片、答题卡、题库选择器、导航、全局搜索面板、相关主题内链与专题自测、动态背景…
│   │       ├── ui/         # 基础组件（Dialog/Sheet/Select/Checkbox/Radio…）
│   │       ├── data.rs     # 题库配置与加载（带缓存）、术语表、外部 JSON 拉取（5s 超时）
│   │       ├── store.rs    # 收藏 / 分组 / 笔记等本地持久化（带跨标签页缓存同步）
│   │       ├── kv.rs       # 统一存储门面：小数据 localStorage、大数据 IndexedDB 双写
│   │       ├── idb.rs      # IndexedDB KV 封装（承载通联日志等大体积数据）
│   │       ├── i18n.rs     # 界面文案词典（zh 为 key，en / es 增量翻译）
│   │       └── …           # exam_history / achievements / bank_updates / shortcuts / speech / theme / photo / pwa / web_threads
│   ├── apt/                # NOAA APT 云图解码（音频 AM 解调 + 图像重建，纯 Rust DSP，无外部依赖）
│   ├── apt-worker/         # APT 解码 Web Worker（后台线程解调，编译为 worker.js）
│   ├── sstv/               # SSTV 慢扫描电视解码（VIS 头识别 + 行同步 + RGB/YC 采样）
│   ├── sstv-worker/        # SSTV 解码 Web Worker
│   ├── wspr/               # WSPR 解码（Type 1 消息编解码 + 卷积码 + Goertzel 解调）
│   ├── wspr-worker/        # WSPR 解码 Web Worker
│   ├── server/             # Axum 静态站点服务器 + /api 数据代理
│   └── tools/              # 构建/维护 CLI（数据集、解析、术语表、i18n、图标、sw.js、sitemap、PSK31/频谱合成样本）
│       └── templates/sw.js # Service Worker 模板
├── data/
│   ├── explanations.json   # 题目解析（key = 题目内容指纹，约 2000 条）
│   ├── glossary/           # 术语表（按一级分类拆分为 10 个 JSON，约 460 条）
│   └── knowledge-i18n/     # 知识库正文译文（按语言 / 模块拆分，缺失自动回退中文）
├── e2e/                    # Playwright 端到端测试（全站冒烟 + 无障碍 + 关键流程，含覆盖率统计脚本）
└── public/
    ├── questions/          # 题库 JSON、config.json、题目图片（构建产物，已提交）
    ├── apt-worker/         # 以下三个 Worker 目录由 `cargo make build-*-worker` 生成
    ├── sstv-worker/
    ├── wspr-worker/
    ├── fonts/              # Geist 字体（本地托管）
    ├── dxcc-entities.bin   # DXCC 实体边界（世界地图着色用）
    ├── manifest.json       # PWA manifest
    └── *.png / *.svg / favicon.ico
```

## 后端 API 与数据源

前端通过 `fetch` 调用以下同源接口，由 Axum 服务器代为请求上游数据（规避 CORS），并在服务端做内存缓存：

| 方法 | 路径 | 作用 | 上游数据源 | 服务端缓存 |
| --- | --- | --- | --- | --- |
| GET | `/healthz` | 健康检查（Docker HEALTHCHECK 使用） | — | — |
| GET | `/api/solar` | 太阳通量、A/K 指数、黑子数、各波段传播条件 | HamQSL | 300s |
| GET | `/api/alerts` | NOAA 空间天气警报（含 G/S/R 级别） | NOAA SWPC | 300s |
| GET | `/api/xray` | GOES X 射线通量（6 小时时序、最新通量与耀斑级别 A/B/C/M/X） | NOAA SWPC | 300s |
| GET | `/api/spots` | DX Cluster 实时热点（最多 50 条） | DXWatch | 60s |
| GET | `/api/most-wanted` | DXCC 最稀有实体榜（前 30） | Club Log | 6h |
| GET | `/api/passes?lat=&lon=&min_elev=` | 卫星过境预报（TLE + SGP4，未来 24 小时） | Celestrak | TLE 缓存 6h |
| GET | `/api/iss` | 国际空间站实时位置 | wheretheiss.at | 30s |
| GET | `/api/geocode?lat=&lon=` | 反向地理编码（国家/城市，中文） | BigDataCloud | 24h |
| GET | `/api/voacap?tx=&rx=&month=&ssn=&hour=` | 点对点 HF 传播预测（简化 VOACAP 模型，`hour` 为 UTC 时刻，省略则按最佳时段） | 本地计算 | 6h |
| GET | `/api/psk-reporter?callsign=` | 数字模式接收报告（谁收到了我） | PSK Reporter | 60s |
| GET | `/api/rbn?callsign=` | CW / RTTY 信标台接收报告 | Reverse Beacon Network | 30s |
| GET | `/api/callsign?callsign=` | 呼号查询（姓名 / 网格 / QTH，日志录入自动补全） | Callook / HamQTH | 7d |
| GET | `/api/sota?ref=` | SOTA 山峰详情（名称 / 海拔 / 积分 / 位置） | SOTA API | 30d |
| GET | `/api/pota?ref=` | POTA 公园详情（名称 / 实体 / 网格 / 位置） | POTA API | 30d |
| GET | `/api/repeaters?country=` | 中继台列表（频率 / 频差 / 亚音 / 城市） | RepeaterBook | 7d |
| GET | `/api/push/vapid-public-key` | 分发 Web Push 的 VAPID 公钥（前端订阅用） | 本地 | — |
| POST | `/api/push/subscribe` | 上报 Web Push 订阅（含每日提醒时间），持久化到 `PUSH_STORE` | 本地 | — |
| POST | `/api/push/unsubscribe` | 删除 Web Push 订阅 | 本地 | — |
| POST | `/api/push/review-count` | 同步「今日待复习数」（错题 + 卡片），供每日提醒推送附带数量 | 本地 | — |

> **关于 `/api/callsign` 的覆盖范围**：上游 Callook 只覆盖**美国 / 加拿大**呼号；其他国家和地区需要在服务端配置 `HAMQTH_USER` / `HAMQTH_PASS` 才会返回姓名 / QTH / 网格。两者都拿不到时，接口退回**内置 DXCC 前缀库**，只返回国家 / 地区（响应里的 `source` 为 `DXCC`），**不会**凭空生成网格或 QTH。状态码：`200` 有结果、`400` 呼号非法、`404` 前缀无法识别（多为拼写错误）、`503` 仅表示服务内部异常。
>
> 只启动前端（`cargo make dev`，仅 3000 端口）时 `/api/*` 无处转发，请求会失败；调试这些接口请用 `cargo make dev-full`（后端 3001 + 前端 3000 代理）。

其余 `/api/*` 之外的路径均由静态文件服务处理：命中文件直接返回，未命中且不带扩展名的路径回退到 `index.html`（SPA 路由）。

### 开放 API（`/api/v1/*`）

上表是**前端专用的同源代理**，接口随时可变。第三方请使用与之物理隔离、向后兼容的 `/api/v1/*`（文档页 `/developers`，OpenAPI 3.1：`/api/v1/openapi.json`）。首批仅开放纯计算与静态参考数据，无上游依赖：

| 路径 | 作用 |
| --- | --- |
| `GET /api/v1/dxcc` | DXCC 实体列表（`limit` 1–200 + `cursor` 游标分页） |
| `GET /api/v1/dxcc/lookup?callsign=` | 呼号 → DXCC 实体、CQ / ITU 分区与呼号结构 |
| `GET /api/v1/bands` | 频谱波段划分表 |
| `GET /api/v1/grid/to-latlon` · `from-latlon` · `distance` | Maidenhead 网格换算与大圆距离 / 方位角 |
| `GET /api/v1/propagation/muf` | 点对点 HF 传播预测（简化 VOACAP） |
| `GET /api/v1/status` · `openapi.json` | 服务状态 / OpenAPI 文档 |

约定：成功 `{ "data", "meta" }`、失败 `{ "error": { "code", "message" } }`；带 `ETag`（`If-None-Match` → 304）；`/api/v1/*` 开放 CORS（内部 `/api/*` 保持同源）；独立配额——匿名 30 次/分/IP，携带 `Authorization: Bearer <key>` 600 次/分，超限返回 429 + `Retry-After`。契约单一事实来源是 `crates/core/src/api_v1.rs`，文档页、`openapi.json` 与服务端路由测试共用，不会漂移。

> 依赖 `/api/*` 的页面：`/dashboard`、`/solar`、`/dx-spots`、`/most-wanted`、`/satellites`、`/grid-map`、`/portable-map`、`/log`、`/repeater`、`/portable`、`/psk-reporter`、`/rbn` 与首页传播卡片；纯静态托管（无后端）时这些页面降级为「数据暂不可用」。其余专题页与题库页均为本地静态数据或纯前端计算（如 `/tools`、`/grayline`、`/morse`、`/sdr-map`、`/events`），无网络也完全可用。

## 快速开始

### 环境要求

- Rust ≥ 1.88（推荐最新 stable，见 `.tool-versions`）
- `wasm32-unknown-unknown` 目标、[Trunk](https://trunkrs.dev)、[cargo-make](https://github.com/sagiegurari/cargo-make)

```bash
cargo install cargo-make
cargo make setup          # 安装 wasm32 目标、Trunk 与 wasm-bindgen CLI
```

> Tailwind CSS、wasm-opt 会在首次构建时由 Trunk 自动下载到本机缓存，无需 Node.js。
> Trunk 自带的 wasm-bindgen 不对外暴露，APT 解码 Worker 需要 PATH 上的 `wasm-bindgen` CLI，
> 版本由 `cargo make setup` 按 `Cargo.lock` 自动固定。

### 本地开发

```bash
cargo make dev            # 仅前端：http://127.0.0.1:3000 ，修改代码自动重新编译并刷新
cargo make dev-full       # 后端 API（3001）+ 前端（3000，代理 /api 到后端）
```

只调样式与题库功能时 `cargo make dev` 即可；需要调试实时数据页（仪表盘、太阳活动、DX 热点、卫星、网格地图）时用 `cargo make dev-full`。

开发模式下不注册 Service Worker，并会自动注销同源下残留的旧 SW，避免缓存和“发现新版本”弹窗干扰调试。开发产物输出到 `target/dev-dist/`，不会覆盖 release 的 `dist/`。

### 生产构建与运行

```bash
cargo make build          # 图标 + 前端（release）+ sw.js/sitemap + 服务器
cargo make serve          # 用 release 服务器托管 dist/：http://127.0.0.1:8080
```

构建产物位于 `dist/`；`dist/` 为纯静态文件，服务器仅额外提供 `/api/*` 代理与 `healthz`。若部署到纯静态托管（Nginx、CDN、GitHub Pages 等），只需把未命中的路径回退到 `index.html`，并为 `sw.js`、`manifest.json`、`changelog.json`、`index.html` 设置 `Cache-Control: no-cache`；此时依赖 `/api/*` 的实时数据页会降级为“数据暂不可用”，其余功能不受影响。

## 使用示例

### 备考闭环

1. `cargo make dev` 打开 http://127.0.0.1:3000 ；
2. 「练习」按顺序或随机做题，答错的题自动进「错题本」；
3. 「模拟考试」按真实配额抽题计时，交卷后可逐题回看「考后复盘」与分类正确率；
4. 「学习进度」查看 A/B/C 覆盖率与薄弱分类，「备考计划」设定考试日期后得到每日任务；
5. 「成就墙」回顾已解锁里程碑，「学习报告」汇总学习成果并一键导出 PNG；
6. 「数据备份」导出 JSON，换设备或清缓存后一键恢复。

### 通联日志与奖状

1. 在「通联日志」逐条录入，或导入 ADIF / CSV（自动去重、读取 LoTW / eQSL 状态）；可查询呼号自动补全姓名与网格；
2. 桌面版 Chrome / Edge 可开启「电台 CAT 联动」，自动回填频率与模式；
3. 「通联统计」「奖状进度」「DXCC 世界地图」查看 DXCC / 波段 / QSL 进度；
4. 「QSL 标签打印」生成标签纸，「竞赛录入」导出 Cabrillo 3.0。

### 实时数据与传播

`cargo make dev-full` 同时启动后端代理，即可使用「实时仪表盘」「太阳活动」「DX 实时热点」「卫星过境」「传播预测」「PSK Reporter / RBN」等页面。

### 浏览器内解码（音频不出本机）

「APT 云图解码」「SSTV 解码器」「WSPR 解码器」「PSK31 解码」都把音频交给 Web Worker 中的纯 Rust DSP 处理，不上传服务器。可先生成合成样本试跑：

```bash
cargo make apt-sample    && cargo make apt-decode     # 或 apt-test 一步到位
cargo make sstv-sample   && cargo make sstv-decode    # 或 sstv-test
cargo make wspr-sample   && cargo make wspr-decode    # 或 wspr-test
cargo make psk31-sample                               # PSK31 样本
cargo make spectrum-sample                            # 频谱 / 瀑布图样本（供 /sdr-waterfall）
```

## cargo make 任务一览

| 任务 | 说明 |
| --- | --- |
| **环境** | — |
| `cargo make setup` | 安装 wasm32 目标、Trunk 与 wasm-bindgen CLI（版本按 `Cargo.lock` 固定） |
| **开发** | — |
| `cargo make dev` | 前端开发服务器（默认任务，http://127.0.0.1:3000） |
| `cargo make dev-full` | 后端 API（3001）+ 前端开发服务器（3000，代理 `/api`） |
| **构建** | — |
| `cargo make build-worker` / `build-sstv-worker` / `build-wspr-worker` | 构建三个解码 Web Worker（wasm-bindgen `--target no-modules`） |
| `cargo make build-web` | 构建前端（release）并执行 `postbuild`（sw.js / sitemap / 预压缩） |
| `cargo make build-server` | 构建静态站点服务器（release） |
| `cargo make build` | 图标 + 前端 + 服务器 |
| `cargo make build-full` | 重新拉取题库数据后完整构建 |
| `cargo make serve` | release 服务器托管 `dist/`（http://127.0.0.1:8080） |
| **数据与资源** | — |
| `cargo make dataset` | 从 CSV 构建题库 JSON 与图片（`DATASET_DIR` / `DATASET_REMOTE` 可覆盖数据源） |
| `cargo make dxcc` | 从 country-files.com 拉取 cty.csv，重新生成 `crates/core/data/dxcc.txt` |
| `cargo make dxcc-map` | 从 Natural Earth 国界 GeoJSON 生成 DXCC 实体边界 `public/dxcc-entities.bin`（i16 定点 + delta 二进制编码） |
| `cargo make icons` | 由 `public/pwa-icon.svg` 生成 PWA / Apple Touch 图标 |
| **解析与术语** | — |
| `cargo make explanations-missing` | 统计缺失解析并导出待填模板（`BANK` / `LIMIT` 可选） |
| `BATCH=… cargo make explanations-add` | 按题目 ID 合并解析 |
| `cargo make explanations-enhance` | 用术语表为解析注入术语解释（幂等） |
| `cargo make glossary-check` | 校验术语表（分类 / 参见 / 重复 / 括号）并输出统计 |
| `cargo make explanations-apply` | 把解析写入题库 JSON（离线，无需重建数据集） |
| `BATCH=… cargo make explanations` | 合并 → 增强 → 写入，一步完成 |
| **国际化** | — |
| `cargo make i18n-check` | 校验界面文案词典（重复 key、占位符数量）并统计覆盖率 |
| **解码器样本** | — |
| `cargo make apt-sample` / `apt-decode` / `apt-test` | 生成 / 命令行解码 / 端到端自检 APT 样本 WAV |
| `cargo make sstv-sample` / `sstv-decode` / `sstv-test` | 同上，SSTV |
| `cargo make wspr-sample` / `wspr-decode` / `wspr-test` | 同上，WSPR |
| `cargo make psk31-sample` | 生成 PSK31 测试样本 WAV（供手动测试 `/psk-decode`） |
| `cargo make spectrum-sample` / `spectrum-analyze` / `spectrum-test` | 生成 / 命令行分析 / 端到端自检合成频谱样本（供 `/sdr-waterfall`） |
| **质量** | — |
| `cargo make fmt` / `fmt-check` | 代码格式化 / 格式检查 |
| `cargo make clippy` | Clippy（原生 crate + wasm 前端，`-D warnings`） |
| `cargo make test` | 单元测试 |
| `cargo make check` | 格式检查 + clippy + 单元测试 + 文案词典校验（不构建前端） |
| `cargo make e2e` | 构建 release 站点并运行 Playwright 端到端测试（见下文「端到端测试」） |
| `cargo make e2e-coverage` | 统计 e2e 覆盖率（路由冒烟 / 交互 / 关键流程），报告写入 `e2e/test-results/coverage.{md,json}` |
| `cargo make ci` | 等价于 `check` + 前端构建 |
| **部署与清理** | — |
| `cargo make docker-build` / `docker-run` | 构建 / 运行 Docker 镜像 |
| `cargo make clean` | 清理构建产物（`cargo clean` + `dist` / `tmp`） |

> 完整任务列表可用 `cargo make --list-all-steps` 查看；也可直接调用底层 CLI，如 `cargo run -p ham-web-tools -- <子命令> --help`。

## Docker 部署

```bash
docker build -t ham-web .
docker run -d --name ham-web -p 3000:3000 \
  -v ham-web-data:/app/data \
  ham-web
# 或
cargo make docker-build && cargo make docker-run
```

构建参数：

| 参数 | 默认值 | 说明 |
| --- | --- | --- |
| `SITE_URL` | `https://ham.onlyxp.me` | 写入 Open Graph 与 `sitemap.xml` 的站点地址 |
| `REBUILD_DATASET` | `0` | 设为 `1` 时构建阶段从远程 CSV 重新生成题库 |
| `TRUNK_VERSION` | `0.21.14` | Trunk 版本 |
| `WASM_BINDGEN_VERSION` | `0.2.129` | wasm-bindgen CLI 版本（构建 APT 解码 Worker），需与 `Cargo.lock` 中的 wasm-bindgen crate 一致 |

```bash
docker build --build-arg SITE_URL=https://exam.example.com -t ham-web .
```

运行时环境变量：`HOST`（默认 `0.0.0.0`）、`PORT`（默认 `3000`）、`DIST_DIR`（默认 `/app/dist`）、`PUSH_STORE`（Web Push 持久化文件，默认 `/app/data/push-subscriptions.json`）、`VAPID_PRIVATE_KEY` / `VAPID_SUBJECT`（可选，见下文）、`HAMQTH_USER` / `HAMQTH_PASS`（可选，呼号查询的国际数据源，不设则仅覆盖美加 + 内置 DXCC 回退）、`RUST_LOG`。

上游与请求相关的调优变量（均有合理默认值，一般无需设置）：

| 变量 | 默认值 | 说明 |
| --- | --- | --- |
| `UPSTREAM_CONNECT_TIMEOUT_SECS` | `5` | 上游建连（含 DNS 与 TLS 握手）超时；防止上游挂起占满阻塞线程池 |
| `UPSTREAM_TIMEOUT_SECS` | `15` | 上游请求端到端超时（从 DNS 到读完响应体） |
| `REQUEST_TIMEOUT_SECS` | `60` | 单个请求的兜底超时，超时返回 `504` |
| `RATE_LIMIT_PER_MIN` | `120` | `/api/*` 每 IP 每分钟限流阈值 |
| `API_V1_RATE_LIMIT_PER_MIN` | `30` | 开放 API `/api/v1/*` 匿名访问每 IP 每分钟配额 |
| `API_KEYS` | 空 | 开放 API 的 key（逗号分隔）；请求头 `Authorization: Bearer <key>` 命中后配额提升到 600 次/分，仅区分配额档位 |
镜像基于 `gcr.io/distroless/cc-debian12:nonroot`，以非 root 用户运行，内置健康检查（`/healthz`）。`/app/data` 目录已内置为可写，建议挂载 volume 持久化推送订阅与 VAPID 密钥。

## Web Push 后台推送

订阅后即使页面关闭，也能在「备考计划 → 每日提醒时间」到点时收到浏览器系统通知（前提是浏览器允许通知、站点为 HTTPS）。完整链路：

1. 前端在通知中心（`/notifications`）点击「订阅后台推送」，用后端分发的 VAPID 公钥向浏览器 `PushManager` 订阅；
2. 订阅信息（含每日提醒时间换算出的 UTC 分钟数）`POST /api/push/subscribe` 上报后端；
3. 后端每分钟检查一次，到点用 VAPID 私钥签发 JWT、按 RFC 8291（`aes128gcm`）加密消息，推送到订阅 endpoint；`410/404` 时自动清理失效订阅；
4. 前端页面打开 / 复习完成时通过 `POST /api/push/review-count` 静默同步「今日待复习数」（错题 + 卡片），推送时一并带上具体数量。

**订阅接口鉴权**：`POST /api/push/subscribe` 与 `/unsubscribe` **默认匿名可写**，无需任何配置 —— 订阅者本就是浏览器里的匿名访客，而前端是共享的静态资源，无从持有服务端令牌，要求鉴权会让功能对所有人失效。

匿名写入的防护依赖：`/api/*` 每 IP 限流、`valid_endpoint` 的推送服务域名白名单、`MAX_SUBSCRIPTIONS` 总数上限，以及不提供订阅列表读取接口（覆盖 / 删除他人订阅需事先拿到对方那条不可枚举的 endpoint）。

设置 `PUSH_API_TOKEN` 后这两个接口会校验 `Authorization: Bearer <token>`，但**官方前端会因此无法订阅**，仅适用于自建前端注入令牌或脚本管理的场景；设置后启动日志会给出相应提示。若只是想收紧来源，更推荐在反向代理层限制这两个路径。

**VAPID 密钥**按以下优先级确定，无需手工生成：

1. 环境变量 `VAPID_PRIVATE_KEY`（base64url，32 字节 P-256 标量）—— 适合多副本 / 自托管固定密钥；
2. `PUSH_STORE` 文件中已持久化的私钥；
3. 都没有时，**首次启动自动生成**，并随订阅一起写入 `PUSH_STORE` 文件（重启复用，公钥稳定）。

```bash
# 可选：固定私钥与 subject（不设则自动生成）
docker run -d --name ham-web -p 3000:3000 \
  -v ham-web-data:/app/data \
  -e VAPID_PRIVATE_KEY='<base64url 32字节>' \
  -e VAPID_SUBJECT='mailto:you@example.com' \
  ham-web
```

> ⚠️ 浏览器只在安全上下文（HTTPS 或 `localhost`）开放 `PushManager`。本地用 `http://127.0.0.1` 可调试，线上必须走 HTTPS。纯静态托管（无后端）时该功能不可用，前端会自动降级为「未配置推送服务」。

## 数据集构建

题库 JSON 与图片已提交在 `public/questions/`，日常开发无需重新构建。需要更新题库时：

```bash
cargo make dataset                                   # 从默认远程仓库拉取
DATASET_DIR=/path/to/csv-repo cargo make dataset     # 优先使用本地目录
DATASET_REMOTE=https://raw.githubusercontent.com/<you>/<repo>/main cargo make dataset
```

默认数据源为 [`xiedada05/crac-amateur-radio-exam-questions-2025-csv`](https://github.com/xiedada05/crac-amateur-radio-exam-questions-2025-csv)。数据源需包含：`class_a.csv`、`class_b.csv`、`class_c.csv`、`full.csv`、`images.csv` 与 `images_2/{题号}.jpg`。CSV 列：`J`（题号）、`P`（分类码）、`Q`（题干）、`T`（答案）、`A`–`D`（选项），可选解析列（`Explanation` / `解析` / `analysis`）。

输出：

- `public/questions/{A,B,C,full}.json`：题目列表，ID 形如 `A-1`（按题库内顺序编号）
- `public/questions/images/N.jpg`：题目附图

解析来源优先级：`data/explanations.json`（按内容指纹匹配） > CSV 解析列。

## 题目解析维护流程

### 核心概念：内容指纹

每道题的解析都以**内容指纹**为 key 存储在 `data/explanations.json`：

```text
指纹 = 规整后的题干 || A:选项A|B:选项B|… || 排序后的答案
```

例如：`我国专门针对无线电管理的行政法规及其制定机构是：||A:《中华人民共和国无线电管理条例》|B:…|D:工业和信息化部||AC`

- 与题号、题目顺序、所属题库**无关**：题库重排、增删题目不会导致解析错位；
- A/B/C 中内容相同的题目**共享同一条解析**，只需写一次；
- 题干、选项或答案发生**任何变化**都会产生新指纹 —— 官方修订过的题目会自动显示为「缺失解析」，避免旧解析误用。

> ⚠️ 指纹算法实现于 `crates/core/src/fingerprint.rs`，与历史数据逐字节兼容，请勿修改。

### 维护流程总览

```text
 ┌──────────────────────────┐
 │ 1. 统计缺失               │  cargo make explanations-missing
 │    → tmp/batch.json      │  （{ "B-12": "" , … } 待填模板）
 │    → tmp/context.json    │  （题干 / 选项 / 答案，供撰写参考）
 └────────────┬─────────────┘
              ▼
 ┌──────────────────────────┐
 │ 2. 撰写解析               │  编辑 tmp/batch.json，填入解析文本
 └────────────┬─────────────┘
              ▼
 ┌──────────────────────────┐
 │ 3. 合并 + 增强 + 写入      │  BATCH=tmp/batch.json cargo make explanations
 └────────────┬─────────────┘
              ▼
 ┌──────────────────────────┐
 │ 4. 本地预览并提交          │  cargo make dev → git commit data/ public/questions/
 └──────────────────────────┘
```

> 所有参数（`BANK`、`LIMIT`、`BATCH`）均通过**环境变量**传入：`VAR=值 cargo make 任务` 或 `cargo make -e VAR=值 任务`。

### 第 1 步：统计缺失解析

```bash
cargo make explanations-missing              # 全部题库
BANK=C LIMIT=50 cargo make explanations-missing   # 仅 C 类，最多导出 50 题
```

输出每个题库的解析覆盖情况，并生成：

- `tmp/batch.json`：待填模板（已按指纹去重，跨题库重合的题只出现一次）
  ```json
  { "B-12": "", "C-305": "" }
  ```
- `tmp/context.json`：题目上下文，便于人工或借助 AI 批量撰写
  ```json
  [{ "id": "B-12", "J": "LK0123", "P": "3.3.2", "type": "单选",
     "question": "…", "options": ["A. …", "B. …"], "answer": "B" }]
  ```

也可直接调用：`cargo run -p ham-web-tools -- missing-explanations --help`。

### 第 2 步：撰写解析

在 `tmp/batch.json` 中为每个 ID 填写解析。建议遵循现有风格：

1. **先给结论**：点明正确选项及依据，如「故 A、C 正确」；
2. **逐项排除**：简述错误选项错在哪里；
3. **引用依据**：法规题写明条款（如「《业余无线电台管理办法》第二十条」），技术题给出公式或原理；
4. **控制篇幅**：通常 1–4 句，面向初学者，避免堆砌术语；
5. **专业术语不必手动解释**：第 3 步会用术语表自动注入通俗解释；
6. 留空（`""`）的条目会被跳过，可分多批完成。

> 已有解析需要修改时，同样把对应题目 ID 写进 batch 文件即可覆盖；ID 可在「题库分类浏览」页每道题左上角看到。

### 第 3 步：合并、增强、写入

```bash
BATCH=tmp/batch.json cargo make explanations
# 或：cargo make -e BATCH=tmp/batch.json explanations
```

等价于依次执行：

| 步骤 | 命令 | 作用 |
| --- | --- | --- |
| 合并 | `BATCH=… cargo make explanations-add` | 按 ID 找到题目、计算指纹，写入 `data/explanations.json`（未知 ID 会告警） |
| 增强 | `cargo make explanations-enhance` | 用 `data/glossary/` 中 `inject: true` 的词条，为每条解析中**首次出现**的术语追加 `（通俗解释）`，幂等可重复执行 |
| 写入 | `cargo make explanations-apply` | 把解析写入 `public/questions/*.json`（离线完成，无需重新拉取 CSV） |

术语增强规则：长术语优先（「对流层散射」先于「散射」）；术语后已有括号、或处于原文括号内时跳过；每个术语每条解析只注入一次；「发射频率」中的「射频」不视为术语。

### 第 4 步：预览与提交

```bash
cargo make dev    # 在练习模式 / 分类浏览页检查解析显示
git add data/ public/questions/
git commit -m "docs(explanations): 补充 C 类解析 50 条"
```

### 维护术语表

术语表按一级分类拆分为 `data/glossary/` 目录下的多个文件（`law.json`、`frequency.json`、`operation.json`、`slang.json`、`modulation.json`、`equipment.json`、`antenna.json`、`propagation.json`、`basics.json`、`safety.json`），前端编译期嵌入（离线可用），`check-glossary` 会读取并合并目录下全部文件后校验。每个文件为 JSON 对象，key 为术语（中文名称，或 `QRM`、`73` 这类缩写本身），词条顺序即页面同分类内的展示顺序：

```json
{
  "驻波比": {
    "abbr": "SWR",
    "en": "Standing Wave Ratio",
    "category": "天线",
    "desc": "衡量天线和连接线配合得好不好的指标；配合越好能量浪费越少，数值越接近 1:1 越好",
    "aliases": ["电压驻波比", "VSWR"],
    "inject": true
  },
  "PEP": {
    "en": "Peak Envelope Power",
    "category": "设备",
    "desc": "峰包功率，即发射信号在最高点那瞬间的功率",
    "see": "峰包功率",
    "inject": true
  }
}
```

| 字段 | 必填 | 说明 |
| --- | --- | --- |
| `desc` | 是 | 通俗解释，一句话、面向初学者；**不要包含全角括号**（与注入格式冲突） |
| `category` | 建议 | 一级分类 key：`法规` `频率` `操作` `用语` `调制` `设备` `天线` `传播` `基础` `安全`，缺省归入「其他」 |
| `abbr` | 否 | 英文缩写，如 `SWR`；术语本身是 ASCII 时自动视为缩写，无需重复填写 |
| `en` | 否 | 英文全称 |
| `aliases` | 否 | 别名 / 同义词，参与搜索并显示为「又称」 |
| `see` | 否 | 参见的其他术语（必须是已存在的 key），页面上可点击跳转 |
| `inject` | 否 | 默认 `false`；为 `true` 时参与解析术语注入，此时解释中不能含任何括号 |
| `common` | 否 | 默认 `true`；`false` 表示「全量补充」的冷门词条。简语页（`/q-code`）据此区分「常用 / 全部」两个视图 |

旧写法 `"术语": "解释"` 仍然兼容，等价于 `{ "desc": "解释", "inject": true }`。

> **关于 `inject`**：注入是按子串匹配的，短缩写（如 `AM`、`CW`）或常见词会在解析中被大量误注入，因此新词条默认不参与注入。只有确实需要在解析中自动附加解释的中文专业术语才建议设为 `true`，并在执行后抽查效果。

修改后：

```bash
cargo make glossary-check                                    # 校验：分类是否存在、参见目标、重复词条、括号等
cargo make explanations-enhance && cargo make explanations-apply   # 仅当新增/修改了 inject=true 的词条时需要
cargo make dev                                               # 在 /glossary 预览
```

术语表页面功能：按分类筛选、搜索（术语 / 缩写 / 英文 / 别名 / 解释，精确匹配优先）、「只看英文缩写」、参见跳转，并统计该术语在 A/B/C 各题库中出现的题数（与分类浏览页的关键词搜索一致），点击即跳转到 `/browse?bank=…&q=…`。长度小于 3 的纯 ASCII 术语（如 `K`、`73`）误匹配过多，不做统计。也支持 `/glossary?q=驻波比` 直接定位。全站搜索面板同样会检索术语表与简语条目。

### 题库更新后的解析迁移

官方更新题库后：

```bash
cargo make dataset                 # 重新生成题库（内容未变的题目自动沿用解析）
cargo make explanations-missing    # 查看因题目修订/新增而缺失的解析
```

旧解析保留在 `explanations.json` 中不会自动删除，不影响使用。

## 新增题库版本

1. 把新版本题库 JSON 放到 `public/questions/`（当前版本直接覆盖 `A/B/C.json`）；
2. 编辑 `public/questions/config.json`，在 `versions` 中添加版本并更新顶层 `version`（前端检测到版本号变化会清空缓存）：
   ```json
   {
     "id": "2026-04", "name": "2026年4月版本", "description": "…", "isLatest": true,
     "banks": { "A": { "path": "/questions/A.json", "description": "A类题库" }, "B": { … }, "C": { … } },
     "updatedAt": "2026-04-01T00:00:00Z"
   }
   ```
3. 旧版本的 `isLatest` 改为 `false`。`/questions/<版本>/A.json` 形式的路径会被映射为 `/questions/A.json`。

## PWA 与离线

- `cargo make build-web` 的最后一步 `postbuild` 会扫描 `dist/`，为所有静态资源计算 SHA-256 修订号并生成 `dist/sw.js`（模板：`crates/tools/templates/sw.js`）；
- 缓存策略与原版保持一致：
  - 题库 JSON（`/questions/*.json`）：**NetworkFirst**，离线回退缓存（最多 10 条，7 天）；
  - 题目图片（`/questions/images/*`）：**StaleWhileRevalidate**（最多 300 张，30 天）；
  - 页面导航：离线时回退到预缓存的 `index.html`；其余静态资源预缓存；
- 部署新版本后，页面可见时与此后每小时检查一次更新；发现新 Service Worker 时在右下角提示「发现新版本」并列出更新内容（读取 `/changelog.json`），点击「刷新以更新」即切换到新版。提示不遮挡页面，考试中可以先忽略；
- 刷新后首次打开会显示一次「已更新到新版本」及更新内容；首次访问的用户不会看到；
- 更新内容写在 `crates/core/src/changelog.rs` 的 `CHANGELOG` 中（新版本插到最前面，日期 + 条目），`postbuild` 会导出为 `dist/changelog.json`；
- `postbuild` 还会为每个题库计算内容哈希，写入 `dist/questions/config.json` 的 `banks.*.rev`；前端请求题库时带上 `?v=<rev>`，题库内容变化后一定会重新下载。前端记录上次看到的题目摘要，题库变化时提示「题库已更新：修改 N 题，新增 N 题…」；
- 应用图标由 `cargo make icons` 从 `public/pwa-icon.svg` 渲染（纯 Rust，resvg）。

## 本地存储与兼容性

题库相关的 key 与数据结构与旧版（Next.js）完全一致，升级后用户已有进度与偏好可继续使用：

| key | 内容 |
| --- | --- |
| `practice:{版本}:{题库}` | 练习进度（顺序模式） |
| `practice:lastMode` | 上次练习题序 |
| `practice:noResumePrompt:{版本}:{题库}` | 本题库不再提示恢复 |
| `exam:savedState:{版本}:{题库}` | 未完成的考试 |
| `exam:answerCardFilter:{题库}` / `exam:showExplanation:{题库}` | 考试偏好 |
| `ui:shortcutsHelpSeen:{practice,exam}` | 快捷键说明是否已展示 |
| `theme` | `light` / `dark` / `system` |
| `locale` | `zh` / `en` / `es`（界面语言） |

后续新增功能的 key（同样只存在本地）：

| key | 内容 |
| --- | --- |
| `bookmarks` | 收藏题目的 stable_id 集合 |
| `exam-history` | 模拟考试成绩历史（最多 50 次；薄弱项组卷带 `weak` 标记，不计入备考状态） |
| `logbook` | 通联日志（`/stats`、`/grid-map`、`/progress`、`/most-wanted` 共用） |
| `station-info` | 本台信息（呼号 / 操作员 / 网格 / 设备 / 天线，写入 ADIF 台站字段） |
| `daily-checkin` | 首页每日打卡与连续天数 |
| `countdowns` | 倒计时条目 |
| `morse-stats` | 摩尔斯解码练习统计 |
| `dxcc_wanted_done` | 已通联的 DXCC 稀有前缀集合 |
| `mistake-book` | 错题本（题目快照、错误次数、连续答对次数、难度系数、复习间隔、下次复习时间，最多 1000 题） |
| `study-stats` | 按分类累计的作答数与正确数（含 A/B/C 分题库统计与已做过题目集合） |
| `mistake-book:seeded` | 是否已从旧版练习 / 考试存档迁移错题 |
| `study-stats:seen-seeded` | 是否已从旧版存档补齐「已做过」题目集合 |
| `morse-koch` | Koch 法抄收训练级别、速度与逐字符统计 |
| `dx-alerts` | DX 热点通知设置（新 DXCC / 新波段 / 关注呼号） |
| `learning-plan` | 备考计划（考试日期与类别） |
| `study-daily` | 每天的作答量（保留 60 天） |
| `morse-runner` | CW 呼号抄收设置与统计（最高分、累计通联、常错字符） |
| `contest-session` | 竞赛录入当前场次（竞赛、开始时间、本方交换信息、频率 / 模式 / 功率） |
| `sat-watch` | 卫星过境设置（位置、最低仰角、收藏卫星、提醒开关与提前量） |
| `sat-watch-notified` | 已提醒过的过境，避免重复通知 |
| `grid-map-filters` | 日志网格地图筛选条件（含是否显示通联路径） |
| `listen-settings` | 听题模式设置（题源、语速、思考时间、是否读解析、各题源听到的位置） |
| `cat-settings` | 电台 CAT 协议、波特率与 CI-V 地址 |
| `qsl-label-layout` | QSL 标签纸版式 |
| `card-review` | 知识卡片复习进度（每张卡的难度系数、间隔、下次复习时间，及今日已学新卡数） |
| `app:changelog-seen` | 已看过的更新说明日期 |
| `bank-digest:{A\|B\|C}` | 上次加载的题库修订号与题目摘要，用于提示题库变化 |
| `question-notes` | 题目私人笔记（题目 ID → 笔记文本） |
| `bookmark-groups` | 收藏分组 |
| `question-stats` | 题目级作答统计 |
| `achievements-seen` | 已提示过的成就解锁 id |
| `push-subscription` | Web Push 订阅的本地快照 |
| `morse-trainer` / `morse-send` | Koch 抄收与按键发报训练设置 |
| `sat-apt-notified` | 已提醒过的 APT 录制机会 |
| `exam:last-review` | 上次考后复盘的题目与作答 |

> 通联日志（`logbook`）可能超过 localStorage 5MB 配额：日志改由 `IndexedDB` 权威存储 + `localStorage` 静默快照双写（见 `crates/app/src/kv.rs` / `idb.rs`），跨标签页可见性由 `storage` 事件保证。数据备份仍以 localStorage 快照为准；日志极大（快照被静默丢弃）时建议同时用「通联日志 → 导出 ADIF」留底。

`/tools` 页底部提供「数据备份」：导出全部 `localStorage` 为 `ham-backup-YYYYMMDD.json`，可在其他设备或清缓存后一键导入恢复。

## 开发指南

### 快捷键

- `/`：打开全站搜索（不在输入框内时生效）
- `← / →`：上一题 / 下一题
- `1-9`：选择对应选项；多选题为切换，`Shift` / `Cmd` + 数字为仅选该项
- `Enter`：打开搜索（练习 · 顺序模式；焦点在按钮 / 链接上时为默认操作）
- `Esc`：关闭搜索面板 / 对话框

### 代码规范

- Rust 2024 edition，格式遵循根目录 `rustfmt.toml`（2 空格缩进，行宽 100）；
- workspace 统一开启 `unsafe_code = "forbid"` 与 Clippy `correctness/suspicious/style/complexity/perf`；
- 提交前执行 `cargo make ci`（仅前端样式改动时可先用 `cargo make check` 快速自检，两者均已包含格式、Clippy、单元测试与文案词典校验）；
- 不依赖浏览器的逻辑（日志、奖状、DX 通知匹配、Koch、错题本、考试判定、CAT 协议解析、CW 解码、标签排版等）放在 `ham-web-core`，并附单元测试；
- 修改界面文案时同步补 `crates/app/src/i18n.rs` 词典，`cargo make i18n-check` 会校验重复 key 与占位符数量；知识库正文译文放在 `data/knowledge-i18n/`；
- 改动 APT / SSTV / WSPR Worker 时，注意 `wasm-bindgen` CLI 版本必须与 `Cargo.lock` 一致（`cargo make setup` 会按锁文件安装，CI 亦有版本漂移校验）。

CI 位于 `.github/workflows/`：`check.yml` 在 push 到 `main` 与 PR 时运行 `cargo make check`，并另起一个 job 构建 release 站点后运行 Playwright 端到端测试（失败时上传报告）；`build-docker.yml` 在 push 到 `main` 或 `v*` tag 时构建并推送 Docker 镜像。

### 端到端测试

**框架**：[Playwright Test](https://playwright.dev)（`@playwright/test` + `@axe-core/playwright`），单 project（Chromium），`fullyParallel: true`，每个用例一个全新浏览器上下文（`localStorage` 为空）。

**断言方式**：以可访问性语义为主（`getByRole` / `getByLabel` / `getByPlaceholder` + `toBeVisible` / `toHaveText` / `toHaveAttribute`），配合 `expect.poll` 等待异步落盘；数据落点直接读 `localStorage`（`page.evaluate`）校验；文件导入 / 导出用 `setInputFiles` + `waitForEvent("download")`；原生 `alert` / `confirm` 用 `waitForEvent("dialog")` 或预先挂 `page.on("dialog")` 自动确认（否则渲染进程被阻塞）。

**覆盖范围**（`e2e/tests/`）：

| 模块 | 用例文件 |
| --- | --- |
| 练习流程 | `practice_flow.spec.ts`（题库切换、只看本类新增 / 只练没做过、题内搜索跳题、题序切换确认、收藏、专项链接）、`study.spec.ts`、`swipe.spec.ts` |
| 模拟考试 | `exam_flow.spec.ts`（A/B 类规则与倒计时、标记、答题卡筛选 / 跳转、交卷确认、交卷后对错、中断恢复、薄弱项与自定义组卷、多选题勾选）、`exam_review.spec.ts`（考后复盘） |
| 复习与题库 | `study_modes.spec.ts`（闪卡、每日挑战、打印版）、`browse.spec.ts`（分类浏览搜索 / 筛选 / 分页）、`glossary.spec.ts`（术语表）、`mistake_topics.spec.ts`（易错知识点）、`notes.spec.ts`、`cards.spec.ts`、`listen.spec.ts` |
| 日志与竞赛 | `log_form.spec.ts`（表单提交、必填校验、搜索筛选、编辑、清空二次确认、QSL 同步、本台信息）、`log.spec.ts`（ADIF 导入去重 / 导出）、`qsl_labels.spec.ts`、`contest_log.spec.ts` |
| 工具与实时 | `tools_backup.spec.ts`（计算器、数据备份导出 / 导入 / 合并 / 非法文件）、`photo_processor.spec.ts`、`apt_decoder.spec.ts`、`psk_decode.spec.ts`、`cw_decoder.spec.ts`、`cat.spec.ts`、`morse.spec.ts` |
| 计划与全局 | `planning.spec.ts`（倒计时边界、备考计划）、`progress` 相关、`navigation.spec.ts`（导航跳转、主题、语言）、`search.spec.ts`（全站搜索）、`notifications.spec.ts`（权限与推送降级） |
| 异常与降级 | `resilience.spec.ts`（题库加载失败、本地数据损坏、接口失败、存储写满）、`fatal.spec.ts`（wasm 兜底页）、`updates.spec.ts`、`new_features.spec.ts`、`a11y.spec.ts`、`knowledge_i18n.spec.ts` |

`smoke.spec.ts` 从路由表读取全部页面，在浅色 / 深色主题下逐一检查运行时错误、资源 404 与 axe（WCAG 2.1 AA）—— **新增页面会自动纳入**，无需改测试。`i18n_layout.spec.ts` 对全部路由做中 / 英 / 西三语布局回归（横向溢出、文本裁剪、越界、浮点未取整）。

```bash
cd e2e && npm ci && npx playwright install chromium   # 首次
cargo make e2e                                         # 构建 dist/ 并由 Playwright 启动 release 服务器（4173 端口）
cd e2e && E2E_BASE_URL=http://127.0.0.1:3000 npx playwright test   # 直接测正在运行的 trunk dev
cargo make e2e-coverage                                # 只统计覆盖率，不跑用例
```

- `E2E_CHANNEL=chrome` 改用本机已安装的 Chrome；`E2E_PORT` 修改 release 服务器端口；
- 每个测试使用全新浏览器上下文（`localStorage` 为空），fixture 默认把练习 / 考试的快捷键说明标记为已看过；
- 需要「已有数据」的场景用 `page.addInitScript` 直接写 `localStorage`，不依赖真实操作累积。

#### 覆盖率统计口径

`cargo make e2e-coverage`（`e2e/scripts/coverage.mjs`）输出三条互不替代的指标，报告写入 `e2e/test-results/coverage.{md,json}`：

| 指标 | 定义 | 用途 |
| --- | --- | --- |
| 路由冒烟覆盖率 | `smoke` / `i18n_layout` 从路由表遍历全部路由做渲染 + 无障碍体检，恒为 100% | 保证「新增页面不漏测」 |
| 交互覆盖率 | 至少有一条功能用例通过 `page.goto` 显式访问过的路由数 / 路由总数（不含点击链接间接到达，口径保守可复现） | 衡量有多少页面有专属功能用例 |
| 关键流程覆盖率 | `e2e/coverage-targets.json` 中列出的核心用户旅程，已有用例标题命中的比例 | 真正反映业务场景覆盖，CI 门禁看这个 |

关键旅程清单（`coverage-targets.json`）覆盖：核心用户路径（练习 → 错题 → 复习 → 考试 → 复盘）、异常场景（题库加载失败、本地数据损坏、接口失败、存储写满、wasm 兜底）、边界条件（空呼号提交、空标题倒计时、多选题取消勾选、超大文件、非法备份文件）、权限校验（通知被拒、Web Push 不支持）、页面跳转（导航下拉、空态引导链接、404）、表单提交与数据交互（日志录入 / 编辑 / 清空、QSL 同步、备份导入导出、ADIF / Cabrillo 导入导出）。

新增功能时：先补用例，再把旅程追加到 `coverage-targets.json`（`grep` 为用例标题子串，需唯一命中；脚本会校验没有歧义）。

#### 集成到 CI

`.github/workflows/check.yml` 的 `e2e` job 在构建 release 站点后执行 `npx playwright test`，随后运行覆盖率脚本：

1. 报告同时写入 job summary（PR 里直接可见趋势）与 `e2e-coverage` artifact（保留 30 天）；
2. 用仓库变量 `E2E_MIN_JOURNEY` / `E2E_MIN_ROUTE` 设阈值即可变成硬门禁（低于阈值时 job 失败）；当前默认不设阈值，只做趋势观测；
3. 失败时上传 `playwright-report`（含 trace，保留 7 天）。

本地提效：`cargo make check` 只做静态检查；改了 UI 再跑 `cargo make e2e`，只关心覆盖率变化则跑 `cargo make e2e-coverage`。

### 修改 UI 的注意事项

- 基础组件的 Tailwind 类名与原 shadcn/ui 完全一致，位于 `crates/app/src/ui/`；类名合并使用 `cn(&[...])`（语义同 `tailwind-merge`）；
- Tailwind 会扫描 `crates/app/src` 下的 Rust 源码，新类名需以**完整字面量**出现（不要字符串拼接类名片段）；
- 在 `leptos` 中向依赖 context 的子组件（如 `RadioGroupItem`、`SelectItem`）传递子元素时，需在父组件的 children 内构建，而不是提前 `collect_view()`；
- 知识库表格统一使用「卡片 + 内部网格线」的样式：卡片内的 `table.border-collapse` 会自动去掉与卡片边框重合的最外圈边框（见 `style/input.css`），因此单元格只需写 `border`，无需手动处理外边线。

### 动态背景（Web Threads）

全站背景层是 React Bits [`WebThreads`](https://reactbits.dev/backgrounds/web-threads) 的 Rust / WASM 复刻，与上游**同一套技术方案**：WebGL2 + 全屏三角形 + GLSL ES 3.00 片元着色器（逐像素叠加 10 条正弦丝线，`glow()` 幂次衰减生成柔光，浅色主题走 `uLightMode` 分支输出「墨线」）。顶点与片元着色器与上游**逐字一致**（SHA-256 相同），改动前请先比对上游。

| 关注点 | 实现 |
| --- | --- |
| 引擎 | `crates/app/src/web_threads.rs`：上下文创建、uniform 位置缓存、渲染循环、资源释放（只依赖 web-sys，不依赖 Leptos） |
| 组件 | `crates/app/src/components/web_threads.rs`：`WebThreadsBackground`（全站背景）+ `WebThreads`（低阶、可复用） |
| 主题适配 | `ThemeCtx::is_dark()` → `Memo` 生成预设；切主题只更新 uniform，**不重建 WebGL 上下文**。浅色画布底色运行时从 `--background` 解析（回退 `#FAFBFC`），与页面背景严丝合缝 |
| 背景层 | `style/input.css` 的 `.web-threads-layer`：`fixed inset-0` + `z-index: -10`（低于 `body::before` 环境光晕与全部内容）+ `pointer-events: none`，配合「顶部最浓、向下 `mask` 淡出」保住正文可读性 |
| 不干扰前景 | 鼠标扰动在 `window` 上监听（画布不接收指针事件），因此不抢占前景的点击 / 悬停；图层 `aria-hidden="true"`，打印时不渲染 |
| 省电 | 触屏设备（`(hover: hover)` 为假）用 `RenderMode::Static` 只渲染**一帧**当背景图，GPU 占用归零；桌面端帧率上限 30fps；DPR 上限 2、最长边 1920，紧凑设备（视口最小边 ≤ 820px）再降到 1.5 / 1280；声明 `powerPreference: "low-power"`；离屏与切后台暂停；卸载时 `WEBGL_lose_context` 释放上下文 |
| 无障碍 | 「减少动态效果」下同样只渲染一帧（与移动端共用同一条静态路径） |

实测口径（浏览器内 `drawArrays` 计数）：桌面 1440×900 @DPR2 → 缓冲区 1920×1206、30.0fps；手机 390×844 @DPR3 触屏 → 缓冲区 585×1266、初始化 3 帧后 5 秒内 0 次绘制。

接入与调参：根布局已挂载 `<WebThreadsBackground />`，局部复用可直接包裹 `<WebThreads config=… />`（`config` 传 `Signal` 可热更新，同样只改 uniform）。预设配色 / 亮度 / 不透明度 / 线程数在 `components/web_threads.rs` 的 `dark_preset()` 与 `light_preset()`；帧率与设备档位预算在 `web_threads.rs` 的 `MAX_FPS` / `COMPACT_*` 常量；背景层透明度与淡出范围在 `style/input.css`。

## 路由

路由表定义在 `crates/app/src/app/main_content.rs`（外层布局见同目录 `mod.rs`），页面按三大模块组织。导航菜单与路由的对应关系由 `crates/core/src/registry.rs` 的能力注册表统一维护：注册表是路径、标题、图标与分组的唯一事实来源，顶部导航、`sitemap.xml` 均由其派生，`crates/app/src/registry_check.rs` 的测试保证三者不会漂移；新增页面会自动纳入冒烟测试。

| 模块 | 路径 |
| --- | --- |
| 首页 | `/` |
| 考试中心 | `/practice` `/exam` `/daily-challenge` `/browse` `/flashcards` `/listen` `/cards` `/mistakes` `/mistake-topics` `/bookmarks` `/weekly` `/progress` `/study-calendar` `/exam-review` `/print` `/countdown` `/photo-processor` |
| 备考速查 | `/reference` `/cheat-sheet` `/confusables` `/formulas` `/prefixes` `/glossary` `/q-code` `/phonetic` `/rst` `/morse` `/cw-operating` `/license-classes` |
| 模式 · 传播 | `/analog-modes` `/atv` `/sstv` `/modes` `/dv-network` `/packet` `/rtty` `/ft8` `/sdr` `/gnuradio` `/aprs` `/frequencies` `/coordination` `/zone-map` `/propagation` `/beacons` `/special-prop` `/meteor-scatter` `/eme` `/muf` `/wspr` `/weather-sat` `/apt-decoder` `/aurora` |
| 天线 · 设备 | `/antennas` `/polarization` `/feedline` `/balun` `/antenna-diy` `/practical-antennas` `/antenna-installation` `/antenna-farm` `/antenna-tuning` `/antenna-analyzer` `/vna` `/antenna-modeling` `/antenna-array` `/nvis` `/electronics` `/filters` `/meters` `/power` `/power-supply` `/transceiver` `/receiver` `/gear` `/amplifier` `/bands` `/bandplan` `/microwave` `/mobile` |
| 通联 · 活动 | `/operating` `/sat-operation` `/contest` `/cabrillo` `/awards` `/iota` `/dx` `/dxpedition` `/most-wanted` `/qrp` `/eqsl` `/qsl-card` `/ardf` `/emcomm` `/winlink` `/events` `/portable` `/grid` `/callsign` `/repeater` `/repeater-build` `/logging-software` |
| 进阶 · 关于 | `/organizations` `/safety` `/grounding` `/rfi` `/noise` `/beginner` `/learning-path` `/learning-resources` `/swl` `/license` `/regulations` `/history` `/remote` `/open-source` `/community` `/diy-projects` `/developers` |
| 工具 | `/dashboard` `/tools` `/log` `/contest-log` `/contest-calendar` `/qsl-labels` `/qsl-designer` `/grid-map` `/portable-map` `/dx-spots` `/solar` `/satellites` `/sdr-map` `/sdr-waterfall` `/grayline` `/dxcc-map` `/stats` `/psk-reporter` `/rbn` `/psk-decode` `/sstv-decode` `/wspr-decode` `/callsign-copy` `/notifications` |
| 可直链访问（未在导航中） | `/achievements` `/report` |

带参数的页面：

| 路径 | 参数 |
| --- | --- |
| `/practice` `/exam` | `version`（题库版本）、`bank`（A\|B\|C）；`/practice` 另有 `multi=1`（只练多选，页面顶部有同名开关），`/exam` 另有 `mode=weak`（薄弱项组卷） |
| `/browse` | `bank`、`q`（关键词） |
| `/glossary` | `q`（关键词） |
| `/zone-map` | `q`（呼号 / 实体名，定位分区） |
| `/callsign` | `call`（呼号，打开即自动在线查询） |
| `/print` | `src`（`mistakes` \| `bookmarks`）、`bank`（A\|B\|C） |
| `/cards` | `deck`（`qcode` \| `abbrev` \| `phonetic` \| `morse` \| `glossary`） |
| `/contest-log` | `contest`（预选竞赛模板 ID，如 `CQ-WW-CW`） |

其余路径回退到 404 页。

## 常见问题

- **题库为空或 404？** 确认 `public/questions/` 下存在 `A/B/C.json` 与 `config.json`，必要时执行 `cargo make dataset`。
- **实时数据页显示“暂不可用”？** 这些页面依赖服务器 `/api/*` 代理（`cargo make dev-full` 或 `cargo make serve`）；纯静态托管时上游请求无处转发，属于预期降级。
- **首次构建很慢？** Trunk 会下载 Tailwind、wasm-bindgen、wasm-opt 并编译依赖，之后为增量构建。
- **wasm-opt 报 `bulk memory` 相关错误？** 已在 `index.html` 中通过 `data-wasm-opt-params` 启用新特性，请使用 `Trunk.toml` 中固定的 wasm-opt 版本。
- **更新部署后页面没有变化？** Service Worker 需要一次刷新才能激活，页面右下角会出现「发现新版本」提示（每小时自动检查一次）；也可在浏览器 DevTools → Application → Service Workers 中手动更新。
- **练习模式的搜索按钮不见了？** 题目搜索仅在练习模式的顺序模式下可用；跨全站的知识检索请用 `/` 唤起的搜索面板。
- **朗读 / 摩尔斯试听没有声音？** 浏览器需要一次用户交互后才允许播放音频，请先点击页面；朗读依赖系统 TTS 语音包。
- **`cargo make dev-full` 和 `cargo make i18n-check` 能同时跑吗？** 不能：两者都要占用 `target/` 的构建锁，同时执行会互相等待，建议串行运行。
- **改了解码 Worker 却不生效？** 三个 Worker 的 wasm 由 `cargo make build-worker` / `build-sstv-worker` / `build-wspr-worker` 生成到 `public/*-worker/`；`cargo make dev` 与 `build-web` 会自动先构建，但单独改了 worker 代码后需重跑对应任务（首次还需 `cargo make setup` 提供版本匹配的 `wasm-bindgen`）。
- **端到端测试跑不起来？** 首次需在 `e2e/` 下执行 `npm ci && npx playwright install chromium`；`cargo make e2e` 会先构建 release 站点。
- **通联日志很大还能用吗？** 可以：日志由 `IndexedDB` 承载（突破 localStorage 5MB 配额），localStorage 仅保留静默快照。清浏览器数据会同时清除两者，重要日志请定期用「数据备份」导出。

## 贡献指南

欢迎提交 Bug 报告、功能建议、题库解析与术语补充、文档改进与代码 PR。

### 开发流程

1. Fork 并切出分支（如 `feat/…`、`fix/…`、`docs/…`）；
2. 本地执行 `cargo make setup`；改完后跑 `cargo make check`（或 `cargo make ci`，一并构建前端）与 `cargo make e2e`；
3. 提交 PR 并简述改动动机与验证方式，CI 会自动运行 `cargo make check` 与 Playwright 端到端测试。

提交信息沿用仓库现状的约定式风格，例如 `feat(beacons): 新增信标页`、`docs(explanations): 补充 C 类解析 50 条`。

### 各类贡献的入口

| 想做的事 | 从哪开始 |
| --- | --- |
| 修订 / 补充题目解析 | `data/explanations.json`，流程见「题目解析维护流程」 |
| 补充术语 | `data/glossary/*.json`，随后 `cargo make glossary-check` |
| 新增知识专题页 | `crates/core/src/<name>.rs`（数据）+ `crates/app/src/pages/<name>.rs`（页面）+ `crates/core/src/registry.rs` 注册一条 + `main_content.rs` 加路由（导航与站点地图自动派生） |
| 界面文案翻译 | `crates/app/src/i18n.rs`（界面）与 `data/knowledge-i18n/`（知识库正文） |
| 修 Bug / 加功能 | 不依赖浏览器的逻辑请在 `ham-web-core` 补单元测试；UI 改动注意 `crates/app/src/ui/` 的类名约定 |

> 新增页面时只改两处：**在 `crates/core/src/registry.rs` 的 `MODULES` 追加一条**（路径 / 标题 / 图标 / 分组），再在 `crates/app/src/app/main_content.rs` 加一行 `<Route>`（Leptos 需要具体组件类型，无法数据驱动）。导航菜单、`sitemap.xml` 都由注册表派生；`crates/app/src/registry_check.rs` 的测试会断言「注册表 ↔ 导航 ↔ 路由」三者一致，漏注册或写错路径会让 `cargo make check` 直接失败。页面同时自动纳入 `smoke.spec.ts` 的全站冒烟与无障碍检查。

### 报告问题

请附上复现步骤、浏览器与系统版本，以及控制台报错或截图；涉及题库数据的问题请注明类别与题号（题目左上角可复制 ID）。

## 许可

本项目以 **MIT** 许可发布（见 `Cargo.toml` 的 `license` 字段）。题库数据版权归原作者所有，使用前请遵守上游仓库的许可与署名要求。

## 致谢

- **题库数据**：[xiedada05/crac-amateur-radio-exam-questions-2025-csv](https://github.com/xiedada05/crac-amateur-radio-exam-questions-2025-csv)
- **相关项目**：[AlliotTech/ham-exam-web](https://github.com/AlliotTech/ham-exam-web)
- **开源项目**：Leptos、Axum、Trunk、Tailwind CSS、tw-animate-css、Lucide、Geist 字体、resvg、sgp4
- **数据来源**：HamQSL、NOAA SWPC、Celestrak、DXWatch、Club Log、wheretheiss.at、BigDataCloud
