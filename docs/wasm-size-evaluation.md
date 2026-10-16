# 主包 wasm 体积：`erase_components` 与按路由拆分的评估

评估日期：2026-10-15。下文数字为当时的独立评估，不是本轮构建结果。

2026-10-16 修复轮次：已在 `.cargo/config.toml` 的 wasm 目标启用 `--cfg erase_components`，
并将地图四组动态标注改为独立构造工厂；静态大陆轮廓仍缓存。类型擦除同时作用于 dev / release，
避免两种构建口径不同。主包预算未上调；最终体积和修复后回归结果以
`docs/code-review-2026-10-16.md` 的本轮验收记录为准。按路由拆分仍只是评估、未落地。

本文回答：主包 wasm（br）逼近 `perf-budget.json` 上限之后，还有哪些手段、各自收益与代价多大、先做哪个。
所有数字都是**实测**，来源与口径见第 2 节；不能由实测或源码阅读支持的结论，会明确标成「未验证」。

## 1. 结论（先读这个）

| 手段 | 主包 br | 完整 e2e | 改动面 | 建议 |
| --- | --- | --- | --- | --- |
| `--cfg erase_components` | **−644 KiB（−28%）** | 673 通过 / 1 跳过（与基线一致） | 一行构建参数 + 4 处代码 | **先做** |
| 拆分体积最大的 5 条路由 | −399 KiB（−17%），总下载量 +174 KiB | 673 通过 / 1 跳过（与基线一致） | 换构建流水线、chunk 哈希、预算与 dev 开关 | 先做上一条再重测，**再决定** |
| 拆分体积最大的 14 条路由 | −674 KiB（−29%），总下载量 +350 KiB | 未跑（只做了浏览器冒烟） | 同上 | 不单独推荐 |
| 拆分全部 169 条路由 | −1444 KiB（−63%），总下载量 **+1430 KiB（+62%）** | 未跑 | 同上，且产物 1970 个文件 | **不可行** |
| `ToolSection` 抽取（已做） | −16.1 KiB | 35 个 `/tools` 相关用例通过 | 已落地 | 已完成 |

三条判断：

1. **`erase_components` 的收益比拆分大、风险比拆分小得多**，且不改构建链。它是 Leptos 自带的开关：把每个 `view!` 里的闭包类型擦除，换来更少的单态化。代价是每个响应式闭包多一次 `Arc<Mutex<_>>` 分配和一次虚调用。本机实测挂载反而更快（−6% 到 −18%），因为 wasm 更小、编译更快；但**没有在频谱 / 瀑布图这类每帧更新的页面上压测运行时开销**（见第 6 节）。
2. **拆分是可行的，不是理论方案**：我在干净 HEAD 副本上跑通了完整流水线，真实浏览器里首页只加载主包，进入被拆出的路由时才按需取 chunk，冷启动直达也正常。但它要改的东西很多（第 5 节），且「总下载量」一定变大，因为 PWA 预缓存会把所有 chunk 都装进来。拆分收益的本质是**首次访问的关键路径变短**，不是总字节变少。
3. **「全部路由都拆」是死路**：主包确实只剩 855 KiB，但产物有 1970 个文件、其中 1799 个是共享 chunk，**97% 的共享 chunk 小于 2 KiB**，合计比不拆多出 62%。拆分只对少数大页面划算。

## 2. 方法与可信度

- **干净基线**：评估期间发现工作区被并发修改（`ui/toast`、`ui/table`、`ui/pagination`、多处 `alert` → `push_error` 在进行中），在工作区上直接对比会混入别人的改动。所以所有对照都在 `git archive HEAD` 导出到 `/tmp` 的副本里做，用独立的 `CARGO_TARGET_DIR`，不碰工作区、`dist/`、`target/`。
  - 这也**更正了我之前的一个数字**：`ToolSection` 的纯收益是 **−16.1 KiB**（干净 HEAD 对照 `2354384 → 2337877` 字节），不是此前记录的 −11.6 KiB；那个差值里混入了约 +4.5 KiB 的并发改动。
