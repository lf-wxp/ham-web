# 许可与第三方声明

> README 只留结论；例外清单、第三方声明文件与部署方义务都在这份。

## 许可

本项目未另行声明的原创组件以 **MIT** 许可发布，许可全文见仓库根目录 [`LICENSE`](../LICENSE)；包含 GPL 组件的完整站点分发不是纯 MIT，例外见下。

需要注意的是，MIT 只覆盖本仓库的代码，**不覆盖题库数据**：题目与解析的版权归原作者所有，使用前请遵守上游仓库的许可与署名要求。

使用或再分发 MIT 组件时，须保留版权声明与 MIT 许可全文；GPL 组件须另外遵守其许可及对应源码交付要求，不能用 MIT 条款概括整个发布物。

### 第三方 MIT / BSD 转写代码

`crates/ft8`、`crates/ft8-worker` 转写自第三方的宽松许可项目，**仍按本仓库的 MIT 发布**，但再分发时必须保留对方的版权与许可声明，
全文随发布物提供（`/licenses/` 下，站内「许可」页有链接）：

| 来源 | 许可 | 声明文件 | 适用范围 |
| --- | --- | --- | --- |
| [kgoba/ft8_lib](https://github.com/kgoba/ft8_lib)（© 2018 Kārlis Goba） | MIT | [`LICENSES/MIT-ft8_lib.txt`](../LICENSES/MIT-ft8_lib.txt) | 消息打包、CRC-14、LDPC 软译码、同步与 waterfall、GFSK 合成；`crates/ft8/samples` 里的真实录音与黄金标准结果 |
| [KISS FFT](https://github.com/mborgerding/kissfft)（© 2003–2010 Mark Borgerding） | BSD-3-Clause | [`LICENSES/BSD-3-Clause-kissfft.txt`](../LICENSES/BSD-3-Clause-kissfft.txt) | `crates/ft8/src/fft.rs` 的混合基复数 / 实数 FFT |

### 例外：GPL-3.0 的六个 crate

下面六个 crate 含有转写自 GPL-3.0 项目的代码，因此**整个 crate 按 GPL-3.0（`GPL-3.0-only`）发布**，
许可全文见 [`LICENSES/GPL-3.0-only.txt`](../LICENSES/GPL-3.0-only.txt)，各自的 `Cargo.toml` 也已声明：

| crate | 为什么是 GPL |
| --- | --- |
| `crates/dominoex`、`crates/dominoex-worker` | 算法与 Varicode 表转写自 [fldigi](https://github.com/w1hkj/fldigi)（GPL-3.0） |
| `crates/lrpt`、`crates/lrpt-worker` | 帧层 / 80k 链路等部分转写自 [artlav/meteor_decoder](https://github.com/artlav/meteor_decoder)（GPL-3.0）；其余部分转写自 MIT 的 lrpt-encoder |
| `crates/navtex`、`crates/navtex-worker` | CCIR 476（SITOR-B）码表转写自 AI5GW/CCIR476（GPL-3.0），来源声明见 `crates/navtex/src/codec.rs` 文件头 |

要点：

- 这几个解码器编译成独立的 wasm 文件，由 `public/decode-worker.js` 经 `postMessage` 调用，
  与站点其余部分（MIT）之间没有链接关系；本站公开分发的 wasm 对应源码就是本仓库。
- **复用这六个 crate 时必须遵守 GPL-3.0**（衍生作品需以 GPL-3.0 开源）；其余 crate 仍是 MIT。
- `e2e/tests/dominoex_varicode_data.ts`（fldigi `dominovar.cxx`）与 `e2e/tests/navtex_varicode_data.ts`
  （转录自 `crates/navtex/src/codec.rs`）是测试用的 GPL 衍生数据，同样按 GPL-3.0 对待。
- 发布物自带 `/license.html`、`/licenses/LICENSE`、`/licenses/GPL-3.0-only.txt` 以及第三方 MIT / BSD 声明文件；许可说明是站内页面，点击与刷新均复用全站应用壳、主题和语言，许可全文链接使用完整文档导航。`postbuild` 生成 `/license.html` 应用壳别名，Docker 构建检查这些文件，OCI 许可标签为 `MIT AND GPL-3.0-only`。部署方必须提供与实际构建版本一致的源码提交或归档，不以仓库最新版本代替。
- 这份划分是工程上的处理，不是法律意见；若有疑问请自行咨询。

