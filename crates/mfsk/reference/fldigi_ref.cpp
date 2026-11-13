// 与 fldigi 上游代码对拍用的 C++ 参考程序（GPL-3.0，随 crate 一起按 GPL-3.0 对待）。
//
// 它**直接编译 fldigi 上游的** viterbi.cxx / interleave.cxx / mfskvaricode.cxx / thorvaricode.cxx，
// 只把「怎么调用它们」的那几十行（mfsk.cxx 的 sendbit / sendchar / clearbits / flushtx，
// thor.cxx 的 sendsymbol / sendchar / Clearbits / flushtx）照抄过来 —— 所以对拍覆盖的是
// 「Rust 版的卷积码 / 交织器 / Varicode / Viterbi 与上游类逐位一致」，以及「帧结构的编排与上游一致」。
// 音频部分（音调频率、滤波、同步）不在这里：那部分由真实录音验证（见 tests/real_recordings.rs）。
//
// 用法（由 tests/upstream_xcheck.rs 调用）：
//   fldigi_ref mfsk <symbits> <depth> <preamble> <lead_zeros:0|1> <text>
//   fldigi_ref thor <k15:0|1> <depth> <flushlength> <micro:0|1> <text>
//   fldigi_ref viterbi <k> <poly1> <poly2> <traceback> <seed> <n>
// 前两个输出音调序列（空格分隔）；viterbi 输出 `位 度量` 每步一行。

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <vector>

#include "interleave.h"
#include "mfskvaricode.h"
#include "thorvaricode.h"
#include "viterbi.h"

// viterbi.cxx 用到的 `parity`（上游在 misc.cxx 里，这里不整个编译 misc.cxx）。
int parity(unsigned long w) { return __builtin_popcountl(w) & 1; }

// fldigi `grayencode`：前缀异或（见 misc.cxx，名字与传统叫法相反）。
static unsigned int grayencode_(unsigned int data) {
  unsigned int bits = data;
  for (int s = 1; s < 8; s++) bits ^= data >> s;
  return bits;
}

// ---------------------------------------------------------------- MFSK
struct MfskTx {
  int symbits, depth, preamble, numtones;
  encoder *enc;
  interleave *txinlv;
  unsigned int bitshreg = 0;
  int bitstate = 0;
  std::vector<int> tones;

  MfskTx(int sb, int d, int p) : symbits(sb), depth(d), preamble(p), numtones(1 << sb) {
    enc = new encoder(7, 0x6d, 0x4f);
    txinlv = new interleave(symbits, depth, INTERLEAVE_FWD);
  }
  void sendsymbol(int sym) { tones.push_back((int)grayencode_(sym & (numtones - 1))); }
  void sendbit(int bit) {
    int data = enc->encode(bit);
    for (int i = 0; i < 2; i++) {
      bitshreg = (bitshreg << 1) | ((data >> i) & 1);
      bitstate++;
      if (bitstate == symbits) {
        txinlv->bits(&bitshreg);
        sendsymbol(bitshreg);
        bitstate = 0;
        bitshreg = 0;
      }
    }
  }
  void sendchar(unsigned char c) {
    const char *code = varienc(c);
    while (*code) sendbit(*code++ - '0');
  }
  void flushtx(int nbits) {
    sendbit(1);
    for (int i = 0; i < nbits; i++) sendbit(0);
    bitstate = 0;
  }
  void clearbits() {
    int data = enc->encode(0);
    for (int k = 0; k < preamble; k++) {
      for (int i = 0; i < 2; i++) {
        bitshreg = (bitshreg << 1) | ((data >> i) & 1);
        bitstate++;
        if (bitstate == symbits) {
          txinlv->bits(&bitshreg);
          bitstate = 0;
          bitshreg = 0;
        }
      }
    }
  }
};

// `lead_zeros`：clearbits 之后是否发 preamble / 3 个零位（mfsk.cxx 的 tx_process 对 64L / 128L 不发）。
static void run_mfsk(int symbits, int depth, int preamble, bool lead_zeros, const char *text) {
  MfskTx tx(symbits, depth, preamble);
  tx.clearbits();
  if (lead_zeros)
    for (int i = 0; i < preamble / 3; i++) tx.sendbit(0);
  tx.sendchar('\r');
  tx.sendchar(2);
  tx.sendchar('\r');
  for (const char *p = text; *p; p++) tx.sendchar((unsigned char)*p);
  tx.sendchar('\r');
  tx.sendchar(4);
  tx.sendchar('\r');
  tx.flushtx(preamble);
  for (size_t i = 0; i < tx.tones.size(); i++) printf("%d%c", tx.tones[i], i + 1 == tx.tones.size() ? '\n' : ' ');
}