- **口径**：与 `size-check` 一致，都是 brotli q11 之后的字节数。
- **基线**：干净 HEAD、Trunk release + `wasm-opt -Oz`：**2354384 字节 = 2299.2 KiB**。
- **流水线对照**：拆分用的是手工流水线（见第 7 节）。同一份源码不拆分、走同一条手工流水线得到 2356795 字节，与 Trunk 基线只差 +0.1%，所以拆分数字可以直接和基线比。
- **没做**：拆分后的完整 e2e 只在 top-5 上跑；14 路与全部路由只做了体积测量和首页 / `/tools` / `/nec` 的浏览器冒烟。

## 3. 数据

| 变体 | 主包 br | 相对基线 | chunk 合计 br | 总计 br | 总计相对基线 |
| --- | --- | --- | --- | --- | --- |
| 基线 A（干净 HEAD） | 2354384 | — | — | 2354384 | — |
| B = A + `ToolSection` | 2337877 | −16.1 KiB | — | 2337877 | −16.1 KiB |
| C = B + `erase_components` | 1678182 | **−644.2 KiB（−28.2%）** | — | 1678182 | −644.2 KiB |
| 拆 5 条（log / tools / morse / nec / satellites） | 1945557 | −399.2 KiB（−17%） | 587388 | 2532945 | +174.4 KiB（+8%） |
| 拆 14 条 | 1664384 | −673.8 KiB（−29%） | 1048171 | 2712555 | +349.8 KiB（+15%） |
| 拆全部 169 条 | 875582 | −1444.1 KiB（−63%） | 2942690 | 3818272 | +1429.6 KiB（+62%） |

拆 14 条时的构成：主包 1 个、页面 chunk 14 个（br 合计 946 KiB）、共享 chunk 98 个（br 合计 77 KiB，其中 92 个小于 2 KiB）。
拆全部时：共享 chunk 1799 个（97% 小于 2 KiB），页面 chunk 169 个，最小的页面 chunk 只有约 460 字节 br。

**进入一个被拆出的页面时的网络代价**（拆 14 条产物，真实浏览器测得，未压缩字节）：

| 路由 | 页面 chunk | 共享 chunk | 请求数 | 合计 |
| --- | --- | --- | --- | --- |
| `/tools` | 499 KiB | 30 个，44 KiB | 31 | 543 KiB |
| `/nec` | 376 KiB | 23 个，17 KiB | 24 | 393 KiB |

首页首次加载只取主包（raw 6086 KiB）与 `__wasm_split.js`（40 KiB）。

**`erase_components` 的运行时对照**（同一台机器、`serviceWorkers: block`、每项 7 次取中位数，B → C）：

| 路由 | 挂载到出现标题（ms） |
| --- | --- |
| `/` | 100 → 85 |
| `/tools` | 144 → 136（频率框连续输入 20 次：52 → 48 ms） |
| `/log` | 98 → 85 |
| `/morse` | 124 → 102 |
| `/satellites` | 97 → 85 |

这是本机回环、无网络，挂载时间里包含 wasm 编译，所以 wasm 更小本身就会更快；它**不能**证明稳态的响应式更新没有变慢。JS 堆指标（`usedJSHeapSize`）不包含 wasm 线性内存，没有参考价值，没有采用。

## 4. 已确认的事实

