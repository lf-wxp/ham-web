# 架构与项目结构

> README 只留一张技术栈表和顶层目录；逐目录说明、分层边界与 crate 依赖方向看这里。

## 技术栈

| 层 | 技术 |
| --- | --- |
| 语言 | Rust 2024 edition（MSRV 1.88） |
| 前端 | [Leptos 0.8](https://leptos.dev)（CSR）+ `leptos_router`，编译为 WebAssembly |
| 前端构建 | [Trunk](https://trunkrs.dev)（自动下载 Tailwind CSS v4 独立版、wasm-bindgen、wasm-opt） |
| 样式 | Tailwind CSS v4 + tw-animate-css，沿用原 shadcn/ui（new-york）设计令牌与类名 |
| 图标 | Lucide（内联 SVG，与原版路径一致） |
| 浏览器 API | Web Speech API（朗读）、Web Audio API（摩尔斯 / 信号音 / 实时频谱）、Canvas（照片处理、瀑布图）、WebGL2（天线求解器的三维方向图，不支持时回退 SVG）、Notification（倒计时提醒）、IndexedDB（大体积日志存储） |
| 服务器 | Axum 0.8 + tower-http（SPA 回退、缓存头、gzip/brotli），并提供 `/api/*` 外部数据代理（`ureq` + 内存缓存 + `sgp4` 过境计算） |
| 数字信号解码 | `ham-web-apt` / `ham-web-sstv` / `ham-web-wspr` / `ham-web-ft8` 等十一个解码 crate（纯 Rust DSP，编译为 Worker，音频不上传服务器；另有一个 AX.25 软 TNC Worker，解码核心在 `ham-web-core`） |
| 工具链 | `ham-web-tools`（clap、ureq、resvg、sha2、brotli、walkdir） |
| 任务编排 | [cargo-make](https://github.com/sagiegurari/cargo-make) |
| PWA | 构建时由 Rust 生成 Service Worker（预缓存 + 运行时缓存策略） |
| 许可 | MIT（见仓库根目录 [`LICENSE`](../LICENSE)）；DominoEX / LRPT / NAVTEX 解码器及其 Worker 例外，为 GPL-3.0（见 [`licensing.md`](licensing.md)） |

## 项目结构

```text
.
├── AGENTS.md               # 动代码前的入口：文档索引 + 硬规则
├── ROADMAP.md              # 路线图：未完成项、刻意不做的边界与工程门禁
├── Cargo.toml              # workspace（edition 2024、MSRV 1.88、统一依赖与 lint）
├── Makefile.toml           # cargo make 任务
├── Dockerfile              # 多阶段构建（distroless 运行镜像）
├── rustfmt.toml            # 2 空格缩进、行宽 100
├── cspell.json             # 专有名词拼写白名单
├── perf-budget.json        # 性能预算（size-check 读它）
├── LICENSE / LICENSES/     # MIT 全文与第三方 / GPL 许可全文（随发布物分发）
├── docs/                   # 专题规范（UI / i18n / 数据源 / 解码器协议 / 代码质量 / README 审核记录）与 images/ 截图
├── scripts/                # 构建脚本：解码 Worker 清单、Docker 镜像冒烟
├── .github/workflows/      # check.yml（检查 + e2e + 镜像冒烟）、build-docker.yml、data-sync.yml
├── crates/
│   ├── core/               # 领域模型与纯计算：题目结构、内容指纹、考试规则/抽题/计分、分类体系、
│   │                       #   术语表、波段/传播/天线等知识数据、ADIF/Cabrillo/竞赛记分、网格与呼号解析、
│   │                       #   摩尔斯电码、全站知识搜索索引、成就与题库摘要、专题自测/相关主题、CQ/ITU 分区、
│   │                       #   设备机型库、LoRa 链路与 NTS / ICS-213 报文格式、音频频谱/FFT、本地存储结构、
│   │                       #   AX.25 / AFSK1200 编解码、能力注册表（registry）
│   ├── nec/                # NEC 天线建模计算核心（矩量法 / Yagi 优化 / 方向图 / 网格，纯 std 零依赖）
│   ├── dsp/                # 各解码器共用的 DSP 原语（Goertzel 等）
│   ├── wav/                # WAV（RIFF）解析：PCM / IEEE float、8–32 位、单双声道
│   ├── app/                # Leptos 前端
│   │   ├── index.html      # Trunk 入口（meta / manifest / 静态资源拷贝）
│   │   ├── Trunk.toml
│   │   ├── style/          # Tailwind 入口 CSS + 像素风样式（token / 配色方案 / 字体 / 动效）
│   │   └── src/
│   │       ├── app/        # 路由表（main_content.rs）+ 全局布局（导航 / 页脚 / 搜索面板）
│   │       ├── pages/      # 全部页面：首页、练习、考试、日志、地图、实时数据与知识库专题
│   │       ├── components/ # 题目卡片、答题卡、题库选择器、导航、全局搜索面板、相关主题内链与专题自测…
│   │       ├── ui/         # 基础组件（Dialog/Sheet/Select/Checkbox/Radio…），清单见 src/ui/README.md
│   │       ├── data.rs     # 题库配置与加载（带缓存）、术语表、外部 JSON 拉取（5s 超时）
│   │       ├── store.rs    # 收藏 / 分组 / 笔记等本地持久化（带跨标签页缓存同步）
│   │       ├── kv.rs       # 统一存储门面：小数据 localStorage、大数据 IndexedDB 双写
│   │       ├── idb.rs      # IndexedDB KV 封装（承载通联日志等大体积数据）
│   │       ├── i18n/       # 界面文案运行时：mod.rs（t/tf/Locale）+ catalog.rs + pack.rs
│   │       └── …           # exam_history / achievements / bank_updates / shortcuts / speech / theme / photo / pwa / rpg
│   ├── apt/ · sstv/ · wspr/ · ft8/ · rtty/ · olivia/ · dominoex/ · navtex/ · lrpt/ · hell/ · wefax/
│   │                       # 十一个解码核心（纯 Rust DSP + 单测 + 对拍；dominoex / navtex / lrpt 含 GPL 转写）
│   ├── *-worker/           # 十二个 Worker 胶水（wasm-bindgen --target no-modules）；afsk1200 的解码实现在 core
│   ├── server/             # Axum 静态站点服务器 + /api 代理 + /api/v1 开放接口
│   └── tools/              # 构建/维护 CLI（数据集、解析、术语表、i18n、图标、sw.js、sitemap、各项守卫）
│       └── templates/sw.js # Service Worker 模板
├── data/
│   ├── explanations.json   # 题目解析（key = 题目内容指纹，约 2000 条）
│   ├── glossary/           # 术语表（按一级分类拆分为 10 个 JSON，约 460 条）
│   ├── i18n/               # 界面文案译文（按语言 / 域拆分；zh 编进 wasm，其余运行时拉取）
│   └── knowledge-i18n/     # 知识库正文译文（缺失自动回退中文）
├── e2e/                    # Playwright 端到端测试（冒烟 + 无障碍 + 关键流程 + 覆盖率脚本）
└── public/
    ├── questions/          # 题库 JSON、config.json、题目图片（构建产物，已提交）
    ├── decode-worker.js    # 统一解码 Worker 入口（手写，入库；协议见 docs/decode-worker-protocol.md）
    ├── *-worker/           # 十二个 Worker 目录由 `cargo make build-*-worker` 生成（已被 .gitignore 忽略）
    ├── data/               # 运行时语言包与合并后的知识库译文（生成物，不入库）
    ├── fonts/              # 像素字体分片与 Geist（本地托管，许可见 fonts/LICENSES）
    ├── vendor/             # 自托管第三方资源（KaTeX）
    ├── dxcc-entities.bin   # DXCC 实体边界（世界地图着色用）
    ├── cn-provinces.bin    # 中国省级边界（分区号地图用）
    ├── manifest.json       # PWA manifest
    └── pwa-*.svg / *.png   # 图标设计源与生成物（见「PWA 与离线」）
```

