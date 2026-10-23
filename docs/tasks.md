# cargo make 任务一览

> README 只列最常用的几条；这里是全集。任何一条都能用 `cargo make --list-all-steps` 复核。

| 任务 | 说明 |
| --- | --- |
| **环境** | — |
| `cargo make setup` | 安装 wasm32 目标、Trunk 与 wasm-bindgen CLI（版本按 `Cargo.lock` 固定） |
| **开发** | — |
| `cargo make dev` | 前端开发服务器（默认任务，http://127.0.0.1:3000） |
| `cargo make dev-full` | 后端 API（3001）+ 前端开发服务器（3000，代理 `/api`） |
| **构建** | — |
| `cargo make build-worker` / `build-sstv-worker` / `build-wspr-worker` / `build-rtty-worker` / `build-olivia-worker` / `build-dominoex-worker` / `build-navtex-worker` / `build-lrpt-worker` / `build-ft8-worker` / `build-hell-worker` / `build-wefax-worker` / `build-afsk1200-worker` | 构建单个解码 Web Worker（wasm-bindgen `--target no-modules`；APT 的任务名是 `build-worker`，其余带 codec 前缀）。全部一起构建用 `cargo make build-workers`，清单取自 `crates/*-worker`（十二个） |
| `cargo make build-web` | 构建前端（release）并执行 `postbuild`（sw.js / sitemap / 预压缩） |
| `cargo make build-server` | 构建静态站点服务器（release） |
| `cargo make build` | 图标 + 前端 + 服务器 |
| `cargo make build-full` | 重新拉取题库数据后完整构建 |
| `cargo make serve` | release 服务器托管 `dist/`（http://127.0.0.1:8080） |
| **数据与资源** | — |
| `cargo make dataset` | 从 CSV 构建题库 JSON 与图片（`DATASET_DIR` / `DATASET_REMOTE` 可覆盖数据源），建库时统一题面排版 |
| `cargo make questions-normalize` | 一次性迁移：把已提交题库按排版规则归一化（含解析表 key 迁移与搜索索引刷新），不重新下载 CSV |
| `cargo make dxcc` | 从 country-files.com 拉取 cty.csv，重新生成 `crates/core/data/dxcc.txt` |
| `cargo make dxcc-map` | 从 Natural Earth 国界 GeoJSON 生成 DXCC 实体边界 `public/dxcc-entities.bin`（i16 定点 + delta 二进制编码） |
| `cargo make data-sync` | 三个外部数据源一起重新生成（dxcc + dxcc-map + dataset，需联网） |
| `cargo make data-freshness` | 比对上游有无新版；连不上只报告（需联网，见 `docs/data-sync.md`） |
| `cargo make data-sync-check` | 产物来源戳与依赖摘要的离线一致性（已在 `cargo make check` 里，不联网） |
| `cargo make data-staleness` | 人工转录的外部事实（法规 / 器材实测 / 磁环 / 成本档位）多久没复核（离线；超过 6 个月只告警，日期读不出来 / 在未来才失败；已在 `check` 里） |
| `cargo make data-staleness-strict` | 同上，但超过 6 个月也判失败（CI 周检 `staleness` job 用） |
| `cargo make data-stamp` | 给引入来源戳之前生成的旧产物补戳（内容不动，不联网） |
| `cargo make icons` | 由 `public/pwa-icon.svg` 生成 PWA / Apple Touch 图标与 favicon.ico |
| **解析与术语** | — |
| `cargo make explanations-missing` | 统计缺失解析并导出待填模板（`BANK` / `LIMIT` 可选） |
| `BATCH=… cargo make explanations-add` | 按题目 ID 合并解析 |
| `cargo make explanations-enhance` | 用术语表为解析注入术语解释（幂等） |
| `cargo make explanations-sync` | 对齐同一道题多指纹变体的解析文本 |
| `cargo make glossary-check` | 校验术语表（分类 / 参见 / 重复 / 括号）并输出统计 |
| `cargo make explanations-check` | 校验解析表（答案一致性、重复/错位注入、裸释义、同题多版本分叉），已并入 `cargo make check` 与 `cargo make ci` |
| `cargo make explanations-apply` | 把解析写入题库 JSON（离线，无需重建数据集） |
| `BATCH=… cargo make explanations` | 合并 → 增强 → 写入，一步完成 |
| **国际化** | — |
| `cargo make i18n-check` | 校验界面文案词典（重复 key、占位符数量、复数变体、死条目、语言包新鲜度）并统计按域覆盖率 |
| `cargo make i18n-pack` | 生成运行时语言包 `public/data/i18n/{lang}.json`（改了译文后跑一次） |
| `cargo make i18n-plural-candidates` | 导出复数候选骨架（源码用 `tf`/`tp`、中文带数量占位符的文案）到 `tmp/plural-candidates-en.json`；导出西语用 `cargo run -q -p ham-web-tools -- check-i18n --plural-candidates tmp/plural-candidates-es.json --lang es` |
| `cargo make knowledge-i18n-check` | 校验知识库正文译文：中文原文作 key，源码改过句子后译文即失效（静默回退中文），已并入 `cargo make check` |
| `cargo make e2e-check` | e2e 覆盖对账：`e2e/coverage-targets.json` 的每条核心旅程（grep）必须能在 `e2e/tests` 的 spec 里命中，已并入 `cargo make check` |
| **解码器样本** | — |
| `cargo make samples` | 一键生成合成测试样本 WAV 到 `tmp/`（apt / sstv / wspr / lrpt / dominoex / olivia / navtex / rtty / psk31 / spectrum；FT8 用 `ft8-sample` 单独生成） |
| `cargo make apt-sample` / `apt-decode` / `apt-test` | 生成 / 命令行解码 / 端到端自检 APT 样本 WAV |
| `cargo make sstv-sample` / `sstv-decode` / `sstv-test` | 同上，SSTV |
| `cargo make wspr-sample` / `wspr-decode` / `wspr-test` | 同上，WSPR |
| `cargo make ft8-sample` / `ft8-decode` / `ft8-test` | 同上，FT8 / FT4（`MSG` 换发送文本、`NOISE` 换噪声电平） |
| `cargo make rtty-sample` / `rtty-decode` / `rtty-test` | 同上，RTTY |
| `cargo make olivia-sample` / `olivia-decode` / `olivia-test` / `olivia-xcheck` | 同上，Olivia（`xcheck` 与独立参考实现双向对拍） |
| `cargo make dominoex-sample` / `dominoex-decode` / `dominoex-test` / `dominoex-xcheck` | 同上，DominoEX |
| `cargo make navtex-sample` / `navtex-decode` / `navtex-test` | 同上，NAVTEX |
| `cargo make lrpt-sample` / `lrpt-decode` / `lrpt-test` | 同上，LRPT |
| `cargo make psk31-sample` | 生成 PSK31 测试样本 WAV（供手动测试 `/psk-decode`） |
| `cargo make spectrum-sample` / `spectrum-analyze` / `spectrum-test` | 生成 / 命令行分析 / 端到端自检合成频谱样本（供 `/sdr-waterfall`） |
| **像素风资源** | — |
| `cargo make pixel-check` | 只读校验：调色板文字色阶与每个配色方案的对比度达标，`schemes.css` 与方案表一致；已并入 `check` / `ci` |
| `cargo make pixel-palette` | 校正 Tailwind 色阶里的文字档，使其在所有表面（含每个配色方案的亮 / 暗表面）上满足 WCAG AA |
| `cargo make pixel-schemes` | 由方案表（`crates/core/src/color_scheme.rs`）生成 `style/pixel/schemes.css` |
| `cargo make pixel-sprites` | 由字符画（`crates/tools/src/pixel/sprite_art.rs`）生成像素图标与精灵的数据文件（需 `PIXELARTICONS_DIR`） |
| `cargo make fonts-pixel` | 按仓库语料重新子集化像素字体（需 `FONT_SRC`，纯 Rust，不需要 Python） |
| **质量** | — |
| `cargo make fmt` / `fmt-check` | 代码格式化 / 格式检查 |
| `cargo make clippy` | Clippy（原生 crate + wasm 前端，`-D warnings`） |
| `cargo make test` | 单元测试 |
| `cargo make check` | 格式检查 + clippy + 单元测试 + 语言包重生成与界面文案校验 + 知识库译文校验 + 解析表校验 + e2e 覆盖对账 + 界面规范守卫 + 结构守卫 + README 对账 + 像素风生成物校验 + 数据来源戳与时效性核对日期（不构建前端） |
| `cargo make readme-check` | README 对账：Worker 数量、`build-*-worker` 任务清单、路由数、计算器数量、`docs/` 链接、`README「章节」` 引用与 `docs/images/` 截图必须与代码一致 |
| `cargo make e2e` | 构建 release 站点并运行 Playwright 端到端测试（见 [`testing.md`](testing.md)） |
| `cargo make e2e-coverage` | 统计 e2e 覆盖率（路由冒烟 / 交互 / 关键流程），报告写入 `e2e/test-results/coverage.{md,json}` |
| `cargo make size-check` | 性能预算：按 `perf-budget.json` 校验 `dist/` 的 br 压缩后体积（需先 `build-web`） |
| `cargo make ci` | 等价于 `check` + 前端构建（`build-web`）+ `size-check` |
| **部署与清理** | — |
| `cargo make docker-build` / `docker-run` | 构建 / 运行 Docker 镜像 |
| `cargo make clean` | 清理构建产物（`cargo clean` + `dist` / `tmp`） |

> 完整任务列表可用 `cargo make --list-all-steps` 查看；也可直接调用底层 CLI，如 `cargo run -p ham-web-tools -- <子命令> --help`。