| 事实 | 依据 |
| --- | --- |
| `#[lazy]` / `#[lazy_route]` 在纯 CSR 下可用 | 读 `leptos_macro` 源码：`is_wasm = cfg!(feature = "csr") \|\| cfg!(feature = "hydrate")`；并实际构建、运行通过 |
| 拆分工具是独立的库 + CLI，不绑 `cargo-leptos` | `wasm_split_cli_support` 0.2.3 导出 `transform(Options)`，带 `wasm-split-cli` 二进制；`cargo install --locked --features build-binary` 约 15 秒装好 |
| **Trunk 无法托管拆分** | 拆分必须在 cargo 产物与 `wasm-bindgen` 之间插入，且 `wasm-bindgen` 要加 `--keep-lld-exports`；读 Trunk 源码，它只暴露 `data-no-demangle` 等少数属性，没有对应开关，也没有这一步的钩子 |
| `cargo-leptos --split` 不是现成替代 | 它构建 `--lib` crate（带 `hydrate` / `ssr` feature、`[package.metadata.leptos]`）；本项目是 `bin`。**没有实际尝试**，只是从其源码与 README 判断需要重构 |
| 不经拆分工具处理的产物不能直接用 | 未处理的 wasm 经 `wasm-bindgen` 后，JS 里仍有 `import "__wasm_split_placeholder__"`（实测 20 处），浏览器无法解析。所以 **dev 构建（`trunk serve`）不能无条件使用 lazy 路由** |
| `sw.js` 预缓存清单自动包含 chunk | `postbuild::precache_manifest` 遍历整个 `dist/`；实测清单从 130 个涨到 146 个，**无需改生成逻辑** |
| **chunk 文件名没有内容哈希** | `chunk_N.wasm` 的编号每次构建都会重排，`split_*_<数字>.wasm` 里的数字来自源码位置而非内容。服务端 `is_hashed_asset` 因此不会给它们 `immutable`；无 Service Worker 的首访用户可能拿到旧 chunk 配新主包，实例化失败。`server/main.rs` 里已有同类事故的注释（旧胶水配新 wasm） |
| `size-check` 在拆分产物上会直接报错 | 实测：预算项 `ham-web-app-*_bg.wasm` 在 dist 里没有匹配文件（我的演示产物主包没带哈希，真实实现带哈希则不会报这个错）；**但无论如何 chunk 不在任何预算项里，会静默不受限** |
| `erase_components` 在当前代码上只需 4 处修复 | 全量构建只报 4 处 `tick_views.clone()` 一类错误（`pages/map/map_view.rs`，`collect_view()` 的 `Vec<View<…>>` 在擦除模式下不是 `Clone`） |

## 5. 若决定做拆分：必须同时解决的事

1. **替换前端构建流水线**：`cargo rustc`（`-C link-args=--emit-relocs`，`strip=none`，`debug=0`）→ `wasm-split-cli` → `wasm-bindgen --target web --keep-lld-exports --no-demangle` → `wasm-opt -Oz` → 内容哈希改名 → 手写 `index.html` 的加载脚本、`modulepreload` 与 SRI。仓库已有 8 个解码 Worker 走手工流水线（`scripts/build-decode-worker.sh`），可参照。
2. **dev 开关**：用 `cfg(split_wasm)` 之类的自定义 cfg 区分，dev 直接 `view=XPage`，release 才用 `Lazy::<…>::new()`。
3. **chunk 内容哈希**：在 `postbuild` 里按内容重命名，并改写 `__wasm_split.js` 里的引用。
4. **预算门禁**：`perf-budget.json` 增加「主包」「单个 chunk 上限」「总量上限」三类，否则新增页面只是换个地方继续膨胀；同时处理 `max_depth(2)` 与通配。
5. **失败处理**：`#[lazy]` 默认在 chunk 加载失败时 panic。需要 `#[lazy(fallible)]` 加 `ErrorBoundary`，离线依赖 SW 预缓存兜底。
6. **共享 chunk 碎片**：进入一个页面要 24–31 个请求。HTTP/2 下可接受，弱网 / HTTP/1.1 下是实实在在的代价。
7. **版本耦合**：`wasm_split_cli_support` 0.2.3 要与 `wasm_split_helpers` 0.2.3、`leptos` 0.8.21 配套；Dockerfile 里要像 `wasm-bindgen-cli` 一样固定版本。
8. **预缓存下载量上升**：top-5 时 install 阶段多下载约 174 KiB（br）。

## 6. 未验证项

