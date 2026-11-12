# 用 fldigi 自己的收发端验证（fldigi-web）

[`crashdemons-external/fldigi-web`](https://github.com/crashdemons-external/fldigi-web) 把 **fldigi 4.2.13 的原版调制解调器**
编译成了 WebAssembly（`web/fldigi-core.wasm`）。这两个 Node 脚本直接调用它的 API，不经浏览器：

| 脚本 | 做什么 |
| --- | --- |
| `gen.mjs <模式> <文本> <中心 Hz> <输出.wav>` | 用 fldigi 的**原版发端**生成一段完整发射（含前导与收尾）；`node gen.mjs list` 列出 MFSK / THOR 模式名 |
| `dec.mjs <模式> <输入.wav> <中心 Hz>` | 用 fldigi 的**原版接收机**解码（末尾补 6 秒静音，让管线里压着的字符流出来） |
| `gen_all.sh <目录>` | 一次生成全部 MFSK / THOR 子模式的 `fw_<模式>.wav` |

```text
git clone --depth 1 https://github.com/crashdemons-external/fldigi-web /tmp/fldigi-web
FLDIGI_WEB=/tmp/fldigi-web sh gen_all.sh /tmp/fw
MFSK_REAL_DIR=/tmp/fw cargo test -p ham-web-mfsk --release --test real_recordings -- --include-ignored
THOR_REAL_DIR=/tmp/fw cargo test -p ham-web-thor --release --test real_recordings -- --include-ignored
```

## 双向验证（2026-11-11 做过）

- **fldigi 发端 → 本仓收端**：上面两个 `real_recordings` 测试。9 个 MFSK 子模式（含 5 位符号的 MFSK4 / 8 与 3 位符号的 MFSK31）与
  11 个 THOR 子模式（含 K = 15 的 25x4 / 50x1 / 50x2 / 100）全部逐字解出，中心频率自动检测都落在 1/8 音调间隔内。
- **本仓发端 → fldigi 收端**：`generate_sample` 示例生成的音频喂给 `dec.mjs`，MFSK4 / 8 / 16 / 31 / 32 / 64 / 128 与
  THOR micro / 4 / 8 / 16 / 25x4 / 50x1 / 50x2 / 100 全部被 fldigi 的接收机还原出报文。

## 这个验证没覆盖什么

- fldigi-web 里还有 THOR25 / 32 / 44 / 56 四个更新的模式，本仓用的 fldigi GitHub 镜像源码里没有它们的参数，所以没做。
- 只验了干净信号；弱信号 / 衰落 / 时钟漂移下与 fldigi 接收机的差距没有量化（本仓是硬位判决 + 一次性对齐，fldigi 是软位 + 逐符号跟踪）。
- 载频偏差（AFC）、图片模式不在范围内。
