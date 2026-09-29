# 业余无线电执照考试模拟

[![Rust 2024](https://img.shields.io/badge/Rust-2024_edition-orange?logo=rust)](https://www.rust-lang.org)
[![Leptos 0.8](https://img.shields.io/badge/Leptos-0.8-ef3939)](https://leptos.dev)
[![PWA](https://img.shields.io/badge/PWA-offline-5a0fc8)](#pwa-与离线)

基于 2025 年最新题库的业余无线电执照考试模拟应用，支持 A/B/C 三类考试，提供模拟考试、练习、关键词搜索、题库分类浏览、报名照片处理等功能。
**全项目使用 Rust（2024 edition）编写**：前端为 Leptos（CSR → WebAssembly），静态服务器为 Axum，数据集构建与题库维护工具同样是 Rust 命令行程序。

在线 demo：[业余无线电考试](https://ham.onlyxp.me/)

---

## 目录

- [业余无线电执照考试模拟](#业余无线电执照考试模拟)
  - [目录](#目录)
  - [功能特色](#功能特色)
    - [核心功能](#核心功能)
    - [用户体验](#用户体验)
  - [技术栈](#技术栈)
  - [项目结构](#项目结构)
  - [快速开始](#快速开始)
    - [环境要求](#环境要求)
    - [本地开发](#本地开发)
    - [生产构建与运行](#生产构建与运行)
  - [cargo make 任务一览](#cargo-make-任务一览)
  - [Docker 部署](#docker-部署)
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
    - [修改 UI 的注意事项](#修改-ui-的注意事项)
    - [路由](#路由)
  - [常见问题](#常见问题)
  - [致谢](#致谢)

---

## 功能特色

### 核心功能
- **📝 模拟考试**：A/B/C 三类考试，按真实规则抽题（单选/多选配额），计时交卷与成绩统计，中途退出可恢复
- **🎯 练习模式**：顺序/随机练习、即时显示答案与解析、进度自动保存与恢复
- **🔍 智能搜索**：按题号或关键词搜索并跳转（顺序练习模式）
- **🗂️ 分类浏览**：按 10 大题目类型与官方分类码浏览，仅显示正确答案，附解析、知识点与参考依据
- **📖 术语表**：400+ 条业余无线电术语、英文缩写（SWR、CTCSS、QRM…）与通俗解释，按类型分类检索，并统计各题库中相关题目数、一键跳转查看
- **📡 频谱波段划分表**：从带号 -1 到 12 的波长/频率范围，业余业务与卫星业余业务频段、使用状态（专用/唯一主要/主要/次要）及脚注，以 HTML 表格呈现（移动端为卡片）
- **🧩 只看本类新增**：基于题目内容指纹识别 A/B/C 重合题，只看 B（相对 A）或 C（相对 A、B）新增的题目
- **📷 照片处理**：报名证件照/人像照尺寸处理，完全在浏览器本地完成

### 用户体验
- **🧭 全局导航**：所有页面顶部常驻导航栏，一键直达练习、考试、分类浏览、术语表、波段表、照片处理；移动端为汉堡菜单
- **🌗 明暗主题**：导航栏内随时切换，支持跟随系统 / 浅色 / 深色
- **✨ 流畅动效**：页面切换淡入过渡、按钮按压反馈、Logo 悬停动效，并尊重系统「减少动态效果」偏好
- **⌨️ 键盘快捷键**：方向键切题，数字键选择选项
- **📋 答题卡**：快速导航、标记、未答/标记筛选，交卷后显示对错
- **📱 移动优先**：响应式设计，PWA 可安装、可离线使用
- **💾 本地存储**：所有数据仅保存在浏览器 `localStorage`，不上传任何个人数据

## 技术栈

| 层 | 技术 |
| --- | --- |
| 语言 | Rust 2024 edition（MSRV 1.88） |
| 前端 | [Leptos 0.8](https://leptos.dev)（CSR）+ `leptos_router`，编译为 WebAssembly |
| 前端构建 | [Trunk](https://trunkrs.dev)（自动下载 Tailwind CSS v4 独立版、wasm-bindgen、wasm-opt） |
| 样式 | Tailwind CSS v4 + tw-animate-css，沿用原 shadcn/ui（new-york）设计令牌与类名 |
| 图标 | Lucide（内联 SVG，与原版路径一致） |
| 服务器 | Axum 0.8 + tower-http（SPA 回退、缓存头、gzip/brotli） |
| 工具链 | `ham-exam-tools`（clap、ureq、resvg、sha2） |
| 任务编排 | [cargo-make](https://github.com/sagiegurari/cargo-make) |
| PWA | 构建时由 Rust 生成 Service Worker（预缓存 + 运行时缓存策略） |

## 项目结构

```text
.
├── Cargo.toml              # workspace（edition 2024、统一依赖与 lint）
├── Makefile.toml           # cargo make 任务
├── Dockerfile              # 多阶段构建（distroless 运行镜像）
├── rustfmt.toml
├── crates/
│   ├── core/               # 领域模型：题目结构、内容指纹、考试规则/抽题/计分、分类体系、术语表、频谱波段数据、本地存储结构
│   ├── app/                # Leptos 前端
│   │   ├── index.html      # Trunk 入口（meta / manifest / 静态资源拷贝）
│   │   ├── Trunk.toml
│   │   ├── style/          # Tailwind 入口 CSS（主题变量、Geist 字体）
│   │   └── src/
│   │       ├── app.rs      # 路由与全局布局
│   │       ├── pages/      # 首页、练习、考试、分类浏览、术语表、频谱波段表、照片处理、404
│   │       ├── components/ # 题目卡片、答题卡、各类对话框、题库选择器…
│   │       ├── ui/         # 基础组件（Dialog/Sheet/Select/Checkbox/Radio…）
│   │       ├── data.rs     # 题库配置与题目加载（带缓存）
│   │       ├── store.rs    # localStorage 进度持久化
│   │       ├── photo.rs    # 照片压缩（Canvas，对齐 compressorjs 行为）
│   │       └── pwa.rs      # Service Worker 注册与更新提示
│   ├── server/             # 静态站点服务器
│   └── tools/              # 构建/维护 CLI（数据集、解析、图标、sw.js、sitemap）
│       └── templates/sw.js # Service Worker 模板
├── data/
│   ├── explanations.json   # 题目解析（key = 题目内容指纹）
│   └── glossary.json       # 术语表（术语表页面 + 解析增强）
└── public/
    ├── questions/          # 题库 JSON、config.json、题目图片（构建产物，已提交）
    ├── fonts/              # Geist 字体（本地托管）
    ├── manifest.json       # PWA manifest
    └── *.png / *.svg / favicon.ico
```

## 快速开始

### 环境要求

- Rust ≥ 1.88（推荐最新 stable，见 `.tool-versions`）
- `wasm32-unknown-unknown` 目标、[Trunk](https://trunkrs.dev)、[cargo-make](https://github.com/sagiegurari/cargo-make)

```bash
cargo install cargo-make
cargo make setup          # 安装 wasm32 目标与 Trunk
```

> Tailwind CSS、wasm-bindgen、wasm-opt 会在首次构建时由 Trunk 自动下载到本机缓存，无需 Node.js。

### 本地开发

```bash
cargo make dev            # http://127.0.0.1:3000 ，修改代码自动重新编译并刷新
```

开发模式下不注册 Service Worker，并会自动注销同源下残留的旧 SW，避免缓存和“发现新版本”弹窗干扰调试。开发产物输出到 `target/dev-dist/`，不会覆盖 release 的 `dist/`。

### 生产构建与运行

```bash
cargo make build          # 图标 + 前端（release）+ sw.js/sitemap + 服务器
cargo make serve          # 用 release 服务器托管 dist/：http://127.0.0.1:8080
```

构建产物位于 `dist/`，为纯静态文件，也可以直接部署到任意静态托管（Nginx、CDN、GitHub Pages 等），只需把未命中的路径回退到 `index.html`，并为 `sw.js`、`manifest.json`、`index.html` 设置 `Cache-Control: no-cache`。

## cargo make 任务一览

| 任务 | 说明 |
| --- | --- |
| `cargo make setup` | 安装 wasm32 目标与 Trunk |
| `cargo make dev` | 前端开发服务器（默认任务） |
| `cargo make build-web` | 构建前端（release）并执行 `postbuild` |
| `cargo make build` | 图标 + 前端 + 服务器 |
| `cargo make build-full` | 重新拉取题库数据后完整构建 |
| `cargo make serve` | release 服务器托管 `dist/` |
| `cargo make dataset` | 从 CSV 构建题库 JSON 与图片 |
| `cargo make icons` | 由 `public/pwa-icon.svg` 生成 PWA 图标 |
| `cargo make explanations-missing` | 统计缺失解析并导出待填模板 |
| `BATCH=… cargo make explanations-add` | 按题目 ID 合并解析 |
| `cargo make explanations-enhance` | 术语表增强解析 |
| `cargo make glossary-check` | 校验术语表并输出统计 |
| `cargo make explanations-apply` | 把解析写入题库 JSON |
| `BATCH=… cargo make explanations` | 合并 → 增强 → 写入，一步完成 |
| `cargo make fmt` / `fmt-check` | 代码格式化 / 检查 |
| `cargo make clippy` | Clippy（原生 + wasm） |
| `cargo make test` | 单元测试 |
| `cargo make ci` | 格式检查 + clippy + 测试 + 前端构建 |
| `cargo make docker-build` / `docker-run` | 构建 / 运行 Docker 镜像 |
| `cargo make clean` | 清理构建产物 |

## Docker 部署

```bash
docker build -t ham-web .
docker run -d --name ham-exam -p 3000:3000 ham-web
# 或
cargo make docker-build && cargo make docker-run
```

构建参数：

| 参数 | 默认值 | 说明 |
| --- | --- | --- |
| `SITE_URL` | `https://ham.onlyxp.me` | 写入 Open Graph 与 `sitemap.xml` 的站点地址 |
| `REBUILD_DATASET` | `0` | 设为 `1` 时构建阶段从远程 CSV 重新生成题库 |
| `TRUNK_VERSION` | `0.21.14` | Trunk 版本 |

```bash
docker build --build-arg SITE_URL=https://exam.example.com -t ham-web .
```

运行时环境变量：`HOST`（默认 `0.0.0.0`）、`PORT`（默认 `3000`）、`DIST_DIR`（默认 `/app/dist`）、`RUST_LOG`。
镜像基于 `gcr.io/distroless/cc-debian12:nonroot`，以非 root 用户运行，内置健康检查（`/healthz`）。

## 数据集构建

题库 JSON 与图片已提交在 `public/questions/`，日常开发无需重新构建。需要更新题库时：

```bash
cargo make dataset                                   # 从默认远程仓库拉取
DATASET_DIR=/path/to/csv-repo cargo make dataset     # 优先使用本地目录
DATASET_REMOTE=https://raw.githubusercontent.com/<you>/<repo>/main cargo make dataset
```

数据源需包含：`class_a.csv`、`class_b.csv`、`class_c.csv`、`full.csv`、`images.csv` 与 `images_2/{题号}.jpg`。CSV 列：`J`（题号）、`P`（分类码）、`Q`（题干）、`T`（答案）、`A`–`D`（选项），可选解析列（`Explanation` / `解析` / `analysis`）。

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

也可直接调用：`cargo run -p ham-exam-tools -- missing-explanations --help`。

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
| 增强 | `cargo make explanations-enhance` | 用 `data/glossary.json` 中 `inject: true` 的词条，为每条解析中**首次出现**的术语追加 `（通俗解释）`，幂等可重复执行 |
| 写入 | `cargo make explanations-apply` | 把解析写入 `public/questions/*.json`（离线完成，无需重新拉取 CSV） |

术语增强规则：长术语优先（「对流层散射」先于「散射」）；术语后已有括号、或处于原文括号内时跳过；每个术语每条解析只注入一次；「发射频率」中的「射频」不视为术语。

### 第 4 步：预览与提交

```bash
cargo make dev    # 在练习模式 / 分类浏览页检查解析显示
git add data/ public/questions/
git commit -m "docs(explanations): 补充 C 类解析 50 条"
```

### 维护术语表

`data/glossary.json` 同时用于**术语表页面**（`/glossary`，编译期嵌入前端，离线可用）和**解析术语增强**。文件为 JSON 对象，key 为术语（中文名称，或 `QRM`、`73` 这类缩写本身），词条顺序即页面同分类内的展示顺序：

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

旧写法 `"术语": "解释"` 仍然兼容，等价于 `{ "desc": "解释", "inject": true }`。

> **关于 `inject`**：注入是按子串匹配的，短缩写（如 `AM`、`CW`）或常见词会在解析中被大量误注入，因此新词条默认不参与注入。只有确实需要在解析中自动附加解释的中文专业术语才建议设为 `true`，并在执行后抽查效果。

修改后：

```bash
cargo make glossary-check                                    # 校验：分类是否存在、参见目标、重复词条、括号等
cargo make explanations-enhance && cargo make explanations-apply   # 仅当新增/修改了 inject=true 的词条时需要
cargo make dev                                               # 在 /glossary 预览
```

术语表页面功能：按分类筛选、搜索（术语 / 缩写 / 英文 / 别名 / 解释，精确匹配优先）、「只看英文缩写」、参见跳转，并统计该术语在 A/B/C 各题库中出现的题数（与分类浏览页的关键词搜索一致），点击即跳转到 `/browse?bank=…&q=…`。长度小于 3 的纯 ASCII 术语（如 `K`、`73`）误匹配过多，不做统计。也支持 `/glossary?q=驻波比` 直接定位。

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
- 部署新版本后，页面可见时会检查更新，发现新 Service Worker 时弹出「发现新版本」提示，点击「立即更新」即刷新到最新版；
- 应用图标由 `cargo make icons` 从 `public/pwa-icon.svg` 渲染（纯 Rust，resvg）。

## 本地存储与兼容性

`localStorage` 的 key 与数据结构与旧版（Next.js）完全一致，升级后用户已有进度与偏好可继续使用：

| key | 内容 |
| --- | --- |
| `practice:{版本}:{题库}` | 练习进度（顺序模式） |
| `practice:lastMode` | 上次练习题序 |
| `practice:noResumePrompt:{版本}:{题库}` | 本题库不再提示恢复 |
| `exam:savedState:{版本}:{题库}` | 未完成的考试 |
| `exam:answerCardFilter:{题库}` / `exam:showExplanation:{题库}` | 考试偏好 |
| `ui:shortcutsHelpSeen:{practice,exam}` | 快捷键说明是否已展示 |
| `theme` | `light` / `dark` / `system` |

## 开发指南

### 快捷键

- `← / →`：上一题 / 下一题
- `1-9`：选择对应选项；多选题为切换，`Shift` / `Cmd` + 数字为仅选该项
- `Enter`：打开搜索（练习 · 顺序模式）

### 代码规范

- Rust 2024 edition，格式遵循根目录 `rustfmt.toml`（2 空格缩进，行宽 100）；
- workspace 统一开启 `unsafe_code = "forbid"` 与 Clippy `correctness/suspicious/style/complexity/perf`；
- 提交前执行 `cargo make ci`。

### 修改 UI 的注意事项

- 基础组件的 Tailwind 类名与原 shadcn/ui 完全一致，位于 `crates/app/src/ui/`；类名合并使用 `cn(&[...])`（语义同 `tailwind-merge`）；
- Tailwind 会扫描 `crates/app/src` 下的 Rust 源码，新类名需以**完整字面量**出现（不要字符串拼接类名片段）；
- 在 `leptos` 中向依赖 context 的子组件（如 `RadioGroupItem`、`SelectItem`）传递子元素时，需在父组件的 children 内构建，而不是提前 `collect_view()`。

### 路由

| 路径 | 页面 |
| --- | --- |
| `/` | 首页，选择题库版本与类别 |
| `/practice?version=…&bank=A\|B\|C` | 练习模式 |
| `/exam?version=…&bank=A\|B\|C` | 模拟考试 |
| `/browse?bank=A\|B\|C&q=关键词` | 题库分类浏览（参数可选） |
| `/glossary?q=关键词` | 术语表（参数可选） |
| `/bands` | 频谱波段划分表 |
| `/photo-processor` | 报名照片处理 |

## 常见问题

- **题库为空或 404？** 确认 `public/questions/` 下存在 `A/B/C.json` 与 `config.json`，必要时执行 `cargo make dataset`。
- **首次构建很慢？** Trunk 会下载 Tailwind、wasm-bindgen、wasm-opt 并编译依赖，之后为增量构建。
- **wasm-opt 报 `bulk memory` 相关错误？** 已在 `index.html` 中通过 `data-wasm-opt-params` 启用新特性，请使用 `Trunk.toml` 中固定的 wasm-opt 版本。
- **更新部署后页面没有变化？** Service Worker 需要一次刷新才能激活，页面会弹出「发现新版本」提示；也可在浏览器 DevTools → Application → Service Workers 中手动更新。
- **搜索按钮不见了？** 搜索仅在练习模式的顺序模式下可用。

## 致谢

- **题库数据**：[TimXiedada/crac-amateur-radio-exam-questions-2025-csv](https://github.com/TimXiedada/crac-amateur-radio-exam-questions-2025-csv)
- **相关项目**：[AlliotTech/ham-exam-web](https://github.com/AlliotTech/ham-exam-web)
- **开源项目**：Leptos、Axum、Trunk、Tailwind CSS、tw-animate-css、Lucide、Geist 字体、resvg