- **`erase_components` 与拆分叠加后的收益**：没有测。直觉上叠加后拆分的绝对收益会变小（页面代码本身变小了），但这只是推测。
- **运行时压力场景**：频谱 / 瀑布图、NEC 求解后的三维方向图、摩尔斯训练器这类持续更新的页面，没有对比过 `erase_components` 前后的帧率或 CPU 占用。
- **弱网与真机**：所有耗时都在本机回环上测，没有限速，也没有真机。
- **非 Chromium 浏览器**：拆分依赖多个 wasm 实例共享同一份 `memory` 与函数表、`WebAssembly.instantiateStreaming`，只在 Chromium 里跑过。
- **拆分后的完整 e2e**：仅 top-5 跑了（见第 1 节表）；14 路与全部路由没跑。
- **`cargo leptos --split`** 本身没有实际尝试。
- **主包里还剩什么**：拆 14 条后主包 raw 约 6 MiB；其中 1.4 MiB 是数据段（内嵌词典、知识库等），**不会被拆分带走**。

## 7. 推荐顺序与止损线

1. **先做 `erase_components`**（预计半天到一天；这是估计，不是承诺）：
   1. 在 `map_view.rs` 把 4 处 `.clone()` 改成每次调用重新生成视图的闭包；
   2. 在 `.cargo/config.toml` 的 wasm 目标 `rustflags` 里加 `--cfg erase_components`（dev 也会受益：编译更快）；
   3. 跑完整 e2e，并手测频谱、瀑布图、NEC、摩尔斯训练器；
   4. 把 `perf-budget.json` 的主包上限收紧到实测值附近，**否则省下来的空间会被后续页面悄悄吃掉**。
   - **止损**：完整 e2e 出现任何回归，且一天内定位不了，就放弃，回到「新页面进服务端 / Worker」的纪律。
2. **重测**，再回答「主包还需要更小吗」。如果余量已经充裕，拆分可以不做。
3. **只有在仍需要时**，才做 top-5 的拆分：先落地第 5 节的 1–5 项，再上线。**不要**往 14 条或全部路由推。

## 8. 复现方法

下面的脚本都不碰仓库工作区；`SRC` 指向 `git archive HEAD` 导出的副本。

```sh
# 干净副本（public/ 里的生成物不入库，要补拷）
git archive HEAD | tar -x -C /tmp/wt-head

# 基线与 erase_components（Trunk release，独立 target 目录）
(cd /tmp/wt-head/crates/app && CARGO_TARGET_DIR=/tmp/t-A trunk build --release --dist /tmp/dist-A)
(cd /tmp/wt-head/crates/app && RUSTFLAGS="--cfg erase_components" CARGO_TARGET_DIR=/tmp/t-erase trunk build --release --dist /tmp/dist-erase)

# 拆分流水线
cargo install wasm_split_cli_support --features build-binary --locked --root /tmp/ws-bin
CARGO_PROFILE_WASM_RELEASE_STRIP=none CARGO_PROFILE_WASM_RELEASE_DEBUG=0 \
  cargo rustc --profile wasm-release --target wasm32-unknown-unknown -p ham-web-app --bin ham-web-app \
  -- -C link-args=--emit-relocs
/tmp/ws-bin/bin/wasm-split-cli <raw.wasm> <out>/split --out-name ham-web-app
wasm-bindgen --target web --keep-lld-exports --no-demangle --out-dir <out>/bindgen --out-name ham-web-app <out>/split/ham-web-app.wasm
wasm-opt --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext --enable-mutable-globals -Oz <文件> -o <文件>
brotli -q 11 -c <文件> | wc -c
```

踩过的坑：

- `cargo bloat` 不能用于 wasm 目标；归因用 `twiggy`，且需要带 name 段的构建（`strip=none`、`debug=0`）。`debug=1` 会让产物膨胀到 646 MB。
- 带符号构建会被随后的 `trunk build --release` 覆盖，要分析得连着做。
- 拆分脚本里不要在 `set -o pipefail` 下写 `ls | head`：文件数多（全部路由有 1970 个）时 `SIGPIPE` 会让脚本静默退出。
- 同时开两个 Playwright 进程必须用不同的 `--output` 目录。
- 评估期间工作区在被并发修改，对比一律用 `git archive` 副本。