// ---------------------------------------------------------------- THOR
struct ThorTx {
  encoder *enc;
  interleave *txinlv;
  unsigned int bitshreg = 0;
  int bitstate = 0, txprevtone = 0, flushlength;
  std::vector<int> tones;

  ThorTx(bool k15, int depth, int flush) : flushlength(flush) {
    if (k15)
      enc = new encoder(15, 044735, 063057);
    else
      enc = new encoder(7, 0x6d, 0x4f);
    txinlv = new interleave(4, depth, INTERLEAVE_FWD);
  }
  void sendsymbol(int sym) {
    int tone = (txprevtone + 2 + sym) % 18;
    txprevtone = tone;
    tones.push_back(tone);
  }
  void sendchar(unsigned char c, int secondary) {
    const char *code = thorvarienc(c, secondary);
    while (*code) {
      int data = enc->encode(*code++ - '0');
      for (int i = 0; i < 2; i++) {
        bitshreg = (bitshreg << 1) | ((data >> i) & 1);
        bitstate++;
        if (bitstate == 4) {
          txinlv->bits(&bitshreg);
          sendsymbol(bitshreg);
          bitstate = 0;
          bitshreg = 0;
        }
      }
    }
  }
  void sendidle() { sendchar(0, 0); }
  void clearbits() {
    int data = enc->encode(0);
    for (int k = 0; k < 1400; k++) {
      for (int i = 0; i < 2; i++) {
        bitshreg = (bitshreg << 1) | ((data >> i) & 1);
        bitstate++;
        if (bitstate == 4) {
          txinlv->bits(&bitshreg);
          bitstate = 0;
          bitshreg = 0;
        }
      }
    }
  }
  void flushtx() {
    for (int i = 0; i < flushlength; i++) sendidle();
    bitstate = 0;
  }
};

// 文本里以 `\x01` 开头的字符表示「二次信道」字符（`\x01` 之后的下一个字节）。
static void run_thor(bool k15, int depth, int flush, bool micro, const char *text) {
  ThorTx tx(k15, depth, flush);
  tx.clearbits();
  for (int j = 0; j < 16; j++) tx.sendsymbol(0);
  tx.sendidle();
  tx.sendchar('\r', 0);
  if (!micro) {
    tx.sendchar(2, 0);
    tx.sendchar('\r', 0);
  }
  for (const char *p = text; *p; p++) {
    if (*p == 1 && p[1]) {
      tx.sendchar((unsigned char)p[1], 1);
      p++;
    } else {
      tx.sendchar((unsigned char)*p, 0);
    }
  }
  tx.sendchar('\r', 0);
  if (!micro) {
    tx.sendchar(4, 0);
    tx.sendchar('\r', 0);
  }
  tx.flushtx();
  for (size_t i = 0; i < tx.tones.size(); i++) printf("%d%c", tx.tones[i], i + 1 == tx.tones.size() ? '\n' : ' ');
}

// ---------------------------------------------------------------- Viterbi
static void run_viterbi(int k, int p1, int p2, int traceback, unsigned seed, int n) {
  viterbi dec(k, p1, p2);
  dec.settraceback(traceback);
  dec.setchunksize(1);
  for (int i = 0; i < n; i++) {
    unsigned char sym[2];
    seed = seed * 1664525u + 1013904223u;
    sym[0] = (unsigned char)(seed >> 8);
    seed = seed * 1664525u + 1013904223u;
    sym[1] = (unsigned char)(seed >> 8);
    int met = 0;
    int bit = dec.decode(sym, &met);
    printf("%d %d\n", bit, met);
  }
}

int main(int argc, char **argv) {
  if (argc >= 7 && !strcmp(argv[1], "mfsk")) {
    run_mfsk(atoi(argv[2]), atoi(argv[3]), atoi(argv[4]), atoi(argv[5]) != 0, argv[6]);
  } else if (argc >= 7 && !strcmp(argv[1], "thor")) {
    run_thor(atoi(argv[2]) != 0, atoi(argv[3]), atoi(argv[4]), atoi(argv[5]) != 0, argv[6]);
  } else if (argc >= 8 && !strcmp(argv[1], "viterbi")) {
    run_viterbi(atoi(argv[2]), atoi(argv[3]), atoi(argv[4]), atoi(argv[5]), (unsigned)strtoul(argv[6], 0, 10), atoi(argv[7]));
  } else {
    fprintf(stderr, "usage: see header\n");
    return 2;
  }
  return 0;
}
